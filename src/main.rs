mod activewindow;
mod autostart;
mod config;
mod design;
mod exercises;
mod idle;
mod monitors;
mod overlay;
mod pomodoro;
mod raise;
mod settings;
mod singleton;
mod sounds;
mod state;
mod stats;
mod theme;
mod timer;
mod tray;
mod updater;

use config::Config;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("--overlay") => {
            // Internal mode: one overlay window on one monitor. Spawned by the
            // tray/scheduler process; not meant to be invoked directly.
            let x: i32 = args[2].parse().unwrap_or(0);
            let y: i32 = args[3].parse().unwrap_or(0);
            let w: u32 = args[4].parse().unwrap_or(1920);
            let h: u32 = args[5].parse().unwrap_or(1080);
            let display_secs: f32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(5.0);
            let exercise_index: usize = args.get(7).and_then(|s| s.parse().ok()).unwrap_or(0);
            // The window that had keyboard focus right before *any* of this
            // break's overlays were created, captured once by the
            // tray/scheduler and passed down to every per-monitor child —
            // rather than each child re-querying it independently, which
            // would risk one child picking up a sibling's already-mapped
            // overlay window instead of whatever the user was actually
            // using. Empty string means the tray couldn't determine one.
            let original_focus = args.get(8).filter(|s| !s.is_empty()).cloned();
            let _ = overlay::run_overlay(
                monitors::MonitorRect { x, y, w, h, primary: false },
                display_secs,
                exercise_index,
                original_focus,
            );
        }
        Some("--timer") => {
            // Internal mode: the persistent corner countdown window. Guarded
            // even though tray.rs's sync_timer_process already kills any
            // prior child before spawning a new one — this is a belt-and-
            // suspenders backstop against a race or a manual second launch.
            let Some(_lock) = singleton::try_acquire("timer") else {
                return;
            };
            let _ = timer::run_timer();
        }
        Some("--settings") => {
            // The full settings window, spawned by the tray's "Settings…"
            // item. If one's already open, raise it instead of opening a
            // second — clicking the menu item twice shouldn't spawn duplicate
            // windows.
            let Some(_lock) = singleton::try_acquire("settings") else {
                let _ = std::process::Command::new("wmctrl")
                    .args(["-a", "Eye Break — Settings"])
                    .status();
                return;
            };
            let _ = settings::run_settings();
        }
        Some("--open") => {
            // What the application-menu launcher actually runs (see
            // assets/eye-break.desktop: Exec=... --open) — distinct from the
            // separate autostart entry (assets/eye-break-autostart.desktop),
            // which still launches with no arguments at all and stays
            // silent. Clicking the app icon should always show *something*,
            // whether the tray/scheduler happens to be running already
            // (e.g. autostart got there first) or not — so this makes sure
            // it's running, then opens Settings (or raises it, if already
            // open, same as the `--settings` arm above).
            let exe = std::env::current_exe().unwrap_or_else(|_| "eye-break".into());
            // A no-op if the tray is already running: the spawned process
            // just fails its own `singleton::try_acquire("tray")` below and
            // exits immediately, exactly like a redundant manual launch
            // always has.
            let _ = std::process::Command::new(&exe).spawn();

            let Some(_lock) = singleton::try_acquire("settings") else {
                let _ = std::process::Command::new("wmctrl")
                    .args(["-a", "Eye Break — Settings"])
                    .status();
                return;
            };
            let _ = settings::run_settings();
        }
        Some("enable") => {
            let mut cfg = Config::load();
            cfg.enabled = true;
            cfg.save();
            println!("eye-break enabled");
        }
        Some("disable") => {
            let mut cfg = Config::load();
            cfg.enabled = false;
            cfg.save();
            println!("eye-break disabled");
        }
        Some("toggle") => {
            let mut cfg = Config::load();
            cfg.enabled = !cfg.enabled;
            cfg.save();
            println!("eye-break {}", if cfg.enabled { "enabled" } else { "disabled" });
        }
        Some("status") => {
            let cfg = Config::load();
            let st = state::State::load();
            let paused = st.is_manually_paused();
            println!(
                "enabled: {}\ninterval_secs: {}\ndisplay_secs: {}\nsnooze_secs: {}\nshow_timer: {}\npomodoro_enabled: {}",
                cfg.enabled, cfg.interval_secs, cfg.display_secs, cfg.snooze_secs, cfg.show_timer, cfg.pomodoro_enabled
            );
            match st.manual_pause_until_epoch {
                Some(u) if paused && u == state::MANUAL_PAUSE_INDEFINITE => {
                    println!("paused: until `eye-break resume` is run");
                }
                Some(u) if paused => {
                    let r = u.saturating_sub(state::now_epoch());
                    println!("paused: for {}m{}s more", r / 60, r % 60);
                }
                _ => println!("paused: no"),
            }
            // Pomodoro mode replaces the plain-interval scheduler entirely
            // (see tray.rs's scheduler tick) — showing `next_break_in`
            // against `interval_secs` while it's on would describe a
            // schedule nothing is actually driving, exactly the confusion
            // that cost real debugging time tracking down a "still losing
            // focus" report that turned out to be genuine Pomodoro breaks
            // firing on their own cycle the whole time.
            if !cfg.enabled || paused {
                // Covered by the enabled/paused lines above already.
            } else if cfg.pomodoro_enabled {
                let pstate = pomodoro::PomodoroState::load();
                let pcfg = pomodoro::PomodoroConfig::from(&cfg);
                let (remaining, _) = pstate.phase_progress(&pcfg, state::now_epoch());
                println!(
                    "pomodoro_phase: {} ({}m{}s left)",
                    pstate.phase.label(),
                    remaining / 60,
                    remaining % 60
                );
            } else {
                let r = st
                    .next_break_epoch(cfg.interval_secs)
                    .saturating_sub(state::now_epoch());
                println!("next_break_in: {}m{}s", r / 60, r % 60);
            }
        }
        Some("interval") => {
            let secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1200);
            let mut cfg = Config::load();
            cfg.interval_secs = secs;
            cfg.save();
            println!("break interval set to {secs}s");
        }
        Some("duration") => {
            let secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5);
            let mut cfg = Config::load();
            cfg.display_secs = secs;
            cfg.save();
            println!("overlay display duration set to {secs}s");
        }
        Some("snooze") => {
            let secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(300);
            let mut cfg = Config::load();
            cfg.snooze_secs = secs;
            cfg.save();
            println!("snooze length set to {secs}s");
        }
        Some("skip") => {
            let cfg = Config::load();
            let mut st = state::State::load();
            st.snooze_until_epoch = Some(state::now_epoch() + cfg.snooze_secs);
            st.dismiss_token += 1;
            st.save();
            println!("next break pushed out by {}s", cfg.snooze_secs);
        }
        Some("pause") => {
            // A manual, sustained pause for calls/presentations — same
            // mechanism the tray's "Pause" menu uses. No argument (or
            // "indefinite") pauses until `eye-break resume` is run; a
            // number of minutes pauses for exactly that long.
            let mut st = state::State::load();
            match args.get(2).map(String::as_str) {
                None | Some("indefinite") => {
                    st.start_manual_pause(state::MANUAL_PAUSE_INDEFINITE);
                    st.save();
                    println!("eye-break paused until `eye-break resume` is run");
                }
                Some(mins_str) => {
                    let Ok(mins) = mins_str.parse::<u64>() else {
                        eprintln!("eye-break: expected a number of minutes, got {mins_str:?}");
                        return;
                    };
                    st.start_manual_pause(state::now_epoch() + mins * 60);
                    st.save();
                    println!("eye-break paused for {mins} minutes");
                }
            }
        }
        Some("resume") => {
            let mut st = state::State::load();
            if st.is_manually_paused() {
                st.end_manual_pause();
                st.save();
                println!("eye-break resumed");
            } else {
                println!("eye-break wasn't paused");
            }
        }
        Some("--help") | Some("-h") => {
            print_help();
        }
        None => {
            // The default (no-args) launch is the tray icon + scheduler —
            // exactly one of these should ever run at a time, since two
            // would mean two tray icons, two schedulers racing to trigger
            // breaks, and two writers to the same state/config files.
            // Autostart + a manual launch (or double-clicking the app icon
            // twice) is the common way this would otherwise happen.
            let Some(_lock) = singleton::try_acquire("tray") else {
                eprintln!("eye-break: already running — not starting a second instance.");
                return;
            };
            tray::run();
        }
        Some(other) => {
            eprintln!("Unknown argument: {other}");
            print_help();
        }
    }
}

fn print_help() {
    println!(
        "eye-break — 20-20-20 rule reminder\n\
         \n\
         Usage:\n\
         \x20 eye-break            Run the tray icon + scheduler (default)\n\
         \x20 eye-break --open     What the app-menu icon launches: starts the tray if it\n\
         \x20                      isn't running yet, then opens Settings\n\
         \x20 eye-break enable     Enable reminders (works over SSH, no GUI needed)\n\
         \x20 eye-break disable    Disable reminders\n\
         \x20 eye-break toggle     Toggle reminders on/off\n\
         \x20 eye-break status     Show current settings\n\
         \x20 eye-break interval <secs>  Set break interval\n\
         \x20 eye-break duration <secs>  Set overlay display duration\n\
         \x20 eye-break snooze <secs>    Set the Skip snooze length\n\
         \x20 eye-break skip             Push the next break out by the snooze length\n\
         \x20 eye-break pause [mins]     Pause for calls/presentations — omit mins to\n\
         \x20                            pause until `resume` is run\n\
         \x20 eye-break resume           End a manual pause early"
    );
}

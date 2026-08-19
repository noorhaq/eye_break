mod config;
mod exercises;
mod monitors;
mod overlay;
mod pomodoro;
mod raise;
mod sounds;
mod state;
mod stats;
mod theme;
mod timer;
mod tray;

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
            let _ = overlay::run_overlay(
                monitors::MonitorRect { x, y, w, h, primary: false },
                display_secs,
                exercise_index,
            );
        }
        Some("--timer") => {
            // Internal mode: the persistent corner countdown window.
            let _ = timer::run_timer();
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
            let remaining = cfg.enabled.then(|| {
                st.next_break_epoch(cfg.interval_secs)
                    .saturating_sub(state::now_epoch())
            });
            println!(
                "enabled: {}\ninterval_secs: {}\ndisplay_secs: {}\nsnooze_secs: {}\nshow_timer: {}",
                cfg.enabled, cfg.interval_secs, cfg.display_secs, cfg.snooze_secs, cfg.show_timer
            );
            if let Some(r) = remaining {
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
        Some("--help") | Some("-h") => {
            print_help();
        }
        None => {
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
         \x20 eye-break enable     Enable reminders (works over SSH, no GUI needed)\n\
         \x20 eye-break disable    Disable reminders\n\
         \x20 eye-break toggle     Toggle reminders on/off\n\
         \x20 eye-break status     Show current settings\n\
         \x20 eye-break interval <secs>  Set break interval\n\
         \x20 eye-break duration <secs>  Set overlay display duration\n\
         \x20 eye-break snooze <secs>    Set the Skip snooze length\n\
         \x20 eye-break skip             Push the next break out by the snooze length"
    );
}

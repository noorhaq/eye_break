use crate::activewindow;
use crate::config::{self, Config};
use crate::exercises;
use crate::idle;
use crate::monitors::list_monitors;
use crate::pomodoro::{self, PomodoroState};
use crate::state::{now_epoch, State};
use crate::stats::{self, UsageLog};
use std::cell::RefCell;
use std::process::Child;
use std::rc::Rc;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{TrayIconBuilder, TrayIconEvent};

const DURATION_CHOICES_SECS: &[u64] = &[5, 10, 15, 20, 30, 45, 60];
const INTERVAL_CHOICES_MIN: &[u64] = &[10, 15, 20, 30, 45, 60];
const SNOOZE_CHOICES_MIN: &[u64] = &[5, 10, 15, 20];

fn build_icon() -> tray_icon::Icon {
    // Simple procedurally-drawn eye glyph: a light circle (iris) on transparent bg.
    const SIZE: u32 = 32;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    let cx = SIZE as f32 / 2.0;
    let cy = SIZE as f32 / 2.0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - cx;
            let dy = (y as f32 - cy) * 1.6; // squash vertically -> eye shape
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = ((y * SIZE + x) * 4) as usize;
            if dist < cx * 0.55 {
                // pupil
                rgba[idx] = 20;
                rgba[idx + 1] = 120;
                rgba[idx + 2] = 220;
                rgba[idx + 3] = 255;
            } else if dist < cx * 0.95 {
                // sclera
                rgba[idx] = 240;
                rgba[idx + 1] = 240;
                rgba[idx + 2] = 240;
                rgba[idx + 3] = 255;
            } else {
                rgba[idx + 3] = 0;
            }
        }
    }
    tray_icon::Icon::from_rgba(rgba, SIZE, SIZE).expect("failed to build tray icon")
}

/// Spawns the persistent corner-countdown process if enabled, killing any
/// previous instance first so toggling / restarts don't leak processes.
fn sync_timer_process(exe: &std::path::Path, cfg: &Config, handle: &Rc<RefCell<Option<Child>>>) {
    if let Some(mut child) = handle.borrow_mut().take() {
        let _ = child.kill();
    }
    if cfg.show_timer {
        if let Ok(child) = std::process::Command::new(exe).arg("--timer").spawn() {
            *handle.borrow_mut() = Some(child);
        }
    }
}

#[cfg(target_os = "linux")]
pub fn run() {
    gtk::init().expect("failed to init GTK (needed for the tray icon)");
    let should_quit = Rc::new(RefCell::new(false));
    let mut tick = build_tick(should_quit.clone());
    glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
        tick();
        if *should_quit.borrow() {
            gtk::main_quit();
        }
        glib::ControlFlow::Continue
    });
    println!("[eye-break] running. Right-click the tray icon for settings.");
    gtk::main();
}

/// macOS (and any other non-Linux target): `tray-icon` there just needs an
/// NSApplication-style run loop pumped periodically, not a GTK main loop —
/// `tao`'s `EventLoop` gives us that without pulling in GTK/X11 at all.
#[cfg(not(target_os = "linux"))]
pub fn run() {
    use tao::event_loop::{ControlFlow, EventLoopBuilder};

    let should_quit = Rc::new(RefCell::new(false));
    let mut tick = build_tick(should_quit.clone());

    println!("[eye-break] running. Click the tray icon for settings.");
    let event_loop = EventLoopBuilder::new().build();
    event_loop.run(move |_event, _target, control_flow| {
        *control_flow = ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(500),
        );
        tick();
        if *should_quit.borrow() {
            *control_flow = ControlFlow::Exit;
        }
    });
}

/// Builds the tray menu/icon and returns the recurring scheduler-tick
/// closure, shared verbatim between the GTK (Linux) and `tao` (everywhere
/// else) event-loop drivers above. Setting `*should_quit.borrow_mut() = true`
/// from within the closure (on the "Quit" menu item) is how it asks its
/// caller's loop to stop, since each platform's loop is stopped differently.
fn build_tick(should_quit: Rc<RefCell<bool>>) -> impl FnMut() {
    let config = Rc::new(RefCell::new(Config::load()));
    let exe = std::env::current_exe().unwrap_or_else(|_| "eye-break".into());

    let menu = Menu::new();
    let toggle_item = CheckMenuItem::new("Enabled", true, config.borrow().enabled, None);
    let timer_item = CheckMenuItem::new("Show corner countdown", true, config.borrow().show_timer, None);

    // "Reminder Interval" submenu: how often a break is triggered.
    let interval_menu = Submenu::new("Reminder Interval", true);
    let mut interval_items = Vec::new();
    for &min in INTERVAL_CHOICES_MIN {
        let checked = config.borrow().interval_secs == min * 60;
        let item = CheckMenuItem::new(format!("{min} min"), true, checked, None);
        interval_menu.append(&item).unwrap();
        interval_items.push((item, min * 60));
    }

    // "Break Duration" submenu: how long the overlay stays on screen.
    let duration_menu = Submenu::new("Break Duration", true);
    let mut duration_items = Vec::new();
    for &secs in DURATION_CHOICES_SECS {
        let checked = config.borrow().display_secs == secs;
        let item = CheckMenuItem::new(format!("{secs}s"), true, checked, None);
        duration_menu.append(&item).unwrap();
        duration_items.push((item, secs));
    }

    // "Snooze Length" submenu: how long "Skip" postpones the next break by.
    let snooze_menu = Submenu::new("Snooze Length", true);
    let mut snooze_items = Vec::new();
    for &min in SNOOZE_CHOICES_MIN {
        let checked = config.borrow().snooze_secs == min * 60;
        let item = CheckMenuItem::new(format!("{min} min"), true, checked, None);
        snooze_menu.append(&item).unwrap();
        snooze_items.push((item, min * 60));
    }

    let break_now_item = MenuItem::new("Take a break now", true, None);
    let skip_item = MenuItem::new("Skip next break (snooze)", true, None);
    let settings_item = MenuItem::new("Settings…", true, None);
    let quit_item = MenuItem::new("Quit", true, None);

    menu.append(&toggle_item).unwrap();
    menu.append(&timer_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&interval_menu).unwrap();
    menu.append(&duration_menu).unwrap();
    menu.append(&snooze_menu).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&break_now_item).unwrap();
    menu.append(&skip_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&settings_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&quit_item).unwrap();

    let toggle_id = toggle_item.id().clone();
    let timer_id = timer_item.id().clone();
    let break_now_id = break_now_item.id().clone();
    let skip_id = skip_item.id().clone();
    let settings_id = settings_item.id().clone();
    let quit_id = quit_item.id().clone();
    let interval_ids: Vec<_> = interval_items
        .iter()
        .map(|(item, secs)| (item.id().clone(), *secs))
        .collect();
    let duration_ids: Vec<_> = duration_items
        .iter()
        .map(|(item, secs)| (item.id().clone(), *secs))
        .collect();
    let snooze_ids: Vec<_> = snooze_items
        .iter()
        .map(|(item, secs)| (item.id().clone(), *secs))
        .collect();

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Eye Break — 20-20-20 rule")
        .with_icon(build_icon())
        .build()
        .expect("failed to build tray icon");

    let menu_channel = MenuEvent::receiver();
    let _tray_channel = TrayIconEvent::receiver();

    let timer_child: Rc<RefCell<Option<Child>>> = Rc::new(RefCell::new(None));
    sync_timer_process(&exe, &config.borrow(), &timer_child);

    let exe_tick = exe.clone();
    let timer_child_tick = timer_child.clone();

    let usage_log = Rc::new(RefCell::new(UsageLog::load()));
    let pomodoro_state = Rc::new(RefCell::new(PomodoroState::load()));
    let mut usage_save_countdown: u32 = 0;

    // Idle/fullscreen checks shell out to xprintidle/xdotool/xprop, so they
    // aren't run every 500ms tick — only every ~2s, with the result cached
    // for the ticks in between.
    let mut smart_pause_countdown: u32 = 0;
    let mut cached_is_idle = false;
    let mut cached_is_fullscreen = false;
    let mut was_idle = false;

    move || {
        // Keep the tray icon alive for as long as this closure (and thus the
        // whole scheduler loop) lives — it would otherwise be dropped, and
        // the icon vanish, the instant `build_tick` returns.
        let _tray = &_tray;

        // Pick up whatever's on disk before doing anything else this tick.
        // `config` was loaded once when this closure was built, and every
        // local mutation below re-saves it immediately — but the Settings
        // window runs as its own separate process, and any change made
        // there (e.g. corner timer opacity) only reaches *this* process by
        // being written to config.json. Without this reload, the next
        // `cfg.save()` from a menu toggle or `trigger_break()` would
        // overwrite that on-disk change with our stale in-memory copy,
        // making Settings changes appear to silently revert.
        *config.borrow_mut() = Config::load();

        // Handle menu events.
        while let Ok(event) = menu_channel.try_recv() {
            if event.id == toggle_id {
                let mut cfg = config.borrow_mut();
                cfg.enabled = !cfg.enabled;
                cfg.save();
                toggle_item.set_checked(cfg.enabled);
                println!(
                    "[eye-break] {}",
                    if cfg.enabled { "Enabled" } else { "Disabled" }
                );
            } else if event.id == timer_id {
                let mut cfg = config.borrow_mut();
                cfg.show_timer = !cfg.show_timer;
                cfg.save();
                timer_item.set_checked(cfg.show_timer);
                sync_timer_process(&exe_tick, &cfg, &timer_child_tick);
            } else if event.id == break_now_id {
                let mut cfg = config.borrow_mut();
                trigger_break(&mut cfg);
            } else if event.id == skip_id {
                let cfg = config.borrow();
                let mut st = State::load();
                st.snooze_until_epoch = Some(now_epoch() + cfg.snooze_secs);
                st.dismiss_token += 1;
                st.save();
                println!("[eye-break] next break snoozed by {}s", cfg.snooze_secs);
            } else if event.id == settings_id {
                let _ = std::process::Command::new(&exe_tick).arg("--settings").spawn();
            } else if event.id == quit_id {
                if let Some(mut child) = timer_child_tick.borrow_mut().take() {
                    let _ = child.kill();
                }
                *should_quit.borrow_mut() = true;
            } else if let Some((_, secs)) = interval_ids.iter().find(|(id, _)| *id == event.id) {
                let mut cfg = config.borrow_mut();
                cfg.interval_secs = *secs;
                cfg.save();
                for (item, item_secs) in &interval_items {
                    item.set_checked(item_secs == secs);
                }
                println!("[eye-break] reminder interval set to {}min", secs / 60);
            } else if let Some((_, secs)) = duration_ids.iter().find(|(id, _)| *id == event.id) {
                let mut cfg = config.borrow_mut();
                cfg.display_secs = *secs;
                cfg.save();
                for (item, item_secs) in &duration_items {
                    item.set_checked(item_secs == secs);
                }
                println!("[eye-break] break duration set to {secs}s");
            } else if let Some((_, secs)) = snooze_ids.iter().find(|(id, _)| *id == event.id) {
                let mut cfg = config.borrow_mut();
                cfg.snooze_secs = *secs;
                cfg.save();
                for (item, item_secs) in &snooze_items {
                    item.set_checked(item_secs == secs);
                }
                println!("[eye-break] snooze length set to {}min", secs / 60);
            }
        }

        // Idle / fullscreen detection, throttled to every ~2s (4 ticks) since
        // each check shells out to an external tool. Computed before the
        // usage-stats and scheduler blocks below since both depend on it.
        smart_pause_countdown += 1;
        if smart_pause_countdown >= 4 {
            smart_pause_countdown = 0;
            let cfg = config.borrow();
            cached_is_idle = cfg.idle_pause_enabled
                && idle::idle_secs()
                    .map(|secs| secs >= cfg.idle_pause_after_mins as u64 * 60)
                    .unwrap_or(false);
            cached_is_fullscreen =
                cfg.fullscreen_pause_enabled && activewindow::is_fullscreen_app_active();
        }

        // On the idle -> active transition, reset the schedule's baseline to
        // "now" rather than leaving it at whenever the last break was, so
        // returning from being away (including reconnecting a remote/RDP
        // session after it sat idle) doesn't immediately ambush the user
        // with a break for time they weren't even at the desk for.
        //
        // Both schedulers need this, not just the plain-interval one:
        // Pomodoro mode tracks its own separate `phase_started_epoch`, and
        // — critically — `pomodoro_due` is simply never called at all while
        // `cached_is_idle` is true (see the `due` block below), so that
        // clock doesn't freeze on its own the way it might appear to. It
        // just stops being *read*, so nothing advances the phase forward
        // while idle — but its start time is also never pushed forward,
        // so the instant the user comes back, `now - phase_started_epoch`
        // is however long they were actually away. If that's longer than
        // the phase's own length (a 10-minute Work phase after a 20-minute
        // RDP disconnect, say), the very first tick after reconnecting
        // sees the phase as already overdue and fires a break immediately
        // — the exact "ambushed the moment I reconnect" symptom.
        if cached_is_idle {
            was_idle = true;
        } else if was_idle {
            was_idle = false;
            let mut st = State::load();
            st.last_break_epoch = now_epoch();
            st.save();

            let mut pstate = pomodoro_state.borrow_mut();
            pstate.phase_started_epoch = now_epoch();
            pstate.save();
        }

        // Usage-stats: this tick fires every 500ms, so accumulate a whole
        // second every other tick (rather than double-counting by recording
        // 1s on every 500ms tick), and flush to disk every ~10s to keep disk
        // writes light. Skipped while idle so "usage" reflects active time.
        usage_save_countdown += 1;
        if usage_save_countdown % 2 == 0 && !cached_is_idle {
            stats::record_tick(&mut usage_log.borrow_mut(), 1);
        }
        if usage_save_countdown >= 20 {
            usage_save_countdown = 0;
            usage_log.borrow().save();
        }

        // Scheduler tick, driven off the shared epoch-based state so that
        // skips/snoozes triggered from an overlay window are respected.
        // Gated by the workday schedule, idle state, and fullscreen state
        // (each a no-op when its own toggle is disabled), and driven by
        // either the Pomodoro cycle or the plain interval scheduler,
        // depending on which mode is active.
        let due = {
            let cfg = config.borrow();
            if !cfg.enabled
                || !config::is_within_workday(&cfg)
                || cached_is_idle
                || cached_is_fullscreen
            {
                false
            } else if cfg.pomodoro_enabled {
                pomodoro::pomodoro_due(&mut pomodoro_state.borrow_mut(), &cfg)
            } else {
                now_epoch() >= State::load().next_break_epoch(cfg.interval_secs)
            }
        };
        if due {
            let mut cfg = config.borrow_mut();
            trigger_break(&mut cfg);
        }
    }
}

/// Spawn one overlay child process per monitor, all showing the same
/// (rotating) exercise, and record that a break happened. Each overlay
/// child manages its own lifetime and exits on its own, so this is
/// fire-and-forget.
fn trigger_break(cfg: &mut Config) {
    crate::sounds::play(&cfg.sound);
    let monitors = list_monitors();
    let exe = std::env::current_exe().unwrap_or_else(|_| "eye-break".into());
    let exercise_index = cfg.next_exercise;
    for m in monitors {
        let _ = std::process::Command::new(&exe)
            .arg("--overlay")
            .arg(m.x.to_string())
            .arg(m.y.to_string())
            .arg(m.w.to_string())
            .arg(m.h.to_string())
            .arg(cfg.display_secs.to_string())
            .arg(exercise_index.to_string())
            .spawn();
    }
    cfg.next_exercise = (exercise_index + 1) % exercises::count();
    cfg.save();

    let mut st = State::load();
    st.last_break_epoch = now_epoch();
    st.snooze_until_epoch = None;
    st.save();
}

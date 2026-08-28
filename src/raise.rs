use std::time::Duration;

/// Continuously forces this process's window(s) above everything else —
/// code editors, browsers, fullscreen apps, whatever the user is looking at
/// — and, if `restore_focus_to` is given, keeps keyboard input pinned back
/// on that window instead of letting ours steal it.
///
/// A single always-on-top hint set at window-creation time (what
/// `ViewportBuilder::with_always_on_top()` gives us) is often not enough on
/// GNOME/Mutter: another app can still raise itself over it later. So
/// instead we repeatedly re-assert the EWMH "above" state and re-raise the
/// window via `wmctrl`/`xdotool`, found by this process's PID, for as long
/// as the overlay is alive. Runs in a background thread; silently does
/// nothing if the tools aren't installed.
///
/// Raising and keyboard focus are independent in X11 — `windowraise` alone
/// never steals input focus — but newly-mapped windows are commonly
/// auto-focused by the window manager anyway (Mutter's default "focus new
/// windows" policy), which is what actually pulls keystrokes away from
/// whatever the user was typing into the instant a break overlay appears.
/// `xdotool windowfocus` sets input focus directly via `XSetInputFocus`
/// without going through the window manager's `_NET_ACTIVE_WINDOW` request
/// — unlike `windowactivate`, it does *not* also raise its target, so it
/// can safely fight to keep focus on the user's app while our window stays
/// visually on top via the separate raise calls above.
///
/// Only ever pass `Some` here for a window that's guaranteed to be
/// short-lived (the break overlay, which auto-closes within seconds and
/// whose background thread dies with the process) — never for a
/// long-running one like the corner timer. This loop never stops on its
/// own; for a process that runs the whole session, continuously forcing
/// focus back onto whatever was focused *when that process started*
/// fights every subsequent click into any other window, indefinitely,
/// which is a much worse bug than the one-time focus-steal-on-creation
/// this is meant to correct (previously shipped this way for the timer —
/// symptom was VS Code becoming totally unresponsive to clicks/typing
/// until eye-break itself was killed, since nothing short of that stopped
/// the fight).
pub fn keep_on_top_in_background(restore_focus_to: Option<String>) {
    let pid = std::process::id();
    std::thread::spawn(move || loop {
        assert_above_once(pid);
        if let Some(win) = &restore_focus_to {
            let _ = std::process::Command::new("xdotool")
                .args(["windowfocus", win])
                .output();
        }
        std::thread::sleep(Duration::from_millis(400));
    });
}

fn assert_above_once(pid: u32) {
    // Find the window(s) owned by this process.
    let Ok(out) = std::process::Command::new("xdotool")
        .args(["search", "--pid", &pid.to_string()])
        .output()
    else {
        return;
    };
    if !out.status.success() {
        return;
    }
    for win_id in String::from_utf8_lossy(&out.stdout).lines() {
        let win_id = win_id.trim();
        if win_id.is_empty() {
            continue;
        }
        // Pin "above" via EWMH state (wmctrl) and force it to the top of the
        // stack (xdotool) — belt and suspenders, since WM behavior varies.
        let _ = std::process::Command::new("wmctrl")
            .args(["-i", "-r", win_id, "-b", "add,above,sticky"])
            .output();
        let _ = std::process::Command::new("xdotool")
            .args(["windowraise", win_id])
            .output();
    }
}

/// The X window ID currently holding keyboard focus, if any — meant to be
/// captured *before* a new window (an overlay, the corner timer) is
/// created, so that ID can be handed to `keep_on_top_in_background` as the
/// window to keep returning focus to. `None` if `xdotool` is unavailable or
/// the query fails; callers degrade gracefully (nothing to restore focus
/// to, so the new window just behaves as it would have before this fix).
pub fn capture_focused_window() -> Option<String> {
    let out = std::process::Command::new("xdotool")
        .arg("getactivewindow")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

/// The mouse cursor's current position in global (root-window) screen
/// coordinates, queried independently of any particular window's input
/// shape. Used to hit-test the overlay's buttons while the overlay itself
/// is click-through everywhere else: once a window's input shape is empty
/// (mouse pass-through engaged), the X server stops delivering it motion
/// events entirely, so egui's own pointer tracking goes blind — this gives
/// an out-of-band way to know where the cursor is regardless. `None` if
/// `xdotool` is unavailable or the query fails.
pub fn global_mouse_pos() -> Option<(f32, f32)> {
    let out = std::process::Command::new("xdotool")
        .args(["getmouselocation", "--shell"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut x = None;
    let mut y = None;
    for line in text.lines() {
        let (key, val) = line.split_once('=')?;
        match key {
            "X" => x = val.parse::<f32>().ok(),
            "Y" => y = val.parse::<f32>().ok(),
            _ => {}
        }
    }
    Some((x?, y?))
}

use std::time::Duration;

/// Continuously forces this process's window(s) above everything else —
/// code editors, browsers, fullscreen apps, whatever the user is looking at.
///
/// A single always-on-top hint set at window-creation time (what
/// `ViewportBuilder::with_always_on_top()` gives us) is often not enough on
/// GNOME/Mutter: another app can still raise itself over it later. So
/// instead we repeatedly re-assert the EWMH "above" state and re-raise the
/// window via `wmctrl`/`xdotool`, found by this process's PID, for as long
/// as the overlay is alive. Runs in a background thread; silently does
/// nothing if the tools aren't installed.
pub fn keep_on_top_in_background() {
    let pid = std::process::id();
    std::thread::spawn(move || loop {
        assert_above_once(pid);
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

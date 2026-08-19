use std::time::Duration;

/// Continuously forces this process's window(s) above everything else —
/// code editors, browsers, fullscreen apps, whatever the user is looking at.
///
/// A single always-on-top hint set at window-creation time (what
/// `ViewportBuilder::with_always_on_top()` gives us) is often not enough —
/// another app can still raise itself over it later. So instead we
/// repeatedly re-assert "above" for as long as the overlay is alive. Runs in
/// a background thread; silently does nothing if the platform tools/APIs
/// aren't available.
pub fn keep_on_top_in_background() {
    let pid = std::process::id();
    std::thread::spawn(move || loop {
        assert_above_once(pid);
        std::thread::sleep(Duration::from_millis(400));
    });
}

#[cfg(target_os = "linux")]
fn assert_above_once(pid: u32) {
    // Find the window(s) owned by this process via `xdotool`, then pin
    // "above" via EWMH state (wmctrl) and force it to the top of the stack
    // (xdotool) — belt and suspenders, since WM behavior varies.
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
        let _ = std::process::Command::new("wmctrl")
            .args(["-i", "-r", win_id, "-b", "add,above,sticky"])
            .output();
        let _ = std::process::Command::new("xdotool")
            .args(["windowraise", win_id])
            .output();
    }
}

#[cfg(windows)]
fn assert_above_once(pid: u32) {
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, SetForegroundWindow, SetWindowPos, ShowWindow,
        HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SW_SHOW,
    };

    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let target_pid = lparam as u32;
        let mut owner_pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut owner_pid);
        if owner_pid == target_pid {
            ShowWindow(hwnd, SW_SHOW);
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE,
            );
            SetForegroundWindow(hwnd);
        }
        1 // continue enumeration; a process can own more than one top-level window
    }

    unsafe {
        EnumWindows(Some(callback), pid as LPARAM);
    }
}

use std::process::Command;

/// Whether the currently active/focused window appears to be fullscreen —
/// video calls, presentations, movies — via the EWMH `_NET_WM_STATE`
/// property (`xdotool` to find the active window, `xprop` to read its
/// state). Returns `false` ("not fullscreen, go ahead and interrupt") if
/// either tool is missing or the query fails, same fail-open philosophy as
/// the rest of the app's X11 shell-outs.
pub fn is_fullscreen_app_active() -> bool {
    let Ok(out) = Command::new("xdotool").arg("getactivewindow").output() else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    let win_id = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if win_id.is_empty() {
        return false;
    }

    let Ok(prop) = Command::new("xprop")
        .args(["-id", &win_id, "_NET_WM_STATE"])
        .output()
    else {
        return false;
    };
    if !prop.status.success() {
        return false;
    }
    String::from_utf8_lossy(&prop.stdout).contains("_NET_WM_STATE_FULLSCREEN")
}

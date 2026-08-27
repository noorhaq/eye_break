use std::process::Command;

/// Whether the currently active/focused window appears to be fullscreen —
/// video calls, presentations, movies — via the EWMH `_NET_WM_STATE`
/// property.
///
/// Both the "which window has focus" and "what state is it in" halves are
/// answered by `xprop` (x11-utils) rather than `xdotool`: the root window's
/// `_NET_ACTIVE_WINDOW` property carries the focused window's id, so there
/// is no reason to require a second tool for it. `xdotool` remains a
/// fallback for the window-id half, for the odd WM that doesn't publish
/// `_NET_ACTIVE_WINDOW`.
///
/// Returns `false` ("not fullscreen, go ahead and interrupt") if the tools
/// are missing or the query fails, same fail-open philosophy as the rest of
/// the app's X11 shell-outs — note that this means a missing `xprop` turns
/// fullscreen suppression off silently, which is why x11-utils is a hard
/// package dependency rather than a recommendation.
pub fn is_fullscreen_app_active() -> bool {
    let Some(win_id) = active_window_id() else {
        return false;
    };

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

/// The focused window's X id, as a string `xprop -id` accepts.
fn active_window_id() -> Option<String> {
    if let Some(id) = active_window_id_via_xprop() {
        return Some(id);
    }
    active_window_id_via_xdotool()
}

fn active_window_id_via_xprop() -> Option<String> {
    let out = Command::new("xprop")
        .args(["-root", "_NET_ACTIVE_WINDOW"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    // e.g. `_NET_ACTIVE_WINDOW(WINDOW): window id # 0x3c00007`
    let text = String::from_utf8_lossy(&out.stdout);
    let id = text.split_whitespace().find(|t| t.starts_with("0x"))?;
    // Some WMs report 0x0 when nothing is focused (e.g. the desktop itself).
    if id.trim_end_matches(',') == "0x0" {
        return None;
    }
    Some(id.trim_end_matches(',').to_string())
}

fn active_window_id_via_xdotool() -> Option<String> {
    let out = Command::new("xdotool").arg("getactivewindow").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if id.is_empty() {
        return None;
    }
    Some(id)
}

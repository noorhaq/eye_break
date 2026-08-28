use crate::monitors::list_monitors;
use std::process::Command;

/// Whether the currently active/focused window appears to be fullscreen —
/// video calls, movies, games, presentations. Tries three signals, from
/// most to least standard, since real-world apps don't agree on how they
/// announce fullscreen:
///
/// 1. `_NET_WM_STATE_FULLSCREEN` — the formal EWMH state. What GNOME/most
///    browsers set when a window (or an HTML5 video) goes fullscreen.
/// 2. `_NET_WM_BYPASS_COMPOSITOR = 1` — a separate hint fullscreen video
///    players and games set specifically to say "don't waste GPU
///    compositing me", independent of whether they also set the formal
///    state above. Chrome, mpv, and most compositor-aware fullscreen apps
///    set this even in cases where (1) is inconsistent.
/// 3. Geometry fallback — the active window's bounds exactly match one of
///    the real monitor rects from `xrandr` (not just "large", the *exact*
///    full monitor). Catches games and older/simpler apps that resize
///    themselves to cover the screen via direct XRandR/geometry rather
///    than asking the window manager for fullscreen at all, and so never
///    set either state above. A window that merely fills the GNOME work
///    area (i.e. a normal maximize) doesn't match this, since the top bar
///    still claims part of the monitor.
///
/// Returns `false` ("not fullscreen, go ahead and interrupt") if every
/// signal is unavailable or says no — same fail-open philosophy as the
/// rest of the app's X11 shell-outs.
pub fn is_fullscreen_app_active() -> bool {
    let Some(win_id) = active_window_id() else {
        return false;
    };

    if window_state_contains(&win_id, "_NET_WM_STATE_FULLSCREEN") {
        return true;
    }
    if window_bypasses_compositor(&win_id) {
        return true;
    }
    if window_covers_a_monitor(&win_id) {
        return true;
    }
    false
}

fn active_window_id() -> Option<String> {
    let out = Command::new("xdotool")
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

fn window_state_contains(win_id: &str, atom: &str) -> bool {
    let Ok(prop) = Command::new("xprop")
        .args(["-id", win_id, "_NET_WM_STATE"])
        .output()
    else {
        return false;
    };
    prop.status.success() && String::from_utf8_lossy(&prop.stdout).contains(atom)
}

fn window_bypasses_compositor(win_id: &str) -> bool {
    let Ok(prop) = Command::new("xprop")
        .args(["-id", win_id, "_NET_WM_BYPASS_COMPOSITOR"])
        .output()
    else {
        return false;
    };
    if !prop.status.success() {
        return false;
    }
    // Expected form: `_NET_WM_BYPASS_COMPOSITOR(CARDINAL) = 1`
    String::from_utf8_lossy(&prop.stdout)
        .split('=')
        .nth(1)
        .map(|v| v.trim() == "1")
        .unwrap_or(false)
}

fn window_covers_a_monitor(win_id: &str) -> bool {
    let Some((x, y, w, h)) = window_geometry(win_id) else {
        return false;
    };
    list_monitors()
        .iter()
        .any(|m| m.x == x && m.y == y && m.w == w && m.h == h)
}

fn window_geometry(win_id: &str) -> Option<(i32, i32, u32, u32)> {
    let out = Command::new("xdotool")
        .args(["getwindowgeometry", "--shell", win_id])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut x = None;
    let mut y = None;
    let mut w = None;
    let mut h = None;
    for line in text.lines() {
        let (key, val) = line.split_once('=')?;
        match key {
            "X" => x = val.parse().ok(),
            "Y" => y = val.parse().ok(),
            "WIDTH" => w = val.parse().ok(),
            "HEIGHT" => h = val.parse().ok(),
            _ => {}
        }
    }
    Some((x?, y?, w?, h?))
}

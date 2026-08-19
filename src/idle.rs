use std::process::Command;

/// Seconds the X11 session has been idle (no keyboard/mouse input), queried
/// via the `xprintidle` CLI tool. Returns `None` if the tool isn't
/// installed or the call fails, so callers degrade gracefully — idle
/// detection just doesn't gate anything — rather than blocking the app's
/// core function, matching the fail-open pattern the rest of the app uses
/// for its other X11 shell-outs (xrandr, xdotool, wmctrl).
pub fn idle_secs() -> Option<u64> {
    let out = Command::new("xprintidle").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let ms: u64 = text.trim().parse().ok()?;
    Some(ms / 1000)
}

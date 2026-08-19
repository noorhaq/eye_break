/// Query monitor geometries via `xrandr`. Works on X11 (this system's session type).
/// Falls back to a single 1920x1080 "monitor" at origin if xrandr is unavailable
/// or nothing could be parsed, so the app still functions.
#[derive(Debug, Clone, Copy)]
pub struct MonitorRect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub primary: bool,
}

pub fn list_monitors() -> Vec<MonitorRect> {
    let output = std::process::Command::new("xrandr")
        .arg("--query")
        .output();

    let mut monitors = Vec::new();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if !line.contains(" connected") {
                    continue;
                }
                let primary = line.contains(" primary ") || line.contains(" primary\t");
                // Look for a token like "1920x1080+0+0"
                for token in line.split_whitespace() {
                    if let Some(mut rect) = parse_geometry(token) {
                        rect.primary = primary;
                        monitors.push(rect);
                        break;
                    }
                }
            }
        }
    }

    if monitors.is_empty() {
        monitors.push(MonitorRect { x: 0, y: 0, w: 1920, h: 1080, primary: true });
    } else if !monitors.iter().any(|m| m.primary) {
        monitors[0].primary = true;
    }

    monitors
}

/// The monitor to anchor single-instance UI (like the corner countdown) to.
pub fn primary_monitor() -> MonitorRect {
    let monitors = list_monitors();
    monitors
        .iter()
        .find(|m| m.primary)
        .copied()
        .unwrap_or(monitors[0])
}

fn parse_geometry(token: &str) -> Option<MonitorRect> {
    // Format: WxH+X+Y  (possibly with trailing stuff we ignore)
    let (wh, rest) = token.split_once('+')?;
    let (x_str, y_str) = rest.split_once('+')?;
    let (w_str, h_str) = wh.split_once('x')?;

    let w: u32 = w_str.parse().ok()?;
    let h: u32 = h_str.parse().ok()?;
    let x: i32 = x_str.parse().ok()?;
    let y: i32 = y_str.parse().ok()?;

    Some(MonitorRect { x, y, w, h, primary: false })
}

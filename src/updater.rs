//! Checks GitHub for a newer release of eye-break.
//!
//! Not wired into the tray menu yet — a future integration pass is expected
//! to call `check_for_update` (typically on a background thread, since this
//! whole app is otherwise synchronous) and surface the result in the UI.

use serde::Deserialize;
use std::time::Duration;

const RELEASES_URL: &str = "https://api.github.com/repos/noorhaq/eye_break/releases/latest";

#[derive(Deserialize)]
struct ReleaseResponse {
    tag_name: String,
}

/// Parses a "major.minor.patch"-ish version string into a comparable tuple.
/// Non-numeric / missing components default to 0, and a leading 'v' is
/// stripped so tags like "v1.2.3" compare the same as "1.2.3".
fn parse_version(v: &str) -> (u64, u64, u64) {
    let v = v.trim().trim_start_matches('v');
    let mut parts = v.split('.').map(|p| {
        p.chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<u64>()
            .unwrap_or(0)
    });
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

/// Returns `Some(new_version)` if the latest GitHub release tag is newer
/// than `current_version`. Returns `None` on any network/parse error, or if
/// already up to date. Never panics; the underlying HTTP call has a short
/// timeout so this never blocks for long.
#[allow(dead_code)]
pub fn check_for_update(current_version: &str) -> Option<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(3))
        .timeout(Duration::from_secs(5))
        .build();

    let resp = agent
        .get(RELEASES_URL)
        .set("User-Agent", "eye-break-updater")
        .call()
        .ok()?;

    let release: ReleaseResponse = resp.into_json().ok()?;
    let latest = parse_version(&release.tag_name);
    let current = parse_version(current_version);

    if latest > current {
        Some(release.tag_name)
    } else {
        None
    }
}

/// Spawns a background thread that performs the update check and calls
/// `on_result` with the outcome. Never blocks the caller.
#[allow(dead_code)]
pub fn check_for_update_async(
    current_version: &'static str,
    on_result: impl FnOnce(Option<String>) + Send + 'static,
) {
    std::thread::spawn(move || {
        let result = check_for_update(current_version);
        on_result(result);
    });
}

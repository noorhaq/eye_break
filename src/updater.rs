//! Checks for a newer release of eye-break.
//!
//! Wired into Settings' General tab ("Check for updates", via
//! `check_for_update_async`, on a background thread since this whole app is
//! otherwise synchronous).
//!
//! Reads `version.json` off the project website rather than hitting
//! GitHub's API (`.../releases/latest`) directly. GitHub's unauthenticated
//! REST API is capped at 60 requests/hour *per IP* — shared with everything
//! else on a user's network — so every install phoning home to it directly
//! doesn't scale and can start failing for reasons that have nothing to do
//! with eye-break. `version.json` is a small static file on Vercel's CDN
//! instead, with no comparable limit; it's kept in sync with the actual
//! GitHub release by hand on each version bump (see eye-break-website).
//! The website's own "latest version" display reads the same file, for the
//! same reason.

use serde::Deserialize;
use std::time::Duration;

const VERSION_URL: &str = "https://eye-break-one.vercel.app/version.json";

#[derive(Deserialize)]
struct ReleaseResponse {
    /// Plain semver, e.g. "0.5.0" (no leading "v") — used for the actual
    /// version comparison.
    version: String,
    /// The matching git tag, e.g. "v0.5.0" — only used for display, so a
    /// "new version available" message reads the same as the GitHub tag
    /// a user would go looking for.
    tag: String,
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

/// Returns `Some(new_version_tag)` if the latest release is newer than
/// `current_version`. Returns `None` on any network/parse error, or if
/// already up to date. Never panics; the underlying HTTP call has a short
/// timeout so this never blocks for long.
pub fn check_for_update(current_version: &str) -> Option<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(3))
        .timeout(Duration::from_secs(5))
        .build();

    let resp = agent
        .get(VERSION_URL)
        .set("User-Agent", "eye-break-updater")
        .call()
        .ok()?;

    let release: ReleaseResponse = resp.into_json().ok()?;
    let latest = parse_version(&release.version);
    let current = parse_version(current_version);

    if latest > current {
        Some(release.tag)
    } else {
        None
    }
}

/// Spawns a background thread that performs the update check and calls
/// `on_result` with the outcome. Never blocks the caller.
pub fn check_for_update_async(
    current_version: &'static str,
    on_result: impl FnOnce(Option<String>) + Send + 'static,
) {
    std::thread::spawn(move || {
        let result = check_for_update(current_version);
        on_result(result);
    });
}

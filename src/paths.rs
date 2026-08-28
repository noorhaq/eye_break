//! Shared config-directory resolution. eye-break's persisted files
//! (`config.json`, `state.json`, `pomodoro_state.json`, `usage_log.json`,
//! and the per-kind instance locks) all live side by side in the same
//! platform config directory, so the "find/create that directory and join
//! a file name onto it" logic is centralized here rather than repeated in
//! each module.

use std::path::PathBuf;

/// Returns the path to `file_name` inside eye-break's config directory,
/// creating that directory first if it doesn't exist yet.
pub fn config_file(file_name: &str) -> PathBuf {
    let dirs = directories::ProjectDirs::from("dev", "eye-break", "eye-break")
        .expect("could not determine config dir");
    let dir = dirs.config_dir();
    std::fs::create_dir_all(dir).ok();
    dir.join(file_name)
}

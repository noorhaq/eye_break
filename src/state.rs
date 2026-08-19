use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Runtime scheduling state, shared (via a small JSON file) between the
/// tray/scheduler process, the corner countdown process, and the overlay
/// break-window processes. Kept separate from `Config` (user settings)
/// because it changes every break / snooze, not just on user action.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct State {
    /// Epoch seconds when the last break was shown (baseline for the next one).
    pub last_break_epoch: u64,
    /// If set and still in the future, overrides the normal schedule —
    /// used by "Skip" to push the next break out by the snooze length.
    pub snooze_until_epoch: Option<u64>,
    /// Bumped whenever a break should be dismissed everywhere (e.g. Skip was
    /// clicked on one monitor's overlay); sibling overlay processes watch
    /// this and close themselves when it changes.
    pub dismiss_token: u64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            last_break_epoch: now_epoch(),
            snooze_until_epoch: None,
            dismiss_token: 0,
        }
    }
}

pub fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn state_path() -> PathBuf {
    let dirs = directories::ProjectDirs::from("dev", "eye-break", "eye-break")
        .expect("could not determine config dir");
    let dir = dirs.config_dir();
    std::fs::create_dir_all(dir).ok();
    dir.join("state.json")
}

impl State {
    pub fn load() -> Self {
        let path = state_path();
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => {
                let state = State::default();
                state.save();
                state
            }
        }
    }

    pub fn save(&self) {
        let path = state_path();
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, s);
        }
    }

    /// Epoch seconds at which the next break is due, given the configured interval.
    pub fn next_break_epoch(&self, interval_secs: u64) -> u64 {
        if let Some(snooze) = self.snooze_until_epoch {
            if snooze > now_epoch() {
                return snooze;
            }
        }
        self.last_break_epoch + interval_secs
    }
}

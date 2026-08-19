use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub enabled: bool,
    pub interval_secs: u64,
    #[serde(default = "default_display_secs")]
    pub display_secs: u64,
    /// Index into the exercises list; advances by one each break so
    /// exercises rotate instead of always showing the same one.
    #[serde(default)]
    pub next_exercise: usize,
    /// How long "Skip" postpones the next break by.
    #[serde(default = "default_snooze_secs")]
    pub snooze_secs: u64,
    /// Whether to show the always-on corner countdown to the next break.
    #[serde(default = "default_true")]
    pub show_timer: bool,
    /// Whether breaks are restricted to a configured workday schedule.
    /// When false (the default), reminders fire all day, matching the
    /// original behavior.
    #[serde(default)]
    pub workday_enabled: bool,
    /// Hour (0-23, local time) the workday schedule starts at.
    #[serde(default = "default_workday_start_hour")]
    pub workday_start_hour: u8,
    /// Hour (0-23, local time) the workday schedule ends at.
    #[serde(default = "default_workday_end_hour")]
    pub workday_end_hour: u8,
    /// Which weekdays the workday schedule applies to, Monday first
    /// (`workday_days[0]` = Monday, ..., `workday_days[6]` = Sunday).
    #[serde(default = "default_workday_days")]
    pub workday_days: [bool; 7],
}

fn default_display_secs() -> u64 {
    5
}

fn default_snooze_secs() -> u64 {
    5 * 60
}

fn default_true() -> bool {
    true
}

fn default_workday_start_hour() -> u8 {
    9
}

fn default_workday_end_hour() -> u8 {
    17
}

fn default_workday_days() -> [bool; 7] {
    [true; 7]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 20 * 60, // 20 minutes
            display_secs: default_display_secs(),
            next_exercise: 0,
            snooze_secs: default_snooze_secs(),
            show_timer: default_true(),
            workday_enabled: false,
            workday_start_hour: default_workday_start_hour(),
            workday_end_hour: default_workday_end_hour(),
            workday_days: default_workday_days(),
        }
    }
}

fn config_path() -> PathBuf {
    let dirs = directories::ProjectDirs::from("dev", "eye-break", "eye-break")
        .expect("could not determine config dir");
    let dir = dirs.config_dir();
    std::fs::create_dir_all(dir).ok();
    dir.join("config.json")
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => {
                let cfg = Config::default();
                cfg.save();
                cfg
            }
        }
    }

    pub fn save(&self) {
        let path = config_path();
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, s);
        }
    }
}

/// Returns whether the current moment falls inside the configured workday
/// schedule (`workday_start_hour`..`workday_end_hour`, on a
/// `workday_days`-enabled weekday). Always returns `true` when
/// `workday_enabled` is false, matching the original always-on behavior.
///
/// Intended for the tray/scheduler integration step to call before
/// triggering a break.
///
/// Note: time-of-day and weekday are computed from UTC (no timezone crate
/// dependency), so on machines far from UTC this may not line up with local
/// wall-clock hours. Good enough as a first pass; revisit if precise local
/// time is needed.
#[allow(dead_code)]
pub fn is_within_workday(cfg: &Config) -> bool {
    if !cfg.workday_enabled {
        return true;
    }
    let epoch_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let days_since_epoch = (epoch_secs / 86_400) as i64;
    let secs_of_day = epoch_secs % 86_400;
    let hour = (secs_of_day / 3600) as u8;

    // 1970-01-01 was a Thursday (weekday index 3 if Monday = 0).
    let weekday = (((days_since_epoch % 7) + 7 + 3) % 7) as usize; // 0 = Monday .. 6 = Sunday

    if !cfg.workday_days[weekday] {
        return false;
    }
    hour >= cfg.workday_start_hour && hour < cfg.workday_end_hour
}

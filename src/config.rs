use crate::sounds;
use crate::theme::Theme;
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
    /// Visual theme for the overlay and corner timer windows.
    #[serde(default)]
    pub theme: Theme,
    /// Notification sound to play when a break overlay is triggered.
    #[serde(default)]
    pub sound: sounds::SoundChoice,

    /// Whether Pomodoro-technique scheduling is enabled (alternate/complementary
    /// mode to the plain interval scheduler above). Off by default.
    #[serde(default)]
    pub pomodoro_enabled: bool,
    /// Length of a Pomodoro work phase, in minutes.
    #[serde(default = "default_pomodoro_work_mins")]
    pub pomodoro_work_mins: u32,
    /// Length of a short break, in minutes.
    #[serde(default = "default_pomodoro_short_break_mins")]
    pub pomodoro_short_break_mins: u32,
    /// Length of a long break, in minutes.
    #[serde(default = "default_pomodoro_long_break_mins")]
    pub pomodoro_long_break_mins: u32,
    /// Number of completed work cycles before a long break is taken.
    #[serde(default = "default_pomodoro_cycles_before_long_break")]
    pub pomodoro_cycles_before_long_break: u32,

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

    /// The headline shown at the top of the break overlay.
    #[serde(default = "default_reminder_text")]
    pub reminder_text: String,

    /// Whether to skip triggering a break while the active window appears
    /// fullscreen (video calls, presentations, movies, ...).
    #[serde(default = "default_true")]
    pub fullscreen_pause_enabled: bool,
    /// Whether to pause the break schedule while the system is idle (no
    /// keyboard/mouse input for `idle_pause_after_mins`), so reminders
    /// don't fire at an empty desk.
    #[serde(default = "default_true")]
    pub idle_pause_enabled: bool,
    /// Minutes of no keyboard/mouse input before the system is considered
    /// idle for the purposes of `idle_pause_enabled`.
    #[serde(default = "default_idle_pause_after_mins")]
    pub idle_pause_after_mins: u32,
}

fn default_reminder_text() -> String {
    "Time for an eye break!".to_string()
}

fn default_idle_pause_after_mins() -> u32 {
    5
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

fn default_pomodoro_work_mins() -> u32 {
    25
}

fn default_pomodoro_short_break_mins() -> u32 {
    5
}

fn default_pomodoro_long_break_mins() -> u32 {
    15
}

fn default_pomodoro_cycles_before_long_break() -> u32 {
    4
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
            theme: Theme::default(),
            sound: sounds::SoundChoice::default(),
            pomodoro_enabled: false,
            pomodoro_work_mins: default_pomodoro_work_mins(),
            pomodoro_short_break_mins: default_pomodoro_short_break_mins(),
            pomodoro_long_break_mins: default_pomodoro_long_break_mins(),
            pomodoro_cycles_before_long_break: default_pomodoro_cycles_before_long_break(),
            workday_enabled: false,
            workday_start_hour: default_workday_start_hour(),
            workday_end_hour: default_workday_end_hour(),
            workday_days: default_workday_days(),
            reminder_text: default_reminder_text(),
            fullscreen_pause_enabled: default_true(),
            idle_pause_enabled: default_true(),
            idle_pause_after_mins: default_idle_pause_after_mins(),
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

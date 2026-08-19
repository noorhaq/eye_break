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

fn default_display_secs() -> u64 {
    5
}

fn default_snooze_secs() -> u64 {
    5 * 60
}

fn default_true() -> bool {
    true
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

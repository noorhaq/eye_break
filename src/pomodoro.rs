use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config::Config;
use crate::state::now_epoch;

/// Which phase of the Pomodoro cycle is currently active.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum PomodoroPhase {
    Work,
    ShortBreak,
    LongBreak,
}

/// Persistent Pomodoro scheduling state, analogous to `state::State` but for
/// the Pomodoro alternate/complementary scheduling mode. Stored in its own
/// JSON file so it doesn't interfere with the plain interval scheduler's
/// state file.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PomodoroState {
    pub phase: PomodoroPhase,
    /// Number of Work phases completed so far (resets after a long break).
    pub cycles_completed: u32,
    /// Epoch seconds when the current phase started.
    pub phase_started_epoch: u64,
}

impl Default for PomodoroState {
    fn default() -> Self {
        Self {
            phase: PomodoroPhase::Work,
            cycles_completed: 0,
            phase_started_epoch: now_epoch(),
        }
    }
}

/// Config-driven Pomodoro durations, extracted from `Config` so `tick` isn't
/// coupled to every unrelated config field.
#[derive(Debug, Clone, Copy)]
pub struct PomodoroConfig {
    pub work_mins: u32,
    pub short_break_mins: u32,
    pub long_break_mins: u32,
    pub cycles_before_long_break: u32,
}

impl From<&Config> for PomodoroConfig {
    fn from(cfg: &Config) -> Self {
        Self {
            work_mins: cfg.pomodoro_work_mins,
            short_break_mins: cfg.pomodoro_short_break_mins,
            long_break_mins: cfg.pomodoro_long_break_mins,
            cycles_before_long_break: cfg.pomodoro_cycles_before_long_break,
        }
    }
}

impl PomodoroPhase {
    fn duration_secs(self, cfg: &PomodoroConfig) -> u64 {
        let mins = match self {
            PomodoroPhase::Work => cfg.work_mins,
            PomodoroPhase::ShortBreak => cfg.short_break_mins,
            PomodoroPhase::LongBreak => cfg.long_break_mins,
        };
        mins as u64 * 60
    }
}

fn pomodoro_state_path() -> PathBuf {
    let dirs = directories::ProjectDirs::from("dev", "eye-break", "eye-break")
        .expect("could not determine config dir");
    let dir = dirs.config_dir();
    std::fs::create_dir_all(dir).ok();
    dir.join("pomodoro_state.json")
}

impl PomodoroState {
    pub fn load() -> Self {
        let path = pomodoro_state_path();
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => {
                let state = PomodoroState::default();
                state.save();
                state
            }
        }
    }

    pub fn save(&self) {
        let path = pomodoro_state_path();
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, s);
        }
    }

    /// Epoch seconds at which the current phase is due to end.
    pub fn phase_due_epoch(&self, cfg: &PomodoroConfig) -> u64 {
        self.phase_started_epoch + self.phase.duration_secs(cfg)
    }
}

/// Advances `state` to the next phase if the current phase's duration has
/// elapsed (epoch-based, consistent with `state::State`'s scheduling).
/// Returns true if the phase just changed, so a caller can show a
/// notification/overlay for the transition.
pub fn tick(state: &mut PomodoroState, cfg: &PomodoroConfig) -> bool {
    if now_epoch() < state.phase_due_epoch(cfg) {
        return false;
    }

    state.phase = match state.phase {
        PomodoroPhase::Work => {
            state.cycles_completed += 1;
            if state.cycles_completed % cfg.cycles_before_long_break.max(1) == 0 {
                PomodoroPhase::LongBreak
            } else {
                PomodoroPhase::ShortBreak
            }
        }
        PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak => PomodoroPhase::Work,
    };
    state.phase_started_epoch = now_epoch();
    state.save();
    true
}

/// Hook for the tray/scheduler integration pass: returns true when, under
/// Pomodoro scheduling, a break should be triggered right now (i.e. the
/// current phase is a break phase that has just become due). Intended to be
/// called from tray.rs's scheduler tick alongside (or instead of) the
/// existing interval-based `due` check when `cfg.pomodoro_enabled` is true.
/// Advances and persists `state` as a side effect via `tick`.
pub fn pomodoro_due(state: &mut PomodoroState, cfg: &Config) -> bool {
    let pcfg = PomodoroConfig::from(cfg);
    let phase_changed = tick(state, &pcfg);
    phase_changed && matches!(state.phase, PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak)
}

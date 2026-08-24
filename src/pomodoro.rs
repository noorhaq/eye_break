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

    /// Short label for the current phase, shown by the corner countdown in
    /// place of the plain scheduler's "NEXT BREAK IN".
    pub fn label(self) -> &'static str {
        match self {
            PomodoroPhase::Work => "FOCUS TIME",
            PomodoroPhase::ShortBreak => "SHORT BREAK",
            PomodoroPhase::LongBreak => "LONG BREAK",
        }
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

    /// Total length of the current phase, in seconds — the denominator for
    /// a "time elapsed in this phase" progress display.
    pub fn phase_duration_secs(&self, cfg: &PomodoroConfig) -> u64 {
        self.phase.duration_secs(cfg)
    }

    /// `(seconds remaining, fraction of the phase elapsed)` for the current
    /// phase as of `now_epoch_secs` — what the corner countdown paints when
    /// Pomodoro mode is active. Pulled out as a pure function (rather than
    /// inlined in timer.rs's painting code) so it's unit-testable on its
    /// own: this is the exact math that used to be missing entirely, which
    /// left the countdown stuck at 00:00 once its (unrelated, plain-
    /// interval) countdown ran out instead of tracking the real phase.
    pub fn phase_progress(&self, cfg: &PomodoroConfig, now_epoch_secs: u64) -> (u64, f32) {
        let due = self.phase_due_epoch(cfg);
        let remaining = due.saturating_sub(now_epoch_secs);
        let total = self.phase_duration_secs(cfg).max(1);
        let elapsed_frac = 1.0 - (remaining as f32 / total as f32).clamp(0.0, 1.0);
        (remaining, elapsed_frac)
    }
}

/// Advances `state` to the next phase if the current phase's duration has
/// elapsed (epoch-based, consistent with `state::State`'s scheduling).
/// Returns true if the phase just changed, so a caller can show a
/// notification/overlay for the transition.
pub fn tick(state: &mut PomodoroState, cfg: &PomodoroConfig) -> bool {
    if !advance_if_due(state, cfg, now_epoch()) {
        return false;
    }
    state.save();
    true
}

/// The pure phase-advance decision behind `tick`, split out so it can be
/// unit tested without touching disk (`state.save()`) or depending on the
/// real wall clock. If `state`'s current phase has elapsed as of
/// `now_epoch_secs`, advances it to the next phase (resetting
/// `phase_started_epoch` to `now_epoch_secs`) and returns true.
fn advance_if_due(state: &mut PomodoroState, cfg: &PomodoroConfig, now_epoch_secs: u64) -> bool {
    if now_epoch_secs < state.phase_due_epoch(cfg) {
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
    state.phase_started_epoch = now_epoch_secs;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> PomodoroConfig {
        PomodoroConfig {
            work_mins: 25,
            short_break_mins: 5,
            long_break_mins: 15,
            cycles_before_long_break: 4,
        }
    }

    /// This is the exact math timer.rs's corner countdown was missing
    /// entirely before this fix — it used to compute remaining time
    /// against the plain interval scheduler even in Pomodoro mode, a
    /// schedule nothing was actually driving.
    #[test]
    fn phase_progress_at_phase_start_is_full_and_unelapsed() {
        let cfg = cfg();
        let state = PomodoroState {
            phase: PomodoroPhase::Work,
            cycles_completed: 0,
            phase_started_epoch: 1_000,
        };
        let (remaining, elapsed_frac) = state.phase_progress(&cfg, 1_000);
        assert_eq!(remaining, 25 * 60);
        assert_eq!(elapsed_frac, 0.0);
    }

    #[test]
    fn phase_progress_halfway_through() {
        let cfg = cfg();
        let state = PomodoroState {
            phase: PomodoroPhase::ShortBreak,
            cycles_completed: 1,
            phase_started_epoch: 1_000,
        };
        let half = (5 * 60) / 2;
        let (remaining, elapsed_frac) = state.phase_progress(&cfg, 1_000 + half);
        assert_eq!(remaining, 5 * 60 - half);
        assert!((elapsed_frac - 0.5).abs() < 0.01);
    }

    /// Before this fix, once the (wrong) countdown timer.rs displayed ran
    /// out, `remaining` never moved again — because it was tracking a
    /// scheduler that Pomodoro mode had bypassed. `phase_progress` must
    /// clamp cleanly to `(0, 1.0)` here, not panic or go negative, since
    /// this is exactly the "past due" instant the corner card renders
    /// while waiting for the next `tick()` (driven by tray.rs) to flip the
    /// phase and reset `phase_started_epoch`.
    #[test]
    fn phase_progress_past_due_clamps_to_zero_remaining() {
        let cfg = cfg();
        let state = PomodoroState {
            phase: PomodoroPhase::Work,
            cycles_completed: 0,
            phase_started_epoch: 1_000,
        };
        let (remaining, elapsed_frac) = state.phase_progress(&cfg, 1_000 + 25 * 60 + 500);
        assert_eq!(remaining, 0);
        assert_eq!(elapsed_frac, 1.0);
    }

    #[test]
    fn tick_cycles_work_short_break_and_long_break_in_order() {
        let cfg = cfg();
        let mut state = PomodoroState {
            phase: PomodoroPhase::Work,
            cycles_completed: 0,
            phase_started_epoch: now_epoch(),
        };

        // Force each phase to already be due, then advance one tick at a
        // time, checking the sequence: Work -> Short -> Work -> Short ->
        // Work -> Short -> Work -> Long (4th work cycle) -> Work.
        let expected = [
            PomodoroPhase::ShortBreak,
            PomodoroPhase::Work,
            PomodoroPhase::ShortBreak,
            PomodoroPhase::Work,
            PomodoroPhase::ShortBreak,
            PomodoroPhase::Work,
            PomodoroPhase::LongBreak,
            PomodoroPhase::Work,
        ];
        for want in expected {
            state.phase_started_epoch = 0; // guarantee the phase is due
            let changed = advance_if_due(&mut state, &cfg, i64::MAX as u64);
            assert!(changed, "expected a phase change to {want:?}");
            assert_eq!(state.phase, want);
        }
    }
}

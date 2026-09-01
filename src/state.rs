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
    /// Manual pause, set from the tray's "Pause" menu (or `eye-break pause`)
    /// for calls, presentations, etc. — distinct from `enabled` (a
    /// deliberate, remembered-forever off switch) and from
    /// `snooze_until_epoch` (a one-shot push-out of the *next* break only,
    /// triggered from an overlay's Skip button). `Some(epoch)` pauses until
    /// that time; `MANUAL_PAUSE_INDEFINITE` means "until manually resumed"
    /// rather than any specific time.
    #[serde(default)]
    pub manual_pause_until_epoch: Option<u64>,
    /// Number of micro breaks fired since the last long break, for the
    /// plain-interval scheduler's tiered-breaks mode (`Config::
    /// tiered_breaks_enabled`). Reset to 0 whenever a long break fires;
    /// unused (stays 0) while that mode is off.
    #[serde(default)]
    pub micro_breaks_since_long: u32,
}

/// Sentinel for `manual_pause_until_epoch` meaning "paused with no set end
/// time — resume has to be clicked/run explicitly." Far enough out that
/// nothing will ever legitimately reach it by comparison against `now_epoch`.
pub const MANUAL_PAUSE_INDEFINITE: u64 = u64::MAX;

impl Default for State {
    fn default() -> Self {
        Self {
            last_break_epoch: now_epoch(),
            snooze_until_epoch: None,
            dismiss_token: 0,
            manual_pause_until_epoch: None,
            micro_breaks_since_long: 0,
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
    crate::paths::config_file("state.json")
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

    /// Whether a manual pause (tray "Pause" menu / `eye-break pause`) is
    /// currently in effect.
    pub fn is_manually_paused(&self) -> bool {
        self.manual_pause_until_epoch.is_some_and(|u| u > now_epoch())
    }

    /// Pushes the schedule's baseline to "now" and clears any pending Skip
    /// snooze — shared by every "the user just came back" transition (idle
    /// -> active, a manual pause starting/ending) so the next break is a
    /// full interval away instead of ambushing whoever just returned.
    ///
    /// Clearing the snooze matters here too, not just the baseline:
    /// `next_break_epoch` prefers a live `snooze_until_epoch` over
    /// `last_break_epoch` outright, so a snooze started shortly before a
    /// (generally longer) idle stretch or pause would otherwise keep
    /// overriding the just-reset baseline — silently outliving whatever
    /// reset it and firing on its own old schedule instead of a fresh one.
    pub fn reset_schedule_baseline(&mut self) {
        self.last_break_epoch = now_epoch();
        self.snooze_until_epoch = None;
    }

    /// Starts (or extends/shortens) a manual pause.
    pub fn start_manual_pause(&mut self, until_epoch: u64) {
        self.manual_pause_until_epoch = Some(until_epoch);
        self.reset_schedule_baseline();
    }

    /// Ends a manual pause immediately (the tray's "Resume now" / `eye-break
    /// resume`, or a timed pause simply running out with nobody around to
    /// click anything).
    pub fn end_manual_pause(&mut self) {
        self.manual_pause_until_epoch = None;
        self.reset_schedule_baseline();
    }

    /// Whether tiered-breaks scheduling's *next* break should be a long
    /// break rather than a micro break — true once `micro_breaks_since_long`
    /// has caught up to `micro_breaks_before_long`. Shared by tray.rs's
    /// scheduler (to decide what `trigger_break` actually shows) and
    /// timer.rs (to label the corner countdown correctly ahead of time), so
    /// the two can't drift out of sync with each other.
    pub fn next_break_is_long(&self, micro_breaks_before_long: u32) -> bool {
        self.micro_breaks_since_long >= micro_breaks_before_long.max(1)
    }

    /// Advances tiered-breaks bookkeeping for a break that just fired:
    /// resets the counter after a long break, otherwise increments it.
    /// Call alongside the `last_break_epoch`/`snooze_until_epoch` update in
    /// `trigger_break`, using the same `micro_breaks_before_long` the
    /// break's own long/short decision was already made from.
    pub fn advance_tiered_breaks(&mut self, micro_breaks_before_long: u32) {
        if self.next_break_is_long(micro_breaks_before_long) {
            self.micro_breaks_since_long = 0;
        } else {
            self.micro_breaks_since_long += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiered_breaks_cycle_micro_then_long() {
        let mut st = State::default();
        // Three micro breaks, then a long one, repeating — matches the
        // default `micro_breaks_before_long` of 3.
        let before_long = 3;
        let expected_is_long = [false, false, false, true, false, false, false, true];
        for &want_long in &expected_is_long {
            assert_eq!(st.next_break_is_long(before_long), want_long);
            st.advance_tiered_breaks(before_long);
        }
    }

    #[test]
    fn tiered_breaks_disabled_stays_micro() {
        // With the counter never advanced (mode off), every break reads as
        // a micro break regardless of the threshold.
        let st = State::default();
        assert!(!st.next_break_is_long(3));
        assert!(!st.next_break_is_long(1));
    }
}

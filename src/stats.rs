use crate::state::now_epoch;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Maximum number of daily entries retained in the usage log; older entries
/// are trimmed on save.
const MAX_DAYS: usize = 90;

/// One day's worth of recorded active-usage time.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DailyUsage {
    /// Calendar date this entry covers, formatted `YYYY-MM-DD` (local time —
    /// see [`today_string`] for how "local" is derived).
    pub date: String,
    /// Accumulated active seconds recorded for this date.
    pub active_secs: u64,
    /// Accumulated active seconds recorded for this date, broken down by
    /// hour-of-day (UTC, index 0 = 00:00-00:59, ..., 23 = 23:00-23:59) — the
    /// "most active hours" wheel's data source. `#[serde(default)]` so
    /// entries recorded before this field existed still deserialize.
    #[serde(default = "default_hourly_secs")]
    pub hourly_secs: [u64; 24],
}

fn default_hourly_secs() -> [u64; 24] {
    [0; 24]
}

/// Rolling log of daily device-usage time, persisted as JSON next to
/// `config.json` / `state.json` (same `directories` layout as `state.rs`).
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UsageLog {
    /// Oldest-first list of per-day usage entries (at most [`MAX_DAYS`] long).
    pub days: Vec<DailyUsage>,
}

fn usage_log_path() -> PathBuf {
    crate::paths::config_file("usage_log.json")
}

impl UsageLog {
    /// Load the usage log from disk, creating an empty one if none exists yet.
    pub fn load() -> Self {
        let path = usage_log_path();
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => {
                let log = UsageLog::default();
                log.save();
                log
            }
        }
    }

    /// Persist the usage log to disk, trimming to the most recent
    /// [`MAX_DAYS`] entries first.
    pub fn save(&self) {
        let mut log = self.clone();
        if log.days.len() > MAX_DAYS {
            let excess = log.days.len() - MAX_DAYS;
            log.days.drain(0..excess);
        }
        let path = usage_log_path();
        if let Ok(s) = serde_json::to_string_pretty(&log) {
            let _ = std::fs::write(path, s);
        }
    }
}

/// Days since the Unix epoch (1970-01-01) for the given epoch-seconds
/// timestamp, using UTC. Used as the basis for calendar-date math without
/// pulling in a date/time crate.
fn days_since_epoch(epoch_secs: u64) -> i64 {
    (epoch_secs / 86_400) as i64
}

/// Converts a day count since 1970-01-01 (UTC, proleptic Gregorian) into a
/// `YYYY-MM-DD` string, using the well-known civil-from-days algorithm
/// (Howard Hinnant's `civil_from_days`).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Returns today's calendar date as `YYYY-MM-DD`.
///
/// Note: computed from UTC (no timezone crate dependency), so on machines
/// far from UTC the "day" boundary may not line up exactly with local
/// midnight. Good enough for daily usage bucketing; revisit if precise
/// local-midnight rollover matters later.
pub fn today_string() -> String {
    date_string_for_epoch(now_epoch())
}

fn date_string_for_epoch(epoch_secs: u64) -> String {
    let (y, m, d) = civil_from_days(days_since_epoch(epoch_secs));
    format!("{y:04}-{m:02}-{d:02}")
}

/// Accumulates `elapsed_secs` of active usage into today's entry, rolling
/// over to a new [`DailyUsage`] if the date has changed since the last
/// recorded entry. Call this periodically from the scheduler loop (e.g.
/// every 500ms-1s tick) with the elapsed time since the previous tick.
///
/// Does not save to disk; call [`UsageLog::save`] as needed (e.g. once per
/// tick, or on a slower cadence).
pub fn record_tick(log: &mut UsageLog, elapsed_secs: u64) {
    let today = today_string();
    let hour = ((now_epoch() % 86_400) / 3600) as usize;
    match log.days.last_mut() {
        Some(last) if last.date == today => {
            last.active_secs = last.active_secs.saturating_add(elapsed_secs);
            last.hourly_secs[hour] = last.hourly_secs[hour].saturating_add(elapsed_secs);
        }
        _ => {
            let mut hourly_secs = [0u64; 24];
            hourly_secs[hour] = elapsed_secs;
            log.days.push(DailyUsage {
                date: today,
                active_secs: elapsed_secs,
                hourly_secs,
            });
        }
    }
}

/// Returns the accumulated active seconds recorded for today, or 0 if no
/// entry exists yet.
pub fn today_usage_secs(log: &UsageLog) -> u64 {
    let today = today_string();
    log.days
        .iter()
        .rev()
        .find(|d| d.date == today)
        .map(|d| d.active_secs)
        .unwrap_or(0)
}

/// Returns up to the last `n` days of usage entries (oldest first), for use
/// in the usage chart.
pub fn usage_last_n_days(log: &UsageLog, n: usize) -> Vec<DailyUsage> {
    let len = log.days.len();
    let start = len.saturating_sub(n);
    log.days[start..].to_vec()
}

/// Returns each hour-of-day's share (0.0-1.0) of that hour's busiest peer
/// across the last `n` days — the "most active hours" wheel's data.
/// Normalized against the single busiest hour in the period (rather than
/// each day's own total) so the wheel reads as "when", not "how much
/// overall", matching the design's per-hour intensity wedges.
pub fn hourly_activity_fractions(log: &UsageLog, n: usize) -> [f32; 24] {
    let days = usage_last_n_days(log, n);
    let mut totals = [0u64; 24];
    for day in &days {
        for (h, secs) in day.hourly_secs.iter().enumerate() {
            totals[h] = totals[h].saturating_add(*secs);
        }
    }
    let max = totals.iter().copied().max().unwrap_or(0).max(1);
    let mut fractions = [0.0f32; 24];
    for (h, total) in totals.iter().enumerate() {
        fractions[h] = *total as f32 / max as f32;
    }
    fractions
}

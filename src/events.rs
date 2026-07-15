//! Domain model: the four event kinds `sysevents` reports, and the logic
//! that assembles them for a given day from the lower-level `journal`
//! module.

use crate::date::DayRange;
use crate::error::AppError;
use crate::journal;
use crate::patterns;
use chrono::{DateTime, Local};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Boot,
    Shutdown,
    Suspend,
    Resume,
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EventKind::Boot => "BOOT",
            EventKind::Shutdown => "SHUTDOWN",
            EventKind::Suspend => "SUSPEND",
            EventKind::Resume => "RESUME",
        };
        write!(f, "{s}")
    }
}

impl EventKind {
    /// ANSI colour code used in human-readable output (ignored in JSON
    /// mode and when `--no-color`/non-tty output is active).
    pub fn ansi_color(&self) -> &'static str {
        match self {
            EventKind::Boot => "\x1b[32m",     // green
            EventKind::Shutdown => "\x1b[31m", // red
            EventKind::Suspend => "\x1b[33m",  // yellow
            EventKind::Resume => "\x1b[36m",   // cyan
        }
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub kind: EventKind,
    pub timestamp: DateTime<Local>,
    pub detail: Option<String>,
}

/// Collects every boot/shutdown/suspend/resume event that falls within
/// `range`, sorted in chronological order.
pub fn collect(range: &DayRange) -> Result<Vec<Event>, AppError> {
    let mut events = Vec::new();

    events.extend(collect_boot_and_shutdown(range)?);
    events.extend(collect_suspend_resume(range)?);

    events.sort_by_key(|e| e.timestamp);
    Ok(events)
}

fn within_range(ts: DateTime<Local>, range: &DayRange) -> bool {
    let start = range.date.and_hms_opt(0, 0, 0).unwrap();
    let end = start + chrono::Duration::days(1);
    let ts_naive = ts.naive_local();
    ts_naive >= start && ts_naive < end
}

/// BOOT events come from the first journal entry of each boot; SHUTDOWN
/// events (a reliable proxy, not a guarantee against crashes/hard resets)
/// come from the last journal entry of each *past* boot, i.e. excluding
/// the boot the tool is currently running under.
fn collect_boot_and_shutdown(range: &DayRange) -> Result<Vec<Event>, AppError> {
    let boot_ids = journal::list_boot_ids()?;
    let current_boot = journal::current_boot_id().ok();
    let mut events = Vec::new();

    for boot_id in &boot_ids {
        if let Some((ts, _)) = journal::boot_boundary_timestamp(boot_id, false)? {
            if within_range(ts, range) {
                events.push(Event {
                    kind: EventKind::Boot,
                    timestamp: ts,
                    detail: Some(format!("boot id {boot_id}")),
                });
            }
        }

        let is_current = current_boot.as_deref() == Some(boot_id.as_str());
        if is_current {
            continue; // the running boot has not shut down yet
        }

        if let Some((ts, _)) = journal::boot_boundary_timestamp(boot_id, true)? {
            if within_range(ts, range) {
                events.push(Event {
                    kind: EventKind::Shutdown,
                    timestamp: ts,
                    detail: Some(format!("boot id {boot_id}")),
                });
            }
        }
    }

    Ok(events)
}

fn collect_suspend_resume(range: &DayRange) -> Result<Vec<Event>, AppError> {
    let mut events = Vec::new();

    let suspend_pattern = patterns::alternation(patterns::SUSPEND_PATTERNS);
    for (ts, msg) in journal::grep_range(&suspend_pattern, &range.since, &range.until)? {
        events.push(Event { kind: EventKind::Suspend, timestamp: ts, detail: msg });
    }

    let resume_pattern = patterns::alternation(patterns::RESUME_PATTERNS);
    for (ts, msg) in journal::grep_range(&resume_pattern, &range.since, &range.until)? {
        events.push(Event { kind: EventKind::Resume, timestamp: ts, detail: msg });
    }

    deduplicate_adjacent(&mut events);
    Ok(events)
}

/// Removes duplicate SUSPEND/RESUME entries that represent the same
/// real-world transition logged twice by different sources (e.g.
/// systemd's "Reached target Sleep." and the kernel's "PM: suspend
/// entry" are typically emitted within the same second for one physical
/// suspend). Comparison is done at one-second granularity, matching the
/// precision shown in the human-readable table, so what looks like one
/// event in the output is never silently split into duplicate lines.
/// The first (chronologically earliest) message for each kind/second is
/// kept; the rest are dropped.
fn deduplicate_adjacent(events: &mut Vec<Event>) {
    events.sort_by_key(|e| (e.timestamp, e.kind as u8 as i32));
    events.dedup_by(|a, b| a.kind == b.kind && a.timestamp.timestamp() == b.timestamp.timestamp());
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone};

    fn range() -> DayRange {
        DayRange {
            date: NaiveDate::from_ymd_opt(2026, 7, 15).unwrap(),
            since: "2026-07-15 00:00:00".into(),
            until: "2026-07-16 00:00:00".into(),
        }
    }

    #[test]
    fn within_range_accepts_boundaries_correctly() {
        let r = range();
        let midday = Local.with_ymd_and_hms(2026, 7, 15, 12, 0, 0).unwrap();
        let just_before_next_day = Local.with_ymd_and_hms(2026, 7, 15, 23, 59, 59).unwrap();
        let next_day = Local.with_ymd_and_hms(2026, 7, 16, 0, 0, 0).unwrap();
        assert!(within_range(midday, &r));
        assert!(within_range(just_before_next_day, &r));
        assert!(!within_range(next_day, &r));
    }
}

//! Date parsing and day-range computation.
//!
//! `journalctl --since/--until` accepts free-form date strings; we compute
//! an explicit `[start, end)` boundary ourselves so that behaviour does not
//! depend on journalctl's own (locale-sensitive) date parser, and so we can
//! validate the user's input strictly before ever spawning a subprocess.

use crate::cli::DateSource;
use crate::error::AppError;
use chrono::{Duration, Local, NaiveDate};

/// Inclusive-start, exclusive-end boundary of one calendar day, expressed
/// in the format journalctl expects for `--since` / `--until`
/// (`YYYY-MM-DD HH:MM:SS`), plus the parsed date for display purposes.
#[derive(Debug, Clone)]
pub struct DayRange {
    pub date: NaiveDate,
    pub since: String,
    pub until: String,
}

/// Resolves a [`DateSource`] into a concrete, validated [`DayRange`].
pub fn resolve(source: &DateSource) -> Result<DayRange, AppError> {
    let date = match source {
        DateSource::Today => Local::now().date_naive(),
        DateSource::Yesterday => Local::now().date_naive() - Duration::days(1),
        DateSource::Explicit(raw) => parse_date(raw)?,
    };
    Ok(day_range(date))
}

/// Strictly parses `YYYY-MM-DD`, rejecting anything else (including valid
/// but ambiguous alternative formats) so that error messages stay precise.
fn parse_date(input: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(input.trim(), "%Y-%m-%d").map_err(|_| AppError::InvalidDate {
        input: input.to_string(),
    })
}

fn day_range(date: NaiveDate) -> DayRange {
    let next = date + Duration::days(1);
    DayRange {
        date,
        since: format!("{} 00:00:00", date.format("%Y-%m-%d")),
        until: format!("{} 00:00:00", next.format("%Y-%m-%d")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_date() {
        let d = parse_date("2026-07-15").unwrap();
        assert_eq!(d.to_string(), "2026-07-15");
    }

    #[test]
    fn rejects_malformed_date() {
        assert!(parse_date("15-07-2026").is_err());
        assert!(parse_date("2026/07/15").is_err());
        assert!(parse_date("not-a-date").is_err());
        assert!(parse_date("2026-13-40").is_err());
    }

    #[test]
    fn day_range_spans_exactly_24h_boundaries() {
        let date = NaiveDate::from_ymd_opt(2026, 7, 15).unwrap();
        let range = day_range(date);
        assert_eq!(range.since, "2026-07-15 00:00:00");
        assert_eq!(range.until, "2026-07-16 00:00:00");
    }
}

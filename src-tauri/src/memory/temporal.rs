//! Temporal reasoning queries — Chunk 17.3.
//!
//! Parses natural-language time-range expressions ("last week", "last month",
//! "since 2026-04-01", "last 30 days") into `(start_ms, end_ms)` intervals,
//! then filters memories by `created_at` within that range.
//!
//! **No external crate** — uses only `std::time` for "now" and manual
//! calendar math for relative ranges.

use serde::{Deserialize, Serialize};

/// A resolved time range in Unix milliseconds.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimeRange {
    /// Inclusive lower bound (Unix ms).
    pub start_ms: i64,
    /// Exclusive upper bound (Unix ms).
    pub end_ms: i64,
}

// ── Constants ────────────────────────────────────────────────────────

const MS_PER_SEC: i64 = 1_000;
const MS_PER_MIN: i64 = 60 * MS_PER_SEC;
const MS_PER_HOUR: i64 = 60 * MS_PER_MIN;
const MS_PER_DAY: i64 = 24 * MS_PER_HOUR;
const MS_PER_WEEK: i64 = 7 * MS_PER_DAY;

// ── Public API ───────────────────────────────────────────────────────

/// Try to extract a `TimeRange` from a natural-language question.
///
/// Returns `None` if no recognisable time expression is found.
/// All relative ranges are computed against `now_ms`.
///
/// Supported patterns:
/// - `"last N days/weeks/months/hours"` — relative window ending now.
/// - `"last day/week/month/year"` — shorthand for N=1.
/// - `"today"` — midnight-to-now (UTC).
/// - `"yesterday"` — midnight-to-midnight (UTC).
/// - `"since YYYY-MM-DD"` — from that date to now.
/// - `"before YYYY-MM-DD"` — from epoch to that date.
/// - `"between YYYY-MM-DD and YYYY-MM-DD"` — explicit interval.
pub fn parse_time_range(question: &str, now_ms: i64) -> Option<TimeRange> {
    let lower = question.to_ascii_lowercase();

    // "between YYYY-MM-DD and YYYY-MM-DD"
    if let Some(range) = try_between(&lower) {
        return Some(range);
    }

    // "since YYYY-MM-DD"
    if let Some(range) = try_since(&lower, now_ms) {
        return Some(range);
    }

    // "before YYYY-MM-DD"
    if let Some(range) = try_before(&lower) {
        return Some(range);
    }

    // "last N days/weeks/months/hours"
    if let Some(range) = try_last_n(&lower, now_ms) {
        return Some(range);
    }

    // "last day/week/month/year" (no number)
    if let Some(range) = try_last_unit(&lower, now_ms) {
        return Some(range);
    }

    // "today"
    if lower.contains("today") {
        let start = midnight_utc(now_ms);
        return Some(TimeRange {
            start_ms: start,
            end_ms: now_ms,
        });
    }

    // "yesterday"
    if lower.contains("yesterday") {
        let today_start = midnight_utc(now_ms);
        return Some(TimeRange {
            start_ms: today_start - MS_PER_DAY,
            end_ms: today_start,
        });
    }

    None
}

// ── Parsers ──────────────────────────────────────────────────────────

fn try_between(lower: &str) -> Option<TimeRange> {
    // "between 2026-01-01 and 2026-04-01"
    let idx = lower.find("between ")?;
    let rest = &lower[idx + 8..];
    let and_idx = rest.find(" and ")?;
    let date_a = rest[..and_idx].trim();
    let date_b = rest[and_idx + 5..].trim();
    // Take only the first 10 chars (YYYY-MM-DD) from each
    let a_str = if date_a.len() >= 10 {
        &date_a[..10]
    } else {
        date_a
    };
    let b_str = if date_b.len() >= 10 {
        &date_b[..10]
    } else {
        date_b
    };
    let a = parse_ymd(a_str)?;
    let b = parse_ymd(b_str)?;
    Some(TimeRange {
        start_ms: a.min(b),
        end_ms: a.max(b) + MS_PER_DAY, // inclusive end date
    })
}

fn try_since(lower: &str, now_ms: i64) -> Option<TimeRange> {
    let idx = lower.find("since ")?;
    let rest = &lower[idx + 6..];
    let raw = rest.split_whitespace().next()?;
    let date_str = strip_punct(raw);
    let d_str = if date_str.len() >= 10 {
        &date_str[..10]
    } else {
        date_str
    };

    // Try "since april", "since march" etc.
    if let Some(month_ms) = try_month_name(d_str, now_ms) {
        return Some(TimeRange {
            start_ms: month_ms,
            end_ms: now_ms,
        });
    }

    let start = parse_ymd(d_str)?;
    Some(TimeRange {
        start_ms: start,
        end_ms: now_ms,
    })
}

fn try_before(lower: &str) -> Option<TimeRange> {
    let idx = lower.find("before ")?;
    let rest = &lower[idx + 7..];
    let raw = rest.split_whitespace().next()?;
    let date_str = strip_punct(raw);
    let d_str = if date_str.len() >= 10 {
        &date_str[..10]
    } else {
        date_str
    };
    let end = parse_ymd(d_str)?;
    Some(TimeRange {
        start_ms: 0,
        end_ms: end,
    })
}

fn try_last_n(lower: &str, now_ms: i64) -> Option<TimeRange> {
    // Match "last N days|weeks|months|hours|minutes"
    let idx = lower.find("last ")?;
    let rest = &lower[idx + 5..];
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let n: i64 = parts[0].parse().ok()?;
    if n <= 0 {
        return None;
    }
    let unit = strip_punct(parts[1]).trim_end_matches('s'); // "days?" → "day"
    let duration_ms = unit_to_ms(unit)? * n;
    Some(TimeRange {
        start_ms: now_ms - duration_ms,
        end_ms: now_ms,
    })
}

fn try_last_unit(lower: &str, now_ms: i64) -> Option<TimeRange> {
    let idx = lower.find("last ")?;
    let rest = &lower[idx + 5..];
    let raw = rest.split_whitespace().next()?;
    let unit = strip_punct(raw).trim_end_matches('s');
    // Only match if it's NOT a number (that's handled by try_last_n)
    if unit.chars().next()?.is_ascii_digit() {
        return None;
    }
    let duration_ms = unit_to_ms(unit)?;
    Some(TimeRange {
        start_ms: now_ms - duration_ms,
        end_ms: now_ms,
    })
}

/// Strip common trailing punctuation and possessives from a token.
fn strip_punct(s: &str) -> &str {
    // Strip possessive "'s" first
    let s = s.strip_suffix("'s").unwrap_or(s);
    s.trim_end_matches(|c: char| !c.is_ascii_alphanumeric())
}

fn unit_to_ms(unit: &str) -> Option<i64> {
    match unit {
        "minute" => Some(MS_PER_MIN),
        "hour" => Some(MS_PER_HOUR),
        "day" => Some(MS_PER_DAY),
        "week" => Some(MS_PER_WEEK),
        "month" => Some(30 * MS_PER_DAY),
        "year" => Some(365 * MS_PER_DAY),
        _ => None,
    }
}

/// Try to parse a month name and return the start of that month in the
/// current (or previous) year relative to `now_ms`.
fn try_month_name(s: &str, now_ms: i64) -> Option<i64> {
    let month = match s {
        "january" | "jan" => 1,
        "february" | "feb" => 2,
        "march" | "mar" => 3,
        "april" | "apr" => 4,
        "may" => 5,
        "june" | "jun" => 6,
        "july" | "jul" => 7,
        "august" | "aug" => 8,
        "september" | "sep" => 9,
        "october" | "oct" => 10,
        "november" | "nov" => 11,
        "december" | "dec" => 12,
        _ => return None,
    };

    // Derive year from now_ms
    let (year, current_month, _) = ms_to_ymd(now_ms);
    // Use current year if month <= current month, else previous year
    let target_year = if month <= current_month {
        year
    } else {
        year - 1
    };
    Some(ymd_to_ms(target_year, month, 1))
}

// ── Calendar helpers (pure, no external crate) ───────────────────────

/// Parse "YYYY-MM-DD" → Unix ms at midnight UTC.
fn parse_ymd(s: &str) -> Option<i64> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i64 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || y < 1970 {
        return None;
    }
    Some(ymd_to_ms(y, m, d))
}

/// Convert year/month/day → Unix ms at midnight UTC.
/// Uses the inverse of Howard Hinnant's civil_from_days algorithm.
fn ymd_to_ms(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 { month + 9 } else { month - 3 };
    let era = if y >= 0 { y / 400 } else { (y - 399) / 400 };
    let yoe = (y - era * 400) as u32;
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let day_count = era * 146_097 + doe as i64 - 719_468;
    day_count * MS_PER_DAY
}

/// Convert Unix ms → (year, month, day) UTC.
fn ms_to_ymd(ms: i64) -> (i64, u32, u32) {
    let day_count = ms.div_euclid(MS_PER_DAY) + 719_468;
    let era = if day_count >= 0 {
        day_count / 146_097
    } else {
        (day_count - 146_096) / 146_097
    };
    let doe = (day_count - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

/// Midnight UTC for the day containing `ms`.
fn midnight_utc(ms: i64) -> i64 {
    ms - ms.rem_euclid(MS_PER_DAY)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-04-25T12:00:00Z in ms
    const NOW: i64 = 1_777_118_400_000;

    // ── parse_ymd / ymd_to_ms ────────────────────────────────────────

    #[test]
    fn ymd_epoch() {
        assert_eq!(ymd_to_ms(1970, 1, 1), 0);
    }

    #[test]
    fn ymd_known_date() {
        // 2024-01-01T00:00:00Z = 1704067200000
        assert_eq!(ymd_to_ms(2024, 1, 1), 1_704_067_200_000);
    }

    #[test]
    fn ms_to_ymd_roundtrip() {
        let (y, m, d) = ms_to_ymd(ymd_to_ms(2026, 4, 25));
        assert_eq!((y, m, d), (2026, 4, 25));
    }

    #[test]
    fn ms_to_ymd_epoch() {
        let (y, m, d) = ms_to_ymd(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    // ── parse_time_range ─────────────────────────────────────────────

    #[test]
    fn last_7_days() {
        let r = parse_time_range("what did I learn in the last 7 days?", NOW).unwrap();
        assert_eq!(r.end_ms, NOW);
        assert_eq!(r.start_ms, NOW - 7 * MS_PER_DAY);
    }

    #[test]
    fn last_week() {
        let r = parse_time_range("memories from last week", NOW).unwrap();
        assert_eq!(r.end_ms, NOW);
        assert_eq!(r.start_ms, NOW - MS_PER_WEEK);
    }

    #[test]
    fn last_month() {
        let r = parse_time_range("show me last month's entries", NOW).unwrap();
        assert_eq!(r.end_ms, NOW);
        assert_eq!(r.start_ms, NOW - 30 * MS_PER_DAY);
    }

    #[test]
    fn last_3_months() {
        let r = parse_time_range("what happened in the last 3 months?", NOW).unwrap();
        assert_eq!(r.start_ms, NOW - 90 * MS_PER_DAY);
    }

    #[test]
    fn since_date() {
        let r = parse_time_range("what did I learn since 2026-04-01?", NOW).unwrap();
        assert_eq!(r.start_ms, ymd_to_ms(2026, 4, 1));
        assert_eq!(r.end_ms, NOW);
    }

    #[test]
    fn since_month_name() {
        let r = parse_time_range("have my preferences shifted since april?", NOW).unwrap();
        assert_eq!(r.start_ms, ymd_to_ms(2026, 4, 1));
        assert_eq!(r.end_ms, NOW);
    }

    #[test]
    fn before_date() {
        let r = parse_time_range("what was stored before 2026-01-01?", NOW).unwrap();
        assert_eq!(r.start_ms, 0);
        assert_eq!(r.end_ms, ymd_to_ms(2026, 1, 1));
    }

    #[test]
    fn between_dates() {
        let r = parse_time_range("between 2026-01-01 and 2026-03-31 what changed?", NOW).unwrap();
        assert_eq!(r.start_ms, ymd_to_ms(2026, 1, 1));
        assert_eq!(r.end_ms, ymd_to_ms(2026, 3, 31) + MS_PER_DAY);
    }

    #[test]
    fn today() {
        let r = parse_time_range("what did I add today?", NOW).unwrap();
        assert_eq!(r.start_ms, midnight_utc(NOW));
        assert_eq!(r.end_ms, NOW);
    }

    #[test]
    fn yesterday() {
        let r = parse_time_range("show yesterday's memories", NOW).unwrap();
        let today_start = midnight_utc(NOW);
        assert_eq!(r.start_ms, today_start - MS_PER_DAY);
        assert_eq!(r.end_ms, today_start);
    }

    #[test]
    fn no_time_expression() {
        assert!(parse_time_range("tell me about Rust", NOW).is_none());
    }

    #[test]
    fn last_year() {
        let r = parse_time_range("what did I learn last year?", NOW).unwrap();
        assert_eq!(r.start_ms, NOW - 365 * MS_PER_DAY);
    }

    #[test]
    fn since_future_month_wraps_to_previous_year() {
        // NOW is April 2026; "since december" → December 2025
        let r = parse_time_range("since december", NOW).unwrap();
        assert_eq!(r.start_ms, ymd_to_ms(2025, 12, 1));
    }

    // ── midnight_utc ─────────────────────────────────────────────────

    #[test]
    fn midnight_of_epoch() {
        assert_eq!(midnight_utc(12345), 0);
    }

    #[test]
    fn midnight_of_now() {
        let m = midnight_utc(NOW);
        assert_eq!(m, ymd_to_ms(2026, 4, 25));
    }
}

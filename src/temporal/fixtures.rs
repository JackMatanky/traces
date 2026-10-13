//! Shared builders for the temporal test suites.
//!
//! Contains only value builders; assertions live in the owning test modules.

use chrono::{NaiveDate, TimeZone as _, Utc};

use super::{DateTimeValue, DateValue};

/// Builds a [`DateValue`] from calendar fields.
///
/// # Panics
///
/// Panics if `year`, `month`, and `day` do not form a valid calendar date;
/// fixtures state dates that are valid by construction.
pub(super) fn ymd(year: i32, month: u32, day: u32) -> DateValue {
    DateValue::from(
        NaiveDate::from_ymd_opt(year, month, day)
            .expect("fixture fields form a valid calendar date"),
    )
}

/// Builds the fixed reference date-time `2026-07-29 14:30:05 UTC`.
///
/// # Panics
///
/// Panics only if the constant fixture ceases to be a valid instant; the
/// literal is unambiguous (UTC carries no fold or gap).
pub(super) fn fixed_datetime() -> DateTimeValue {
    DateTimeValue::from(
        Utc.with_ymd_and_hms(2026, 7, 29, 14, 30, 5)
            .single()
            .expect("2026-07-29 14:30:05 UTC is a valid, unambiguous instant"),
    )
}

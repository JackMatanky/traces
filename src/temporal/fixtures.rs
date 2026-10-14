//! Shared builders for the temporal test suites.
//!
//! Contains only value builders; assertions live in the owning test modules.

use super::{DateTimeValue, DateValue};

/// Builds a [`DateValue`] from calendar fields.
///
/// # Panics
///
/// Panics if `year`, `month`, and `day` do not form a valid calendar date;
/// fixtures state dates that are valid by construction.
pub(super) fn ymd(year: i32, month: u32, day: u32) -> DateValue {
    DateValue::from_ymd(year, month, day)
        .expect("fixture fields form a valid calendar date")
}

/// Builds the fixed reference date-time `2026-07-29 14:30:05 UTC`.
///
/// # Panics
///
/// Panics only if the constant fixture ceases to be a valid date-time; the
/// literal is unambiguous (UTC carries no fold or gap).
pub(super) fn fixed_datetime() -> DateTimeValue {
    DateTimeValue::parse_iso("2026-07-29T14:30:05Z")
        .expect("fixture timestamp is valid and in Jiff's range")
}

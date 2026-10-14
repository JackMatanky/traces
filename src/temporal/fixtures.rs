//! Defines shared builders for the temporal test suites.
//!
//! Contains only value builders; assertions live in the owning test modules.

use jiff::tz::{Offset, TimeZone};

use super::{DateTimeValue, DateValue};

/// Builds a fixed-offset zone without consulting the host time-zone database.
///
/// # Panics
///
/// Panics if `seconds` is outside Jiff's supported offset range.
pub(crate) fn fixed_offset_zone(seconds: i32) -> TimeZone {
    let offset = Offset::from_seconds(seconds)
        .expect("fixture offset is within Jiff's supported range");
    TimeZone::fixed(offset)
}

/// Builds a POSIX-rule fixture for modern New York daylight-saving rules.
pub(crate) fn new_york_zone() -> TimeZone {
    TimeZone::posix("EST5EDT,M3.2.0,M11.1.0")
        .expect("New York POSIX fixture is valid")
}

/// Builds a POSIX-rule fixture for modern London daylight-saving rules.
pub(crate) fn london_zone() -> TimeZone {
    TimeZone::posix("GMT0BST,M3.5.0/1,M10.5.0/2")
        .expect("London POSIX fixture is valid")
}

/// Builds a POSIX-rule fixture for Lord Howe's half-hour daylight-saving rules.
pub(crate) fn lord_howe_zone() -> TimeZone {
    TimeZone::posix("LHST-10:30LHDT-11,M10.1.0,M4.1.0")
        .expect("Lord Howe POSIX fixture is valid")
}

/// Builds a POSIX-rule fixture whose transition occurs at local midnight.
pub(crate) fn midnight_gap_zone() -> TimeZone {
    TimeZone::posix("STD0DST,M3.2.0/0,M11.1.0/0")
        .expect("midnight-gap POSIX fixture is valid")
}

/// Builds a [`DateValue`] from calendar fields.
///
/// # Panics
///
/// Panics if `year`, `month`, and `day` do not form a valid calendar date;
/// fixtures state dates that are valid by construction.
pub(crate) fn ymd(year: i32, month: u32, day: u32) -> DateValue {
    DateValue::from_ymd(year, month, day)
        .expect("fixture fields form a valid calendar date")
}

/// Builds the fixed reference date-time `2026-07-29 14:30:05 UTC`.
///
/// # Panics
///
/// Panics only if the constant fixture ceases to be a valid date-time; the
/// literal is unambiguous (UTC carries no fold or gap).
pub(crate) fn fixed_datetime() -> DateTimeValue {
    DateTimeValue::parse_iso("2026-07-29T14:30:05Z")
        .expect("fixture timestamp is valid and in Jiff's range")
}

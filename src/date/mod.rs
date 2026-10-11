//! ISO-8601 and RFC-3339 date and date-time parsing, formatting, and
//! arithmetic.
//!
//! Recognition entry points are [`DateValue::classify`] and
//! [`DateValue::parse_prefix`]. The strict typed parsers
//! [`DateValue::parse_iso`] and [`DateTimeValue::parse_iso`] back their
//! respective [`FromStr`] implementations.
//!
//! # Clock doctrine
//!
//! A naive datetime means what a human means: it is read in the process's local
//! zone and stored as UTC (see [`resolve_local`]); a date-only value stays
//! a zone-free civil date. Instants render as the local wall clock and travel
//! as UTC for storage and comparison.
//!
//! # Key types
//!
//! - [`DateValue`] - Parsed calendar date with no time-of-day component.
//! - [`DateTimeValue`] - Parsed UTC date-time instant.
//! - [`DateShape`] - Format grammar for calendar date recognition.
//! - [`DateTimeShape`] - Format grammar for date-time recognition.
//! - [`DatePoint`] - Wall-clock/instant pair measured by the calendar owner.
//! - [`Difference`] - Result of a calendar-owner difference measurement.
//! - [`DateError`] - Error type for parsing, local-zone resolution, and
//!   calendar arithmetic failures.
//!
//! # Calendar owner
//!
//! [`DateValue::shift`], [`DateValue::apply`], [`DateTimeValue::shift`], and
//! [`DateTimeValue::apply`] implement typed arithmetic. [`DatePoint::shift`],
//! [`DatePoint::apply`], [`DatePoint::diff`], [`DatePoint::month_boundary`],
//! [`DatePoint::week_boundary`], [`DatePoint::year_boundary`], and
//! [`DatePoint::weekday_point`] handle precision-aware calendar operations.

mod calendar;
mod parse;
mod value;

pub(crate) use calendar::Difference;
pub(crate) use parse::{DateShape, DateTimeShape};
pub(crate) use value::{
    DATE_PATTERN, DATETIME_PATTERN, DatePoint, Precision, RecognizedDate,
    RecognizedDateValue,
};
pub use value::{DateError, DateTimeValue, DateValue};

#[cfg(test)]
mod fixtures {
    use chrono::{TimeZone as _, Utc};

    use super::DateTimeValue;

    pub(super) fn fixed_datetime() -> DateTimeValue {
        DateTimeValue(
            Utc.with_ymd_and_hms(2026, 7, 29, 14, 30, 5).single().expect(
                "2026-07-29 14:30:05 UTC is a valid, unambiguous instant",
            ),
        )
    }
}

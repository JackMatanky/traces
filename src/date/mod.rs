//! ISO-8601 and RFC-3339 date and date-time parsing, formatting, and
//! arithmetic.
//!
//! Recognition entry points are [`Point::parse`], [`Point::parse_prefix`], and
//! [`Point::parse_with_pattern`]. The strict typed parsers
//! [`DateValue::parse_iso`] and [`DateTimeValue::parse_iso`] back their
//! respective [`FromStr`] implementations.
//!
//! # Clock doctrine
//!
//! A naive datetime means what a human means: it is read in the process's local
//! zone and stored as UTC (see [`resolve_local`]); a date-only value stays a
//! zone-free civil date. Instants render as the local wall clock and travel as
//! UTC for storage and comparison.
//!
//! # Key types
//!
//! - [`DateValue`] - Parsed calendar date with no time-of-day component.
//! - [`DateTimeValue`] - Parsed UTC date-time instant.
//! - [`parse::DateShape`] - Format grammar for calendar date recognition.
//! - [`parse::DateTimeShape`] - Format grammar for date-time recognition.
//! - [`Point`] - Civil date or UTC instant with written precision.
//! - [`Difference`] - Result of a calendar-owner difference measurement.
//! - [`DateError`] - Error type for parsing, local-zone resolution, and
//!   calendar arithmetic failures.
//!
//! # Calendar owner
//!
//! [`DateTimeValue::shift`] is the instant-frame shift primitive.
//! [`Point::shift`], [`Point::apply`], [`Point::diff`], [`Point::start_of`],
//! [`Point::end_of`], and [`Point::with_weekday`] own precision-aware calendar
//! arithmetic.

mod calendar;
mod error;
mod parse;
mod value;

pub(crate) use calendar::{
    DateComponent, Difference, Edge, Period, weekday_from_number,
};
pub use error::DateError;
pub(crate) use value::{CivilPrecision, DATE_PATTERN, DATETIME_PATTERN, Point};
pub use value::{DateTimeValue, DateValue};

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

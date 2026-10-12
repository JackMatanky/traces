//! ISO-8601 and RFC-3339 date and date-time parsing, formatting, and
//! arithmetic.
//!
//! Recognition entry points are [`DatePoint::parse`],
//! [`DatePoint::parse_prefix`], and [`DatePoint::parse_with_pattern`]. The
//! strict typed parsers [`DateValue::parse_iso`] and
//! [`DateTimeValue::parse_iso`] back their respective
//! [`FromStr`](std::str::FromStr) implementations.
//!
//! # Clock doctrine
//!
//! A naive datetime means what a human means: it is read in the process's local
//! zone and stored as UTC (see [`DateTimeValue::from_local`]); a date-only
//! value stays a zone-free calendar date. Instants render as the local clock
//! time and travel as UTC for storage and comparison.
//!
//! # Key types
//!
//! - [`DateValue`] - Parsed calendar date with no time-of-day component.
//! - [`DateTimeValue`] - Parsed UTC date-time instant.
//! - [`parse::DateShape`] - Format grammar for calendar date recognition.
//! - [`parse::DateTimeShape`] - Format grammar for date-time recognition.
//! - [`DatePoint`] - Calendar date or UTC instant with written precision.
//! - [`Difference`] - Result of a calendar-owner difference measurement.
//! - [`DateError`] - Error type for parsing, local-zone resolution, and
//!   calendar arithmetic failures.
//!
//! # Calendar owner
//!
//! [`DateTimeValue::shift`] is the instant-frame shift primitive.
//! [`DatePoint::shift`], [`DatePoint::apply`], [`DatePoint::diff`],
//! [`DatePoint::start_of`], [`DatePoint::end_of`], and
//! [`DatePoint::with_weekday`] own precision-aware calendar arithmetic.

mod calendar;
mod error;
mod parse;
mod value;

pub(crate) use calendar::{
    CalendarPeriod, DateComponent, Difference, PeriodEdge, weekday_from_number,
};
pub use error::DateError;
pub(crate) use value::{
    DATE_PATTERN, DATETIME_PATTERN, DatePoint, DatePrecision,
};
pub use value::{DateTimeValue, DateValue};

#[cfg(test)]
mod fixtures {
    use chrono::{TimeZone as _, Utc};

    use super::DateTimeValue;

    pub(super) fn fixed_datetime() -> DateTimeValue {
        DateTimeValue::from(
            Utc.with_ymd_and_hms(2026, 7, 29, 14, 30, 5).single().expect(
                "2026-07-29 14:30:05 UTC is a valid, unambiguous instant",
            ),
        )
    }
}

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
//! # Key Types
//!
//! - [`DateValue`]: Parsed calendar date with no time-of-day component.
//! - [`DateTimeValue`]: Parsed UTC date-time instant.
//! - [`DatePoint`]: Calendar date or UTC instant with written precision.
//! - [`Difference`]: Result of a calendar-owner difference measurement.
//! - [`DateError`]: Error type for parsing, local-zone resolution, and calendar
//!   arithmetic failures.
//!
//! # Calendar owner
//!
//! [`DateTimeValue::shift`] is the instant-frame shift primitive.
//! [`DatePoint::shift`], [`DatePoint::apply`], [`DatePoint::diff`],
//! [`DatePoint::start_of`], [`DatePoint::end_of`], and
//! [`DatePoint::with_weekday`] own precision-aware calendar arithmetic.
//!
//! Duration parsing, validation, and arithmetic.
//!
//! Accepts human-readable duration strings like `"1h 30m"` or `"4 yrs, 6 wks"`
//! and converts them to a total seconds value. Components are `<number><unit>`
//! pairs, optionally separated by whitespace or commas.
//!
//! # Key Types
//!
//! - [`DurationValue`]: A parsed duration carrying its total seconds, its
//!   original spelling, and its retained written components (which witness the
//!   calendar-versus-fixed regime).
//! - [`TimeUnit`]: Unit registry (parsing and seconds conversion). Single
//!   source of truth; callers should not maintain their own registries.
//! - [`DurationSeconds`]: An `f64` newtype that is finite on construction, with
//!   [`Ord`], [`Add`](std::ops::Add), [`Sub`](std::ops::Sub), and
//!   [`Mul`](std::ops::Mul).
//! - [`DurationError`]: Error type for parse and conversion failures.

mod calendar;
mod date;
mod datetime;
mod duration;
mod error;
#[cfg(test)]
mod fixtures;
mod point;
mod scan;
mod unit;

pub(crate) use calendar::{
    CalendarPeriod, DateComponent, Difference, PeriodEdge, weekday_from_number,
};
pub(crate) use date::DATE_PATTERN;
pub use date::DateValue;
pub(crate) use datetime::DATETIME_PATTERN;
pub use datetime::DateTimeValue;
pub use duration::DurationValue;
pub(crate) use duration::{DurationComponent, DurationSeconds};
pub use error::{DateError, DurationError};
pub(crate) use point::{DatePoint, DatePrecision};
pub(crate) use unit::{TimeUnit, UNIT_SPELLINGS_HINT};

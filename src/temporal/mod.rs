//! Provides calendar dates, Date-Times, Durations, and the arithmetic that
//! relates them.
//!
//! This module owns recognition, parsing, formatting, and value semantics for
//! date-shaped and duration-shaped input. Callers use validated values; parse
//! failures are public errors, while arithmetic failures stay internal and map
//! at the query and template boundaries.
//!
//! # Clock doctrine
//!
//! Unzoned Date-Time input resolves in the process-local zone and is stored as
//! UTC; explicit-offset input is normalized to UTC. A Date remains zone-free.
//! Date-Time display uses the local clock, except at the representable range
//! edge where the local offset cannot apply and the UTC clock is used instead.
//! Repeated local times resolve to the earlier occurrence; skipped local times
//! shift forward by the gap length.
//!
//! # Calendar regime
//!
//! Calendar units (day, week, month, and year) apply left-to-right on the local
//! clock. Exact units (hour and smaller) apply as elapsed time. Equal Durations
//! such as `1d` and `24h` can therefore move a Date-Time differently across
//! calendar boundaries and daylight-saving transitions.
//!
//! # Key Types
//!
//! - [`DateValue`]: Represents a zone-free calendar date with written
//!   precision.
//! - [`DateTimeValue`]: Stores a UTC Date-Time and renders it as the local
//!   clock.
//! - [`DurationValue`]: Represents a validated duration with nominal identity
//!   and a calendar or exact application regime.
mod calendar;
mod date;
mod datetime;
mod duration;
mod error;
#[cfg(test)]
mod fixtures;
mod point;
mod unit;
mod zone;
pub(crate) use calendar::{
    CalendarPeriod, DateComponent, DateDiff, PERIOD_SHORTHANDS, PeriodEdge,
    weekday_from_number,
};
pub(crate) use date::DATE_PATTERN;
pub use date::DateValue;
pub(crate) use datetime::DATETIME_PATTERN;
pub use datetime::DateTimeValue;
pub use duration::DurationValue;
pub(crate) use duration::{DurationComponent, DurationSeconds};
pub(crate) use error::CalendarError;
pub use error::{DateError, DurationError};
pub(crate) use point::DatePoint;
pub(crate) use unit::{TimeUnit, UNIT_SPELLINGS_HINT};
#[cfg(any(test, feature = "test-utils"))]
pub use zone::ZoneGuard;

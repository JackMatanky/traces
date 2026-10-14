//! Provides calendar dates, date-times, durations, and the arithmetic that
//! relates them.
//!
//! This module owns recognition, parsing, formatting, and value semantics for
//! date-shaped and duration-shaped input. Callers use validated values; parse
//! failures are public errors, while arithmetic failures stay internal and map
//! at the query and template boundaries.
//!
//! # Clock doctrine
//!
//! Naive Date-Time input resolves in the process-local zone and is stored as
//! UTC; explicit-offset input is normalized to UTC. A Date remains zone-free.
//! Date-Time display uses the local clock. Repeated local times resolve to the
//! earlier occurrence; skipped local times shift forward by the gap length.
//!
//! # Calendar arithmetic
//!
//! A duration is applied in one Jiff calendar operation, independent of the
//! order in which its units were written. Calendar units use the local clock;
//! exact units measure elapsed time. Calendar-day differences count local
//! calendar boundaries, while hour and smaller differences measure elapsed
//! time.
//!
//! # Key types
//!
//! - [`DateValue`]: A zone-free calendar date on the proleptic Gregorian
//!   calendar with whole-day precision.
//! - [`DateTimeValue`]: Stores a UTC timestamp and renders it as local civil
//!   time.
//! - [`DurationValue`]: A validated duration with nominal identity and
//!   calendar-aware application.
mod calendar;
mod date;
mod datetime;
mod duration;
mod error;
#[cfg(test)]
pub(crate) mod fixtures;
mod point;
mod unit;
mod zone;
pub(crate) use calendar::{
    CALENDAR_PERIOD_EDGES, CalendarPeriod, DateComponent, DateDiff, PeriodEdge,
    weekday_from_number,
};
pub(crate) use date::DEFAULT_DATE_PATTERN;
pub use date::DateValue;
pub(crate) use datetime::DEFAULT_DATETIME_PATTERN;
pub use datetime::DateTimeValue;
pub use duration::DurationValue;
pub(crate) use error::CalendarError;
pub use error::{DateError, DurationError};
pub(crate) use point::DatePoint;
pub(crate) use unit::{TimeUnit, UNIT_SPELLINGS_HINT};
#[cfg(any(test, feature = "test-utils"))]
pub use zone::ZoneGuard;
pub(crate) use zone::local_zone;

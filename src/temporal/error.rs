/// Error type for date/date-time parsing, local-zone resolution, and calendar
/// arithmetic failures.
///
/// Raised by the crate-internal `parse_iso` parsers on [`DateValue`] and
/// [`DateTimeValue`] and by the calendar owner's `shift`, `apply`, and `diff`
/// operations; the template engine translates the arithmetic failures into
/// render errors.
///
/// # Examples
///
/// ```rust
/// use traces_pkm::{DateError, DateValue};
///
/// let err = "26-08-22".parse::<DateValue>().expect_err("short year");
/// assert!(matches!(err, DateError::YearNotFourDigits { .. }));
/// ```
///
/// [`DateValue`]: crate::DateValue
/// [`DateTimeValue`]: crate::DateTimeValue
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum DateError {
    /// The input has no recognized date or date-time shape.
    #[error("not a date or date-time shape")]
    UnknownShape,
    /// No accepted date/time shape matched `input` (e.g., `"2026/08/22"`).
    #[error("`{input}` is not a recognized date/time: {reason}")]
    InvalidValue {
        /// The raw input that failed to parse.
        input: Box<str>,
        /// Why the last-attempted shape rejected the input.
        reason: &'static str,
    },
    /// `input` does not begin with 4 ASCII digits (e.g., `"26-08-22"`).
    ///
    /// The `chrono` `%Y` specifier accepts fewer digits, silently misreading
    /// the year.
    #[error("`{input}` does not have a 4-digit year")]
    YearNotFourDigits {
        /// The raw input that failed to parse.
        input: Box<str>,
    },
    /// The process's local timezone could not resolve a naive input, or the
    /// resolved offset would push it outside `NaiveDateTime`'s range.
    ///
    /// DST ambiguities and gaps never produce this error; they resolve
    /// deterministically.
    #[error("local timezone lookup failed for `{input}`")]
    LocalZoneLookup {
        /// The naive clock-time input that could not be resolved.
        input: Box<str>,
    },
    /// A date/time arithmetic operation overflowed or exceeded the
    /// representable range.
    #[error("date/time value is out of range")]
    OutOfRange,
}

/// Error returned when a duration operation fails.
///
/// Three failure domains:
///
/// - Parsing human-readable duration text;
/// - Validating raw seconds;
/// - Converting a duration or seconds value to a `TimeDelta`.
///
/// Parse failures carry no copy of the input: the caller already holds it.
///
/// # Examples
///
/// ```rust
/// use traces_pkm::{DurationError, DurationValue};
///
/// let err = "".parse::<DurationValue>().expect_err("empty input");
/// assert!(matches!(err, DurationError::Empty));
/// ```
#[derive(Clone, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DurationError {
    /// Input is empty or contains only separators.
    #[error("duration is empty")]
    Empty,

    /// A unit appears without a preceding number (e.g., `"h"` or `"hours"`).
    #[error("no number before unit")]
    MissingNumber,

    /// The number portion has a malformed sign (e.g., `"+h"`), or a part after
    /// the first carries an explicit `-` (e.g., `"1h -30m"`).
    #[error("misplaced sign in duration")]
    MisplacedSign,

    /// The number portion is not valid float syntax (e.g., a bare `"."`).
    #[error("invalid number syntax: {source}")]
    MalformedNumber {
        /// The underlying float-parse failure.
        #[source]
        source: std::num::ParseFloatError,
    },

    /// A number appears without a trailing unit (e.g., `"1"` or `"42"`).
    #[error("no unit after number")]
    MissingUnit,

    /// The unit string is not recognized (e.g., `"1x"`).
    #[error("unknown unit")]
    UnknownUnit,

    /// A single part's number overflows to infinity (e.g., a 400-digit
    /// literal).
    #[error("number overflows to infinity")]
    NumberOverflow,

    /// A raw seconds value is `NaN` or infinite.
    #[error("duration seconds must be finite")]
    NonFiniteSeconds,

    /// A finite seconds value does not fit `TimeDelta`'s representable range.
    #[error("duration seconds are outside the representable range")]
    SecondsOutOfRange,
}

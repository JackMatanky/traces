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
        /// The naive wall-clock input that could not be resolved.
        input: Box<str>,
    },
    /// A date/time arithmetic operation overflowed or exceeded the
    /// representable range.
    #[error("date/time value is out of range")]
    OutOfRange,
}

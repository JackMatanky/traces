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

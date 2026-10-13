//! Public parsing errors and internal temporal arithmetic errors.
//!
//! [`DateError`] reports date and date-time parsing or local-zone resolution
//! failures. [`CalendarError`] reports internal calendar-arithmetic failures,
//! while [`DurationError`] reports duration parsing, validation, and conversion
//! failures. The [`Display`](std::fmt::Display) text of each variant is the
//! message shown by the CLI and is part of the crate's contract.

/// Error type for date/date-time parsing and local-zone resolution failures.
///
/// Raised by the typed [`DateValue`] and [`DateTimeValue`] parsers. Calendar
/// arithmetic failures use the crate-internal `CalendarError` instead.
///
/// # Examples
///
/// ```
/// use traces_pkm::{DateError, DateValue};
///
/// let valid = "2026-07-29".parse::<DateValue>()?;
/// assert_eq!(valid.to_string(), "2026-07-29");
///
/// let short_year = "26-08-22".parse::<DateValue>();
/// assert!(matches!(short_year, Err(DateError::YearNotFourDigits { .. })));
///
/// let malformed = "2026/08/22".parse::<DateValue>();
/// assert!(matches!(malformed, Err(DateError::InvalidValue { .. })));
/// # Ok::<(), DateError>(())
/// ```
///
/// [`DateValue`]: crate::DateValue
/// [`DateTimeValue`]: crate::DateTimeValue
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum DateError {
    /// No accepted date/time shape matched `input` (e.g., "2026/08/22").
    #[error("`{input}` is not a recognized date/time: {reason}")]
    InvalidValue {
        /// The raw input that failed to parse.
        input: Box<str>,
        /// The reason the final shape check rejected the input.
        reason: &'static str,
    },
    /// `input` does not begin with 4 ASCII digits (e.g., "26-08-22").
    ///
    /// The year must be exactly 4 ASCII digits; a shorter prefix would
    /// otherwise be silently misread.
    #[error("`{input}` does not have a 4-digit year")]
    YearNotFourDigits {
        /// The raw input that failed to parse.
        input: Box<str>,
    },
    /// The process's local zone could not resolve a naive input.
    ///
    /// DST ambiguities and gaps never produce this error; they resolve
    /// deterministically.
    #[error("local zone lookup failed for `{input}`")]
    LocalZoneLookup {
        /// The naive clock-time input that could not be resolved.
        input: Box<str>,
    },
}

/// Error returned by internal calendar arithmetic.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum CalendarError {
    /// Calendar arithmetic exceeded the representable range.
    #[error("date/time value is out of range")]
    OutOfRange,
    /// Local-zone resolution failed for a computed local clock time.
    #[error("local zone lookup failed for {input}")]
    Zone {
        /// Local clock time that could not be resolved.
        input: Box<str>,
    },
}

impl CalendarError {
    /// Maps a calendar failure to the parse error exposed by typed date
    /// parsers.
    pub(crate) fn into_date_error(self, input: &str) -> DateError {
        match self {
            Self::OutOfRange => DateError::InvalidValue {
                input: input.into(),
                reason: "date/time is outside the representable range",
            },
            Self::Zone {
                input: zone_input,
            } => DateError::LocalZoneLookup {
                input: zone_input,
            },
        }
    }
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
/// ```
/// use traces_pkm::{DurationError, DurationValue};
///
/// let valid = "1h 30m".parse::<DurationValue>()?;
/// assert_eq!(valid.to_string(), "1h 30m");
///
/// let error = "1 fortnight".parse::<DurationValue>();
/// assert!(matches!(error, Err(DurationError::UnknownUnit)));
/// # Ok::<(), DurationError>(())
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

    /// A finite seconds value does not fit the range representable by
    /// `TimeDelta`.
    #[error("duration seconds are outside the representable range")]
    SecondsOutOfRange,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod formatting {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::invalid_value(
            DateError::InvalidValue {
                input: "2026/08/22".into(),
                reason: "not enough input",
            },
            "`2026/08/22` is not a recognized date/time: not enough input"
        )]
        #[case::year_not_four_digits(
            DateError::YearNotFourDigits { input: "26-08-22".into() },
            "`26-08-22` does not have a 4-digit year"
        )]
        #[case::local_zone_lookup(
            DateError::LocalZoneLookup { input: "2026-07-29 14:30".into() },
            "local zone lookup failed for `2026-07-29 14:30`"
        )]
        fn renders_exact_display_text_for_date_errors(
            #[case] error: DateError,
            #[case] expected: &str,
        ) {
            // Arrange: `error` and `expected` are the case literals.
            // Act
            let rendered = error.to_string();
            // Assert
            assert_eq!(
                rendered, expected,
                "Display text of {error:?} is the CLI contract"
            );
        }

        #[test]
        fn renders_the_out_of_range_calendar_error() {
            // Arrange
            let error = CalendarError::OutOfRange;
            let expected = "date/time value is out of range";
            // Act
            let rendered = error.to_string();
            // Assert
            assert_eq!(rendered, expected);
        }

        #[test]
        fn renders_the_zone_calendar_error() {
            // Arrange
            let error = CalendarError::Zone {
                input: "2026-03-08 02:30:00".into(),
            };
            let expected = "local zone lookup failed for 2026-03-08 02:30:00";
            // Act
            let rendered = error.to_string();
            // Assert
            assert_eq!(rendered, expected);
        }

        #[rstest]
        #[case::empty(DurationError::Empty, "duration is empty")]
        #[case::missing_number(
            DurationError::MissingNumber,
            "no number before unit"
        )]
        #[case::misplaced_sign(
            DurationError::MisplacedSign,
            "misplaced sign in duration"
        )]
        #[case::missing_unit(
            DurationError::MissingUnit,
            "no unit after number"
        )]
        #[case::unknown_unit(DurationError::UnknownUnit, "unknown unit")]
        #[case::number_overflow(
            DurationError::NumberOverflow,
            "number overflows to infinity"
        )]
        #[case::non_finite_seconds(
            DurationError::NonFiniteSeconds,
            "duration seconds must be finite"
        )]
        #[case::seconds_out_of_range(
            DurationError::SecondsOutOfRange,
            "duration seconds are outside the representable range"
        )]
        fn renders_exact_display_text_for_duration_errors(
            #[case] error: DurationError,
            #[case] expected: &str,
        ) {
            // Arrange: `error` and `expected` are the case literals.
            // Act
            let rendered = error.to_string();
            // Assert
            assert_eq!(
                rendered, expected,
                "Display text of {error:?} is the CLI contract"
            );
        }

        #[test]
        fn renders_its_own_prefix_before_the_source_error() {
            // Arrange
            let source =
                "1.2.3".parse::<f64>().expect_err("`1.2.3` is not a float");
            let source_text = source.to_string();
            let error = DurationError::MalformedNumber {
                source,
            };

            // Act
            let rendered = error.to_string();

            // Assert
            assert_eq!(
                rendered,
                format!("invalid number syntax: {source_text}"),
                "MalformedNumber must prefix the source error with its own \
                 message"
            );
        }
    }

    mod conversions {
        use super::*;
        use crate::temporal::{DurationSeconds, DurationValue};

        #[test]
        fn maps_out_of_range_calendar_failure_to_parse_error() {
            // Arrange
            let input = "9999-12-31";
            // Act
            let actual = CalendarError::OutOfRange.into_date_error(input);
            // Assert
            assert_eq!(actual, DateError::InvalidValue {
                input: input.into(),
                reason: "date/time is outside the representable range",
            });
        }

        #[test]
        fn maps_zone_calendar_failure_to_local_zone_lookup_error() {
            // Arrange
            let input = "2026-03-08 02:30:00";
            let error = CalendarError::Zone {
                input: input.into(),
            };
            // Act
            let actual = error.into_date_error(input);
            // Assert
            assert_eq!(actual, DateError::LocalZoneLookup {
                input: input.into(),
            });
        }

        #[test]
        fn rejects_non_finite_seconds_at_the_seconds_boundary() {
            // Arrange
            let raw = f64::NAN;

            // Act
            let actual = DurationSeconds::try_from(raw);

            // Assert
            assert!(
                matches!(actual, Err(DurationError::NonFiniteSeconds)),
                "expected NonFiniteSeconds, got {actual:?}"
            );
        }

        #[test]
        fn accepts_in_range_seconds_at_the_time_delta_boundary() {
            // Arrange
            let seconds =
                DurationSeconds::try_from(90.0).expect("90.0 is finite");

            // Act
            let actual = chrono::TimeDelta::try_from(seconds);

            // Assert
            assert!(
                actual.is_ok(),
                "90 seconds must fit TimeDelta, got {actual:?}"
            );
        }

        #[test]
        fn rejects_out_of_range_seconds_at_the_time_delta_boundary() {
            // Arrange
            let seconds =
                DurationSeconds::try_from(1e30).expect("1e30 is finite");

            // Act
            let actual = chrono::TimeDelta::try_from(seconds);

            // Assert
            assert!(
                matches!(actual, Err(DurationError::SecondsOutOfRange)),
                "expected SecondsOutOfRange, got {actual:?}"
            );
        }

        #[test]
        fn rejects_out_of_range_durations_at_the_time_delta_boundary() {
            // Arrange
            let seconds =
                DurationSeconds::try_from(1e17).expect("1e17 is finite");
            let duration = DurationValue::from_nominal_seconds(seconds);

            // Act
            let actual = chrono::TimeDelta::try_from(duration);

            // Assert
            assert!(
                matches!(actual, Err(DurationError::SecondsOutOfRange)),
                "expected SecondsOutOfRange, got {actual:?}"
            );
        }
    }
}

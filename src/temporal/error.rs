//! Defines public parsing errors and internal temporal arithmetic errors.
//!
//! [`DateError`] reports date and date-time parsing or local-zone resolution
//! failures. Internal calendar arithmetic uses a separate error type.
//! [`DurationError`] reports duration parsing, validation, and conversion
//! failures. The [`Display`](std::fmt::Display) text of each variant is the
//! message shown by the CLI and is part of the crate's contract.

/// Reports date/date-time parsing and local-zone resolution failures.
///
/// Typed [`DateValue`] and [`DateTimeValue`] parsers return this error.
/// Calendar arithmetic failures use a separate internal error type.
///
/// # Examples
///
/// ```
/// use traces::{DateError, DateValue};
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
    /// Reports that no accepted date/time shape matched `input` (e.g.,
    /// "2026/08/22").
    #[error("`{input}` is not a recognized date/time: {reason}")]
    InvalidValue {
        /// Stores the raw input that failed to parse.
        input: Box<str>,
        /// Stores the reason the final shape check rejected the input.
        reason: &'static str,
    },
    /// Reports that `input` does not begin with a four-digit year (e.g.,
    /// `26-08-22`).
    ///
    /// Years use four ASCII digits, optionally prefixed by `-` for negative
    /// years. Shorter or non-ASCII year forms are rejected rather than guessed.
    #[error("`{input}` does not have a 4-digit year")]
    YearNotFourDigits {
        /// Stores the raw input that failed to parse.
        input: Box<str>,
    },
    /// Reports that the process-local zone could not resolve a naive input.
    ///
    /// DST ambiguities and gaps never produce this error; they resolve
    /// deterministically.
    #[error("local zone lookup failed for `{input}`")]
    LocalZoneLookup {
        /// Stores the naive clock-time input that could not be resolved.
        input: Box<str>,
    },
}

/// Reports failures in duration parsing, validation, and conversion.
///
/// Covers three failure domains:
///
/// - Parsing human-readable duration text.
/// - Validating raw seconds.
/// - Converting nominal seconds to a Jiff span.
///
/// Parse failures carry no copy of the input: the caller already holds it.
///
/// # Examples
///
/// ```
/// use traces::{DurationError, DurationValue};
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
    /// Reports that the input is empty or contains only separators.
    #[error("duration is empty")]
    Empty,

    /// Reports that a unit appears without a preceding number (e.g., `"h"` or
    /// `"hours"`).
    #[error("no number before unit")]
    MissingNumber,

    /// Reports that the number portion has a malformed sign (e.g., `"+h"`), or
    /// a part after the first carries an explicit `-` (e.g., `"1h -30m"`).
    #[error("misplaced sign in duration")]
    MisplacedSign,

    /// Reports that the number portion is not valid float syntax (e.g., a bare
    /// `"."`).
    #[error("invalid number syntax: {source}")]
    MalformedNumber {
        /// Stores the underlying float-parse failure.
        #[source]
        source: std::num::ParseFloatError,
    },

    /// Reports that a number appears without a trailing unit (e.g., `"1"` or
    /// `"42"`).
    #[error("no unit after number")]
    MissingUnit,

    /// Reports that the unit string is not recognized (e.g., `"1x"`).
    #[error("unknown unit")]
    UnknownUnit,

    /// Reports that a single part's number overflows to infinity (e.g., a
    /// 400-digit literal).
    #[error("number overflows to infinity")]
    NumberOverflow,

    /// Reports that a raw seconds value is `NaN` or infinite.
    #[error("duration seconds must be finite")]
    NonFiniteSeconds,

    /// Reports that a finite seconds value does not fit the range representable
    /// by a Jiff [`Span`](jiff::Span).
    #[error("duration seconds are outside the representable range")]
    SecondsOutOfRange,
}

/// Reports failures in internal calendar arithmetic.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum CalendarError {
    /// Reports that calendar arithmetic exceeds the representable range.
    #[error("date/time value is out of range")]
    OutOfRange,
    /// Reports that local-zone resolution failed for a computed local clock
    /// time.
    #[error("local zone lookup failed for {input}")]
    Zone {
        /// Stores the local clock time that could not be resolved.
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
    }

    mod conversions {
        use super::*;
        use crate::temporal::DurationValue;

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
        fn rejects_durations_outside_the_jiff_span_range() {
            // Act
            let actual = DurationValue::from_nominal_seconds(1e17);

            // Assert
            assert!(
                matches!(actual, Err(DurationError::SecondsOutOfRange)),
                "expected SecondsOutOfRange, got {actual:?}"
            );
        }
    }
}

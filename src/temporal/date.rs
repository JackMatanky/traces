//! Defines zone-free calendar dates.
//!
//! Owns [`DateValue`] and the ISO recognition shape ([`DateShape`]) that turns
//! written `YYYY-MM-DD` and `YYYY-MM` input into it. Every accepted input year
//! is exactly four ASCII digits, and a parsed value always names a real day on
//! the proleptic Gregorian calendar. Callers hold the validated date or receive
//! a [`DateError`].

use std::{borrow::Cow, fmt, str::FromStr, time::SystemTime};

use jiff::{Span, civil};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    CalendarError, DateTimeValue, error::DateError, lexer::starts_with_year,
    point::DatePrecision,
};

/// Defines the canonical display format for [`DateValue`]: `2026-07-29`.
pub(crate) const DEFAULT_DATE_PATTERN: &str = "%Y-%m-%d";

/// Represents a zone-free calendar date with no time of day.
///
/// Carries a validated proleptic Gregorian date: no zone and no clock time.
/// Full dates require zero-padded month and day fields; `YYYY-MM` input
/// defaults its day to 1. Years use exactly four ASCII digits, with a leading
/// minus sign for negative years in `-YYYY-MM-DD` form. Deserialization runs
/// the same parser as [`FromStr`](std::str::FromStr), so inline and
/// deserialized dates follow one grammar.
///
/// # Examples
///
/// ```
/// use traces_pkm::{DateError, DateValue};
///
/// let date: DateValue = "2026-07-29".parse()?;
/// assert_eq!(date.to_string(), "2026-07-29");
/// assert_eq!(DateValue::from_ymd(2024, 2, 29)?.to_string(), "2024-02-29");
///
/// let error = "26-08-22".parse::<DateValue>();
/// assert!(matches!(error, Err(DateError::YearNotFourDigits { .. })));
/// # Ok::<(), DateError>(())
/// ```
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DateValue(civil::Date);

impl DateValue {
    /// Constructs a calendar date from year, month, and day fields.
    ///
    /// # Errors
    ///
    /// - [`DateError::InvalidValue`] if the year is outside `-9999..=9999` or
    ///   the fields do not name a representable Gregorian date.
    #[inline]
    pub fn from_ymd(
        year: i32,
        month: u32,
        day: u32,
    ) -> Result<Self, DateError> {
        let invalid = || DateError::InvalidValue {
            input: format!("{year}-{month}-{day}").into(),
            reason: "a field is out of range",
        };
        if !(-9999..=9999).contains(&year) {
            return Err(invalid());
        }
        let year = i16::try_from(year).map_err(|_| invalid())?;
        let month = i8::try_from(month).map_err(|_| invalid())?;
        let day = i8::try_from(day).map_err(|_| invalid())?;
        civil::Date::new(year, month, day).map(Self).map_err(|_| invalid())
    }

    /// Parses an ISO-8601 date string (`YYYY-MM-DD`, `YYYY-MM`, or a signed
    /// four-digit year in the same forms) using the recognition layer.
    ///
    /// Trims surrounding whitespace first, then requires a four-digit year,
    /// optionally prefixed by a minus sign, before trying accepted shapes.
    ///
    /// # Errors
    ///
    /// - [`DateError::YearNotFourDigits`] if the trimmed `s` does not begin
    ///   with four ASCII digits or a minus sign followed by four ASCII digits.
    /// - [`DateError::InvalidValue`] if no accepted shape matches.
    pub(crate) fn parse_iso(s: &str) -> Result<Self, DateError> {
        let trimmed = s.trim();
        if !starts_with_year(trimmed) {
            return Err(DateError::YearNotFourDigits {
                input: trimmed.into(),
            });
        }
        DateShape::parse_any(trimmed).map(|(date, _)| Self(date)).map_err(
            |reason| DateError::InvalidValue {
                input: trimmed.into(),
                reason,
            },
        )
    }

    /// Wraps a valid Jiff civil date.
    #[inline]
    #[must_use]
    pub(crate) const fn from_civil_date(date: civil::Date) -> Self {
        Self(date)
    }

    /// Returns the wrapped Jiff civil date.
    #[inline]
    #[must_use]
    pub(crate) const fn civil_date(self) -> civil::Date {
        self.0
    }

    /// Adds a Jiff span, discarding the time of day as date-only arithmetic
    /// does.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the resulting date exceeds Jiff's
    ///   representable range.
    pub(super) fn advance_span(
        self,
        span: Span,
    ) -> Result<Self, CalendarError> {
        self.0
            .checked_add(span)
            .map(Self)
            .map_err(|_| CalendarError::OutOfRange)
    }
}

/// Renders as a four-digit year with a leading minus sign for negative years.
impl fmt::Display for DateValue {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let date = self.0;
        if date.year() < 0 {
            write!(
                f,
                "-{:04}-{:02}-{:02}",
                date.year().unsigned_abs(),
                date.month(),
                date.day()
            )
        } else {
            write!(
                f,
                "{:04}-{:02}-{:02}",
                date.year(),
                date.month(),
                date.day()
            )
        }
    }
}

impl From<civil::Date> for DateValue {
    #[inline]
    fn from(date: civil::Date) -> Self {
        Self(date)
    }
}

impl FromStr for DateValue {
    type Err = DateError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_iso(s)
    }
}

/// Converts to the calendar day `time` falls on in the local zone, not the UTC
/// calendar day.
///
/// A file's date means the reader's local day, the same doctrine that reads
/// unzoned date-time input on the local clock: a time in the final hours of
/// December 31 locally can already be January 1 in UTC, or the reverse.
impl TryFrom<SystemTime> for DateValue {
    type Error = DateError;

    /// Converts `time` to its date in the current local time zone.
    ///
    /// # Errors
    ///
    /// - [`DateError::InvalidValue`] if `time` is outside the representable
    ///   timestamp range.
    /// - [`DateError::LocalZoneLookup`] if system time-zone discovery fails.
    #[inline]
    fn try_from(time: SystemTime) -> Result<Self, Self::Error> {
        DateTimeValue::try_from(time)?.local_date()
    }
}

/// Serializes as the canonical `YYYY-MM-DD` display string.
impl Serialize for DateValue {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Deserializes through the same parser as [`FromStr`](std::str::FromStr), so
/// the four-digit-year rule and `YYYY-MM` acceptance apply identically to
/// inline and deserialized dates.
impl<'de> Deserialize<'de> for DateValue {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = Cow::<'de, str>::deserialize(deserializer)?;
        Self::parse_iso(&s).map_err(serde::de::Error::custom)
    }
}

/// Defines the recognized date input shapes, tried in order by
/// [`DateShape::parse_any`] for [`DatePoint::parse`](super::DatePoint::parse)
/// and [`DateValue::parse_iso`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DateShape {
    /// Matches a full ISO date: `2026-07-29`.
    YearMonthDay,
    /// Matches year-month reduced precision: `2026-07` (day defaults to 1).
    YearMonth,
}

impl DateShape {
    /// Parses `s` as a full date, falling back to a year-month.
    ///
    /// # Errors
    ///
    /// Returns a static reason from the best-fitting shape: the year-month
    /// reason for 7-byte input (`YYYY-MM`), or the full-date reason otherwise.
    pub(crate) fn parse_any(
        s: &str,
    ) -> Result<(civil::Date, DatePrecision), &'static str> {
        Self::YearMonthDay.parse_precision(s).or_else(|day_reason| {
            Self::YearMonth.parse_precision(s).map_err(|month_reason| {
                if s.len() == 7 {
                    month_reason
                } else {
                    day_reason
                }
            })
        })
    }

    /// Parses `s` according to this shape, returning the date and precision.
    ///
    /// # Errors
    ///
    /// Returns a static reason if `s` does not match this shape.
    fn parse_precision(
        self,
        s: &str,
    ) -> Result<(civil::Date, DatePrecision), &'static str> {
        match self {
            Self::YearMonthDay => {
                let date = Self::parse_year_month_day(s)?;
                Ok((date, DatePrecision::Day))
            }
            Self::YearMonth => {
                Self::parse_year_month(s).map(|d| (d, DatePrecision::Month))
            }
        }
    }

    /// Parses `YYYY-MM-DD` or `-YYYY-MM-DD`.
    ///
    /// # Errors
    ///
    /// Returns a static reason if `s` is not a valid full date string.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "year length is 4 or 5 bytes"
    )]
    fn parse_year_month_day(s: &str) -> Result<civil::Date, &'static str> {
        let year_len = if s.starts_with('-') {
            5
        } else {
            4
        };
        let date_len = year_len + 6;
        if s.len() < date_len && starts_with_year(s) {
            return Err("input is too short");
        }
        if s.len() > date_len && has_padded_date_prefix(s) {
            return Err("unexpected trailing input");
        }
        if s.len() != date_len || !has_padded_date_prefix(s) {
            return Err("invalid input");
        }
        let year = s.get(..year_len).ok_or("invalid input")?;
        let month = s.get(year_len + 1..year_len + 3).ok_or("invalid input")?;
        let day = s.get(year_len + 4..year_len + 6).ok_or("invalid input")?;
        civil::Date::new(
            year.parse().map_err(|_| "invalid input")?,
            month.parse().map_err(|_| "invalid input")?,
            day.parse().map_err(|_| "invalid input")?,
        )
        .map_err(|_| "a field is out of range")
    }

    /// Parses `YYYY-MM` or `-YYYY-MM`, defaulting the day to 1.
    ///
    /// # Errors
    ///
    /// Returns a static reason if `s` is not a valid year-month string.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "year length is 4 or 5 bytes"
    )]
    fn parse_year_month(s: &str) -> Result<civil::Date, &'static str> {
        let year_len = if s.starts_with('-') {
            5
        } else {
            4
        };
        let expected_len = year_len + 3;
        if s.len() != expected_len {
            return Err("not enough input");
        }
        let year = s.get(..year_len).ok_or("not enough input")?;
        if s.as_bytes().get(year_len) != Some(&b'-') {
            return Err("not enough input");
        }
        let month_str = s.get(year_len + 1..).ok_or("not enough input")?;
        if !month_str.bytes().all(|b| b.is_ascii_digit()) {
            return Err("not enough input");
        }
        let year: i16 = year.parse().map_err(|_| "invalid input")?;
        let month: i8 = month_str.parse().map_err(|_| "invalid input")?;
        civil::Date::new(year, month, 1).map_err(|_| "a field is out of range")
    }
}

/// Returns `true` if `s` starts with a four-digit zero-padded date prefix.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "year length is 4 or 5 bytes"
)]
pub(crate) fn has_padded_date_prefix(s: &str) -> bool {
    let year_len = if s.starts_with('-') {
        5
    } else {
        4
    };
    let year_start = usize::from(s.starts_with('-'));
    let month_start = year_len + 1;
    let day_start = month_start + 3;
    s.get(..year_len + 6).is_some_and(|date| {
        let bytes = date.as_bytes();
        bytes.get(year_len) == Some(&b'-')
            && bytes.get(month_start + 2) == Some(&b'-')
            && bytes
                .get(year_start..year_len)
                .is_some_and(|year| year.iter().all(u8::is_ascii_digit))
            && bytes
                .get(month_start..month_start + 2)
                .is_some_and(|month| month.iter().all(u8::is_ascii_digit))
            && bytes
                .get(day_start..day_start + 2)
                .is_some_and(|day| day.iter().all(u8::is_ascii_digit))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::test_support::{ZoneGuard, fixtures::ymd};

    mod constructor {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn constructs_dates_from_valid_boundary_fields() {
            // Arrange: the leap day and the supported year endpoints.
            // Act
            let dates = [
                DateValue::from_ymd(-9999, 1, 1),
                DateValue::from_ymd(2024, 2, 29),
                DateValue::from_ymd(0, 1, 1),
                DateValue::from_ymd(9999, 12, 31),
            ];
            // Assert
            assert_eq!(dates, [
                Ok(ymd(-9999, 1, 1)),
                Ok(ymd(2024, 2, 29)),
                Ok(ymd(0, 1, 1)),
                Ok(ymd(9999, 12, 31))
            ]);
        }

        #[rstest]
        #[case::common_year_leap_day(2026, 2, 29)]
        #[case::month_zero(2026, 0, 1)]
        #[case::month_thirteen(2026, 13, 1)]
        #[case::day_zero(2026, 1, 0)]
        #[case::day_thirty_two(2026, 1, 32)]
        #[case::year_below_range(-10000, 1, 1)]
        #[case::year_above_range(10000, 1, 1)]
        fn rejects_invalid_or_out_of_range_fields(
            #[case] year: i32,
            #[case] month: u32,
            #[case] day: u32,
        ) {
            // Arrange: the fields are invalid or outside the supported range.
            // Act
            let result = DateValue::from_ymd(year, month, day);
            // Assert
            assert!(matches!(result, Err(DateError::InvalidValue { .. })));
        }
        #[test]
        fn accepts_leap_day() {
            // Arrange: 2024 is a leap year.
            // Act
            let result = "2024-02-29".parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Ok(ymd(2024, 2, 29)),
                "`2024-02-29` must construct as a valid leap day"
            );
        }

        #[test]
        fn rejects_feb_29_in_a_common_year() {
            // Arrange: 2026 is not a leap year.
            // Act
            let result = "2026-02-29".parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Err(DateError::InvalidValue {
                    input: "2026-02-29".into(),
                    reason: "a field is out of range",
                }),
                "`2026-02-29` must not construct in a common year"
            );
        }

        #[rstest]
        #[case::month_zero("2026-00-01")]
        #[case::month_thirteen("2026-13-01")]
        fn rejects_months_outside_the_calendar(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let result = input.parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Err(DateError::InvalidValue {
                    input: input.into(),
                    reason: "a field is out of range",
                }),
                "`{input}` must not construct"
            );
        }

        #[rstest]
        #[case::day_zero("2026-01-00")]
        #[case::day_thirty_two("2026-01-32")]
        fn rejects_days_outside_the_month(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let result = input.parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Err(DateError::InvalidValue {
                    input: input.into(),
                    reason: "a field is out of range",
                }),
                "`{input}` must not construct"
            );
        }

        #[rstest]
        #[case::minimum_signed_year("-9999-01-01", (-9999, 1, 1))]
        #[case::year_zero("0000-01-01", (0, 1, 1))]
        #[case::maximum_year("9999-12-31", (9999, 12, 31))]
        fn accepts_year_boundaries(
            #[case] input: &str,
            #[case] (year, month, day): (i32, u32, u32),
        ) {
            // Arrange: the input and expected date are paired boundary values.
            // Act
            let result = input.parse::<DateValue>();
            // Assert
            assert_eq!(result, Ok(ymd(year, month, day)));
            let date = result.expect("boundary date parses");
            assert_eq!(date.to_string(), input);
            assert_eq!(input.parse::<DateValue>(), Ok(date));
        }

        #[test]
        fn rejects_a_negative_year_without_four_digits() {
            // Arrange
            let input = "-001-01-01";
            // Act
            let result = input.parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Err(DateError::YearNotFourDigits {
                    input: input.into()
                })
            );
        }
    }

    mod parse {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn parses_a_canonical_iso_date() {
            // Arrange: the input literal is the whole input.
            // Act
            let result = "2026-07-29".parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Ok(ymd(2026, 7, 29)),
                "`2026-07-29` must parse to its calendar day"
            );
        }

        #[rstest]
        #[case::unpadded_month_and_day("2026-1-1")]
        #[case::unpadded_day("2026-07-2")]
        fn rejects_unpadded_dates(#[case] input: &str) {
            // Arrange: strict date parsing requires zero-padded month and day.
            // Act
            let result = input.parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Err(DateError::InvalidValue {
                    input: input.into(),
                    reason: "input is too short",
                }),
                "unpadded date {input} must be rejected"
            );
        }

        #[test]
        fn parses_year_month_precision_to_the_first_of_the_month() {
            // Arrange: `2026-07` carries month precision only.
            // Act
            let result = "2026-07".parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Ok(ymd(2026, 7, 1)),
                "`YYYY-MM` input must default its day to 1"
            );
        }

        #[test]
        fn rejects_two_digit_year_with_year_not_four_digits() {
            // Arrange: the input literal is the whole input.
            // Act
            let result = "26-08-22".parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Err(DateError::YearNotFourDigits {
                    input: "26-08-22".into(),
                }),
                "a two-digit year must not be padded silently"
            );
        }

        #[test]
        fn rejects_five_digit_year_as_invalid_value() {
            // Arrange: five digits pass the leading-four-digit check, then
            // no shape matches.
            // Act
            let result = "12345-01-01".parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Err(DateError::InvalidValue {
                    input: "12345-01-01".into(),
                    reason: "invalid input",
                }),
                "a five-digit year must not construct"
            );
        }

        #[test]
        fn rejects_february_thirtieth_as_invalid_value() {
            // Arrange: February never has 30 days.
            // Act
            let result = "2026-02-30".parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Err(DateError::InvalidValue {
                    input: "2026-02-30".into(),
                    reason: "a field is out of range",
                }),
                "an impossible calendar day must not construct"
            );
        }

        #[rstest]
        #[case::iso_t_separator("2026-07-29T12:00:00")]
        #[case::space_separator("2026-07-29 12:00:00")]
        fn rejects_date_time_strings(#[case] input: &str) {
            // Arrange: `input` carries a time the date parser never accepts.
            // Act
            let result = input.parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Err(DateError::InvalidValue {
                    input: input.into(),
                    reason: "unexpected trailing input",
                }),
                "`{input}` must not parse as a date"
            );
        }

        #[rstest]
        #[case::empty("")]
        #[case::blank("   ")]
        fn rejects_empty_or_blank_input(#[case] input: &str) {
            // Arrange: `input` is the case literal; blank input trims to "".
            // Act
            let result = input.parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Err(DateError::YearNotFourDigits {
                    input: "".into(),
                }),
                "`{input:?}` carries no year to read"
            );
        }

        #[test]
        fn rejects_non_ascii_digits_with_year_not_four_digits() {
            // Arrange: full-width digits are not ASCII digits.
            // Act
            let result = "２０２６-01-01".parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Err(DateError::YearNotFourDigits {
                    input: "２０２６-01-01".into(),
                }),
                "non-ASCII digits must not satisfy the year rule"
            );
        }

        #[rstest]
        #[case::leading_space(" 2026-07-29")]
        #[case::trailing_space("2026-07-29 ")]
        fn trims_surrounding_whitespace(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let result = input.parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Ok(ymd(2026, 7, 29)),
                "`{input:?}` must parse after trimming"
            );
        }

        #[rstest]
        #[case::seven_byte_takes_the_year_month_reason(
            "2026-ab",
            "not enough input"
        )]
        #[case::other_length_takes_the_full_date_reason(
            "2026-7",
            "input is too short"
        )]
        fn selects_the_rejection_reason_by_input_length(
            #[case] input: &str,
            #[case] reason: &'static str,
        ) {
            // Arrange: `input` and `reason` are the case literals.
            // Act
            let result = input.parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Err(DateError::InvalidValue {
                    input: input.into(),
                    reason,
                }),
                "`{input}` must report its own shape's reason"
            );
        }

        #[test]
        fn rejects_non_iso_separators_as_invalid_value() {
            // Arrange: slashes never introduce an accepted shape.
            // Act
            let result = "2026/08/22".parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Err(DateError::InvalidValue {
                    input: "2026/08/22".into(),
                    reason: "invalid input",
                }),
                "`2026/08/22` must not construct"
            );
        }
    }

    mod formatting {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::mid_year((2026, 7, 29), "2026-07-29")]
        #[case::single_digit_fields((2001, 1, 2), "2001-01-02")]
        #[case::end_of_year((1999, 12, 31), "1999-12-31")]
        fn renders_the_zero_padded_iso_form(
            #[case] (year, month, day): (i32, u32, u32),
            #[case] expected: &str,
        ) {
            // Arrange
            let date = ymd(year, month, day);

            // Act
            let rendered = date.to_string();

            // Assert
            assert_eq!(
                rendered, expected,
                "the canonical form pads every field to two digits"
            );
        }

        #[test]
        fn renders_year_month_precision_as_the_first_of_the_month() {
            // Arrange
            let date =
                "2026-07".parse::<DateValue>().expect("valid year-month input");

            // Act
            let rendered = date.to_string();

            // Assert
            assert_eq!(
                rendered, "2026-07-01",
                "a month-precision date renders as its stored day 1"
            );
        }

        #[test]
        fn parses_its_own_display_output() {
            // Arrange
            let date = ymd(2026, 7, 29);

            // Act
            let reparsed = date.to_string().parse::<DateValue>();

            // Assert
            assert_eq!(
                reparsed,
                Ok(date),
                "Display output must re-parse to the same date"
            );
        }
    }

    mod serialization {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn serializes_as_the_iso_string() {
            // Arrange
            let date = ymd(2026, 7, 29);

            // Act
            let json =
                serde_json::to_string(&date).map_err(|error| error.to_string());

            // Assert
            assert_eq!(
                json.as_deref(),
                Ok("\"2026-07-29\""),
                "serialization is the canonical ISO string"
            );
        }

        #[test]
        fn keeps_its_postcard_spelling() {
            // Arrange
            let date = ymd(2026, 7, 29);

            // Act
            let bytes = postcard::to_allocvec(&date);

            // Assert
            assert_eq!(
                bytes.as_deref(),
                Ok(b"\x0a2026-07-29".as_slice()),
                "postcard encodes the length-prefixed ISO string"
            );
            assert_eq!(
                bytes
                    .and_then(|bytes| postcard::from_bytes::<DateValue>(&bytes))
                    .map_err(|error| error.to_string()),
                Ok(date),
                "postcard encode then decode must return the same date"
            );
        }

        #[test]
        fn round_trips_through_json_serde() {
            // Arrange
            let date = ymd(2026, 7, 29);

            // Act
            let restored = serde_json::to_string(&date)
                .and_then(|json| serde_json::from_str::<DateValue>(&json))
                .map_err(|error| error.to_string());

            // Assert
            assert_eq!(
                restored,
                Ok(date),
                "encode then decode must return the same date"
            );
        }
    }

    mod deserialization {
        use pretty_assertions::assert_eq;
        use rstest::rstest;
        use serde::Deserialize;

        use super::*;

        #[derive(Debug, Deserialize)]
        struct YamlDate {
            date: DateValue,
        }

        #[test]
        fn deserializes_an_iso_date_string() {
            // Arrange: JSON strings share the serde string path.
            // Act
            let result = serde_json::from_str::<DateValue>("\"2026-07-29\"")
                .map_err(|error| error.to_string());
            // Assert
            assert_eq!(
                result,
                Ok(ymd(2026, 7, 29)),
                "an ISO string must deserialize to its date"
            );
        }

        #[test]
        fn deserializes_a_year_month_string_to_the_first_of_the_month() {
            // Arrange: JSON strings share the serde string path.
            // Act
            let result = serde_json::from_str::<DateValue>("\"2026-07\"")
                .map_err(|error| error.to_string());
            // Assert
            assert_eq!(
                result,
                Ok(ymd(2026, 7, 1)),
                "a `YYYY-MM` string must default its day to 1"
            );
        }

        #[test]
        fn rejects_a_two_digit_year_string() {
            // Arrange: the input literal is the whole input.
            // Act
            let result = serde_json::from_str::<DateValue>("\"26-08-22\"");
            // Assert
            let surfaced = result.as_ref().err().is_some_and(|error| {
                error
                    .to_string()
                    .contains("`26-08-22` does not have a 4-digit year")
            });
            assert!(
                surfaced,
                "the 4-digit-year rule must surface through serde, got \
                 {result:?}"
            );
        }

        #[test]
        fn deserializes_a_year_month_date_from_a_yaml_note() {
            // Arrange
            let yaml = "date: \"2026-07\"\n";

            // Act
            let note = noyalib::from_str::<YamlDate>(yaml);

            // Assert
            assert_eq!(
                note.map(|note| note.date).map_err(|e| e.to_string()),
                Ok(ymd(2026, 7, 1)),
                "a deserialized `YYYY-MM` date must match inline parsing"
            );
        }

        #[rstest]
        #[case::number("date: 42\n")]
        #[case::boolean("date: true\n")]
        #[case::null("date: null\n")]
        #[case::sequence("date: [2026]\n")]
        fn rejects_non_string_scalars(#[case] yaml: &str) {
            // Arrange: `yaml` is the case literal.
            // Act
            let result = noyalib::from_str::<YamlDate>(yaml);
            // Assert
            assert!(
                result.is_err(),
                "`{yaml}` must be rejected as a non-string scalar, got \
                 {result:?}"
            );
        }

        #[test]
        fn accepts_an_unquoted_iso_date_scalar() {
            // Arrange: a bare `2026-07-29` is a specially typed scalar in
            // YAML 1.1; noyalib yields the plain string the parser accepts.
            let yaml = "date: 2026-07-29\n";

            // Act
            let result = noyalib::from_str::<YamlDate>(yaml);

            // Assert
            assert_eq!(
                result.map(|note| note.date).map_err(|e| e.to_string()),
                Ok(ymd(2026, 7, 29)),
                "an unquoted ISO date must deserialize like its quoted form"
            );
        }

        #[rstest]
        #[case::anchor_definition("date: &d \"2026-07-29\"\n")]
        #[case::alias_reference("base: &d \"2026-07-29\"\ndate: *d\n")]
        fn accepts_anchored_string_fields(#[case] yaml: &str) {
            // Arrange: `yaml` is the case literal.
            // Act
            let result = noyalib::from_str::<YamlDate>(yaml);
            // Assert
            assert_eq!(
                result.map(|note| note.date).map_err(|e| e.to_string()),
                Ok(ymd(2026, 7, 29)),
                "an anchored or aliased string must resolve to its date"
            );
        }
    }

    mod conversions {

        use super::*;

        #[test]
        fn converts_a_system_time_to_the_local_calendar_day() {
            // Arrange: UTC-05:00 has no DST; 1000 seconds after the epoch is
            // 00:16:40Z on January 1, which is 19:16:40 on December 31 local.
            ZoneGuard::set("Etc/GMT+5");
            let system_time =
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000);

            // Act
            let result = DateValue::try_from(system_time);

            // Assert
            assert!(
                matches!(result, Ok(date) if date == ymd(1969, 12, 31)),
                "a file's date is the reader's local day, not the UTC day"
            );
        }

        #[test]
        fn converts_a_system_time_to_the_date_under_the_utc_zone() {
            // Arrange: under UTC the local day and the UTC day agree.
            ZoneGuard::set("UTC");
            let system_time =
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000);

            // Act
            let result = DateValue::try_from(system_time);

            // Assert
            assert!(
                matches!(result, Ok(date) if date == ymd(1970, 1, 1)),
                "the epoch's UTC day converts unchanged under UTC"
            );
        }
    }

    mod ordering {
        use std::cmp::Ordering;

        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn orders_dates_chronologically() {
            // Arrange
            let earlier = ymd(2026, 1, 1);
            let later = ymd(2026, 12, 31);

            // Act
            let forward = earlier.cmp(&later);
            let backward = later.cmp(&earlier);
            let same = earlier.cmp(&earlier);

            // Assert
            assert_eq!(
                forward,
                Ordering::Less,
                "an earlier date must order before a later one"
            );
            assert_eq!(
                backward,
                Ordering::Greater,
                "ordering must reverse with the operands"
            );
            assert_eq!(
                same,
                Ordering::Equal,
                "a date must equal itself in ordering"
            );
        }
    }

    mod hashing {
        use std::collections::HashSet;

        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn collapses_equal_dates_into_one_hash_set_slot() {
            // Arrange
            let mut seen = HashSet::new();

            // Act
            seen.insert(ymd(2026, 7, 29));
            let duplicate = seen.insert(ymd(2026, 7, 29));
            seen.insert(ymd(2026, 7, 30));

            // Assert
            assert!(
                !duplicate,
                "re-inserting an equal date must not add a slot, got \
                 {duplicate}"
            );
            assert_eq!(
                seen.len(),
                2,
                "the set holds one slot per distinct date"
            );
        }
    }

    mod proptests {
        use proptest::prelude::*;

        use super::*;

        proptest! {
            #[test]
            fn iso_display_round_trips_supported_dates(
                year in 0_i32..=9999,
                month in 1_u32..=12,
                day in 1_u32..=28,
            ) {
                // Arrange
                let date = DateValue::from_ymd(year, month, day)
                    .expect("generated four-digit date is valid");

                // Act
                let displayed = date.to_string();
                let parsed = DateValue::parse_iso(&displayed);

                // Assert
                prop_assert_eq!(parsed, Ok(date));
            }
        }
    }
}

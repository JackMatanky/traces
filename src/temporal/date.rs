//! A zone-free calendar date.
//!
//! Owns [`DateValue`] and the ISO recognition shape ([`DateShape`]) that turns
//! written `YYYY-MM-DD` and `YYYY-MM` input into it. Every accepted input year
//! is exactly four ASCII digits, and a parsed value always names a real day on
//! the proleptic Gregorian calendar. Callers hold the validated date or
//! receive a [`DateError`].

use std::{borrow::Cow, fmt, str::FromStr, time::SystemTime};

use chrono::{Datelike as _, NaiveDate, NaiveTime, TimeDelta};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    DatePrecision, DateTimeValue,
    error::DateError,
    point::{parse_reason, starts_with_year},
};

/// Canonical output format of [`DateValue`]: `2026-07-29`.
pub(crate) const DATE_PATTERN: &str = "%Y-%m-%d";

/// A zone-free calendar date with no time of day.
///
/// Carries a validated `YYYY-MM-DD` day on the proleptic Gregorian calendar:
/// no zone and no clock time. Every accepted input year is exactly four ASCII
/// digits, every value names a real day, and written `YYYY-MM` input defaults
/// its day to 1. Deserialization runs the same parser as
/// [`FromStr`](std::str::FromStr), so inline and deserialized dates follow one
/// grammar.
///
/// # Examples
///
/// ```
/// use traces_pkm::{DateError, DateValue};
///
/// let date: DateValue = "2026-07-29".parse()?;
/// assert_eq!(date.to_string(), "2026-07-29");
///
/// let error = "26-08-22".parse::<DateValue>();
/// assert!(matches!(error, Err(DateError::YearNotFourDigits { .. })));
/// # Ok::<(), DateError>(())
/// ```
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DateValue(NaiveDate);

impl DateValue {
    /// Returns the wrapped [`NaiveDate`].
    #[must_use]
    pub(crate) const fn naive_date(self) -> NaiveDate {
        self.0
    }

    /// Returns the whole calendar months from `self` to `to`, signed by
    /// ordering.
    ///
    /// Counts the total calendar months between the two dates and subtracts
    /// one when the day-of-month of `to` is below that of `self`, because
    /// that final month is not complete. Either ordering is accepted: `to`
    /// before `self` yields a negative count.
    pub(super) fn whole_months_until(self, to: Self) -> i64 {
        let (from, to) = (self.naive_date(), to.naive_date());
        let (earlier, later, sign) = if to >= from {
            (from, to, 1)
        } else {
            (to, from, -1)
        };
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "year/month/day are all bounded by NaiveDate's \
                      representable range (~±262,000 years), so the year \
                      subtraction, ×12 month conversion, day comparison, and \
                      sign multiply can't overflow i64"
        )]
        let months = sign
            * (i64::from(later.year() - earlier.year()) * 12
                + i64::from(later.month())
                - i64::from(earlier.month())
                - i64::from(later.day() < earlier.day()));
        months
    }

    /// Returns the whole calendar years from `self` to `to`, signed by
    /// ordering.
    ///
    /// A year counts only once the month and day of `to` reach those of
    /// `self`, so this is [`DateValue::whole_months_until`] divided down to
    /// years.
    pub(super) fn whole_years_until(self, to: Self) -> i64 {
        self.whole_months_until(to) / 12
    }

    /// Returns the date advanced by `delta`, keeping the date part.
    ///
    /// Reads `delta` from midnight of `self` and drops the resulting time of
    /// day, so sub-day deltas leave the date unchanged.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the result falls outside the
    ///   representable `chrono` date range.
    pub(super) fn advance(self, delta: TimeDelta) -> Result<Self, DateError> {
        self.naive_date()
            .and_time(NaiveTime::MIN)
            .checked_add_signed(delta)
            .map(|local_datetime| Self::from(local_datetime.date()))
            .ok_or(DateError::OutOfRange)
    }

    /// Parses an ISO-8601 date string (`YYYY-MM-DD` or `YYYY-MM`) using the
    /// recognition layer.
    ///
    /// Trims surrounding whitespace first, then requires four leading ASCII
    /// digits before trying each accepted shape in order.
    ///
    /// # Errors
    ///
    /// - [`DateError::YearNotFourDigits`] if the trimmed `s` does not begin
    ///   with 4 ASCII digits.
    /// - [`DateError::InvalidValue`] if no accepted shape matches, carrying `s`
    ///   and the static reason the best-fitting shape rejected it.
    pub(crate) fn parse_iso(s: &str) -> Result<Self, DateError> {
        let trimmed = s.trim();
        if !starts_with_year(trimmed) {
            return Err(DateError::YearNotFourDigits {
                input: trimmed.into(),
            });
        }
        DateShape::parse_any(trimmed).map(|(date, _)| Self::from(date)).map_err(
            |reason| DateError::InvalidValue {
                input: trimmed.into(),
                reason,
            },
        )
    }
}

/// Recognized date input format shapes, tried in order by
/// [`DateShape::parse_any`] for [`DatePoint::parse`](super::DatePoint::parse)
/// and [`DateValue::parse_iso`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DateShape {
    /// Full ISO date: `2026-07-29`.
    YearMonthDay,
    /// Year-month reduced precision: `2026-07`. Day defaults to 1.
    YearMonth,
}

impl DateShape {
    /// Parses `s` as a full date, falling back to a year-month.
    ///
    /// # Errors
    ///
    /// Returns the static reason the best-fitting shape rejected `s`: the
    /// year-month reason for 7-byte input (`YYYY-MM`), the full-date reason
    /// otherwise.
    pub(crate) fn parse_any(
        s: &str,
    ) -> Result<(NaiveDate, DatePrecision), &'static str> {
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

    fn parse_precision(
        self,
        s: &str,
    ) -> Result<(NaiveDate, DatePrecision), &'static str> {
        match self {
            Self::YearMonthDay => NaiveDate::parse_from_str(s, DATE_PATTERN)
                .map(|d| (d, DatePrecision::Day))
                .map_err(parse_reason),
            Self::YearMonth => {
                Self::parse_year_month(s).map(|d| (d, DatePrecision::Month))
            }
        }
    }

    /// Parses `"YYYY-MM"`, defaulting the day to 1.
    fn parse_year_month(s: &str) -> Result<NaiveDate, &'static str> {
        let (year_str, month_str) =
            s.split_once('-').ok_or("not enough input")?;
        if year_str.len() != 4
            || month_str.len() != 2
            || !month_str.bytes().all(|b| b.is_ascii_digit())
        {
            return Err("not enough input");
        }
        let year: i32 = year_str.parse().map_err(|_| "invalid input")?;
        let month: u32 = month_str.parse().map_err(|_| "invalid input")?;
        NaiveDate::from_ymd_opt(year, month, 1).ok_or("a field is out of range")
    }
}

/// Renders as the canonical zero-padded `YYYY-MM-DD` string.
impl fmt::Display for DateValue {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format(DATE_PATTERN))
    }
}

impl From<DateValue> for NaiveDate {
    #[inline]
    fn from(value: DateValue) -> Self {
        value.naive_date()
    }
}
impl From<NaiveDate> for DateValue {
    #[inline]
    fn from(date: NaiveDate) -> Self {
        Self(date)
    }
}

/// Converts to the calendar day `time` falls on in the local zone, not the
/// UTC calendar day.
///
/// A file's date means the reader's local day, the same doctrine that reads
/// naive date-time input on the local clock: an instant in the last hours of
/// December 31 locally is already January 1 in UTC, or the reverse.
impl From<SystemTime> for DateValue {
    #[inline]
    fn from(time: SystemTime) -> Self {
        DateTimeValue::from(time).local_date()
    }
}

impl FromStr for DateValue {
    type Err = DateError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_iso(s)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{temporal::fixtures::ymd, test_support::TzGuard};

    mod constructor {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

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
        #[case::year_zero("0000-01-01", (0, 1, 1))]
        #[case::year_9999("9999-12-31", (9999, 12, 31))]
        fn accepts_the_extreme_four_digit_years(
            #[case] input: &str,
            #[case] (year, month, day): (i32, u32, u32),
        ) {
            // Arrange: `input` and the expected fields are the case literals.
            // Act
            let result = input.parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Ok(ymd(year, month, day)),
                "the four-digit rule admits `{input}`"
            );
        }

        #[test]
        fn rejects_negative_years_before_the_four_digit_rule() {
            // Arrange: `-001` begins with a sign, not four ASCII digits.
            // Act
            let result = "-001-01-01".parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Err(DateError::YearNotFourDigits {
                    input: "-001-01-01".into(),
                }),
                "years below `0000` fall outside the four-digit rule"
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
        #[case::unpadded_month_and_day("2026-1-1", (2026, 1, 1))]
        #[case::unpadded_day("2026-07-2", (2026, 7, 2))]
        fn accepts_unpadded_dates(
            #[case] input: &str,
            #[case] (year, month, day): (i32, u32, u32),
        ) {
            // Arrange: `input` and the expected fields are the case literals.
            // Act
            let result = input.parse::<DateValue>();
            // Assert
            assert_eq!(
                result,
                Ok(ymd(year, month, day)),
                "`{input}` is accepted while padding stays lenient"
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
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn converts_a_system_time_to_the_local_calendar_day() {
            // Arrange: UTC-05:00 has no DST; 1000 seconds after the epoch is
            // 00:16:40Z on January 1, which is 19:16:40 on December 31 local.
            TzGuard::set("Etc/GMT+5");
            let system_time =
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000);

            // Act
            let date = DateValue::from(system_time);

            // Assert
            assert_eq!(
                date,
                ymd(1969, 12, 31),
                "a file's date is the reader's local day, not the UTC day"
            );
        }

        #[test]
        fn converts_a_system_time_to_the_date_under_the_utc_zone() {
            // Arrange: under UTC the local day and the UTC day agree.
            TzGuard::set("UTC");
            let system_time =
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000);

            // Act
            let date = DateValue::from(system_time);

            // Assert
            assert_eq!(
                date,
                ymd(1970, 1, 1),
                "the epoch's UTC day converts unchanged under UTC"
            );
        }

        #[test]
        fn wraps_a_chrono_date_through_the_from_trait() {
            // Arrange
            let chrono_date = NaiveDate::from_ymd_opt(2026, 7, 29)
                .expect("valid fixture date");

            // Act
            let date = DateValue::from(chrono_date);

            // Assert
            assert_eq!(
                date,
                ymd(2026, 7, 29),
                "the From impl wraps without loss"
            );
        }

        #[test]
        fn unwraps_to_a_chrono_date_through_the_from_trait() {
            // Arrange
            let date = ymd(2026, 7, 29);
            let expected = NaiveDate::from_ymd_opt(2026, 7, 29)
                .expect("valid fixture date");

            // Act
            let chrono_date = NaiveDate::from(date);

            // Assert
            assert_eq!(
                chrono_date, expected,
                "the inverse From impl unwraps without loss"
            );
        }

        #[rstest]
        #[case::minimum(NaiveDate::MIN)]
        #[case::maximum(NaiveDate::MAX)]
        fn wraps_the_extremes_of_the_chrono_date_range(
            #[case] extreme: NaiveDate,
        ) {
            // Arrange: `extreme` is the case literal, a year far outside the
            // four-digit parse rule.
            // Act
            let round_tripped = NaiveDate::from(DateValue::from(extreme));
            // Assert
            assert_eq!(
                round_tripped, extreme,
                "construction never narrows the wrapped range"
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
}

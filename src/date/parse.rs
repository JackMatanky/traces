use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc, format::ParseErrorKind};

use super::{
    error::DateError,
    value::{DATE_PATTERN, DatePoint, DatePrecision, DateTimeValue, DateValue},
};

impl DatePoint {
    /// Parses a date or date-time shape, retaining date precision.
    ///
    /// # Errors
    ///
    /// - [`DateError::UnknownShape`] if the trimmed input does not start with
    ///   two or four ASCII digits followed by `-`.
    /// - [`DateError::YearNotFourDigits`] if it starts that way but the year is
    ///   not four ASCII digits.
    /// - [`DateError::InvalidValue`] if no date or date-time shape matches, or
    ///   a matched field is out of range.
    /// - [`DateError::LocalZoneLookup`] if a naive date-time cannot be resolved
    ///   in the local zone.
    pub(crate) fn parse(s: &str) -> Result<Self, DateError> {
        let trimmed = s.trim();
        if !starts_like_date(trimmed.as_bytes()) {
            return Err(DateError::UnknownShape);
        }
        if !starts_with_year(trimmed) {
            return Err(DateError::YearNotFourDigits {
                input: trimmed.into(),
            });
        }
        let calendar_point = |(date, date_precision)| Self::Calendar {
            date: DateValue::from(date),
            date_precision,
        };
        // Date-only shapes cannot match the date-time cascade.
        if trimmed.len() <= 10 {
            return DateShape::parse_any(trimmed).map(calendar_point).map_err(
                |reason| DateError::InvalidValue {
                    input: trimmed.into(),
                    reason,
                },
            );
        }
        match DateTimeShape::parse_any(trimmed) {
            Ok(ParsedDateTime::Instant(instant)) => {
                Ok(Self::Instant(DateTimeValue::from(instant)))
            }
            Ok(ParsedDateTime::Unzoned(local_datetime)) => {
                Ok(Self::Instant(DateTimeValue::from_local(local_datetime)?))
            }
            Err(datetime_err) => match DateShape::parse_any(trimmed) {
                Ok(date) => Ok(calendar_point(date)),
                Err(date_err) => {
                    let reason = if matches!(
                        trimmed.as_bytes().get(10),
                        Some(b'T' | b' ')
                    ) {
                        datetime_err
                    } else {
                        date_err
                    };
                    Err(DateError::InvalidValue {
                        input: trimmed.into(),
                        reason,
                    })
                }
            },
        }
    }

    /// Parses a 10-byte `YYYY-MM-DD` or 7-byte `YYYY-MM` date prefix of `s`.
    ///
    /// Returns the date point and the number of bytes consumed, or [`None`]
    /// when `s` does not start with either shape.
    #[must_use]
    pub(crate) fn parse_prefix(s: &str) -> Option<(Self, usize)> {
        let bytes = s.as_bytes();
        if bytes.len() >= 8 && bytes.get(7) == Some(&b'-') {
            if let Some(prefix) = s.get(..10)
                && let Ok(
                    date_point @ Self::Calendar {
                        date_precision: DatePrecision::Day,
                        ..
                    },
                ) = Self::parse(prefix)
            {
                return Some((date_point, 10));
            }
            return None;
        }
        if let Some(prefix) = s.get(..7)
            && let Ok(
                date_point @ Self::Calendar {
                    date_precision: DatePrecision::Month,
                    ..
                },
            ) = Self::parse(prefix)
        {
            return Some((date_point, 7));
        }
        None
    }

    /// Parses a date or date-time with a chrono strftime pattern.
    ///
    /// # Errors
    ///
    /// - [`DateError::InvalidValue`] if `text` matches `pattern` neither as an
    ///   offset date-time, a naive date-time, nor a date.
    /// - [`DateError::LocalZoneLookup`] if a naive date-time match cannot be
    ///   resolved in the local zone.
    pub(crate) fn parse_with_pattern(
        text: &str,
        pattern: &str,
    ) -> Result<Self, DateError> {
        let trimmed_text = text.trim();
        let trimmed_pattern = pattern.trim();
        if let Ok(dt) = DateTime::parse_from_str(trimmed_text, trimmed_pattern)
        {
            return Ok(Self::Instant(DateTimeValue::from(
                dt.with_timezone(&Utc),
            )));
        }
        if let Ok(local_datetime) =
            NaiveDateTime::parse_from_str(trimmed_text, trimmed_pattern)
        {
            return Ok(Self::Instant(DateTimeValue::from_local(
                local_datetime,
            )?));
        }
        if trimmed_pattern == "%Y-%m"
            && let Ok(
                date_point @ Self::Calendar {
                    date_precision: DatePrecision::Month,
                    ..
                },
            ) = Self::parse(trimmed_text)
        {
            return Ok(date_point);
        }
        NaiveDate::parse_from_str(trimmed_text, trimmed_pattern)
            .map(|date| Self::Calendar {
                date: DateValue::from(date),
                date_precision: DatePrecision::Day,
            })
            .map_err(|source| DateError::InvalidValue {
                input: trimmed_text.into(),
                reason: parse_reason(source),
            })
    }
}

impl DateValue {
    /// Parses an ISO-8601 date string (`YYYY-MM-DD` or `YYYY-MM`) using the
    /// recognition layer.
    ///
    /// # Errors
    ///
    /// - [`DateError::YearNotFourDigits`] if `s` does not begin with 4 ASCII
    ///   digits.
    /// - [`DateError::InvalidValue`] if no accepted shape matches.
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

impl DateTimeValue {
    /// Parses an RFC 3339 or ISO-8601 date-time string using
    /// [`DateTimeShape::ALL`].
    ///
    /// Surrounding whitespace is ignored. The cascade matches shapes only: a
    /// matched naive input then resolves through [`DateTimeValue::from_local`]
    /// (an explicit-offset input already is the instant). Resolution runs
    /// once after the cascade, because a resolution failure must not retry
    /// later formats.
    ///
    /// # Errors
    ///
    /// - [`DateError::YearNotFourDigits`] if `s` does not begin with 4 ASCII
    ///   digits.
    /// - [`DateError::InvalidValue`] if no accepted shape matches.
    /// - [`DateError::LocalZoneLookup`] if the local timezone lookup fails
    ///   while resolving a naive input.
    pub(crate) fn parse_iso(s: &str) -> Result<Self, DateError> {
        let trimmed = s.trim();
        if !starts_with_year(trimmed) {
            return Err(DateError::YearNotFourDigits {
                input: trimmed.into(),
            });
        }
        let shape = DateTimeShape::parse_any(trimmed).map_err(|reason| {
            DateError::InvalidValue {
                input: trimmed.into(),
                reason,
            }
        })?;
        match shape {
            ParsedDateTime::Instant(instant) => Ok(Self::from(instant)),
            ParsedDateTime::Unzoned(local_datetime) => {
                Self::from_local(local_datetime)
            }
        }
    }
}

/// Recognized date input format shapes, tried in order by
/// [`DateShape::parse_any`] for [`DatePoint::parse`] and
/// [`DateValue::parse_iso`].
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

/// Recognized date-time input format shapes tried in order by
/// [`DateTimeValue::parse_iso`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DateTimeShape {
    /// RFC 3339 format with offset: `2026-07-29T14:30:00Z`.
    Rfc3339,
    /// `T`-separated seconds, optionally fractional: `2026-07-29T14:30:00.123`.
    LocalTSeconds,
    /// `T`-separated minute-only precision: `2026-07-29T14:30`.
    LocalTMinutes,
    /// Space-separated seconds, optionally fractional: `2026-07-29
    /// 14:30:00.123`.
    LocalSpaceSeconds,
    /// Space-separated minute-only precision: `2026-07-29 14:30`.
    LocalSpaceMinutes,
}

impl DateTimeShape {
    /// Formats tried in order by [`DateTimeValue::parse_iso`] (most
    /// specific/unambiguous first).
    pub(crate) const ALL: [Self; 5] = [
        Self::Rfc3339,
        Self::LocalTSeconds,
        Self::LocalTMinutes,
        Self::LocalSpaceSeconds,
        Self::LocalSpaceMinutes,
    ];

    /// Returns the strftime pattern for this format, or `None` for
    /// [`Self::Rfc3339`] which uses dedicated RFC 3339 parsing.
    #[must_use]
    pub(crate) const fn pattern(self) -> Option<&'static str> {
        match self {
            Self::Rfc3339 => None,
            Self::LocalTSeconds => Some("%Y-%m-%dT%H:%M:%S%.f"),
            Self::LocalTMinutes => Some("%Y-%m-%dT%H:%M"),
            Self::LocalSpaceSeconds => Some("%Y-%m-%d %H:%M:%S%.f"),
            Self::LocalSpaceMinutes => Some("%Y-%m-%d %H:%M"),
        }
    }

    /// Parses `s` as the first matching shape in [`Self::ALL`] order.
    ///
    /// # Errors
    ///
    /// Returns the static reason chrono gave for the last shape tried when no
    /// shape matches.
    pub(crate) fn parse_any(s: &str) -> Result<ParsedDateTime, &'static str> {
        let [first, rest @ ..] = Self::ALL;
        let mut result = first.parse_shape(s);
        for format in rest {
            if result.is_ok() {
                break;
            }
            result = format.parse_shape(s);
        }
        result.map_err(parse_reason)
    }

    /// Parses according to this format, returning a parsed datetime shape.
    fn parse_shape(
        self,
        s: &str,
    ) -> Result<ParsedDateTime, chrono::ParseError> {
        match self.pattern() {
            None => DateTime::parse_from_rfc3339(s)
                .map(|dt| ParsedDateTime::Instant(dt.with_timezone(&Utc))),
            Some(pat) => NaiveDateTime::parse_from_str(s, pat)
                .map(ParsedDateTime::Unzoned),
        }
    }
}

/// The parsed shape of a date-time string before local-zone resolution.
#[derive(Copy, Clone, Debug)]
pub(crate) enum ParsedDateTime {
    /// Explicit RFC 3339 instant.
    Instant(DateTime<Utc>),
    /// Local naive datetime that must resolve through
    /// [`DateTimeValue::from_local`].
    Unzoned(NaiveDateTime),
}

/// Maps a chrono parse failure to a short static reason, so errors carry no
/// allocation or foreign error type.
fn parse_reason(error: chrono::ParseError) -> &'static str {
    match error.kind() {
        ParseErrorKind::OutOfRange => "a field is out of range",
        ParseErrorKind::Impossible => "the date does not exist",
        ParseErrorKind::NotEnough => "not enough input",
        ParseErrorKind::TooShort => "input is too short",
        ParseErrorKind::TooLong => "unexpected trailing input",
        ParseErrorKind::BadFormat => "bad format pattern",
        _ => "invalid input",
    }
}

/// Returns `true` if `bytes` begins with a date-like prefix: two or four
/// ASCII digits followed by `-`.
fn starts_like_date(bytes: &[u8]) -> bool {
    bytes.len() >= 5
        && [2, 4].into_iter().any(|digits| {
            bytes
                .get(..digits)
                .is_some_and(|b| b.iter().all(u8::is_ascii_digit))
                && bytes.get(digits) == Some(&b'-')
        })
}

/// Returns `true` if `s`'s first four bytes are ASCII digits.
fn starts_with_year(s: &str) -> bool {
    s.as_bytes().get(..4).is_some_and(|b| b.iter().all(u8::is_ascii_digit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TzGuard;

    mod parsing {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::utc_offset("2026-07-29T14:30:00Z", "2026-07-29T14:30:00")]
        #[case::numeric_offset(
            "2026-07-29T14:30:00+02:00",
            "2026-07-29T12:30:00"
        )]
        #[case::fractional_seconds(
            "2026-07-29T14:30:00.123",
            "2026-07-29T14:30:00.123"
        )]
        #[case::t_separated_with_seconds(
            "2026-07-29T14:30:00",
            "2026-07-29T14:30:00"
        )]
        #[case::t_separated_minute_only(
            "2026-07-29T14:30",
            "2026-07-29T14:30:00"
        )]
        #[case::space_separated_fractional(
            "2026-07-29 14:30:00.123",
            "2026-07-29T14:30:00.123"
        )]
        #[case::space_separated_with_seconds(
            "2026-07-29 14:30:00",
            "2026-07-29T14:30:00"
        )]
        #[case::space_separated_minute_only(
            "2026-07-29 14:30",
            "2026-07-29T14:30:00"
        )]
        fn parses_every_datetime_shape_to_the_expected_instant(
            #[case] input: &str,
            #[case] expected_utc_rendering: &str,
        ) {
            // Arrange
            TzGuard::set("UTC");

            // Act
            let parsed = DateTimeValue::parse_iso(input).expect("valid shape");

            // Assert
            assert_eq!(parsed.to_string(), expected_utc_rendering);
        }

        #[rstest]
        #[case::full_date("2026-07-29", (2026, 7, 29))]
        #[case::year_month("2026-08", (2026, 8, 1))]
        #[case::missing_leading_zeros("2026-8-22", (2026, 8, 22))]
        fn parses_date_shapes_to_the_expected_date(
            #[case] input: &str,
            #[case] (year, month, day): (i32, u32, u32),
        ) {
            // Act
            let parsed = DateValue::parse_iso(input).expect("valid shape");

            // Assert
            let expected = NaiveDate::from_ymd_opt(year, month, day)
                .expect("valid fixture date");
            assert_eq!(parsed, DateValue::from(expected));
        }

        #[rstest]
        #[case::too_short("202")]
        #[case::empty("")]
        #[case::non_digit_year("abcd-01-01")]
        fn rejects_a_missing_four_digit_year(#[case] input: &str) {
            assert!(!starts_with_year(input));
        }

        #[test]
        fn accepts_a_five_digit_year_prefix_as_having_four_digits() {
            // The `starts_with_year` guard only checks the first 4 bytes;
            // a longer numeric prefix still passes this guard and is rejected
            // later, by the format cascade, as InvalidValue rather than
            // YearNotFourDigits.
            assert!(starts_with_year("20265-01-01"));
            assert!(matches!(
                DateValue::parse_iso("20265-01-01"),
                Err(DateError::InvalidValue { .. })
            ));
        }

        #[test]
        fn date_value_rejects_a_short_year_with_year_not_four_digits() {
            let result = DateValue::parse_iso("26-08-22");
            assert!(matches!(result, Err(DateError::YearNotFourDigits { .. })));
        }

        #[test]
        fn datetime_value_rejects_a_short_year_with_year_not_four_digits() {
            let result = DateTimeValue::parse_iso("26-08-22T14:30:00");
            assert!(matches!(result, Err(DateError::YearNotFourDigits { .. })));
        }

        #[rstest]
        #[case::invalid_month_number("2026-13")]
        #[case::non_numeric_month("2026-ab")]
        #[case::single_digit_month("2026-7")]
        #[case::slashes("2026/08/22")]
        fn date_value_rejects_malformed_input_as_invalid_value(
            #[case] input: &str,
        ) {
            assert!(matches!(
                DateValue::parse_iso(input),
                Err(DateError::InvalidValue { .. })
            ));
        }

        #[test]
        fn datetime_value_rejects_an_unrecognized_shape_as_invalid_value() {
            let result = DateTimeValue::parse_iso("2026-07-29Tinvalid");
            assert!(matches!(result, Err(DateError::InvalidValue { .. })));
        }

        #[test]
        fn date_time_shape_parses_whole_seconds_via_the_fractional_shape() {
            // Arrange
            let local_datetime = NaiveDate::from_ymd_opt(2026, 7, 29)
                .and_then(|date| date.and_hms_opt(14, 30, 0))
                .expect("valid fixture datetime");

            // Act
            let t_separated = DateTimeShape::parse_any("2026-07-29T14:30:00");
            let space_separated =
                DateTimeShape::parse_any("2026-07-29 14:30:00");

            // Assert
            assert!(matches!(
                t_separated,
                Ok(ParsedDateTime::Unzoned(parsed)) if parsed == local_datetime
            ));
            assert!(matches!(
                space_separated,
                Ok(ParsedDateTime::Unzoned(parsed)) if parsed == local_datetime
            ));
        }

        #[rstest]
        #[case::rfc3339(DateTimeShape::Rfc3339, None)]
        #[case::t_seconds(
            DateTimeShape::LocalTSeconds,
            Some("%Y-%m-%dT%H:%M:%S%.f")
        )]
        #[case::t_minutes(DateTimeShape::LocalTMinutes, Some("%Y-%m-%dT%H:%M"))]
        #[case::space_seconds(
            DateTimeShape::LocalSpaceSeconds,
            Some("%Y-%m-%d %H:%M:%S%.f")
        )]
        #[case::space_minutes(
            DateTimeShape::LocalSpaceMinutes,
            Some("%Y-%m-%d %H:%M")
        )]
        fn date_time_shape_maps_to_its_strftime_pattern(
            #[case] shape: DateTimeShape,
            #[case] expected: Option<&str>,
        ) {
            assert_eq!(shape.pattern(), expected);
        }
    }

    mod date_point_parsing {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::day("2026-07-29", DatePrecision::Day)]
        #[case::month("2026-07", DatePrecision::Month)]
        #[case::unpadded_day("2026-1-1", DatePrecision::Day)]
        fn parse_keeps_the_written_date_precision(
            #[case] text: &str,
            #[case] expected: DatePrecision,
        ) {
            // Act
            let parsed = DatePoint::parse(text).expect("valid date");

            // Assert
            assert!(matches!(
                parsed,
                DatePoint::Calendar { date_precision, .. }
                    if date_precision == expected
            ));
        }

        #[rstest]
        #[case::offset("2026-07-29T14:30:00Z")]
        #[case::t_separated("2026-07-29T14:30:00")]
        #[case::space_separated_fractional("2026-07-29 14:30:00.123")]
        fn parse_returns_an_instant_for_date_times(#[case] text: &str) {
            // Arrange
            TzGuard::set("UTC");

            // Act
            let parsed = DatePoint::parse(text).expect("valid date-time");

            // Assert
            assert!(matches!(parsed, DatePoint::Instant(_)));
        }

        #[test]
        fn parse_denotes_the_first_day_for_a_year_month() {
            // Act
            let year_month = DatePoint::parse("2026-07").expect("valid");

            // Assert
            assert_eq!(
                year_month.date(),
                DateValue::parse_iso("2026-07-01").expect("valid")
            );
        }

        #[rstest]
        #[case::word("hello")]
        #[case::empty("")]
        #[case::year_only("2026")]
        #[case::slashes("2026/08/22")]
        #[case::five_digit_year("20265-01-01")]
        fn parse_rejects_text_without_a_date_shape(#[case] text: &str) {
            assert_eq!(DatePoint::parse(text), Err(DateError::UnknownShape));
        }

        #[test]
        fn parse_rejects_a_two_digit_year() {
            assert!(matches!(
                DatePoint::parse("26-08-22"),
                Err(DateError::YearNotFourDigits { .. })
            ));
        }

        #[rstest]
        #[case::month_out_of_range(
            "2026-13",
            "`2026-13` is not a recognized date/time: a field is out of range"
        )]
        #[case::impossible_day(
            "2026-02-30",
            "`2026-02-30` is not a recognized date/time: a field is out of \
             range"
        )]
        fn parse_reports_the_reason_the_date_was_rejected(
            #[case] text: &str,
            #[case] expected_message: &str,
        ) {
            // Act
            let error = DatePoint::parse(text).expect_err("invalid date");

            // Assert
            assert_eq!(error.to_string(), expected_message);
        }

        #[rstest]
        #[case::signed_month("2026-+7")]
        #[case::trailing_garbage("2026-07-29Tinvalid")]
        fn parse_rejects_malformed_values(#[case] text: &str) {
            assert!(matches!(
                DatePoint::parse(text),
                Err(DateError::InvalidValue { .. })
            ));
        }

        #[rstest]
        #[case::day("2026-07-29T14:30:00", 10, DatePrecision::Day)]
        #[case::month("2026-07 remainder", 7, DatePrecision::Month)]
        fn parse_prefix_returns_the_date_and_its_length(
            #[case] text: &str,
            #[case] expected_length: usize,
            #[case] expected_precision: DatePrecision,
        ) {
            // Act
            let (parsed, length) =
                DatePoint::parse_prefix(text).expect("date prefix");

            // Assert
            assert_eq!(length, expected_length);
            assert!(matches!(
                parsed,
                DatePoint::Calendar { date_precision, .. }
                    if date_precision == expected_precision
            ));
        }

        #[rstest]
        #[case::word("not a date")]
        #[case::signed_month("2026-+7 later")]
        fn parse_prefix_returns_none_without_a_date_prefix(#[case] text: &str) {
            assert_eq!(DatePoint::parse_prefix(text), None);
        }
    }

    mod parse_with_pattern {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::iso_date("2026-07-29", "%Y-%m-%d", "2026-07-29")]
        #[case::custom_date("29/07/2026", "%d/%m/%Y", "2026-07-29")]
        #[case::date_time(
            "2026-07-29 14:30:00",
            "%Y-%m-%d %H:%M:%S",
            "2026-07-29T14:30:00"
        )]
        #[case::year_month("2026-07", "%Y-%m", "2026-07")]
        fn parses_text_matching_the_pattern(
            #[case] text: &str,
            #[case] pattern: &str,
            #[case] expected_text: &str,
        ) {
            // Arrange
            TzGuard::set("UTC");
            let expected = DatePoint::parse(expected_text).expect("valid");

            // Act
            let parsed = DatePoint::parse_with_pattern(text, pattern);

            // Assert
            assert_eq!(parsed, Ok(expected));
        }

        #[rstest]
        #[case::bad_pattern("2026-07-29", "%Q")]
        #[case::not_a_date("not-a-date", "%Y-%m-%d")]
        #[case::mismatched_pattern("2026-07-29", "%d/%m/%Y")]
        fn rejects_text_not_matching_the_pattern(
            #[case] text: &str,
            #[case] pattern: &str,
        ) {
            assert!(matches!(
                DatePoint::parse_with_pattern(text, pattern),
                Err(DateError::InvalidValue { .. })
            ));
        }
    }
}

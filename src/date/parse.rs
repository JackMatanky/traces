use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc, format::ParseErrorKind};

use super::{
    error::DateError,
    value::{
        CivilPrecision, DATE_PATTERN, DateTimeValue, DateValue, Point,
        resolve_local,
    },
};

/// Returns `true` if `bytes` begins with a date-like prefix.
fn starts_like_date(bytes: &[u8]) -> bool {
    if bytes.len() >= 5
        && bytes.get(0..2).is_some_and(|b| b.iter().all(u8::is_ascii_digit))
        && bytes.get(2) == Some(&b'-')
    {
        return true;
    }
    if bytes.len() >= 5
        && bytes.get(0..4).is_some_and(|b| b.iter().all(u8::is_ascii_digit))
        && bytes.get(4) == Some(&b'-')
    {
        return true;
    }
    false
}

/// Returns `true` if `s`'s first four bytes are ASCII digits.
fn starts_with_year(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.get(0..4).is_some_and(|b| b.iter().all(u8::is_ascii_digit))
}

impl Point {
    /// Parses a date or date-time shape, retaining civil precision.
    ///
    /// # Errors
    ///
    /// Returns [`DateError::UnknownShape`] when no date-like prefix exists, or
    /// the corresponding parsing or local-zone error for invalid dates.
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
        let civil = |(date, precision)| Self::Civil {
            date: DateValue(date),
            precision,
        };
        // Date-only shapes cannot match the date-time cascade.
        if trimmed.len() <= 10 {
            return DateShape::parse_any(trimmed).map(civil).map_err(
                |reason| DateError::InvalidValue {
                    input: trimmed.into(),
                    reason,
                },
            );
        }
        match DateTimeShape::parse_any(trimmed) {
            Ok(ParsedDateTime::Instant(instant)) => {
                Ok(Self::Instant(DateTimeValue(instant)))
            }
            Ok(ParsedDateTime::LocalWall(wall)) => {
                Ok(Self::Instant(DateTimeValue(resolve_local(wall)?)))
            }
            Err(datetime_err) => match DateShape::parse_any(trimmed) {
                Ok(date) => Ok(civil(date)),
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

    /// Parses a 10-byte day or 7-byte month date prefix and its byte length.
    #[must_use]
    pub(crate) fn parse_prefix(s: &str) -> Option<(Self, usize)> {
        let bytes = s.as_bytes();
        if bytes.len() >= 8 && bytes.get(7) == Some(&b'-') {
            if let Some(prefix) = s.get(..10)
                && let Ok(
                    point @ Self::Civil {
                        precision: CivilPrecision::Day,
                        ..
                    },
                ) = Self::parse(prefix)
            {
                return Some((point, 10));
            }
            return None;
        }
        if let Some(prefix) = s.get(..7)
            && let Ok(
                point @ Self::Civil {
                    precision: CivilPrecision::Month,
                    ..
                },
            ) = Self::parse(prefix)
        {
            return Some((point, 7));
        }
        None
    }

    /// Parses a date or date-time with a chrono strftime pattern.
    ///
    /// # Errors
    ///
    /// Returns parsing or local-zone errors when the pattern cannot resolve.
    pub(crate) fn parse_with_pattern(
        text: &str,
        pattern: &str,
    ) -> Result<Self, DateError> {
        let trimmed_text = text.trim();
        let trimmed_pattern = pattern.trim();
        if let Ok(dt) = DateTime::parse_from_str(trimmed_text, trimmed_pattern)
        {
            return Ok(Self::Instant(DateTimeValue(dt.with_timezone(&Utc))));
        }
        if let Ok(wall) =
            NaiveDateTime::parse_from_str(trimmed_text, trimmed_pattern)
        {
            return Ok(Self::Instant(DateTimeValue(resolve_local(wall)?)));
        }
        if trimmed_pattern == "%Y-%m"
            && let Ok(
                point @ Self::Civil {
                    precision: CivilPrecision::Month,
                    ..
                },
            ) = Self::parse(trimmed_text)
        {
            return Ok(point);
        }
        NaiveDate::parse_from_str(trimmed_text, trimmed_pattern)
            .map(|date| Self::Civil {
                date: DateValue(date),
                precision: CivilPrecision::Day,
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
        DateShape::parse_any(trimmed).map(|(date, _)| Self(date)).map_err(
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
    /// matched naive input then resolves through [`resolve_local`] (an
    /// explicit-offset input already is the instant). Resolution runs once
    /// after the cascade, because a resolution failure must not retry later
    /// formats.
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
        let instant = match shape {
            ParsedDateTime::Instant(instant) => instant,
            ParsedDateTime::LocalWall(wall) => resolve_local(wall)?,
        };
        Ok(Self(instant))
    }
}

/// Recognized date input format shapes tried in order by
/// [`DateValue::parse_iso`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DateShape {
    /// Full ISO date: `2026-07-29`.
    YearMonthDay,
    /// Year-month reduced precision: `2026-07`. Day defaults to 1.
    YearMonth,
}

impl DateShape {
    /// Formats tried in order by [`DateValue::parse_iso`].
    pub(crate) const ALL: [Self; 2] = [Self::YearMonthDay, Self::YearMonth];

    /// Attempts to parse `s` across all recognized date formats in priority
    /// order.
    pub(crate) fn parse_any(
        s: &str,
    ) -> Result<(NaiveDate, CivilPrecision), &'static str> {
        let [first, rest @ ..] = Self::ALL;
        let mut result = first.parse_precision(s);
        for format in rest {
            if result.is_ok() {
                break;
            }
            result = format.parse_precision(s);
        }
        result
    }

    fn parse_precision(
        self,
        s: &str,
    ) -> Result<(NaiveDate, CivilPrecision), &'static str> {
        match self {
            Self::YearMonthDay => NaiveDate::parse_from_str(s, DATE_PATTERN)
                .map(|d| (d, CivilPrecision::Day))
                .map_err(parse_reason),
            Self::YearMonth => {
                Self::parse_year_month(s).map(|d| (d, CivilPrecision::Month))
            }
        }
    }

    /// Parses `"YYYY-MM"`, defaulting the day to 1.
    fn parse_year_month(s: &str) -> Result<NaiveDate, &'static str> {
        let (year_str, month_str) =
            s.split_once('-').ok_or("not enough input")?;
        if year_str.len() != 4 || month_str.len() != 2 {
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

    /// Attempts to parse `s` across all recognized date-time formats in
    /// priority order.
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
                .map(ParsedDateTime::LocalWall),
        }
    }
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

/// The parsed shape of a date-time string before local-zone resolution.
#[derive(Copy, Clone, Debug)]
pub(crate) enum ParsedDateTime {
    /// Explicit RFC 3339 instant.
    Instant(DateTime<Utc>),
    /// Local naive datetime that must resolve through [`resolve_local`].
    LocalWall(NaiveDateTime),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{duration::DurationUnit, test_support::TzGuard};

    mod parsing {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::has_offset_z("2026-07-29T14:30:00Z")]
        #[case::has_numeric_offset("2026-07-29T14:30:00+00:00")]
        #[case::has_fractional_seconds("2026-07-29T14:30:00.123")]
        #[case::t_separated_with_seconds("2026-07-29T14:30:00")]
        #[case::t_separated_minute_only("2026-07-29T14:30")]
        #[case::space_separated_fractional("2026-07-29 14:30:00.123")]
        #[case::space_separated_with_seconds("2026-07-29 14:30:00")]
        #[case::space_separated_minute_only("2026-07-29 14:30")]
        fn accepts_every_datetime_shape(#[case] input: &str) {
            TzGuard::keep();

            assert!(
                DateTimeValue::parse_iso(input).is_ok(),
                "{input} should parse"
            );
        }

        #[test]
        fn accepts_full_iso_date_shape() {
            assert!(DateValue::parse_iso("2026-07-29").is_ok());
        }

        #[test]
        fn accepts_year_month_reduced_precision_date_shape() {
            let year_month =
                DateValue::parse_iso("2026-08").expect("year-month parses");
            let expected = DateValue(
                NaiveDate::from_ymd_opt(2026, 8, 1).expect("valid date"),
            );
            assert_eq!(year_month, expected);
        }

        #[test]
        fn accepts_missing_leading_zeros() {
            assert!(DateValue::parse_iso("2026-8-22").is_ok());
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
        fn date_value_rejects_a_short_year_with_invalid_year_digits() {
            let result = DateValue::parse_iso("26-08-22");
            assert!(matches!(result, Err(DateError::YearNotFourDigits { .. })));
        }

        #[test]
        fn datetime_value_rejects_a_short_year_with_invalid_year_digits() {
            let result = DateTimeValue::parse_iso("26-08-22T14:30:00");
            assert!(matches!(result, Err(DateError::YearNotFourDigits { .. })));
        }

        #[rstest]
        #[case::invalid_month_number("2026-13")]
        #[case::non_numeric_month("2026-ab")]
        #[case::single_digit_month("2026-7")]
        fn rejects_a_malformed_year_month_as_unparseable(#[case] input: &str) {
            assert!(matches!(
                DateValue::parse_iso(input),
                Err(DateError::InvalidValue { .. })
            ));
        }

        #[test]
        fn rejects_an_unrecognized_date_shape_as_unparseable() {
            let result = DateValue::parse_iso("2026/08/22");
            assert!(matches!(result, Err(DateError::InvalidValue { .. })));
        }

        #[test]
        fn rejects_an_unrecognized_datetime_shape_as_unparseable() {
            let result = DateTimeValue::parse_iso("2026-07-29Tinvalid");
            assert!(matches!(result, Err(DateError::InvalidValue { .. })));
        }

        #[test]
        fn date_value_parses_via_from_str() {
            let date: DateValue = "2026-07-29".parse().expect("valid date");
            let expected =
                DateValue::parse_iso("2026-07-29").expect("valid date");
            assert_eq!(date, expected);
        }

        #[test]
        fn datetime_value_parses_via_from_str() {
            TzGuard::keep();

            let datetime: DateTimeValue =
                "2026-07-29T14:30:00".parse().expect("valid datetime");
            let expected = DateTimeValue::parse_iso("2026-07-29T14:30:00")
                .expect("valid datetime");
            assert_eq!(datetime, expected);
        }

        #[test]
        fn date_time_format_whole_seconds_parse_via_the_fractional_variant() {
            // Arrange
            let wall = NaiveDate::from_ymd_opt(2026, 7, 29)
                .unwrap()
                .and_hms_opt(14, 30, 0)
                .unwrap();

            // Act & Assert
            for text in ["2026-07-29T14:30:00", "2026-07-29 14:30:00"] {
                assert!(
                    matches!(DateTimeShape::parse_any(text), Ok(ParsedDateTime::LocalWall(w)) if w == wall)
                );
            }
        }

        #[test]
        fn datetime_format_pattern_maps_each_variant_to_its_strftime_spec() {
            assert_eq!(DateTimeShape::Rfc3339.pattern(), None);
            assert_eq!(
                DateTimeShape::LocalSpaceSeconds.pattern(),
                Some("%Y-%m-%d %H:%M:%S%.f")
            );
            assert_eq!(
                DateTimeShape::LocalSpaceMinutes.pattern(),
                Some("%Y-%m-%d %H:%M")
            );
        }
    }

    mod point_parsing {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn parse_short_inputs_match_pre_gate_results() {
            let cases = [
                ("2026-07-29", "%Y-%m-%d"),
                ("2026-07", "%Y-%m"),
                ("2026-1-1", "%Y-%m-%d"),
            ];

            for (text, pattern) in cases {
                assert_eq!(
                    Point::parse(text).unwrap().canonical_pattern(),
                    pattern
                );
            }
            assert!(matches!(
                Point::parse("26-08-22"),
                Err(DateError::YearNotFourDigits { .. })
            ));
            assert_eq!(
                Point::parse("2026/08/22"),
                Err(DateError::UnknownShape)
            );

            let failures = ["2026-13", "2026-02-30"];
            let messages = failures.map(|text| {
                let err = Point::parse(text).unwrap_err();
                assert!(matches!(err, DateError::InvalidValue { .. }));
                err.to_string()
            });
            assert_eq!(messages, [
                "`2026-13` is not a recognized date/time: a field is out of \
                 range",
                "`2026-02-30` is not a recognized date/time: not enough input",
            ]);
        }

        #[test]
        fn returns_unknown_shape_for_non_temporal_text() {
            for text in ["hello", "", "2026", "2026/08/22", "20265-01-01"] {
                assert_eq!(Point::parse(text), Err(DateError::UnknownShape));
            }
        }

        #[test]
        fn returns_error_for_shape_matching_but_invalid_input() {
            let res1 = Point::parse("2026-13");
            assert!(matches!(res1, Err(DateError::InvalidValue { .. })));

            let res2 = Point::parse("26-08-22");
            assert!(matches!(res2, Err(DateError::YearNotFourDigits { .. })));

            let res3 = Point::parse("2026-07-29Tinvalid");
            assert!(matches!(res3, Err(DateError::InvalidValue { .. })));
        }

        #[test]
        fn returns_point_with_appropriate_precision() {
            TzGuard::keep();

            let ym = Point::parse("2026-07").expect("valid year-month");
            assert_eq!(ym.canonical_pattern(), "%Y-%m");
            assert_eq!(ym.date(), DateValue::parse_iso("2026-07-01").unwrap());

            let d = Point::parse("2026-07-29").expect("valid date");
            assert_eq!(d.canonical_pattern(), "%Y-%m-%d");
            assert_eq!(d.date(), DateValue::parse_iso("2026-07-29").unwrap());

            let dt =
                Point::parse("2026-07-29T14:30:00").expect("valid datetime");
            assert_eq!(dt.canonical_pattern(), "%Y-%m-%dT%H:%M:%S");

            let dt_space_frac = Point::parse("2026-07-29 14:30:00.123")
                .expect("valid fractional datetime");
            assert_eq!(
                dt_space_frac.canonical_pattern(),
                "%Y-%m-%dT%H:%M:%S%.f"
            );
        }

        #[test]
        fn demo_contract_year_month_shifts_and_reformats_as_year_month() {
            let recognized = Point::parse("2026-07").expect("valid year-month");
            assert_eq!(recognized.canonical_pattern(), "%Y-%m");

            let shifted_point =
                recognized.shift(1, DurationUnit::Month).expect("shifted");
            let pat = shifted_point.canonical_pattern();
            assert_eq!(pat, "%Y-%m");
            assert_eq!(shifted_point.wall().format(pat).to_string(), "2026-08");
        }

        #[test]
        fn equates_values_at_different_precisions_denoting_the_same_day() {
            let ym = DateValue::parse_iso("2026-07").expect("valid");
            let d = DateValue::parse_iso("2026-07-01").expect("valid");
            assert_eq!(ym, d);

            let ym_rec = Point::parse("2026-07").unwrap();
            let d_rec = Point::parse("2026-07-01").unwrap();
            assert_eq!(ym_rec.canonical_pattern(), "%Y-%m");
            assert_eq!(d_rec.canonical_pattern(), "%Y-%m-%d");
            assert_eq!(ym_rec.date(), d_rec.date());
        }

        #[test]
        fn round_trips_fractional_seconds_through_shift_and_format() {
            TzGuard::set("UTC");

            let recognized = Point::parse("2026-07-29T14:30:00.123")
                .expect("valid fractional datetime");
            assert_eq!(recognized.canonical_pattern(), "%Y-%m-%dT%H:%M:%S%.f");

            let shifted =
                recognized.shift(1, DurationUnit::Hour).expect("shifted");
            let pat = shifted.canonical_pattern();
            assert_eq!(
                shifted.wall().format(pat).to_string(),
                "2026-07-29T15:30:00.123"
            );
        }

        #[test]
        fn parse_prefix_extracts_date_and_year_month_with_consumed_lengths() {
            let (d, consumed_d) =
                Point::parse_prefix("2026-07-29T14:30:00").expect("prefix");
            assert_eq!(consumed_d, 10);
            assert_eq!(d.canonical_pattern(), "%Y-%m-%d");
            assert_eq!(d.date(), DateValue::parse_iso("2026-07-29").unwrap());

            let (ym, consumed_ym) =
                Point::parse_prefix("2026-07 remainder").expect("prefix");
            assert_eq!(consumed_ym, 7);
            assert_eq!(ym.canonical_pattern(), "%Y-%m");
            assert_eq!(ym.date(), DateValue::parse_iso("2026-07-01").unwrap());

            assert!(Point::parse_prefix("not a date").is_none());
        }
    }

    mod parse_with_pattern_and_boundaries {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn parse_with_pattern_parses_matching_date_and_datetime_formats() {
            TzGuard::set("UTC");
            let parsed_date =
                Point::parse_with_pattern("2026-07-29", "%Y-%m-%d")
                    .expect("valid date");
            assert_eq!(parsed_date.canonical_pattern(), "%Y-%m-%d");
            assert_eq!(
                parsed_date.date(),
                DateValue::parse_iso("2026-07-29").unwrap()
            );

            let parsed_custom =
                Point::parse_with_pattern("29/07/2026", "%d/%m/%Y")
                    .expect("valid custom date");
            assert_eq!(
                parsed_custom.date(),
                DateValue::parse_iso("2026-07-29").unwrap()
            );

            let parsed_dt = Point::parse_with_pattern(
                "2026-07-29 14:30:00",
                "%Y-%m-%d %H:%M:%S",
            )
            .expect("valid datetime");
            assert_eq!(parsed_dt.canonical_pattern(), "%Y-%m-%dT%H:%M:%S");
            assert_eq!(
                parsed_dt.instant(),
                DateTimeValue::parse_iso("2026-07-29T14:30:00Z").unwrap().utc()
            );

            let parsed_ym = Point::parse_with_pattern("2026-07", "%Y-%m")
                .expect("valid year-month");
            assert_eq!(parsed_ym.canonical_pattern(), "%Y-%m");
            assert_eq!(
                parsed_ym.date(),
                DateValue::parse_iso("2026-07-01").unwrap()
            );
        }

        #[test]
        fn parse_with_pattern_rejects_bad_format_pattern_or_mismatched_text() {
            assert!(Point::parse_with_pattern("2026-07-29", "%Q").is_err());
            assert!(
                Point::parse_with_pattern("not-a-date", "%Y-%m-%d").is_err()
            );
            assert!(
                Point::parse_with_pattern("2026-07-29", "%d/%m/%Y").is_err()
            );
        }
    }
}

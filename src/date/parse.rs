use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

use super::value::{
    CivilPrecision, DATE_PATTERN, DateError, DateTimeValue, DateValue, Point,
    Precision, RecognizedDate, RecognizedDateValue, resolve_local,
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

#[expect(
    dead_code,
    reason = "Point parser is migrated into callers in phase 6.4"
)]
impl Point {
    /// Parses a date or date-time shape, retaining civil precision.
    ///
    /// # Errors
    ///
    /// Returns [`DateError::UnknownShape`] when no date-like prefix exists,
    /// or the corresponding parsing or local-zone error for invalid dates.
    pub(crate) fn parse(s: &str) -> Result<Self, DateError> {
        let trimmed = s.trim();
        if !starts_like_date(trimmed.as_bytes()) {
            return Err(DateError::UnknownShape);
        }
        if !starts_with_year(trimmed) {
            return Err(DateError::InvalidYearDigits {
                input: trimmed.into(),
            });
        }
        let civil = |(date, precision)| Self::Civil {
            date: DateValue(date),
            precision: if precision == Precision::YearMonth {
                CivilPrecision::Month
            } else {
                CivilPrecision::Day
            },
        };
        // Date-only shapes cannot match the date-time cascade.
        if trimmed.len() <= 10 {
            return DateShape::parse_any(trimmed).map(civil).map_err(
                |source| DateError::Unparseable {
                    input: trimmed.into(),
                    source,
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
                    let source = if matches!(
                        trimmed.as_bytes().get(10),
                        Some(b'T' | b' ')
                    ) {
                        datetime_err
                    } else {
                        date_err
                    };
                    Err(DateError::Unparseable {
                        input: trimmed.into(),
                        source,
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
            .map_err(|source| DateError::Unparseable {
                input: trimmed_text.into(),
                source,
            })
    }
}

impl DateValue {
    /// Classifies `s` as a date or date-time expression: `None` if it does
    /// not match an accepted date/time shape, `Some(Err)` on parse failure,
    /// or `Some(Ok)` on valid input with its recognized precision.
    ///
    /// # Errors
    ///
    /// - [`DateError::InvalidYearDigits`] if a date-like input does not begin
    ///   with 4 ASCII digits.
    /// - [`DateError::Unparseable`] if no accepted format shape matches.
    /// - [`DateError::LocalZoneLookup`] if the process's local timezone lookup
    ///   fails for a naive datetime.
    #[must_use]
    pub(crate) fn classify(
        s: &str,
    ) -> Option<Result<RecognizedDate, DateError>> {
        let trimmed = s.trim();
        let bytes = trimmed.as_bytes();
        if bytes.is_empty() || !starts_like_date(bytes) {
            return None;
        }

        if !starts_with_year(trimmed) {
            return Some(Err(DateError::InvalidYearDigits {
                input: trimmed.into(),
            }));
        }

        // Date-only shapes are at most 10 bytes; even the shortest date-time
        // shape (YYYY-M-DTH:M) is 12, so the date-time cascade cannot match.
        if trimmed.len() <= 10 {
            return Some(match DateShape::parse_any(trimmed) {
                Ok((date, precision)) => Ok(RecognizedDate {
                    value: RecognizedDateValue::Date(Self(date)),
                    precision,
                }),
                Err(source) => Err(DateError::Unparseable {
                    input: trimmed.into(),
                    source,
                }),
            });
        }

        match DateTimeShape::parse_any(trimmed) {
            Ok(ParsedDateTime::Instant(instant)) => Some(Ok(RecognizedDate {
                value: RecognizedDateValue::DateTime(DateTimeValue(instant)),
                precision: Precision::DateTime,
            })),
            Ok(ParsedDateTime::LocalWall(wall)) => match resolve_local(wall) {
                Ok(instant) => Some(Ok(RecognizedDate {
                    value: RecognizedDateValue::DateTime(DateTimeValue(
                        instant,
                    )),
                    precision: Precision::DateTime,
                })),
                Err(err) => Some(Err(err)),
            },
            Err(datetime_err) => match DateShape::parse_any(trimmed) {
                Ok((date, precision)) => Some(Ok(RecognizedDate {
                    value: RecognizedDateValue::Date(Self(date)),
                    precision,
                })),
                Err(date_err) => {
                    let source = if trimmed.len() > 10
                        && matches!(
                            trimmed.as_bytes().get(10),
                            Some(b'T' | b' ')
                        ) {
                        datetime_err
                    } else {
                        date_err
                    };
                    Some(Err(DateError::Unparseable {
                        input: trimmed.into(),
                        source,
                    }))
                }
            },
        }
    }

    /// Parses a date prefix from `s`, returning the recognized date and the
    /// number of consumed bytes.
    ///
    /// Checks 10-byte (`YYYY-MM-DD`) then 7-byte (`YYYY-MM`) prefixes.
    #[must_use]
    pub(crate) fn parse_prefix(s: &str) -> Option<(RecognizedDate, usize)> {
        let bytes = s.as_bytes();
        if bytes.len() >= 8 && bytes.get(7) == Some(&b'-') {
            if bytes.len() >= 10
                && let Some(prefix10) = s.get(..10)
                && let Some(Ok(rec)) = Self::classify(prefix10)
                && matches!(rec.precision, Precision::Date)
            {
                return Some((rec, 10));
            }
            return None;
        }

        if bytes.len() >= 7
            && let Some(prefix7) = s.get(..7)
            && let Some(Ok(rec)) = Self::classify(prefix7)
            && matches!(rec.precision, Precision::YearMonth)
        {
            return Some((rec, 7));
        }
        None
    }

    /// Parses an ISO-8601 date string (`YYYY-MM-DD` or `YYYY-MM`) using
    /// the recognition layer.
    ///
    /// # Errors
    ///
    /// - [`DateError::InvalidYearDigits`] if `s` does not begin with 4 ASCII
    ///   digits.
    /// - [`DateError::Unparseable`] if no accepted shape matches.
    pub(crate) fn parse_iso(s: &str) -> Result<Self, DateError> {
        let trimmed = s.trim();
        if !starts_with_year(trimmed) {
            return Err(DateError::InvalidYearDigits {
                input: trimmed.into(),
            });
        }
        DateShape::parse_any(trimmed).map(|(date, _)| Self(date)).map_err(
            |source| DateError::Unparseable {
                input: trimmed.into(),
                source,
            },
        )
    }

    /// Parses `text` according to `fmt` (`chrono::format::strftime` grammar).
    ///
    /// # Errors
    ///
    /// - [`DateError::Unparseable`] if `fmt` is an invalid format pattern or
    ///   does not match `text`.
    /// - [`DateError::LocalZoneLookup`] if a parsed naive datetime cannot be
    ///   resolved in the local zone.
    pub(crate) fn parse_with(
        text: &str,
        fmt: &str,
    ) -> Result<RecognizedDate, DateError> {
        let trimmed_text = text.trim();
        let trimmed_fmt = fmt.trim();

        if let Ok(dt) = DateTime::parse_from_str(trimmed_text, trimmed_fmt) {
            return Ok(RecognizedDate {
                value: RecognizedDateValue::DateTime(DateTimeValue(
                    dt.with_timezone(&Utc),
                )),
                precision: Precision::DateTime,
            });
        }

        if let Ok(naive_dt) =
            NaiveDateTime::parse_from_str(trimmed_text, trimmed_fmt)
        {
            let instant = resolve_local(naive_dt)?;
            return Ok(RecognizedDate {
                value: RecognizedDateValue::DateTime(DateTimeValue(instant)),
                precision: Precision::DateTime,
            });
        }

        if trimmed_fmt == "%Y-%m"
            && let Some(Ok(rec)) = Self::classify(trimmed_text)
            && matches!(rec.precision, Precision::YearMonth)
        {
            return Ok(rec);
        }

        match NaiveDate::parse_from_str(trimmed_text, trimmed_fmt) {
            Ok(naive_d) => Ok(RecognizedDate {
                value: RecognizedDateValue::Date(Self(naive_d)),
                precision: Precision::Date,
            }),
            Err(source) => Err(DateError::Unparseable {
                input: trimmed_text.into(),
                source,
            }),
        }
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
    /// - [`DateError::InvalidYearDigits`] if `s` does not begin with 4 ASCII
    ///   digits.
    /// - [`DateError::Unparseable`] if no accepted shape matches.
    /// - [`DateError::LocalZoneLookup`] if the local timezone lookup fails
    ///   while resolving a naive input.
    pub(crate) fn parse_iso(s: &str) -> Result<Self, DateError> {
        let trimmed = s.trim();
        if !starts_with_year(trimmed) {
            return Err(DateError::InvalidYearDigits {
                input: trimmed.into(),
            });
        }
        let shape = DateTimeShape::parse_any(trimmed).map_err(|source| {
            DateError::Unparseable {
                input: trimmed.into(),
                source,
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
    ) -> Result<(NaiveDate, Precision), chrono::ParseError> {
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
    ) -> Result<(NaiveDate, Precision), chrono::ParseError> {
        match self {
            Self::YearMonthDay => {
                let pat = DATE_PATTERN;
                NaiveDate::parse_from_str(s, pat).map(|d| (d, Precision::Date))
            }
            Self::YearMonth => match Self::parse_year_month(s) {
                Some(d) => Ok((d, Precision::YearMonth)),
                None => NaiveDate::parse_from_str(s, "%Y-%m")
                    .map(|d| (d, Precision::YearMonth)),
            },
        }
    }

    /// Parses `"YYYY-MM"`, defaulting the day to 1.
    fn parse_year_month(s: &str) -> Option<NaiveDate> {
        let (year_str, month_str) = s.split_once('-')?;
        if year_str.len() != 4 || month_str.len() != 2 {
            return None;
        }
        let year: i32 = year_str.parse().ok()?;
        let month: u32 = month_str.parse().ok()?;
        NaiveDate::from_ymd_opt(year, month, 1)
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
    pub(crate) fn parse_any(
        s: &str,
    ) -> Result<ParsedDateTime, chrono::ParseError> {
        let [first, rest @ ..] = Self::ALL;
        let mut result = first.parse_shape(s);
        for format in rest {
            if result.is_ok() {
                break;
            }
            result = format.parse_shape(s);
        }
        result
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
            // later, by the format cascade, as Unparseable rather than
            // InvalidYearDigits.
            assert!(starts_with_year("20265-01-01"));
            assert!(matches!(
                DateValue::parse_iso("20265-01-01"),
                Err(DateError::Unparseable { .. })
            ));
        }

        #[test]
        fn date_value_rejects_a_short_year_with_invalid_year_digits() {
            let result = DateValue::parse_iso("26-08-22");
            assert!(matches!(result, Err(DateError::InvalidYearDigits { .. })));
        }

        #[test]
        fn datetime_value_rejects_a_short_year_with_invalid_year_digits() {
            let result = DateTimeValue::parse_iso("26-08-22T14:30:00");
            assert!(matches!(result, Err(DateError::InvalidYearDigits { .. })));
        }

        #[rstest]
        #[case::invalid_month_number("2026-13")]
        #[case::non_numeric_month("2026-ab")]
        #[case::single_digit_month("2026-7")]
        fn rejects_a_malformed_year_month_as_unparseable(#[case] input: &str) {
            assert!(matches!(
                DateValue::parse_iso(input),
                Err(DateError::Unparseable { .. })
            ));
        }

        #[test]
        fn rejects_an_unrecognized_date_shape_as_unparseable() {
            let result = DateValue::parse_iso("2026/08/22");
            assert!(matches!(result, Err(DateError::Unparseable { .. })));
        }

        #[test]
        fn rejects_an_unrecognized_datetime_shape_as_unparseable() {
            let result = DateTimeValue::parse_iso("2026-07-29Tinvalid");
            assert!(matches!(result, Err(DateError::Unparseable { .. })));
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

    mod classify {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn classify_short_inputs_match_pre_gate_results() {
            // Arrange
            let cases = [
                ("2026-07-29", Precision::Date),
                ("2026-07", Precision::YearMonth),
                ("2026-1-1", Precision::Date),
            ];

            // Act & Assert
            for (text, precision) in cases {
                assert_eq!(
                    DateValue::classify(text).unwrap().unwrap().precision,
                    precision
                );
            }
            assert!(matches!(
                DateValue::classify("26-08-22"),
                Some(Err(DateError::InvalidYearDigits { .. }))
            ));
            assert!(DateValue::classify("2026/08/22").is_none());

            let failures = ["2026-13", "2026-02-30"];
            let messages = failures.map(|text| {
                let err = DateValue::classify(text).unwrap().unwrap_err();
                assert!(matches!(err, DateError::Unparseable { .. }));
                err.to_string()
            });
            assert_eq!(messages, [
                "`2026-13` is not a recognized date/time: input is out of \
                 range",
                "`2026-02-30` is not a recognized date/time: trailing input",
            ]);
        }

        #[test]
        fn returns_none_for_non_temporal_text() {
            assert!(DateValue::classify("hello").is_none());
            assert!(DateValue::classify("").is_none());
            assert!(DateValue::classify("2026").is_none());
            assert!(DateValue::classify("2026/08/22").is_none());
            assert!(DateValue::classify("20265-01-01").is_none());
        }

        #[test]
        fn returns_some_err_for_shape_matching_but_invalid_input() {
            let res1 =
                DateValue::classify("2026-13").expect("shape gate passes");
            assert!(matches!(res1, Err(DateError::Unparseable { .. })));

            let res2 =
                DateValue::classify("26-08-22").expect("shape gate passes");
            assert!(matches!(res2, Err(DateError::InvalidYearDigits { .. })));

            let res3 = DateValue::classify("2026-07-29Tinvalid")
                .expect("shape gate passes");
            assert!(matches!(res3, Err(DateError::Unparseable { .. })));
        }

        #[test]
        fn returns_some_ok_with_appropriate_precision() {
            TzGuard::keep();

            let ym =
                DateValue::classify("2026-07").expect("parses").expect("valid");
            assert_eq!(ym.precision, Precision::YearMonth);
            assert_eq!(ym.date(), DateValue::parse_iso("2026-07-01").unwrap());

            let d = DateValue::classify("2026-07-29")
                .expect("parses")
                .expect("valid");
            assert_eq!(d.precision, Precision::Date);
            assert_eq!(d.date(), DateValue::parse_iso("2026-07-29").unwrap());

            let dt = DateValue::classify("2026-07-29T14:30:00")
                .expect("parses")
                .expect("valid");
            assert_eq!(dt.precision, Precision::DateTime);

            let dt_space_frac = DateValue::classify("2026-07-29 14:30:00.123")
                .expect("parses")
                .expect("valid");
            assert_eq!(dt_space_frac.precision, Precision::DateTime);
        }

        #[test]
        fn demo_contract_year_month_shifts_and_reformats_as_year_month() {
            let recognized = DateValue::classify("2026-07")
                .expect("recognized")
                .expect("valid");
            assert_eq!(recognized.precision, Precision::YearMonth);

            let shifted_point = recognized
                .point()
                .shift(1, DurationUnit::Month)
                .expect("shifted");
            assert_eq!(shifted_point.precision, Precision::YearMonth);

            let pat =
                shifted_point.precision.format_pattern(shifted_point.wall);
            assert_eq!(pat, "%Y-%m");
            assert_eq!(shifted_point.wall.format(pat).to_string(), "2026-08");
        }

        #[test]
        fn equates_values_at_different_precisions_denoting_the_same_day() {
            let ym = DateValue::parse_iso("2026-07").expect("valid");
            let d = DateValue::parse_iso("2026-07-01").expect("valid");
            assert_eq!(ym, d);

            let ym_rec = DateValue::classify("2026-07").unwrap().unwrap();
            let d_rec = DateValue::classify("2026-07-01").unwrap().unwrap();
            assert_eq!(ym_rec.precision, Precision::YearMonth);
            assert_eq!(d_rec.precision, Precision::Date);
            assert_eq!(ym_rec.date(), d_rec.date());
        }

        #[test]
        fn round_trips_fractional_seconds_through_shift_and_format() {
            TzGuard::set("UTC");

            let recognized = DateValue::classify("2026-07-29T14:30:00.123")
                .expect("recognized")
                .expect("valid");
            assert_eq!(recognized.precision, Precision::DateTime);

            let shifted = recognized
                .point()
                .shift(1, DurationUnit::Hour)
                .expect("shifted");
            let pat = shifted.precision.format_pattern(shifted.wall);
            assert_eq!(
                shifted.wall.format(pat).to_string(),
                "2026-07-29T15:30:00.123"
            );
        }

        #[test]
        fn parse_prefix_extracts_date_and_year_month_with_consumed_lengths() {
            let (d, consumed_d) =
                DateValue::parse_prefix("2026-07-29T14:30:00").expect("prefix");
            assert_eq!(consumed_d, 10);
            assert_eq!(d.precision, Precision::Date);
            assert_eq!(d.date(), DateValue::parse_iso("2026-07-29").unwrap());

            let (ym, consumed_ym) =
                DateValue::parse_prefix("2026-07 remainder").expect("prefix");
            assert_eq!(consumed_ym, 7);
            assert_eq!(ym.precision, Precision::YearMonth);
            assert_eq!(ym.date(), DateValue::parse_iso("2026-07-01").unwrap());

            assert!(DateValue::parse_prefix("not a date").is_none());
        }
    }

    mod parse_with_and_boundaries {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn parse_with_parses_matching_date_and_datetime_formats() {
            TzGuard::set("UTC");
            let parsed_date = DateValue::parse_with("2026-07-29", "%Y-%m-%d")
                .expect("valid date");
            assert_eq!(parsed_date.precision, Precision::Date);
            assert_eq!(
                parsed_date.date(),
                DateValue::parse_iso("2026-07-29").unwrap()
            );

            let parsed_custom = DateValue::parse_with("29/07/2026", "%d/%m/%Y")
                .expect("valid custom date");
            assert_eq!(
                parsed_custom.date(),
                DateValue::parse_iso("2026-07-29").unwrap()
            );

            let parsed_dt = DateValue::parse_with(
                "2026-07-29 14:30:00",
                "%Y-%m-%d %H:%M:%S",
            )
            .expect("valid datetime");
            assert_eq!(parsed_dt.precision, Precision::DateTime);
            assert_eq!(
                parsed_dt.instant(),
                DateTimeValue::parse_iso("2026-07-29T14:30:00Z").unwrap().utc()
            );

            let parsed_ym = DateValue::parse_with("2026-07", "%Y-%m")
                .expect("valid year-month");
            assert_eq!(parsed_ym.precision, Precision::YearMonth);
            assert_eq!(
                parsed_ym.date(),
                DateValue::parse_iso("2026-07-01").unwrap()
            );
        }

        #[test]
        fn parse_with_rejects_bad_format_pattern_or_mismatched_text() {
            assert!(DateValue::parse_with("2026-07-29", "%Q").is_err());
            assert!(DateValue::parse_with("not-a-date", "%Y-%m-%d").is_err());
            assert!(DateValue::parse_with("2026-07-29", "%d/%m/%Y").is_err());
        }
    }
}

use std::{borrow::Cow, fmt, str::FromStr, time::SystemTime};

use chrono::{Datelike as _, NaiveDate, NaiveTime, TimeDelta};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    DatePrecision, DateTimeValue,
    error::DateError,
    point::{parse_reason, starts_with_year},
};
/// [`DateValue`]'s canonical output format: `2026-07-29`.
pub(crate) const DATE_PATTERN: &str = "%Y-%m-%d";

/// A calendar date without timezone or time-of-day ([`NaiveDate`]).
///
/// Wraps a [`NaiveDate`] representing a calendar date (`YYYY-MM-DD` or
/// `YYYY-MM`). Parsed input keeps precision in `DatePoint`; deserialization
/// uses the same strict date parser.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DateValue(NaiveDate);

impl DateValue {
    /// Returns the wrapped [`NaiveDate`].
    #[must_use]
    pub(crate) const fn naive_date(self) -> NaiveDate {
        self.0
    }

    /// Whole calendar months from `self` to `to`, signed.
    ///
    /// Chrono has no `months_since` equivalent, so this mirrors
    /// [`NaiveDate::years_since`]'s algorithm at month granularity: total
    /// calendar months between the dates, minus one when the day-of-month has
    /// not yet been reached. Either ordering is accepted.
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

    /// Whole calendar years from `self` to `to`, signed.
    ///
    /// A year is not "up" until `to`'s month and day reach `self`'s, which is
    /// the whole-month count truncated to years.
    pub(super) fn whole_years_until(self, to: Self) -> i64 {
        self.whole_months_until(to) / 12
    }

    /// Moves the date by `delta` read from midnight, keeping the date part.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the result is outside chrono's date
    ///   range.
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

/// Renders as the canonical `YYYY-MM-DD` string.
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

impl From<SystemTime> for DateValue {
    #[inline]
    fn from(time: SystemTime) -> Self {
        // A file's calendar date means the reader's local day (the same
        // doctrine as `DateTimeValue::local_date`), not the UTC calendar day.
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

/// Deserializes via the crate-internal `parse_iso` parser, so the
/// four-digit-year rule and `YYYY-MM` acceptance apply identically to inline
/// and deserialized dates.
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
    use crate::{
        temporal::{DatePoint, DurationValue},
        test_support::TzGuard,
    };

    fn point_at(text: &str) -> DatePoint {
        DatePoint::parse(text).expect("valid date point")
    }

    mod calendar_owner {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn date_value_calendar_arithmetic_handles_sub_day_units() {
            let base = point_at("2026-07-29");

            // Sub-day 1 hour shift is a no-op on date (stays 2026-07-29)
            let one_hour = DurationValue::parse("1h").unwrap();
            assert_eq!(base.apply(&one_hour).unwrap(), base);

            // 24 hour shift advances the calendar date
            let twenty_four_hours = DurationValue::parse("24h").unwrap();
            assert_eq!(
                base.apply(&twenty_four_hours).unwrap(),
                point_at("2026-07-30")
            );
        }
    }

    mod formatting {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn round_trips_a_date_value_through_display_and_parse_iso() {
            let date = DateValue::parse_iso("2026-07-29").expect("valid date");
            let reparsed = DateValue::parse_iso(&date.to_string())
                .expect("formatted output re-parses");
            assert_eq!(reparsed, date);
        }
    }

    mod parsing {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

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

        #[test]
        fn date_value_rejects_a_short_year_with_year_not_four_digits() {
            let result = DateValue::parse_iso("26-08-22");
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
    }

    mod conversions {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn converts_from_system_time_to_the_local_date() {
            TzGuard::set("Etc/GMT+5"); // UTC-05:00, no DST

            let system_time = std::time::SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(1_000);

            // 00:16:40Z is 19:16:40 on December 31 in UTC-5.
            assert_eq!(
                DateValue::from(system_time),
                DateValue::parse_iso("1969-12-31").expect("valid date")
            );
        }

        #[test]
        fn converts_from_system_time_to_date_value() {
            TzGuard::set("UTC");

            let system_time = std::time::SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(1_000);
            let converted = DateValue::from(system_time);
            assert_eq!(
                converted,
                DateValue::parse_iso("1970-01-01").expect("valid date")
            );
        }

        #[test]
        fn converts_date_value_into_naive_date_via_from_trait() {
            let date = DateValue::parse_iso("2026-07-29").expect("valid date");
            let naive: NaiveDate = date.into();
            assert_eq!(
                naive,
                NaiveDate::from_ymd_opt(2026, 7, 29).expect("valid date")
            );
        }

        #[test]
        fn converts_from_naive_date_via_from_trait() {
            let naive =
                NaiveDate::from_ymd_opt(2026, 7, 29).expect("valid date");
            let date: DateValue = naive.into();
            assert_eq!(
                date,
                DateValue::parse_iso("2026-07-29").expect("valid date")
            );
        }
    }

    mod ordering {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn compares_dates_chronologically() {
            let earlier =
                DateValue::parse_iso("2026-01-01").expect("valid date");
            let later = DateValue::parse_iso("2026-12-31").expect("valid date");
            assert!(earlier < later);
            assert!(later > earlier);
            assert_eq!(earlier.cmp(&earlier), std::cmp::Ordering::Equal);
        }
    }

    mod display_and_traits {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn displays_a_date_value_as_its_canonical_string() {
            let date = DateValue::parse_iso("2026-07-29").expect("valid date");
            assert_eq!(date.to_string(), "2026-07-29");
        }

        #[test]
        fn displays_invalid_value_error_naming_the_input_and_reason() {
            let err = DateValue::parse_iso("2026/08/22")
                .expect_err("unrecognized shape");
            assert_eq!(
                err.to_string(),
                "`2026/08/22` is not a recognized date/time: invalid input"
            );
        }

        #[test]
        fn displays_year_not_four_digits_error_naming_the_input() {
            let err = DateValue::parse_iso("26-08-22")
                .expect_err("short year rejected");
            assert_eq!(
                err.to_string(),
                "`26-08-22` does not have a 4-digit year"
            );
        }

        #[test]
        fn round_trips_a_date_value_through_json() {
            let date = DateValue::parse_iso("2026-07-29").expect("valid date");
            let json = serde_json::to_string(&date).expect("serializable");
            let restored: DateValue =
                serde_json::from_str(&json).expect("deserializable");
            assert_eq!(restored, date);
        }

        #[test]
        fn deserializes_year_month_precision_date_through_json() {
            let restored: DateValue =
                serde_json::from_str("\"2026-07\"").expect("deserializable");
            assert_eq!(
                restored,
                DateValue::parse_iso("2026-07-01").expect("valid date")
            );
        }

        #[test]
        fn rejects_a_two_digit_year_through_json_deserialization() {
            let err = serde_json::from_str::<DateValue>("\"26-08-22\"")
                .expect_err("short year rejected");
            assert!(err.to_string().contains("does not have a 4-digit year"));
        }

        #[test]
        fn is_usable_as_a_hash_set_key() {
            let mut set = std::collections::HashSet::new();
            set.insert(DateValue::parse_iso("2026-07-29").expect("valid date"));
            set.insert(DateValue::parse_iso("2026-07-29").expect("valid date"));
            set.insert(DateValue::parse_iso("2026-07-30").expect("valid date"));
            assert_eq!(set.len(), 2);
        }
    }
}

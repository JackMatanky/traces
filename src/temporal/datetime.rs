//! A UTC-stored Date-Time that renders as the local clock.
//!
//! Owns [`DateTimeValue`] and the recognition shapes ([`DateTimeShape`],
//! [`ParsedDateTime`]) that turn written RFC 3339 and naive clock times into
//! it. Written input without an offset resolves in the process's local zone and
//! stores as UTC; every human-facing rendering converts back to the local
//! clock. Callers hold the validated date-time or receive a [`DateError`].

use std::{borrow::Cow, fmt, str::FromStr, time::SystemTime};

use chrono::{
    DateTime, Datelike as _, NaiveDate, NaiveDateTime, NaiveTime,
    SecondsFormat, Timelike as _, Utc,
};
use jiff::{Timestamp, civil, tz::TimeZone};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    CalendarError, DateValue, TimeUnit,
    calendar::shift_datetime,
    date::has_padded_date_prefix,
    error::DateError,
    point::{parse_reason, starts_with_year},
    zone::local_zone,
};

/// Canonical output format of [`DateTimeValue`]: `2026-07-29T14:30:00`.
pub(crate) const DATETIME_PATTERN: &str = "%Y-%m-%dT%H:%M:%S";

/// A Date-Time stored as UTC that renders as the local clock.
///
/// Accepts RFC 3339 input with an explicit offset except for leap seconds,
/// which Jiff timestamps cannot represent. ISO date portions require
/// zero-padded month and day fields. Naive input in the five written shapes of
/// `DateTimeShape` resolves in the process’s local zone (ambiguous fall-back
/// times take the earlier occurrence; spring-forward gaps shift forward by the
/// gap length) and stores as UTC. `DatePoint::parse_with_pattern` is a separate
/// strftime parser and retains its pattern-driven padding rules.
/// [`Display`](fmt::Display) renders the local clock time without a zone
/// offset, while serialization carries an explicit RFC 3339 `…Z` spelling.
/// Deserialization runs the same parser as [`FromStr`](std::str::FromStr), so
/// inline and deserialized date-times follow one grammar.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "test-utils")]
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use std::time::{Duration, SystemTime};
///
/// use traces_pkm::{DateTimeValue, testing::ZoneGuard};
///
/// ZoneGuard::set("UTC");
///
/// let epoch = DateTimeValue::try_from(SystemTime::UNIX_EPOCH)?;
/// assert_eq!(epoch.to_string(), "1970-01-01T00:00:00");
///
/// let minute = DateTimeValue::try_from(
///     SystemTime::UNIX_EPOCH - Duration::from_secs(60),
/// )?;
/// assert_eq!(minute.to_string(), "1969-12-31T23:59:00");
///
/// let parsed: DateTimeValue = "2026-07-29T14:30:00Z".parse()?;
/// assert_eq!(parsed.to_string(), "2026-07-29T14:30:00");
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "test-utils"))]
/// # fn main() {}
/// ```
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DateTimeValue(Timestamp);

impl DateTimeValue {
    #[inline]
    pub(crate) const fn from_timestamp(timestamp: Timestamp) -> Self {
        Self(timestamp)
    }

    /// Returns the local clock-time rendering of this date-time.
    ///
    /// Every human-facing rendering uses checked offset application and returns
    /// [`None`] when the result falls outside the supported date-time range.
    #[inline]
    #[must_use]
    pub(crate) fn local_datetime(self) -> Option<NaiveDateTime> {
        self.local_datetime_in(&local_zone())
    }

    /// Converts the UTC value to local civil time using the supplied zone.
    #[inline]
    #[must_use]
    pub(crate) fn local_datetime_in(
        self,
        zone: &TimeZone,
    ) -> Option<NaiveDateTime> {
        let local = self.0.to_zoned(zone.clone()).datetime();
        let date = NaiveDate::from_ymd_opt(
            i32::from(local.year()),
            u32::try_from(local.month()).ok()?,
            u32::try_from(local.day()).ok()?,
        )?;
        let time = NaiveTime::from_hms_nano_opt(
            u32::try_from(local.hour()).ok()?,
            u32::try_from(local.minute()).ok()?,
            u32::try_from(local.second()).ok()?,
            u32::try_from(local.time().subsec_nanosecond()).ok()?,
        )?;
        Some(NaiveDateTime::new(date, time))
    }

    /// Returns the local clock time, falling back to the UTC clock time when
    /// the local offset cannot be applied (see [`Self::local_datetime`]).
    #[inline]
    #[must_use]
    pub(crate) fn local_datetime_or_utc(self) -> NaiveDateTime {
        self.local_datetime().unwrap_or_else(|| self.utc().naive_utc())
    }

    /// Returns the local calendar date of this date-time, discarding
    /// time-of-day.
    ///
    /// At the representable date-time edge where the local offset cannot be
    /// applied, the UTC date is used.
    #[inline]
    pub(crate) fn local_date(self) -> Result<DateValue, CalendarError> {
        DateValue::try_from_naive_date(self.local_datetime_or_utc().date())
    }

    /// Returns true if this date-time is exactly local-zone midnight on date.
    #[must_use]
    pub(crate) fn is_midnight_of(self, date: DateValue) -> bool {
        Self::try_from(date).is_ok_and(|midnight| self.0 == midnight.0)
    }

    /// Returns the wrapped [`DateTime<Utc>`].
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "Jiff's timestamp range is strictly inside chrono's UTC range"
    )]
    pub(crate) fn utc(self) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(
            self.0.as_second(),
            u32::try_from(self.0.subsec_nanosecond())
                .expect("Jiff timestamps have nonnegative nanoseconds"),
        )
        .expect("Jiff timestamp range fits Chrono's UTC range")
    }

    /// Canonical strftime pattern for this date-time: whole seconds, plus
    /// fractional seconds only when the nanosecond component is nonzero.
    ///
    /// Local offsets are whole seconds, so the UTC and local readings share
    /// their nanosecond component and no zone lookup is needed.
    #[must_use]
    pub(crate) fn canonical_pattern(self) -> &'static str {
        if self.0.subsec_nanosecond() == 0 {
            DATETIME_PATTERN
        } else {
            "%Y-%m-%dT%H:%M:%S%.f"
        }
    }

    /// Resolves a naive local clock time under Jiff's compatible DST policy.
    ///
    /// Ambiguous fall-back times choose their earlier occurrence;
    /// spring-forward gaps shift forward by the gap length.
    ///
    /// # Errors
    ///
    /// Returns [`CalendarError::Zone`] when the civil time or resolved instant
    /// is outside the supported range.
    pub(super) fn from_local(
        local_datetime: NaiveDateTime,
    ) -> Result<Self, CalendarError> {
        Self::from_local_in(local_datetime, &local_zone())
    }

    /// Resolves a naive local clock time in the explicitly supplied zone.
    pub(super) fn from_local_in(
        local_datetime: NaiveDateTime,
        zone: &TimeZone,
    ) -> Result<Self, CalendarError> {
        let zone_lookup = || CalendarError::Zone {
            input: local_datetime.to_string().into(),
        };
        let civil_datetime = civil::DateTime::new(
            i16::try_from(local_datetime.year()).map_err(|_| zone_lookup())?,
            i8::try_from(local_datetime.month()).map_err(|_| zone_lookup())?,
            i8::try_from(local_datetime.day()).map_err(|_| zone_lookup())?,
            i8::try_from(local_datetime.hour()).map_err(|_| zone_lookup())?,
            i8::try_from(local_datetime.minute()).map_err(|_| zone_lookup())?,
            i8::try_from(local_datetime.second()).map_err(|_| zone_lookup())?,
            i32::try_from(local_datetime.nanosecond())
                .map_err(|_| zone_lookup())?,
        )
        .map_err(|_| zone_lookup())?;
        let timestamp = civil_datetime
            .to_zoned(zone.clone())
            .map_err(|_| zone_lookup())?
            .timestamp();
        Ok(Self(timestamp))
    }

    /// Shifts calendar units in the local datetime frame and clock units in
    /// UTC.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the shift overflows or its local
    ///   date-time lies outside the supported range.
    /// - [`CalendarError::Zone`] if the shifted local datetime cannot be
    ///   resolved to a stored date-time.
    pub(crate) fn shift(
        self,
        n: i64,
        unit: TimeUnit,
    ) -> Result<Self, CalendarError> {
        if unit.is_calendar() {
            let local_datetime =
                self.local_datetime().ok_or(CalendarError::OutOfRange)?;
            Self::from_local(shift_datetime(local_datetime, n, unit)?)
        } else {
            Self::try_from(
                shift_datetime(self.utc().naive_utc(), n, unit)?.and_utc(),
            )
            .map_err(|_| CalendarError::OutOfRange)
        }
    }

    /// Parses an RFC 3339 or ISO-8601 date-time string using
    /// [`DateTimeShape::ALL`].
    ///
    /// Surrounding whitespace is ignored. The cascade matches shapes only: a
    /// matched naive input then resolves through [`DateTimeValue::from_local`]
    /// (an explicit-offset input already carries an offset and stores
    /// directly). Resolution runs once after the cascade, because a resolution
    /// failure must not retry later formats.
    ///
    /// # Errors
    ///
    /// - [`DateError::YearNotFourDigits`] if `s` does not begin with 4 ASCII
    ///   digits.
    /// - [`DateError::InvalidValue`] if no accepted shape matches.
    /// - [`DateError::LocalZoneLookup`] if local zone lookup fails while
    ///   resolving a naive input.
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
            ParsedDateTime::Instant(date_time) => Self::try_from(date_time)
                .map_err(|_| DateError::InvalidValue {
                    input: trimmed.into(),
                    reason: "date/time is outside the representable range",
                }),
            ParsedDateTime::Unzoned(local_datetime) => {
                Self::from_local(local_datetime)
                    .map_err(|error| error.into_date_error(trimmed))
            }
        }
    }
}

/// Renders the local clock time without a UTC offset, so a date or time
/// component reads as the reader's own clock.
///
/// Fractional seconds appear only for a nonzero nanosecond component, so
/// `DateTimeShape::LocalTSeconds` input keeps its full sub-second precision.
/// When the local offset cannot be applied, the UTC clock time renders instead
/// of panicking (see `DateTimeValue::local_datetime`).
impl fmt::Display for DateTimeValue {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.local_datetime_or_utc().format(self.canonical_pattern())
        )
    }
}

impl TryFrom<SystemTime> for DateTimeValue {
    type Error = DateError;

    #[inline]
    fn try_from(time: SystemTime) -> Result<Self, Self::Error> {
        Timestamp::try_from(time).map(Self).map_err(|_| {
            DateError::InvalidValue {
                input: "system time".into(),
                reason: "date/time is outside the representable range",
            }
        })
    }
}

impl TryFrom<DateTime<Utc>> for DateTimeValue {
    type Error = DateError;

    #[inline]
    fn try_from(dt: DateTime<Utc>) -> Result<Self, Self::Error> {
        let nanoseconds =
            i32::try_from(dt.timestamp_subsec_nanos()).map_err(|_| {
                DateError::InvalidValue {
                    input: dt.to_rfc3339().into(),
                    reason: "date/time is outside the representable range",
                }
            })?;
        Timestamp::new(dt.timestamp(), nanoseconds).map(Self).map_err(|_| {
            DateError::InvalidValue {
                input: dt.to_rfc3339().into(),
                reason: "date/time is outside the representable range",
            }
        })
    }
}

impl From<DateTimeValue> for DateTime<Utc> {
    #[inline]
    fn from(value: DateTimeValue) -> Self {
        value.utc()
    }
}

/// Resolves a calendar date to local midnight.
///
/// If local resolution fails, the date is retried at UTC midnight.
///
/// # Errors
///
/// Returns `DateError::InvalidValue` when the date's midnight is outside the
/// supported timestamp range.
impl TryFrom<DateValue> for DateTimeValue {
    type Error = DateError;

    #[inline]
    fn try_from(date: DateValue) -> Result<Self, Self::Error> {
        let midnight = date.naive_date().and_time(NaiveTime::MIN);
        Self::from_local(midnight)
            .or_else(|_| Self::from_local_in(midnight, &TimeZone::UTC))
            .map_err(|_| DateError::InvalidValue {
                input: midnight.to_string().into(),
                reason: "date/time is outside the representable range",
            })
    }
}

impl FromStr for DateTimeValue {
    type Err = DateError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_iso(s)
    }
}

/// Serializes as an explicit RFC 3339 `…Z` interop spelling, independent of the
/// local-clock rendering [`Display`](fmt::Display) performs.
impl Serialize for DateTimeValue {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(
            &self.utc().to_rfc3339_opts(SecondsFormat::AutoSi, true),
        )
    }
}

/// Deserializes via the crate-internal `parse_iso` parser, so the
/// four-digit-year rule and every accepted shape (including the `…Z` interop
/// spelling) apply identically to inline and deserialized date-times.
impl<'de> Deserialize<'de> for DateTimeValue {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = Cow::<'de, str>::deserialize(deserializer)?;
        Self::parse_iso(&s).map_err(serde::de::Error::custom)
    }
}

/// Recognized date-time input format shapes, tried in order by
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
        result.map_err(parse_reason).and_then(|parsed| {
            has_padded_date_prefix(s)
                .then_some(parsed)
                .ok_or("input is too short")
        })
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
    /// An explicit RFC 3339 date-time with its offset applied; stores as UTC
    /// directly.
    Instant(DateTime<Utc>),
    /// Local naive datetime that must resolve through
    /// [`DateTimeValue::from_local`].
    Unzoned(NaiveDateTime),
}

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, TimeZone as _};

    use super::*;
    use crate::{
        temporal::fixtures::{fixed_datetime, ymd},
        test_support::ZoneGuard,
    };

    // ZoneGuard pins a Jiff time zone to the test thread without changing the
    // process TZ environment variable; repeated sets replace the override.

    mod constructor {
        use std::time::Duration;

        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn constructs_the_unix_epoch_from_system_time() {
            // Arrange: the epoch and the UTC date-time it must equal.
            let epoch = SystemTime::UNIX_EPOCH;
            let expected = Utc
                .with_ymd_and_hms(1970, 1, 1, 0, 0, 0)
                .single()
                .expect("the epoch is an unambiguous UTC date-time");

            // Act
            let value = DateTimeValue::try_from(epoch)
                .expect("the Unix epoch is within Timestamp's range");

            // Assert
            assert_eq!(
                value.utc(),
                expected,
                "the epoch must construct as 1970-01-01T00:00:00Z"
            );
        }

        #[test]
        fn constructs_a_pre_epoch_date_time_from_system_time() {
            // Arrange: one thousand seconds before the epoch.
            let before_epoch =
                SystemTime::UNIX_EPOCH - Duration::from_secs(1_000);
            let expected = Utc
                .with_ymd_and_hms(1969, 12, 31, 23, 43, 20)
                .single()
                .expect("the pre-epoch reading is unambiguous");

            // Act
            let value = DateTimeValue::try_from(before_epoch)
                .expect("the pre-epoch value is within Timestamp's range");

            // Assert
            assert_eq!(
                value.utc(),
                expected,
                "negative timeline positions must construct before the epoch"
            );
        }

        #[rstest]
        #[case::minimum(Timestamp::MIN)]
        #[case::maximum(Timestamp::MAX)]
        fn wraps_the_representable_range_edges(#[case] edge: Timestamp) {
            // Arrange: the edge is a bound of the Jiff timestamp range.
            let value = DateTimeValue(edge);

            // Act and assert
            assert_eq!(value.0, edge);
        }

        #[test]
        fn returns_none_one_second_past_each_representable_edge() {
            // Arrange: the bounds one second outside the representable range.
            let min_seconds = NaiveDateTime::MIN.and_utc().timestamp();
            let max_seconds = NaiveDateTime::MAX.and_utc().timestamp();
            let below = min_seconds
                .checked_sub(1)
                .expect("the minimum second is not i64::MIN");
            let above = max_seconds
                .checked_add(1)
                .expect("the maximum second is not i64::MAX");

            // Act
            let lower = DateTime::<Utc>::from_timestamp(below, 0);
            let upper = DateTime::<Utc>::from_timestamp(above, 0);

            // Assert
            assert!(
                lower.is_none(),
                "one second past the minimum must not construct, got {lower:?}"
            );
            assert!(
                upper.is_none(),
                "one second past the maximum must not construct, got {upper:?}"
            );
        }
    }

    mod disambiguation {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn resolves_an_ambiguous_fall_back_time_to_the_earlier_occurrence() {
            // Arrange: 01:30 happens twice on this fall-back day.
            ZoneGuard::set("America/New_York");

            // Act
            let parsed = DateTimeValue::parse_iso("2026-11-01 01:30:00");
            let earlier = DateTimeValue::parse_iso("2026-11-01T05:30:00Z");

            // Assert
            assert_eq!(
                parsed, earlier,
                "an ambiguous clock time must take the earlier (EDT) \
                 occurrence"
            );
        }

        #[test]
        fn shifts_a_spring_forward_gap_forward_by_the_gap() {
            // Arrange: 02:30 does not exist on this spring-forward day.
            ZoneGuard::set("America/New_York");

            // Act
            let parsed = DateTimeValue::parse_iso("2026-03-08 02:30:00");
            let shifted = DateTimeValue::parse_iso("2026-03-08T07:30:00Z");

            // Assert
            assert_eq!(
                parsed, shifted,
                "a gap clock time must shift forward by the one-hour gap"
            );
            assert_eq!(
                parsed.map(|value| value.to_string()),
                Ok("2026-03-08T03:30:00".to_owned()),
                "the shifted gap must land on the post-transition clock"
            );
        }

        #[test]
        fn resolves_local_times_adjacent_to_a_gap_without_error() {
            // Arrange: one second before the gap, and at the gap's end.
            ZoneGuard::set("America/New_York");

            // Act
            let before = DateTimeValue::parse_iso("2026-03-08 01:59:59");
            let after = DateTimeValue::parse_iso("2026-03-08 03:00:00");
            let before_expected =
                DateTimeValue::parse_iso("2026-03-08T06:59:59Z");
            let after_expected =
                DateTimeValue::parse_iso("2026-03-08T07:00:00Z");

            // Assert
            assert_eq!(
                before, before_expected,
                "the last second before the gap must resolve normally"
            );
            assert_eq!(
                after, after_expected,
                "the first second after the gap must resolve normally"
            );
        }

        #[test]
        fn resolves_the_london_fold_to_the_earlier_occurrence() {
            // Arrange: one zone per test; London falls back on 2026-10-25.
            ZoneGuard::set("Europe/London");
            let earlier = DateTimeValue::parse_iso("2026-10-25T01:30:00+01:00")
                .expect("an explicit offset must parse");
            let later = DateTimeValue::parse_iso("2026-10-25T01:30:00+00:00")
                .expect("an explicit offset must parse");

            // Act
            let resolved = DateTimeValue::parse_iso("2026-10-25T01:30:00");

            // Assert
            assert_eq!(
                resolved,
                Ok(earlier),
                "the London fold must take the first (BST) occurrence"
            );
            assert_ne!(
                resolved,
                Ok(later),
                "the later (GMT) occurrence must not be selected"
            );
        }

        #[test]
        fn resolves_the_lord_howe_half_hour_fold_to_the_earlier_occurrence() {
            // Arrange: Lord Howe shifts by 30 minutes, folding 2026-04-05.
            ZoneGuard::set("Australia/Lord_Howe");
            let earlier = DateTimeValue::parse_iso("2026-04-05T01:45:00+11:00")
                .expect("an explicit offset must parse");
            let later = DateTimeValue::parse_iso("2026-04-05T01:45:00+10:30")
                .expect("an explicit offset must parse");

            // Act
            let resolved = DateTimeValue::parse_iso("2026-04-05T01:45:00");

            // Assert
            assert_eq!(
                resolved,
                Ok(earlier),
                "the half-hour fold must take the first (DST) occurrence"
            );
            assert_ne!(
                resolved,
                Ok(later),
                "the later standard-time occurrence must not be selected"
            );
        }

        #[test]
        fn shifts_the_lord_howe_half_hour_gap_forward_by_the_gap() {
            // Arrange: the 2026-10-04 gap skips 02:00 to 02:30.
            ZoneGuard::set("Australia/Lord_Howe");
            let shifted = DateTimeValue::parse_iso("2026-10-04T02:45:00+11:00")
                .expect("an explicit offset must parse");

            // Act
            let resolved = DateTimeValue::parse_iso("2026-10-04T02:15:00");

            // Assert
            assert_eq!(
                resolved,
                Ok(shifted),
                "an in-gap time must shift forward by the 30-minute gap"
            );
        }

        #[test]
        fn reports_local_zone_lookup_when_the_extreme_local_datetime_cannot_resolve()
         {
            // Arrange: UTC+14 pushes the range edge out of range.
            ZoneGuard::set("Etc/GMT-14");

            // Act
            let result = DateTimeValue::from_local(NaiveDateTime::MIN);

            // Assert
            assert!(matches!(result, Err(CalendarError::Zone { .. })));
        }
    }

    mod parse {
        use chrono::NaiveDate;
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
        fn parses_each_accepted_shape_to_the_expected_utc_rendering(
            #[case] input: &str,
            #[case] expected_rendering: &str,
        ) {
            // Arrange: the case literals are the input and its UTC rendering.
            ZoneGuard::set("UTC");

            // Act
            let result = DateTimeValue::parse_iso(input);

            // Assert
            assert_eq!(
                result.map(|parsed| parsed.to_string()),
                Ok(expected_rendering.to_owned()),
                "`{input}` must parse and render as `{expected_rendering}`"
            );
        }

        #[test]
        fn parses_a_naive_date_time_in_the_local_zone() {
            // Arrange: UTC+2, no DST.
            ZoneGuard::set("Etc/GMT-2");

            // Act
            let result = DateTimeValue::parse_iso("2026-07-29 14:30:00");
            let expected = DateTimeValue::parse_iso("2026-07-29T12:30:00Z");

            // Assert
            assert_eq!(
                result, expected,
                "a naive clock time must resolve in the local zone and store \
                 as UTC"
            );
        }

        #[test]
        fn rejects_a_short_year_with_year_not_four_digits() {
            // Arrange: the two-digit year is the whole input's prefix.
            // Act
            let result = DateTimeValue::parse_iso("26-08-22T14:30:00");

            // Assert
            assert!(matches!(result, Err(DateError::YearNotFourDigits { .. })));
        }

        #[test]
        fn rejects_an_unrecognized_shape_as_invalid_value() {
            // Arrange: a valid year prefix with an unrecognized body.
            // Act
            let result = DateTimeValue::parse_iso("2026-07-29Tinvalid");

            // Assert
            assert!(matches!(result, Err(DateError::InvalidValue { .. })));
        }

        #[test]
        fn rejects_rfc3339_leap_seconds() {
            // Arrange
            let input = "2026-07-29T23:59:60Z";

            // Act
            let result = DateTimeValue::parse_iso(input);

            // Assert
            assert!(matches!(result, Err(DateError::InvalidValue { .. })));
        }
        #[rstest]
        #[case::missing_time("2026-07-29", "input is too short")]
        #[case::trailing_offset_text(
            "2026-07-29T14:30:00+02:00x",
            "invalid input"
        )]
        #[case::midnight_as_hour_24("2026-07-29T24:00:00", "invalid input")]
        #[case::trailing_text("2026-07-29T14:30:00junk", "invalid input")]
        #[case::over_padded_hour("2026-07-29T014:30:00", "invalid input")]
        fn rejects_malformed_inputs_with_the_expected_reason(
            #[case] input: &str,
            #[case] reason: &str,
        ) {
            // Arrange: `input` and the expected reason are the case literals.
            // Act
            let result = DateTimeValue::parse_iso(input);

            // Assert
            assert!(
                matches!(
                    result,
                    Err(DateError::InvalidValue {
                        reason: actual,
                        ..
                    }) if actual == reason
                ),
                "`{input}` must be rejected with `{reason}`, got {result:?}"
            );
        }

        #[rstest]
        #[case::leading(" 2026-07-29T14:30:00Z")]
        #[case::trailing("2026-07-29T14:30:00Z ")]
        fn trims_surrounding_whitespace(#[case] input: &str) {
            // Arrange: `input` is the case literal with one stray space.
            ZoneGuard::set("UTC");

            // Act
            let result = DateTimeValue::parse_iso(input);

            // Assert
            assert_eq!(
                result.map(|parsed| parsed.to_string()),
                Ok("2026-07-29T14:30:00".to_owned()),
                "surrounding whitespace must be ignored"
            );
        }

        #[test]
        fn accepts_whitespace_runs_at_the_date_time_separator() {
            // Arrange: UTC, so the rendering is the stored clock time.
            ZoneGuard::set("UTC");

            // Act
            let double_space = DateTimeValue::parse_iso("2026-07-29  14:30:00");
            let tab = DateTimeValue::parse_iso("2026-07-29\t14:30:00");

            // Assert
            assert_eq!(
                double_space.map(|parsed| parsed.to_string()),
                Ok("2026-07-29T14:30:00".to_owned()),
                "a run of spaces between date and time must still parse"
            );
            assert_eq!(
                tab.map(|parsed| parsed.to_string()),
                Ok("2026-07-29T14:30:00".to_owned()),
                "a tab between date and time must still parse"
            );
        }

        #[test]
        fn rejects_unpadded_month_day_fields() {
            // Arrange: strict ISO parsing requires a padded date component.
            let input = "2026-7-29T14:30:00";

            // Act
            let result = DateTimeValue::parse_iso(input);

            // Assert
            assert_eq!(
                result,
                Err(DateError::InvalidValue {
                    input: input.into(),
                    reason: "input is too short",
                })
            );
        }

        #[test]
        fn parses_whole_seconds_through_the_fractional_shape() {
            // Arrange: the same wall-clock time in both separators.
            let local_datetime = NaiveDate::from_ymd_opt(2026, 7, 29)
                .and_then(|date| date.and_hms_opt(14, 30, 0))
                .expect("valid fixture date-time");

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
        fn maps_each_shape_to_its_strftime_pattern(
            #[case] shape: DateTimeShape,
            #[case] expected: Option<&str>,
        ) {
            // Arrange: `shape` is the case value.
            // Act
            let pattern = shape.pattern();

            // Assert
            assert_eq!(pattern, expected, "one strftime pattern per shape");
        }
    }

    mod formatting {
        use pretty_assertions::{assert_eq, assert_ne};

        use super::*;

        #[test]
        fn renders_the_local_clock_time_without_the_offset() {
            // Arrange: UTC+2, no DST.
            ZoneGuard::set("Etc/GMT-2");

            // Act
            let rendered = fixed_datetime().to_string();

            // Assert
            assert_eq!(
                rendered, "2026-07-29T16:30:05",
                "the human-facing spelling is the local clock time"
            );
        }

        #[test]
        fn renders_the_zero_padded_canonical_form() {
            // Arrange: UTC, so the rendering is the stored clock time.
            ZoneGuard::set("UTC");

            // Act
            let rendered = fixed_datetime().to_string();

            // Assert
            assert_eq!(
                rendered, "2026-07-29T14:30:05",
                "the canonical form is zero-padded ISO without an offset"
            );
        }

        #[test]
        fn renders_fractional_seconds_only_when_nonzero() {
            // Arrange: whole seconds plus 123 milliseconds.
            ZoneGuard::set("UTC");
            let with_nanos = DateTimeValue::try_from(
                fixed_datetime().utc() + TimeDelta::milliseconds(123),
            )
            .expect("the fixture and millisecond shift fit Timestamp's range");
            assert_eq!(
                with_nanos.local_datetime().map(|local| local.nanosecond()),
                Some(123_000_000),
            );

            // Act
            let rendered = with_nanos.to_string();

            // Assert
            assert_eq!(
                rendered, "2026-07-29T14:30:05.123",
                "fractional seconds must appear only for a nonzero nanosecond \
                 component"
            );
        }

        #[test]
        fn renders_the_fold_hour_that_re_parses_to_the_earlier_occurrence() {
            // Arrange: the second occurrence of the folded 01:30 clock time.
            ZoneGuard::set("America/New_York");
            let later = DateTimeValue::parse_iso("2026-11-01T06:30:00Z")
                .expect("explicit-offset fixture parses");

            // Act
            let rendered = later.to_string();
            let reparsed = DateTimeValue::parse_iso(&rendered);
            let earlier = DateTimeValue::parse_iso("2026-11-01T05:30:00Z");

            // Assert
            assert_eq!(
                rendered, "2026-11-01T01:30:00",
                "a fold-hour date-time renders its local clock time"
            );
            assert_eq!(
                reparsed, earlier,
                "a fold-hour rendering must re-parse to the earlier occurrence"
            );
            assert_ne!(
                reparsed,
                Ok(later),
                "re-parsing the fold-hour rendering must not return the later \
                 occurrence"
            );
        }

        #[test]
        fn renders_the_local_clock_at_the_jiff_timestamp_upper_edge() {
            // Arrange: Jiff reserves offset headroom at the timestamp bound.
            ZoneGuard::set("Etc/GMT-2");
            let edge = DateTimeValue(Timestamp::MAX);

            // Act
            let rendered = edge.to_string();

            // Assert
            assert_eq!(rendered, "9999-12-31T00:00:00.999999999");
            assert_ne!(rendered, edge.utc().naive_utc().to_string());
        }

        #[test]
        fn round_trips_display_output_back_through_parse_iso() {
            // Arrange: the current zone (kept) hosts this rendering.
            ZoneGuard::keep();
            let value = fixed_datetime();

            // Act
            let reparsed = DateTimeValue::parse_iso(&value.to_string());

            // Assert
            assert_eq!(
                Ok(value),
                reparsed,
                "display output must re-parse to the same date-time"
            );
        }
    }

    mod serialization {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn serializes_as_the_utc_z_string() {
            // Arrange: the fixed reference date-time.
            // Act
            let json = serde_json::to_string(&fixed_datetime());

            // Assert
            assert_eq!(
                json.ok().as_deref(),
                Some("\"2026-07-29T14:30:05Z\""),
                "serialization must carry an explicit RFC 3339 UTC spelling"
            );
        }

        #[test]
        fn keeps_its_postcard_spelling() {
            // Arrange
            let value = fixed_datetime();

            // Act
            let bytes = postcard::to_allocvec(&value);

            // Assert
            assert_eq!(
                bytes.as_deref(),
                Ok(b"\x142026-07-29T14:30:05Z".as_slice()),
                "postcard encodes the length-prefixed UTC Z string"
            );
            assert_eq!(
                bytes
                    .and_then(|bytes| {
                        postcard::from_bytes::<DateTimeValue>(&bytes)
                    })
                    .map_err(|error| error.to_string()),
                Ok(value),
                "postcard encode then decode must return the same date-time"
            );
        }

        #[test]
        fn serializes_sub_second_precision_with_autosi() {
            // Arrange: a three-fraction-digit date-time.
            let value = DateTimeValue::parse_iso("2026-07-29T14:30:00.123Z")
                .expect("explicit-offset fixture parses");

            // Act
            let json = serde_json::to_string(&value);

            // Assert
            assert_eq!(
                json.ok().as_deref(),
                Some("\"2026-07-29T14:30:00.123Z\""),
                "AutoSi precision must keep the three written fraction digits"
            );
        }

        #[rstest]
        #[case::whole_seconds(0)]
        #[case::sub_second(123)]
        fn round_trips_through_json(#[case] milliseconds: i64) {
            // Arrange: the case adds whole seconds or 123 milliseconds.
            let value = DateTimeValue::try_from(
                fixed_datetime()
                    .utc()
                    .checked_add_signed(TimeDelta::milliseconds(milliseconds))
                    .expect("the fixture plus milliseconds is in range"),
            )
            .expect("the fixture plus milliseconds fits Timestamp's range");
            let json =
                serde_json::to_string(&value).expect("the case serializes");

            // Act
            let restored: Result<DateTimeValue, _> =
                serde_json::from_str(&json);

            // Assert
            assert_eq!(
                restored.ok(),
                Some(value),
                "JSON round trips must be lossless"
            );
        }
    }

    mod deserialization {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn deserializes_an_rfc3339_utc_string() {
            // Arrange: the serialized spelling of the fixed reference
            // date-time.
            let json = "\"2026-07-29T14:30:05Z\"";

            // Act
            let result = serde_json::from_str::<DateTimeValue>(json);

            // Assert
            assert_eq!(
                result.ok(),
                Some(fixed_datetime()),
                "an RFC 3339 UTC string must deserialize to the same date-time"
            );
        }

        #[test]
        fn deserializes_a_yaml_z_spelling_to_the_same_utc_date_time() {
            #[derive(serde::Deserialize)]
            struct YamlNote {
                datetime: DateTimeValue,
            }

            // Arrange: YAML strings share the serde string path; the "Z"
            // spelling must not mean the local zone.
            let yaml = "datetime: \"2026-07-29T14:30:00Z\"\n";

            // Act
            let note = noyalib::from_str::<YamlNote>(yaml)
                .expect("YAML note deserializes");

            // Assert
            assert_eq!(
                note.datetime,
                DateTimeValue::parse_iso("2026-07-29T14:30:00+00:00")
                    .expect("valid datetime"),
                "\"Z\" must parse as UTC, not the local zone"
            );
            assert_eq!(
                serde_json::to_string(&note.datetime).expect("serializable"),
                "\"2026-07-29T14:30:00Z\"",
                "re-serialization must keep the Z spelling"
            );
        }

        #[test]
        fn rejects_an_invalid_string_with_the_parse_error() {
            // Arrange: an input the parser rejects before any zone lookup.
            let json = "\"not-a-date\"";

            // Act
            let result = serde_json::from_str::<DateTimeValue>(json);

            // Assert
            assert!(
                matches!(
                    result,
                    Err(error) if error
                        .to_string()
                        .contains("`not-a-date` does not have a 4-digit year")
                ),
                "deserialization must surface the parser's own error"
            );
        }
    }

    mod conversions {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn promotes_a_date_to_local_zone_midnight() {
            // Arrange: UTC+2, no DST.
            ZoneGuard::set("Etc/GMT-2");
            let date = ymd(2026, 7, 29);

            // Act
            let promoted = DateTimeValue::try_from(date)
                .expect("test date has a representable local midnight");
            let expected = DateTimeValue::parse_iso("2026-07-28T22:00:00Z");

            // Assert
            assert_eq!(
                expected,
                Ok(promoted),
                "local midnight in UTC+2 is 22:00Z on the previous day"
            );
        }

        #[test]
        fn advances_a_promoted_midnight_across_a_zone_gap() {
            // Arrange: local midnight falls inside this spring-forward gap.
            ZoneGuard::set("America/Santiago");
            let date = ymd(2026, 9, 6);

            // Act
            let promoted = DateTimeValue::try_from(date)
                .expect("test date has a representable local midnight");
            let expected = DateTimeValue::parse_iso("2026-09-06T04:00:00Z");

            // Assert
            assert_eq!(
                expected,
                Ok(promoted),
                "midnight inside a DST gap must advance by the gap length"
            );
            assert_eq!(
                promoted.to_string(),
                "2026-09-06T01:00:00",
                "the advanced midnight must render as the post-gap clock time"
            );
        }

        #[test]
        fn rejects_a_date_midnight_outside_the_timestamp_range() {
            // Arrange
            ZoneGuard::set("Etc/GMT-14");
            let date = ymd(-9999, 1, 1);

            // Act
            let error = DateTimeValue::try_from(date).expect_err(
                "the minimum civil date is outside the timestamp range",
            );

            // Assert
            assert!(matches!(
                error,
                DateError::InvalidValue { input, reason }
                    if input.as_ref() == "-9999-01-01 00:00:00"
                        && reason == "date/time is outside the representable range"
            ));
        }

        #[test]
        fn converts_a_system_time() {
            // Arrange: one thousand seconds past the epoch, UTC.
            ZoneGuard::set("UTC");
            let system_time =
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000);

            // Act
            let converted = DateTimeValue::try_from(system_time)
                .expect("test system time is within the supported range");
            let expected = DateTimeValue::parse_iso("1970-01-01T00:16:40");

            // Assert
            assert_eq!(
                expected,
                Ok(converted),
                "a SystemTime position must convert to the same date-time"
            );
        }

        #[test]
        fn round_trips_through_chrono_date_time_utc() {
            // Arrange: the fixed reference date-time in the UTC frame.
            let chrono_dt = fixed_datetime().utc();

            // Act
            let converted = DateTimeValue::try_from(chrono_dt)
                .expect("test date-time is within the supported range");
            let back: DateTime<Utc> = converted.into();

            // Assert
            assert_eq!(
                converted,
                fixed_datetime(),
                "a wrapped UTC date-time must equal the original"
            );
            assert_eq!(
                back, chrono_dt,
                "unwrapping must return the original UTC date-time"
            );
        }
    }

    mod shift {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn keeps_fractional_seconds_through_a_clock_unit_shift() {
            // Arrange: a date-time with 123 milliseconds.
            ZoneGuard::set("UTC");
            let base = DateTimeValue::parse_iso("2026-07-29T14:30:00.123Z")
                .expect("explicit-offset fixture parses");

            // Act
            let shifted = base.shift(1, TimeUnit::Hour);

            // Assert
            assert_eq!(
                shifted.map(|value| value.to_string()),
                Ok("2026-07-29T15:30:00.123".to_owned()),
                "a clock-unit shift must keep the fractional seconds"
            );
        }

        #[test]
        fn re_resolves_a_fold_hour_to_the_earlier_occurrence_on_a_zero_day_shift()
         {
            // Arrange: the second occurrence of the folded 01:30 clock time.
            ZoneGuard::set("America/New_York");
            let second_occurrence =
                DateTimeValue::parse_iso("2026-11-01T06:30:00Z")
                    .expect("explicit-offset fixture parses");

            // Act: calendar shifts re-resolve the local clock time.
            let shifted = second_occurrence.shift(0, TimeUnit::Day);

            // Assert
            assert_eq!(
                shifted,
                Ok(DateTimeValue::parse_iso("2026-11-01T05:30:00Z")
                    .expect("explicit-offset fixture parses")),
                "a zero-day calendar shift must re-resolve the fold to the \
                 earlier occurrence"
            );
        }

        #[test]
        fn keeps_the_date_time_under_a_zero_day_shift_in_utc() {
            // Arrange: UTC carries no fold or gap.
            ZoneGuard::set("UTC");
            let base = DateTimeValue::parse_iso("2026-07-29T14:30:00Z")
                .expect("explicit-offset fixture parses");

            // Act
            let shifted = base.shift(0, TimeUnit::Day);

            // Assert
            assert_eq!(
                shifted,
                Ok(base),
                "a zero-day shift in UTC must keep the date-time"
            );
        }
    }

    mod ordering {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn orders_date_times_chronologically() {
            // Arrange: two explicit-offset date-times, half a year apart.
            let earlier = DateTimeValue::parse_iso("2026-01-01T00:00:00Z")
                .expect("explicit-offset fixture parses");
            let later = fixed_datetime();

            // Act
            let ordering = earlier.cmp(&later);

            // Assert
            assert_eq!(
                ordering,
                std::cmp::Ordering::Less,
                "the earlier date-time must order first"
            );
            assert!(
                later > earlier,
                "the later date-time must compare greater"
            );
            assert_eq!(
                earlier.cmp(&earlier),
                std::cmp::Ordering::Equal,
                "equal date-times must compare equal"
            );
        }
    }

    mod equality {
        use super::*;

        #[test]
        fn requires_exact_local_midnight_for_is_midnight_of() {
            // Arrange: the current zone (kept) hosts the promotion.
            ZoneGuard::keep();
            let date = ymd(2026, 7, 29);

            // Act
            let midnight = DateTimeValue::try_from(date)
                .expect("test date has a representable local midnight");
            let time_of_day = fixed_datetime();

            // Assert
            assert!(
                midnight.is_midnight_of(date),
                "the promoted midnight must count as midnight of its date"
            );
            assert!(
                !time_of_day.is_midnight_of(date),
                "a date-time with a time of day must not count as midnight"
            );
        }
    }

    mod accessors {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn extracts_the_calendar_date_discarding_time_of_day() {
            // Arrange: UTC, so the stored calendar date is the local one.
            ZoneGuard::set("UTC");

            // Act
            let extracted = fixed_datetime().local_date();

            // Assert
            assert_eq!(
                extracted,
                Ok(ymd(2026, 7, 29)),
                "the UTC calendar date must be extracted"
            );
        }

        #[test]
        fn extracts_the_local_calendar_date_of_a_date_time() {
            // Arrange: UTC+12 rolls 14:30Z into the next local day.
            ZoneGuard::set("Etc/GMT-12");

            // Act
            let extracted = fixed_datetime().local_date();

            // Assert
            assert_eq!(
                extracted,
                Ok(ymd(2026, 7, 30)),
                "the local calendar date must roll forward with the offset"
            );
        }
    }
}

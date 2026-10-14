//! Defines UTC-stored Date-Times that render as the local clock.
//!
//! Owns [`DateTimeValue`] and the recognition shapes ([`DateTimeShape`],
//! [`ParsedDateTimeKind`]) that turn written RFC 3339 and naive clock times
//! into it. Written input without an offset resolves in the process's local
//! zone and stores as UTC; every human-facing rendering converts back to the
//! local clock. Callers hold the validated date-time or receive a
//! [`DateError`].

use std::{borrow::Cow, fmt, str::FromStr, time::SystemTime};

use jiff::{SignedDuration, Span, Timestamp, civil, tz::TimeZone};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    CalendarError, DateValue, TimeUnit, calendar::shift_datetime,
    date::has_padded_date_prefix, error::DateError, point::starts_with_year,
    zone::local_zone,
};

/// Defines the canonical display format for [`DateTimeValue`]:
/// `2026-07-29T14:30:00`.
pub(crate) const DEFAULT_DATETIME_PATTERN: &str = "%Y-%m-%dT%H:%M:%S";

/// Stores a Date-Time in UTC and renders it as the local clock.
///
/// Accepts RFC 3339 input with an explicit offset, except leap seconds, which
/// Jiff cannot represent. ISO date portions require zero-padded month and day
/// fields. Naive input in the five accepted shapes resolves in the
/// process-local zone. Folds take the earlier occurrence; gaps shift forward by
/// their length. Pattern-based parsing uses a separate strftime grammar and
/// retains its pattern-driven padding rules.
///
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
/// let from_seconds = DateTimeValue::from_unix_seconds(0)?;
/// assert_eq!(from_seconds, epoch);
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
    /// Constructs a UTC date-time from whole seconds since the UTC epoch.
    ///
    /// The input is a number of seconds from `1970-01-01T00:00:00Z`.
    ///
    /// # Errors
    ///
    /// - [`DateError::InvalidValue`] if `seconds` lies outside Jiff’s
    ///   representable timestamp range.
    ///
    /// # Examples
    ///
    /// ```
    /// use traces_pkm::DateTimeValue;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let epoch = DateTimeValue::from_unix_seconds(0)?;
    /// assert_eq!(epoch, "1970-01-01T00:00:00Z".parse()?);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn from_unix_seconds(seconds: i64) -> Result<Self, DateError> {
        Timestamp::from_second(seconds).map(Self).map_err(|_| {
            DateError::InvalidValue {
                input: seconds.to_string().into(),
                reason: "date/time is outside the representable range",
            }
        })
    }

    /// Wraps a Jiff [`Timestamp`].
    #[inline]
    pub(crate) const fn from_timestamp(timestamp: Timestamp) -> Self {
        Self(timestamp)
    }

    /// Returns the exact elapsed time from `earlier` to this value.
    #[inline]
    #[must_use]
    pub(crate) fn duration_since(self, earlier: Self) -> SignedDuration {
        self.0.duration_since(earlier.0)
    }

    /// Returns the local clock-time rendering of this date-time.
    ///
    /// # Errors
    ///
    /// - [`DateError::LocalZoneLookup`] if system time-zone discovery fails.
    #[inline]
    pub(crate) fn local_datetime(self) -> Result<civil::DateTime, DateError> {
        Ok(self.local_datetime_in(&local_zone()?))
    }

    /// Converts the UTC value to local civil time using the supplied zone.
    #[inline]
    #[must_use]
    pub(crate) fn local_datetime_in(self, zone: &TimeZone) -> civil::DateTime {
        self.0.to_zoned(zone.clone()).datetime()
    }

    /// Returns the local clock time of this date-time.
    ///
    /// # Errors
    ///
    /// - [`DateError::LocalZoneLookup`] if system time-zone discovery fails.
    #[inline]
    pub(crate) fn local_datetime_or_utc(
        self,
    ) -> Result<civil::DateTime, DateError> {
        self.local_datetime()
    }

    /// Returns the local calendar date of this date-time, discarding
    /// time-of-day.
    ///
    /// # Errors
    ///
    /// - [`DateError::LocalZoneLookup`] if system time-zone discovery fails.
    #[inline]
    pub(crate) fn local_date(self) -> Result<DateValue, DateError> {
        Ok(self.local_datetime_or_utc()?.date().into())
    }

    /// Returns `true` if this date-time is exactly local-zone midnight on
    /// `date`.
    #[must_use]
    pub(crate) fn is_midnight_of(self, date: DateValue) -> bool {
        Self::try_from(date).is_ok_and(|midnight| self.0 == midnight.0)
    }

    /// Returns the wrapped UTC Date-Time.
    #[inline]
    #[must_use]
    pub(crate) const fn utc(self) -> Timestamp {
        self.0
    }

    /// Returns the canonical strftime pattern for this date-time: whole
    /// seconds, plus fractional seconds only when the nanosecond component is
    /// nonzero.
    ///
    /// Local offsets are whole seconds, so the UTC and local readings share
    /// their nanosecond component and no zone lookup is needed.
    #[must_use]
    pub(crate) fn canonical_pattern(self) -> &'static str {
        let nanoseconds = self.0.subsec_nanosecond();
        if nanoseconds == 0 {
            DEFAULT_DATETIME_PATTERN
        } else if nanoseconds % 1_000_000 == 0 {
            "%Y-%m-%dT%H:%M:%S%.3f"
        } else if nanoseconds % 1_000 == 0 {
            "%Y-%m-%dT%H:%M:%S%.6f"
        } else {
            "%Y-%m-%dT%H:%M:%S%.9f"
        }
    }

    /// Resolves a naive local clock time under Jiff's compatible DST policy.
    ///
    /// Ambiguous fall-back times choose their earlier occurrence;
    /// spring-forward gaps shift forward by the gap length.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::Zone`] if process-local time-zone discovery fails.
    /// - [`CalendarError::OutOfRange`] if the resolved local time falls outside
    ///   Jiff's supported timestamp range.
    pub(super) fn from_local(
        local_datetime: civil::DateTime,
    ) -> Result<Self, CalendarError> {
        let zone = local_zone().map_err(|error| CalendarError::Zone {
            input: error.to_string().into(),
        })?;
        Self::from_local_in(local_datetime, &zone)
    }

    /// Resolves a naive local clock time in the explicitly supplied zone.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the supplied zone shifts the local
    ///   time outside Jiff's supported timestamp range.
    pub(super) fn from_local_in(
        local_datetime: civil::DateTime,
        zone: &TimeZone,
    ) -> Result<Self, CalendarError> {
        local_datetime
            .to_zoned(zone.clone())
            .map(|zoned| Self(zoned.timestamp()))
            .map_err(|_| CalendarError::OutOfRange)
    }

    /// Shifts calendar units in the local datetime frame and clock units in
    /// UTC.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the shift overflows or its local
    ///   date-time lies outside the supported range.
    pub(crate) fn shift(
        self,
        n: i64,
        unit: TimeUnit,
        zone: &TimeZone,
    ) -> Result<Self, CalendarError> {
        if unit.is_calendar() {
            let local_datetime = self.local_datetime_in(zone);
            return Self::from_local_in(
                shift_datetime(local_datetime, n, unit)?,
                zone,
            );
        }
        let span = match unit {
            TimeUnit::Millisecond => Span::new().try_milliseconds(n),
            TimeUnit::Second => Span::new().try_seconds(n),
            TimeUnit::Minute => Span::new().try_minutes(n),
            TimeUnit::Hour => Span::new().try_hours(n),
            TimeUnit::Day
            | TimeUnit::Week
            | TimeUnit::Month
            | TimeUnit::Year => {
                return Err(CalendarError::OutOfRange);
            }
        }
        .map_err(|_| CalendarError::OutOfRange)?;
        self.checked_add_span(span, zone)
    }

    /// Adds a Jiff span to this Date-Time in the supplied time zone.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the resulting Date-Time exceeds the
    ///   representable range.
    pub(crate) fn checked_add_span(
        self,
        span: Span,
        zone: &TimeZone,
    ) -> Result<Self, CalendarError> {
        self.0
            .to_zoned(zone.clone())
            .checked_add(span)
            .map(|zoned| Self(zoned.timestamp()))
            .map_err(|_| CalendarError::OutOfRange)
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
            ParsedDateTimeKind::Instant(timestamp) => {
                Ok(Self::from_timestamp(timestamp))
            }
            ParsedDateTimeKind::Unzoned(local_datetime) => {
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
/// sub-second precision survives display.
impl fmt::Display for DateTimeValue {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let local = self.local_datetime().map_err(|_| fmt::Error)?;
        let rendered =
            jiff::fmt::strtime::format(self.canonical_pattern(), local)
                .map_err(|_| fmt::Error)?;
        f.write_str(&rendered)
    }
}

impl TryFrom<SystemTime> for DateTimeValue {
    type Error = DateError;

    /// Converts a system time to a UTC-stored Date-Time.
    ///
    /// # Errors
    ///
    /// - [`DateError::InvalidValue`] if `time` lies outside Jiff's
    ///   representable timestamp range.
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

impl TryFrom<DateValue> for DateTimeValue {
    type Error = DateError;

    /// Converts a calendar date to local midnight.
    ///
    /// # Errors
    ///
    /// - [`DateError::LocalZoneLookup`] if system time-zone discovery fails.
    /// - [`DateError::InvalidValue`] if midnight is outside the supported
    ///   range.
    #[inline]
    fn try_from(date: DateValue) -> Result<Self, Self::Error> {
        let midnight = date.civil_date().at(0, 0, 0, 0);
        let zone = local_zone()?;
        Self::from_local_in(midnight, &zone)
            .map_err(|error| error.into_date_error(&midnight.to_string()))
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
        let mut rendered = jiff::fmt::strtime::format(
            self.canonical_pattern(),
            self.utc().to_zoned(TimeZone::UTC).datetime(),
        )
        .map_err(serde::ser::Error::custom)?;
        rendered.push('Z');
        serializer.serialize_str(&rendered)
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

/// Defines recognized date-time input shapes, tried in order by
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
    /// Returns a static reason if no accepted date-time shape matches `s`.
    pub(crate) fn parse_any(
        s: &str,
    ) -> Result<ParsedDateTimeKind, &'static str> {
        let normalized = Self::truncate_fractional_seconds(s);
        let normalized_input = normalized.as_ref();
        if normalized_input.len() <= 10 {
            return Err("input is too short");
        }
        if normalized_input.get(16..19) == Some(":60") {
            return Err("invalid input");
        }
        let [first, rest @ ..] = Self::ALL;
        let mut result = first.parse_shape(normalized_input);
        for format in rest {
            if result.is_ok() {
                break;
            }
            result = format.parse_shape(normalized_input);
        }
        result.and_then(|parsed| {
            has_padded_date_prefix(normalized_input)
                .then_some(parsed)
                .ok_or("input is too short")
        })
    }

    fn truncate_fractional_seconds(input: &str) -> Cow<'_, str> {
        let Some((prefix, fraction)) = input.split_once('.') else {
            return Cow::Borrowed(input);
        };
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if digits <= 9 {
            return Cow::Borrowed(input);
        }
        let Some(leading) = fraction.get(..9) else {
            return Cow::Borrowed(input);
        };
        let Some(suffix) = fraction.get(digits..) else {
            return Cow::Borrowed(input);
        };
        Cow::Owned(format!("{prefix}.{leading}{suffix}"))
    }

    /// Parses this format into a parsed date-time shape.
    ///
    /// # Errors
    ///
    /// - A static reason if the input does not match this format.
    fn parse_shape(self, s: &str) -> Result<ParsedDateTimeKind, &'static str> {
        match self.pattern() {
            None if !numeric_rfc3339_offset_is_valid(s) => {
                Err("input does not match RFC 3339")
            }
            None => s
                .parse::<Timestamp>()
                .map(ParsedDateTimeKind::Instant)
                .map_err(|_| "input does not match RFC 3339"),
            Some(pattern) => civil::DateTime::strptime(pattern, s)
                .map(ParsedDateTimeKind::Unzoned)
                .map_err(|_| "invalid input"),
        }
    }
}

/// Returns `true` if the RFC 3339 numeric offset in `input` is valid.
fn numeric_rfc3339_offset_is_valid(input: &str) -> bool {
    let Some(tail) = input.get(10..) else {
        return true;
    };
    let Some(start) = tail.rfind(['+', '-']) else {
        return true;
    };
    let Some(offset) = tail.as_bytes().get(start..) else {
        return false;
    };
    let Ok(offset) = <&[u8; 6]>::try_from(offset) else {
        return false;
    };
    let [sign, hour_tens, hour_ones, colon, minute_tens, minute_ones] = *offset;
    if !matches!(sign, b'+' | b'-')
        || colon != b':'
        || !hour_tens.is_ascii_digit()
        || !hour_ones.is_ascii_digit()
        || !minute_tens.is_ascii_digit()
        || !minute_ones.is_ascii_digit()
    {
        return false;
    }
    let Some(hours) = offset
        .get(1..3)
        .and_then(|digits| std::str::from_utf8(digits).ok())
        .and_then(|digits| digits.parse::<u16>().ok())
    else {
        return false;
    };
    let Some(minutes) = offset
        .get(4..6)
        .and_then(|digits| std::str::from_utf8(digits).ok())
        .and_then(|digits| digits.parse::<u16>().ok())
    else {
        return false;
    };
    hours <= 23 && minutes <= 59
}

/// Stores the parsed date-time shape before local-zone resolution.
#[derive(Copy, Clone, Debug)]
pub(crate) enum ParsedDateTimeKind {
    /// Stores an explicit RFC 3339 date-time with its offset applied.
    Instant(Timestamp),
    /// Stores a naive local civil date-time that must resolve through
    /// [`DateTimeValue::from_local`].
    Unzoned(civil::DateTime),
}

#[cfg(test)]
mod tests {

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
        fn constructs_unix_seconds_at_both_supported_edges() {
            // Arrange: both endpoints, the epoch, and one second before it.
            let before_epoch = -1;
            let seconds = [
                Timestamp::MIN.as_second(),
                0,
                before_epoch,
                Timestamp::MAX.as_second(),
            ];
            // Act
            let results = seconds.map(DateTimeValue::from_unix_seconds);
            // Assert
            assert!(results.into_iter().zip(seconds).all(|(result, expected)| {
                matches!(result, Ok(value) if value.utc().as_second() == expected)
            }));
        }

        #[test]
        fn rejects_unix_seconds_one_past_each_supported_edge() {
            // Arrange: each value is immediately beyond Jiff's timestamp range.
            let below_minimum = Timestamp::MIN.as_second() - 1;
            let above_maximum = Timestamp::MAX.as_second() + 1;
            let seconds = [below_minimum, above_maximum];
            // Act
            let results = seconds.map(DateTimeValue::from_unix_seconds);
            // Assert
            assert!(results.into_iter().all(|result| {
                matches!(result, Err(DateError::InvalidValue { .. }))
            }));
        }

        #[test]
        fn constructs_the_unix_epoch_from_system_time() {
            // Arrange: the epoch and the UTC date-time it must equal.
            let epoch = SystemTime::UNIX_EPOCH;
            let expected: Timestamp =
                "1970-01-01T00:00:00Z".parse().expect("valid epoch timestamp");
            // Act
            let result = DateTimeValue::try_from(epoch);

            // Assert
            assert!(
                matches!(result, Ok(value) if value.utc() == expected),
                "the epoch must construct as 1970-01-01T00:00:00Z"
            );
        }

        #[test]
        fn constructs_a_pre_epoch_date_time_from_system_time() {
            // Arrange: one thousand seconds before the epoch.
            let before_epoch =
                SystemTime::UNIX_EPOCH - Duration::from_secs(1_000);
            let expected: Timestamp = "1969-12-31T23:43:20Z"
                .parse()
                .expect("valid pre-epoch timestamp");
            // Act
            let result = DateTimeValue::try_from(before_epoch);

            // Assert
            assert!(
                matches!(result, Ok(value) if value.utc() == expected),
                "negative timeline positions must construct before the epoch"
            );
        }

        #[rstest]
        #[case::minimum(Timestamp::MIN)]
        #[case::maximum(Timestamp::MAX)]
        fn wraps_the_representable_range_edges(#[case] edge: Timestamp) {
            // Arrange: the edge is a bound of the Jiff timestamp range.
            let value = DateTimeValue(edge);

            // Act
            let actual = value.0;

            // Assert
            assert_eq!(actual, edge);
        }

        #[test]
        fn rejects_one_second_past_each_representable_edge() {
            // Arrange: the seconds immediately outside Jiff's timestamp range.
            let below = Timestamp::MIN
                .as_second()
                .checked_sub(1)
                .expect("the minimum second is not i64::MIN");
            let above = Timestamp::MAX
                .as_second()
                .checked_add(1)
                .expect("the maximum second is not i64::MAX");

            // Act
            let lower = Timestamp::new(below, 0);
            let upper = Timestamp::new(above, 0);

            // Assert
            assert!(lower.is_err(), "one second past the minimum is invalid");
            assert!(upper.is_err(), "one second past the maximum is invalid");
        }
    }

    mod disambiguation {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn resolves_an_ambiguous_fall_back_time_to_the_earlier_occurrence() {
            // Arrange: 01:30 happens twice on this fall-back day.
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());

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
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());

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
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());

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
            ZoneGuard::set_timezone(crate::temporal::fixtures::london_zone());
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
            ZoneGuard::set_timezone(crate::temporal::fixtures::lord_howe_zone());
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
            ZoneGuard::set_timezone(crate::temporal::fixtures::lord_howe_zone());
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
        fn reports_out_of_range_when_the_extreme_local_datetime_cannot_resolve()
        {
            // Arrange: UTC+14 pushes the range edge out of range.
            ZoneGuard::set("Etc/GMT-14");

            // Act
            let result = DateTimeValue::from_local(civil::DateTime::MIN);

            // Assert
            assert_eq!(result, Err(CalendarError::OutOfRange));
        }
        #[test]
        fn resolves_the_sydney_southern_dst_fold_to_the_earlier_occurrence() {
            // Arrange: Sydney's 2026 fall-back repeats 02:30.
            ZoneGuard::set("Australia/Sydney");
            let earlier = DateTimeValue::parse_iso("2026-04-04T15:30:00Z")
                .expect("explicit earlier instant is valid");
            let later = DateTimeValue::parse_iso("2026-04-04T16:30:00Z")
                .expect("explicit later instant is valid");
            // Act
            let resolved = DateTimeValue::parse_iso("2026-04-05T02:30:00");
            // Assert
            assert_eq!(resolved, Ok(earlier));
            assert_ne!(resolved, Ok(later));
        }

        #[test]
        fn shifts_a_midnight_gap_forward_by_the_gap() {
            // Arrange: the fixture advances from 23:59 to 01:00 on this date.
            ZoneGuard::set_timezone(
                crate::temporal::fixtures::midnight_gap_zone(),
            );
            let shifted = DateTimeValue::parse_iso("2026-03-08T00:30:00Z")
                .expect("explicit shifted instant is valid");
            // Act
            let resolved = DateTimeValue::parse_iso("2026-03-08T00:30:00");
            // Assert
            assert_eq!(resolved, Ok(shifted));
        }

        #[test]
        fn resolves_unambiguous_input_in_a_no_dst_zone() {
            // Arrange
            ZoneGuard::set("Etc/UTC");
            let expected = DateTimeValue::parse_iso("2026-07-29T14:30:00Z");
            // Act
            let resolved = DateTimeValue::parse_iso("2026-07-29T14:30:00");
            // Assert
            assert_eq!(resolved, expected);
        }
    }

    mod parse {
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
        #[case::overprecision_truncates_to_nanoseconds(
            "2026-07-29T14:30:00.1234567891Z",
            "2026-07-29T14:30:00.123456789"
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
        #[rstest]
        #[case::unpadded_day("2026-07-9T14:30:00")]
        #[case::offset_hour_out_of_range("2026-07-29T14:30:00+25:00")]
        #[case::offset_minute_out_of_range("2026-07-29T14:30:00+02:60")]
        fn rejects_malformed_date_time_inputs(#[case] input: &str) {
            // Arrange: `input` violates the strict date-time grammar.
            // Act
            let result = DateTimeValue::parse_iso(input);
            // Assert
            assert!(
                matches!(result, Err(DateError::InvalidValue { .. })),
                "invalid date-time {input:?} must be rejected"
            );
        }

        #[rstest]
        #[case::empty("")]
        #[case::whitespace("   ")]
        fn rejects_empty_input_before_shape_matching(#[case] input: &str) {
            // Arrange: empty input has no four-digit year prefix.
            // Act
            let result = DateTimeValue::parse_iso(input);
            // Assert
            assert!(matches!(result, Err(DateError::YearNotFourDigits { .. })));
        }

        #[test]
        fn parses_whole_seconds_through_the_fractional_shape() {
            // Arrange: the same wall-clock time in both separators.
            let local_datetime = civil::Date::new(2026, 7, 29)
                .expect("valid fixture date")
                .at(14, 30, 0, 0);
            // Act
            let t_separated = DateTimeShape::parse_any("2026-07-29T14:30:00");
            let space_separated =
                DateTimeShape::parse_any("2026-07-29 14:30:00");

            // Assert
            assert!(matches!(
                t_separated,
                Ok(ParsedDateTimeKind::Unzoned(parsed)) if parsed == local_datetime
            ));
            assert!(matches!(
                space_separated,
                Ok(ParsedDateTimeKind::Unzoned(parsed)) if parsed == local_datetime
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
        fn renders_microseconds_with_six_fractional_digits() {
            // Arrange: UTC makes the local rendering identical to the input.
            ZoneGuard::set("UTC");
            let value = DateTimeValue::parse_iso("2026-07-29T14:30:00.123456Z")
                .expect("microsecond timestamp is valid");
            // Act
            let rendered = value.to_string();
            // Assert
            assert_eq!(rendered, "2026-07-29T14:30:00.123456");
        }

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
            let with_nanos = DateTimeValue::from_timestamp(
                fixed_datetime()
                    .utc()
                    .checked_add(
                        Span::new()
                            .try_milliseconds(123)
                            .expect("valid millisecond span"),
                    )
                    .expect("the fixture and millisecond shift fit"),
            );
            assert_eq!(
                with_nanos
                    .local_datetime()
                    .expect("system zone available")
                    .subsec_nanosecond(),
                123_000_000
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
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());
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
            assert_ne!(rendered, edge.utc().to_string());
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

        #[rstest]
        #[case::milliseconds("123", ".123")]
        #[case::trailing_zero_milliseconds("500", ".500")]
        #[case::trailing_zero_microseconds("250000", ".250")]
        fn serializes_sub_second_precision_with_autosi(
            #[case] fraction: &str,
            #[case] expected: &str,
        ) {
            // Arrange
            let value = DateTimeValue::parse_iso(&format!(
                "2026-07-29T14:30:00.{fraction}Z"
            ))
            .expect("explicit-offset fixture parses");

            // Act
            let json = serde_json::to_string(&value);

            // Assert
            assert_eq!(
                json.ok().as_deref(),
                Some(format!("\"2026-07-29T14:30:00{expected}Z\"").as_str()),
                "AutoSi precision must retain the canonical fraction width"
            );
        }

        #[rstest]
        #[case::whole_seconds(0)]
        #[case::sub_second(123)]
        fn round_trips_through_json(#[case] milliseconds: i64) {
            // Arrange: the case adds whole seconds or 123 milliseconds.
            let value = DateTimeValue::from_timestamp(
                fixed_datetime()
                    .utc()
                    .checked_add(
                        Span::new()
                            .try_milliseconds(milliseconds)
                            .expect("valid millisecond span"),
                    )
                    .expect("the fixture plus milliseconds is in range"),
            );
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
        fn rejects_non_string_json_scalars() {
            // Arrange: date-time deserialization accepts strings only.
            let inputs = ["42", "true", "[]"];
            // Act
            for input in inputs {
                let result = serde_json::from_str::<DateTimeValue>(input);
                // Assert
                assert!(
                    result.is_err(),
                    "non-string scalar {input} must be rejected"
                );
            }
        }

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
            let expected =
                DateTimeValue::parse_iso("2026-07-29T14:30:00+00:00")
                    .expect("valid datetime");

            // Act
            let result = noyalib::from_str::<YamlNote>(yaml).map(|note| {
                let YamlNote {
                    datetime,
                } = note;
                let serialized = serde_json::to_string(&datetime);
                (datetime, serialized)
            });

            // Assert
            assert!(
                matches!(
                    result,
                    Ok((datetime, Ok(serialized)))
                        if datetime == expected && serialized == "\"2026-07-29T14:30:00Z\""
                ),
                "YAML Z values remain UTC when deserialized and serialized"
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

        use super::*;

        #[test]
        fn promotes_a_date_to_local_zone_midnight() {
            // Arrange: UTC+2, no DST.
            ZoneGuard::set("Etc/GMT-2");
            let date = ymd(2026, 7, 29);
            let expected = DateTimeValue::parse_iso("2026-07-28T22:00:00Z")
                .expect("valid expected date-time");

            // Act
            let promoted = DateTimeValue::try_from(date);

            // Assert
            assert!(
                matches!(promoted, Ok(value) if value == expected),
                "local midnight in UTC+2 is 22:00Z on the previous day"
            );
        }

        #[test]
        fn advances_a_promoted_midnight_across_a_zone_gap() {
            // Arrange: local midnight falls inside the fixture's spring-forward
            // gap.
            ZoneGuard::set_timezone(
                crate::temporal::fixtures::midnight_gap_zone(),
            );
            let date = ymd(2026, 3, 8);
            let expected = DateTimeValue::parse_iso("2026-03-08T00:00:00Z")
                .expect("valid expected date-time");

            // Act
            let promoted = DateTimeValue::try_from(date);

            // Assert
            assert!(
                matches!(
                    promoted,
                    Ok(value)
                        if value == expected
                            && value.to_string() == "2026-03-08T01:00:00"
                ),
                "midnight inside a DST gap advances by the gap length"
            );
        }

        #[test]
        fn rejects_a_date_midnight_outside_the_timestamp_range() {
            // Arrange
            ZoneGuard::set("Etc/GMT-14");
            let date = DateValue::from_civil_date(civil::Date::MIN);

            // Act
            let error = DateTimeValue::try_from(date).expect_err(
                "the minimum civil date is outside the timestamp range",
            );

            // Assert
            assert!(matches!(
                error,
                DateError::InvalidValue { input, reason }
                    if input.as_ref() == "-009999-01-01T00:00:00"
                        && reason == "date/time is outside the representable range"
            ));
        }

        #[test]
        fn converts_a_system_time() {
            // Arrange: one thousand seconds past the epoch, UTC.
            ZoneGuard::set("UTC");
            let system_time =
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000);
            let expected = DateTimeValue::parse_iso("1970-01-01T00:16:40")
                .expect("valid expected date-time");

            // Act
            let converted = DateTimeValue::try_from(system_time);

            // Assert
            assert!(
                matches!(converted, Ok(value) if value == expected),
                "a SystemTime position must convert to the same date-time"
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
            let shifted = base.shift(
                1,
                TimeUnit::Hour,
                &local_zone().expect("system time zone available"),
            );

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
            ZoneGuard::set_timezone(crate::temporal::fixtures::new_york_zone());
            let second_occurrence =
                DateTimeValue::parse_iso("2026-11-01T06:30:00Z")
                    .expect("explicit-offset fixture parses");

            // Act
            // Calendar shifts re-resolve the local clock time.
            let shifted = second_occurrence.shift(
                0,
                TimeUnit::Day,
                &local_zone().expect("system time zone available"),
            );

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
            let shifted = base.shift(
                0,
                TimeUnit::Day,
                &local_zone().expect("system time zone available"),
            );

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
            let midnight = DateTimeValue::try_from(date)
                .expect("test date has a representable local midnight");
            let time_of_day = fixed_datetime();

            // Act
            let is_midnight = midnight.is_midnight_of(date);
            let has_time = time_of_day.is_midnight_of(date);

            // Assert
            assert!(
                is_midnight,
                "the promoted midnight must count as midnight of its date"
            );
            assert!(
                !has_time,
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
            let extracted =
                fixed_datetime().local_date().expect("system zone available");

            // Assert
            assert_eq!(
                extracted,
                ymd(2026, 7, 29),
                "the UTC calendar date must be extracted"
            );
        }

        #[test]
        fn extracts_the_local_calendar_date_of_a_date_time() {
            // Arrange: UTC+12 rolls 14:30Z into the next local day.
            ZoneGuard::set("Etc/GMT-12");

            // Act
            let extracted =
                fixed_datetime().local_date().expect("system zone available");

            // Assert
            assert_eq!(
                extracted,
                ymd(2026, 7, 30),
                "the local calendar date must roll forward with the offset"
            );
        }
    }

    mod proptests {
        use proptest::prelude::*;

        use super::*;

        proptest! {
            #[test]
            fn local_iso_display_round_trips_at_second_precision(
                seconds in 946_684_800_u64..=1_893_456_000_u64,
            ) {
                // Arrange: pin the process-local zone for a deterministic round trip.
                ZoneGuard::set("UTC");
                let system_time = SystemTime::UNIX_EPOCH
                    .checked_add(std::time::Duration::from_secs(seconds))
                    .expect("generated timestamp fits SystemTime");
                let value = DateTimeValue::try_from(system_time)
                    .expect("generated timestamp fits DateTimeValue");

                // Act
                let displayed = value.to_string();
                let parsed = DateTimeValue::parse_iso(&displayed);

                // Assert
                prop_assert_eq!(parsed, Ok(value));
            }
        }
    }
}

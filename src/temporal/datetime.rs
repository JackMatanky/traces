use std::{borrow::Cow, fmt, str::FromStr, time::SystemTime};

use chrono::{
    DateTime, FixedOffset, Local, MappedLocalTime, NaiveDateTime, NaiveTime,
    Offset as _, SecondsFormat, TimeDelta, TimeZone as _, Timelike as _, Utc,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{DateValue, error::DateError};

/// [`DateTimeValue`]'s canonical output format: `2026-07-29T14:30:00`.
pub(crate) const DATETIME_PATTERN: &str = "%Y-%m-%dT%H:%M:%S";

/// Parsed UTC date-time instant.
///
/// Wraps [`DateTime<Utc>`] as a newtype, enforcing ISO-8601/RFC-3339
/// recognition through its crate-internal `parse_iso` parser. All values are
/// UTC-normalized; offset-bearing input is converted to UTC at parse time,
/// while naive input resolves in the process's local zone.
///
/// # Examples
///
/// ```rust
/// use traces_pkm::DateTimeValue;
///
/// let utc: DateTimeValue =
///     "2026-07-29T14:30:00Z".parse().expect("valid RFC 3339");
/// let offset: DateTimeValue =
///     "2026-07-29T14:30:00+00:00".parse().expect("equivalent offset");
/// assert_eq!(utc, offset);
///
/// // Outside a DST fold hour, the local clock time this value renders
/// // re-parses to the same instant.
/// let round_trip: DateTimeValue =
///     utc.to_string().parse().expect("rendered spelling re-parses");
/// assert_eq!(utc, round_trip);
/// ```
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DateTimeValue(DateTime<Utc>);

impl DateTimeValue {
    /// Returns the local clock-time rendering of this instant.
    ///
    /// Every human-facing rendering goes through this conversion: the offset
    /// comes from [`Local::offset_from_utc_datetime`] and is applied with
    /// [`NaiveDateTime::checked_add_offset`], which returns [`None`] instead of
    /// panicking at `NaiveDateTime`'s range edge (`.naive_local()` would panic
    /// there).
    #[inline]
    #[must_use]
    pub(crate) fn local_datetime(self) -> Option<NaiveDateTime> {
        let naive_utc = self.0.naive_utc();
        let offset = Local.offset_from_utc_datetime(&naive_utc);
        naive_utc.checked_add_offset(offset.fix())
    }

    /// Returns the local clock time, falling back to the UTC clock time when
    /// the local offset cannot be applied (see [`Self::local_datetime`]).
    #[inline]
    #[must_use]
    pub(crate) fn local_datetime_or_utc(self) -> NaiveDateTime {
        self.local_datetime().unwrap_or_else(|| self.0.naive_utc())
    }

    /// Returns the local calendar date of this instant, discarding time-of-day.
    ///
    /// When the local offset cannot be applied (an instant at the extreme edge
    /// of [`NaiveDateTime`]'s range), the UTC date renders instead.
    #[inline]
    #[must_use]
    pub(crate) fn local_date(self) -> DateValue {
        DateValue::from(self.local_datetime_or_utc().date())
    }

    /// Returns `true` if this date-time is exactly local-zone midnight on
    /// `date` (see the [`From<DateValue>`] promotion).
    #[must_use]
    pub(crate) fn is_midnight_of(self, date: DateValue) -> bool {
        self.0 == Self::from(date).0
    }

    /// Returns the wrapped [`DateTime<Utc>`].
    #[must_use]
    pub(crate) const fn utc(self) -> DateTime<Utc> {
        self.0
    }

    /// Canonical strftime pattern for this instant: whole seconds, plus
    /// fractional seconds only when the nanosecond component is nonzero.
    ///
    /// Local offsets are whole seconds, so the UTC and local readings share
    /// their nanosecond component and no zone lookup is needed.
    #[must_use]
    pub(crate) fn canonical_pattern(self) -> &'static str {
        if self.0.nanosecond() == 0 {
            DATETIME_PATTERN
        } else {
            "%Y-%m-%dT%H:%M:%S%.f"
        }
    }

    /// Resolves a naive local datetime to an instant under the crate's DST
    /// doctrine.
    ///
    /// A clock time written without a zone is read in the process's local zone
    /// and stored as UTC. Resolution never fails on a DST boundary:
    ///
    /// - Ambiguous fall-back times resolve to their earliest occurrence;
    /// - Spring-forward gaps shift forward by the gap, matching Temporal's
    ///   `'compatible'` disambiguation and jiff's
    ///   [`Disambiguation::Compatible`].
    ///
    /// Only a broken tz-data/OS lookup or an offset application outside
    /// `NaiveDateTime`'s range errors. On the `WebAssembly` target, chrono's
    /// `Local` reports every local time as unambiguous ([chrono#1701]), so the
    /// ambiguous arm is unreachable there.
    ///
    /// # Errors
    ///
    /// - [`DateError::LocalZoneLookup`] if the lookup finds no offset for
    ///   `local_datetime` or a probe (a broken tz-data/OS environment rather
    ///   than a DST boundary), or if applying the resolved offset would leave
    ///   `NaiveDateTime`'s range.
    ///
    /// [`Disambiguation::Compatible`]:
    ///     https://docs.rs/jiff/latest/jiff/tz/enum.Disambiguation.html
    /// [chrono#1701]: https://github.com/chronotope/chrono/issues/1701
    pub(super) fn from_local(
        local_datetime: NaiveDateTime,
    ) -> Result<Self, DateError> {
        let zone_lookup = || DateError::LocalZoneLookup {
            input: local_datetime.to_string().into(),
        };
        // The checked forms return `None` instead of panicking when the instant
        // leaves `NaiveDateTime`'s range (impossible for the parsers'
        // 4-digit-year inputs).
        match Local.offset_from_local_datetime(&local_datetime) {
            MappedLocalTime::Single(offset) => {
                let instant = local_datetime
                    .checked_sub_offset(offset)
                    .ok_or_else(zone_lookup)?;
                Ok(Self(instant.and_utc()))
            }
            MappedLocalTime::Ambiguous(a, b) => {
                // `chrono` orders an ambiguous pair by offset (tzfile data) or
                // transition side (POSIX rules); the earliest occurrence has
                // the larger offset, since instant = local_datetime - offset.
                let earliest = if a.local_minus_utc() >= b.local_minus_utc() {
                    a
                } else {
                    b
                };
                let instant = local_datetime
                    .checked_sub_offset(earliest)
                    .ok_or_else(zone_lookup)?;
                Ok(Self(instant.and_utc()))
            }
            MappedLocalTime::None => {
                // A gap resolves `None` across the whole skipped span, so no
                // probe inside it resolves; the pre-transition offset shifts
                // the gap time forward by exactly the gap.
                let offset =
                    Self::offset_across_gap(local_datetime, &zone_lookup)?;
                let instant = local_datetime
                    .checked_sub_offset(offset)
                    .ok_or_else(zone_lookup)?;
                Ok(Self(instant.and_utc()))
            }
        }
    }

    /// Finds the offset in effect just before a DST gap by stepping back one
    /// hour at a time from `local_datetime` (the widest recorded gap is 24
    /// hours).
    ///
    /// # Errors
    ///
    /// - [`DateError::LocalZoneLookup`] if no nearby local time resolves within
    ///   the 25-hour backward search (a broken tz-data/OS lookup rather than a
    ///   gap), or if stepping the probe back would leave `NaiveDateTime`'s
    ///   range.
    fn offset_across_gap(
        local_datetime: NaiveDateTime,
        zone_lookup: &impl Fn() -> DateError,
    ) -> Result<FixedOffset, DateError> {
        let hour = TimeDelta::try_hours(1).ok_or_else(zone_lookup)?;
        let mut probe = local_datetime;
        for _ in 0..25 {
            probe = probe.checked_sub_signed(hour).ok_or_else(zone_lookup)?;
            if let MappedLocalTime::Single(offset)
            | MappedLocalTime::Ambiguous(offset, _) =
                Local.offset_from_local_datetime(&probe)
            {
                return Ok(offset);
            }
        }
        Err(zone_lookup())
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

impl From<SystemTime> for DateTimeValue {
    #[inline]
    fn from(time: SystemTime) -> Self {
        Self(DateTime::<Utc>::from(time))
    }
}

impl From<DateTime<Utc>> for DateTimeValue {
    #[inline]
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}

impl From<DateTimeValue> for DateTime<Utc> {
    #[inline]
    fn from(value: DateTimeValue) -> Self {
        value.utc()
    }
}

/// Promotes a [`DateValue`] to a [`DateTimeValue`] at midnight in the local
/// zone: a zone-free calendar date has no instant until a reader's zone
/// supplies one.
///
/// On zone-lookup failure, this promotion degrades to the neutral UTC frame
/// (calendar date read as UTC, deterministic and shift-free) instead of
/// surfacing [`DateError::LocalZoneLookup`], because the infallible conversion
/// is consumed by equality, sort-key, and ordering paths that cannot propagate
/// an error, while `TryFrom` would duplicate that fallback at each of them. All
/// fallible entry points surface broken local-zone lookups as
/// [`DateError::LocalZoneLookup`] instead.
impl From<DateValue> for DateTimeValue {
    #[inline]
    fn from(date: DateValue) -> Self {
        let midnight = date.naive_date().and_time(NaiveTime::MIN);
        Self::from_local(midnight)
            .unwrap_or_else(|_zone_failure| Self(midnight.and_utc()))
    }
}

impl FromStr for DateTimeValue {
    type Err = DateError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_iso(s)
    }
}

/// Serializes as an explicit RFC 3339 `…Z` interop spelling via
/// [`DateTime::to_rfc3339_opts`], independent of [`Display`](fmt::Display)'s
/// local-naive rendering.
impl Serialize for DateTimeValue {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer
            .serialize_str(&self.0.to_rfc3339_opts(SecondsFormat::AutoSi, true))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{temporal::fixtures::fixed_datetime, test_support::TzGuard};

    /// Local-zone doctrine: a naive datetime means the reader's local clock
    /// time, resolved to UTC through the DST resolver; a date-only value stays
    /// zone-free until a zone is needed. Every test here injects `TZ` via
    /// [`TzGuard`] so the asserted zone is deterministic.
    mod local_zone {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn parses_a_naive_datetime_in_the_local_zone_and_stores_utc() {
            TzGuard::set("Etc/GMT-2"); // UTC+02:00, no DST

            let parsed = DateTimeValue::parse_iso("2026-07-29 14:30:00")
                .expect("naive datetime parses");

            assert_eq!(
                parsed,
                DateTimeValue::parse_iso("2026-07-29T12:30:00Z")
                    .expect("equivalent explicit-offset instant parses")
            );
        }

        #[test]
        fn resolves_an_ambiguous_fall_back_time_to_the_earliest_occurrence() {
            TzGuard::set("America/New_York");

            let parsed = DateTimeValue::parse_iso("2026-11-01 01:30:00")
                .expect("ambiguous clock time still parses");

            // 01:30 happens twice (EDT, then EST); the earliest occurrence is
            // the EDT reading, 05:30Z.
            assert_eq!(
                parsed,
                DateTimeValue::parse_iso("2026-11-01T05:30:00Z")
                    .expect("equivalent explicit-offset instant parses")
            );
        }

        #[test]
        fn shifts_a_spring_forward_gap_forward_by_the_gap() {
            TzGuard::set("America/New_York");

            let parsed = DateTimeValue::parse_iso("2026-03-08 02:30:00")
                .expect("gap clock time still parses, never fails");

            // 02:30 does not exist; interpreting it with the pre-transition
            // offset shifts it forward by the one-hour gap to 03:30 EDT.
            assert_eq!(
                parsed,
                DateTimeValue::parse_iso("2026-03-08T07:30:00Z")
                    .expect("equivalent explicit-offset instant parses")
            );
        }

        #[test]
        fn resolves_local_times_adjacent_to_a_gap_without_error() {
            TzGuard::set("America/New_York");

            let before = DateTimeValue::parse_iso("2026-03-08 01:59:59")
                .expect("clock time one second before the gap parses");
            let after = DateTimeValue::parse_iso("2026-03-08 03:00:00")
                .expect("clock time at the gap's end parses");

            // A real gap resolves `None` only inside the gap; the adjacent
            // local times must resolve normally, not be swept into a silent
            // shift or an error.
            assert_eq!(
                before,
                DateTimeValue::parse_iso("2026-03-08T06:59:59Z")
                    .expect("equivalent explicit-offset instant parses")
            );
            assert_eq!(
                after,
                DateTimeValue::parse_iso("2026-03-08T07:00:00Z")
                    .expect("equivalent explicit-offset instant parses")
            );
        }

        #[test]
        fn reports_local_zone_lookup_when_extreme_local_datetime_cannot_resolve()
         {
            // Arrange
            TzGuard::set("Etc/GMT-14");

            // Act
            let result = DateTimeValue::from_local(NaiveDateTime::MIN);

            // Assert
            assert!(matches!(result, Err(DateError::LocalZoneLookup { .. })));
        }

        #[test]
        fn promotes_a_date_value_to_local_zone_midnight() {
            TzGuard::set("Etc/GMT-2"); // UTC+02:00, no DST

            let date = DateValue::parse_iso("2026-07-29").expect("valid date");

            let promoted = DateTimeValue::from(date);

            // Calendar midnight in UTC+2 is 22:00Z on the previous day, so this
            // pins the promotion to the local zone rather than UTC.
            assert_eq!(
                promoted.utc(),
                Utc.with_ymd_and_hms(2026, 7, 28, 22, 0, 0)
                    .single()
                    .expect("unambiguous instant")
            );
        }
    }

    mod formatting {
        use pretty_assertions::{assert_eq, assert_ne};

        use super::*;

        #[test]
        fn fold_hour_display_re_parses_to_the_earlier_instant() {
            // Arrange
            TzGuard::set("America/New_York");
            let later =
                DateTimeValue::parse_iso("2026-11-01T06:30:00Z").unwrap();

            // Act
            let local_datetime = later.to_string();
            let reparsed = DateTimeValue::parse_iso(&local_datetime).unwrap();

            // Assert
            assert_eq!(local_datetime, "2026-11-01T01:30:00");
            assert_eq!(
                reparsed,
                DateTimeValue::parse_iso("2026-11-01T05:30:00Z").unwrap()
            );
            assert_ne!(reparsed, later);
        }

        #[test]
        fn renders_to_display_without_the_offset() {
            TzGuard::set("UTC");

            assert_eq!(fixed_datetime().to_string(), "2026-07-29T14:30:05");
        }

        #[test]
        fn round_trips_fractional_seconds_in_display() {
            TzGuard::set("UTC");

            let with_nanos = DateTimeValue(
                fixed_datetime().utc() + chrono::TimeDelta::milliseconds(123),
            );
            assert_eq!(with_nanos.to_string(), "2026-07-29T14:30:05.123");
        }

        #[test]
        fn extracts_the_calendar_date_discarding_time_of_day() {
            TzGuard::set("UTC");

            let extracted = fixed_datetime().local_date();
            let expected =
                DateValue::parse_iso("2026-07-29").expect("valid date");
            assert_eq!(extracted, expected);
        }

        #[test]
        fn extracts_the_local_calendar_date_of_the_instant() {
            TzGuard::set("Etc/GMT-12"); // UTC+12: 14:30Z is already July 30

            let extracted = fixed_datetime().local_date();
            let expected =
                DateValue::parse_iso("2026-07-30").expect("valid date");
            assert_eq!(extracted, expected);
        }

        #[test]
        fn round_trips_a_datetime_value_through_display_and_parse_iso() {
            TzGuard::keep();

            let value = fixed_datetime();
            let reparsed = DateTimeValue::parse_iso(&value.to_string())
                .expect("formatted output re-parses");
            assert_eq!(reparsed, value);
        }
    }

    mod comparison {
        use super::*;

        #[test]
        fn requires_exact_midnight_in_is_midnight_of() {
            TzGuard::keep();

            let midnight = DateTimeValue::from(
                DateValue::parse_iso("2026-07-29").expect("valid date"),
            );
            assert!(midnight.is_midnight_of(
                DateValue::parse_iso("2026-07-29").expect("valid date")
            ));
            assert!(!fixed_datetime().is_midnight_of(
                DateValue::parse_iso("2026-07-29").expect("valid date")
            ));
        }
    }

    mod conversions {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn converts_from_system_time() {
            TzGuard::set("UTC");

            let system_time = std::time::SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(1_000);
            let converted = DateTimeValue::from(system_time);
            assert_eq!(
                converted,
                DateTimeValue::parse_iso("1970-01-01T00:16:40")
                    .expect("valid datetime")
            );
        }

        #[test]
        fn round_trips_through_chrono_date_time_utc() {
            let chrono_dt = fixed_datetime().utc();
            let converted = DateTimeValue::from(chrono_dt);
            assert_eq!(converted, fixed_datetime());
            let back: DateTime<Utc> = converted.into();
            assert_eq!(back, chrono_dt);
        }
    }

    mod ordering {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn compares_datetimes_chronologically() {
            TzGuard::keep();

            let earlier = DateTimeValue::parse_iso("2026-01-01T00:00:00")
                .expect("valid datetime");
            let later = fixed_datetime();
            assert!(earlier < later);
            assert!(later > earlier);
            assert_eq!(earlier.cmp(&earlier), std::cmp::Ordering::Equal);
        }
    }

    mod display_and_traits {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn displays_a_datetime_value_as_its_canonical_string() {
            TzGuard::set("UTC");

            assert_eq!(fixed_datetime().to_string(), "2026-07-29T14:30:05");
        }

        #[test]
        fn displays_the_local_clock_time_of_the_instant() {
            TzGuard::set("Etc/GMT-2"); // UTC+02:00, no DST

            // The everyday human spelling is the local clock time, so a
            // 14:30:05Z instant reads as 16:30:05 local.
            assert_eq!(fixed_datetime().to_string(), "2026-07-29T16:30:05");
        }

        #[test]
        fn round_trips_a_datetime_value_through_json() {
            let json =
                serde_json::to_string(&fixed_datetime()).expect("serializable");
            let restored: DateTimeValue =
                serde_json::from_str(&json).expect("deserializable");
            assert_eq!(restored, fixed_datetime());
        }

        #[test]
        fn serializes_datetime_value_with_the_explicit_rfc3339_z_suffix() {
            let json =
                serde_json::to_string(&fixed_datetime()).expect("serializable");
            assert_eq!(json, "\"2026-07-29T14:30:05Z\"");
        }

        #[test]
        fn round_trips_sub_second_datetime_through_json_with_autosi() {
            let dt = DateTimeValue::parse_iso("2026-07-29T14:30:00.123Z")
                .expect("valid instant");
            let json = serde_json::to_string(&dt).expect("serializable");
            assert_eq!(json, "\"2026-07-29T14:30:00.123Z\"");
            let restored: DateTimeValue =
                serde_json::from_str(&json).expect("deserializable");
            assert_eq!(restored, dt);
        }
    }
}

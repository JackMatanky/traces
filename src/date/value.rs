use std::{borrow::Cow, fmt, str::FromStr, time::SystemTime};

use chrono::{
    DateTime, FixedOffset, Local, MappedLocalTime, NaiveDate, NaiveDateTime,
    NaiveTime, Offset as _, SecondsFormat, TimeDelta, TimeZone as _, Utc,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::error::DateError;

/// [`DateValue`]'s canonical output format: `2026-07-29`.
pub(crate) const DATE_PATTERN: &str = "%Y-%m-%d";

/// [`DateTimeValue`]'s canonical output format: `2026-07-29T14:30:00`.
pub(crate) const DATETIME_PATTERN: &str = "%Y-%m-%dT%H:%M:%S";

/// Calendar precision of a zone-free point.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum CivilPrecision {
    /// Year and month.
    Month,
    /// Full calendar date.
    Day,
}

/// A parsed date or date-time at the precision it was written.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum Point {
    /// Zone-free calendar date.
    Civil {
        date: DateValue,
        precision: CivilPrecision,
    },
    /// UTC instant.
    Instant(DateTimeValue),
}

impl Point {
    /// Whether this point carries a time-of-day component.
    #[must_use]
    pub(crate) const fn has_time(self) -> bool {
        matches!(self, Self::Instant(_))
    }

    /// Calendar date of this point in the local zone.
    #[must_use]
    pub(crate) fn date(self) -> DateValue {
        match self {
            Self::Civil {
                date,
                ..
            } => date,
            Self::Instant(dt) => dt.local_date(),
        }
    }

    /// Civil wall clock, or local clock for an instant.
    #[must_use]
    pub(crate) fn wall(self) -> NaiveDateTime {
        match self {
            Self::Civil {
                date,
                ..
            } => date.naive_date().and_time(NaiveTime::MIN),
            Self::Instant(dt) => dt.local_wall_or_utc(),
        }
    }

    /// UTC instant represented by this point.
    #[must_use]
    pub(crate) fn instant(self) -> DateTime<Utc> {
        match self {
            Self::Civil {
                date,
                ..
            } => DateTimeValue::from(date).utc(),
            Self::Instant(dt) => dt.utc(),
        }
    }

    /// Canonical strftime pattern preserving written precision.
    #[must_use]
    pub(crate) fn canonical_pattern(self) -> &'static str {
        use chrono::Timelike as _;
        match self {
            Self::Civil {
                precision: CivilPrecision::Month,
                ..
            } => "%Y-%m",
            Self::Civil {
                precision: CivilPrecision::Day,
                ..
            } => DATE_PATTERN,
            Self::Instant(_) if self.wall().nanosecond() == 0 => {
                DATETIME_PATTERN
            }
            Self::Instant(_) => "%Y-%m-%dT%H:%M:%S%.f",
        }
    }
}

/// A calendar date without timezone or time-of-day ([`NaiveDate`]).
///
/// Wraps a [`NaiveDate`] representing a civil calendar date (`YYYY-MM-DD` or
/// `YYYY-MM`). Parsed input keeps precision in `Point`; deserialization uses
/// the same strict date parser.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DateValue(pub(super) NaiveDate);

impl DateValue {
    /// Returns the wrapped [`NaiveDate`].
    #[must_use]
    pub(crate) const fn naive_date(self) -> NaiveDate {
        self.0
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
/// // Outside a DST fold hour, the local wall clock this value renders re-parses to the same instant.
/// let round_trip: DateTimeValue =
///     utc.to_string().parse().expect("rendered spelling re-parses");
/// assert_eq!(utc, round_trip);
/// ```
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DateTimeValue(pub(super) DateTime<Utc>);

impl DateTimeValue {
    /// Returns the local wall-clock rendering of this instant.
    ///
    /// Every human-facing rendering goes through this conversion: the offset
    /// comes from [`Local::offset_from_utc_datetime`] and is applied with
    /// [`NaiveDateTime::checked_add_offset`], which returns [`None`] instead of
    /// panicking at `NaiveDateTime`'s range edge (`.naive_local()` would panic
    /// there).
    #[inline]
    #[must_use]
    pub(crate) fn local_wall(self) -> Option<NaiveDateTime> {
        let naive_utc = self.0.naive_utc();
        let offset = Local.offset_from_utc_datetime(&naive_utc);
        naive_utc.checked_add_offset(offset.fix())
    }

    /// Returns the local wall clock, falling back to the UTC wall clock when
    /// the local offset cannot be applied (see [`Self::local_wall`]).
    #[inline]
    #[must_use]
    pub(crate) fn local_wall_or_utc(self) -> NaiveDateTime {
        self.local_wall().unwrap_or_else(|| self.0.naive_utc())
    }

    /// Returns the local calendar date of this instant, discarding time-of-day.
    ///
    /// When the local offset cannot be applied (an instant at the extreme edge
    /// of [`NaiveDateTime`]'s range), the UTC date renders instead.
    #[inline]
    #[must_use]
    pub(crate) fn local_date(self) -> DateValue {
        DateValue(self.local_wall_or_utc().date())
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
}

/// Renders the local wall clock without a UTC offset, so a date or time
/// component reads as the reader's own clock.
///
/// Fractional seconds appear only for a nonzero nanosecond component, so
/// `DateTimeShape::LocalTSeconds` input keeps its full sub-second precision.
/// When the local offset cannot be applied, the UTC wall clock renders instead
/// of panicking (see `DateTimeValue::local_wall`).
impl fmt::Display for DateTimeValue {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use chrono::Timelike as _;
        let wall = self.local_wall_or_utc();
        let format = if wall.nanosecond() == 0 {
            DATETIME_PATTERN
        } else {
            "%Y-%m-%dT%H:%M:%S%.f"
        };
        write!(f, "{}", wall.format(format))
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
/// zone: a zone-free civil date has no instant until a reader's zone supplies
/// one.
///
/// On zone-lookup failure, this promotion degrades to the neutral UTC frame
/// (civil date read as UTC, deterministic and shift-free) instead of surfacing
/// [`DateError::LocalZoneLookup`], because the infallible conversion is
/// consumed by equality, sort-key, and ordering paths that cannot propagate an
/// error, while `TryFrom` would duplicate that fallback at each of them. All
/// fallible entry points surface broken local-zone lookups as
/// [`DateError::LocalZoneLookup`] instead.
impl From<DateValue> for DateTimeValue {
    #[inline]
    fn from(date: DateValue) -> Self {
        let midnight = date.0.and_time(NaiveTime::MIN);
        Self(
            resolve_local(midnight)
                .unwrap_or_else(|_zone_failure| midnight.and_utc()),
        )
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

/// Resolves a naive local wall-clock datetime to a UTC instant under the
/// crate's DST doctrine.
///
/// A wall-clock time written without a zone is read in the process's local zone
/// and stored as UTC. Resolution never fails on a DST boundary:
///
/// - Ambiguous fall-back times resolve to their earliest occurrence;
/// - Spring-forward gaps shift forward by the gap, matching Temporal's
///   `'compatible'` disambiguation and jiff's [`Disambiguation::Compatible`].
///
/// Only a broken tz-data/OS lookup or an offset application outside
/// `NaiveDateTime`'s range errors. On the `WebAssembly` target, chrono's
/// `Local` reports every local time as unambiguous ([chrono#1701]), so the
/// ambiguous arm is unreachable there.
///
/// # Errors
///
/// - [`DateError::LocalZoneLookup`] if the lookup finds no offset for `wall` or
///   a probe (a broken tz-data/OS environment rather than a DST boundary), or
///   if applying the resolved offset would leave `NaiveDateTime`'s range.
///
/// [`Disambiguation::Compatible`]:
///     https://docs.rs/jiff/latest/jiff/tz/enum.Disambiguation.html
/// [chrono#1701]: https://github.com/chronotope/chrono/issues/1701
pub(super) fn resolve_local(
    wall: NaiveDateTime,
) -> Result<DateTime<Utc>, DateError> {
    let zone_lookup = || DateError::LocalZoneLookup {
        input: wall.to_string().into(),
    };
    // The checked forms return `None` instead of panicking when the instant
    // leaves `NaiveDateTime`'s range (impossible for the parsers' 4-digit-year
    // inputs).
    match Local.offset_from_local_datetime(&wall) {
        MappedLocalTime::Single(offset) => {
            let instant =
                wall.checked_sub_offset(offset).ok_or_else(zone_lookup)?;
            Ok(instant.and_utc())
        }
        MappedLocalTime::Ambiguous(a, b) => {
            // `chrono` orders an ambiguous pair by offset (tzfile data) or
            // transition side (POSIX rules); the earliest occurrence has the
            // larger offset, since instant = wall - offset.
            let earliest = if a.local_minus_utc() >= b.local_minus_utc() {
                a
            } else {
                b
            };
            let instant =
                wall.checked_sub_offset(earliest).ok_or_else(zone_lookup)?;
            Ok(instant.and_utc())
        }
        MappedLocalTime::None => {
            // A gap resolves `None` across the whole skipped span, so no probe
            // inside it resolves; the pre-transition offset shifts the gap time
            // forward by exactly the gap.
            let offset = offset_across_gap(wall, &zone_lookup)?;
            let instant =
                wall.checked_sub_offset(offset).ok_or_else(zone_lookup)?;
            Ok(instant.and_utc())
        }
    }
}

/// Finds the offset in effect just before a DST gap by stepping back one hour
/// at a time from `wall` (the widest recorded gap is 24 hours).
///
/// # Errors
///
/// - [`DateError::LocalZoneLookup`] if no nearby local time resolves within the
///   25-hour backward search (a broken tz-data/OS lookup rather than a gap), or
///   if stepping the probe back would leave `NaiveDateTime`'s range.
fn offset_across_gap(
    wall: NaiveDateTime,
    zone_lookup: &impl Fn() -> DateError,
) -> Result<FixedOffset, DateError> {
    let hour = TimeDelta::try_hours(1).ok_or_else(zone_lookup)?;
    let mut probe = wall;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        date::fixtures::fixed_datetime, duration::DurationValue,
        test_support::TzGuard,
    };

    /// Local-zone doctrine: a naive datetime means the reader's local wall
    /// clock, resolved to UTC through the DST resolver; a date-only value
    /// stays zone-free until a zone is needed. Every test here injects `TZ`
    /// via [`TzGuard`] so the asserted zone is deterministic.
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
                .expect("ambiguous wall clock still parses");

            // 01:30 happens twice (EDT, then EST); the earliest occurrence
            // is the EDT reading, 05:30Z.
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
                .expect("gap wall clock still parses, never fails");

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
                .expect("wall clock one second before the gap parses");
            let after = DateTimeValue::parse_iso("2026-03-08 03:00:00")
                .expect("wall clock at the gap's end parses");

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
        fn reports_local_zone_lookup_when_extreme_wall_clock_cannot_resolve() {
            // Arrange
            TzGuard::set("Etc/GMT-14");

            // Act
            let result = resolve_local(NaiveDateTime::MIN);

            // Assert
            assert!(matches!(result, Err(DateError::LocalZoneLookup { .. })));
        }

        #[test]
        fn promotes_a_date_value_to_local_zone_midnight() {
            TzGuard::set("Etc/GMT-2"); // UTC+02:00, no DST

            let date = DateValue::parse_iso("2026-07-29").expect("valid date");

            let promoted = DateTimeValue::from(date);

            // Civil midnight in UTC+2 is 22:00Z on the previous day, so this
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
            let wall = later.to_string();
            let reparsed = DateTimeValue::parse_iso(&wall).unwrap();

            // Assert
            assert_eq!(wall, "2026-11-01T01:30:00");
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
        fn round_trips_a_date_value_through_display_and_parse_iso() {
            let date = DateValue::parse_iso("2026-07-29").expect("valid date");
            let reparsed = DateValue::parse_iso(&date.to_string())
                .expect("formatted output re-parses");
            assert_eq!(reparsed, date);
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

    mod point_api {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn has_time_only_for_instants() {
            TzGuard::set("UTC");
            assert!(!Point::parse("2026-07").unwrap().has_time());
            assert!(!Point::parse("2026-07-29").unwrap().has_time());
            assert!(Point::parse("2026-07-29T14:30:00Z").unwrap().has_time());
        }

        #[test]
        fn canonical_pattern_preserves_civil_precision_and_fractional_seconds()
        {
            TzGuard::set("UTC");
            assert_eq!(
                Point::parse("2026-07").unwrap().canonical_pattern(),
                "%Y-%m"
            );
            assert_eq!(
                Point::parse("2026-07-29").unwrap().canonical_pattern(),
                "%Y-%m-%d"
            );
            assert_eq!(
                Point::parse("2026-07-29T14:30:00Z")
                    .unwrap()
                    .canonical_pattern(),
                "%Y-%m-%dT%H:%M:%S"
            );
            assert_eq!(
                Point::parse("2026-07-29T14:30:00.123Z")
                    .unwrap()
                    .canonical_pattern(),
                "%Y-%m-%dT%H:%M:%S%.f"
            );
        }
    }

    mod golden_serde {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn date_and_datetime_keep_their_json_and_postcard_spellings() {
            // Arrange
            TzGuard::set("UTC");
            let date = DateValue::parse_iso("2026-07-29").unwrap();
            let datetime =
                DateTimeValue::parse_iso("2026-07-29T14:30:05Z").unwrap();

            // Act
            let date_bytes = postcard::to_allocvec(&date).unwrap();
            let datetime_bytes = postcard::to_allocvec(&datetime).unwrap();

            // Assert
            assert_eq!(serde_json::to_string(&date).unwrap(), "\"2026-07-29\"");
            assert_eq!(
                serde_json::to_string(&datetime).unwrap(),
                "\"2026-07-29T14:30:05Z\""
            );
            assert_eq!(date_bytes, b"\x0a2026-07-29");
            assert_eq!(datetime_bytes, b"\x142026-07-29T14:30:05Z");
            assert_eq!(
                postcard::from_bytes::<DateValue>(&date_bytes).unwrap(),
                date
            );
            assert_eq!(
                postcard::from_bytes::<DateTimeValue>(&datetime_bytes).unwrap(),
                datetime
            );
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
        fn round_trips_through_chrono_date_time_utc() {
            let chrono_dt = fixed_datetime().utc();
            let converted = DateTimeValue::from(chrono_dt);
            assert_eq!(converted, fixed_datetime());
            let back: DateTime<Utc> = converted.into();
            assert_eq!(back, chrono_dt);
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
        fn displays_a_date_value_as_its_canonical_string() {
            let date = DateValue::parse_iso("2026-07-29").expect("valid date");
            assert_eq!(date.to_string(), "2026-07-29");
        }

        #[test]
        fn displays_a_datetime_value_as_its_canonical_string() {
            TzGuard::set("UTC");

            assert_eq!(fixed_datetime().to_string(), "2026-07-29T14:30:05");
        }

        #[test]
        fn displays_the_local_wall_clock_of_the_instant() {
            TzGuard::set("Etc/GMT-2"); // UTC+02:00, no DST

            // The everyday human spelling is the local wall clock, so a
            // 14:30:05Z instant reads as 16:30:05 local.
            assert_eq!(fixed_datetime().to_string(), "2026-07-29T16:30:05");
        }

        #[test]
        fn displays_unparseable_error_naming_the_input_and_source() {
            let err = DateValue::parse_iso("2026/08/22")
                .expect_err("unrecognized shape");
            assert_eq!(
                err.to_string(),
                "`2026/08/22` is not a recognized date/time: input contains \
                 invalid characters"
            );
        }

        #[test]
        fn displays_invalid_year_digits_error_naming_the_input() {
            let err = DateValue::parse_iso("26-08-22")
                .expect_err("short year rejected");
            assert_eq!(
                err.to_string(),
                "`26-08-22` does not have a 4-digit year"
            );
        }

        #[test]
        fn exposes_the_chrono_source_for_an_unparseable_error() {
            use std::error::Error as _;
            let err = DateValue::parse_iso("2026/08/22")
                .expect_err("unrecognized shape");
            let source = err.source().expect("chrono parse error is chained");
            assert_eq!(source.to_string(), "input contains invalid characters");
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

    mod hostile_yaml_note {
        use pretty_assertions::{assert_eq, assert_ne};

        use super::*;

        #[derive(Deserialize)]
        struct HostileNote {
            duration_extreme: DurationValue,
            duration_zero: DurationValue,
            date: DateValue,
            datetime: DateTimeValue,
        }

        #[test]
        fn parses_round_trips_and_displays_without_panic_or_lying() {
            let extreme_digits = "9".repeat(50);
            let yaml = format!(
                "duration_extreme: {extreme_digits}y\nduration_zero: \
                 \"-0m\"\ndate: \"2026-07\"\ndatetime: \
                 \"2026-07-29T14:30:00Z\"\n"
            );

            let note = noyalib::from_str::<HostileNote>(&yaml)
                .expect("hostile note deserializes without panicking");

            // A value rebuilt from its parsed seconds (no original
            // spelling to echo) renders in scientific notation, never a
            // silent "0s".
            let synthesized = DurationValue::from_nominal_seconds(
                note.duration_extreme.nominal_seconds(),
            );
            let rendered_extreme = synthesized.to_string();
            assert_ne!(rendered_extreme, "0s");
            assert!(rendered_extreme.contains('e'));

            // "-0m": equals and orders as zero, not a hidden negative.
            assert_eq!(
                note.duration_zero,
                DurationValue::parse("0m").expect("valid duration")
            );
            assert!(note.duration_zero >= DurationValue::parse("0m").unwrap());

            // "YYYY-MM" date: precision defaults to day 1, matches inline
            // parsing exactly.
            assert_eq!(
                note.date,
                DateValue::parse_iso("2026-07-01").expect("valid date")
            );

            // "…Z" datetime: parses to the same instant as the explicit-offset
            // spelling (naive would mean the local zone) and re-serializes
            // with the same spelling.
            assert_eq!(
                note.datetime,
                DateTimeValue::parse_iso("2026-07-29T14:30:00+00:00")
                    .expect("valid datetime")
            );
            assert_eq!(
                serde_json::to_string(&note.datetime).expect("serializable"),
                "\"2026-07-29T14:30:00Z\""
            );
        }
    }
}

use chrono::{
    DateTime, Datelike as _, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta,
    Timelike as _, Utc, Weekday, format::ParseErrorKind,
};
use num_traits::ToPrimitive as _;

use super::{
    CalendarPeriod, DATE_PATTERN, DateComponent, DateError, DateTimeValue,
    DateValue, Difference, DurationComponent, DurationUnit, DurationValue,
    PeriodEdge,
    calendar::{shift_datetime, time_delta_from_seconds},
    date::DateShape,
    datetime::{DateTimeShape, ParsedDateTime},
};

/// Calendar precision of a zone-free date point.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DatePrecision {
    /// Year and month.
    Month,
    /// Full calendar date.
    Day,
}

/// A parsed date or date-time at the precision it was written.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DatePoint {
    /// Zone-free calendar date.
    Calendar {
        /// The calendar date; month precision stores the first of the month.
        date: DateValue,
        /// How much of `date` was written.
        date_precision: DatePrecision,
    },
    /// UTC instant.
    Instant(DateTimeValue),
}

impl DatePoint {
    /// Whether this date point carries a time-of-day component.
    #[must_use]
    pub(crate) const fn has_time(self) -> bool {
        matches!(self, Self::Instant(_))
    }

    /// Calendar date of this date point in the local zone.
    #[must_use]
    pub(crate) fn date(self) -> DateValue {
        match self {
            Self::Calendar {
                date,
                ..
            } => date,
            Self::Instant(dt) => dt.local_date(),
        }
    }

    /// Local datetime reading of this date point: midnight for a calendar date,
    /// the local clock time (UTC when the offset cannot apply) for an instant.
    #[must_use]
    pub(crate) fn local_datetime_or_utc(self) -> NaiveDateTime {
        match self {
            Self::Calendar {
                date,
                ..
            } => date.naive_date().and_time(NaiveTime::MIN),
            Self::Instant(dt) => dt.local_datetime_or_utc(),
        }
    }

    /// UTC instant represented by this date point.
    #[must_use]
    pub(crate) fn instant(self) -> DateTime<Utc> {
        match self {
            Self::Calendar {
                date,
                ..
            } => DateTimeValue::from(date).utc(),
            Self::Instant(dt) => dt.utc(),
        }
    }

    /// Canonical strftime pattern preserving written precision.
    #[must_use]
    pub(crate) fn canonical_pattern(self) -> &'static str {
        match self {
            Self::Calendar {
                date_precision: DatePrecision::Month,
                ..
            } => "%Y-%m",
            Self::Calendar {
                date_precision: DatePrecision::Day,
                ..
            } => DATE_PATTERN,
            Self::Instant(dt) => dt.canonical_pattern(),
        }
    }

    /// Shifts by `n` whole `unit`s: calendar units move the date or local
    /// datetime, clock units move the exact instant (calendar dates move only
    /// when the shift crosses midnight).
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the shift overflows chrono's range.
    /// - [`DateError::LocalZoneLookup`] if an instant's shifted local datetime
    ///   cannot be resolved.
    pub(crate) fn shift(
        self,
        n: i64,
        unit: DurationUnit,
    ) -> Result<Self, DateError> {
        match self {
            Self::Calendar {
                date_precision,
                ..
            } => {
                let local_datetime =
                    shift_datetime(self.local_datetime_or_utc(), n, unit)?;
                Ok(Self::Calendar {
                    date: DateValue::from(local_datetime.date()),
                    date_precision,
                })
            }
            Self::Instant(dt) => dt.shift(n, unit).map(Self::Instant),
        }
    }

    /// Applies retained components in written order, or fixed seconds exactly.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if a component is non-finite, a calendar
    ///   magnitude does not fit `i64`, or any shift overflows.
    /// - [`DateError::LocalZoneLookup`] if an instant's shifted local datetime
    ///   cannot be resolved.
    pub(crate) fn apply(
        self,
        duration: &DurationValue,
    ) -> Result<Self, DateError> {
        match duration.components() {
            Some(components) => {
                components.iter().try_fold(self, |date_point, &component| {
                    date_point.apply_component(component)
                })
            }
            None => self.advance_exact(
                TimeDelta::try_from(duration.nominal_seconds())
                    .map_err(|_| DateError::OutOfRange)?,
            ),
        }
    }

    /// Returns the first day of the period, preserving written precision and
    /// time.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the boundary date is outside chrono's
    ///   date range.
    /// - [`DateError::LocalZoneLookup`] if an instant's boundary datetime
    ///   cannot be resolved.
    pub(crate) fn start_of(
        self,
        calendar_period: CalendarPeriod,
    ) -> Result<Self, DateError> {
        self.boundary(calendar_period, PeriodEdge::Start)
    }

    /// Returns the last day of the period, preserving written precision and
    /// time.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the boundary date is outside chrono's
    ///   date range.
    /// - [`DateError::LocalZoneLookup`] if an instant's boundary datetime
    ///   cannot be resolved.
    pub(crate) fn end_of(
        self,
        calendar_period: CalendarPeriod,
    ) -> Result<Self, DateError> {
        self.boundary(calendar_period, PeriodEdge::End)
    }

    /// Selects a weekday of this date point's ISO week.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the ISO week's `target` day is outside
    ///   chrono's date range.
    /// - [`DateError::LocalZoneLookup`] if an instant's new local datetime
    ///   cannot be resolved.
    pub(crate) fn with_weekday(
        self,
        target: Weekday,
    ) -> Result<Self, DateError> {
        let iso = self.local_datetime_or_utc().date().iso_week();
        let date = NaiveDate::from_isoywd_opt(iso.year(), iso.week(), target)
            .ok_or(DateError::OutOfRange)?;
        self.with_date(date)
    }

    /// Returns one calendar or clock component of the local datetime reading.
    ///
    /// Weekdays are ISO numbered, Monday = 1 through Sunday = 7.
    #[must_use]
    pub(crate) fn component(self, component: DateComponent) -> i64 {
        let local_datetime = self.local_datetime_or_utc();
        match component {
            DateComponent::Year => i64::from(local_datetime.year()),
            DateComponent::Month => i64::from(local_datetime.month()),
            DateComponent::Day => i64::from(local_datetime.day()),
            DateComponent::Hour => i64::from(local_datetime.hour()),
            DateComponent::Minute => i64::from(local_datetime.minute()),
            DateComponent::Second => i64::from(local_datetime.second()),
            DateComponent::Weekday => {
                i64::from(local_datetime.weekday().number_from_monday())
            }
            DateComponent::Week => i64::from(local_datetime.iso_week().week()),
        }
    }

    /// Measures the difference to another date point in the requested unit.
    ///
    /// Years and months are whole calendar counts of the local dates.
    /// Fixed-length units are fractional when both points carry a time and
    /// whole, truncated toward zero, otherwise.
    pub(crate) fn diff(self, to: Self, unit: DurationUnit) -> Difference {
        let local_delta = || {
            to.local_datetime_or_utc()
                .signed_duration_since(self.local_datetime_or_utc())
        };
        match unit {
            DurationUnit::Year => {
                Difference::Whole(self.date().whole_years_until(to.date()))
            }
            DurationUnit::Month => {
                Difference::Whole(self.date().whole_months_until(to.date()))
            }
            fixed if self.has_time() && to.has_time() => {
                let seconds =
                    to.instant().signed_duration_since(self.instant());
                Difference::Fractional(
                    seconds.as_seconds_f64() / fixed.nominal_seconds(),
                )
            }
            DurationUnit::Week => Difference::Whole(local_delta().num_weeks()),
            DurationUnit::Day => Difference::Whole(local_delta().num_days()),
            DurationUnit::Hour => Difference::Whole(local_delta().num_hours()),
            DurationUnit::Minute => {
                Difference::Whole(local_delta().num_minutes())
            }
            DurationUnit::Second => {
                Difference::Whole(local_delta().num_seconds())
            }
            DurationUnit::Millisecond => {
                Difference::Whole(local_delta().num_milliseconds())
            }
        }
    }

    fn advance_local(self, delta: TimeDelta) -> Result<Self, DateError> {
        match self {
            Self::Calendar {
                date,
                date_precision,
            } => Ok(Self::Calendar {
                date: date.advance(delta)?,
                date_precision,
            }),
            Self::Instant(dt) => {
                let local_datetime =
                    dt.local_datetime().ok_or(DateError::OutOfRange)?;
                let shifted = local_datetime
                    .checked_add_signed(delta)
                    .ok_or(DateError::OutOfRange)?;
                Ok(Self::Instant(DateTimeValue::from_local(shifted)?))
            }
        }
    }

    fn advance_exact(self, delta: TimeDelta) -> Result<Self, DateError> {
        match self {
            Self::Calendar {
                date,
                date_precision,
            } => Ok(Self::Calendar {
                date: date.advance(delta)?,
                date_precision,
            }),
            Self::Instant(dt) => Ok(Self::Instant(DateTimeValue::from(
                dt.utc()
                    .checked_add_signed(delta)
                    .ok_or(DateError::OutOfRange)?,
            ))),
        }
    }

    fn apply_component(
        self,
        component: DurationComponent,
    ) -> Result<Self, DateError> {
        let (magnitude, unit) = (component.magnitude(), component.unit());
        if !magnitude.is_finite() {
            return Err(DateError::OutOfRange);
        }
        if unit.is_calendar() {
            let whole =
                magnitude.trunc().to_i64().ok_or(DateError::OutOfRange)?;
            let shifted = if whole != 0 {
                self.shift(whole, unit)?
            } else {
                self
            };
            let remainder = magnitude.fract() * unit.nominal_seconds();
            if remainder == 0.0 {
                Ok(shifted)
            } else {
                shifted.advance_local(time_delta_from_seconds(remainder)?)
            }
        } else {
            self.advance_exact(time_delta_from_seconds(
                magnitude * unit.nominal_seconds(),
            )?)
        }
    }

    fn with_date(self, date: NaiveDate) -> Result<Self, DateError> {
        match self {
            Self::Calendar {
                date_precision,
                ..
            } => Ok(Self::Calendar {
                date: DateValue::from(date),
                date_precision,
            }),
            Self::Instant(_) => {
                let local_datetime =
                    date.and_time(self.local_datetime_or_utc().time());
                Ok(Self::Instant(DateTimeValue::from_local(local_datetime)?))
            }
        }
    }

    fn boundary(
        self,
        calendar_period: CalendarPeriod,
        period_edge: PeriodEdge,
    ) -> Result<Self, DateError> {
        let date = self.local_datetime_or_utc().date();
        let new_date = match (calendar_period, period_edge) {
            (CalendarPeriod::Week, period_edge) => {
                let iso = date.iso_week();
                NaiveDate::from_isoywd_opt(
                    iso.year(),
                    iso.week(),
                    if period_edge == PeriodEdge::Start {
                        Weekday::Mon
                    } else {
                        Weekday::Sun
                    },
                )
            }
            (CalendarPeriod::Month, period_edge) => {
                date.with_day(if period_edge == PeriodEdge::Start {
                    1
                } else {
                    u32::from(date.num_days_in_month())
                })
            }
            (CalendarPeriod::Year, PeriodEdge::Start) => {
                NaiveDate::from_ymd_opt(date.year(), 1, 1)
            }
            (CalendarPeriod::Year, PeriodEdge::End) => {
                NaiveDate::from_ymd_opt(date.year(), 12, 31)
            }
        }
        .ok_or(DateError::OutOfRange)?;
        self.with_date(new_date)
    }

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

/// Maps a chrono parse failure to a short static reason, so errors carry no
/// allocation or foreign error type.
pub(super) fn parse_reason(error: chrono::ParseError) -> &'static str {
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

/// Returns `true` if `bytes` begins with a date-like prefix: two or four ASCII
/// digits followed by `-`.
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
pub(super) fn starts_with_year(s: &str) -> bool {
    s.as_bytes().get(..4).is_some_and(|b| b.iter().all(u8::is_ascii_digit))
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::{temporal::fixtures::fixed_datetime, test_support::TzGuard};

    fn point_at(text: &str) -> DatePoint {
        DatePoint::parse(text).expect("valid date point")
    }

    mod date_point {
        use chrono::NaiveDate;
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn has_time_is_false_for_a_calendar_point() {
            let point = DatePoint::Calendar {
                date: DateValue::from(NaiveDate::MIN),
                date_precision: DatePrecision::Day,
            };

            assert!(!point.has_time());
        }

        #[test]
        fn has_time_is_true_for_an_instant() {
            assert!(DatePoint::Instant(fixed_datetime()).has_time());
        }

        #[rstest]
        #[case::month("2026-07", "%Y-%m")]
        #[case::day("2026-07-29", "%Y-%m-%d")]
        #[case::whole_seconds("2026-07-29T14:30:00Z", "%Y-%m-%dT%H:%M:%S")]
        #[case::fractional_seconds(
            "2026-07-29T14:30:00.123Z",
            "%Y-%m-%dT%H:%M:%S%.f"
        )]
        fn canonical_pattern_preserves_precision_and_fractional_seconds(
            #[case] text: &str,
            #[case] expected: &str,
        ) {
            // Arrange
            TzGuard::set("UTC");
            let point = DatePoint::parse(text).expect("valid date point");

            // Act
            let pattern = point.canonical_pattern();

            // Assert
            assert_eq!(pattern, expected);
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

    mod hostile_yaml_note {
        use pretty_assertions::{assert_eq, assert_ne};

        use super::*;

        #[derive(Deserialize)]
        struct HostileNote {
            duration_extreme: DurationValue,
            duration_zero: DurationValue,
            datetime: DateTimeValue,
        }

        #[test]
        fn parses_round_trips_and_displays_without_panic_or_lying() {
            let extreme_digits = "9".repeat(50);
            let yaml = format!(
                "duration_extreme: {extreme_digits}y\nduration_zero: \
                 \"-0m\"\ndatetime: \"2026-07-29T14:30:00Z\"\n"
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

    mod calendar_owner {
        use pretty_assertions::{assert_eq, assert_ne};
        use rstest::rstest;

        use super::*;

        #[test]
        fn apply_advances_the_instant_by_the_duration() {
            let base = DatePoint::Instant(fixed_datetime());
            let one_hour = DurationValue::parse("1h").expect("valid duration");
            let forward = base.apply(&one_hour).expect("no overflow");
            assert_eq!(
                forward,
                DatePoint::Instant(DateTimeValue::from(
                    base.instant() + TimeDelta::hours(1)
                ))
            );
        }

        #[test]
        fn apply_rewinds_the_instant_by_negated_duration() {
            let base = DatePoint::Instant(fixed_datetime());
            let one_hour = DurationValue::parse("1h").expect("valid duration");
            let back = base.apply(&(-one_hour)).expect("no overflow");
            assert_eq!(
                back,
                DatePoint::Instant(DateTimeValue::from(
                    base.instant() - TimeDelta::hours(1)
                ))
            );
        }

        #[test]
        fn overflows_near_the_representable_range_in_apply() {
            let near_max = DatePoint::Instant(DateTimeValue::from(
                DateTime::<Utc>::MAX_UTC,
            ));
            let one_second =
                DurationValue::parse("1s").expect("valid duration");
            assert_eq!(near_max.apply(&one_second).ok(), None);
        }

        #[test]
        fn underflows_near_the_representable_range_in_apply() {
            let near_min = DatePoint::Instant(DateTimeValue::from(
                DateTime::<Utc>::MIN_UTC,
            ));
            let one_second =
                DurationValue::parse("1s").expect("valid duration");
            assert_eq!(near_min.apply(&(-one_second)).ok(), None);
        }

        #[test]
        fn shift_preserves_the_local_clock_time_for_calendar_units_across_dst()
        {
            TzGuard::set("America/New_York");

            // Saturday March 7, 2026 at 12:00 EST (UTC 17:00:00)
            let base = point_at("2026-03-07 12:00:00");

            // A one-day `shift` keeps 12:00 on the clock time across the gap.
            let shifted_day = base.shift(1, DurationUnit::Day).unwrap();
            assert_eq!(
                shifted_day.local_datetime_or_utc().to_string(),
                "2026-03-08 12:00:00"
            );

            // Week and 7 days share the calendar path, so both keep 12:00.
            let shifted_week = base.shift(1, DurationUnit::Week).unwrap();
            let shifted_7d = base.shift(7, DurationUnit::Day).unwrap();
            assert_eq!(shifted_week, shifted_7d);
            assert_eq!(
                shifted_week.local_datetime_or_utc().to_string(),
                "2026-03-14 12:00:00"
            );

            // 168 hours shifts the instant and lands 13:00 across the gap.
            let shifted_168h = base.shift(168, DurationUnit::Hour).unwrap();
            assert_ne!(shifted_week, shifted_168h);
            assert_eq!(
                shifted_168h.local_datetime_or_utc().to_string(),
                "2026-03-14 13:00:00"
            );

            // `shift` and `apply` agree for the same calendar unit.
            let applied_week =
                base.apply(&DurationValue::parse("1w").unwrap()).unwrap();
            assert_eq!(shifted_week, applied_week);
        }

        #[test]
        fn equal_duration_values_shift_dates_differently() {
            // "1 month" == "30 days" as duration values (fixed ratios)
            let one_month = DurationValue::parse("1 month").unwrap();
            let thirty_days = DurationValue::parse("30 days").unwrap();
            assert_eq!(one_month, thirty_days);

            // Yet they shift a date differently:
            // 2026-01-15 + 1 month = 2026-02-15 (28 days later)
            // 2026-01-15 + 30 days = 2026-02-14 (30 days later)
            let base = point_at("2026-01-15");
            let shifted_month = base.apply(&one_month).unwrap();
            let shifted_days = base.apply(&thirty_days).unwrap();

            assert_eq!(shifted_month, point_at("2026-02-15"));
            assert_eq!(shifted_days, point_at("2026-02-14"));
            assert_ne!(shifted_month, shifted_days);
        }

        #[test]
        fn month_clamping_matches_chrono_documented_behavior() {
            // Chrono's `checked_add_months` clamps to the target month's last
            // day; the calendar owner delegates to it, so the clamp is pinned
            // here.
            let jan_31_non_leap = point_at("2023-01-31");
            let feb_clamped =
                jan_31_non_leap.shift(1, DurationUnit::Month).unwrap();
            assert_eq!(feb_clamped, point_at("2023-02-28"));

            let jan_31_leap = point_at("2024-01-31");
            let feb_leap_clamped =
                jan_31_leap.shift(1, DurationUnit::Month).unwrap();
            assert_eq!(feb_leap_clamped, point_at("2024-02-29"));
        }

        #[test]
        fn day_is_calendar_application_unit_differing_from_24h_across_dst() {
            TzGuard::set("America/New_York");

            // 1d == 24h as duration values
            let one_day = DurationValue::parse("1d").unwrap();
            let twenty_four_hours = DurationValue::parse("24h").unwrap();
            assert_eq!(one_day, twenty_four_hours);

            // Saturday March 7, 2026 at 12:00:00 EST (UTC 17:00:00)
            let base = point_at("2026-03-07 12:00:00");

            // 1d shifts local clock time to Sunday March 8, 2026 at 12:00:00
            // EDT (UTC 16:00:00)
            let shifted_day = base.apply(&one_day).unwrap();
            assert_eq!(
                shifted_day.local_datetime_or_utc().to_string(),
                "2026-03-08 12:00:00"
            );
            assert_eq!(shifted_day, point_at("2026-03-08T16:00:00Z"));

            // 24h shifts stored UTC instant by 24 hours -> Sunday March 8 at
            // 13:00:00 EDT (UTC 17:00:00)
            let shifted_hours = base.apply(&twenty_four_hours).unwrap();
            assert_eq!(
                shifted_hours.local_datetime_or_utc().to_string(),
                "2026-03-08 13:00:00"
            );
            assert_eq!(shifted_hours, point_at("2026-03-08T17:00:00Z"));

            // Equal values, different shifts across DST
            assert_ne!(shifted_day, shifted_hours);
        }

        #[test]
        fn week_shifts_as_seven_calendar_days_matching_7d_and_differing_from_168h()
         {
            TzGuard::set("America/New_York");

            let one_week = DurationValue::parse("1w").unwrap();
            let seven_days = DurationValue::parse("7d").unwrap();
            let hours_168 = DurationValue::parse("168h").unwrap();

            assert_eq!(one_week, seven_days);
            assert_eq!(one_week, hours_168);

            // Saturday March 7, 2026 at 12:00:00 EST
            let base = point_at("2026-03-07 12:00:00");

            let shifted_week = base.apply(&one_week).unwrap();
            let shifted_7d = base.apply(&seven_days).unwrap();
            let shifted_168h = base.apply(&hours_168).unwrap();

            // The `1w` and `7d` forms shift identically.
            assert_eq!(shifted_week, shifted_7d);
            assert_eq!(
                shifted_week.local_datetime_or_utc().to_string(),
                "2026-03-14 12:00:00"
            );

            // The `1w` and `168h` forms differ across the DST transition.
            assert_ne!(shifted_week, shifted_168h);
            assert_eq!(
                shifted_168h.local_datetime_or_utc().to_string(),
                "2026-03-14 13:00:00"
            );
        }

        #[test]
        fn apply_processes_parts_left_to_right_across_dst_transition() {
            TzGuard::set("America/New_York");

            // Saturday March 7, 2026 at 02:30:00 EST (UTC 07:30:00)
            let base = point_at("2026-03-07 02:30:00");

            let d_1d_1h = DurationValue::parse("1d 1h").unwrap();
            let d_1h_1d = DurationValue::parse("1h 1d").unwrap();

            // Values are equal in duration seconds
            assert_eq!(d_1d_1h, d_1h_1d);

            // Left-to-right application:
            // "1d 1h": 1d lands in the spring-forward gap (Sunday 02:30 shifts
            // to 03:30 EDT = 07:30 UTC), then 1h advances the instant to
            // 08:30 UTC (04:30 EDT).
            let res_1d_1h = base.apply(&d_1d_1h).unwrap();
            assert_eq!(res_1d_1h, point_at("2026-03-08T08:30:00Z"));
            assert_eq!(
                res_1d_1h.local_datetime_or_utc().to_string(),
                "2026-03-08 04:30:00"
            );

            // "1h 1d": 1h advances the Saturday instant to 03:30 EST
            // (08:30 UTC), then 1d shifts the clock time to Sunday 03:30 EDT
            // (07:30 UTC).
            let res_1h_1d = base.apply(&d_1h_1d).unwrap();
            assert_eq!(res_1h_1d, point_at("2026-03-08T07:30:00Z"));
            assert_eq!(
                res_1h_1d.local_datetime_or_utc().to_string(),
                "2026-03-08 03:30:00"
            );

            // Different written order yields different result
            assert_ne!(res_1d_1h, res_1h_1d);
        }

        #[test]
        fn fractional_calendar_part_keeps_the_local_clock_time_across_dst() {
            TzGuard::set("America/New_York");

            // Saturday March 7, 2026 at 06:00 EST (UTC 11:00:00)
            let base = point_at("2026-03-07 06:00:00");

            // 1.5d == 36h == 1d 12h as values (129,600 fixed seconds)
            let one_and_half_days = DurationValue::parse("1.5d").unwrap();
            let thirty_six_hours = DurationValue::parse("36h").unwrap();
            let written_order = DurationValue::parse("1d 12h").unwrap();
            assert_eq!(one_and_half_days, thirty_six_hours);
            assert_eq!(one_and_half_days, written_order);

            // The fractional day is a calendar-unit remainder, so it keeps
            // the local clock time: 1d lands Sunday 06:00 EDT, then the
            // 0.5-day remainder adds 12 clock hours to Sunday 18:00 EDT.
            let shifted_fractional = base.apply(&one_and_half_days).unwrap();
            assert_eq!(
                shifted_fractional.local_datetime_or_utc().to_string(),
                "2026-03-08 18:00:00"
            );
            assert_eq!(shifted_fractional, point_at("2026-03-08T22:00:00Z"));

            // The fractional spelling decomposes exactly like its written-order
            // equivalent "1d 12h".
            assert_eq!(shifted_fractional, base.apply(&written_order).unwrap());

            // 36h shifts the stored instant exactly and lands an hour later
            // on the clock time across the 2026-03-08 gap: equal duration
            // values need not shift a date-time identically across a DST
            // boundary.
            let shifted_exact = base.apply(&thirty_six_hours).unwrap();
            assert_ne!(shifted_fractional, shifted_exact);
            assert_eq!(
                shifted_exact.local_datetime_or_utc().to_string(),
                "2026-03-08 19:00:00"
            );
        }

        #[test]
        fn diff_splits_the_fixed_unit_frame_by_time_component_presence() {
            TzGuard::set("UTC");
            let dt1 = point_at("2026-07-28T23:00:00Z");
            let dt2 = point_at("2026-07-29T00:30:00Z");
            let date_only = point_at("2026-07-29");

            // When both date points carry a time component, fixed units return
            // exact f64:
            let diff_hours = dt1.diff(dt2, DurationUnit::Hour);
            assert_eq!(diff_hours, Difference::Fractional(1.5));

            // When either date point is date-only, fixed units return whole
            // i64:
            let diff_calendar = dt1.diff(date_only, DurationUnit::Hour);
            assert_eq!(diff_calendar, Difference::Whole(1));

            // Calendar units return whole counts:
            let diff_months = point_at("2026-01-15")
                .diff(point_at("2026-03-15"), DurationUnit::Month);
            assert_eq!(diff_months, Difference::Whole(2));
        }

        #[test]
        fn shift_preserves_the_local_clock_time_for_months_and_years_across_dst()
         {
            TzGuard::set("America/New_York");

            // Saturday March 7, 2026 at 12:00 EST: month and year shifts
            // preserve the clock time, re-resolving through the local zone
            // after the chrono arithmetic.
            let base = point_at("2026-03-07 12:00:00");
            let shifted_month = base.shift(1, DurationUnit::Month).unwrap();
            assert_eq!(
                shifted_month.local_datetime_or_utc().to_string(),
                "2026-04-07 12:00:00"
            );
            let shifted_year = base.shift(1, DurationUnit::Year).unwrap();
            assert_eq!(
                shifted_year.local_datetime_or_utc().to_string(),
                "2027-03-07 12:00:00"
            );

            // Month-end clamping also re-resolves locally: Jan 31 noon in
            // EST clamps to Feb 28 noon, still reading 12:00.
            let january = point_at("2026-01-31 12:00:00");
            let clamped = january.shift(1, DurationUnit::Month).unwrap();
            assert_eq!(
                clamped.local_datetime_or_utc().to_string(),
                "2026-02-28 12:00:00"
            );
        }

        #[test]
        fn apply_applies_a_fractional_calendar_part_to_a_calendar_date() {
            // 1.5d: the whole day shifts the calendar; the 12-hour
            // remainder lands on the resulting calendar day (midnight + 36h).
            let base = point_at("2026-03-07");
            let shifted =
                base.apply(&DurationValue::parse("1.5d").unwrap()).unwrap();
            assert_eq!(shifted, point_at("2026-03-08"));

            // 1.5mo: the whole month clamps Jan 31 to Feb 28, then the
            // 15-day remainder (half of the nominal 30-day month) applies
            // as exact calendar time.
            let january = point_at("2026-01-31");
            let clamped =
                january.apply(&DurationValue::parse("1.5mo").unwrap()).unwrap();
            assert_eq!(clamped, point_at("2026-03-15"));
        }

        #[rstest]
        #[case::negative_week(-1, DurationUnit::Week, "2026-07-22")]
        #[case::negative_year(-1, DurationUnit::Year, "2025-07-29")]
        #[case::sub_day_unit_leaves_the_calendar_date_intact(
            500,
            DurationUnit::Millisecond,
            "2026-07-29"
        )]
        fn shift_calendar_date_by_signed_units(
            #[case] n: i64,
            #[case] unit: DurationUnit,
            #[case] expected: &str,
        ) {
            let base = point_at("2026-07-29");
            let shifted = base.shift(n, unit).unwrap();
            assert_eq!(shifted, point_at(expected));
        }

        #[test]
        fn month_shift_clamps_to_the_last_day_and_keeps_day_precision() {
            // Arrange
            let january_31 = point_at("2026-01-31");

            // Act
            let shifted = january_31.shift(1, DurationUnit::Month).unwrap();

            // Assert
            assert_eq!(shifted, point_at("2026-02-28"));
            assert_eq!(shifted.canonical_pattern(), "%Y-%m-%d");
        }

        #[rstest]
        #[case::five_hours(5, "2026-07-29")]
        #[case::just_under_a_day(23, "2026-07-29")]
        #[case::one_day(24, "2026-07-30")]
        #[case::over_a_day(25, "2026-07-30")]
        #[case::backwards(-1, "2026-07-28")]
        fn hour_shift_moves_a_calendar_date_only_across_midnight(
            #[case] hours: i64,
            #[case] expected: &str,
        ) {
            // Arrange
            let base = point_at("2026-07-29");

            // Act
            let shifted = base.shift(hours, DurationUnit::Hour).unwrap();

            // Assert
            assert_eq!(shifted, point_at(expected));
        }

        #[test]
        fn month_shift_keeps_month_precision() {
            // Arrange
            let base = point_at("2026-07");

            // Act
            let shifted = base.shift(1, DurationUnit::Month).unwrap();

            // Assert
            assert_eq!(shifted.canonical_pattern(), "%Y-%m");
            assert_eq!(
                shifted.local_datetime_or_utc().format("%Y-%m").to_string(),
                "2026-08"
            );
        }

        #[test]
        fn date_point_calendar_shift_preserves_noon_across_dst() {
            // Arrange
            TzGuard::set("America/New_York");
            let date_point = point_at("2026-03-07T12:00:00");

            // Act
            let shifted = date_point.shift(1, DurationUnit::Day).unwrap();

            // Assert
            assert_eq!(
                shifted.local_datetime_or_utc().to_string(),
                "2026-03-08 12:00:00"
            );
        }

        #[test]
        fn shift_reports_out_of_range_at_date_and_instant_limits() {
            // Arrange
            TzGuard::set("UTC");
            let instant = DatePoint::Instant(DateTimeValue::from(
                DateTime::<Utc>::MAX_UTC,
            ));
            let date = DatePoint::Calendar {
                date: DateValue::from(NaiveDate::MAX),
                date_precision: DatePrecision::Day,
            };

            // Act
            let instant_result = instant.shift(1, DurationUnit::Day);
            let date_result = date.shift(1, DurationUnit::Day);

            // Assert
            assert_eq!(instant_result, Err(DateError::OutOfRange));
            assert_eq!(date_result, Err(DateError::OutOfRange));
        }

        #[test]
        fn date_apply_restarts_each_sub_day_component_at_midnight() {
            // Arrange
            let date = point_at("2026-07-29");
            let separate = DurationValue::parse("12h 12h").unwrap();
            let together = DurationValue::parse("24h").unwrap();

            // Act
            let separate_date = date.apply(&separate).unwrap();
            let together_date = date.apply(&together).unwrap();

            // Assert
            assert_eq!(separate_date, date);
            assert_eq!(together_date, point_at("2026-07-30"));
        }
    }

    mod date_point_boundaries {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn week_boundary_returns_iso_monday_spanning_month_boundaries() {
            let date_point = point_at("2026-02-01");
            let sow = date_point.start_of(CalendarPeriod::Week).expect("sow");
            let eow = date_point.end_of(CalendarPeriod::Week).expect("eow");
            assert_eq!(
                sow.local_datetime_or_utc().date(),
                DateValue::parse_iso("2026-01-26").unwrap().naive_date()
            );
            assert_eq!(
                eow.local_datetime_or_utc().date(),
                DateValue::parse_iso("2026-02-01").unwrap().naive_date()
            );
        }

        #[test]
        fn year_boundary_returns_first_and_last_day_of_year() {
            let date_point = point_at("2026-07-29");
            let soy = date_point.start_of(CalendarPeriod::Year).expect("soy");
            let eoy = date_point.end_of(CalendarPeriod::Year).expect("eoy");
            assert_eq!(
                soy.local_datetime_or_utc().date(),
                DateValue::parse_iso("2026-01-01").unwrap().naive_date()
            );
            assert_eq!(
                eoy.local_datetime_or_utc().date(),
                DateValue::parse_iso("2026-12-31").unwrap().naive_date()
            );
        }

        #[test]
        fn date_point_with_weekday_maps_to_target_weekday_in_iso_week() {
            let date_point = point_at("2026-07-29");
            let mon = date_point.with_weekday(Weekday::Mon).expect("mon");
            let sun = date_point.with_weekday(Weekday::Sun).expect("sun");
            assert_eq!(
                mon.local_datetime_or_utc().date(),
                DateValue::parse_iso("2026-07-27").unwrap().naive_date()
            );
            assert_eq!(
                sun.local_datetime_or_utc().date(),
                DateValue::parse_iso("2026-08-02").unwrap().naive_date()
            );
        }

        #[test]
        fn date_point_apply_shifts_by_compound_duration() {
            let date_point = point_at("2026-07-29");
            let dur = DurationValue::parse("1w 2d").unwrap();
            let shifted = date_point.apply(&dur).expect("apply succeeds");
            assert_eq!(
                shifted.local_datetime_or_utc().date(),
                DateValue::parse_iso("2026-08-07").unwrap().naive_date()
            );
            assert_eq!(shifted.canonical_pattern(), "%Y-%m-%d");
        }
    }

    mod parsing {
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::too_short("202")]
        #[case::empty("")]
        #[case::non_digit_year("abcd-01-01")]
        fn rejects_a_missing_four_digit_year(#[case] input: &str) {
            assert!(!starts_with_year(input));
        }

        #[test]
        fn accepts_a_five_digit_year_prefix_as_having_four_digits() {
            // The `starts_with_year` guard only checks the first 4 bytes; a
            // longer numeric prefix still passes this guard and is rejected
            // later, by the format cascade, as InvalidValue rather than
            // YearNotFourDigits.
            assert!(starts_with_year("20265-01-01"));
            assert!(matches!(
                DateValue::parse_iso("20265-01-01"),
                Err(DateError::InvalidValue { .. })
            ));
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

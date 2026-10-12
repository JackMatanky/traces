use chrono::{
    Datelike as _, Days, Months, NaiveDate, NaiveDateTime, NaiveTime,
    TimeDelta, Timelike as _, Weekday,
};
use num_traits::ToPrimitive as _;

use super::{
    error::DateError,
    value::{DatePoint, DateTimeValue, DateValue},
};
use crate::duration::{
    DurationComponent, DurationSeconds, DurationUnit, DurationValue,
};

/// Calendar period used for date boundaries.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum CalendarPeriod {
    /// The ISO week, Monday through Sunday.
    Week,
    /// The calendar month.
    Month,
    /// The calendar year.
    Year,
}

/// Which side of a calendar period to select.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum PeriodEdge {
    /// First day of the period.
    Start,
    /// Last day of the period.
    End,
}

/// A named calendar or clock component of a [`DatePoint`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DateComponent {
    /// Calendar year.
    Year,
    /// Calendar month, 1 through 12.
    Month,
    /// Day of the month, 1 through 31.
    Day,
    /// Hour of the day, 0 through 23.
    Hour,
    /// Minute of the hour, 0 through 59.
    Minute,
    /// Second of the minute, 0 through 59.
    Second,
    /// ISO weekday number, Monday = 1 through Sunday = 7.
    Weekday,
    /// ISO week number of the year.
    Week,
}

impl DateComponent {
    /// Resolves a component name, ASCII case-insensitively.
    #[must_use]
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        const TABLE: [(&str, DateComponent); 8] = [
            ("year", DateComponent::Year),
            ("month", DateComponent::Month),
            ("day", DateComponent::Day),
            ("hour", DateComponent::Hour),
            ("minute", DateComponent::Minute),
            ("second", DateComponent::Second),
            ("weekday", DateComponent::Weekday),
            ("week", DateComponent::Week),
        ];
        TABLE
            .iter()
            .find(|(candidate, _)| name.eq_ignore_ascii_case(candidate))
            .map(|&(_, component)| component)
    }
}

/// Result of measuring the difference between two date points.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum Difference {
    /// Whole units elapsed.
    Whole(i64),
    /// Fractional units elapsed.
    Fractional(f64),
}

impl Difference {
    /// Returns the measured difference magnitude as an `f64`.
    #[inline]
    #[must_use]
    pub(crate) fn to_f64(self) -> Option<f64> {
        match self {
            Self::Whole(n) => num_traits::ToPrimitive::to_f64(&n),
            Self::Fractional(n) => Some(n),
        }
    }
}

impl DatePoint {
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
}

impl DateValue {
    /// Whole calendar months from `self` to `to`, signed.
    ///
    /// Chrono has no `months_since` equivalent, so this mirrors
    /// [`NaiveDate::years_since`]'s algorithm at month granularity: total
    /// calendar months between the dates, minus one when the day-of-month has
    /// not yet been reached. Either ordering is accepted.
    fn whole_months_until(self, to: Self) -> i64 {
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
    fn whole_years_until(self, to: Self) -> i64 {
        self.whole_months_until(to) / 12
    }

    /// Moves the date by `delta` read from midnight, keeping the date part.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the result is outside chrono's date
    ///   range.
    fn advance(self, delta: TimeDelta) -> Result<Self, DateError> {
        self.naive_date()
            .and_time(NaiveTime::MIN)
            .checked_add_signed(delta)
            .map(|local_datetime| Self::from(local_datetime.date()))
            .ok_or(DateError::OutOfRange)
    }
}

impl DateTimeValue {
    /// Shifts calendar units in the local datetime frame and clock units in
    /// UTC.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the shift overflows, or the local
    ///   datetime of a calendar-unit shift is outside chrono's range.
    /// - [`DateError::LocalZoneLookup`] if the shifted local datetime cannot be
    ///   resolved to an instant.
    pub(crate) fn shift(
        self,
        n: i64,
        unit: DurationUnit,
    ) -> Result<Self, DateError> {
        if unit.is_calendar() {
            let local_datetime =
                self.local_datetime().ok_or(DateError::OutOfRange)?;
            Self::from_local(shift_datetime(local_datetime, n, unit)?)
        } else {
            Ok(Self::from(
                shift_datetime(self.utc().naive_utc(), n, unit)?.and_utc(),
            ))
        }
    }
}

/// Maps an ISO weekday number to a [`Weekday`]: `1` is Monday through `7`
/// Sunday. `0` is rejected because Sunday-based conventions disagree on it
/// (Sunday is `0` in some and `1` in others).
#[must_use]
pub(crate) const fn weekday_from_number(n: i64) -> Option<Weekday> {
    match n {
        1 => Some(Weekday::Mon),
        2 => Some(Weekday::Tue),
        3 => Some(Weekday::Wed),
        4 => Some(Weekday::Thu),
        5 => Some(Weekday::Fri),
        6 => Some(Weekday::Sat),
        7 => Some(Weekday::Sun),
        _ => None,
    }
}

/// Builds a [`TimeDelta`] from `part_secs`, splitting whole seconds from
/// nanoseconds (the fraction rounded to the nearest nanosecond) and rejecting
/// overflow of either component for `DatePoint::apply`.
///
/// # Errors
///
/// - [`DateError::OutOfRange`] if `part_secs` is not finite or its whole
///   seconds do not fit a [`TimeDelta`].
fn time_delta_from_seconds(part_secs: f64) -> Result<TimeDelta, DateError> {
    let ds = DurationSeconds::try_from(part_secs)
        .map_err(|_| DateError::OutOfRange)?;
    TimeDelta::try_from(ds).map_err(|_| DateError::OutOfRange)
}

/// Shifts a calendar datetime by `n` `unit`s.
///
/// - Calendar units shift calendar days; Month and Year clamp to the last day
///   of the target month (chrono's documented behavior).
/// - Sub-day units shift the time exactly.
///
/// No zone is consulted: callers wanting DST-aware local-clock semantics route
/// through [`DateTimeValue::shift`] instead.
///
/// # Errors
///
/// - [`DateError::OutOfRange`] if the shift overflows representable bounds.
fn shift_datetime(
    local_datetime: NaiveDateTime,
    n: i64,
    unit: DurationUnit,
) -> Result<NaiveDateTime, DateError> {
    match unit {
        DurationUnit::Year => {
            let months = n.checked_mul(12).ok_or(DateError::OutOfRange)?;
            shift_datetime_months(local_datetime, months)
        }
        DurationUnit::Month => shift_datetime_months(local_datetime, n),
        DurationUnit::Week => {
            let days = n.checked_mul(7).ok_or(DateError::OutOfRange)?;
            let days_u64 = days.unsigned_abs();
            if days >= 0 {
                local_datetime.checked_add_days(Days::new(days_u64))
            } else {
                local_datetime.checked_sub_days(Days::new(days_u64))
            }
            .ok_or(DateError::OutOfRange)
        }
        DurationUnit::Day => {
            let days_u64 = n.unsigned_abs();
            if n >= 0 {
                local_datetime.checked_add_days(Days::new(days_u64))
            } else {
                local_datetime.checked_sub_days(Days::new(days_u64))
            }
            .ok_or(DateError::OutOfRange)
        }
        DurationUnit::Millisecond => {
            let delta = chrono::Duration::try_milliseconds(n)
                .ok_or(DateError::OutOfRange)?;
            local_datetime
                .checked_add_signed(delta)
                .ok_or(DateError::OutOfRange)
        }
        DurationUnit::Second | DurationUnit::Minute | DurationUnit::Hour => {
            let secs = unit
                .whole_nominal_seconds()
                .ok_or(DateError::OutOfRange)?
                .checked_mul(n)
                .ok_or(DateError::OutOfRange)?;
            let delta = chrono::Duration::try_seconds(secs)
                .ok_or(DateError::OutOfRange)?;
            local_datetime
                .checked_add_signed(delta)
                .ok_or(DateError::OutOfRange)
        }
    }
}

/// Shifts `local_datetime` by `months` calendar months (positive or negative).
///
/// Clamps to the last day of the target month per chrono's documented behavior.
///
/// # Errors
///
/// - [`DateError::OutOfRange`] if `months` does not fit a `u32` or the
///   arithmetic overflows representable bounds.
fn shift_datetime_months(
    local_datetime: NaiveDateTime,
    months: i64,
) -> Result<NaiveDateTime, DateError> {
    let months_u32 = u32::try_from(months.unsigned_abs())
        .map_err(|_| DateError::OutOfRange)?;
    if months >= 0 {
        local_datetime.checked_add_months(Months::new(months_u32))
    } else {
        local_datetime.checked_sub_months(Months::new(months_u32))
    }
    .ok_or(DateError::OutOfRange)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::{
        date::{DatePrecision, fixtures::fixed_datetime},
        test_support::TzGuard,
    };

    fn point_at(text: &str) -> DatePoint {
        DatePoint::parse(text).expect("valid date point")
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
        fn clock_shift_keeps_fractional_seconds() {
            // Arrange
            TzGuard::set("UTC");
            let base = point_at("2026-07-29T14:30:00.123");

            // Act
            let shifted = base.shift(1, DurationUnit::Hour).unwrap();

            // Assert
            assert_eq!(
                shifted
                    .local_datetime_or_utc()
                    .format(shifted.canonical_pattern())
                    .to_string(),
                "2026-07-29T15:30:00.123"
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
        fn zero_day_shift_re_resolves_fold_to_earlier_occurrence() {
            // Arrange
            TzGuard::set("America/New_York");
            let second_occurrence = point_at("2026-11-01T06:30:00Z");

            // Act: calendar shifts re-resolve through the earliest-occurrence
            // policy.
            let shifted =
                second_occurrence.shift(0, DurationUnit::Day).unwrap();

            // Assert
            assert_eq!(
                shifted.instant(),
                point_at("2026-11-01T05:30:00Z").instant()
            );
        }

        #[test]
        fn zero_day_shift_in_utc_keeps_the_instant() {
            // Arrange
            TzGuard::set("UTC");
            let date_point = point_at("2026-07-29T14:30:00Z");

            // Act
            let shifted = date_point.shift(0, DurationUnit::Day).unwrap();

            // Assert
            assert_eq!(shifted.instant(), date_point.instant());
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

        #[test]
        fn date_diff_to_f64_projects_whole_and_exact_variants() {
            assert_eq!(Difference::Whole(42).to_f64(), Some(42.0));
            assert_eq!(Difference::Fractional(2.5).to_f64(), Some(2.5));
        }
    }
}

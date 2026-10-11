use chrono::{
    Datelike as _, Days, Months, NaiveDate, NaiveDateTime, TimeDelta, Weekday,
};
use num_traits::ToPrimitive as _;

use super::value::{
    DateError, DatePoint, DateTimeValue, DateValue, local_naive_to_utc,
};
use crate::duration::{
    DurationComponent, DurationSeconds, DurationUnit, DurationValue,
};

impl DateValue {
    /// Shifts this civil date by `n` `unit`s.
    ///
    /// Pure civil arithmetic: zone-free dates never touch the DST resolver.
    /// Delegates to [`shift_wall`] at midnight and keeps the resulting calendar
    /// day, so per-unit semantics live in exactly one place.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if arithmetic overflows representable
    ///   bounds.
    pub(crate) fn shift(
        self,
        n: i64,
        unit: DurationUnit,
    ) -> Result<Self, DateError> {
        let wall = self.0.and_hms_opt(0, 0, 0).ok_or(DateError::OutOfRange)?;
        shift_wall(wall, n, unit).map(|shifted| Self(shifted.date()))
    }

    /// Applies `duration` to this civil date.
    ///
    /// Pure civil arithmetic. Retained written components apply in written
    /// left-to-right order; a fixed magnitude applies as exact seconds on the
    /// civil wall clock.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if arithmetic overflows or the duration
    ///   contains non-finite seconds.
    pub(crate) fn apply(
        self,
        duration: &DurationValue,
    ) -> Result<Self, DateError> {
        if let Some(components) = duration.components() {
            let mut current = self;
            for &DurationComponent {
                magnitude,
                unit,
            } in components
            {
                current = current.apply_part(magnitude, unit)?;
            }
            Ok(current)
        } else {
            // Fixed magnitude: exact seconds on the civil wall clock; the
            // `TimeDelta` conversion rejects non-finite input.
            let wall =
                self.0.and_hms_opt(0, 0, 0).ok_or(DateError::OutOfRange)?;
            let delta = TimeDelta::try_from(duration.nominal_seconds())
                .map_err(|_| DateError::OutOfRange)?;
            let shifted_wall =
                wall.checked_add_signed(delta).ok_or(DateError::OutOfRange)?;
            Ok(Self(shifted_wall.date()))
        }
    }

    /// Applies one written `(magnitude, unit)` part to this civil date.
    ///
    /// Calendar units ([`DurationUnit::Year`], [`DurationUnit::Month`],
    /// [`DurationUnit::Week`], [`DurationUnit::Day`]) truncate to a whole count
    /// for [`Self::shift`]. Fractional remainders use nominal fixed ratios
    /// (half a month is 15 days), applied at the resulting midnight. Sub-day
    /// units apply exactly at midnight, even when they advance the civil date.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the magnitude is non-finite or the
    ///   arithmetic overflows.
    fn apply_part(
        self,
        mag: f64,
        unit: DurationUnit,
    ) -> Result<Self, DateError> {
        if !mag.is_finite() {
            return Err(DateError::OutOfRange);
        }
        match unit {
            DurationUnit::Day
            | DurationUnit::Week
            | DurationUnit::Month
            | DurationUnit::Year => {
                let whole =
                    mag.trunc().to_i64().ok_or(DateError::OutOfRange)?;
                let mut current = if whole != 0 {
                    self.shift(whole, unit)?
                } else {
                    self
                };
                let rem_secs = mag.fract() * unit.nominal_seconds();
                if rem_secs != 0.0 {
                    let wall = current
                        .0
                        .and_hms_opt(0, 0, 0)
                        .ok_or(DateError::OutOfRange)?;
                    let delta = seconds_delta(rem_secs)?;
                    let shifted_wall = wall
                        .checked_add_signed(delta)
                        .ok_or(DateError::OutOfRange)?;
                    current = Self(shifted_wall.date());
                }
                Ok(current)
            }
            DurationUnit::Hour
            | DurationUnit::Minute
            | DurationUnit::Second
            | DurationUnit::Millisecond => {
                let wall =
                    self.0.and_hms_opt(0, 0, 0).ok_or(DateError::OutOfRange)?;
                let delta = seconds_delta(mag * unit.nominal_seconds())?;
                let shifted_wall = wall
                    .checked_add_signed(delta)
                    .ok_or(DateError::OutOfRange)?;
                Ok(Self(shifted_wall.date()))
            }
        }
    }
}

impl DateTimeValue {
    /// Shifts this date-time by `n` `unit`s.
    ///
    /// Calendar units ([`DurationUnit::Year`], [`DurationUnit::Month`],
    /// [`DurationUnit::Week`], [`DurationUnit::Day`]) shift the local wall
    /// clock, preserving the wall-clock time across DST transitions. Sub-day
    /// units shift the stored instant exactly.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if arithmetic overflows.
    /// - [`DateError::LocalZoneLookup`] if the shifted local wall clock cannot
    ///   be resolved.
    pub(crate) fn shift(
        self,
        n: i64,
        unit: DurationUnit,
    ) -> Result<Self, DateError> {
        match unit {
            DurationUnit::Year => {
                let months = n.checked_mul(12).ok_or(DateError::OutOfRange)?;
                self.shift_calendar_months(months)
            }
            DurationUnit::Month => self.shift_calendar_months(n),
            DurationUnit::Week => {
                let days = n.checked_mul(7).ok_or(DateError::OutOfRange)?;
                self.shift_calendar_days(days)
            }
            DurationUnit::Day => self.shift_calendar_days(n),
            DurationUnit::Hour
            | DurationUnit::Minute
            | DurationUnit::Second => {
                let secs = unit
                    .whole_nominal_seconds()
                    .ok_or(DateError::OutOfRange)?
                    .checked_mul(n)
                    .ok_or(DateError::OutOfRange)?;
                let delta = chrono::Duration::try_seconds(secs)
                    .ok_or(DateError::OutOfRange)?;
                let instant = self
                    .0
                    .checked_add_signed(delta)
                    .ok_or(DateError::OutOfRange)?;
                Ok(Self(instant))
            }
            DurationUnit::Millisecond => {
                let delta = chrono::Duration::try_milliseconds(n)
                    .ok_or(DateError::OutOfRange)?;
                let instant = self
                    .0
                    .checked_add_signed(delta)
                    .ok_or(DateError::OutOfRange)?;
                Ok(Self(instant))
            }
        }
    }

    /// Shifts the local wall clock by `months` calendar months, then resolves
    /// the result back to an instant through the local zone.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the local wall clock is unavailable or
    ///   the month arithmetic overflows.
    /// - [`DateError::LocalZoneLookup`] if the shifted wall clock cannot be
    ///   resolved in the local zone.
    fn shift_calendar_months(self, months: i64) -> Result<Self, DateError> {
        let wall = self.local_wall().ok_or(DateError::OutOfRange)?;
        let new_wall = shift_months(wall, months)?;
        let instant = local_naive_to_utc(new_wall)?;
        Ok(Self(instant))
    }

    /// Shifts the local wall clock by `days` calendar days, then resolves the
    /// result back to an instant through the local zone.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the local wall clock is unavailable or
    ///   the day arithmetic overflows.
    /// - [`DateError::LocalZoneLookup`] if the shifted wall clock cannot be
    ///   resolved in the local zone.
    fn shift_calendar_days(self, days: i64) -> Result<Self, DateError> {
        let wall = self.local_wall().ok_or(DateError::OutOfRange)?;
        let days_u64 = days.unsigned_abs();
        let new_wall = if days >= 0 {
            wall.checked_add_days(Days::new(days_u64))
        } else {
            wall.checked_sub_days(Days::new(days_u64))
        }
        .ok_or(DateError::OutOfRange)?;
        let instant = local_naive_to_utc(new_wall)?;
        Ok(Self(instant))
    }

    /// Applies `duration` to this date-time.
    ///
    /// - Retained written components ([`DurationValue::components`] is
    ///   [`Some`]) apply in written left-to-right order: calendar units shift
    ///   the local wall clock (wall time survives DST), sub-day units shift the
    ///   stored instant exactly.
    /// - A fixed magnitude ([`DurationValue::components`] is [`None`]) applies
    ///   as exact seconds to the stored instant.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if arithmetic overflows or the duration
    ///   contains non-finite seconds.
    /// - [`DateError::LocalZoneLookup`] if a shifted local wall clock cannot be
    ///   resolved.
    pub(crate) fn apply(
        self,
        duration: &DurationValue,
    ) -> Result<Self, DateError> {
        if let Some(components) = duration.components() {
            let mut current = self;
            for &DurationComponent {
                magnitude,
                unit,
            } in components
            {
                current = current.apply_part(magnitude, unit)?;
            }
            Ok(current)
        } else {
            // Fixed magnitude: exact seconds on the stored instant; the
            // `TimeDelta` conversion rejects non-finite input.
            let delta = TimeDelta::try_from(duration.nominal_seconds())
                .map_err(|_| DateError::OutOfRange)?;
            let instant = self
                .0
                .checked_add_signed(delta)
                .ok_or(DateError::OutOfRange)?;
            Ok(Self(instant))
        }
    }

    /// Applies one written `(magnitude, unit)` part to this date-time.
    ///
    /// Calendar units ([`DurationUnit::Year`], [`DurationUnit::Month`],
    /// [`DurationUnit::Week`], [`DurationUnit::Day`]) truncate to a whole count
    /// for [`Self::shift`]. Fractional remainders use nominal fixed ratios
    /// (half a month is 15 days) on the local wall clock, then re-resolve
    /// through the local zone; sub-day units shift the instant exactly.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the magnitude is non-finite or the
    ///   arithmetic overflows.
    /// - [`DateError::LocalZoneLookup`] if the shifted wall clock cannot be
    ///   resolved in the local zone.
    fn apply_part(
        self,
        mag: f64,
        unit: DurationUnit,
    ) -> Result<Self, DateError> {
        if !mag.is_finite() {
            return Err(DateError::OutOfRange);
        }
        match unit {
            DurationUnit::Day
            | DurationUnit::Week
            | DurationUnit::Month
            | DurationUnit::Year => {
                let whole =
                    mag.trunc().to_i64().ok_or(DateError::OutOfRange)?;
                let mut current = if whole != 0 {
                    self.shift(whole, unit)?
                } else {
                    self
                };
                let rem_secs = mag.fract() * unit.nominal_seconds();
                if rem_secs != 0.0 {
                    let delta = seconds_delta(rem_secs)?;
                    // The nominal remainder advances the local wall clock
                    // rather than the stored instant across DST.
                    let wall =
                        current.local_wall().ok_or(DateError::OutOfRange)?;
                    let shifted_wall = wall
                        .checked_add_signed(delta)
                        .ok_or(DateError::OutOfRange)?;
                    let instant = local_naive_to_utc(shifted_wall)?;
                    current = Self(instant);
                }
                Ok(current)
            }
            DurationUnit::Hour
            | DurationUnit::Minute
            | DurationUnit::Second
            | DurationUnit::Millisecond => {
                let delta = seconds_delta(mag * unit.nominal_seconds())?;
                let instant = self
                    .0
                    .checked_add_signed(delta)
                    .ok_or(DateError::OutOfRange)?;
                Ok(Self(instant))
            }
        }
    }
}

impl DatePoint {
    /// Shifts this date point by `n` `unit`s, owning the civil vs instant
    /// frame policy.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if arithmetic overflows chrono's
    ///   representable range.
    /// - [`DateError::LocalZoneLookup`] if resolving a local wall time to UTC
    ///   fails.
    pub(crate) fn shift(
        self,
        n: i64,
        unit: DurationUnit,
    ) -> Result<Self, DateError> {
        if self.has_time() {
            let dt = DateTimeValue(self.instant).shift(n, unit)?;
            let wall = dt.local_wall().ok_or(DateError::OutOfRange)?;
            Ok(Self {
                wall,
                instant: dt.into_inner(),
                precision: self.precision,
            })
        } else {
            let shifted_wall = shift_wall(self.wall, n, unit)?;
            let shifted_date = DateValue(shifted_wall.date());
            let instant = DateTimeValue::from(shifted_date).into_inner();
            Ok(Self {
                wall: shifted_wall,
                instant,
                precision: self.precision,
            })
        }
    }

    /// Applies `duration` to this date point, preserving its civil vs instant
    /// frame policy and precision.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if arithmetic overflows chrono's
    ///   representable range.
    /// - [`DateError::LocalZoneLookup`] if resolving a local wall time to UTC
    ///   fails.
    pub(crate) fn apply(
        self,
        duration: &DurationValue,
    ) -> Result<Self, DateError> {
        if self.has_time() {
            let dt = DateTimeValue(self.instant).apply(duration)?;
            let wall = dt.local_wall().ok_or(DateError::OutOfRange)?;
            Ok(Self {
                wall,
                instant: dt.into_inner(),
                precision: self.precision,
            })
        } else {
            let date = DateValue(self.wall.date()).apply(duration)?;
            let wall = date
                .into_inner()
                .and_hms_opt(0, 0, 0)
                .ok_or(DateError::OutOfRange)?;
            let instant = DateTimeValue::from(date).into_inner();
            Ok(Self {
                wall,
                instant,
                precision: self.precision,
            })
        }
    }

    /// Moves to the first or last day of this point's month in its civil frame,
    /// preserving its time-of-day and precision.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the target date is outside chrono's
    ///   representable range.
    /// - [`DateError::LocalZoneLookup`] if resolving the target local wall time
    ///   to UTC fails.
    pub(crate) fn month_boundary(self, end: bool) -> Result<Self, DateError> {
        let date = self
            .wall
            .date()
            .with_day(if end {
                u32::from(self.wall.date().num_days_in_month())
            } else {
                1
            })
            .ok_or(DateError::OutOfRange)?;
        let wall = date.and_time(self.wall.time());
        let instant = if self.has_time() {
            local_naive_to_utc(wall)?
        } else {
            DateTimeValue::from(DateValue(date)).into_inner()
        };
        Ok(Self {
            wall,
            instant,
            precision: self.precision,
        })
    }

    /// Moves to the first (Monday) or last (Sunday) day of this point's ISO
    /// week in its civil frame, preserving its time-of-day and precision.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the target date is outside chrono's
    ///   representable range.
    /// - [`DateError::LocalZoneLookup`] if resolving the target local wall time
    ///   to UTC fails.
    pub(crate) fn week_boundary(self, end: bool) -> Result<Self, DateError> {
        let date = self.wall.date();
        let iso = date.iso_week();
        let target_weekday = if end {
            Weekday::Sun
        } else {
            Weekday::Mon
        };
        let new_date =
            NaiveDate::from_isoywd_opt(iso.year(), iso.week(), target_weekday)
                .ok_or(DateError::OutOfRange)?;
        let wall = new_date.and_time(self.wall.time());
        let instant = if self.has_time() {
            local_naive_to_utc(wall)?
        } else {
            DateTimeValue::from(DateValue(new_date)).into_inner()
        };
        Ok(Self {
            wall,
            instant,
            precision: self.precision,
        })
    }

    /// Moves to the first (Jan 1) or last (Dec 31) day of this point's calendar
    /// year in its civil frame, preserving its time-of-day and precision.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the target date is outside chrono's
    ///   representable range.
    /// - [`DateError::LocalZoneLookup`] if resolving the target local wall time
    ///   to UTC fails.
    pub(crate) fn year_boundary(self, end: bool) -> Result<Self, DateError> {
        let date = self.wall.date();
        let new_date = if end {
            NaiveDate::from_ymd_opt(date.year(), 12, 31)
        } else {
            NaiveDate::from_ymd_opt(date.year(), 1, 1)
        }
        .ok_or(DateError::OutOfRange)?;
        let wall = new_date.and_time(self.wall.time());
        let instant = if self.has_time() {
            local_naive_to_utc(wall)?
        } else {
            DateTimeValue::from(DateValue(new_date)).into_inner()
        };
        Ok(Self {
            wall,
            instant,
            precision: self.precision,
        })
    }

    /// Moves to the specified ISO weekday in this point's ISO week, preserving
    /// its time-of-day and precision.
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the target date exceeds representable
    ///   bounds.
    /// - [`DateError::LocalZoneLookup`] if resolving the target local wall time
    ///   to UTC fails.
    pub(crate) fn weekday_point(
        self,
        target: Weekday,
    ) -> Result<Self, DateError> {
        let date = self.wall.date();
        let iso = date.iso_week();
        let new_date =
            NaiveDate::from_isoywd_opt(iso.year(), iso.week(), target)
                .ok_or(DateError::OutOfRange)?;
        let wall = new_date.and_time(self.wall.time());
        let instant = if self.has_time() {
            local_naive_to_utc(wall)?
        } else {
            DateTimeValue::from(DateValue(new_date)).into_inner()
        };
        Ok(Self {
            wall,
            instant,
            precision: self.precision,
        })
    }

    /// Measures the difference from `self` to `to` in `unit`s.
    ///
    /// Preserves the declared measurement split:
    /// - When both points carry a time component, fixed units measure elapsed
    ///   time between the stored UTC instants as exact `f64`.
    /// - When either point is date-only, fixed units measure elapsed civil wall
    ///   clocks as `i64`.
    /// - Calendar units ([`DurationUnit::Year`], [`DurationUnit::Month`])
    ///   measure elapsed calendar periods using day-of-month aware whole counts
    ///   ([`signed_years_since`], [`signed_months_since`]).
    ///
    /// # Errors
    ///
    /// - [`DateError::OutOfRange`] if the difference overflows representable
    ///   bounds. Defensive only: every conversion on this path is total, so no
    ///   input currently produces it.
    pub(crate) fn diff(
        self,
        to: Self,
        unit: DurationUnit,
    ) -> Result<Difference, DateError> {
        let both_datetimes = self.has_time() && to.has_time();
        match unit {
            DurationUnit::Year => Ok(Difference::Whole(signed_years_since(
                self.wall.date(),
                to.wall.date(),
            ))),
            DurationUnit::Month => Ok(Difference::Whole(signed_months_since(
                self.wall.date(),
                to.wall.date(),
            ))),
            DurationUnit::Week
            | DurationUnit::Day
            | DurationUnit::Hour
            | DurationUnit::Minute
            | DurationUnit::Second
            | DurationUnit::Millisecond => {
                if both_datetimes {
                    let delta = to.instant.signed_duration_since(self.instant);
                    let whole_seconds = delta
                        .num_seconds()
                        .to_f64()
                        .ok_or(DateError::OutOfRange)?;
                    let result = (whole_seconds
                        + f64::from(delta.subsec_nanos()) / 1e9)
                        / unit.nominal_seconds();
                    Ok(Difference::Fractional(result))
                } else if unit == DurationUnit::Millisecond {
                    let delta = to.wall.signed_duration_since(self.wall);
                    Ok(Difference::Whole(delta.num_milliseconds()))
                } else {
                    let delta = to.wall.signed_duration_since(self.wall);
                    let unit_secs_i64 = unit
                        .whole_nominal_seconds()
                        .ok_or(DateError::OutOfRange)?;
                    #[expect(
                        clippy::arithmetic_side_effects,
                        reason = "unit_secs_i64 is 604_800, 86_400, 3_600, \
                                  60, or 1 (fixed units only), never zero, so \
                                  this division never panics"
                    )]
                    let result = delta.num_seconds() / unit_secs_i64;
                    Ok(Difference::Whole(result))
                }
            }
        }
    }
}

/// Result of measuring the difference between two date/time points.
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

/// Builds a [`TimeDelta`] from `part_secs`, splitting whole seconds from
/// nanoseconds (the fraction rounded to the nearest nanosecond) and rejecting
/// overflow of either component. The single whole/sub-second split for
/// [`DateValue::apply_part`] and [`DateTimeValue::apply_part`].
fn seconds_delta(part_secs: f64) -> Result<TimeDelta, DateError> {
    let ds = DurationSeconds::try_from(part_secs)
        .map_err(|_| DateError::OutOfRange)?;
    TimeDelta::try_from(ds).map_err(|_| DateError::OutOfRange)
}

/// Shifts a civil wall-clock datetime by `n` `unit`s.
///
/// - Calendar units shift calendar days; Month and Year clamp to the last day
///   of the target month (chrono's documented behavior).
/// - Sub-day units shift the time exactly.
///
/// No zone is consulted: callers wanting DST-aware local-wall semantics route
/// through [`DateTimeValue::shift`] instead.
///
/// # Errors
///
/// - [`DateError::OutOfRange`] if the shift overflows representable bounds.
fn shift_wall(
    wall: NaiveDateTime,
    n: i64,
    unit: DurationUnit,
) -> Result<NaiveDateTime, DateError> {
    match unit {
        DurationUnit::Year => {
            let months = n.checked_mul(12).ok_or(DateError::OutOfRange)?;
            shift_months(wall, months)
        }
        DurationUnit::Month => shift_months(wall, n),
        DurationUnit::Week => {
            let days = n.checked_mul(7).ok_or(DateError::OutOfRange)?;
            let days_u64 = days.unsigned_abs();
            if days >= 0 {
                wall.checked_add_days(Days::new(days_u64))
            } else {
                wall.checked_sub_days(Days::new(days_u64))
            }
            .ok_or(DateError::OutOfRange)
        }
        DurationUnit::Day => {
            let days_u64 = n.unsigned_abs();
            if n >= 0 {
                wall.checked_add_days(Days::new(days_u64))
            } else {
                wall.checked_sub_days(Days::new(days_u64))
            }
            .ok_or(DateError::OutOfRange)
        }
        DurationUnit::Millisecond => {
            let delta = chrono::Duration::try_milliseconds(n)
                .ok_or(DateError::OutOfRange)?;
            wall.checked_add_signed(delta).ok_or(DateError::OutOfRange)
        }
        DurationUnit::Second | DurationUnit::Minute | DurationUnit::Hour => {
            let secs = unit
                .whole_nominal_seconds()
                .ok_or(DateError::OutOfRange)?
                .checked_mul(n)
                .ok_or(DateError::OutOfRange)?;
            let delta = chrono::Duration::try_seconds(secs)
                .ok_or(DateError::OutOfRange)?;
            wall.checked_add_signed(delta).ok_or(DateError::OutOfRange)
        }
    }
}

/// Shifts `wall` by `months` calendar months (positive or negative).
///
/// Clamps to the last day of the target month per chrono's documented behavior.
///
/// # Errors
///
/// - [`DateError::OutOfRange`] if `months` does not fit a `u32` or the
///   arithmetic overflows representable bounds.
fn shift_months(
    wall: NaiveDateTime,
    months: i64,
) -> Result<NaiveDateTime, DateError> {
    let months_u32 = u32::try_from(months.unsigned_abs())
        .map_err(|_| DateError::OutOfRange)?;
    if months >= 0 {
        wall.checked_add_months(Months::new(months_u32))
    } else {
        wall.checked_sub_months(Months::new(months_u32))
    }
    .ok_or(DateError::OutOfRange)
}

/// Whole calendar years from `from` to `to`, signed.
///
/// Delegates to chrono's [`NaiveDate::years_since`], which is day-of-year
/// aware: a year is not "up" until `to`'s month/day reaches `from`'s. This
/// wrapper accepts either ordering.
fn signed_years_since(from: NaiveDate, to: NaiveDate) -> i64 {
    let (earlier, later, sign) = if to >= from {
        (from, to, 1)
    } else {
        (to, from, -1)
    };
    #[expect(
        clippy::expect_used,
        reason = "earlier/later are ordered by construction just above, so \
                  years_since's None case (base > self) is unreachable here"
    )]
    let years = later.years_since(earlier).expect(
        "later >= earlier by construction, so years_since can't return None",
    );
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "sign is always ±1 and years is bounded by NaiveDate's \
                  representable range (~±262,000), so this multiply can't \
                  overflow i64"
    )]
    let result = sign * i64::from(years);
    result
}

/// Whole calendar months from `from` to `to`, signed. See
/// [`signed_years_since`].
///
/// Chrono has no `months_since` equivalent, so this mirrors
/// [`NaiveDate::years_since`]'s algorithm at month granularity: total calendar
/// months between the dates, minus one when the day-of-month has not yet been
/// reached.
fn signed_months_since(from: NaiveDate, to: NaiveDate) -> i64 {
    let (earlier, later, sign) = if to >= from {
        (from, to, 1)
    } else {
        (to, from, -1)
    };
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "year/month/day are all bounded by NaiveDate's representable \
                  range (~±262,000 years), so the year subtraction, ×12 month \
                  conversion, day comparison, and sign multiply can't \
                  overflow i64"
    )]
    let result = sign
        * (i64::from(later.year() - earlier.year()) * 12
            + i64::from(later.month())
            - i64::from(earlier.month())
            - i64::from(later.day() < earlier.day()));
    result
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::{super::value::Precision, *};
    use crate::{date::fixtures::fixed_datetime, test_support::TzGuard};

    mod calendar_owner {
        use pretty_assertions::{assert_eq, assert_ne};
        use rstest::rstest;

        use super::*;

        #[test]
        fn apply_advances_the_instant_by_the_duration() {
            let base = fixed_datetime();
            let one_hour = DurationValue::parse("1h").expect("valid duration");
            let forward = base.apply(&one_hour).expect("no overflow");
            assert_eq!(
                forward,
                DateTimeValue(base.into_inner() + TimeDelta::hours(1))
            );
        }

        #[test]
        fn apply_rewinds_the_instant_by_negated_duration() {
            let base = fixed_datetime();
            let one_hour = DurationValue::parse("1h").expect("valid duration");
            let back = base.apply(&(one_hour * -1.0)).expect("no overflow");
            assert_eq!(
                back,
                DateTimeValue(base.into_inner() - TimeDelta::hours(1))
            );
        }

        #[test]
        fn overflows_near_the_representable_range_in_apply() {
            let near_max = DateTimeValue(DateTime::<Utc>::MAX_UTC);
            let one_second =
                DurationValue::parse("1s").expect("valid duration");
            assert_eq!(near_max.apply(&one_second).ok(), None);
        }

        #[test]
        fn underflows_near_the_representable_range_in_apply() {
            let near_min = DateTimeValue(DateTime::<Utc>::MIN_UTC);
            let one_second =
                DurationValue::parse("1s").expect("valid duration");
            assert_eq!(near_min.apply(&(one_second * -1.0)).ok(), None);
        }

        #[test]
        fn shift_preserves_the_local_wall_clock_for_calendar_units_across_dst()
        {
            TzGuard::set("America/New_York");

            // Saturday March 7, 2026 at 12:00 EST (UTC 17:00:00)
            let base = DateTimeValue::parse_iso("2026-03-07 12:00:00").unwrap();

            // A one-day `shift` keeps 12:00 on the wall clock across the gap.
            let shifted_day = base.shift(1, DurationUnit::Day).unwrap();
            assert_eq!(
                shifted_day.local_wall().unwrap().to_string(),
                "2026-03-08 12:00:00"
            );

            // Week and 7 days share the calendar path, so both keep 12:00.
            let shifted_week = base.shift(1, DurationUnit::Week).unwrap();
            let shifted_7d = base.shift(7, DurationUnit::Day).unwrap();
            assert_eq!(shifted_week, shifted_7d);
            assert_eq!(
                shifted_week.local_wall().unwrap().to_string(),
                "2026-03-14 12:00:00"
            );

            // 168 hours shifts the instant and lands 13:00 across the gap.
            let shifted_168h = base.shift(168, DurationUnit::Hour).unwrap();
            assert_ne!(shifted_week, shifted_168h);
            assert_eq!(
                shifted_168h.local_wall().unwrap().to_string(),
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
            let base = DateValue::parse_iso("2026-01-15").unwrap();
            let shifted_month = base.apply(&one_month).unwrap();
            let shifted_days = base.apply(&thirty_days).unwrap();

            assert_eq!(
                shifted_month,
                DateValue::parse_iso("2026-02-15").unwrap()
            );
            assert_eq!(
                shifted_days,
                DateValue::parse_iso("2026-02-14").unwrap()
            );
            assert_ne!(shifted_month, shifted_days);
        }

        #[test]
        fn month_clamping_matches_chrono_documented_behavior() {
            // Chrono's `checked_add_months` clamps to the target month's last
            // day; the calendar owner delegates to it, so the clamp is pinned
            // here.
            let jan_31_non_leap = DateValue::parse_iso("2023-01-31").unwrap();
            let feb_clamped =
                jan_31_non_leap.shift(1, DurationUnit::Month).unwrap();
            assert_eq!(
                feb_clamped,
                DateValue::parse_iso("2023-02-28").unwrap()
            );

            let jan_31_leap = DateValue::parse_iso("2024-01-31").unwrap();
            let feb_leap_clamped =
                jan_31_leap.shift(1, DurationUnit::Month).unwrap();
            assert_eq!(
                feb_leap_clamped,
                DateValue::parse_iso("2024-02-29").unwrap()
            );
        }

        #[test]
        fn day_is_calendar_application_unit_differing_from_24h_across_dst() {
            TzGuard::set("America/New_York");

            // 1d == 24h as duration values
            let one_day = DurationValue::parse("1d").unwrap();
            let twenty_four_hours = DurationValue::parse("24h").unwrap();
            assert_eq!(one_day, twenty_four_hours);

            // Saturday March 7, 2026 at 12:00:00 EST (UTC 17:00:00)
            let base = DateTimeValue::parse_iso("2026-03-07 12:00:00").unwrap();

            // 1d shifts local wall clock to Sunday March 8, 2026 at 12:00:00
            // EDT (UTC 16:00:00)
            let shifted_day = base.apply(&one_day).unwrap();
            assert_eq!(
                shifted_day.local_wall().unwrap().to_string(),
                "2026-03-08 12:00:00"
            );
            assert_eq!(
                shifted_day,
                DateTimeValue::parse_iso("2026-03-08T16:00:00Z").unwrap()
            );

            // 24h shifts stored UTC instant by 24 hours -> Sunday March 8 at
            // 13:00:00 EDT (UTC 17:00:00)
            let shifted_hours = base.apply(&twenty_four_hours).unwrap();
            assert_eq!(
                shifted_hours.local_wall().unwrap().to_string(),
                "2026-03-08 13:00:00"
            );
            assert_eq!(
                shifted_hours,
                DateTimeValue::parse_iso("2026-03-08T17:00:00Z").unwrap()
            );

            // Equal values, different shifts across DST
            assert_ne!(shifted_day, shifted_hours);
        }

        #[test]
        fn week_shifts_as_seven_wall_days_matching_7d_and_differing_from_168h()
        {
            TzGuard::set("America/New_York");

            let one_week = DurationValue::parse("1w").unwrap();
            let seven_days = DurationValue::parse("7d").unwrap();
            let hours_168 = DurationValue::parse("168h").unwrap();

            assert_eq!(one_week, seven_days);
            assert_eq!(one_week, hours_168);

            // Saturday March 7, 2026 at 12:00:00 EST
            let base = DateTimeValue::parse_iso("2026-03-07 12:00:00").unwrap();

            let shifted_week = base.apply(&one_week).unwrap();
            let shifted_7d = base.apply(&seven_days).unwrap();
            let shifted_168h = base.apply(&hours_168).unwrap();

            // The `1w` and `7d` forms shift identically.
            assert_eq!(shifted_week, shifted_7d);
            assert_eq!(
                shifted_week.local_wall().unwrap().to_string(),
                "2026-03-14 12:00:00"
            );

            // The `1w` and `168h` forms differ across the DST transition.
            assert_ne!(shifted_week, shifted_168h);
            assert_eq!(
                shifted_168h.local_wall().unwrap().to_string(),
                "2026-03-14 13:00:00"
            );
        }

        #[test]
        fn apply_processes_parts_left_to_right_across_dst_transition() {
            TzGuard::set("America/New_York");

            // Saturday March 7, 2026 at 02:30:00 EST (UTC 07:30:00)
            let base = DateTimeValue::parse_iso("2026-03-07 02:30:00").unwrap();

            let d_1d_1h = DurationValue::parse("1d 1h").unwrap();
            let d_1h_1d = DurationValue::parse("1h 1d").unwrap();

            // Values are equal in duration seconds
            assert_eq!(d_1d_1h, d_1h_1d);

            // Left-to-right application:
            // "1d 1h": 1d lands in the spring-forward gap (Sunday 02:30 shifts
            // to 03:30 EDT = 07:30 UTC), then 1h advances the instant to
            // 08:30 UTC (04:30 EDT).
            let res_1d_1h = base.apply(&d_1d_1h).unwrap();
            assert_eq!(
                res_1d_1h,
                DateTimeValue::parse_iso("2026-03-08T08:30:00Z").unwrap()
            );
            assert_eq!(
                res_1d_1h.local_wall().unwrap().to_string(),
                "2026-03-08 04:30:00"
            );

            // "1h 1d": 1h advances the Saturday instant to 03:30 EST
            // (08:30 UTC), then 1d shifts the wall clock to Sunday 03:30 EDT
            // (07:30 UTC).
            let res_1h_1d = base.apply(&d_1h_1d).unwrap();
            assert_eq!(
                res_1h_1d,
                DateTimeValue::parse_iso("2026-03-08T07:30:00Z").unwrap()
            );
            assert_eq!(
                res_1h_1d.local_wall().unwrap().to_string(),
                "2026-03-08 03:30:00"
            );

            // Different written order yields different result
            assert_ne!(res_1d_1h, res_1h_1d);
        }

        #[test]
        fn fractional_calendar_part_keeps_the_local_wall_clock_across_dst() {
            TzGuard::set("America/New_York");

            // Saturday March 7, 2026 at 06:00 EST (UTC 11:00:00)
            let base = DateTimeValue::parse_iso("2026-03-07 06:00:00").unwrap();

            // 1.5d == 36h == 1d 12h as values (129,600 fixed seconds)
            let one_and_half_days = DurationValue::parse("1.5d").unwrap();
            let thirty_six_hours = DurationValue::parse("36h").unwrap();
            let written_order = DurationValue::parse("1d 12h").unwrap();
            assert_eq!(one_and_half_days, thirty_six_hours);
            assert_eq!(one_and_half_days, written_order);

            // The fractional day is a calendar-unit remainder, so it keeps
            // the local wall clock: 1d lands Sunday 06:00 EDT, then the
            // 0.5-day remainder adds 12 wall hours to Sunday 18:00 EDT.
            let shifted_fractional = base.apply(&one_and_half_days).unwrap();
            assert_eq!(
                shifted_fractional.local_wall().unwrap().to_string(),
                "2026-03-08 18:00:00"
            );
            assert_eq!(
                shifted_fractional,
                DateTimeValue::parse_iso("2026-03-08T22:00:00Z").unwrap()
            );

            // The fractional spelling decomposes exactly like its written-order
            // equivalent "1d 12h".
            assert_eq!(shifted_fractional, base.apply(&written_order).unwrap());

            // 36h shifts the stored instant exactly and lands an hour later
            // on the wall clock across the 2026-03-08 gap: equal duration
            // values need not shift a date-time identically across a DST
            // boundary.
            let shifted_exact = base.apply(&thirty_six_hours).unwrap();
            assert_ne!(shifted_fractional, shifted_exact);
            assert_eq!(
                shifted_exact.local_wall().unwrap().to_string(),
                "2026-03-08 19:00:00"
            );
        }

        #[test]
        fn date_value_civil_arithmetic_handles_sub_day_units() {
            let base = DateValue::parse_iso("2026-07-29").unwrap();

            // Sub-day 1 hour shift is a no-op on date (stays 2026-07-29)
            let one_hour = DurationValue::parse("1h").unwrap();
            assert_eq!(base.apply(&one_hour).unwrap(), base);

            // 24 hour shift advances the civil date
            let twenty_four_hours = DurationValue::parse("24h").unwrap();
            assert_eq!(
                base.apply(&twenty_four_hours).unwrap(),
                DateValue::parse_iso("2026-07-30").unwrap()
            );
        }

        #[test]
        fn diff_splits_the_fixed_unit_frame_by_time_component_presence() {
            let dt1 = DateTimeValue::parse_iso("2026-07-29T10:00:00Z").unwrap();
            let dt2 = DateTimeValue::parse_iso("2026-07-29T11:30:00Z").unwrap();
            let point_of = |value: &DateTimeValue, has_time: bool| {
                let precision = if has_time {
                    Precision::DateTime
                } else {
                    Precision::Date
                };
                DatePoint::new(
                    value.wall_or_utc(),
                    value.into_inner(),
                    precision,
                )
            };

            // When both points carry a time component, fixed units return
            // exact f64:
            let diff_hours = point_of(&dt1, true)
                .diff(point_of(&dt2, true), DurationUnit::Hour)
                .unwrap();
            assert_eq!(diff_hours, Difference::Fractional(1.5));

            // When either point is date-only, fixed units return whole i64:
            let diff_civil = point_of(&dt1, true)
                .diff(point_of(&dt2, false), DurationUnit::Hour)
                .unwrap();
            assert_eq!(diff_civil, Difference::Whole(1));

            // Calendar units return whole counts:
            let d1 = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
            let d2 = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
            let diff_months = DatePoint::new(
                d1.and_hms_opt(0, 0, 0).unwrap(),
                dt1.into_inner(),
                Precision::Date,
            )
            .diff(
                DatePoint::new(
                    d2.and_hms_opt(0, 0, 0).unwrap(),
                    dt2.into_inner(),
                    Precision::Date,
                ),
                DurationUnit::Month,
            )
            .unwrap();
            assert_eq!(diff_months, Difference::Whole(2));
        }

        #[test]
        fn shift_preserves_the_local_wall_clock_for_months_and_years_across_dst()
         {
            TzGuard::set("America/New_York");

            // Saturday March 7, 2026 at 12:00 EST: month and year shifts
            // preserve the wall clock, re-resolving through the local zone
            // after the chrono arithmetic.
            let base = DateTimeValue::parse_iso("2026-03-07 12:00:00").unwrap();
            let shifted_month = base.shift(1, DurationUnit::Month).unwrap();
            assert_eq!(
                shifted_month.local_wall().unwrap().to_string(),
                "2026-04-07 12:00:00"
            );
            let shifted_year = base.shift(1, DurationUnit::Year).unwrap();
            assert_eq!(
                shifted_year.local_wall().unwrap().to_string(),
                "2027-03-07 12:00:00"
            );

            // Month-end clamping also re-resolves locally: Jan 31 noon in
            // EST clamps to Feb 28 noon, still reading 12:00.
            let january = DateTimeValue::parse_iso("2026-01-31 12:00:00")
                .expect("valid instant");
            let clamped = january.shift(1, DurationUnit::Month).unwrap();
            assert_eq!(
                clamped.local_wall().unwrap().to_string(),
                "2026-02-28 12:00:00"
            );
        }

        #[test]
        fn apply_applies_a_fractional_calendar_part_to_a_civil_date() {
            // 1.5d: the whole day shifts the civil calendar; the 12-hour
            // remainder lands on the resulting civil day (midnight + 36h).
            let base = DateValue::parse_iso("2026-03-07").unwrap();
            let shifted =
                base.apply(&DurationValue::parse("1.5d").unwrap()).unwrap();
            assert_eq!(shifted, DateValue::parse_iso("2026-03-08").unwrap());

            // 1.5mo: the whole month clamps Jan 31 to Feb 28, then the
            // 15-day remainder (half of the nominal 30-day month) applies
            // as exact civil time.
            let january = DateValue::parse_iso("2026-01-31").unwrap();
            let clamped =
                january.apply(&DurationValue::parse("1.5mo").unwrap()).unwrap();
            assert_eq!(clamped, DateValue::parse_iso("2026-03-15").unwrap());
        }

        #[rstest]
        #[case::negative_week(-1, DurationUnit::Week, "2026-07-22")]
        #[case::negative_year(-1, DurationUnit::Year, "2025-07-29")]
        #[case::sub_day_unit_leaves_the_civil_date_intact(
            500,
            DurationUnit::Millisecond,
            "2026-07-29"
        )]
        fn shift_civil_date_by_signed_units(
            #[case] n: i64,
            #[case] unit: DurationUnit,
            #[case] expected: &str,
        ) {
            let base = DateValue::parse_iso("2026-07-29").unwrap();
            let shifted = base.shift(n, unit).unwrap();
            assert_eq!(
                shifted,
                DateValue::parse_iso(expected).expect("expected date parses")
            );
        }

        #[test]
        fn point_shift_clamps_date_and_keeps_sub_day_precision() {
            // Arrange
            let jan =
                DateValue::classify("2026-01-31").unwrap().unwrap().point();
            let july =
                DateValue::classify("2026-07-29").unwrap().unwrap().point();

            // Act
            let feb = jan.shift(1, DurationUnit::Month).unwrap();
            let same_day = july.shift(5, DurationUnit::Hour).unwrap();

            // Assert
            assert_eq!(
                feb.wall.date(),
                NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
            );
            assert_eq!(feb.precision, Precision::Date);
            assert_eq!(
                same_day.wall.date(),
                NaiveDate::from_ymd_opt(2026, 7, 29).unwrap()
            );
        }

        #[test]
        fn point_calendar_shift_preserves_noon_across_dst() {
            // Arrange
            TzGuard::set("America/New_York");
            let point = DateValue::classify("2026-03-07T12:00:00")
                .unwrap()
                .unwrap()
                .point();

            // Act
            let shifted = point.shift(1, DurationUnit::Day).unwrap();

            // Assert
            assert_eq!(shifted.wall.to_string(), "2026-03-08 12:00:00");
        }

        #[test]
        fn zero_day_shift_re_resolves_fold_to_earlier_occurrence() {
            // Arrange
            TzGuard::set("America/New_York");
            let point = DateValue::classify("2026-11-01T06:30:00Z")
                .unwrap()
                .unwrap()
                .point();

            // Act: calendar shifts re-resolve through the earliest-occurrence
            // policy.
            let shifted = point.shift(0, DurationUnit::Day).unwrap();

            // Assert
            assert_eq!(
                shifted.instant,
                DateTimeValue::parse_iso("2026-11-01T05:30:00Z")
                    .unwrap()
                    .into_inner()
            );
        }

        #[test]
        fn zero_day_shift_in_utc_keeps_the_instant() {
            // Arrange
            TzGuard::set("UTC");
            let point = DateValue::classify("2026-07-29T14:30:00Z")
                .unwrap()
                .unwrap()
                .point();

            // Act
            let shifted = point.shift(0, DurationUnit::Day).unwrap();

            // Assert
            assert_eq!(shifted.instant, point.instant);
        }

        #[test]
        fn shift_reports_out_of_range_at_date_and_instant_limits() {
            // Arrange
            TzGuard::set("UTC");
            let instant = DateTimeValue(DateTime::<Utc>::MAX_UTC);
            let date = DateValue::from(NaiveDate::MAX);

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
            let date = DateValue::parse_iso("2026-07-29").unwrap();
            let separate = DurationValue::parse("12h 12h").unwrap();
            let together = DurationValue::parse("24h").unwrap();

            // Act
            let separate_date = date.apply(&separate).unwrap();
            let together_date = date.apply(&together).unwrap();

            // Assert
            assert_eq!(separate_date, date);
            assert_eq!(
                together_date,
                DateValue::parse_iso("2026-07-30").unwrap()
            );
        }
    }

    mod parse_with_and_boundaries {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn week_boundary_returns_iso_monday_spanning_month_boundaries() {
            let sunday = DateValue::parse_iso("2026-02-01").unwrap();
            let pt = DatePoint::new(
                sunday.into_inner().and_hms_opt(0, 0, 0).unwrap(),
                DateTimeValue::from(sunday).into_inner(),
                Precision::Date,
            );
            let sow = pt.week_boundary(false).expect("sow");
            let eow = pt.week_boundary(true).expect("eow");
            assert_eq!(
                sow.wall.date(),
                DateValue::parse_iso("2026-01-26").unwrap().into_inner()
            );
            assert_eq!(
                eow.wall.date(),
                DateValue::parse_iso("2026-02-01").unwrap().into_inner()
            );
        }

        #[test]
        fn year_boundary_returns_first_and_last_day_of_year() {
            let date = DateValue::parse_iso("2026-07-29").unwrap();
            let pt = DatePoint::new(
                date.into_inner().and_hms_opt(0, 0, 0).unwrap(),
                DateTimeValue::from(date).into_inner(),
                Precision::Date,
            );
            let soy = pt.year_boundary(false).expect("soy");
            let eoy = pt.year_boundary(true).expect("eoy");
            assert_eq!(
                soy.wall.date(),
                DateValue::parse_iso("2026-01-01").unwrap().into_inner()
            );
            assert_eq!(
                eoy.wall.date(),
                DateValue::parse_iso("2026-12-31").unwrap().into_inner()
            );
        }

        #[test]
        fn weekday_point_maps_to_target_weekday_in_iso_week() {
            let wednesday = DateValue::parse_iso("2026-07-29").unwrap();
            let pt = DatePoint::new(
                wednesday.into_inner().and_hms_opt(0, 0, 0).unwrap(),
                DateTimeValue::from(wednesday).into_inner(),
                Precision::Date,
            );
            let mon = pt.weekday_point(Weekday::Mon).expect("mon");
            let sun = pt.weekday_point(Weekday::Sun).expect("sun");
            assert_eq!(
                mon.wall.date(),
                DateValue::parse_iso("2026-07-27").unwrap().into_inner()
            );
            assert_eq!(
                sun.wall.date(),
                DateValue::parse_iso("2026-08-02").unwrap().into_inner()
            );
        }

        #[test]
        fn date_point_apply_shifts_point_by_compound_duration() {
            let wednesday = DateValue::parse_iso("2026-07-29").unwrap();
            let pt = DatePoint::new(
                wednesday.into_inner().and_hms_opt(0, 0, 0).unwrap(),
                DateTimeValue::from(wednesday).into_inner(),
                Precision::Date,
            );
            let dur = DurationValue::parse("1w 2d").unwrap();
            let shifted = pt.apply(&dur).expect("apply succeeds");
            assert_eq!(
                shifted.wall.date(),
                DateValue::parse_iso("2026-08-07").unwrap().into_inner()
            );
            assert_eq!(shifted.precision, Precision::Date);
        }

        #[test]
        fn date_diff_to_f64_projects_whole_and_exact_variants() {
            assert_eq!(Difference::Whole(42).to_f64(), Some(42.0));
            assert_eq!(Difference::Fractional(2.5).to_f64(), Some(2.5));
        }
    }
}

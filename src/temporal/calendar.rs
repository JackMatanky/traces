use chrono::{Days, Months, NaiveDateTime, TimeDelta, Weekday};

use super::error::DateError;
use crate::temporal::{DurationSeconds, DurationUnit};

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

/// A named calendar or clock component of a
/// [`DatePoint`](super::DatePoint).
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
pub(super) fn time_delta_from_seconds(
    part_secs: f64,
) -> Result<TimeDelta, DateError> {
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
/// through [`DateTimeValue::shift`](super::DateTimeValue::shift) instead.
///
/// # Errors
///
/// - [`DateError::OutOfRange`] if the shift overflows representable bounds.
pub(super) fn shift_datetime(
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
pub(super) fn shift_datetime_months(
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
    use super::*;

    mod date_point_boundaries {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn date_diff_to_f64_projects_whole_and_exact_variants() {
            assert_eq!(Difference::Whole(42).to_f64(), Some(42.0));
            assert_eq!(Difference::Fractional(2.5).to_f64(), Some(2.5));
        }
    }
}

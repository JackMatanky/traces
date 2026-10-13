//! Vocabulary shared by date-point arithmetic and its callers: calendar
//! periods, period edges, components, and differences.
//!
//! This module owns the names that date-point arithmetic and its query and
//! template callers share. It holds no arithmetic of its own: the calendar
//! owner modules apply [`CalendarPeriod`] and [`PeriodEdge`] at date-point
//! boundaries, resolve [`DateComponent`] names, and project [`Difference`]
//! measurements, while the free functions here supply the zone-free datetime
//! shifting and sub-second splitting those owners share.
//!
//! # Key Types
//!
//! - [`CalendarPeriod`]: a calendar period that date-point boundaries select.
//! - [`PeriodEdge`]: which side of a calendar period a boundary selects.
//! - [`DateComponent`]: a named calendar or clock component of a date point.
//! - [`Difference`]: the measured difference between two date points.

use chrono::{Days, Months, NaiveDateTime, TimeDelta, Weekday};

use super::error::DateError;
use crate::temporal::{DurationSeconds, TimeUnit};

/// A calendar period that date-point boundaries select.
///
/// Selecting a variant fixes the span a start-of or end-of boundary lands
/// on: the ISO week, the calendar month, or the calendar year.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum CalendarPeriod {
    /// The ISO week, Monday through Sunday.
    Week,
    /// The calendar month.
    Month,
    /// The calendar year.
    Year,
}

/// Which side of a calendar period a boundary selects.
///
/// Selecting a variant fixes whether a boundary lands on the first or the
/// last day of the period.
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
    /// Resolves a component name to its [`DateComponent`].
    ///
    /// Matching is ASCII case-insensitive across the eight registered names
    /// (`year`, `month`, `day`, `hour`, `minute`, `second`, `weekday`,
    /// `week`); any other string returns [`None`].
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
    /// Projects the measured difference as an `f64`.
    ///
    /// Whole magnitudes convert through the standard integer-to-floating
    /// conversion; fractional magnitudes pass through unchanged, non-finite
    /// values included.
    #[inline]
    #[must_use]
    pub(crate) fn to_f64(self) -> Option<f64> {
        match self {
            Self::Whole(n) => num_traits::ToPrimitive::to_f64(&n),
            Self::Fractional(n) => Some(n),
        }
    }
}

/// Maps an ISO weekday number to its [`Weekday`], returning [`None`] for
/// anything outside `1..=7`.
///
/// `0` is rejected because Sunday-based conventions disagree on it (Sunday
/// is `0` in some and `1` in others).
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
/// the fraction rounded to the nearest nanosecond.
///
/// The split lets calendar arithmetic keep whole days while the remainder
/// carries the sub-second part for `DatePoint::apply`.
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

/// Shifts a calendar datetime by `n` in the chosen `unit` and returns the
/// result.
///
/// Calendar units shift calendar days; month and year shifts clamp to the
/// last day of the target month (the documented `chrono` behavior). Exact
/// units shift elapsed time, so the clock reading moves by the same amount
/// regardless of the calendar.
///
/// No zone is consulted: callers wanting DST-aware local-clock semantics
/// route through
/// [`DateTimeValue::shift`](super::DateTimeValue::shift) instead.
///
/// # Errors
///
/// - [`DateError::OutOfRange`] if the shift overflows representable bounds.
pub(super) fn shift_datetime(
    local_datetime: NaiveDateTime,
    n: i64,
    unit: TimeUnit,
) -> Result<NaiveDateTime, DateError> {
    match unit {
        TimeUnit::Year => {
            let months = n.checked_mul(12).ok_or(DateError::OutOfRange)?;
            shift_datetime_months(local_datetime, months)
        }
        TimeUnit::Month => shift_datetime_months(local_datetime, n),
        TimeUnit::Week => {
            let days = n.checked_mul(7).ok_or(DateError::OutOfRange)?;
            let days_u64 = days.unsigned_abs();
            if days >= 0 {
                local_datetime.checked_add_days(Days::new(days_u64))
            } else {
                local_datetime.checked_sub_days(Days::new(days_u64))
            }
            .ok_or(DateError::OutOfRange)
        }
        TimeUnit::Day => {
            let days_u64 = n.unsigned_abs();
            if n >= 0 {
                local_datetime.checked_add_days(Days::new(days_u64))
            } else {
                local_datetime.checked_sub_days(Days::new(days_u64))
            }
            .ok_or(DateError::OutOfRange)
        }
        TimeUnit::Millisecond => {
            let delta = chrono::Duration::try_milliseconds(n)
                .ok_or(DateError::OutOfRange)?;
            local_datetime
                .checked_add_signed(delta)
                .ok_or(DateError::OutOfRange)
        }
        TimeUnit::Second | TimeUnit::Minute | TimeUnit::Hour => {
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

/// Shifts `local_datetime` by `months` calendar months (positive or
/// negative) and returns the result.
///
/// The result clamps to the last day of the target month, per the documented
/// `chrono` behavior.
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

    mod lookup {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::year("year", DateComponent::Year)]
        #[case::month("month", DateComponent::Month)]
        #[case::day("day", DateComponent::Day)]
        #[case::hour("hour", DateComponent::Hour)]
        #[case::minute("minute", DateComponent::Minute)]
        #[case::second("second", DateComponent::Second)]
        #[case::weekday("weekday", DateComponent::Weekday)]
        #[case::week("week", DateComponent::Week)]
        fn resolves_each_registered_component_name(
            #[case] name: &str,
            #[case] expected: DateComponent,
        ) {
            // Arrange: `name` and `expected` are the case literals.
            // Act
            let actual = DateComponent::from_name(name);
            // Assert
            assert_eq!(
                actual,
                Some(expected),
                "`{name}` should resolve to {expected:?}"
            );
        }

        #[rstest]
        #[case::upper_case("YEAR", DateComponent::Year)]
        #[case::title_case("Second", DateComponent::Second)]
        #[case::mixed_case("WeEk", DateComponent::Week)]
        #[case::upper_weekday("WEEKDAY", DateComponent::Weekday)]
        fn resolves_names_ascii_case_insensitively(
            #[case] name: &str,
            #[case] expected: DateComponent,
        ) {
            // Arrange: `name` and `expected` are the case literals.
            // Act
            let actual = DateComponent::from_name(name);
            // Assert
            assert_eq!(
                actual,
                Some(expected),
                "`{name}` should resolve case-insensitively to {expected:?}"
            );
        }

        #[rstest]
        #[case::empty("")]
        #[case::plural("years")]
        #[case::abbreviated("yr")]
        #[case::leading_space(" day")]
        #[case::trailing_space("day ")]
        #[case::compound("year-month")]
        #[case::unrecognized("quarter")]
        fn rejects_names_outside_the_registry(#[case] name: &str) {
            // Arrange: `name` is the case literal.
            // Act
            let actual = DateComponent::from_name(name);
            // Assert
            assert_eq!(
                actual, None,
                "`{name}` must not resolve to a component"
            );
        }

        #[rstest]
        #[case::monday(1, Weekday::Mon)]
        #[case::tuesday(2, Weekday::Tue)]
        #[case::wednesday(3, Weekday::Wed)]
        #[case::thursday(4, Weekday::Thu)]
        #[case::friday(5, Weekday::Fri)]
        #[case::saturday(6, Weekday::Sat)]
        #[case::sunday(7, Weekday::Sun)]
        fn maps_iso_weekday_numbers_to_weekdays(
            #[case] number: i64,
            #[case] expected: Weekday,
        ) {
            // Arrange: `number` and `expected` are the case literals.
            // Act
            let actual = weekday_from_number(number);
            // Assert
            assert_eq!(
                actual,
                Some(expected),
                "ISO weekday number {number} should map to {expected:?}"
            );
        }

        #[rstest]
        #[case::zero(0)]
        #[case::negative_one(-1)]
        #[case::eight(8)]
        #[case::i64_min(i64::MIN)]
        fn rejects_weekday_numbers_outside_the_iso_range(#[case] number: i64) {
            // Arrange: `number` is the case literal.
            // Act
            let actual = weekday_from_number(number);
            // Assert
            assert_eq!(
                actual, None,
                "weekday number {number} must not map to a weekday"
            );
        }
    }

    mod conversions {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::positive(Difference::Whole(42), 42.0)]
        #[case::negative(Difference::Whole(-7), -7.0)]
        #[case::zero(Difference::Whole(0), 0.0)]
        #[case::i64_max(
            Difference::Whole(i64::MAX),
            9_223_372_036_854_775_808.0
        )]
        #[case::i64_min(
            Difference::Whole(i64::MIN),
            -9_223_372_036_854_775_808.0
        )]
        fn projects_whole_differences_as_f64(
            #[case] difference: Difference,
            #[case] expected: f64,
        ) {
            // Arrange: `difference` and `expected` are the case literals.
            // Act
            let actual = difference.to_f64();
            // Assert
            assert_eq!(
                actual,
                Some(expected),
                "{difference:?} should project as {expected}"
            );
        }

        #[rstest]
        #[case::two_and_a_half(Difference::Fractional(2.5), 2.5)]
        #[case::negative_fraction(Difference::Fractional(-0.25), -0.25)]
        #[case::zero(Difference::Fractional(0.0), 0.0)]
        fn projects_fractional_differences_as_f64(
            #[case] difference: Difference,
            #[case] expected: f64,
        ) {
            // Arrange: `difference` and `expected` are the case literals.
            // Act
            let actual = difference.to_f64();
            // Assert
            assert_eq!(
                actual,
                Some(expected),
                "{difference:?} should project as {expected}"
            );
        }

        #[rstest]
        #[case::not_a_number(f64::NAN)]
        #[case::positive_infinity(f64::INFINITY)]
        #[case::negative_infinity(f64::NEG_INFINITY)]
        fn projects_non_finite_fractional_differences_unchanged(
            #[case] magnitude: f64,
        ) {
            // Arrange: `magnitude` is the case literal; only the fractional
            // variant can carry a non-finite magnitude.
            let difference = Difference::Fractional(magnitude);

            // Act
            let actual = difference.to_f64();

            // Assert: bits compare equal for non-finite values, where `==`
            // would report every NaN as unequal.
            assert_eq!(
                actual.map(f64::to_bits),
                Some(magnitude.to_bits()),
                "{difference:?} should project its magnitude unchanged"
            );
        }
    }
}

//! Defines vocabulary shared by date-point arithmetic and its callers: calendar
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
//! - [`CalendarPeriod`]: Represents a calendar period selected by date-point
//!   boundaries.
//! - [`PeriodEdge`]: Selects the side of a calendar period used by a boundary.
//! - [`DateComponent`]: Identifies a calendar or clock component of a date
//!   point.
//! - [`Difference`]: Stores a measured difference between two date points.

use jiff::{Span, civil};

use super::error::CalendarError;
use crate::temporal::TimeUnit;

/// Represents a calendar period that date-point boundaries select.
///
/// Selecting a variant fixes the span a start-of or end-of boundary lands on:
/// the ISO week, the calendar month, or the calendar year.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum CalendarPeriod {
    /// The ISO week, Monday through Sunday.
    Week,
    /// The calendar month.
    Month,
    /// The calendar year.
    Year,
}

/// Selects which side of a calendar period a boundary uses.
///
/// Selecting a variant fixes whether the boundary lands on the first or last
/// day of the period.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum PeriodEdge {
    /// First day of the period.
    Start,
    /// Last day of the period.
    End,
}

/// Lists every accepted period-boundary shorthand in its short and long
/// spellings.
///
/// Shared by query-filter name resolution and template-filter registration,
/// preventing the two surfaces from drifting. Names are matched
/// ASCII-case-insensitively and are unique.
pub(crate) const PERIOD_SHORTHANDS: [(&str, CalendarPeriod, PeriodEdge); 12] = [
    ("sow", CalendarPeriod::Week, PeriodEdge::Start),
    ("start_of_week", CalendarPeriod::Week, PeriodEdge::Start),
    ("eow", CalendarPeriod::Week, PeriodEdge::End),
    ("end_of_week", CalendarPeriod::Week, PeriodEdge::End),
    ("som", CalendarPeriod::Month, PeriodEdge::Start),
    ("start_of_month", CalendarPeriod::Month, PeriodEdge::Start),
    ("eom", CalendarPeriod::Month, PeriodEdge::End),
    ("end_of_month", CalendarPeriod::Month, PeriodEdge::End),
    ("soy", CalendarPeriod::Year, PeriodEdge::Start),
    ("start_of_year", CalendarPeriod::Year, PeriodEdge::Start),
    ("eoy", CalendarPeriod::Year, PeriodEdge::End),
    ("end_of_year", CalendarPeriod::Year, PeriodEdge::End),
];

/// Identifies a named calendar or clock component of a
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
    /// (`year`, `month`, `day`, `hour`, `minute`, `second`, `weekday`, `week`);
    /// any other string returns [`None`].
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

/// Stores the result of measuring the difference between two date points.
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

/// Maps an ISO weekday number to its [`civil::Weekday`], returning [`None`] for
/// anything outside `1..=7`.
///
/// `0` is rejected because Sunday-based conventions disagree on it (Sunday is
/// `0` in some and `1` in others).
#[must_use]
pub(crate) const fn weekday_from_number(n: i64) -> Option<civil::Weekday> {
    match n {
        1 => Some(civil::Weekday::Monday),
        2 => Some(civil::Weekday::Tuesday),
        3 => Some(civil::Weekday::Wednesday),
        4 => Some(civil::Weekday::Thursday),
        5 => Some(civil::Weekday::Friday),
        6 => Some(civil::Weekday::Saturday),
        7 => Some(civil::Weekday::Sunday),
        _ => None,
    }
}

/// Builds a Jiff [`Span`] from nominal seconds, rounding the fractional part to
/// the nearest nanosecond.
///
/// # Errors
///
/// - [`CalendarError::OutOfRange`] if the value cannot be represented.
pub(super) fn span_from_seconds(part_secs: f64) -> Result<Span, CalendarError> {
    super::duration::span_from_nominal_seconds(part_secs)
        .map_err(|_| CalendarError::OutOfRange)
}

/// Shifts a calendar datetime by `n` in the chosen `unit` and returns the
/// result.
///
/// Calendar units shift calendar days; month and year shifts clamp to the last
/// day of the target month. Exact units shift elapsed time, so the clock
/// reading moves by the same amount regardless of the calendar.
///
/// No zone is consulted: callers wanting DST-aware local-clock semantics route
/// through [`DateTimeValue::shift`](super::DateTimeValue::shift) instead.
///
/// # Errors
///
/// - [`CalendarError::OutOfRange`] if the shift overflows representable bounds.
pub(super) fn shift_date(
    date: civil::Date,
    n: i64,
    unit: TimeUnit,
) -> Result<civil::Date, CalendarError> {
    date.checked_add(shift_span(n, unit)?)
        .map_err(|_| CalendarError::OutOfRange)
}

pub(super) fn shift_datetime(
    local_datetime: civil::DateTime,
    n: i64,
    unit: TimeUnit,
) -> Result<civil::DateTime, CalendarError> {
    local_datetime
        .checked_add(shift_span(n, unit)?)
        .map_err(|_| CalendarError::OutOfRange)
}

fn shift_span(n: i64, unit: TimeUnit) -> Result<Span, CalendarError> {
    match unit {
        TimeUnit::Year => Span::new().try_years(n),
        TimeUnit::Month => Span::new().try_months(n),
        TimeUnit::Week => Span::new().try_weeks(n),
        TimeUnit::Day => Span::new().try_days(n),
        TimeUnit::Hour => Span::new().try_hours(n),
        TimeUnit::Minute => Span::new().try_minutes(n),
        TimeUnit::Second => Span::new().try_seconds(n),
        TimeUnit::Millisecond => Span::new().try_milliseconds(n),
    }
    .map_err(|_| CalendarError::OutOfRange)
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
        #[case::monday(1, civil::Weekday::Monday)]
        #[case::tuesday(2, civil::Weekday::Tuesday)]
        #[case::wednesday(3, civil::Weekday::Wednesday)]
        #[case::thursday(4, civil::Weekday::Thursday)]
        #[case::friday(5, civil::Weekday::Friday)]
        #[case::saturday(6, civil::Weekday::Saturday)]
        #[case::sunday(7, civil::Weekday::Sunday)]
        fn maps_iso_weekday_numbers_to_weekdays(
            #[case] number: i64,
            #[case] expected: civil::Weekday,
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
            // Arrange: non-finite fractional magnitudes are valid Difference
            // values.
            let difference = Difference::Fractional(magnitude);

            // Act
            let actual = difference.to_f64();

            // Assert
            // Bit comparison treats NaN consistently.
            assert_eq!(
                actual.map(f64::to_bits),
                Some(magnitude.to_bits()),
                "{difference:?} should project its magnitude unchanged"
            );
        }
    }

    mod shorthands {
        use std::collections::HashSet;

        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn holds_exactly_the_twelve_names() {
            // Arrange: the table is the arranged input.
            // Act
            // Assert
            assert_eq!(
                PERIOD_SHORTHANDS.len(),
                12,
                "three periods by two edges by short and long spelling"
            );
        }

        #[test]
        fn keeps_every_name_unique() {
            // Arrange
            let total = PERIOD_SHORTHANDS.len();

            // Act
            let unique: HashSet<&str> =
                PERIOD_SHORTHANDS.iter().map(|&(name, _, _)| name).collect();

            // Assert
            assert_eq!(
                unique.len(),
                total,
                "no shorthand name may resolve to two (period, edge) rows"
            );
        }

        #[rstest]
        #[case::sow("sow", CalendarPeriod::Week, PeriodEdge::Start)]
        #[case::start_of_week(
            "start_of_week",
            CalendarPeriod::Week,
            PeriodEdge::Start
        )]
        #[case::eow("eow", CalendarPeriod::Week, PeriodEdge::End)]
        #[case::end_of_week(
            "end_of_week",
            CalendarPeriod::Week,
            PeriodEdge::End
        )]
        #[case::som("som", CalendarPeriod::Month, PeriodEdge::Start)]
        #[case::start_of_month(
            "start_of_month",
            CalendarPeriod::Month,
            PeriodEdge::Start
        )]
        #[case::eom("eom", CalendarPeriod::Month, PeriodEdge::End)]
        #[case::end_of_month(
            "end_of_month",
            CalendarPeriod::Month,
            PeriodEdge::End
        )]
        #[case::soy("soy", CalendarPeriod::Year, PeriodEdge::Start)]
        #[case::start_of_year(
            "start_of_year",
            CalendarPeriod::Year,
            PeriodEdge::Start
        )]
        #[case::eoy("eoy", CalendarPeriod::Year, PeriodEdge::End)]
        #[case::end_of_year(
            "end_of_year",
            CalendarPeriod::Year,
            PeriodEdge::End
        )]
        fn maps_each_shorthand_to_its_period_edge(
            #[case] name: &str,
            #[case] expected_period: CalendarPeriod,
            #[case] expected_edge: PeriodEdge,
        ) {
            // Arrange: the expected row is declared in the case.
            // Act
            let actual = PERIOD_SHORTHANDS
                .iter()
                .find(|&&(candidate, _, _)| {
                    candidate.eq_ignore_ascii_case(name)
                })
                .map(|&(_, period, edge)| (period, edge));
            // Assert
            assert_eq!(
                actual,
                Some((expected_period, expected_edge)),
                "{name} must map to {expected_period:?}/{expected_edge:?}"
            );
        }
        #[rstest]
        #[case::week(
            CalendarPeriod::Week,
            ["end_of_week", "eow", "sow", "start_of_week"]
        )]
        #[case::month(
            CalendarPeriod::Month,
            ["end_of_month", "eom", "som", "start_of_month"]
        )]
        #[case::year(
            CalendarPeriod::Year,
            ["end_of_year", "eoy", "soy", "start_of_year"]
        )]
        fn returns_both_shorthand_lengths_for_each_edge(
            #[case] period: CalendarPeriod,
            #[case] expected: [&str; 4],
        ) {
            // Arrange: the rows of one period are the arranged input.
            // Act
            let mut names: Vec<&str> = PERIOD_SHORTHANDS
                .iter()
                .filter(|&&(_, p, _)| p == period)
                .map(|&(name, _, _)| name)
                .collect();
            names.sort_unstable();

            // Assert
            // Both edges are present once short and once long, pairing each
            // abbreviation with its full spelling.
            assert_eq!(names, expected, "{period:?} shorthand spellings");
        }
    }
}

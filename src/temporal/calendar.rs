//! Defines vocabulary shared by date-point arithmetic and its callers: calendar
//! periods and period edges.
//!
//! This module owns the names that date-point arithmetic and its query and
//! template callers share. It holds no arithmetic of its own: the calendar
//! owner modules apply [`CalendarPeriod`] and [`PeriodEdge`] at date-point
//! boundaries, while the free functions here supply the zone-free datetime
//! shifting those owners share.
//!
//! # Key Types
//!
//! - [`CalendarPeriod`]: Represents a calendar period selected by date-point
//!   boundaries.
//! - [`PeriodEdge`]: Selects the side of a calendar period used by a boundary.

use jiff::civil;

use super::error::CalendarError;

/// Represents a calendar period that date-point boundaries select.
///
/// Selecting a variant fixes the span a start-of or end-of boundary lands on:
/// the ISO week, the calendar month, or the calendar year.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum CalendarPeriod {
    /// Identifies the ISO week, Monday through Sunday.
    Week,
    /// Identifies the calendar month.
    Month,
    /// Identifies the calendar year.
    Year,
}

impl CalendarPeriod {
    /// Computes the boundary date of this calendar period for a given date.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the target date cannot be
    ///   represented.
    pub(crate) fn snap_date(
        self,
        edge: PeriodEdge,
        date: civil::Date,
    ) -> Result<civil::Date, CalendarError> {
        match self {
            Self::Week => {
                let iso = date.iso_week_date();
                let target = match edge {
                    PeriodEdge::Start => civil::Weekday::Monday,
                    PeriodEdge::End => civil::Weekday::Sunday,
                };
                civil::ISOWeekDate::new(iso.year(), iso.week(), target)
                    .map(civil::Date::from)
            }
            Self::Month => {
                civil::Date::new(date.year(), date.month(), match edge {
                    PeriodEdge::Start => 1,
                    PeriodEdge::End => date.days_in_month(),
                })
            }
            Self::Year => civil::Date::new(
                date.year(),
                match edge {
                    PeriodEdge::Start => 1,
                    PeriodEdge::End => 12,
                },
                match edge {
                    PeriodEdge::Start => 1,
                    PeriodEdge::End => 31,
                },
            ),
        }
        .map_err(|_| CalendarError::OutOfRange)
    }

    /// Aligns a date to a target weekday within its ISO week.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if the target date cannot be
    ///   represented.
    pub(crate) fn snap_weekday(
        date: civil::Date,
        target: civil::Weekday,
    ) -> Result<civil::Date, CalendarError> {
        let iso = date.iso_week_date();
        civil::ISOWeekDate::new(iso.year(), iso.week(), target)
            .map(civil::Date::from)
            .map_err(|_| CalendarError::OutOfRange)
    }
}

/// Selects which side of a calendar period a boundary uses.
///
/// Selecting a variant fixes whether the boundary lands on the first or last
/// day of the period.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum PeriodEdge {
    /// Selects the first day of the period.
    Start,
    /// Selects the last day of the period.
    End,
}

/// Lists every accepted period-boundary shorthand in its short and long
/// spellings.
///
/// Shared by query-filter name resolution and template-filter registration,
/// preventing the two surfaces from drifting. Names are matched
/// ASCII-case-insensitively and are unique.
pub(crate) const CALENDAR_PERIOD_EDGES: [(&str, CalendarPeriod, PeriodEdge);
    12] = [
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

/// Maps an ISO weekday number to its [`civil::Weekday`].
///
/// Returns [`None`] for anything outside `1..=7`. `0` is rejected because
/// Sunday-based conventions disagree on it (Sunday is `0` in some and `1` in
/// others).
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

#[cfg(test)]
mod tests {
    use super::*;

    mod period_edges {
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
                CALENDAR_PERIOD_EDGES.len(),
                12,
                "three periods by two edges by short and long spelling"
            );
        }

        #[test]
        fn keeps_every_name_unique() {
            // Arrange
            let total = CALENDAR_PERIOD_EDGES.len();

            // Act
            let unique: HashSet<&str> = CALENDAR_PERIOD_EDGES
                .iter()
                .map(|&(name, _, _)| name)
                .collect();

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
            let actual = CALENDAR_PERIOD_EDGES
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
            let mut names: Vec<&str> = CALENDAR_PERIOD_EDGES
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

    mod lookup {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

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

    mod boundaries {
        use jiff::civil::date;
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::start_of_week(
            CalendarPeriod::Week,
            PeriodEdge::Start,
            date(2026, 7, 29), // Wednesday
            date(2026, 7, 27)  // Monday
        )]
        #[case::end_of_week(
            CalendarPeriod::Week,
            PeriodEdge::End,
            date(2026, 7, 29), // Wednesday
            date(2026, 8, 2)   // Sunday
        )]
        #[case::start_of_month(
            CalendarPeriod::Month,
            PeriodEdge::Start,
            date(2026, 7, 29),
            date(2026, 7, 1)
        )]
        #[case::end_of_month(
            CalendarPeriod::Month,
            PeriodEdge::End,
            date(2026, 7, 29),
            date(2026, 7, 31)
        )]
        #[case::start_of_year(
            CalendarPeriod::Year,
            PeriodEdge::Start,
            date(2026, 7, 29),
            date(2026, 1, 1)
        )]
        #[case::end_of_year(
            CalendarPeriod::Year,
            PeriodEdge::End,
            date(2026, 7, 29),
            date(2026, 12, 31)
        )]
        fn snaps_dates_to_calendar_period_boundaries(
            #[case] period: CalendarPeriod,
            #[case] edge: PeriodEdge,
            #[case] input: civil::Date,
            #[case] expected: civil::Date,
        ) {
            // Arrange
            // Act
            let actual = period.snap_date(edge, input);
            // Assert
            assert_eq!(actual, Ok(expected));
        }

        #[test]
        fn aligns_to_target_weekday_within_iso_week() {
            // Arrange: 2026-07-29 is a Wednesday.
            let wednesday = date(2026, 7, 29);
            // Act
            let friday =
                CalendarPeriod::snap_weekday(wednesday, civil::Weekday::Friday);
            // Assert
            assert_eq!(friday, Ok(date(2026, 7, 31)));
        }
    }
}

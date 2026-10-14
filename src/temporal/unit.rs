//! Defines the closed sets of temporal units the crate accepts: duration
//! [`TimeUnit`] and date-point [`DateUnit`].
//!
//! [`TEMPORAL_UNITS`] is the only place a duration unit's accepted spellings,
//! nominal length, frame, and human names are stated; callers read those facts
//! from [`TimeUnit`] instead of keeping private copies. Calendar units
//! ([`TimeUnit::Day`], [`TimeUnit::Week`], [`TimeUnit::Month`],
//! [`TimeUnit::Year`]) take effect on the local clock; exact units
//! ([`TimeUnit::Hour`] and smaller) are exact elapsed time.
//!
//! [`DateUnit`] identifies calendar or clock components of a date reading and
//! provides projection from local datetime values.
//!
//! [`decompose`] turns a magnitude into whole-unit counts over a caller-chosen
//! unit set, and is the one decomposition routine behind both canonical
//! spelling and human duration rendering.

use jiff::{Span, civil};

use super::error::CalendarError;

/// Identifies a recognized duration unit.
///
/// Selecting a variant fixes the unit's accepted spellings, canonical short
/// spelling, nominal length in seconds, frame (calendar or exact), and human
/// names. Match on [`TimeUnit`] directly for type-safe dispatch rather than
/// converting to strings.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum TimeUnit {
    /// Thousandths of a second (`"ms"`, `"millisecond(s)"`).
    Millisecond,
    /// Seconds (`"s"`, `"sec(s)"`, `"second(s)"`).
    Second,
    /// Minutes (`"m"`, `"min(s)"`, `"minute(s)"`).
    Minute,
    /// Hours (`"h"`, `"hr(s)"`, `"hour(s)"`).
    Hour,
    /// Days (`"d"`, `"day(s)"`); a calendar unit.
    Day,
    /// Weeks (`"w"`, `"wk(s)"`, `"week(s)"`); a calendar unit.
    Week,
    /// Months (`"mo(s)"`, `"month(s)"`); a calendar unit.
    Month,
    /// Years (`"y"`, `"yr(s)"`, `"year(s)"`); a calendar unit.
    Year,
}

impl TimeUnit {
    /// Looks up a unit by its case-insensitive spelling.
    ///
    /// Scans the alias lists in [`TEMPORAL_UNITS`] with ASCII-case-insensitive
    /// equality, so every alias in the registry is accepted and nothing else
    /// is: an empty, too long, or non-ASCII input matches no alias and returns
    /// [`None`].
    #[must_use]
    pub(crate) fn from_spelling(unit: &str) -> Option<Self> {
        TEMPORAL_UNITS
            .iter()
            .find(|row| {
                row.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(unit))
            })
            .map(|row| row.unit)
    }

    /// Returns this unit's [`UnitSpec`] row.
    ///
    /// Listed by variant, so adding a [`TimeUnit`] variant with no
    /// [`TEMPORAL_UNITS`] row yet fails to compile here first.
    #[must_use]
    pub(crate) const fn spec(self) -> &'static UnitSpec {
        match self {
            Self::Year => &TEMPORAL_UNITS[0],
            Self::Month => &TEMPORAL_UNITS[1],
            Self::Week => &TEMPORAL_UNITS[2],
            Self::Day => &TEMPORAL_UNITS[3],
            Self::Hour => &TEMPORAL_UNITS[4],
            Self::Minute => &TEMPORAL_UNITS[5],
            Self::Second => &TEMPORAL_UNITS[6],
            Self::Millisecond => &TEMPORAL_UNITS[7],
        }
    }

    /// Returns the exact nominal nanoseconds per unit from the registry.
    #[must_use]
    pub(crate) const fn nominal_nanoseconds(self) -> i128 {
        self.spec().nominal_nanoseconds
    }

    /// Returns the fixed nominal seconds per unit.
    ///
    /// Calendar units ([`Self::Day`], [`Self::Week`], [`Self::Month`],
    /// [`Self::Year`]) get nominal fixed ratios for value identity, magnitude
    /// ordering, and fractional-remainder application; their whole-count
    /// date-shifting behavior belongs to the calendar owner modules.
    #[must_use]
    #[inline]
    pub(crate) const fn nominal_seconds(self) -> f64 {
        self.spec().nominal_seconds
    }

    /// Returns the singular and plural English names for human-readable
    /// durations.
    #[must_use]
    #[inline]
    pub(crate) const fn human_names(self) -> (&'static str, &'static str) {
        self.spec().names
    }

    /// Returns `true` if this unit is a calendar unit ([`Self::Day`],
    /// [`Self::Week`], [`Self::Month`], or [`Self::Year`]).
    #[inline]
    #[must_use]
    pub(crate) const fn is_calendar(self) -> bool {
        self.spec().calendar
    }

    /// Constructs a single-unit [`Span`] of `n` units.
    ///
    /// # Errors
    ///
    /// - [`CalendarError::OutOfRange`] if `n` cannot be represented as a
    ///   [`Span`].
    pub(crate) fn to_span(self, n: i64) -> Result<Span, CalendarError> {
        let span = match self {
            Self::Year => Span::new().try_years(n),
            Self::Month => Span::new().try_months(n),
            Self::Week => {
                let count =
                    i32::try_from(n).map_err(|_| CalendarError::OutOfRange)?;
                Span::new().try_weeks(count)
            }
            Self::Day => {
                let count =
                    i32::try_from(n).map_err(|_| CalendarError::OutOfRange)?;
                Span::new().try_days(count)
            }
            Self::Hour => Span::new().try_hours(n),
            Self::Minute => Span::new().try_minutes(n),
            Self::Second => Span::new().try_seconds(n),
            Self::Millisecond => {
                let count =
                    i32::try_from(n).map_err(|_| CalendarError::OutOfRange)?;
                Span::new().try_milliseconds(count)
            }
        };
        span.map_err(|_| CalendarError::OutOfRange)
    }
}

/// Stores the registry facts for one time unit.
#[derive(Copy, Clone, Debug)]
pub(crate) struct UnitSpec {
    /// Identifies the unit described by this row.
    pub unit: TimeUnit,
    /// Lists every accepted lowercase input spelling.
    pub aliases: &'static [&'static str],
    /// Stores singular and plural English names for human-readable output.
    pub names: (&'static str, &'static str),
    /// Stores the fixed nominal seconds per unit.
    ///
    /// Calendar units ([`TimeUnit::Day`], [`TimeUnit::Week`],
    /// [`TimeUnit::Month`], [`TimeUnit::Year`]) use nominal fixed ratios for
    /// value identity, magnitude ordering, and fractional-remainder
    /// application; their whole-count date-shifting behavior belongs to the
    /// calendar owner modules because actual calendar lengths vary.
    pub nominal_seconds: f64,
    /// Stores the exact nominal milliseconds used by integer decomposition.
    pub nominal_milliseconds: u64,
    /// Stores the exact nominal nanoseconds per unit; the authoritative
    /// integer ratio for nominal-value identity and hashing.
    pub nominal_nanoseconds: i128,
    /// Reports whether the unit takes effect on the local clock (`true`) or as
    /// exact elapsed time (`false`).
    pub calendar: bool,
}

/// Lists the time-unit registry in largest-first order.
///
/// [`decompose`] requires caller-chosen largest-first order. A new duration
/// unit adds a row here and a variant to [`TimeUnit`]; the `invariants` tests
/// fail to compile or assert until both agree.
pub(crate) static TEMPORAL_UNITS: [UnitSpec; 8] = [
    UnitSpec {
        unit: TimeUnit::Year,
        aliases: &["y", "yr", "yrs", "year", "years"],
        names: ("year", "years"),
        nominal_seconds: 31_536_000.0,
        nominal_milliseconds: 31_536_000_000,
        nominal_nanoseconds: 365 * 86_400 * 1_000_000_000,
        calendar: true,
    },
    UnitSpec {
        unit: TimeUnit::Month,
        aliases: &["mo", "mos", "month", "months"],
        names: ("month", "months"),
        nominal_seconds: 2_592_000.0,
        nominal_milliseconds: 2_592_000_000,
        nominal_nanoseconds: 30 * 86_400 * 1_000_000_000,
        calendar: true,
    },
    UnitSpec {
        unit: TimeUnit::Week,
        aliases: &["w", "wk", "wks", "week", "weeks"],
        names: ("week", "weeks"),
        nominal_seconds: 604_800.0,
        nominal_milliseconds: 604_800_000,
        nominal_nanoseconds: 7 * 86_400 * 1_000_000_000,
        calendar: true,
    },
    UnitSpec {
        unit: TimeUnit::Day,
        aliases: &["d", "day", "days"],
        names: ("day", "days"),
        nominal_seconds: 86_400.0,
        nominal_milliseconds: 86_400_000,
        nominal_nanoseconds: 86_400 * 1_000_000_000,
        calendar: true,
    },
    UnitSpec {
        unit: TimeUnit::Hour,
        aliases: &["h", "hr", "hrs", "hour", "hours"],
        names: ("hour", "hours"),
        nominal_seconds: 3_600.0,
        nominal_milliseconds: 3_600_000,
        nominal_nanoseconds: 3_600 * 1_000_000_000,
        calendar: false,
    },
    UnitSpec {
        unit: TimeUnit::Minute,
        aliases: &["m", "min", "mins", "minute", "minutes"],
        names: ("minute", "minutes"),
        nominal_seconds: 60.0,
        nominal_milliseconds: 60_000,
        nominal_nanoseconds: 60 * 1_000_000_000,
        calendar: false,
    },
    UnitSpec {
        unit: TimeUnit::Second,
        aliases: &["s", "sec", "secs", "second", "seconds"],
        names: ("second", "seconds"),
        nominal_seconds: 1.0,
        nominal_milliseconds: 1_000,
        nominal_nanoseconds: 1_000_000_000,
        calendar: false,
    },
    UnitSpec {
        unit: TimeUnit::Millisecond,
        aliases: &["ms", "millisecond", "milliseconds"],
        names: ("millisecond", "milliseconds"),
        nominal_seconds: 0.001,
        nominal_milliseconds: 1,
        nominal_nanoseconds: 1_000_000,
        calendar: false,
    },
];

/// Stores one whole-unit row of a magnitude decomposition.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct DecomposedPart {
    /// Identifies the unit being counted.
    unit: TimeUnit,
    /// Stores the whole-unit count; it is always at least 1.
    count: u64,
}

impl DecomposedPart {
    /// Returns the unit counted by this part.
    #[must_use]
    pub(crate) const fn unit(&self) -> TimeUnit {
        self.unit
    }

    /// Returns the positive whole-unit count.
    #[must_use]
    pub(crate) const fn count(&self) -> u64 {
        self.count
    }
}

/// Greedily decomposes a non-negative magnitude into whole-unit counts.
///
/// `total_ms` is a magnitude in milliseconds and `units` must be ordered
/// largest first, as [`TEMPORAL_UNITS`] and duration composition sets are.
/// Unit sizes are [`UnitSpec::nominal_milliseconds`], so the registry stays the
/// sole ratio table. Units larger than the remaining magnitude contribute no
/// row. Returns the rows and the sub-unit remainder in milliseconds; rendering
/// or rounding that remainder is the caller's decision.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "registered unit sizes are nonzero and decomposition maintains a \
              u64 remainder"
)]
pub(crate) fn decompose(
    total_ms: u64,
    units: &[TimeUnit],
) -> (Vec<DecomposedPart>, u64) {
    let mut parts = Vec::new();
    let mut rem = total_ms;
    for &unit in units {
        let unit_ms = whole_milliseconds(unit);
        if rem >= unit_ms {
            let count = rem / unit_ms;
            rem %= unit_ms;
            parts.push(DecomposedPart {
                unit,
                count,
            });
        }
    }
    (parts, rem)
}

/// Returns the exact nominal millisecond count for [`TimeUnit`].
fn whole_milliseconds(unit: TimeUnit) -> u64 {
    unit.spec().nominal_milliseconds
}

/// Identifies a named calendar or clock component of a date point.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DateUnit {
    /// Identifies the calendar year.
    Year,
    /// Identifies the calendar month, 1 through 12.
    Month,
    /// Identifies the day of the month, 1 through 31.
    Day,
    /// Identifies the hour of the day, 0 through 23.
    Hour,
    /// Identifies the minute of the hour, 0 through 59.
    Minute,
    /// Identifies the second of the minute, 0 through 59.
    Second,
    /// Identifies the ISO weekday number, Monday = 1 through Sunday = 7.
    Weekday,
    /// Identifies the ISO week number of the year.
    Week,
}

impl DateUnit {
    /// Resolves a component name to its [`DateUnit`].
    ///
    /// Matching is ASCII case-insensitive across the eight registered names
    /// (`year`, `month`, `day`, `hour`, `minute`, `second`, `weekday`, `week`);
    /// any other string returns [`None`].
    #[must_use]
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        const TABLE: [(&str, DateUnit); 8] = [
            ("year", DateUnit::Year),
            ("month", DateUnit::Month),
            ("day", DateUnit::Day),
            ("hour", DateUnit::Hour),
            ("minute", DateUnit::Minute),
            ("second", DateUnit::Second),
            ("weekday", DateUnit::Weekday),
            ("week", DateUnit::Week),
        ];
        TABLE
            .iter()
            .find(|(candidate, _)| name.eq_ignore_ascii_case(candidate))
            .map(|&(_, component)| component)
    }

    /// Projects the requested component value from a local civil datetime.
    ///
    /// Weekdays are ISO numbered, Monday = 1 through Sunday = 7. Weeks are ISO
    /// week numbers.
    #[must_use]
    pub(crate) fn of(self, local_datetime: civil::DateTime) -> i64 {
        match self {
            Self::Year => i64::from(local_datetime.year()),
            Self::Month => i64::from(local_datetime.month()),
            Self::Day => i64::from(local_datetime.day()),
            Self::Hour => i64::from(local_datetime.hour()),
            Self::Minute => i64::from(local_datetime.minute()),
            Self::Second => i64::from(local_datetime.second()),
            Self::Weekday => i64::from(match local_datetime.weekday() {
                civil::Weekday::Monday => 1,
                civil::Weekday::Tuesday => 2,
                civil::Weekday::Wednesday => 3,
                civil::Weekday::Thursday => 4,
                civil::Weekday::Friday => 5,
                civil::Weekday::Saturday => 6,
                civil::Weekday::Sunday => 7,
            }),
            Self::Week => i64::from(local_datetime.iso_week_date().week()),
        }
    }
}

/// Provides a human-readable hint of accepted unit spellings.
///
/// Lists one spelling per [`TEMPORAL_UNITS`] row, not every accepted alias; the
/// `invariants` tests pin that every row’s compact spelling appears here.
pub(crate) const UNIT_SPELLINGS_HINT: &str =
    "\"years\"/\"y\", \"months\"/\"mo\", \"weeks\"/\"w\", \"days\"/\"d\", \
     \"hours\"/\"h\", \"minutes\"/\"m\", \"seconds\"/\"s\", or \"ms\"";

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// Returns the quoted tokens of [`UNIT_SPELLINGS_HINT`] in order.
    fn hint_tokens() -> impl Iterator<Item = &'static str> {
        UNIT_SPELLINGS_HINT
            .split('"')
            .enumerate()
            .filter(|(i, _)| i % 2 == 1)
            .map(|(_, token)| token)
    }

    mod lookup {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::milliseconds("ms", TimeUnit::Millisecond)]
        #[case::seconds("s", TimeUnit::Second)]
        #[case::minutes("m", TimeUnit::Minute)]
        #[case::hours("h", TimeUnit::Hour)]
        #[case::days("d", TimeUnit::Day)]
        #[case::weeks("w", TimeUnit::Week)]
        #[case::months("mo", TimeUnit::Month)]
        #[case::years("y", TimeUnit::Year)]
        fn accepts_each_compact_spelling(
            #[case] input: &str,
            #[case] expected: TimeUnit,
        ) {
            // Arrange: `input` and `expected` are the case literals.
            // Act
            let actual = TimeUnit::from_spelling(input);
            // Assert
            assert_eq!(
                actual,
                Some(expected),
                "`{input}` should parse as {expected:?}"
            );
        }

        #[rstest]
        #[case::sec("sec", TimeUnit::Second)]
        #[case::secs("secs", TimeUnit::Second)]
        #[case::min("min", TimeUnit::Minute)]
        #[case::mins("mins", TimeUnit::Minute)]
        #[case::hr("hr", TimeUnit::Hour)]
        #[case::hrs("hrs", TimeUnit::Hour)]
        #[case::wk("wk", TimeUnit::Week)]
        #[case::wks("wks", TimeUnit::Week)]
        #[case::mos("mos", TimeUnit::Month)]
        #[case::yr("yr", TimeUnit::Year)]
        #[case::yrs("yrs", TimeUnit::Year)]
        fn accepts_each_long_alias(
            #[case] input: &str,
            #[case] expected: TimeUnit,
        ) {
            // Arrange: `input` and `expected` are the case literals.
            // Act
            let actual = TimeUnit::from_spelling(input);
            // Assert
            assert_eq!(
                actual,
                Some(expected),
                "`{input}` should parse as {expected:?}"
            );
        }

        #[rstest]
        #[case::millisecond("millisecond", TimeUnit::Millisecond)]
        #[case::milliseconds("milliseconds", TimeUnit::Millisecond)]
        #[case::second("second", TimeUnit::Second)]
        #[case::seconds("seconds", TimeUnit::Second)]
        #[case::minute("minute", TimeUnit::Minute)]
        #[case::minutes("minutes", TimeUnit::Minute)]
        #[case::hour("hour", TimeUnit::Hour)]
        #[case::hours("hours", TimeUnit::Hour)]
        #[case::day("day", TimeUnit::Day)]
        #[case::days("days", TimeUnit::Day)]
        #[case::week("week", TimeUnit::Week)]
        #[case::weeks("weeks", TimeUnit::Week)]
        #[case::month("month", TimeUnit::Month)]
        #[case::months("months", TimeUnit::Month)]
        #[case::year("year", TimeUnit::Year)]
        #[case::years("years", TimeUnit::Year)]
        fn accepts_each_full_name(
            #[case] input: &str,
            #[case] expected: TimeUnit,
        ) {
            // Arrange: `input` and `expected` are the case literals.
            // Act
            let actual = TimeUnit::from_spelling(input);
            // Assert
            assert_eq!(
                actual,
                Some(expected),
                "`{input}` should parse as {expected:?}"
            );
        }

        #[rstest]
        #[case::hour_letter("H", TimeUnit::Hour)]
        #[case::hour_word("HOUR", TimeUnit::Hour)]
        #[case::months_abbrev_upper("MOS", TimeUnit::Month)]
        #[case::years_abbrev_upper("YRS", TimeUnit::Year)]
        fn accepts_upper_case_spelling(
            #[case] input: &str,
            #[case] expected: TimeUnit,
        ) {
            // Arrange: `input` and `expected` are the case literals.
            // Act
            let actual = TimeUnit::from_spelling(input);
            // Assert
            assert_eq!(
                actual,
                Some(expected),
                "`{input}` should parse case-insensitively as {expected:?}"
            );
        }

        #[rstest]
        #[case::empty("")]
        #[case::unknown_word("fortnight")]
        #[case::leading_space(" h")]
        #[case::trailing_space("h ")]
        #[case::non_ascii_microsecond("µs")]
        #[case::microsecond_excluded("us")]
        #[case::nanosecond_excluded("ns")]
        #[case::sixteen_unknown_bytes("aaaaaaaaaaaaaaaa")]
        #[case::over_sixteen_bytes("aaaaaaaaaaaaaaaaa")]
        fn rejects_unrecognized_unit_spelling(#[case] input: &str) {
            // Arrange: `input` is the case literal.
            // Act
            let actual = TimeUnit::from_spelling(input);
            // Assert
            assert_eq!(actual, None, "`{input}` must not resolve to a unit");
        }

        #[rstest]
        #[case::year("year", DateUnit::Year)]
        #[case::month("month", DateUnit::Month)]
        #[case::day("day", DateUnit::Day)]
        #[case::hour("hour", DateUnit::Hour)]
        #[case::minute("minute", DateUnit::Minute)]
        #[case::second("second", DateUnit::Second)]
        #[case::weekday("weekday", DateUnit::Weekday)]
        #[case::week("week", DateUnit::Week)]
        fn resolves_each_registered_date_unit_name(
            #[case] name: &str,
            #[case] expected: DateUnit,
        ) {
            // Arrange: `name` and `expected` are the case literals.
            // Act
            let actual = DateUnit::from_name(name);
            // Assert
            assert_eq!(
                actual,
                Some(expected),
                "`{name}` should resolve to {expected:?}"
            );
        }

        #[rstest]
        #[case::upper_case("YEAR", DateUnit::Year)]
        #[case::title_case("Second", DateUnit::Second)]
        #[case::mixed_case("WeEk", DateUnit::Week)]
        #[case::upper_weekday("WEEKDAY", DateUnit::Weekday)]
        fn resolves_date_unit_names_ascii_case_insensitively(
            #[case] name: &str,
            #[case] expected: DateUnit,
        ) {
            // Arrange: `name` and `expected` are the case literals.
            // Act
            let actual = DateUnit::from_name(name);
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
        fn rejects_date_unit_names_outside_the_registry(#[case] name: &str) {
            // Arrange: `name` is the case literal.
            // Act
            let actual = DateUnit::from_name(name);
            // Assert
            assert_eq!(
                actual, None,
                "`{name}` must not resolve to a date unit"
            );
        }
    }

    mod accessors {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::millisecond(
            TimeUnit::Millisecond,
            "millisecond",
            "milliseconds"
        )]
        #[case::second(TimeUnit::Second, "second", "seconds")]
        #[case::minute(TimeUnit::Minute, "minute", "minutes")]
        #[case::hour(TimeUnit::Hour, "hour", "hours")]
        #[case::day(TimeUnit::Day, "day", "days")]
        #[case::week(TimeUnit::Week, "week", "weeks")]
        #[case::month(TimeUnit::Month, "month", "months")]
        #[case::year(TimeUnit::Year, "year", "years")]
        fn returns_human_names_for_each_unit(
            #[case] unit: TimeUnit,
            #[case] singular: &str,
            #[case] plural: &str,
        ) {
            // Arrange: `unit`, `singular`, and `plural` are the case literals.
            // Act
            let actual = unit.human_names();
            // Assert
            assert_eq!(
                actual,
                (singular, plural),
                "human names of {unit:?} must be ({singular}, {plural})"
            );
        }

        #[rstest]
        #[case::millisecond(TimeUnit::Millisecond, 0.001)]
        #[case::second(TimeUnit::Second, 1.0)]
        #[case::minute(TimeUnit::Minute, 60.0)]
        #[case::hour(TimeUnit::Hour, 3_600.0)]
        #[case::day(TimeUnit::Day, 86_400.0)]
        #[case::week(TimeUnit::Week, 7.0 * 86_400.0)]
        #[case::month(TimeUnit::Month, 30.0 * 86_400.0)]
        #[case::year(TimeUnit::Year, 365.0 * 86_400.0)]
        fn returns_nominal_seconds_for_each_unit(
            #[case] unit: TimeUnit,
            #[case] expected: f64,
        ) {
            // Arrange: `unit` and `expected` (a literal day multiple for the
            // calendar units) are the case literals.
            // Act
            let actual = unit.nominal_seconds();
            // Assert
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "nominal seconds of {unit:?} must be exactly {expected}"
            );
        }

        #[rstest]
        #[case::day(TimeUnit::Day)]
        #[case::week(TimeUnit::Week)]
        #[case::month(TimeUnit::Month)]
        #[case::year(TimeUnit::Year)]
        fn returns_true_for_calendar_units(#[case] unit: TimeUnit) {
            // Arrange: `unit` is a calendar-unit case literal.
            // Act
            let actual = unit.is_calendar();
            // Assert
            assert!(actual, "{unit:?} must be a calendar unit");
        }

        #[rstest]
        #[case::millisecond(TimeUnit::Millisecond)]
        #[case::second(TimeUnit::Second)]
        #[case::minute(TimeUnit::Minute)]
        #[case::hour(TimeUnit::Hour)]
        fn returns_false_for_exact_units(#[case] unit: TimeUnit) {
            // Arrange: `unit` is an exact-unit case literal.
            // Act
            let actual = unit.is_calendar();
            // Assert
            assert!(!actual, "{unit:?} must be an exact unit");
        }
    }

    mod invariants {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn matches_every_unit_exhaustively() {
            // Arrange: a match with no wildcard arm; adding a `TimeUnit`
            // variant stops this test compiling until the new unit is
            // classified.
            fn frame(unit: TimeUnit) -> bool {
                match unit {
                    TimeUnit::Millisecond
                    | TimeUnit::Second
                    | TimeUnit::Minute
                    | TimeUnit::Hour => false,
                    TimeUnit::Day
                    | TimeUnit::Week
                    | TimeUnit::Month
                    | TimeUnit::Year => true,
                }
            }

            // Act
            // Assert
            for row in &TEMPORAL_UNITS {
                assert_eq!(
                    frame(row.unit),
                    row.unit.is_calendar(),
                    "exhaustive match must classify {:?} the same way as \
                     is_calendar",
                    row.unit
                );
            }
        }

        #[test]
        fn keeps_every_registry_spelling_unique() {
            // Arrange
            let total: usize =
                TEMPORAL_UNITS.iter().map(|row| row.aliases.len()).sum();

            // Act
            let unique: HashSet<&str> = TEMPORAL_UNITS
                .iter()
                .flat_map(|row| row.aliases.iter().copied())
                .collect();

            // Assert
            assert_eq!(
                unique.len(),
                total,
                "no spelling may be shared between two registry rows"
            );
        }

        #[test]
        fn orders_the_registry_largest_first() {
            // Arrange: adjacent registry pairs are the arranged input.
            // Act
            // Assert
            for (larger, smaller) in
                TEMPORAL_UNITS.iter().zip(TEMPORAL_UNITS.iter().skip(1))
            {
                assert!(
                    larger.nominal_seconds > smaller.nominal_seconds,
                    "{:?} must precede the smaller {:?}; decomposition \
                     depends on descending order",
                    larger.unit,
                    smaller.unit
                );
            }
        }

        #[test]
        fn keeps_nominal_ratios_consistent_across_scales() {
            // Assert: the nanosecond ratio is authoritative; the millisecond
            // column must derive from it exactly.
            for row in &TEMPORAL_UNITS {
                assert_eq!(
                    i128::from(row.nominal_milliseconds),
                    row.nominal_nanoseconds / 1_000_000,
                    "{:?} millisecond ratio must match its nanosecond ratio",
                    row.unit
                );
            }
        }

        #[test]
        fn resolves_every_registered_spelling() {
            // Arrange: the registry itself is the arranged input.
            // Act
            // Assert
            for row in &TEMPORAL_UNITS {
                for alias in row.aliases {
                    assert_eq!(
                        TimeUnit::from_spelling(alias),
                        Some(row.unit),
                        "registry alias `{alias}` must parse as {:?}",
                        row.unit
                    );
                }
            }
        }

        #[test]
        fn covers_every_compact_spelling_in_the_error_hint() {
            // Arrange
            let hint_tokens: HashSet<&str> = hint_tokens().collect();

            // Act
            // Assert
            for row in &TEMPORAL_UNITS {
                // The first alias of every row is its compact spelling.
                let compact =
                    row.aliases.first().expect("unit row has aliases");
                assert!(
                    hint_tokens.contains(compact),
                    "UNIT_SPELLINGS_HINT must list the compact spelling \
                     `{compact}` of {:?}",
                    row.unit
                );
            }
        }

        #[test]
        fn keeps_the_error_hint_catalog_in_sync() {
            // Arrange
            let hint_tokens: HashSet<&str> = hint_tokens().collect();

            // Act
            let mut hint_variants = HashSet::new();
            for token in &hint_tokens {
                let parsed = TimeUnit::from_spelling(token);
                assert!(
                    parsed.is_some(),
                    "hint spelling `{token}` must resolve in \
                     TimeUnit::from_spelling"
                );
                hint_variants.extend(parsed);
            }

            // Assert
            for row in &TEMPORAL_UNITS {
                assert!(
                    hint_variants.contains(&row.unit),
                    "UNIT_SPELLINGS_HINT must cover {:?}",
                    row.unit
                );
            }
        }
    }

    mod decomposition {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn decomposes_whole_units_greedily_over_both_sets() {
            use crate::temporal::duration::{
                COMPOUND_FORMAT_ORDER, NOMINAL_COMPOSITION_ORDER,
            };

            // Arrange: one magnitude that fills a unit from each set.
            // `90_000_000 ms` is 1 day 1 hour; `31_596_000_000 ms` is
            // 1 year 16 hours 40 minutes.
            // Act
            let (canonical, canonical_rem) =
                decompose(90_000_000, &NOMINAL_COMPOSITION_ORDER);
            let (human, human_rem) =
                decompose(31_596_000_000, &COMPOUND_FORMAT_ORDER);

            // Assert
            assert_eq!(
                canonical,
                vec![
                    DecomposedPart {
                        unit: TimeUnit::Day,
                        count: 1
                    },
                    DecomposedPart {
                        unit: TimeUnit::Hour,
                        count: 1
                    },
                ],
                "canonical decomposition must fill day and hour rows"
            );
            assert_eq!(canonical_rem, 0);
            assert_eq!(
                human,
                vec![
                    DecomposedPart {
                        unit: TimeUnit::Year,
                        count: 1
                    },
                    DecomposedPart {
                        unit: TimeUnit::Hour,
                        count: 16
                    },
                    DecomposedPart {
                        unit: TimeUnit::Minute,
                        count: 40
                    },
                ],
                "human decomposition must skip units larger than the \
                 remaining magnitude"
            );
            assert_eq!(human_rem, 0);
        }

        #[test]
        fn leaves_the_sub_unit_remainder_to_the_caller() {
            use crate::temporal::duration::COMPOUND_FORMAT_ORDER;

            // Arrange: the magnitude is 1.5 seconds.
            // Act
            let (parts, remainder_ms) =
                decompose(1_500, &COMPOUND_FORMAT_ORDER);
            assert_eq!(
                parts,
                vec![DecomposedPart {
                    unit: TimeUnit::Second,
                    count: 1
                }],
                "the sub-second remainder must not gain a human row"
            );
            assert_eq!(remainder_ms, 500);
        }

        #[test]
        fn decomposes_a_sub_second_magnitude_only_where_ms_is_a_row() {
            use crate::temporal::duration::{
                COMPOUND_FORMAT_ORDER, NOMINAL_COMPOSITION_ORDER,
            };

            // Arrange: 999 milliseconds is below every human unit but is a
            // canonical row on its own.
            // Act
            let (canonical, canonical_rem) =
                decompose(999, &NOMINAL_COMPOSITION_ORDER);
            let (human, human_rem) = decompose(999, &COMPOUND_FORMAT_ORDER);
            // Assert
            assert_eq!(canonical, vec![DecomposedPart {
                unit: TimeUnit::Millisecond,
                count: 999,
            }],);
            assert_eq!(canonical_rem, 0);
            assert!(human.is_empty(), "human units start at seconds");
            assert_eq!(human_rem, 999);
        }
    }

    mod projection {
        use jiff::civil::date;
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::year(DateUnit::Year, 2026)]
        #[case::month(DateUnit::Month, 7)]
        #[case::day(DateUnit::Day, 29)]
        #[case::hour(DateUnit::Hour, 14)]
        #[case::minute(DateUnit::Minute, 30)]
        #[case::second(DateUnit::Second, 45)]
        #[case::weekday(DateUnit::Weekday, 3)]
        #[case::week(DateUnit::Week, 31)]
        fn projects_each_component_from_datetime(
            #[case] unit: DateUnit,
            #[case] expected: i64,
        ) {
            // Arrange: 2026-07-29 is a Wednesday (ISO weekday 3), in ISO week
            // 31.
            let dt = date(2026, 7, 29).at(14, 30, 45, 0);

            // Act
            let actual = unit.of(dt);

            // Assert
            assert_eq!(
                actual, expected,
                "{unit:?} of {dt} must project as {expected}"
            );
        }
    }
}

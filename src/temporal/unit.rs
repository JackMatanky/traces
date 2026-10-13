//! The closed set of time units the crate accepts, with one row of facts per
//! unit.
//!
//! [`UNITS`] is the only place a unit's accepted spellings, nominal length,
//! frame, and human names are stated; callers read those facts from
//! [`TimeUnit`] instead of keeping private copies. Calendar units
//! ([`TimeUnit::Day`], [`TimeUnit::Week`], [`TimeUnit::Month`],
//! [`TimeUnit::Year`]) take effect on the local clock; exact units
//! ([`TimeUnit::Hour`] and smaller) are exact elapsed time.
//!
//! [`decompose`] turns a magnitude into whole-unit counts over a caller-chosen
//! unit set, and is the one decomposition routine behind both the canonical
//! spelling and [`DurationValue::humanize`](super::DurationValue::humanize).

/// A recognized duration unit.
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

/// Every fact the crate states about one time unit.
#[derive(Copy, Clone, Debug)]
pub(crate) struct UnitSpec {
    /// The unit this row describes.
    pub unit: TimeUnit,
    /// Canonical short spelling used in synthesized duration text.
    pub spelling: &'static str,
    /// Every accepted input spelling, lowercase, [`Self::spelling`] included.
    pub aliases: &'static [&'static str],
    /// Singular and plural English names for human-readable output.
    pub names: (&'static str, &'static str),
    /// Fixed nominal seconds per unit, the sole unit-ratio table in the crate.
    ///
    /// Calendar units get nominal fixed ratios here for value identity,
    /// magnitude ordering, and fractional-remainder application; their
    /// whole-count date-shifting behavior belongs to the calendar owner
    /// modules.
    pub nominal_seconds: f64,
    /// Exact nominal milliseconds used by integer decomposition.
    pub nominal_milliseconds: u64,
    /// Whether the unit takes effect on the local clock (`true`) or as exact
    /// elapsed time (`false`).
    pub calendar: bool,
}

/// The unit registry, ordered largest unit first.
///
/// [`decompose`] and the unit sets below depend on this order. A new unit adds
/// a row here and a variant to [`TimeUnit`]; the `invariants` tests fail to
/// compile or assert until both agree.
pub(crate) static UNITS: [UnitSpec; 8] = [
    UnitSpec {
        unit: TimeUnit::Year,
        spelling: "y",
        aliases: &["y", "yr", "yrs", "year", "years"],
        names: ("year", "years"),
        nominal_seconds: 31_536_000.0,
        nominal_milliseconds: 31_536_000_000,
        calendar: true,
    },
    UnitSpec {
        unit: TimeUnit::Month,
        spelling: "mo",
        aliases: &["mo", "mos", "month", "months"],
        names: ("month", "months"),
        nominal_seconds: 2_592_000.0,
        nominal_milliseconds: 2_592_000_000,
        calendar: true,
    },
    UnitSpec {
        unit: TimeUnit::Week,
        spelling: "w",
        aliases: &["w", "wk", "wks", "week", "weeks"],
        names: ("week", "weeks"),
        nominal_seconds: 604_800.0,
        nominal_milliseconds: 604_800_000,
        calendar: true,
    },
    UnitSpec {
        unit: TimeUnit::Day,
        spelling: "d",
        aliases: &["d", "day", "days"],
        names: ("day", "days"),
        nominal_seconds: 86_400.0,
        nominal_milliseconds: 86_400_000,
        calendar: true,
    },
    UnitSpec {
        unit: TimeUnit::Hour,
        spelling: "h",
        aliases: &["h", "hr", "hrs", "hour", "hours"],
        names: ("hour", "hours"),
        nominal_seconds: 3_600.0,
        nominal_milliseconds: 3_600_000,
        calendar: false,
    },
    UnitSpec {
        unit: TimeUnit::Minute,
        spelling: "m",
        aliases: &["m", "min", "mins", "minute", "minutes"],
        names: ("minute", "minutes"),
        nominal_seconds: 60.0,
        nominal_milliseconds: 60_000,
        calendar: false,
    },
    UnitSpec {
        unit: TimeUnit::Second,
        spelling: "s",
        aliases: &["s", "sec", "secs", "second", "seconds"],
        names: ("second", "seconds"),
        nominal_seconds: 1.0,
        nominal_milliseconds: 1_000,
        calendar: false,
    },
    UnitSpec {
        unit: TimeUnit::Millisecond,
        spelling: "ms",
        aliases: &["ms", "millisecond", "milliseconds"],
        names: ("millisecond", "milliseconds"),
        nominal_seconds: 0.001,
        nominal_milliseconds: 1,
        calendar: false,
    },
];

/// Units the canonical spelling decomposes over: sub-year, largest first.
///
/// Month and Year are omitted because their nominal ratios are identities, not
/// applications: a synthesized magnitude never claims calendar meaning.
pub(crate) const CANONICAL_UNITS: [TimeUnit; 6] = [
    TimeUnit::Week,
    TimeUnit::Day,
    TimeUnit::Hour,
    TimeUnit::Minute,
    TimeUnit::Second,
    TimeUnit::Millisecond,
];

/// Units [`DurationValue::humanize`] decomposes over, largest first.
///
/// Millisecond is omitted: the sub-second remainder is rounded and rendered as
/// one tail count instead of a decomposed row.
pub(crate) const HUMAN_UNITS: [TimeUnit; 7] = [
    TimeUnit::Year,
    TimeUnit::Month,
    TimeUnit::Week,
    TimeUnit::Day,
    TimeUnit::Hour,
    TimeUnit::Minute,
    TimeUnit::Second,
];

/// One whole-unit row of a magnitude decomposition.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct DecomposedPart {
    /// The unit being counted.
    pub unit: TimeUnit,
    /// Whole units in the magnitude; always at least 1.
    pub count: u64,
}

/// Greedily decomposes a non-negative magnitude into whole-unit counts.
///
/// `total_ms` is a magnitude in milliseconds and `units` must be ordered
/// largest first, as [`UNITS`], [`CANONICAL_UNITS`], and [`HUMAN_UNITS`] are.
/// Unit sizes are [`UnitSpec::nominal_milliseconds`],
/// so the registry stays the sole ratio table. Units larger than the remaining
/// magnitude contribute no row. Returns the rows and the sub-unit remainder in
/// milliseconds; rendering or rounding that remainder is the caller's
/// decision.
#[must_use]
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

/// Returns the exact nominal millisecond count for [unit].
fn whole_milliseconds(unit: TimeUnit) -> u64 {
    unit.spec().nominal_milliseconds
}

impl TimeUnit {
    /// Looks up a unit by its case-insensitive spelling.
    ///
    /// Scans the alias lists in [`UNITS`] with ASCII-case-insensitive
    /// equality, so
    /// every alias in the registry is accepted and nothing else is: an empty,
    /// too long, or non-ASCII input matches no alias and returns [`None`].
    #[must_use]
    pub(crate) fn from_spelling(unit: &str) -> Option<Self> {
        UNITS
            .iter()
            .find(|row| {
                row.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(unit))
            })
            .map(|row| row.unit)
    }

    /// Returns this unit's [`UnitSpec`] row.
    ///
    /// Listed by variant, so adding a [`TimeUnit`] variant with no
    /// [`UNITS`] row yet fails to compile here first.
    #[must_use]
    pub(crate) const fn spec(self) -> &'static UnitSpec {
        match self {
            Self::Year => &UNITS[0],
            Self::Month => &UNITS[1],
            Self::Week => &UNITS[2],
            Self::Day => &UNITS[3],
            Self::Hour => &UNITS[4],
            Self::Minute => &UNITS[5],
            Self::Second => &UNITS[6],
            Self::Millisecond => &UNITS[7],
        }
    }

    /// Returns the canonical short spelling used in synthesized duration text.
    #[must_use]
    #[inline]
    pub(crate) const fn spelling(self) -> &'static str {
        self.spec().spelling
    }

    /// Returns the singular and plural English names for human-readable
    /// durations.
    #[must_use]
    #[inline]
    pub(crate) const fn human_names(self) -> (&'static str, &'static str) {
        self.spec().names
    }

    /// Returns the fixed nominal seconds per unit, the sole unit-ratio table
    /// in the crate.
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

    /// Returns `true` if this unit is a calendar unit ([`Self::Day`],
    /// [`Self::Week`], [`Self::Month`], or [`Self::Year`]).
    #[inline]
    #[must_use]
    pub(crate) const fn is_calendar(self) -> bool {
        self.spec().calendar
    }

    /// Returns the whole fixed seconds per unit as `i64`, derived from
    /// [`Self::nominal_seconds`].
    ///
    /// [`None`] only for [`Self::Millisecond`]; every other variant is listed
    /// explicitly, so adding a unit forces it to declare whether its ratio is
    /// a whole number of seconds.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "derives whole seconds from the sole nominal_seconds ratio \
                  definition"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "nominal_seconds for whole-second units always fit in i64"
    )]
    pub(crate) const fn whole_nominal_seconds(self) -> Option<i64> {
        match self {
            Self::Millisecond => None,
            Self::Second
            | Self::Minute
            | Self::Hour
            | Self::Day
            | Self::Week
            | Self::Month
            | Self::Year => Some(self.nominal_seconds() as i64),
        }
    }
}

/// Human-readable list of accepted unit spellings for error messages.
///
/// Lists one spelling per [`UNITS`] row, not every accepted alias; the
/// `invariants` tests pin that every row's compact spelling appears here.
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
    }

    mod accessors {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::millisecond(TimeUnit::Millisecond, "ms")]
        #[case::second(TimeUnit::Second, "s")]
        #[case::minute(TimeUnit::Minute, "m")]
        #[case::hour(TimeUnit::Hour, "h")]
        #[case::day(TimeUnit::Day, "d")]
        #[case::week(TimeUnit::Week, "w")]
        #[case::month(TimeUnit::Month, "mo")]
        #[case::year(TimeUnit::Year, "y")]
        fn returns_canonical_spelling_for_each_unit(
            #[case] unit: TimeUnit,
            #[case] expected: &str,
        ) {
            // Arrange: `unit` and `expected` are the case literals.
            // Act
            let actual = unit.spelling();
            // Assert
            assert_eq!(
                actual, expected,
                "canonical spelling of {unit:?} must stay `{expected}`"
            );
            assert_eq!(
                TimeUnit::from_spelling(actual),
                Some(unit),
                "canonical spelling `{actual}` must parse back to {unit:?}"
            );
        }

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
        #[case::millisecond(TimeUnit::Millisecond, None)]
        #[case::second(TimeUnit::Second, Some(1))]
        #[case::minute(TimeUnit::Minute, Some(60))]
        #[case::hour(TimeUnit::Hour, Some(3_600))]
        #[case::day(TimeUnit::Day, Some(86_400))]
        #[case::week(TimeUnit::Week, Some(604_800))]
        #[case::month(TimeUnit::Month, Some(2_592_000))]
        #[case::year(TimeUnit::Year, Some(31_536_000))]
        fn returns_whole_nominal_seconds_for_each_unit(
            #[case] unit: TimeUnit,
            #[case] expected: Option<i64>,
        ) {
            // Arrange: `unit` and `expected` are the case literals.
            // Act
            let actual = unit.whole_nominal_seconds();
            // Assert
            assert_eq!(
                actual, expected,
                "whole nominal seconds of {unit:?} must be {expected:?}"
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
            for row in &UNITS {
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
            let total: usize = UNITS.iter().map(|row| row.aliases.len()).sum();

            // Act
            let unique: HashSet<&str> = UNITS
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
            for (larger, smaller) in UNITS.iter().zip(UNITS.iter().skip(1)) {
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
        fn resolves_every_registered_spelling() {
            // Arrange: the registry itself is the arranged input.
            // Act
            // Assert
            for row in &UNITS {
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
            for row in &UNITS {
                let spelling = row.unit.spelling();
                assert!(
                    hint_tokens.contains(spelling),
                    "UNIT_SPELLINGS_HINT must list the compact spelling \
                     `{spelling}` of {:?}",
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
            for row in &UNITS {
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
            // Arrange: one magnitude that fills a unit from each set.
            // `90_000_000 ms` is 1 day 1 hour; `31_596_000_000 ms` is
            // 1 year 16 hours 40 minutes.
            // Act
            let (canonical, canonical_rem) =
                decompose(90_000_000, &CANONICAL_UNITS);
            let (human, human_rem) = decompose(31_596_000_000, &HUMAN_UNITS);

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
            // Arrange: 1.5 seconds is one second plus 500 milliseconds of
            // remainder. Act
            let (parts, remainder_ms) = decompose(1_500, &HUMAN_UNITS);

            // Assert
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
            // Arrange: 999 milliseconds is below every human unit but is a
            // canonical row on its own.
            // Act
            let (canonical, canonical_rem) = decompose(999, &CANONICAL_UNITS);
            let (human, human_rem) = decompose(999, &HUMAN_UNITS);

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
}

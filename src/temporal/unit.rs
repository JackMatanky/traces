//! The closed set of time units the crate accepts, with one row of facts per
//! unit.
//!
//! This module's registry is the only place a unit's accepted spellings,
//! nominal length, frame, and human names are stated; callers read those facts
//! from [`DurationUnit`] instead of keeping private copies. Calendar units
//! ([`DurationUnit::Day`], [`DurationUnit::Week`], [`DurationUnit::Month`],
//! [`DurationUnit::Year`]) take effect on the local clock; exact units
//! ([`DurationUnit::Hour`] and smaller) are exact elapsed time.

/// A recognized duration unit.
///
/// Selecting a variant fixes the unit's accepted spellings, canonical short
/// spelling, nominal length in seconds, frame (calendar or exact), and human
/// names. Match on [`DurationUnit`] directly for type-safe dispatch rather
/// than converting to strings.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum DurationUnit {
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

impl DurationUnit {
    /// Looks up a unit from its case-insensitive spelling.
    ///
    /// Accepts strings up to 16 bytes; returns [`None`] if the string is
    /// empty, exceeds 16 bytes, or does not match a registered unit spelling.
    #[must_use]
    pub(crate) fn from_spelling(unit: &str) -> Option<Self> {
        let mut buf = [0u8; 16];
        let slice = buf.get_mut(..unit.len())?;
        slice.copy_from_slice(unit.as_bytes());
        slice.make_ascii_lowercase();
        let lower = core::str::from_utf8(slice).ok()?;
        UNIT_SPELLINGS.get(lower).copied()
    }

    /// Returns the canonical short spelling used in synthesized duration text.
    #[must_use]
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::Millisecond => "ms",
            Self::Second => "s",
            Self::Minute => "m",
            Self::Hour => "h",
            Self::Day => "d",
            Self::Week => "w",
            Self::Month => "mo",
            Self::Year => "y",
        }
    }

    /// Returns the singular and plural English names for human-readable
    /// durations.
    #[must_use]
    pub(crate) const fn human_names(self) -> (&'static str, &'static str) {
        match self {
            Self::Millisecond => ("millisecond", "milliseconds"),
            Self::Second => ("second", "seconds"),
            Self::Minute => ("minute", "minutes"),
            Self::Hour => ("hour", "hours"),
            Self::Day => ("day", "days"),
            Self::Week => ("week", "weeks"),
            Self::Month => ("month", "months"),
            Self::Year => ("year", "years"),
        }
    }

    /// Returns the fixed nominal seconds per unit, the sole unit-ratio table
    /// in the crate.
    ///
    /// Calendar units ([`Self::Day`], [`Self::Week`], [`Self::Month`],
    /// [`Self::Year`]) get nominal fixed ratios here for value identity,
    /// magnitude ordering, and fractional-remainder application; their
    /// whole-count date-shifting behavior belongs to the calendar owner
    /// modules.
    #[must_use]
    pub(crate) const fn nominal_seconds(self) -> f64 {
        match self {
            Self::Millisecond => 0.001,
            Self::Second => 1.0,
            Self::Minute => 60.0,
            Self::Hour => 3_600.0,
            Self::Day => 86_400.0,
            Self::Week => 604_800.0,
            Self::Month => 2_592_000.0,
            Self::Year => 31_536_000.0,
        }
    }

    /// Returns `true` if this unit is a calendar unit ([`Self::Day`],
    /// [`Self::Week`], [`Self::Month`], or [`Self::Year`]).
    #[inline]
    #[must_use]
    pub(crate) const fn is_calendar(self) -> bool {
        matches!(self, Self::Day | Self::Week | Self::Month | Self::Year)
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
/// Kept beside [`UNIT_SPELLINGS`] as a reminder to update both when a unit or
/// spelling family is added to the registry: this lists one spelling per
/// family, not every accepted alias.
pub(crate) const UNIT_SPELLINGS_HINT: &str =
    "\"years\"/\"y\", \"months\"/\"mo\", \"weeks\"/\"w\", \"days\"/\"d\", \
     \"hours\"/\"h\", \"minutes\"/\"m\", \"seconds\"/\"s\", or \"ms\"";

/// Case-insensitive unit string to [`DurationUnit`] mapping.
///
/// Contains all accepted abbreviations and full names. Keys are stored
/// lowercase; lookup in [`DurationUnit::from_spelling`] lowercases the input
/// before matching.
static UNIT_SPELLINGS: phf::Map<&'static str, DurationUnit> = phf::phf_map! {
    "ms" => DurationUnit::Millisecond,
    "millisecond" => DurationUnit::Millisecond,
    "milliseconds" => DurationUnit::Millisecond,
    "s" => DurationUnit::Second,
    "sec" => DurationUnit::Second,
    "secs" => DurationUnit::Second,
    "second" => DurationUnit::Second,
    "seconds" => DurationUnit::Second,
    "m" => DurationUnit::Minute,
    "min" => DurationUnit::Minute,
    "mins" => DurationUnit::Minute,
    "minute" => DurationUnit::Minute,
    "minutes" => DurationUnit::Minute,
    "h" => DurationUnit::Hour,
    "hr" => DurationUnit::Hour,
    "hrs" => DurationUnit::Hour,
    "hour" => DurationUnit::Hour,
    "hours" => DurationUnit::Hour,
    "d" => DurationUnit::Day,
    "day" => DurationUnit::Day,
    "days" => DurationUnit::Day,
    "w" => DurationUnit::Week,
    "wk" => DurationUnit::Week,
    "wks" => DurationUnit::Week,
    "week" => DurationUnit::Week,
    "weeks" => DurationUnit::Week,
    "mo" => DurationUnit::Month,
    "mos" => DurationUnit::Month,
    "month" => DurationUnit::Month,
    "months" => DurationUnit::Month,
    "y" => DurationUnit::Year,
    "yr" => DurationUnit::Year,
    "yrs" => DurationUnit::Year,
    "year" => DurationUnit::Year,
    "years" => DurationUnit::Year,
};

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// Every [`DurationUnit`] variant, for suite-wide iteration; replaced by
    /// the catalog's own `ALL` when the unit table lands.
    const ALL_UNITS: [DurationUnit; 8] = [
        DurationUnit::Millisecond,
        DurationUnit::Second,
        DurationUnit::Minute,
        DurationUnit::Hour,
        DurationUnit::Day,
        DurationUnit::Week,
        DurationUnit::Month,
        DurationUnit::Year,
    ];

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
        #[case::milliseconds("ms", DurationUnit::Millisecond)]
        #[case::seconds("s", DurationUnit::Second)]
        #[case::minutes("m", DurationUnit::Minute)]
        #[case::hours("h", DurationUnit::Hour)]
        #[case::days("d", DurationUnit::Day)]
        #[case::weeks("w", DurationUnit::Week)]
        #[case::months("mo", DurationUnit::Month)]
        #[case::years("y", DurationUnit::Year)]
        fn accepts_each_compact_spelling(
            #[case] input: &str,
            #[case] expected: DurationUnit,
        ) {
            // Arrange: `input` and `expected` are the case literals.
            // Act
            let actual = DurationUnit::from_spelling(input);
            // Assert
            assert_eq!(
                actual,
                Some(expected),
                "`{input}` should parse as {expected:?}"
            );
        }

        #[rstest]
        #[case::sec("sec", DurationUnit::Second)]
        #[case::secs("secs", DurationUnit::Second)]
        #[case::min("min", DurationUnit::Minute)]
        #[case::mins("mins", DurationUnit::Minute)]
        #[case::hr("hr", DurationUnit::Hour)]
        #[case::hrs("hrs", DurationUnit::Hour)]
        #[case::wk("wk", DurationUnit::Week)]
        #[case::wks("wks", DurationUnit::Week)]
        #[case::mos("mos", DurationUnit::Month)]
        #[case::yr("yr", DurationUnit::Year)]
        #[case::yrs("yrs", DurationUnit::Year)]
        fn accepts_each_long_alias(
            #[case] input: &str,
            #[case] expected: DurationUnit,
        ) {
            // Arrange: `input` and `expected` are the case literals.
            // Act
            let actual = DurationUnit::from_spelling(input);
            // Assert
            assert_eq!(
                actual,
                Some(expected),
                "`{input}` should parse as {expected:?}"
            );
        }

        #[rstest]
        #[case::millisecond("millisecond", DurationUnit::Millisecond)]
        #[case::milliseconds("milliseconds", DurationUnit::Millisecond)]
        #[case::second("second", DurationUnit::Second)]
        #[case::seconds("seconds", DurationUnit::Second)]
        #[case::minute("minute", DurationUnit::Minute)]
        #[case::minutes("minutes", DurationUnit::Minute)]
        #[case::hour("hour", DurationUnit::Hour)]
        #[case::hours("hours", DurationUnit::Hour)]
        #[case::day("day", DurationUnit::Day)]
        #[case::days("days", DurationUnit::Day)]
        #[case::week("week", DurationUnit::Week)]
        #[case::weeks("weeks", DurationUnit::Week)]
        #[case::month("month", DurationUnit::Month)]
        #[case::months("months", DurationUnit::Month)]
        #[case::year("year", DurationUnit::Year)]
        #[case::years("years", DurationUnit::Year)]
        fn accepts_each_full_name(
            #[case] input: &str,
            #[case] expected: DurationUnit,
        ) {
            // Arrange: `input` and `expected` are the case literals.
            // Act
            let actual = DurationUnit::from_spelling(input);
            // Assert
            assert_eq!(
                actual,
                Some(expected),
                "`{input}` should parse as {expected:?}"
            );
        }

        #[rstest]
        #[case::hour_letter("H", DurationUnit::Hour)]
        #[case::hour_word("HOUR", DurationUnit::Hour)]
        #[case::months_abbrev_upper("MOS", DurationUnit::Month)]
        #[case::years_abbrev_upper("YRS", DurationUnit::Year)]
        fn accepts_upper_case_spelling(
            #[case] input: &str,
            #[case] expected: DurationUnit,
        ) {
            // Arrange: `input` and `expected` are the case literals.
            // Act
            let actual = DurationUnit::from_spelling(input);
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
            let actual = DurationUnit::from_spelling(input);
            // Assert
            assert_eq!(actual, None, "`{input}` must not resolve to a unit");
        }
    }

    mod accessors {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::millisecond(DurationUnit::Millisecond, "ms")]
        #[case::second(DurationUnit::Second, "s")]
        #[case::minute(DurationUnit::Minute, "m")]
        #[case::hour(DurationUnit::Hour, "h")]
        #[case::day(DurationUnit::Day, "d")]
        #[case::week(DurationUnit::Week, "w")]
        #[case::month(DurationUnit::Month, "mo")]
        #[case::year(DurationUnit::Year, "y")]
        fn returns_canonical_spelling_for_each_unit(
            #[case] unit: DurationUnit,
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
                DurationUnit::from_spelling(actual),
                Some(unit),
                "canonical spelling `{actual}` must parse back to {unit:?}"
            );
        }

        #[rstest]
        #[case::millisecond(
            DurationUnit::Millisecond,
            "millisecond",
            "milliseconds"
        )]
        #[case::second(DurationUnit::Second, "second", "seconds")]
        #[case::minute(DurationUnit::Minute, "minute", "minutes")]
        #[case::hour(DurationUnit::Hour, "hour", "hours")]
        #[case::day(DurationUnit::Day, "day", "days")]
        #[case::week(DurationUnit::Week, "week", "weeks")]
        #[case::month(DurationUnit::Month, "month", "months")]
        #[case::year(DurationUnit::Year, "year", "years")]
        fn returns_human_names_for_each_unit(
            #[case] unit: DurationUnit,
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
        #[case::millisecond(DurationUnit::Millisecond, 0.001)]
        #[case::second(DurationUnit::Second, 1.0)]
        #[case::minute(DurationUnit::Minute, 60.0)]
        #[case::hour(DurationUnit::Hour, 3_600.0)]
        #[case::day(DurationUnit::Day, 86_400.0)]
        #[case::week(DurationUnit::Week, 7.0 * 86_400.0)]
        #[case::month(DurationUnit::Month, 30.0 * 86_400.0)]
        #[case::year(DurationUnit::Year, 365.0 * 86_400.0)]
        fn returns_nominal_seconds_for_each_unit(
            #[case] unit: DurationUnit,
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
        #[case::millisecond(DurationUnit::Millisecond, None)]
        #[case::second(DurationUnit::Second, Some(1))]
        #[case::minute(DurationUnit::Minute, Some(60))]
        #[case::hour(DurationUnit::Hour, Some(3_600))]
        #[case::day(DurationUnit::Day, Some(86_400))]
        #[case::week(DurationUnit::Week, Some(604_800))]
        #[case::month(DurationUnit::Month, Some(2_592_000))]
        #[case::year(DurationUnit::Year, Some(31_536_000))]
        fn returns_whole_nominal_seconds_for_each_unit(
            #[case] unit: DurationUnit,
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
        #[case::day(DurationUnit::Day)]
        #[case::week(DurationUnit::Week)]
        #[case::month(DurationUnit::Month)]
        #[case::year(DurationUnit::Year)]
        fn returns_true_for_calendar_units(#[case] unit: DurationUnit) {
            // Arrange: `unit` is a calendar-unit case literal.
            // Act
            let actual = unit.is_calendar();
            // Assert
            assert!(actual, "{unit:?} must be a calendar unit");
        }

        #[rstest]
        #[case::millisecond(DurationUnit::Millisecond)]
        #[case::second(DurationUnit::Second)]
        #[case::minute(DurationUnit::Minute)]
        #[case::hour(DurationUnit::Hour)]
        fn returns_false_for_exact_units(#[case] unit: DurationUnit) {
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
            // Arrange: a match with no wildcard arm; adding a `DurationUnit`
            // variant stops this test compiling until the new unit is
            // classified.
            fn frame(unit: DurationUnit) -> bool {
                match unit {
                    DurationUnit::Millisecond
                    | DurationUnit::Second
                    | DurationUnit::Minute
                    | DurationUnit::Hour => false,
                    DurationUnit::Day
                    | DurationUnit::Week
                    | DurationUnit::Month
                    | DurationUnit::Year => true,
                }
            }
            let units = ALL_UNITS;

            // Act
            let classified: Vec<bool> =
                units.iter().copied().map(frame).collect();

            // Assert
            for (unit, is_calendar) in units.iter().zip(&classified) {
                assert_eq!(
                    *is_calendar,
                    unit.is_calendar(),
                    "exhaustive match must classify {unit:?} the same way as \
                     is_calendar"
                );
            }
        }

        #[test]
        fn assigns_a_unique_compact_spelling_per_unit() {
            // Arrange
            let units = ALL_UNITS;

            // Act
            let spellings: HashSet<&str> =
                units.iter().map(|unit| unit.spelling()).collect();

            // Assert
            assert_eq!(
                spellings.len(),
                units.len(),
                "no compact spelling may be shared between two units"
            );
        }

        #[test]
        fn resolves_every_registered_spelling() {
            // Arrange: the registry itself is the arranged input.
            // Act
            // Assert
            for (key, variant) in UNIT_SPELLINGS.entries() {
                assert_eq!(
                    DurationUnit::from_spelling(key),
                    Some(*variant),
                    "UNIT_SPELLINGS entry `{key}` must parse as {variant:?}"
                );
            }
        }

        #[test]
        fn covers_every_compact_spelling_in_the_error_hint() {
            // Arrange
            let hint_tokens: HashSet<&str> = hint_tokens().collect();

            // Act
            // Assert
            for unit in ALL_UNITS {
                assert!(
                    hint_tokens.contains(unit.spelling()),
                    "UNIT_SPELLINGS_HINT must list the compact spelling `{}` \
                     of {unit:?}",
                    unit.spelling()
                );
            }
        }

        #[test]
        fn keeps_the_error_hint_catalog_in_sync() {
            // Arrange
            let hint_tokens = hint_tokens();

            // Act
            let mut hint_variants = HashSet::new();
            for token in hint_tokens {
                let parsed = DurationUnit::from_spelling(token);
                assert!(
                    parsed.is_some(),
                    "hint spelling `{token}` must resolve in \
                     DurationUnit::from_spelling"
                );
                hint_variants.extend(parsed);
            }

            // Assert
            for variant in ALL_UNITS {
                assert!(
                    hint_variants.contains(&variant),
                    "UNIT_SPELLINGS_HINT must cover {variant:?}"
                );
            }
            for (key, variant) in UNIT_SPELLINGS.entries() {
                assert!(
                    hint_variants.contains(variant),
                    "UNIT_SPELLINGS_HINT must cover variant for key `{key}`"
                );
            }
        }
    }
}

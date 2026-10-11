/// Human-readable list of accepted unit spellings for error messages.
///
/// Kept beside [`UNIT_MAP`] as a reminder to update both when a unit or
/// spelling family is added to the registry: this lists one spelling per
/// family, not every accepted alias.
pub(crate) const UNIT_HINT: &str =
    "\"years\"/\"y\", \"months\"/\"mo\", \"weeks\"/\"w\", \"days\"/\"d\", \
     \"hours\"/\"h\", \"minutes\"/\"m\", \"seconds\"/\"s\", or \"ms\"";

/// Case-insensitive unit string to [`DurationUnit`] mapping.
///
/// Contains all accepted abbreviations and full names. Keys are stored
/// lowercase; lookup in [`DurationUnit::parse`] lowercases the input before
/// matching.
static UNIT_MAP: phf::Map<&'static str, DurationUnit> = phf::phf_map! {
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

/// A recognized duration unit.
///
/// Single source of truth for unit parsing and seconds conversion. Match on
/// [`DurationUnit`] directly for type-safe dispatch rather than converting to
/// strings.
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
    /// Days (`"d"`, `"day(s)"`); a calendar application unit.
    Day,
    /// Weeks (`"w"`, `"wk(s)"`, `"week(s)"`); a calendar application unit.
    Week,
    /// Months (`"mo(s)"`, `"month(s)"`); a calendar application unit.
    Month,
    /// Years (`"y"`, `"yr(s)"`, `"year(s)"`); a calendar application unit.
    Year,
}

impl DurationUnit {
    /// Case-insensitive lookup of a unit string.
    ///
    /// Accepts strings up to 16 bytes. Returns `None` if the string is empty,
    /// exceeds 16 bytes, or does not match a known unit name.
    #[must_use]
    pub(crate) fn parse(unit: &str) -> Option<Self> {
        let mut buf = [0u8; 16];
        let slice = buf.get_mut(..unit.len())?;
        slice.copy_from_slice(unit.as_bytes());
        slice.make_ascii_lowercase();
        let lower = core::str::from_utf8(slice).ok()?;
        UNIT_MAP.get(lower).copied()
    }

    /// Fixed seconds per unit for duration magnitude identity.
    ///
    /// The sole unit-ratio table in the crate. Calendar units (Day, Week,
    /// Month, Year) get nominal fixed ratios here for value identity,
    /// magnitude ordering, and fractional-remainder application; their
    /// whole-count date-shifting behavior belongs to the date module's
    /// calendar owner.
    #[must_use]
    pub(crate) const fn fixed_seconds(self) -> f64 {
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

    /// Returns `true` if this unit is a calendar application unit
    /// ([`Self::Day`], [`Self::Week`], [`Self::Month`], or [`Self::Year`]).
    #[cfg(test)]
    #[inline]
    #[must_use]
    pub(crate) const fn is_calendar(self) -> bool {
        matches!(self, Self::Day | Self::Week | Self::Month | Self::Year)
    }

    /// Whole fixed seconds per unit as `i64`, derived from
    /// [`Self::fixed_seconds`].
    ///
    /// `None` only for [`Self::Millisecond`]; every other variant is listed
    /// explicitly, so adding a unit forces it to declare whether its ratio is a
    /// whole number of seconds.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "derives whole seconds from the sole fixed_seconds ratio \
                  definition"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "fixed_seconds for whole-second units always fit in i64"
    )]
    pub(crate) const fn fixed_seconds_i64(self) -> Option<i64> {
        match self {
            Self::Millisecond => None,
            Self::Second
            | Self::Minute
            | Self::Hour
            | Self::Day
            | Self::Week
            | Self::Month
            | Self::Year => Some(self.fixed_seconds() as i64),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod unit {
        use super::*;

        mod parse {
            use pretty_assertions::assert_eq;
            use rstest::rstest;

            use super::*;

            #[rstest]
            #[case::single_upper("H", DurationUnit::Hour)]
            #[case::all_upper("HOUR", DurationUnit::Hour)]
            #[case::lower_plural("hours", DurationUnit::Hour)]
            fn is_case_insensitive(
                #[case] input: &str,
                #[case] expected: DurationUnit,
            ) {
                assert_eq!(DurationUnit::parse(input).unwrap(), expected);
            }

            #[rstest]
            #[case::ms("ms", DurationUnit::Millisecond)]
            #[case::millisecond("millisecond", DurationUnit::Millisecond)]
            #[case::milliseconds("milliseconds", DurationUnit::Millisecond)]
            #[case::s("s", DurationUnit::Second)]
            #[case::sec("sec", DurationUnit::Second)]
            #[case::secs("secs", DurationUnit::Second)]
            #[case::second("second", DurationUnit::Second)]
            #[case::seconds("seconds", DurationUnit::Second)]
            #[case::m("m", DurationUnit::Minute)]
            #[case::min("min", DurationUnit::Minute)]
            #[case::mins("mins", DurationUnit::Minute)]
            #[case::minute("minute", DurationUnit::Minute)]
            #[case::minutes("minutes", DurationUnit::Minute)]
            #[case::h("h", DurationUnit::Hour)]
            #[case::hr("hr", DurationUnit::Hour)]
            #[case::hrs("hrs", DurationUnit::Hour)]
            #[case::hour("hour", DurationUnit::Hour)]
            #[case::hours("hours", DurationUnit::Hour)]
            #[case::d("d", DurationUnit::Day)]
            #[case::day("day", DurationUnit::Day)]
            #[case::days("days", DurationUnit::Day)]
            #[case::w("w", DurationUnit::Week)]
            #[case::wk("wk", DurationUnit::Week)]
            #[case::wks("wks", DurationUnit::Week)]
            #[case::week("week", DurationUnit::Week)]
            #[case::weeks("weeks", DurationUnit::Week)]
            #[case::mo("mo", DurationUnit::Month)]
            #[case::mos("mos", DurationUnit::Month)]
            #[case::month("month", DurationUnit::Month)]
            #[case::months("months", DurationUnit::Month)]
            #[case::y("y", DurationUnit::Year)]
            #[case::yr("yr", DurationUnit::Year)]
            #[case::yrs("yrs", DurationUnit::Year)]
            #[case::year("year", DurationUnit::Year)]
            #[case::years("years", DurationUnit::Year)]
            fn parses_all_units_and_abbreviations(
                #[case] input: &str,
                #[case] expected: DurationUnit,
            ) {
                assert_eq!(DurationUnit::parse(input).unwrap(), expected);
            }

            #[rstest]
            #[case::unknown("foo")]
            #[case::exactly_16_bytes("aaaaaaaaaaaaaaaa")]
            #[case::longer_than_16_bytes("aaaaaaaaaaaaaaaaa")]
            #[case::empty("")]
            fn rejects_unit(#[case] input: &str) {
                assert!(DurationUnit::parse(input).is_none());
            }
            #[test]
            fn parses_all_unit_map_keys_into_their_expected_variants() {
                for (key, expected_variant) in UNIT_MAP.entries() {
                    assert_eq!(
                        DurationUnit::parse(key),
                        Some(*expected_variant),
                        "UNIT_MAP entry `{key}` must parse as \
                         `{expected_variant:?}`"
                    );
                }
            }

            #[test]
            fn verifies_unit_hint_and_unit_map_are_in_sync() {
                let mut hint_variants = std::collections::HashSet::new();
                let quoted_tokens = UNIT_HINT
                    .split('"')
                    .enumerate()
                    .filter(|(i, _)| i % 2 == 1)
                    .map(|(_, t)| t);
                for token in quoted_tokens {
                    let parsed = DurationUnit::parse(token);
                    assert!(
                        parsed.is_some(),
                        "hint spelling `{token}` must resolve in \
                         DurationUnit::parse"
                    );
                    hint_variants.extend(parsed);
                }

                let all_variants = [
                    DurationUnit::Millisecond,
                    DurationUnit::Second,
                    DurationUnit::Minute,
                    DurationUnit::Hour,
                    DurationUnit::Day,
                    DurationUnit::Week,
                    DurationUnit::Month,
                    DurationUnit::Year,
                ];
                for variant in all_variants {
                    assert!(
                        hint_variants.contains(&variant),
                        "UNIT_HINT must cover {variant:?}"
                    );
                }

                for (key, variant) in UNIT_MAP.entries() {
                    assert!(
                        hint_variants.contains(variant),
                        "UNIT_HINT must cover variant for key `{key}`"
                    );
                }
            }
        }

        mod seconds {
            use super::*;

            #[test]
            fn fixed_seconds_are_positive_for_all_variants() {
                for entry in UNIT_MAP.entries() {
                    assert!(entry.1.fixed_seconds() > 0.0);
                }
            }

            #[test]
            fn fixed_seconds_i64_matches_fixed_seconds_for_each_variant() {
                let units = [
                    (DurationUnit::Millisecond, None),
                    (DurationUnit::Second, Some(1)),
                    (DurationUnit::Minute, Some(60)),
                    (DurationUnit::Hour, Some(3_600)),
                    (DurationUnit::Day, Some(86_400)),
                    (DurationUnit::Week, Some(604_800)),
                    (DurationUnit::Month, Some(2_592_000)),
                    (DurationUnit::Year, Some(31_536_000)),
                ];
                for (unit, expected) in units {
                    assert_eq!(unit.fixed_seconds_i64(), expected);
                }
            }
        }
    }
}

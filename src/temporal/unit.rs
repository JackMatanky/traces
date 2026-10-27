//! Defines the closed sets of temporal units the crate accepts: duration
//! [`TimeUnit`] and date-point [`DateUnit`].
//!
//! [`TEMPORAL_UNITS`] stores each duration unit's nominal length, frame, and
//! human names. [`UNIT_LOOKUP`] stores accepted spellings. Callers read these
//! facts from [`TimeUnit`] instead of keeping private copies. Calendar units
//! ([`TimeUnit::Day`], [`TimeUnit::Week`], [`TimeUnit::Month`],
//! [`TimeUnit::Year`]) take effect on the local clock; exact units
//! ([`TimeUnit::Hour`] and smaller) are exact elapsed time.
//!
//! [`DateUnit`] identifies calendar or clock components of a date reading and
//! provides projection from local datetime values.
//!
//! [`decompose`] turns a magnitude in whole seconds into whole-unit counts over
//! a caller-chosen unit set, and is the one decomposition routine behind both
//! canonical spelling and human duration rendering.

use jiff::{Span, civil};

use super::error::{CalendarError, UnitError};

/// Identifies a recognized duration unit.
///
/// Selecting a variant fixes the unit's accepted spellings, canonical short
/// spelling, nominal length in seconds, frame (calendar or exact), and human
/// names. Match on [`TimeUnit`] directly for type-safe dispatch rather than
/// converting to strings.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum TimeUnit {
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

/// Maps every accepted duration unit spelling to its unit.
static UNIT_LOOKUP: phf::Map<&'static str, TimeUnit> = phf::phf_map! {
    "y" => TimeUnit::Year,
    "yr" => TimeUnit::Year,
    "yrs" => TimeUnit::Year,
    "year" => TimeUnit::Year,
    "years" => TimeUnit::Year,
    "mo" => TimeUnit::Month,
    "mos" => TimeUnit::Month,
    "month" => TimeUnit::Month,
    "months" => TimeUnit::Month,
    "w" => TimeUnit::Week,
    "wk" => TimeUnit::Week,
    "wks" => TimeUnit::Week,
    "week" => TimeUnit::Week,
    "weeks" => TimeUnit::Week,
    "d" => TimeUnit::Day,
    "day" => TimeUnit::Day,
    "days" => TimeUnit::Day,
    "h" => TimeUnit::Hour,
    "hr" => TimeUnit::Hour,
    "hrs" => TimeUnit::Hour,
    "hour" => TimeUnit::Hour,
    "hours" => TimeUnit::Hour,
    "m" => TimeUnit::Minute,
    "min" => TimeUnit::Minute,
    "mins" => TimeUnit::Minute,
    "minute" => TimeUnit::Minute,
    "minutes" => TimeUnit::Minute,
    "s" => TimeUnit::Second,
    "sec" => TimeUnit::Second,
    "secs" => TimeUnit::Second,
    "second" => TimeUnit::Second,
    "seconds" => TimeUnit::Second,
};

impl TimeUnit {
    /// Looks up a unit by its case-insensitive spelling.
    ///
    /// Uses a fixed-size ASCII lowercase buffer to avoid an allocation.
    #[must_use]
    pub(crate) fn from_spelling(unit: &str) -> Option<Self> {
        let mut buf = [0u8; 16];
        if unit.len() > buf.len() {
            return None;
        }
        for (dst, byte) in buf.iter_mut().zip(unit.as_bytes()) {
            *dst = byte.to_ascii_lowercase();
        }
        let lower = std::str::from_utf8(buf.get(..unit.len())?).ok()?;
        UNIT_LOOKUP.get(lower).copied()
    }

    /// Resolves a unit spelling or reports why it was rejected.
    ///
    /// The [`UnitError`] message quotes [`UNIT_SPELLINGS_HINT`], so a
    /// rejected `unit="..."` kwarg names every accepted spelling without a
    /// second copy of the list beside the caller.
    ///
    /// # Errors
    ///
    /// - [`UnitError::UnknownSpelling`] if `unit` matches no registered
    ///   spelling; matching is ASCII case-insensitive (via
    ///   [`Self::from_spelling`]).
    pub(crate) fn parse_spelling(unit: &str) -> Result<Self, UnitError> {
        Self::from_spelling(unit).ok_or_else(|| UnitError::UnknownSpelling {
            unit: unit.into(),
        })
    }

    /// Returns this unit's [`UnitSpec`] row.
    ///
    /// Listed by variant, so adding a [`TimeUnit`] variant with no
    /// [`TEMPORAL_UNITS`] row yet fails to compile here first.
    #[must_use]
    const fn spec(self) -> &'static UnitSpec {
        match self {
            Self::Year => &TEMPORAL_UNITS[0],
            Self::Month => &TEMPORAL_UNITS[1],
            Self::Week => &TEMPORAL_UNITS[2],
            Self::Day => &TEMPORAL_UNITS[3],
            Self::Hour => &TEMPORAL_UNITS[4],
            Self::Minute => &TEMPORAL_UNITS[5],
            Self::Second => &TEMPORAL_UNITS[6],
        }
    }

    /// Returns the exact nominal integer seconds per unit from the registry.
    #[must_use]
    #[inline]
    pub(crate) const fn nominal_seconds(self) -> i64 {
        self.spec().nominal_seconds
    }

    /// Returns the fixed nominal seconds per unit as an `f64`.
    #[must_use]
    #[inline]
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "registry nominal seconds are well within f64 mantissa"
    )]
    pub(crate) const fn nominal_seconds_f64(self) -> f64 {
        self.spec().nominal_seconds as f64
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
    pub(super) fn to_span(self, n: i64) -> Result<Span, CalendarError> {
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
        };
        span.map_err(|_| CalendarError::OutOfRange)
    }
}

/// Stores the registry facts for one time unit.
#[derive(Copy, Clone, Debug)]
struct UnitSpec {
    names: (&'static str, &'static str),
    nominal_seconds: i64,
    calendar: bool,
}

/// Lists the time-unit registry in largest-first order.
///
/// [`decompose`] requires caller-chosen largest-first order. A new duration
/// unit adds a row here and a variant to [`TimeUnit`].
static TEMPORAL_UNITS: [UnitSpec; 7] = [
    UnitSpec {
        names: ("year", "years"),
        nominal_seconds: 31_536_000,
        calendar: true,
    },
    UnitSpec {
        names: ("month", "months"),
        nominal_seconds: 2_592_000,
        calendar: true,
    },
    UnitSpec {
        names: ("week", "weeks"),
        nominal_seconds: 604_800,
        calendar: true,
    },
    UnitSpec {
        names: ("day", "days"),
        nominal_seconds: 86_400,
        calendar: true,
    },
    UnitSpec {
        names: ("hour", "hours"),
        nominal_seconds: 3_600,
        calendar: false,
    },
    UnitSpec {
        names: ("minute", "minutes"),
        nominal_seconds: 60,
        calendar: false,
    },
    UnitSpec {
        names: ("second", "seconds"),
        nominal_seconds: 1,
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

/// Greedily decomposes a non-negative magnitude in whole seconds into
/// whole-unit counts.
///
/// `total_secs` is a magnitude in whole seconds and `units` must be ordered
/// largest first, as [`TEMPORAL_UNITS`] and duration composition sets are.
/// Unit sizes are [`TimeUnit::nominal_seconds`], so the registry stays the
/// sole ratio table. Units larger than the remaining magnitude contribute no
/// row. Returns the rows and the sub-unit remainder in seconds.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "registered unit sizes are nonzero and decomposition maintains a \
              u64 remainder"
)]
pub(crate) fn decompose(
    total_secs: u64,
    units: &[TimeUnit],
) -> (Vec<DecomposedPart>, u64) {
    let mut parts = Vec::new();
    let mut rem = total_secs;
    for &unit in units {
        let unit_secs = unit.nominal_seconds().unsigned_abs();
        if rem >= unit_secs {
            let count = rem / unit_secs;
            rem %= unit_secs;
            parts.push(DecomposedPart {
                unit,
                count,
            });
        }
    }
    (parts, rem)
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
/// Lists one compact spelling per [`TEMPORAL_UNITS`] row rather than every
/// accepted alias.
pub(crate) const UNIT_SPELLINGS_HINT: &str =
    "\"years\"/\"y\", \"months\"/\"mo\", \"weeks\"/\"w\", \"days\"/\"d\", \
     \"hours\"/\"h\", \"minutes\"/\"m\", or \"seconds\"/\"s\"";

#[cfg(test)]
mod tests {

    use super::*;

    mod lookup {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
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
        #[case::hour_upper("H", TimeUnit::Hour)]
        #[case::hour_word("Hour", TimeUnit::Hour)]
        #[case::day_mixed("DaY", TimeUnit::Day)]
        #[case::days_upper("DAYS", TimeUnit::Day)]
        #[case::years_caps("YEARS", TimeUnit::Year)]
        fn accepts_case_insensitively(
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
                "`{input}` must parse as {expected:?} despite casing"
            );
        }

        #[rstest]
        #[case::millisecond_compact("ms")]
        #[case::millisecond_singular("millisecond")]
        #[case::millisecond_plural("milliseconds")]
        #[case::microsecond_compact("us")]
        #[case::nanosecond_compact("ns")]
        #[case::unknown_spelling("decades")]
        #[case::partial_match("mili")]
        #[case::empty("")]
        #[case::non_ascii_microsecond("µs")]
        #[case::trailing_space("s ")]
        fn rejects_unrecognized_unit_spelling(#[case] input: &str) {
            // Arrange: `input` is an invalid spelling.
            // Act
            let actual = TimeUnit::from_spelling(input);
            // Assert
            assert_eq!(
                actual, None,
                "`{input}` is not an accepted unit spelling and must return \
                 None"
            );
        }
    }

    mod spelling_errors {
        use pretty_assertions::assert_eq;

        use super::*;

        /// The `UnitError` message quotes [`UNIT_SPELLINGS_HINT`] verbatim,
        /// so the hint may only advertise spellings the registry accepts
        /// under its own plural name.
        #[test]
        fn unit_spellings_hint_matches_the_registry() {
            // Arrange: the hint is one `"plural"/"short"` pair per row.
            let row_units = [
                TimeUnit::Year,
                TimeUnit::Month,
                TimeUnit::Week,
                TimeUnit::Day,
                TimeUnit::Hour,
                TimeUnit::Minute,
                TimeUnit::Second,
            ];
            let entries: Vec<&str> = UNIT_SPELLINGS_HINT
                .split(", ")
                .map(|entry| entry.trim_start_matches("or "))
                .collect();

            // Act & Assert: every pair parses to the row that owns it.
            assert_eq!(
                entries.len(),
                row_units.len(),
                "the hint must name one pair per registry row"
            );
            for (&entry, unit) in entries.iter().zip(row_units) {
                let (plural, short) = entry
                    .split_once("\"/\"")
                    .expect("hint pairs are `\"plural\"/\"short\"`");
                let plural = plural.trim_matches('"');
                let short = short.trim_matches('"');

                assert_eq!(
                    plural,
                    unit.human_names().1,
                    "the hint's plural must be the registry's own plural"
                );
                assert_eq!(
                    TimeUnit::parse_spelling(plural),
                    Ok(unit),
                    "`{plural}` must parse from the hint"
                );
                assert_eq!(
                    TimeUnit::parse_spelling(short),
                    Ok(unit),
                    "`{short}` must parse from the hint"
                );
            }
        }

        #[test]
        fn unknown_spelling_error_names_the_unit_and_the_hint() {
            // Arrange
            let error = TimeUnit::parse_spelling("decades")
                .expect_err("`decades` is not an accepted unit spelling");

            // Act
            let rendered = error.to_string();

            // Assert
            assert_eq!(
                rendered,
                format!(
                    "unknown unit \"decades\" (expected {UNIT_SPELLINGS_HINT})"
                )
            );
        }
    }

    mod accessors {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
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
        #[case::second(TimeUnit::Second, 1)]
        #[case::minute(TimeUnit::Minute, 60)]
        #[case::hour(TimeUnit::Hour, 3_600)]
        #[case::day(TimeUnit::Day, 86_400)]
        #[case::week(TimeUnit::Week, 7 * 86_400)]
        #[case::month(TimeUnit::Month, 30 * 86_400)]
        #[case::year(TimeUnit::Year, 365 * 86_400)]
        fn returns_nominal_seconds_for_each_unit(
            #[case] unit: TimeUnit,
            #[case] expected: i64,
        ) {
            // Arrange: `unit` and `expected` are the case literals.
            // Act
            let actual = unit.nominal_seconds();
            // Assert
            assert_eq!(
                actual, expected,
                "nominal seconds of {unit:?} must be exactly {expected}"
            );
        }

        #[rstest]
        #[case::second(TimeUnit::Second, 1.0)]
        #[case::minute(TimeUnit::Minute, 60.0)]
        #[case::hour(TimeUnit::Hour, 3_600.0)]
        #[case::day(TimeUnit::Day, 86_400.0)]
        #[case::week(TimeUnit::Week, 7.0 * 86_400.0)]
        #[case::month(TimeUnit::Month, 30.0 * 86_400.0)]
        #[case::year(TimeUnit::Year, 365.0 * 86_400.0)]
        fn returns_nominal_seconds_f64_for_each_unit(
            #[case] unit: TimeUnit,
            #[case] expected: f64,
        ) {
            // Arrange: `unit` and `expected` are the case literals.
            // Act
            let actual = unit.nominal_seconds_f64();
            // Assert
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "nominal seconds f64 of {unit:?} must be exactly {expected}"
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
        use super::*;

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
                    "nominal unit sizes must be in descending order: {} > {}",
                    larger.nominal_seconds,
                    smaller.nominal_seconds
                );
            }
        }
    }

    mod decomposition {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn decomposes_human_units_greedily() {
            use crate::temporal::duration::COMPOUND_FORMAT_ORDER;

            // Arrange: 31,596,000 seconds is 1 year 16 hours 40 minutes.
            // Act
            let (human, remainder) =
                decompose(31_596_000, &COMPOUND_FORMAT_ORDER);

            // Assert
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
                "human decomposition must skip units larger than the                  remaining magnitude"
            );
            assert_eq!(remainder, 0);
        }
        #[test]
        fn leaves_the_remainder_to_the_caller() {
            // Arrange: 65 seconds decomposed into minutes/seconds.
            // Act
            let (parts, remainder) = decompose(65, &[TimeUnit::Minute]);
            assert_eq!(
                parts,
                vec![DecomposedPart {
                    unit: TimeUnit::Minute,
                    count: 1
                }],
                "one minute part should be produced"
            );
            assert_eq!(remainder, 5);
        }
    }

    mod projection {
        use jiff::civil::date;
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::year("year", Some(DateUnit::Year))]
        #[case::month("month", Some(DateUnit::Month))]
        #[case::day("day", Some(DateUnit::Day))]
        #[case::hour("hour", Some(DateUnit::Hour))]
        #[case::minute("minute", Some(DateUnit::Minute))]
        #[case::second("second", Some(DateUnit::Second))]
        #[case::weekday("weekday", Some(DateUnit::Weekday))]
        #[case::week("week", Some(DateUnit::Week))]
        #[case::case_insensitive("YEAR", Some(DateUnit::Year))]
        #[case::unknown("decade", None)]
        fn from_name_resolves_all_valid_names(
            #[case] input: &str,
            #[case] expected: Option<DateUnit>,
        ) {
            // Arrange: `input` and `expected` are case literals.
            // Act
            let actual = DateUnit::from_name(input);
            // Assert
            assert_eq!(actual, expected);
        }

        #[test]
        fn projects_all_units_from_datetime() {
            // Arrange: 2026-10-15 14:30:45 (Thursday, week 42)
            let dt = date(2026, 10, 15).at(14, 30, 45, 0);

            // Act & Assert
            assert_eq!(DateUnit::Year.of(dt), 2026);
            assert_eq!(DateUnit::Month.of(dt), 10);
            assert_eq!(DateUnit::Day.of(dt), 15);
            assert_eq!(DateUnit::Hour.of(dt), 14);
            assert_eq!(DateUnit::Minute.of(dt), 30);
            assert_eq!(DateUnit::Second.of(dt), 45);
            assert_eq!(DateUnit::Weekday.of(dt), 4); // Thursday = 4
            assert_eq!(DateUnit::Week.of(dt), 42);
        }
    }
}

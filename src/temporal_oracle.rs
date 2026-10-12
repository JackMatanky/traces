//! Migration oracle for the temporal redesign.
//!
//! Replays a fixed corpus of date, date-time, and duration operations through
//! the [`probe`] adapter and compares the transcript against a committed
//! snapshot. Stage 1 refactors must leave the snapshot byte-identical. Stage 2
//! changes it only on lines whose tag names a row of the divergence register in
//! `docs/divergence-register.md`; a `-` tag means the behavior must not change.
//!
//! The adapter is the only code that changes between stages: it renders results
//! through public-shaped accessors and reduces errors to a stable class name,
//! so the transcript never depends on a dependency's message wording.
//!
//! Remove this module once the migration lands and the doctrine cases are
//! promoted to named unit tests.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    reason = "the oracle is a temporary migration harness: the probe applies \
              the
              duration operators under test, and the corpus functions are flat
              case lists"
)]

use std::fmt::Write as _;

use crate::test_support::TzGuard;

/// Renders operation results as stable transcript text.
///
/// Every function takes plain strings and integers so the corpus below does
/// not name any engine type.
mod probe {
    use crate::{
        DateError, DateTimeValue, DateValue, DurationError, DurationUnit,
        DurationValue,
        date::{
            CalendarPeriod, DateComponent, DatePoint, DatePrecision,
            Difference, weekday_from_number,
        },
    };

    fn date_error_class(error: &DateError) -> &'static str {
        match error {
            DateError::UnknownShape => "NotADate",
            DateError::YearNotFourDigits {
                ..
            } => "YearNotFourDigits",
            DateError::InvalidValue {
                ..
            } => "Invalid",
            DateError::LocalZoneLookup {
                ..
            } => "Zone",
            DateError::OutOfRange => "OutOfRange",
        }
    }

    fn duration_error_class(_error: &DurationError) -> &'static str {
        "Invalid"
    }

    fn json<T: serde::Serialize>(value: &T) -> String {
        serde_json::to_string(value).expect("temporal values serialize")
    }

    fn unit(spelling: &str) -> DurationUnit {
        DurationUnit::from_spelling(spelling).expect("corpus uses real units")
    }

    fn render_point(point: DatePoint) -> String {
        match point {
            DatePoint::Calendar {
                date,
                date_precision,
            } => {
                let precision = match date_precision {
                    DatePrecision::Month => "month",
                    DatePrecision::Day => "day",
                };
                format!("date {date} ({precision})")
            }
            DatePoint::Instant(date_time) => {
                format!("datetime {} local {date_time}", json(&date_time))
            }
        }
    }

    fn render_duration(duration: &DurationValue) -> String {
        let regime = match duration.components() {
            None => "synthesized",
            Some(components)
                if components.iter().any(|part| part.unit().is_calendar()) =>
            {
                "calendar"
            }
            Some(_) => "exact",
        };
        format!(
            "{duration} | seconds {} | {} | {regime}",
            duration.nominal_seconds(),
            duration.humanize(),
        )
    }

    fn point_or(input: &str) -> Result<DatePoint, String> {
        DatePoint::parse(input)
            .map_err(|error| format!("Err({})", date_error_class(&error)))
    }

    fn render_result(result: Result<DatePoint, String>) -> String {
        result.map_or_else(|error| error, render_point)
    }

    pub(super) fn duration(input: &str) -> String {
        match DurationValue::parse(input) {
            Ok(duration) => render_duration(&duration),
            Err(error) => format!("Err({})", duration_error_class(&error)),
        }
    }

    pub(super) fn duration_prefix(input: &str) -> String {
        match DurationValue::parse_prefix(input) {
            Some((duration, end)) => {
                format!("{} @{end}", render_duration(&duration))
            }
            None => "None".to_owned(),
        }
    }

    pub(super) fn duration_op(left: &str, op: &str, right: &str) -> String {
        let (Ok(left), Ok(right)) =
            (DurationValue::parse(left), DurationValue::parse(right))
        else {
            return "Err(corpus)".to_owned();
        };
        let result = match op {
            "+" => left + right,
            "-" => left - right,
            _ => return "Err(corpus)".to_owned(),
        };
        render_duration(&result)
    }

    pub(super) fn duration_scale(input: &str, factor: f64) -> String {
        let Ok(duration) = DurationValue::parse(input) else {
            return "Err(corpus)".to_owned();
        };
        render_duration(&(duration * factor))
    }

    pub(super) fn duration_negate(input: &str) -> String {
        let Ok(duration) = DurationValue::parse(input) else {
            return "Err(corpus)".to_owned();
        };
        render_duration(&-duration)
    }

    pub(super) fn duration_order(left: &str, right: &str) -> String {
        match (DurationValue::parse(left), DurationValue::parse(right)) {
            (Ok(left), Ok(right)) => {
                format!("{:?} eq={}", left.cmp(&right), left == right)
            }
            _ => "Err(corpus)".to_owned(),
        }
    }

    pub(super) fn duration_json(input: &str) -> String {
        match DurationValue::parse(input) {
            Ok(duration) => json(&duration),
            Err(error) => format!("Err({})", duration_error_class(&error)),
        }
    }

    pub(super) fn duration_from_json(text: &str) -> String {
        serde_json::from_str::<DurationValue>(text).map_or_else(
            |_| "Err(Invalid)".to_owned(),
            |duration| render_duration(&duration),
        )
    }

    pub(super) fn date_value(input: &str) -> String {
        match input.parse::<DateValue>() {
            Ok(date) => format!("{date} {}", json(&date)),
            Err(error) => format!("Err({})", date_error_class(&error)),
        }
    }

    pub(super) fn date_value_from_json(text: &str) -> String {
        serde_json::from_str::<DateValue>(text).map_or_else(
            |_| "Err(Invalid)".to_owned(),
            |date| format!("{date} {}", json(&date)),
        )
    }

    pub(super) fn date_time_value(input: &str) -> String {
        match input.parse::<DateTimeValue>() {
            Ok(date_time) => format!("{} local {date_time}", json(&date_time)),
            Err(error) => format!("Err({})", date_error_class(&error)),
        }
    }

    pub(super) fn date_time_from_json(text: &str) -> String {
        serde_json::from_str::<DateTimeValue>(text).map_or_else(
            |_| "Err(Invalid)".to_owned(),
            |date_time| format!("{} local {date_time}", json(&date_time)),
        )
    }

    pub(super) fn promote(input: &str) -> String {
        match input.parse::<DateValue>() {
            Ok(date) => {
                let date_time = DateTimeValue::from(date);
                format!("{} local {date_time}", json(&date_time))
            }
            Err(error) => format!("Err({})", date_error_class(&error)),
        }
    }

    pub(super) fn point(input: &str) -> String {
        render_result(point_or(input))
    }

    pub(super) fn point_prefix(input: &str) -> String {
        match DatePoint::parse_prefix(input) {
            Some((point, end)) => format!("{} @{end}", render_point(point)),
            None => "None".to_owned(),
        }
    }

    pub(super) fn point_with_pattern(text: &str, pattern: &str) -> String {
        render_result(
            DatePoint::parse_with_pattern(text, pattern)
                .map_err(|error| format!("Err({})", date_error_class(&error))),
        )
    }

    pub(super) fn shift(input: &str, n: i64, unit_spelling: &str) -> String {
        render_result(point_or(input).and_then(|point| {
            point
                .shift(n, unit(unit_spelling))
                .map_err(|error| format!("Err({})", date_error_class(&error)))
        }))
    }

    pub(super) fn apply(input: &str, duration: &str) -> String {
        let Ok(duration) = DurationValue::parse(duration) else {
            return "Err(corpus)".to_owned();
        };
        render_result(point_or(input).and_then(|point| {
            point
                .apply(&duration)
                .map_err(|error| format!("Err({})", date_error_class(&error)))
        }))
    }

    pub(super) fn boundary(input: &str, period: &str, edge: &str) -> String {
        let period = match period {
            "week" => CalendarPeriod::Week,
            "month" => CalendarPeriod::Month,
            "year" => CalendarPeriod::Year,
            _ => return "Err(corpus)".to_owned(),
        };
        render_result(point_or(input).and_then(|point| {
            match edge {
                "start" => point.start_of(period),
                _ => point.end_of(period),
            }
            .map_err(|error| format!("Err({})", date_error_class(&error)))
        }))
    }

    pub(super) fn weekday(input: &str, iso_number: i64) -> String {
        let Some(target) = weekday_from_number(iso_number) else {
            return "None".to_owned();
        };
        render_result(point_or(input).and_then(|point| {
            point
                .with_weekday(target)
                .map_err(|error| format!("Err({})", date_error_class(&error)))
        }))
    }

    pub(super) fn component(input: &str, name: &str) -> String {
        let Some(component) = DateComponent::from_name(name) else {
            return "None".to_owned();
        };
        point_or(input).map_or_else(
            |error| error,
            |point| point.component(component).to_string(),
        )
    }

    pub(super) fn diff(from: &str, to: &str, unit_spelling: &str) -> String {
        match (point_or(from), point_or(to)) {
            (Ok(from), Ok(to)) => match from.diff(to, unit(unit_spelling)) {
                Difference::Whole(n) => format!("whole {n}"),
                Difference::Fractional(f) => format!("fractional {f:.6}"),
            },
            _ => "Err(parse)".to_owned(),
        }
    }
}

/// Ordered transcript of `tag | operation => result` lines.
#[derive(Default)]
struct Transcript {
    text: String,
}

impl Transcript {
    fn section(&mut self, title: &str) {
        let _ = writeln!(self.text, "\n## {title}");
    }

    fn line(&mut self, tag: &str, operation: &str, result: &str) {
        let _ = writeln!(self.text, "{tag} | {operation} => {result}");
    }
}

/// One zone the date-time corpus runs in, with its own DST transition times.
struct ZoneCase {
    /// IANA zone name.
    name: &'static str,
    /// A local time inside the spring-forward gap.
    gap: &'static str,
    /// A local time that occurs twice at the fall-back transition.
    fold: &'static str,
    /// Extra local times around the transitions for the arithmetic grids.
    around: &'static [&'static str],
    /// Whether the full arithmetic grids run here or only the parse and
    /// promotion corpus.
    full: bool,
}

/// Northern, 30-minute, southern, and midnight-gap DST zones, plus fixed
/// offsets: a gap or fold in each, and the full arithmetic grids in the two
/// zones whose behavior the unit suites pin most heavily.
const ZONES: [ZoneCase; 7] = [
    ZoneCase {
        name: "America/New_York",
        gap: "2026-03-08T02:30:00",
        fold: "2026-11-01T01:30:00",
        around: &[
            "2026-03-07T12:00:00",
            "2026-03-08T12:00:00",
            "2026-03-08T01:30:00",
            "2026-10-31T12:00:00",
        ],
        full: true,
    },
    ZoneCase {
        name: "Australia/Lord_Howe",
        gap: "2026-10-04T02:15:00",
        fold: "2026-04-05T01:45:00",
        around: &["2026-10-03T12:00:00", "2026-04-04T12:00:00"],
        full: true,
    },
    ZoneCase {
        name: "Europe/London",
        gap: "2026-03-29T01:30:00",
        fold: "2026-10-25T01:30:00",
        around: &["2026-03-28T12:00:00"],
        full: false,
    },
    ZoneCase {
        name: "America/Santiago",
        gap: "2026-09-06T00:30:00",
        fold: "2026-04-04T23:30:00",
        around: &["2026-09-05T12:00:00"],
        full: false,
    },
    ZoneCase {
        name: "America/Havana",
        gap: "2026-03-08T00:30:00",
        fold: "2026-11-01T00:30:00",
        around: &["2026-03-07T12:00:00"],
        full: false,
    },
    ZoneCase {
        name: "Etc/GMT-2",
        gap: "2026-03-08T02:30:00",
        fold: "2026-11-01T01:30:00",
        around: &[],
        full: false,
    },
    ZoneCase {
        name: "UTC",
        gap: "2026-03-08T02:30:00",
        fold: "2026-11-01T01:30:00",
        around: &[],
        full: false,
    },
];

const UNITS: [&str; 8] = ["ms", "s", "m", "h", "d", "w", "mo", "y"];

fn durations(log: &mut Transcript) {
    log.section("duration parse");
    let accepted = [
        "1h",
        "90m",
        "1h 30m",
        "1h30m",
        "1h, 30m",
        "1h,30m",
        "-1h 30m",
        "+1h",
        ".5h",
        "1.5h",
        "0.5h 15m",
        "1e3s",
        "1E-2s",
        "1h +30m",
        "1H",
        "1 h",
        "1 H",
        "4 yrs, 6 wks",
        "2 weeks",
        "3 days",
        "1d 1mo",
        "1mo 1d",
        "12h 12h",
        "1y 2mo 3w 4d 5h 6m 7s 8ms",
        "0s",
        "-0m",
        "1ms",
        "1.5ms",
        "0.0005s",
        "1.5d",
        "1.5mo",
        "7304484d",
        "7304485d",
        "9e15s",
        "1e400s",
        "100000000y",
        "3 Hours",
        "2 MINUTES",
        "10 secs",
    ];
    for input in accepted {
        let tag = match input {
            "9e15s" | "1e400s" | "100000000y" | "0.0005s" | "7304484d"
            | "7304485d" => "R5",
            "1e3s" | "1E-2s" | "1h +30m" | "1H" | "1 H" | "1d 1mo" | "1.5d"
            | "1.5mo" | "0.5h 15m" | ".5h" | "1h30m" | "1h,30m" | "+1h"
            | "1mo 1d" | "12h 12h" | "4 yrs, 6 wks" | "3 Hours"
            | "2 MINUTES" => "R3,R4",
            _ => "R4",
        };
        log.line(tag, &format!("duration {input:?}"), &probe::duration(input));
    }
    let rejected = [
        "", "  ", "h", "1", "1x", "1h30", "1h -30m", "1 hh", "--1h", "1h--30m",
        "1.2.3h", "e3s", "NaN s", "infh", "1,h", ",1h", "1h,", "1 h 30", "P1D",
        "PT90M", "1µs", "1us", "1ns",
    ];
    for input in rejected {
        let tag = if input.starts_with('P') || input.contains(['µ', 'u', 'n'])
        {
            "R3"
        } else {
            "-"
        };
        log.line(tag, &format!("duration {input:?}"), &probe::duration(input));
    }

    log.section("duration prefix");
    let prefixes = [
        "1h rest",
        "1h, 2m",
        "1h,2m",
        "30m, next",
        "1h 30m)",
        "1h x",
        "x1h",
        "-1h",
        "1h -2m",
        "2d\n3",
        "- 1h",
        "1h30m",
        "1.5d x",
        "1e3s,x",
        "1h+2m",
        "1h 2",
        "5",
        "",
    ];
    for input in prefixes {
        log.line(
            "R3",
            &format!("prefix {input:?}"),
            &probe::duration_prefix(input),
        );
    }

    log.section("duration operations");
    let pairs = [
        ("23h", "1h"),
        ("1d", "1h"),
        ("1h 30m", "30m"),
        ("1mo", "1d"),
        ("2d", "1d"),
        ("1d", "2d"),
        ("1h", "1h"),
        ("1w", "1d"),
        ("90m", "1h 30m"),
        ("1y", "1mo"),
    ];
    for (left, right) in pairs {
        for op in ["+", "-"] {
            log.line(
                "R4,R16",
                &format!("{left} {op} {right}"),
                &probe::duration_op(left, op, right),
            );
        }
    }
    for input in ["1h", "1d", "1mo", "1h 30m", "-2d", "0s"] {
        for factor in [2.0, 0.5, -1.0, 1.5, 0.0, 1e308, f64::NAN] {
            log.line(
                "R15",
                &format!("{input} * {factor}"),
                &probe::duration_scale(input, factor),
            );
        }
        log.line("R4", &format!("-({input})"), &probe::duration_negate(input));
    }
    for (left, right) in [
        ("1h", "60m"),
        ("1d", "24h"),
        ("1mo", "30d"),
        ("1y", "365d"),
        ("1h", "2h"),
        ("1w", "7d"),
        ("0s", "-0m"),
    ] {
        log.line(
            "-",
            &format!("order {left} vs {right}"),
            &probe::duration_order(left, right),
        );
    }

    log.section("duration serde");
    for input in ["1h 30m", "90m", "4 yrs, 6 wks", "-1d", "0s", "1.5h"] {
        log.line(
            "R4",
            &format!("to json {input}"),
            &probe::duration_json(input),
        );
    }
    for text in
        ["\"1h 30m\"", "\"90m\"", "\"\"", "\"nope\"", "42", "null", "\"1d 1h\""]
    {
        log.line(
            "R3",
            &format!("from json {text}"),
            &probe::duration_from_json(text),
        );
    }
}

/// Zone-free corpus: calendar dates never consult the local zone, so one run
/// under UTC covers them.
fn dates(log: &mut Transcript) {
    log.section("date value parse");
    let dates = [
        "2026-07-29",
        "2026-1-1",
        "2026-7-29",
        "2026-02-30",
        "2026-13-01",
        "26-07-29",
        "20260-01-01",
        "2026-07",
        "2026-07-29 ",
        "0000-01-01",
        "9999-12-31",
        "-2026-01-01",
        "10000-01-01",
        "2026/07/29",
        "2028-02-29",
        "2026-02-29",
        "2026-00-10",
        "2026-12-00",
        "2026-12-32",
        "2026-07-29x",
        "2026-07-29T10:00",
        "",
        "  2026-07-29  ",
    ];
    for input in dates {
        let tag = match input {
            "2026-1-1" | "2026-7-29" => "R6",
            "0000-01-01" | "9999-12-31" => "R14",
            _ => "-",
        };
        log.line(tag, &format!("date {input:?}"), &probe::date_value(input));
    }
    for text in [
        "\"2026-07-29\"",
        "\"2026-1-1\"",
        "\"2026-07\"",
        "\"\"",
        "7",
        "\"2026-07-29T10:00:00Z\"",
    ] {
        let tag = if text.contains("2026-1-1") {
            "R6"
        } else {
            "-"
        };
        log.line(
            tag,
            &format!("date from json {text}"),
            &probe::date_value_from_json(text),
        );
    }

    log.section("date point parse");
    let points = [
        "2026-07-29",
        "2026-07",
        "2026",
        "2026-",
        "26-07-29",
        "2026-1",
        "2026-13",
        "2026-00",
        "2026-02-30",
        "  2026-07-29  ",
        "20260729",
        "",
        "2026-1-1",
        "202-07-29",
    ];
    for input in points {
        let tag = if input == "2026-1-1" {
            "R6"
        } else {
            "-"
        };
        log.line(tag, &format!("point {input:?}"), &probe::point(input));
    }
    for input in [
        "2026-07-29 rest",
        "2026-07 rest",
        "2026-07-29T10:00",
        "2026-1-1",
        "2026-07-2",
        "26-07-29",
        "2026-07-",
        "2026-07-29",
        "2026-07",
        "2026-13-01 x",
        "2026-07-299",
        "x2026-07-29",
        "",
    ] {
        let tag = if input == "2026-1-1" {
            "R6"
        } else {
            "-"
        };
        log.line(
            tag,
            &format!("point prefix {input:?}"),
            &probe::point_prefix(input),
        );
    }

    log.section("date point arithmetic");
    let shift_points =
        ["2026-01-31", "2026-02-28", "2026-07", "2026-12-31", "2028-02-29"];
    for input in shift_points {
        for unit in UNITS {
            for n in [1_i64, -1, 2, 24, 36] {
                log.line(
                    "-",
                    &format!("shift {input} {n}{unit}"),
                    &probe::shift(input, n, unit),
                );
            }
        }
    }
    for (input, n, unit) in [
        ("2026-01-31", i64::MAX, "d"),
        ("2026-01-31", i64::MIN, "d"),
        ("2026-01-31", i64::MAX, "y"),
        ("2026-01-31", 1_000_000_000, "y"),
        ("2026-01-31", 300_000, "y"),
        ("2026-01-31", 5_000_000, "mo"),
        ("9999-12-31", 1, "d"),
        ("2026-07-29", 3_000_000, "y"),
    ] {
        log.line(
            "R14",
            &format!("shift {input} {n}{unit}"),
            &probe::shift(input, n, unit),
        );
    }
    for input in ["2026-01-31", "2026-07", "2026-03-07", "2026-12-31"] {
        for duration in APPLY_DURATIONS {
            log.line(
                apply_tag(duration),
                &format!("apply {input} + {duration}"),
                &probe::apply(input, duration),
            );
        }
    }
    for input in [
        "2026-01-01",
        "2026-07-29",
        "2026-07",
        "2026-12-31",
        "2024-12-30",
        "2021-01-03",
    ] {
        boundaries(log, input);
    }
    for input in [
        "2026-07-29",
        "2026-07",
        "2020-12-31",
        "2021-01-03",
        "2021-01-04",
        "2024-12-30",
        "2026-01-01",
    ] {
        components(log, input);
    }
    for (from, to) in [
        ("2026-01-31", "2026-02-28"),
        ("2026-01-31", "2026-03-01"),
        ("2026-02-28", "2026-01-31"),
        ("2025-07-29", "2026-07-28"),
        ("2026-07-29", "2025-07-29"),
        ("2026-07", "2026-09"),
        ("2026-03-07", "2026-03-09"),
        ("2024-02-29", "2028-02-29"),
        ("2024-02-29", "2025-02-28"),
        ("2026-01-01", "2026-01-01"),
    ] {
        diffs(log, from, to);
    }
}

const APPLY_DURATIONS: [&str; 22] = [
    "1d", "24h", "36h", "1d 1h", "1h 1d", "1mo", "1mo 1mo", "2mo", "1y", "1w",
    "1.5d", "0.5mo", "12h 12h", "-1d", "1h 30m", "1ms", "90m", "1d 12h",
    "1mo 1d", "0s", "100y", "-8000y",
];

fn apply_tag(duration: &str) -> &'static str {
    match duration {
        "1mo 1mo" | "1mo 1d" | "1d 1h" | "1h 1d" | "1d 12h" => "R1,R3",
        "12h 12h" => "R2,R3",
        "1.5d" | "0.5mo" => "R3,R15",
        "100y" | "-8000y" => "R14",
        _ => "-",
    }
}

fn boundaries(log: &mut Transcript, input: &str) {
    for period in ["week", "month", "year"] {
        for edge in ["start", "end"] {
            log.line(
                "-",
                &format!("{edge} of {period} {input}"),
                &probe::boundary(input, period, edge),
            );
        }
    }
    for iso in [0_i64, 1, 3, 7, 8] {
        log.line(
            "-",
            &format!("weekday {iso} of {input}"),
            &probe::weekday(input, iso),
        );
    }
}

fn components(log: &mut Transcript, input: &str) {
    for name in [
        "year", "month", "day", "hour", "minute", "second", "weekday", "week",
        "WEEK", "quarter",
    ] {
        log.line(
            "-",
            &format!("{name} of {input}"),
            &probe::component(input, name),
        );
    }
}

fn diffs(log: &mut Transcript, from: &str, to: &str) {
    for unit in UNITS {
        let tag = if from.contains('T') && to.contains('T') && unit == "d"
            || from == "2026-03-07" && unit == "d"
        {
            "R7"
        } else {
            "-"
        };
        log.line(
            tag,
            &format!("diff {from} -> {to} in {unit}"),
            &probe::diff(from, to, unit),
        );
    }
}

const DATE_TIMES: [&str; 32] = [
    "2026-07-29T14:30:00Z",
    "2026-07-29T14:30:00+02:00",
    "2026-07-29T14:30:00-05:30",
    "2026-07-29t14:30:00z",
    "2026-07-29T14:30:00.123",
    "2026-07-29T14:30:00.123456789",
    "2026-07-29T14:30:00.1234567891",
    "2026-07-29T14:30:00",
    "2026-07-29T14:30",
    "2026-07-29 14:30:00",
    "2026-07-29 14:30:00.5",
    "2026-07-29 14:30",
    "2026-07-29T24:00:00",
    "2026-07-29T23:59:60Z",
    "2026-07-29T14",
    "2026-07-29T14:30:00 ",
    "2026-07-29T14:30:00Zjunk",
    "2026-03-08T01:59:59",
    "2026-03-08 02:00",
    "2026-03-08 02:30",
    "2026-03-08 03:00",
    "2026-03-08T00:30:00",
    "2026-03-29T01:30:00",
    "2026-11-01 01:30",
    "2026-10-25T01:30:00",
    "2026-04-05T01:45:00",
    "2026-10-04T02:15:00",
    "2026-09-06T00:30:00",
    "2026-1-1T10:00:00",
    "999-01-01T10:00:00Z",
    "9999-12-31T23:59:59Z",
    "2026-07-29T14:30:00+99:00",
];

const PATTERNS: [(&str, &str); 17] = [
    ("29/07/2026", "%d/%m/%Y"),
    ("2026-07", "%Y-%m"),
    ("07/29/26 14:30", "%m/%d/%y %H:%M"),
    ("2026-07-29 14:30 +0200", "%Y-%m-%d %H:%M %z"),
    ("2026-07-29 14:30 +02:00", "%Y-%m-%d %H:%M %:z"),
    ("2026-07-29", "%Y-%m-%d"),
    ("2026-7-9", "%Y-%m-%d"),
    ("29 Jul 2026", "%d %b %Y"),
    ("July 29, 2026", "%B %d, %Y"),
    ("2026-W31-3", "%G-W%V-%u"),
    ("14:30", "%H:%M"),
    ("2026-07-29 14:30:00.250", "%Y-%m-%d %H:%M:%S%.f"),
    ("2026-07-29", "%Q"),
    ("2026-03-08 02:30", "%Y-%m-%d %H:%M"),
    ("  2026-07-29  ", "  %Y-%m-%d  "),
    ("2026-07-29 10:00 AM", "%Y-%m-%d %I:%M %p"),
    ("1785335400", "%s"),
];

fn zone_corpus(log: &mut Transcript, zone: &ZoneCase) {
    log.section(&format!("date-time parse in {}", zone.name));
    for input in DATE_TIMES {
        let tag = if input.contains("2026-1-1") {
            "R6"
        } else if input.contains("9999") || input.starts_with("999-") {
            "R14"
        } else {
            "R8"
        };
        log.line(
            tag,
            &format!("date-time {input:?}"),
            &probe::date_time_value(input),
        );
    }
    for text in [
        "\"2026-07-29T14:30:00Z\"",
        "\"2026-03-08T02:30:00\"",
        "\"2026-11-01T01:30:00\"",
        "\"2026-07-29\"",
        "\"nope\"",
        "12",
    ] {
        log.line(
            "R8",
            &format!("date-time from json {text}"),
            &probe::date_time_from_json(text),
        );
    }
    for input in [
        "2026-03-08",
        "2026-03-29",
        "2026-11-01",
        "2026-04-05",
        "2026-10-04",
        "2026-09-06",
        "2026-04-04",
        "2026-07-29",
    ] {
        log.line("R8", &format!("promote {input}"), &probe::promote(input));
    }
    for input in [
        "2026-07-29T14:30:00Z",
        "2026-07-29 14:30",
        "2026-07-29T25:00",
        "2026-07-29T14:30:00+99:00",
        "2026-1-1",
    ] {
        let tag = if input == "2026-1-1" {
            "R6"
        } else {
            "-"
        };
        log.line(tag, &format!("point {input:?}"), &probe::point(input));
    }
    for (text, pattern) in PATTERNS {
        let tag = if pattern == "%s" || pattern == "%Q" {
            "R9"
        } else {
            "-"
        };
        log.line(
            tag,
            &format!("pattern {text:?} with {pattern:?}"),
            &probe::point_with_pattern(text, pattern),
        );
    }

    let mut points = vec![zone.gap, zone.fold];
    points.extend_from_slice(zone.around);
    for &input in &points {
        for duration in ["1d", "24h", "36h", "1d 1h", "1h 1d", "1mo", "1h"] {
            log.line(
                apply_tag(duration),
                &format!("apply {input} + {duration}"),
                &probe::apply(input, duration),
            );
        }
        for (n, unit) in [(1_i64, "d"), (-1, "d"), (1, "h"), (24, "h")] {
            log.line(
                "-",
                &format!("shift {input} {n}{unit}"),
                &probe::shift(input, n, unit),
            );
        }
    }
    if !zone.full {
        return;
    }

    log.section(&format!("date-time arithmetic in {}", zone.name));
    for &input in &points {
        for unit in UNITS {
            for n in [1_i64, -1, 36] {
                log.line(
                    "-",
                    &format!("shift {input} {n}{unit}"),
                    &probe::shift(input, n, unit),
                );
            }
        }
        for duration in APPLY_DURATIONS {
            log.line(
                apply_tag(duration),
                &format!("apply {input} + {duration}"),
                &probe::apply(input, duration),
            );
        }
        boundaries(log, input);
    }
    for input in
        ["2026-07-29T14:30:00Z", "2026-12-31T23:59:59", "2026-01-31T12:00:00"]
    {
        for unit in UNITS {
            log.line(
                "-",
                &format!("shift {input} 1{unit}"),
                &probe::shift(input, 1, unit),
            );
        }
        boundaries(log, input);
    }
    for (input, n, unit) in [
        ("2026-07-29T10:00:00Z", i64::MAX, "ms"),
        ("2026-07-29T10:00:00Z", 1_000_000_000_000, "h"),
        ("2026-07-29T10:00:00Z", 1_000_000_000, "w"),
        ("2026-07-29T10:00:00Z", 8_000, "y"),
        ("9999-12-31T12:00:00Z", 1, "d"),
    ] {
        log.line(
            "R14",
            &format!("shift {input} {n}{unit}"),
            &probe::shift(input, n, unit),
        );
    }
    for input in points.iter().take(2) {
        components(log, input);
    }
    components(log, "2026-07-29T23:59:59Z");
    let (first, second) = (zone.gap, zone.fold);
    let before = points.get(2).copied().unwrap_or(first);
    let after = points.get(3).copied().unwrap_or(second);
    for (from, to) in [
        (before, after),
        (after, before),
        (before, first),
        (second, first),
        ("2026-07-29T10:00:00Z", "2026-07-29T12:30:00Z"),
        ("2026-07-29T12:30:00Z", "2026-07-29T10:00:00Z"),
        ("2026-07-29", "2026-07-29T12:00:00Z"),
        ("2026-07-29T12:00:00", "2026-08-30T13:30:15"),
    ] {
        diffs(log, from, to);
    }
}

/// Builds the full transcript of the corpus.
fn transcript() -> String {
    let mut log = Transcript::default();
    TzGuard::set("UTC");
    durations(&mut log);
    dates(&mut log);
    for zone in &ZONES {
        TzGuard::set(zone.name);
        log.section(&format!("ZONE {}", zone.name));
        zone_corpus(&mut log, zone);
    }
    log.text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_committed_transcript() {
        // Arrange
        let mut settings = insta::Settings::clone_current();
        settings.set_snapshot_path("../tests/snapshots");
        settings.set_prepend_module_to_snapshot(false);
        settings.set_omit_expression(true);

        // Act
        let transcript = transcript();

        // Assert
        settings
            .bind(|| insta::assert_snapshot!("temporal_oracle", transcript));
    }
}

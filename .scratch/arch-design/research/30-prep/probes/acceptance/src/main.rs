//! Runs one explicit case in a fresh process; emits one JSON object.
//!
//! Usage: `<bin> INPUT_JSON ANCHOR ZONE us|uk [--baseline] [--lenient]`
//! Example: `<bin> '"today at 00:00"' 2015-10-18T15:00:00Z America/Sao_Paulo
//! us` Encode NUL as `\u0000` inside INPUT_JSON. ANCHOR is an offset timestamp,
//! not an ambiguous civil time. ZONE selects the parser's named-zone rules.
//! `--lenient` selects English's non-strict API; other builds reject it.
//!
//! Build with no feature for a parser-free Jiff baseline. Select exactly
//! one of `english`, `fuzzy`, `gnu`, or `chronoenglish` for a candidate.
//! `chronoanchor` alone is a parser-free baseline with the Chrono stack.
//! A candidate build's `--baseline` skips calls, not parser linkage.
//! Compare whole harness sizes only against the matching anchor stack.
//!
//! Decode input, parse the anchor, and look up zones before counting.
//! Construction counts typed anchor projection from those fixtures.
//! Parse counts include dependency input copies and unwind machinery, but
//! exclude formatting and returned-result destruction. Drop regions count
//! result destruction separately. The first result stays alive through the
//! warmed call; neither result is formatted before both calls finish.
//! Warmed means exactly one prior call with the same input in this process.
//!
//! The no-op panic hook avoids counting diagnostic output. Caught panics
//! remain PANIC results. This unwind instrument is research only. It does
//! not establish safety for a product built with panic=abort.
//! chrono-tz is harness-only: its IANA rules make named-zone Chrono tests
//! possible without the host's local zone or a frozen fixed offset.

use std::{alloc::System, hint::black_box};

use jiff::{Timestamp, Zoned, tz::TimeZone};
use serde_json::{Value, json};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, Stats, StatsAlloc};

#[cfg(panic = "abort")]
compile_error!(
    "this research harness requires panic=unwind to distinguish PANIC from \
     ERROR"
);

#[cfg(any(
    all(feature = "english", feature = "fuzzy"),
    all(feature = "english", feature = "gnu"),
    all(feature = "english", feature = "chronoenglish"),
    all(feature = "fuzzy", feature = "gnu"),
    all(feature = "fuzzy", feature = "chronoenglish"),
    all(feature = "gnu", feature = "chronoenglish")
))]
compile_error!("select at most one parser feature");

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

struct Options {
    input: String,
    anchor: String,
    zone: String,
    dialect: String,
    baseline: bool,
    lenient: bool,
}

impl Options {
    fn read() -> Result<Self, String> {
        let mut args = std::env::args().skip(1);
        let usage = "usage: temporal-acceptance-probe INPUT_JSON ANCHOR ZONE \
                     us|uk [--baseline] [--lenient]";
        let encoded = args.next().ok_or(usage)?;
        let input =
            serde_json::from_str::<String>(&encoded).map_err(|error| {
                format!("INPUT_JSON must be a JSON string: {error}")
            })?;
        let anchor = args.next().ok_or(usage)?;
        let zone = args.next().ok_or(usage)?;
        let dialect = args.next().ok_or(usage)?;
        if dialect != "us" && dialect != "uk" {
            return Err("dialect must be us or uk".into());
        }
        let mut baseline = false;
        let mut lenient = false;
        for arg in args {
            match arg.as_str() {
                "--baseline" => baseline = true,
                "--lenient" if cfg!(feature = "english") => lenient = true,
                _ => return Err(format!("unsupported argument: {arg}")),
            }
        }
        Ok(Self {
            input,
            anchor,
            zone,
            dialect,
            baseline,
            lenient,
        })
    }
}

struct Anchors {
    jiff: Zoned,
    #[cfg(feature = "chronoanchor")]
    chrono: chrono::DateTime<chrono_tz::Tz>,
}

fn construct(options: &Options) -> Result<(Anchors, Stats), String> {
    let timestamp = options
        .anchor
        .parse::<Timestamp>()
        .map_err(|error| format!("invalid anchor: {error}"))?;
    let zone = TimeZone::get(&options.zone)
        .map_err(|error| format!("invalid Jiff zone: {error}"))?;
    #[cfg(feature = "chronoanchor")]
    let chrono_zone = options
        .zone
        .parse::<chrono_tz::Tz>()
        .map_err(|error| format!("invalid Chrono zone: {error}"))?;
    #[cfg(feature = "chronoanchor")]
    let chrono_utc = {
        // Jiff's negative fractional timestamp uses a negative remainder.
        let (seconds, nanos) = epoch_parts(timestamp);
        chrono::DateTime::<chrono::Utc>::from_timestamp(seconds, nanos)
            .ok_or("anchor is outside Chrono's range")?
    };

    let region = Region::new(&GLOBAL);
    let anchors = Anchors {
        jiff: black_box(timestamp).to_zoned(black_box(zone)),
        #[cfg(feature = "chronoanchor")]
        chrono: black_box(chrono_utc).with_timezone(black_box(&chrono_zone)),
    };
    let construction = region.change();
    Ok((anchors, construction))
}

fn epoch_parts(timestamp: Timestamp) -> (i64, u32) {
    let nanos = timestamp.subsec_nanosecond();
    let seconds = timestamp.as_second() - i64::from(nanos < 0);
    (seconds, nanos.rem_euclid(1_000_000_000) as u32)
}

fn jiff_point(point: &Zoned) -> Value {
    let timestamp = point.timestamp();
    let (seconds, nanos) = epoch_parts(timestamp);
    json!({
        "kind": "zoned",
        "native": point.to_string(),
        "utc": timestamp.to_string(),
        "unix_seconds_floor": seconds,
        "nanosecond": nanos,
    })
}

#[cfg(feature = "chronoanchor")]
fn chrono_point(point: &chrono::DateTime<chrono_tz::Tz>) -> Value {
    json!({
        "kind": "zoned",
        "native": point.to_rfc3339(),
        "utc": point.with_timezone(&chrono::Utc).to_rfc3339(),
        "unix_seconds_floor": point.timestamp(),
        "nanosecond": point.timestamp_subsec_nanos(),
    })
}

fn stats_json(stats: Stats) -> Value {
    json!({
        "allocations": stats.allocations,
        "deallocations": stats.deallocations,
        "reallocations": stats.reallocations,
        "bytes_allocated": stats.bytes_allocated,
        "bytes_deallocated": stats.bytes_deallocated,
        "bytes_reallocated": stats.bytes_reallocated,
    })
}

#[cfg(any(
    test,
    feature = "english",
    feature = "fuzzy",
    feature = "gnu",
    feature = "chronoenglish"
))]
enum Outcome<T, E> {
    Ok(T),
    Error(E),
    Panic(Box<dyn std::any::Any + Send>),
}

#[cfg(any(
    test,
    feature = "english",
    feature = "fuzzy",
    feature = "gnu",
    feature = "chronoenglish"
))]
fn capture<T, E>(operation: impl FnOnce() -> Result<T, E>) -> Outcome<T, E> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)) {
        Ok(Ok(point)) => Outcome::Ok(point),
        Ok(Err(error)) => Outcome::Error(error),
        Err(payload) => Outcome::Panic(payload),
    }
}

#[cfg(any(
    test,
    feature = "english",
    feature = "fuzzy",
    feature = "gnu",
    feature = "chronoenglish"
))]
fn outcome_json<T, E: std::fmt::Display + std::fmt::Debug>(
    outcome: &Outcome<T, E>,
    point_json: impl FnOnce(&T) -> Value,
) -> Value {
    match outcome {
        Outcome::Ok(point) => {
            json!({"status": "OK", "outputpoint": point_json(point)})
        }
        Outcome::Error(error) => json!({
            "status": "ERROR", "error": error.to_string(), "error_debug": format!("{error:?}"),
            "outputpoint": null,
        }),
        Outcome::Panic(payload) => {
            let message = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied());
            json!({"status": "PANIC", "panic_message": message, "outputpoint": null})
        }
    }
}

#[cfg(feature = "english")]
mod candidate {
    use super::*;
    pub const NAME: &str = "jiff-english=0.0.1";
    pub const DIALECT_APPLIED: bool = true;
    pub fn entry(options: &Options) -> &'static str {
        if options.lenient {
            "parse_date_string"
        } else {
            "parse_strict"
        }
    }
    pub fn parse(
        options: &Options,
        anchors: &Anchors,
    ) -> Result<Zoned, jiff_english::DateError> {
        let dialect = if options.dialect == "us" {
            jiff_english::Dialect::Us
        } else {
            jiff_english::Dialect::Uk
        };
        if options.lenient {
            jiff_english::parse_date_string(
                &options.input,
                &anchors.jiff,
                dialect,
            )
        } else {
            jiff_english::parse_strict(&options.input, &anchors.jiff, dialect)
        }
    }
    pub fn point_json(point: &Zoned) -> Value {
        jiff_point(point)
    }
}

#[cfg(feature = "fuzzy")]
mod candidate {
    use super::*;
    pub const NAME: &str = "fuzzydate=0.4.1";
    pub const DIALECT_APPLIED: bool = false;
    pub fn entry(_: &Options) -> &'static str {
        "aware_parse(Some(anchor), zone)"
    }
    pub fn parse(
        options: &Options,
        anchors: &Anchors,
    ) -> Result<chrono::DateTime<chrono_tz::Tz>, fuzzydate::Error> {
        fuzzydate::aware_parse(
            options.input.as_str(),
            Some(anchors.chrono),
            anchors.chrono.timezone(),
        )
    }
    pub fn point_json(point: &chrono::DateTime<chrono_tz::Tz>) -> Value {
        chrono_point(point)
    }
}

#[cfg(feature = "gnu")]
mod candidate {
    use super::*;
    pub const NAME: &str = "parse_datetime=0.17.0";
    pub const DIALECT_APPLIED: bool = false;
    pub fn entry(_: &Options) -> &'static str {
        "parse_datetime_at_date"
    }
    pub fn parse(
        options: &Options,
        anchors: &Anchors,
    ) -> Result<
        parse_datetime::ParsedDateTime,
        parse_datetime::ParseDateTimeError,
    > {
        // This API requires an owned Zoned; both samples need the same anchor.
        parse_datetime::parse_datetime_at_date(
            anchors.jiff.clone(),
            options.input.as_str(),
        )
    }
    pub fn point_json(point: &parse_datetime::ParsedDateTime) -> Value {
        match point {
            parse_datetime::ParsedDateTime::InRange(zoned) => jiff_point(zoned),
            parse_datetime::ParsedDateTime::Extended(extended) => json!({
                "kind": "extended",
                "native": extended.to_string(),
                "year": extended.year, "month": extended.month, "day": extended.day,
                "hour": extended.hour, "minute": extended.minute, "second": extended.second,
                "nanosecond": extended.nanosecond, "offset_seconds": extended.offset_seconds,
                "unix_seconds_floor": point.unix_epoch_second(),
            }),
        }
    }
}

#[cfg(feature = "chronoenglish")]
mod candidate {
    use super::*;
    pub const NAME: &str = "chrono-english=0.2.1";
    pub const DIALECT_APPLIED: bool = true;
    pub fn entry(_: &Options) -> &'static str {
        "parse_date_string (lenient API only)"
    }
    pub fn parse(
        options: &Options,
        anchors: &Anchors,
    ) -> Result<chrono::DateTime<chrono_tz::Tz>, chrono_english::DateError>
    {
        let dialect = if options.dialect == "us" {
            chrono_english::Dialect::Us
        } else {
            chrono_english::Dialect::Uk
        };
        chrono_english::parse_date_string(
            &options.input,
            anchors.chrono,
            dialect,
        )
    }
    pub fn point_json(point: &chrono::DateTime<chrono_tz::Tz>) -> Value {
        chrono_point(point)
    }
}

fn baseline_samples(anchors: &Anchors) -> Value {
    let region = Region::new(&GLOBAL);
    black_box(anchors);
    let first = region.change();
    let region = Region::new(&GLOBAL);
    black_box(anchors);
    let warmed = region.change();
    json!([
        {"phase": "first_call", "prior_parser_calls": 0,
         "result": {"status": "BASELINE", "outputpoint": null},
         "call": stats_json(first), "result_drop": stats_json(Stats::default())},
        {"phase": "warmed", "prior_parser_calls": 0,
         "result": {"status": "BASELINE", "outputpoint": null},
         "call": stats_json(warmed), "result_drop": stats_json(Stats::default())},
    ])
}

fn samples(options: &Options, anchors: &Anchors) -> Value {
    if options.baseline {
        return baseline_samples(anchors);
    }
    #[cfg(any(
        feature = "english",
        feature = "fuzzy",
        feature = "gnu",
        feature = "chronoenglish"
    ))]
    {
        let region = Region::new(&GLOBAL);
        let first = capture(|| {
            candidate::parse(black_box(options), black_box(anchors))
        });
        let first_stats = region.change();
        let region = Region::new(&GLOBAL);
        let warmed = capture(|| {
            candidate::parse(black_box(options), black_box(anchors))
        });
        let warmed_stats = region.change();

        let first_json = outcome_json(&first, candidate::point_json);
        let warmed_json = outcome_json(&warmed, candidate::point_json);
        let region = Region::new(&GLOBAL);
        drop(first);
        let first_drop = region.change();
        let region = Region::new(&GLOBAL);
        drop(warmed);
        let warmed_drop = region.change();
        json!([
            {"phase": "first_call", "prior_parser_calls": 0, "result": first_json,
             "call": stats_json(first_stats), "result_drop": stats_json(first_drop)},
            {"phase": "warmed", "prior_parser_calls": 1, "result": warmed_json,
             "call": stats_json(warmed_stats), "result_drop": stats_json(warmed_drop)},
        ])
    }
    #[cfg(not(any(
        feature = "english",
        feature = "fuzzy",
        feature = "gnu",
        feature = "chronoenglish"
    )))]
    baseline_samples(anchors)
}

fn run() -> Result<Value, String> {
    let options = Options::read()?;
    let (anchors, construction) = construct(&options)?;
    let samples = samples(&options, &anchors);
    #[cfg(any(
        feature = "english",
        feature = "fuzzy",
        feature = "gnu",
        feature = "chronoenglish"
    ))]
    let (name, entry, dialect_applied) = (
        candidate::NAME,
        candidate::entry(&options),
        candidate::DIALECT_APPLIED,
    );
    #[cfg(not(any(
        feature = "english",
        feature = "fuzzy",
        feature = "gnu",
        feature = "chronoenglish"
    )))]
    let (name, entry, dialect_applied) = ("baseline", "none", false);
    #[cfg(not(feature = "chronoanchor"))]
    let prepared = json!({"jiff": jiff_point(&anchors.jiff)});
    #[cfg(feature = "chronoanchor")]
    let prepared = json!({
        "jiff": jiff_point(&anchors.jiff),
        "chrono": chrono_point(&anchors.chrono),
        "chrono_tzdb_version": chrono_tz::IANA_TZDB_VERSION,
    });
    let region = Region::new(&GLOBAL);
    drop(anchors);
    let construction_drop = region.change();
    Ok(json!({
        "schema_version": 1,
        "candidate": name,
        "entry_point": entry,
        "input": options.input,
        "injected": {"anchor": options.anchor, "zone": options.zone, "dialect": options.dialect},
        "dialect_applied": dialect_applied && !options.baseline,
        "lenient_requested": options.lenient,
        "baseline_requested": options.baseline,
        "chrono_tz_harness_only": cfg!(feature = "chronoanchor"),
        "panic_strategy": "unwind; research only, not product panic=abort safety",
        "measurement_scope": "construction excludes fixture parsing and zone lookup; call excludes formatting and returned-result drop; bytes_reallocated is signed net growth",
        "prepared_anchor": prepared,
        "construction": stats_json(construction),
        "construction_drop": stats_json(construction_drop),
        "samples": samples,
    }))
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    match run() {
        Ok(output) => println!("{output}"),
        Err(error) => {
            println!("{}", json!({"status": "HARNESS_ERROR", "error": error}));
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod capture {
        use pretty_assertions::assert_eq;

        use super::*;

        fn date_json(date: &jiff::civil::Date) -> Value {
            json!(date.to_string())
        }

        #[test]
        fn returns_ok_with_successful_date() {
            let outcome =
                super::super::capture(|| jiff::civil::Date::new(2024, 2, 29));

            let result = outcome_json(&outcome, date_json);

            assert_eq!(result["status"], "OK");
            assert_eq!(result["outputpoint"], "2024-02-29");
        }

        #[test]
        fn returns_error_with_invalid_date() {
            let outcome =
                super::super::capture(|| jiff::civil::Date::new(2024, 2, 30));

            let result = outcome_json(&outcome, date_json);

            assert_eq!(result["status"], "ERROR");
            assert_eq!(result["outputpoint"], Value::Null);
        }

        #[test]
        fn returns_panic_with_unwinding_parser() {
            let outcome = super::super::capture(
                || -> Result<jiff::civil::Date, &'static str> {
                    panic!("candidate panic")
                },
            );

            let result = outcome_json(&outcome, date_json);

            assert_eq!(result["status"], "PANIC");
            assert_eq!(result["panic_message"], "candidate panic");
            assert_eq!(result["outputpoint"], Value::Null);
        }
    }
}

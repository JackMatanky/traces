//! Behavior and indicative-cost probe for the natural-language candidates
//! (`Q14`). Each feature set runs the same phrase list through its candidate
//! against one fixed anchor, so results are comparable across candidates.
//!
//! Usage:
//!   `<bin> "phrase"`            — one parse, both result forms printed
//!   `<bin> "phrase" --bench`    — indicative ns/parse over 20,000 in-process
//!                                 iterations (release build, black_box)
//!
//! Run: `cargo +nightly-2026-10-01 build --release --locked [--features
//! english|fuzzy]`

#[cfg(any(feature = "english", feature = "fuzzy"))]
use std::hint::black_box;
#[cfg(any(feature = "english", feature = "fuzzy"))]
use std::time::Instant;

use jiff::tz::TimeZone;

fn main() {
    let input =
        std::env::args().nth(1).unwrap_or_else(|| "tomorrow".to_owned());
    let bench = std::env::args().nth(2).as_deref() == Some("--bench");
    let base = "2026-03-14T12:00:00Z"
        .parse::<jiff::Timestamp>()
        .expect("fixture")
        .to_zoned(TimeZone::UTC);
    #[cfg(feature = "english")]
    if bench {
        let start = Instant::now();
        let mut ok = 0u64;
        for _ in 0..20_000 {
            let parsed = jiff_english::parse_date_string(
                black_box(&input),
                black_box(&base),
                jiff_english::Dialect::Us,
            );
            ok += u64::from(black_box(parsed.is_ok()));
        }
        println!(
            "english ns_per_parse={} parses=20000 ok={ok}",
            start.elapsed().as_nanos() / 20_000
        );
        return;
    }
    #[cfg(feature = "fuzzy")]
    if bench {
        let anchor: chrono::DateTime<chrono::Utc> =
            "2026-03-14T12:00:00Z".parse().expect("fixture");
        let start = Instant::now();
        let mut ok = 0u64;
        for _ in 0..20_000 {
            let parsed = fuzzydate::aware_parse(
                black_box(&input),
                Some(anchor),
                chrono::Utc,
            );
            ok += u64::from(black_box(parsed.is_ok()));
        }
        println!(
            "fuzzy ns_per_parse={} parses=20000 ok={ok}",
            start.elapsed().as_nanos() / 20_000
        );
        return;
    }
    #[cfg(feature = "english")]
    {
        println!(
            "lenient={:?}",
            jiff_english::parse_date_string(
                &input,
                &base,
                jiff_english::Dialect::Us
            )
        );
        println!(
            "strict={:?}",
            jiff_english::parse_strict(
                &input,
                &base,
                jiff_english::Dialect::Us
            )
        );
    }
    #[cfg(feature = "fuzzy")]
    {
        let anchor: chrono::DateTime<chrono::Utc> =
            "2026-03-14T12:00:00Z".parse().expect("fixture");
        println!(
            "{:?}",
            fuzzydate::aware_parse(&input, Some(anchor), chrono::Utc)
        );
    }
    #[cfg(not(any(feature = "english", feature = "fuzzy")))]
    println!("{:?}", input.parse::<jiff::civil::Date>());
    println!("base={base}");
}

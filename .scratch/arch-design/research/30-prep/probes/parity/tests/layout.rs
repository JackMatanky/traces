//! Proves the complete proposed `NoteFieldValue` stays inside the 80-byte cap
//! (`Q19`). The 24-byte payload measured in `src/main.rs` alone does not bind
//! the cap: the largest variant does. Mirrors `src/note/field.rs` with the
//! proposed inner Civil/Instant enum in place of `DateTimeValue`.
//!
//! Run: `cargo +nightly-2026-10-01 test --locked --test layout -- --nocapture`

use std::{alloc::System, mem::size_of};

use indexmap::IndexMap;
use jiff::civil::Date;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// The proposed inner payload from `src/main.rs`.
// Layout probe: variants must exist for `size_of`, not be constructed.
#[expect(dead_code, reason = "the probe measures variant layouts, not use")]
#[derive(Clone, Debug)]
enum DateTimePayload {
    Civil(jiff::civil::DateTime),
    Instant(jiff::Timestamp),
}

/// Mirrors `NoteFieldValue` with `DateTime(DateTimeValue)` replaced by the
/// proposed two-case payload. `DurationValue` is `Span` plus `i64`
/// nominal seconds; `Link` is two strings plus two flag bytes.
// Layout probe: variants must exist for `size_of`, not be constructed.
#[expect(dead_code, reason = "the probe measures variant layouts, not use")]
#[derive(Clone, Debug)]
enum ProposedNoteFieldValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Date(Date),
    DateTime(DateTimePayload),
    Duration(jiff::Span, i64),
    Link {
        target: String,
        text: Option<String>,
        markdown: bool,
        embedded: bool,
    },
    List(Box<[ProposedNoteFieldValue]>),
    Object(IndexMap<String, ProposedNoteFieldValue>),
}

#[test]
fn complete_enum_stays_inside_the_cap() {
    // Record every variant: the cap binds on the largest variant, not the
    // payload. `List` and `Object` embed the enum itself, so the size is a
    // fixed point: the compiler resolves it to the next larger variant.
    println!(
        "sizes Null=0 Bool={} Number={} String={} Date={} DateTime={} \
         Duration={} Link={} List={} Object={} enum={}",
        size_of::<bool>(),
        size_of::<f64>(),
        size_of::<String>(),
        size_of::<Date>(),
        size_of::<DateTimePayload>(),
        size_of::<(jiff::Span, i64)>(),
        size_of::<(String, Option<String>, bool, bool)>(),
        size_of::<Box<[ProposedNoteFieldValue]>>(),
        size_of::<IndexMap<String, ProposedNoteFieldValue>>(),
        size_of::<ProposedNoteFieldValue>(),
    );
    assert!(
        size_of::<ProposedNoteFieldValue>() <= 80,
        "whole NoteFieldValue exceeds the 80-byte cap"
    );
}

#[test]
fn temporal_payload_allocates_nothing() {
    // Parse before the region window: parser warm-up allocations (error
    // paths, locks, harness lazies) must not pollute the count. The civil
    // datetime is `Copy`, so the payload is a stack move.
    let civil: jiff::civil::DateTime =
        "2026-01-01T12:00:00".parse().expect("civil fixture is valid");
    let region = Region::new(&GLOBAL);
    let value = ProposedNoteFieldValue::DateTime(DateTimePayload::Civil(civil));
    let change = region.change();
    drop(value);
    assert_eq!(
        (change.allocations, change.deallocations),
        (0, 0),
        "a temporal payload must not allocate"
    );
}

//! Feeds arbitrary text to the date-point parser. It must not panic. A
//! parsed day-precision date must survive a `%Y-%m-%d` display and re-parse.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &str| {
    assert!(
        traces::fuzzing::date_point_display_law_holds(input),
        "display-then-parse law broken for {input:?}"
    );
});

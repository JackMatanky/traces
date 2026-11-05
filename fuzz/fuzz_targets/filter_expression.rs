//! Feeds arbitrary text to the filter-expression parser. It must not panic.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &str| {
    let _parsed = traces::fuzzing::parse_filter_expression(input);
});

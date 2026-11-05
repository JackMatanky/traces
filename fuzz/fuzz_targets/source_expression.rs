//! Feeds arbitrary text to the source-expression parser. It must not panic.
#![no_main]

use libfuzzer_sys::fuzz_target;
use traces::testing::SourceSelector;

fuzz_target!(|input: &str| {
    let _parsed = SourceSelector::parse(input);
});

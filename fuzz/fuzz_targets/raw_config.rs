//! Feeds arbitrary TOML text to the Config File parser. It must not panic.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &str| {
    let _parsed = traces::fuzzing::parse_raw_config(input);
});

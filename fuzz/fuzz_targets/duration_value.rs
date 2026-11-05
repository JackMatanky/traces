//! Feeds arbitrary text to the duration parser. It must not panic. A parsed
//! duration must survive display and re-parse unchanged.
#![no_main]

use libfuzzer_sys::fuzz_target;
use traces::DurationValue;

fuzz_target!(|input: &str| {
    if let Ok(duration) = input.parse::<DurationValue>() {
        let displayed = duration.to_string();
        assert_eq!(
            displayed.parse::<DurationValue>().ok(),
            Some(duration),
            "display-then-parse law broken for {input:?} shown as \
             {displayed:?}"
        );
    }
});

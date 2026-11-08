//! Dumps every step this binary registers, with the regex and the specificity
//! score the framework derives for it.
//!
//! `.mise/scripts/check_steps.py --registry` reads the dump. It compares the
//! framework values with its own mirror, and the registered steps with the
//! step definitions it parses from the step files. The dump lets the script
//! check the real steps, not only the probe patterns of the shared baseline.

use std::{fmt::Write as _, fs, path::Path};

use rstest_bdd::{Step, iter};
use rstest_bdd_patterns::{SpecificityScore, build_regex_from_pattern};

/// Writes `step-registry.tsv` in the Cargo target temp directory: one
/// `keyword<TAB>pattern<TAB>regex<TAB>literal/placeholders/typed` line for each
/// registered step, sorted. Fails when the framework rejects a registered
/// pattern, or when the binary registers no step.
#[test]
fn registered_steps_are_dumped_with_their_framework_regexes() {
    let mut lines: Vec<String> = Vec::new();
    for step in iter::<Step> {
        let pattern = step.pattern.as_str();
        let regex = build_regex_from_pattern(pattern)
            .expect("a registered step pattern compiles");
        let score = SpecificityScore::calculate(pattern)
            .expect("a registered step pattern has a score");
        let mut line = String::new();
        write!(
            line,
            "{}\t{pattern}\t{regex}\t{}/{}/{}",
            step.keyword.as_str().to_ascii_lowercase(),
            score.literal_chars,
            score.placeholder_count,
            score.typed_placeholder_count
        )
        .expect("writing to a String cannot fail");
        lines.push(line);
    }
    lines.sort_unstable();

    assert!(!lines.is_empty(), "the features binary registers no step");
    let dump = Path::new(env!("CARGO_TARGET_TMPDIR")).join("step-registry.tsv");
    fs::write(&dump, lines.join("\n") + "\n").expect("write the step dump");
}

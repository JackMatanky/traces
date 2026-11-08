//! Pins how `rstest-bdd` turns a step pattern into a regular expression.
//!
//! `.mise/scripts/check_steps.py` mirrors that conversion to match feature
//! steps with step definitions before the suite runs. This test and the
//! script's self-test share one baseline file, so a framework change that
//! alters the conversion fails here, and a script that drifts from the
//! framework fails in the self-test.

use std::fmt::Write as _;

use pretty_assertions::assert_eq;
use rstest_bdd_patterns::build_regex_from_pattern;

/// Patterns that cover every placeholder hint class and every escape class
/// the mirror in `check_steps.py` accepts, plus two the framework rejects and
/// the mirror must reject too. The mirror also rejects forms the framework
/// accepts (backslashes, doubled braces, spaces inside braces); they are not
/// probed because the mirror fails closed on them.
const PROBES: &[&str] = &[
    "{x}",
    "{x:i32}",
    "{x:u32}",
    "{x:i8}",
    "{x:i128}",
    "{x:u64}",
    "{x:isize}",
    "{x:usize}",
    "{x:f32}",
    "{x:f64}",
    "{x:string}",
    "{x:String}",
    "{x:unknown}",
    "run {a} then {b:u32}",
    "a.b-c#d&e~f",
    "a b \"q\" 'r' /s",
    "{x: u32}",
    "stray { brace",
];

/// Renders the baseline file text for `PROBES`: one `pattern<TAB>regex` line
/// per probe, in probe order. A probe the framework rejects renders as `ERROR`.
fn render_baseline() -> String {
    let mut text = String::new();
    for pattern in PROBES {
        let regex = build_regex_from_pattern(pattern)
            .unwrap_or_else(|_| String::from("ERROR"));
        writeln!(text, "{pattern}\t{regex}")
            .expect("writing to a String cannot fail");
    }
    text
}

/// Fails when the framework's pattern-to-regex conversion no longer matches
/// the shared baseline.
///
/// To accept a framework change, copy the `right` text of the failure into
/// `tests/baselines/step-regexes.tsv`, then update the mirror in
/// `.mise/scripts/check_steps.py` until its self-test passes.
#[test]
fn step_pattern_regexes_match_the_shared_baseline() {
    assert_eq!(
        include_str!("../baselines/step-regexes.tsv"),
        render_baseline()
    );
}

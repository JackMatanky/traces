//! Checks that feature steps and step definitions match one to one, using the
//! framework's own parser and matcher.
//!
//! `scenarios!` reports a missing step only when its scenario runs, and it
//! never reports an unused definition, a duplicate or an ambiguous line. This
//! test parses every feature file with the `gherkin` crate (the parser of the
//! macros) and matches each step with `extract_placeholders` and
//! `StepPattern::specificity`. A step line that no definition matches, a tie
//! between two definitions, an unused definition and a duplicate definition
//! each fail the test.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use gherkin::{Feature, GherkinEnv, Step as GherkinStep};
use rstest_bdd::{
    Step, StepKeyword, StepText, duplicate_steps, extract_placeholders, iter,
};
use rstest_bdd_patterns::SpecificityScore;

/// Feature files below `dir`, sorted.
fn feature_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in
            fs::read_dir(&current).expect("read the features directory")
        {
            let path = entry.expect("read a directory entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().is_some_and(|ext| ext == "feature") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Every step of a feature, including steps in a background or a rule.
fn steps_of(feature: &Feature) -> Vec<&GherkinStep> {
    let rule_steps = feature.rules.iter().flat_map(|rule| {
        rule.background
            .iter()
            .flat_map(|background| &background.steps)
            .chain(rule.scenarios.iter().flat_map(|scenario| &scenario.steps))
    });
    feature
        .background
        .iter()
        .flat_map(|background| &background.steps)
        .chain(feature.scenarios.iter().flat_map(|scenario| &scenario.steps))
        .chain(rule_steps)
        .collect()
}

/// Definitions of `keyword` that match `text`, best first.
fn matches_of(
    keyword: StepKeyword,
    text: &str,
) -> Vec<(&'static Step, SpecificityScore)> {
    let mut hits: Vec<_> = iter::<Step>
        .into_iter()
        .filter(|step| {
            step.keyword == keyword
                && extract_placeholders(step.pattern, StepText::from(text))
                    .is_ok()
        })
        .map(|step| {
            let score = step
                .pattern
                .specificity()
                .expect("a registered pattern has a score");
            (step, score)
        })
        .collect();
    hits.sort_by_key(|hit| std::cmp::Reverse(hit.1));
    hits
}

/// Problems between the feature files under `dir` and the registered steps.
fn problems(dir: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    let mut used: HashSet<(StepKeyword, &'static str)> = HashSet::new();
    for path in feature_files(dir) {
        let feature = Feature::parse_path(&path, GherkinEnv::default())
            .expect("a feature file parses");
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        for step in steps_of(&feature) {
            let at = format!("{name}:{}", step.position.line);
            let keyword =
                StepKeyword::try_from(step.ty).expect("a step has a keyword");
            let hits = matches_of(keyword, &step.value);
            let Some((best, best_score)) = hits.first() else {
                problems.push(format!(
                    "Undefined step at {at}: {keyword:?} '{}'",
                    step.value
                ));
                continue;
            };
            used.insert((best.keyword, best.pattern.as_str()));
            if let Some((second, second_score)) = hits.get(1)
                && second_score == best_score
            {
                problems.push(format!(
                    "Ambiguous step at {at}: '{}' matches '{}' and '{}'",
                    step.value,
                    best.pattern.as_str(),
                    second.pattern.as_str()
                ));
            }
        }
    }
    problems.extend(
        iter::<Step>
            .into_iter()
            .filter(|step| {
                !used.contains(&(step.keyword, step.pattern.as_str()))
            })
            .map(|step| {
                format!(
                    "Unused step definition: {:?} '{}' ({}:{})",
                    step.keyword,
                    step.pattern.as_str(),
                    step.file,
                    step.line
                )
            }),
    );
    problems.extend(
        duplicate_steps().iter().filter_map(|group| group.first()).map(
            |step| {
                format!(
                    "Duplicate step definition: {:?} '{}'",
                    step.keyword,
                    step.pattern.as_str()
                )
            },
        ),
    );
    problems.sort();
    problems
}

/// Fails on an undefined, ambiguous, unused or duplicate step.
#[test]
fn feature_steps_and_step_definitions_match_one_to_one() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/features");
    let problems = problems(&dir);
    assert!(
        problems.is_empty(),
        "step gate problems:\n{}",
        problems.join("\n")
    );
}

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
    borrow::Cow,
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use gherkin::{Feature, GherkinEnv, Scenario, Step as GherkinStep};
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

/// Substitutes `Examples:` row cells into `<header>` placeholders in `raw`.
fn expand_row<'a>(
    raw: &'a str,
    headers: &[String],
    row: &[String],
) -> Cow<'a, str> {
    let mut text = Cow::Borrowed(raw);
    for (header, cell) in headers.iter().zip(row) {
        let token = format!("<{header}>");
        if text.contains(&token) {
            text = Cow::Owned(text.replace(&token, cell));
        }
    }
    text
}

/// Steps of a scenario, expanding `Examples:` rows when present.
fn scenario_steps(scenario: &Scenario) -> Vec<(&GherkinStep, Cow<'_, str>)> {
    if scenario.examples.is_empty() {
        return scenario
            .steps
            .iter()
            .map(|step| (step, Cow::Borrowed(step.value.as_str())))
            .collect();
    }
    let mut out = Vec::new();
    for ex in &scenario.examples {
        if let Some(table) = &ex.table
            && let Some((headers, rows)) = table.rows.split_first()
        {
            for row in rows {
                for step in &scenario.steps {
                    out.push((step, expand_row(&step.value, headers, row)));
                }
            }
        }
    }
    out
}

/// Every step of a feature, including steps in a background, rule, or outline.
fn steps_of(feature: &Feature) -> Vec<(&GherkinStep, Cow<'_, str>)> {
    let bg_steps = feature
        .background
        .iter()
        .flat_map(|bg| &bg.steps)
        .map(|step| (step, Cow::Borrowed(step.value.as_str())));
    let top_steps = feature.scenarios.iter().flat_map(scenario_steps);
    let rule_steps = feature.rules.iter().flat_map(|rule| {
        let r_bg = rule
            .background
            .iter()
            .flat_map(|bg| &bg.steps)
            .map(|step| (step, Cow::Borrowed(step.value.as_str())));
        let r_sc = rule.scenarios.iter().flat_map(scenario_steps);
        r_bg.chain(r_sc)
    });
    bg_steps.chain(top_steps).chain(rule_steps).collect()
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
        for (step, text) in steps_of(&feature) {
            let at = format!("{name}:{}", step.position.line);
            let keyword =
                StepKeyword::try_from(step.ty).expect("a step has a keyword");
            let hits = matches_of(keyword, &text);
            let Some((best, best_score)) = hits.first() else {
                problems.push(format!(
                    "Undefined step at {at}: {keyword:?} '{text}'"
                ));
                continue;
            };
            used.insert((best.keyword, best.pattern.as_str()));
            if let Some((second, second_score)) = hits.get(1)
                && second_score == best_score
            {
                problems.push(format!(
                    "Ambiguous step at {at}: '{text}' matches '{}' and '{}'",
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

# Automating BDD with `rstest-bdd 0.6.0`

> Reference guide for step macros, fixture state, data tables, scenario discovery, silent-green traps, and the `step_gate.rs` guardrail in `rstest-bdd =0.6.0`.

## 1. Crate Pairing and Version Drift

Warning: Pin both `rstest-bdd` and `rstest-bdd-macros` to the exact same version (`=0.6.0`) in `[dev-dependencies]` alongside `rstest`:

```toml
[dev-dependencies]
gherkin = "0.15"
rstest = "0.26"
rstest-bdd = "=0.6.0"
rstest-bdd-macros = "=0.6.0"
rstest-bdd-patterns = "=0.6.0"
```

Do not mix `0.6.0` docs with unreleased `0.7.0` changes on `main`. Version `0.6.0` uses `inventory::submit!` instead of `#[step_library]`.

## 2. Step Definition Macros (`#[given]`, `#[when]`, `#[then]`)

Import step macros from `rstest_bdd_macros::{given, when, then}`:

- **Keyword is part of the registry key:** A `#[given("...")]` function matches only `Given` steps (and chained `And`/`But` steps). It will not match a `When` or `Then` step with the same text.
- **Placeholders:**
  - Bare `{name}` binds a `String` (or `&str`).
  - Typed `{count:u32}` or `{amount:f64}` parses the capture using `FromStr` before invoking the function.
- **Fixture binding:** Match the fixture function name in the parameter list (`world: &mut TestWorld`). If you prefix an unused parameter with `_` (`_world: &mut TestWorld`), attach `#[from(world)]` so `rstest-bdd` still resolves the `world` fixture.
- **Struct bundling (`#[derive(StepArgs)]`):** Bundle >3 step placeholders into a struct deriving `rstest_bdd_macros::StepArgs` to satisfy Clippy argument limits.
- **Fallible steps (`StepResult<T, E>`):** Return `Result<(), E>` (where `E: std::fmt::Display`) to propagate errors with `?` instead of panicking.

```rust
use rstest_bdd_macros::{given, when, then};

#[given("a workspace with retention of {days:u32} days")]
fn given_workspace_retention(world: &mut TestWorld, days: u32) {
    world.init_with_retention(days);
}

#[when("the operator prunes expired records")]
fn when_operator_prunes(world: &mut TestWorld) {
    world.run_cli(["prune"]);
}

#[then("the command exits with code {code:i32}")]
fn then_command_exits_with_code(world: &mut TestWorld, code: i32) {
    assert_eq!(world.last_exit_code(), Some(code));
}
```

## 3. Fixture State Architecture (`&mut Fixture` vs. `Slot<T>`)

Every scenario receives a fresh instance of its `#[fixture]` parameters:

1. **Primary pattern (`&mut TestWorld`):** Define a single per-scenario context struct (`TestWorld`) holding a `tempfile::TempDir`, configuration, and captured command output. Inject `world: &mut TestWorld` into every step function.
2. **Interior-mutability alternative (`ScenarioState` + `Slot<T>`):** Derive `rstest_bdd_macros::ScenarioState` on a struct with `Slot<T>` fields for shared `&State` access.

## 4. Doc Strings and Data Tables

- **Doc strings (`"""`):** Declare a trailing parameter named `docstring: String` on the step function.
- **Raw Data Tables:** Declare a parameter named `datatable: Vec<Vec<String>>` (or `&[Vec<String>]`).
- **Typed Data Tables (`Rows<T>`):** Derive `rstest_bdd_macros::DataTableRow` on a row struct and accept `#[datatable] rows: rstest_bdd::datatable::Rows<RowStruct>`:

```rust
use rstest_bdd::datatable::Rows;
use rstest_bdd_macros::{DataTableRow, given};

#[derive(DataTableRow)]
struct TraceRow {
    service: String,
    duration_ms: u64,
}

#[given("the following recorded traces:")]
fn given_recorded_traces(world: &mut TestWorld, #[datatable] rows: Rows<TraceRow>) {
    for row in rows {
        world.insert_trace(&row.service, row.duration_ms);
    }
}
```

## 5. Scenario Binding (`scenarios!` vs. `#[scenario]`)

Bind `.feature` files in a dedicated test binary (`tests/features.rs`) so all scenarios link once against a single step registry:

```rust
// tests/features.rs
#[path = "features/step_gate.rs"]
mod step_gate;
#[path = "features/steps/mod.rs"]
mod steps;
#[path = "features/world.rs"]
mod world;

use world::world;

rstest_bdd_macros::scenarios!(
    "tests/features",
    fixtures = [world: crate::world::TestWorld]
);
```

Use `#[scenario(path = "tests/features/...", name = "...")]` on an individual `#[rstest]` function when a specific scenario needs custom fixture overrides.

## 6. The 7 Silent-Green Traps in `rstest-bdd 0.6.0` and `step_gate.rs`

`rstest-bdd 0.6.0` contains seven failure modes where a broken or incomplete test suite exits `0`:

1. **`#![no_implicit_prelude]` / macro bypass:** Missing step module imports leave steps unregistered without a compile error until runtime or gate check.
2. **Tied-specificity silent selection:** `find_step()` uses `.max_by(...)` on `SpecificityScore`. When two regex patterns tie in specificity, `rstest-bdd` silently picks whichever registered last instead of failing on ambiguity.
3. **Proc-macro directory walk caching:** `scenarios!("tests/features")` walks the directory inside a proc macro without emitting directory-level `include_str!` tracking. New `.feature` files can leave a cached binary stale unless `step_gate` walks `tests/features` at runtime.
4. **`Rule:` block zero-test generation:** `scenarios!` in `0.6.0` iterates only top-level `feature.scenarios` and silently skips scenarios nested inside Gherkin `Rule:` blocks.
5. **`skip!()` default pass:** Calling `rstest_bdd::skip!()` inside a step marks the scenario as passed by default unless `fail_on_skipped` is enforced.
6. **`@ignore` tag no-op:** Tagging a scenario `@ignore` in a `.feature` file does not skip or flag it unless explicitly configured.
7. **Untyped return value drop:** Step functions returning `bool` (instead of `()` or `Result<(), E>`) have their return value discarded.

### Complete `tests/features/step_gate.rs` Recipe

Include this runtime test inside `tests/features.rs` to check every `.feature` file at test time and fail on undefined, ambiguous, duplicate, or unused steps:

```rust
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

fn feature_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current).expect("read the features directory") {
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

fn expand_row<'a>(raw: &'a str, headers: &[String], row: &[String]) -> Cow<'a, str> {
    let mut text = Cow::Borrowed(raw);
    for (header, cell) in headers.iter().zip(row) {
        let token = format!("<{header}>");
        if text.contains(&token) {
            text = Cow::Owned(text.replace(&token, cell));
        }
    }
    text
}

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

fn matches_of(
    keyword: StepKeyword,
    text: &str,
) -> Vec<(&'static Step, SpecificityScore)> {
    let mut hits: Vec<_> = iter::<Step>
        .into_iter()
        .filter(|step| {
            step.keyword == keyword
                && extract_placeholders(step.pattern, StepText::from(text)).is_ok()
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
                problems.push(format!("Undefined step at {at}: {keyword:?} '{text}'"));
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
            .filter(|step| !used.contains(&(step.keyword, step.pattern.as_str())))
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
        duplicate_steps().iter().filter_map(|group| group.first()).map(|step| {
            format!(
                "Duplicate step definition: {:?} '{}'",
                step.keyword,
                step.pattern.as_str()
            )
        }),
    );
    problems.sort();
    problems
}

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
```

## 7. Strict Clippy and Lint Compatibility

`rstest-bdd` procedural macros expand code that can trip strict workspace Clippy lints:

1. **`clippy::disallowed_methods` (`std::panic::catch_unwind`):** `#[given]`/`#[when]`/`#[then]` macro wrappers call `std::panic::catch_unwind`. Add a narrow `#[expect(clippy::disallowed_methods, reason = "rstest-bdd step wrappers call std::panic::catch_unwind")]` on `mod steps;` in `tests/features.rs`.
2. **`clippy::literal_string_with_formatting_args`:** Typed step patterns such as `#[then("count is {count:u32}")]` look like `format!` strings to Clippy. Add `#[expect(clippy::literal_string_with_formatting_args, reason = "rstest-bdd typed placeholders look like format arguments")]` on `mod steps;`.
3. **Multiline `#[fixture]` bodies:** Keep multi-statement `#[fixture]` functions formatted as standard blocks so Clippy expression/statement lints stay clean.

## Completion

Completion: `rstest-bdd` and `rstest-bdd-macros` are pinned to `=0.6.0`, `step_gate.rs` passes with zero undefined/ambiguous/duplicate/unused steps, and `cargo nextest list --test features` shows the expected scenario count.

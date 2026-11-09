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
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use gherkin::{Feature, GherkinEnv, Step as GherkinStep};
use rstest_bdd::{Step, StepKeyword, StepText, duplicate_steps, extract_placeholders, iter};
use rstest_bdd_patterns::SpecificityScore;

#[test]
fn step_registry_has_no_undefined_ambiguous_unused_or_duplicate_steps() {
    let feature_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/features");
    let feature_files = collect_feature_files(&feature_root);
    assert!(!feature_files.is_empty(), "no .feature files found under {}", feature_root.display());

    let registered: Vec<&'static Step> = iter::<Step>().into_iter().collect();
    let mut used_steps: HashSet<(*const u8, usize)> = HashSet::new();
    let mut undefined = Vec::new();
    let mut ambiguous = Vec::new();

    for path in &feature_files {
        let feature = Feature::parse_path(path, GherkinEnv::default())
            .unwrap_or_else(|err| panic!("failed to parse {}: {err}", path.display()));
        assert!(
            feature.rules.is_empty(),
            "{}: Gherkin `Rule:` blocks are silently ignored by scenarios! in rstest-bdd 0.6.0",
            path.display()
        );

        for step in steps_of(&feature) {
            let keyword = StepKeyword::try_from(step.ty).expect("a step has a keyword");
            let matches = matching_steps(&registered, keyword, &step.value);
            match matches.as_slice() {
                [] => undefined.push(format!("{}:{}: {:?} '{}'", path.display(), step.span.line, keyword, step.value)),
                [only] => { used_steps.insert(step_id(only)); }
                many => {
                    for m in many { used_steps.insert(step_id(m)); }
                    if top_two_tie(many) {
                        ambiguous.push(format!("{}:{}: {:?} '{}'", path.display(), step.span.line, keyword, step.value));
                    }
                }
            }
        }
    }

    let duplicates: Vec<String> = duplicate_steps()
        .into_iter()
        .map(|g| format!("{:?} '{}'", g.keyword, g.pattern.as_str()))
        .collect();
    let unused: Vec<String> = registered
        .into_iter()
        .filter(|s| !used_steps.contains(&step_id(s)))
        .map(|s| format!("{:?} '{}' ({}:{})", s.keyword, s.pattern.as_str(), s.file, s.line))
        .collect();

    assert!(
        undefined.is_empty() && ambiguous.is_empty() && duplicates.is_empty() && unused.is_empty(),
        "BDD step gate failed:\nundefined: {undefined:#?}\nambiguous: {ambiguous:#?}\nduplicates: {duplicates:#?}\nunused: {unused:#?}"
    );
}

fn step_id(step: &Step) -> (*const u8, usize) {
    (step.pattern.as_str().as_ptr(), usize::from(step.line))
}

fn collect_feature_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read features dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "feature") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn steps_of(feature: &Feature) -> impl Iterator<Item = &GherkinStep> {
    let bg = feature.background.iter().flat_map(|b| b.steps.iter());
    let top = feature.scenarios.iter().flat_map(|s| s.steps.iter());
    let rules = feature.rules.iter().flat_map(|r| {
        let r_bg = r.background.iter().flat_map(|b| b.steps.iter());
        let r_sc = r.scenarios.iter().flat_map(|s| s.steps.iter());
        r_bg.chain(r_sc)
    });
    bg.chain(top).chain(rules)
}

fn matching_steps<'a>(registered: &[&'a Step], keyword: StepKeyword, text: &str) -> Vec<&'a Step> {
    registered
        .iter()
        .copied()
        .filter(|s| s.keyword == keyword && extract_placeholders(s.pattern, StepText::from(text)).is_ok())
        .collect()
}

fn top_two_tie(candidates: &[&Step]) -> bool {
    let mut scores: Vec<SpecificityScore> = candidates
        .iter()
        .map(|s| s.pattern.specificity().expect("compile step pattern"))
        .collect();
    scores.sort_unstable_by(|a, b| b.cmp(a));
    scores.len() >= 2 && scores.first() == scores.get(1)
}
```

## 7. Strict Clippy and Lint Compatibility

`rstest-bdd` procedural macros expand code that can trip strict workspace Clippy lints:

1. **`clippy::disallowed_methods` (`std::panic::catch_unwind`):** `#[given]`/`#[when]`/`#[then]` macro wrappers call `std::panic::catch_unwind`. Add a narrow `#[expect(clippy::disallowed_methods, reason = "rstest-bdd step wrappers call std::panic::catch_unwind")]` on `mod steps;` in `tests/features.rs`.
2. **`clippy::literal_string_with_formatting_args`:** Typed step patterns such as `#[then("count is {count:u32}")]` look like `format!` strings to Clippy. Add `#[expect(clippy::literal_string_with_formatting_args, reason = "rstest-bdd typed placeholders look like format arguments")]` on `mod steps;`.
3. **Multiline `#[fixture]` bodies:** Keep multi-statement `#[fixture]` functions formatted as standard blocks so Clippy expression/statement lints stay clean.

## Completion

Completion: `rstest-bdd` and `rstest-bdd-macros` are pinned to `=0.6.0`, `step_gate.rs` passes with zero undefined/ambiguous/duplicate/unused steps, and `cargo nextest list --test features` shows the expected scenario count.

# Research: Prototype BDD inside the e2e binary (ticket 09)

This file supports [09: Prototype BDD inside the e2e binary](../issues/09-bdd-e2e-prototype.md).
It records the test measurements from a throwaway spike in the repo.
The spike folds `rstest-bdd` scenarios into `tests/e2e.rs`.

## 1. Executive Summary & Decisions

1. **Local placement works (Decision: use `tests/e2e.rs` for the pilot).**
   - The spike used `#[path = "e2e/bdd.rs"] mod bdd;`.
   - The task `mise run verify` passed on macOS. The e2e phase found the
     scenario with no runner or CI configuration changes.
   - The `e2e` target compiled with `--no-default-features` and no
     `test-utils`.
   - Linux and Windows runs did not occur. Ticket 08's three-OS condition
     stays unverified. The first adoption CI run must check it.
   - The local evidence does not justify a separate `bdd` test binary.

2. **Build script decision (Decision: Reject `build.rs`, document `touch`).**
   - **With `build.rs`**: Cargo marks the root crate dirty on each `.feature`
     change. When you edit a `.feature` file, Cargo rebuilds the library,
     binary, and tests (6.94s).
   - **Without `build.rs`**: The macro uses `include_str!`. Rustc tracks edits
     to existing `.feature` files through dep-info. It recompiles only
     `tests/e2e.rs` (0.87s).
   - When you add a new `.feature` file, run `touch tests/e2e/bdd.rs` (1.26s).
   - Clean builds and CI always find all feature files.
   - Verdict: Accept documenting `touch`. This prevents a slow 7-second
     rebuild on every edit.

3. **New Dependency Finding: `deny.toml` license rejection.**
   - `rstest-bdd = 0.6.0` pulls in `foldhash = 0.2.0` through `hashbrown
     0.17.1`.
   - `foldhash` uses the Zlib license.
   - The task `cargo deny check licenses` fails with exit code 4.
   - Add `"Zlib"` to `[licenses].allow` in `deny.toml` before adoption.

---

## 2. Evidence Table for Ticket 09 Items

| Item | Expected / claim | Observed result | Limit |
|---|---|---|---|
| `mise run verify` | Full local gate | Passed: 3701 unit, 41 integration, 63 e2e passed (2 skipped), 86 doctests | macOS only |
| `check:layers` | Layer discipline | The task passed | `git grep` did not read the untracked spike files. The gate did not prove coverage of the new paths |
| `check:visibility` | Three baselines | The task passed | No public API or token baseline changed |
| Feature-off e2e | No `test-utils` | `cargo check --no-default-features --test e2e` passed | macOS only |
| Existing feature edit | Incremental rebuild | With `build.rs`: 6.94s, package rebuild. Without it: 0.87s, e2e test rebuild | One warm run of each. Not a benchmark |
| New feature file | Incremental discovery | Without `build.rs`, Cargo did not find the file. `touch tests/e2e/bdd.rs` fixed it (1.26s) | The 6.94s run measured an edit, not a new file |
| Linux and Windows CI | Three-OS condition | Not run. A macOS cross-check failed in native C dependencies | No three-OS approval |
| License policy | `cargo deny check licenses bans sources` | Failed on `foldhash v0.2.0` (Zlib) | Adoption must change `deny.toml` |

---

## 3. Pilot Scenario Specification

### Gherkin (`tests/e2e/features/process_contract.feature`)

```gherkin
Feature: Process-boundary contracts

  Scenario: Unknown subcommand exits 2 with usage on stderr
    Given an isolated sandbox
    When the user runs unknown subcommand nonexistent_command
    Then the process exits with code 2
    And stderr contains unrecognized subcommand
```

### Step Implementation (`tests/e2e/bdd.rs`)

```rust
//! End-to-end BDD scenarios and step definitions.

use rstest::fixture;
use rstest_bdd_macros::{given, scenarios, then, when};

use super::support::{Run, Sandbox};

struct Invocation {
    sandbox: Sandbox,
    run: Option<Run>,
}

#[fixture]
fn invocation() -> Invocation {
    let sandbox = Sandbox::new();
    Invocation {
        sandbox,
        run: None,
    }
}

#[given("an isolated sandbox")]
fn an_isolated_sandbox(invocation: &mut Invocation) {
    let _ = &invocation.sandbox;
}

#[when("the user runs unknown subcommand {subcmd}")]
fn run_unknown_subcommand(invocation: &mut Invocation, subcmd: String) {
    invocation.run = Some(invocation.sandbox.run(&[&subcmd]));
}

#[then("the process exits with code {expected_code}")]
fn exit_code_is(invocation: &Invocation, expected_code: i32) {
    let run = invocation.run.as_ref().expect("command executed");
    run.assert_code(expected_code);
    run.assert_never_exit_101();
}

#[then("stderr contains unrecognized subcommand")]
fn stderr_contains_unrecognized(invocation: &Invocation) {
    let run = invocation.run.as_ref().expect("command executed");
    assert!(
        run.stderr.contains("unrecognized subcommand"),
        "stderr: {}",
        run.stderr
    );
}

scenarios!("tests/e2e/features", fixtures = [invocation: Invocation]);
```

### Entry Wiring (`tests/e2e.rs`)

```rust
#[expect(
    clippy::disallowed_methods,
    reason = "rstest-bdd macro wrapper generates catch_unwind calls"
)]
#[path = "e2e/bdd.rs"]
mod bdd;
```

---

## 4. `check:layers` follow-up

Ticket 08 requires the gate to scan the chosen feature and step paths.
It must stop when `git grep` exits with code 2 or higher.

The current task passed during the spike. The new files were untracked, so
`git grep` did not read them. The pass does not prove that the gate checked
the scenario or step definitions.

The task also uses `|| true` on three `git grep` calls. That hides errors
such as exit code 128. The first `git grep` is in a pipeline, so it needs
its own error handling. A change must cover both cases. It must use the
directory that ticket 11 chooses.

Do not reuse the earlier draft diff. It did not check the status of the
first pipeline. Its Gherkin `#` comment filter also ignored the filename
prefix that `git grep -n` adds to each line.

No `mise.toml` change was made. The adoption slice must write and test the
gate change before it relies on the gate.

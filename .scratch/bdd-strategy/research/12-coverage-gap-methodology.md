"# Research 12: Coverage-gap methodology, evidence dossier

This file supports
[12: Coverage-gap methodology](../issues/12-coverage-gap-methodology.md).
The ticket is claimed. The grilling has not started yet.

This dossier consolidates five parallel research passes into one document.
The passes are: a survey of the repo's quality machinery, a verified
contradiction triage (the input for decision (f)), and three external
best-practices reviews. The reviews cover specification coverage and gap
detection, scenario-quality rubrics and anti-metrics, and Rust mutation and
coverage tooling.

Method: two codegraph-driven codebase passes (machinery, contradictions), plus
three primary-source literature passes. Every claim carries a citation. Repo
facts cite a file path and line. External sources cite a URL. Claims labeled
**Inference** are reasoned from sources but are not stated by any source.

This file obeys ASD-STE100, with one exception. Verbatim quotations from
sources stay exact. A quotation is a protected fact. The checker may warn on
a long quotation. Do not change the quotation.

---

## 0. Integration: what this evidence means for decisions (a)-(f)

### (a) What "covered" means per layer

External grounding: a behavior is *specification-covered* if and only if an
executable example both documents it and enforces it (section 2.1). The basis
is the Specification by Example identity: requirements, specifications, and
acceptance tests are the same thing. Mere exercise does not count.

Repo reality: the enforcement machinery per layer exists and behaves
correctly. The canonical phases use `--no-tests fail`. The gates diff frozen
baselines. The proptests are inline. However, nothing records which guarantee
each test enforces. The invariance map of 88 guarantees has no test column.
The only ledger that exists records deletions. No guarantee-to-test mapping
exists anywhere (sections 1.7 and 1.9). Human review can establish "covered".
A query cannot.

A second fact: no quality threshold is active in the repo. `min_msi` is
commented out. CRAP has `fail-above = false`. No llvm-cov fail-under exists.
Mutation and coverage do not run in CI or in `verify` (section 1.9). Any
coverage definition lands on a substrate that has no thresholds.

### (b) Gap detection: a method a session can run

One ledger method binds this repo. It comes from the integration-e2e-suite:
four columns (behavior, layer, defect class, surviving test). Ticket 08
adopted it (section 1.7).

The sources support one lightweight method (section 2.4):

1. Maintain a **rule inventory** as the denominator. Research/10's 88
   guarantees are the ready-made seed.
2. Point each scenario at its rule. Use a tag or a naming convention.
3. Define the **gap list** as: rules with no enforcing scenario, plus open
   questions.
4. Review the list on Wynne's cadence. Use Adzic's three questions as the
   script.

No Rust tool exists for guarantee-to-test mapping (section 2.6). One novel
mechanism fits this repo. rstest-bdd errors at compile time when a `tags`
filter matches nothing. One tagged probe per inventory rule would turn an
unproven rule into a compile failure. This claim is an **Inference** from
documented behavior (section 2.5).

One tension needs a decision in the grilling. Ticket 11 fixed a flat feature
layout and domain-concept step files. Tags would add a second naming axis.
Decide where the mapping rides: on tags, on the inventory file itself, or on
review only.

### (c) Scenario-quality criteria

Six rubric candidates exist, all sourced (section 3.1):

1. Behaviour-level wording that survives implementation change.
2. Three to five steps, one rule per scenario, illustrative not exhaustive.
3. Concrete, realistic example data.
4. Assertions on observable outcomes and diagnostic codes. This is already
   repo law through ticket 07's channel rule.
5. A failure localizes a business rule without a source read.
6. Quality is produced in review with a checklist, not at authoring.

One gap exists. The only peer-reviewed review checklist (SBES '19) is
paywalled. Its questions were not retrieved (section 3.5).

### (d) Anti-metrics

Pre-reject these numbers (section 3.6):

- Coverage percentage as a gate or score (Kaner, Marick, Google, BBST,
  North).
- Scenario counts, test counts, and pass-rate as KPIs (Goodhart through
  DORA; Manheim).
- Any candidate metric that fails Kaner and Bond's foreseeable-side-effect
  admission question.

Two popular aphorisms are untraceable. Do not cite them as sourced rules.
The rules are "6-word scenario title" and "code coverage is a floor, not a
ceiling" (section 3.5).

### (e) Mutation and coverage tooling disposition

Mutation: an unchanged posture is the evidence-backed answer. No primary
source recommends a universal survivor-rate threshold. Google's production
service deliberately avoids absolute scores. It triages new survivors per
change (section 4.5). The repo's existing baseline and `--fail-on-escaped`
workflow already matches that practice.

Two facts sharpen the decision:

- mutarust runs plain `cargo test` per mutant. It does not run nextest. The
  `--exec` path is serial. The "nextest is decisive" preference binds the
  test and coverage phases. It does not bind the mutation runner (section
  4.2).
- Upstream framing: mutation grades whether the scenarios' assertions would
  notice a behavior change. An escape is fixed by a stronger
  public-interface test, that is, a scenario. Not by a tighter unit test
  (section 4.6).

One cost fact applies. Every mutant re-runs all e2e scenarios. Scoping
(`-m`, `-f`, `--git-diff`) is the documented lever. Both engines support it
natively (section 4.7).

Coverage: one empirical check is needed before the numbers can mean anything
for scenarios. Spawned-binary coverage is mechanically plausible. LLVM docs
cover test harnesses that invoke other programs. cargo-llvm-cov ships an
external-tests recipe. However, the exact path `cargo llvm-cov nextest` +
CARGO_BIN_EXE + rstest-bdd is an **Inference** until one run counts profraw
files (section 4.4).

A separate fact: the coverage tasks run the e2e phase with `--all-features`.
That is not the shipped configuration (sections 1.4, 1.9).

### (f) Contradictions

The full verified dossier is section 5. All 13 families plus 6 hygiene items
still exist as of 2026-10-29.

Scenario-candidate blockers need a recorded disposition before a scenario may
use the guarantee (ticket 07, rule 3):

| Item | Contradiction | Priority |
| --- | --- | --- |
| 2.1 | ADR-0002 says a moved dir is "stale". The code reports `Untrusted`. | Named by the ticket |
| 5 | ADR-0008 names `SourceAtom` variants that do not exist. | Named by the ticket |
| 10 | The glossary claims slugs, callouts and footnotes. No code implements them. | Named by the ticket. Research/10 already excludes them. The disposition is the trivial "drop". |
| 1.1 | The `CommandOutcome` doc claims exit 0 for both variants. `Interrupted` exits 130. | Touches G1 |
| 1.2 | ADR-0004 promised a Ctrl-C-130 process-level contract test. Only an in-process pin exists. | Narrow. One Unix scenario is planned. |
| 2.2 | ADR-0002 says "existence check only". The code reads and hashes. | Partial. The stage order (G22) holds. The performance claim does not. |
| 4 | Two sort tests are named "…_ascending_by_default". Both set `asc: true`. | A hazard. This is evidence *against* G6. Do not mine it. |
| 7 | The template-resolution glossary disagrees with the per-directory loader order. | Blocks the local-stem-vs-global-exact scenario |
| 8 | ADR-0009 has a duplicate number. Both copies are `proposed`. Both have landed. | A citation hazard for date-literals B4 |
| 13 | `yaml.rs` states four quirks. No test pins any of them. | Partial. The parity rule G8 is unaffected. |

Non-candidates: items 3, 6, 9, 11, 12, and B1-B6. They are doc hygiene or
gate-owned. They block no scenario. They mostly belong to ticket 13 or to
ordinary doc fixes.

### Open empirical questions for the grilling

1. **Spawned-binary coverage.** Make one `cargo llvm-cov nextest` run over an
   e2e-scenario-heavy subset. Count the profraw files. Spot-check one src
   line that executes only through the spawned CLI. The result decides
   whether llvm-cov numbers see scenario-executed code at all (section 4.4).
2. **Machinery hygiene preconditions.** Each item needs one decision: record
   it, fix it now, or defer it.
   - `check:layers` wraps 3 of 4 greps in `|| true`. This violates ticket
     08's own exit-code requirement. It is the exact bug that kept the gate
     dead for 224 commits (section 1.2).
   - `test:unit` lacks `--no-tests fail` (section 1.1).
   - Coverage runs e2e with `--all-features` (section 1.4).
   - The CRAP threshold doc says 20.0. The config says 15.0 (section 1.4).
   - `src/lib.rs:181` says 46 facade identifiers. The baseline holds 47
     (section 1.6).
   - `proptest-regressions/` does not exist. Ticket 08(c) says to commit
     those files (section 1.5).

### Evidence-quality caveats

- The SBES '19 scenario-quality checklist is paywalled. Its contents were not
  retrieved.
- Marick's "How to Misuse Code Coverage" has a dead original PDF at
  testing.com. Cite the examper.com mirror.
- specificationbyexample.com is offline. SBE chapter 3 is paywalled.
  mutants.dev and impactmapping.org were unreachable. No claim rests on them.
- Feature Injection belongs to Chris **Matts**, not "Chris George". The
  ticket brief had an error. Section 2.2 carries the correction.
- Machinery counts were measured with `rg` on 2026-10-29. A subagent's task
  enumeration incidentally refreshed `mise.lock`. The change was left in
  place because the worktree is shared.

---

## 1. The repo's quality-measurement and quality-gate machinery

This section is a read-only survey of what exists and is wired today. Every
claim cites a file path with line numbers, or a shell command with its
output. Ticket 08's layer architecture (gates, proptests, BDD scenarios,
e2e, integration, unit) is a given. This section does not re-derive it.

### 1.1 Test phase wiring

| Source | Finding |
| --- | --- |
| `mise.toml:31-37` | `[tools]` installs `github:nextest-rs/nextest`, `cargo:cargo-llvm-cov`, `cargo:cargo-crap`, `messrust`, `cargo:mutarust`, `cargo:cargo-shear`. All are `latest` except messrust, which is `branch:main`. |
| `mise tasks` output | 35 tasks. Test-relevant tasks: `test`, `test:doc`, `test:mutants`, `test:unit`, `verify`, `coverage`, `coverage:html`, `coverage:lcov`, `crap`, `mess`, `lint`, `check`, `check:layers`, `check:visibility`. |
| `.mise/tasks/test/_default:1-28` | The canonical `test` task runs "lib/bin units (production), integration (test-utils), E2E (shipped binary), doctests (all-features)". Line 5 sets `#MISE extends="gate:never-skip"`. |
| `.mise/tasks/test/_default:98-121` | Four phases run in order. (1) `unit` = `nextest run -p traces --no-default-features --lib --bin traces`. (2) `integration` = `nextest run -p traces --features test-utils --test integration`. (3) `e2e` = `nextest run -p traces --no-default-features --test e2e`. (4) `docs` = `mise run test:doc --feature all`. The `--no-docs` flag skips phase 4. |
| `.mise/tasks/test/_default:68-82` | `run_phase()` emits `cargo nextest run -p traces --no-tests fail "$@"`. An unfiltered empty phase is a hard failure. Filtered runs pre-check with `cargo nextest list` and skip empty phases. If no phase matches the filter, the task fails (lines 111-114). |
| `.mise/tasks/test/doc:79-85` | `cargo test --workspace --all-features --doc -- --quiet`. This is not nextest. Doctests are libtest-only. |
| `.mise/tasks/test/unit:116-122` | An ad-hoc runner: `cargo nextest run "${unit_args[@]}"`. It has **no `--no-tests fail`**. A zero-match filter exits 0 here. The feature flag defaults to `--all-features` (lines 56-67). |
| `.config/nextest.toml:6-19` | Only `[profile.default]` exists: `status-level = "slow"`, `slow-timeout = { period = "30s", terminate-after = 2 }`, `global-timeout = "20m"`, `success-output = "never"`. |
| `Cargo.toml:49-51` | `[[test]] name = "integration"` carries `required-features = ["test-utils"]`. This is the silent-green guard. The `e2e` target has no `[[test]]` entry. Cargo auto-discovers it from `tests/e2e.rs`. |
| `Cargo.toml:279-282` | `[profile.mutants]` = `inherits = "test"`, `opt-level = 1`, `debug = false`. This is a cargo profile. Only the mutants task consumes it. |
| `Cargo.toml:169-170` | The only feature is `test-utils = []`. No `default` feature exists, so production config = `--no-default-features` (`.github/workflows/ci.yml:80-82`). |

#### What gates `mise run verify`

| Source | Finding |
| --- | --- |
| `mise.toml:570-580` | `verify` runs `fmt`, then runs `check`, `lint --no-fix`, and `test` concurrently. It sets `extends = "gate:never-skip"`. |
| `mise.toml:98-121` | `check` = `cargo check --workspace`, plus `hk check` with 4 skips, plus `mise run check:visibility`, plus `mise run check:layers`. |
| `hk.pkl:107-129` | The `check` context skips cargo-clippy, cargo-test, cargo-doc and cargo-deny. The **visibility and layer-enforcement steps still run inside `check`**. They run twice per `verify`. |
| `hk.pkl:131-172` | `pre-commit` runs format and validate only. `pre-push` runs validate, security, quality, and cargo-sweep. `check` runs all four groups. |
| `.mise/tasks/lint:138-142` | `lint` = `cargo clippy --workspace --all-targets --all-features` with `-D warnings`. `verify` passes `--no-fix`. |
| `mise.toml:602-604` | The `gate:never-skip` template sets `outputs = []` and disables cache. A gate can never be freshness-skipped. |

The tasks below are **not** in `verify`: `coverage`, `coverage:lcov`,
`coverage:html`, `crap`, `mess`, `test:mutants`, `bench`, `audit`,
`modules:*`, `selftest`. The `mess` task describes itself as "manual, never
wired into `verify`/hk/CI" (`mise.toml:506`).

#### CI parity

| Source | Finding |
| --- | --- |
| `.github/workflows/ci.yml:27-43` | The `check` job runs raw `cargo check --workspace --all-targets --all-features`. It does not call `mise run check`. |
| `ci.yml:59-73` | The `visibility` job runs `mise run check:visibility`, then `mise run check:layers`. |
| `ci.yml:83-101` | `test-unit` = two nextest runs (`-E 'kind(lib)'`, `-E 'binary(traces)'`). Both use `--no-default-features --no-tests fail`. |
| `ci.yml:103-118` | `test-integration` = `--features test-utils -E 'binary(integration)' --no-tests fail`. |
| `ci.yml:120-141` | `test-e2e` = `--no-default-features -E 'binary(e2e)' --no-tests fail`. The job runs a matrix over ubuntu, macos and windows. |
| `ci.yml:143-164` | `test-doctests` = `cargo test --doc --all-features`, plus a grep guard. The guard requires `^test result: ok. [1-9][0-9]* passed;`. "No doctests ran." exits 1. |
| `ci.yml:167-191` | The `hygiene` job runs `hk check --all --no-fail-fast` with 11 skips. |
| `ci.yml`, whole file | No coverage job exists. No mutation job exists. No `crap` job exists. No `bench` job exists. No `verify` job exists. |

### 1.2 Layer gates

#### `check:visibility` (mise.toml:123-146)

| Source | Finding |
| --- | --- |
| `mise.toml:130-134` | (a) `git grep -E 'pub use .*\*' src/` fails the task on any glob re-export in `src/`. |
| `mise.toml:136-137` | (b) `cargo public-api --color=never \| diff -u tests/baselines/public-api-default.txt -`. The default-features public API must be byte-identical to the frozen baseline. |
| `mise.toml:139-140` | (c) The same check with `--features test-utils` against `public-api-test-utils.txt`. The `traces::testing` facade must be byte-identical. |
| `mise.toml:142-143` | (d) A snapshot diff of every `pub`-declaration line in `src/` (normalized and `LC_ALL=C sort`) against `pub-tokens.txt`. |

`check:visibility` scans `src/` only. It does not read test code in `tests/`.
Dev-dependency additions do not move these baselines.

#### `check:layers` (mise.toml:148-190)

The gate scans only these paths: `tests/e2e.rs`, `tests/e2e/`,
`tests/integration.rs`, `tests/integration/`, and `tests/common/` **if that
directory exists** (`mise.toml:157-160`). The directory does not exist. A
delete in commit `7b71ed06` removed it.

| Source | Finding |
| --- | --- |
| `mise.toml:162-166` | (a) E2E must not reference the crate. The pattern matches `traces::` and related forms. The check excludes string literals, backticks, and `//` comment lines (line 163). |
| `mise.toml:168-173` | (b) `process` or `std::process` is forbidden outside `tests/e2e/support.rs`. The task reads matches from `e2e_process_refs` and exits 1 on a non-empty value. |
| `mise.toml:175-180` | (c) `tests/integration/` must not use `process` or `CARGO_BIN_EXE` at all. No `support.rs` exemption applies. |
| `mise.toml:182-187` | (d) `git grep -n -E 'set_current_dir' tests/` is forbidden anywhere under `tests/`. |

The gate has these factual gaps:

1. It makes no rule about `src/`. Unit and proptest code is out of scope.
2. It cannot see untracked files. `git grep` reads tracked files only. The
   ticket-09 spike passed while its new files were untracked
   (`research/09:45,140-143`).
3. It does not scan `*.feature` text (`map.md:151-152`).
4. The path list at `mise.toml:162,168,175` is a hardcoded literal. A future
   `tests/features/` or `tests/e2e/bdd/` path is absent from it.
5. Three of the four greps are wrapped in `|| true` (`mise.toml:168,175,182`).
   A `git grep` exit of 2 or 128 is swallowed. Ticket 08 requires the gate to
   "fail when `git grep` exits with code 2 or higher" (`issues/08:86-87`).
   `mise.toml` does **not** implement that requirement today.

Historical context for gap 5: the gate was silently dead for 11 days and 20
hours, that is, 224 commits. A deleted `tests/common/` pathspec made
`git grep` exit 128. The `if` condition treated that as "no violation"
(`research/08-verify-suite-decisions.md:37-67`).

### 1.3 Mutation testing (`test:mutants`)

The engine is **mutarust**, not cargo-mutants. The task name is a legacy
entrypoint (`docs/refs/mutation_testing.md:30-31`).

| Source | Finding |
| --- | --- |
| `.mise/tasks/test/mutants:1-6` | The task sets `#MISE depends=["test"]`. A mutants run is always preceded by a full `mise run test`. |
| `.mise/tasks/test/mutants:127-129` | `TEST_FLAGS="--features test-utils --all-targets --profile mutants"`. `TIMEOUT_COEFFICIENT=5`. `DEFAULT_TARGET="./src..."`. |
| `.mise/tasks/test/mutants:287-306` | The task always passes `--config mutarust.yml --logger-agentic-json`. Unless `--dry-run` is set, it also passes `--test-flags "$TEST_FLAGS"` and `--timeout-coefficient 5`. |
| `.mise/tasks/test/mutants:327-350` | `-f <file>` and `-m <module>` set the scope. A bare invocation defaults to `./src...`, that is, production code only. |
| `.mise/tasks/test/mutants:370-387` | Gate flags are **opt-in**: `--min-msi`, `--min-covered-msi`, `--fail-on-escaped`, `--update-baseline`, `--git-diff` (auto-pairs `--ignore-msi-with-no-mutations`). None is enabled by default. |
| `mutarust.yml:12-14` | `json_output: true`. `html_output: true`. `silent_mode: false`. |
| `mutarust.yml:22-23` | `exclude_dirs: [src/cli]`. This is the port of the old cargo-mutants `src/cli/**` exclude. |
| `mutarust.yml:28-29` | `disable_mutators: ["select/*"]`. The reason is zero yield for this repo. |
| `mutarust.yml:35-36` | `skip_without_test: false` and `skip_with_cfg: false` are both off. The config warns that enabling either needs a `--dry-run` A/B. |
| `mutarust.yml:38-46` | **`min_msi` is commented out.** The comment says "enable ONLY if measured MSI >= 60". Full-project measurement is deferred. The recorded measurement is scoped MSI = 0.8333 on `strsim`, dated 2026-09-28 (`quality_gates_mise_adoption.md:931-946`). |
| `mutarust.yml:50-51` | `enable_mutators: []` (all enabled minus `disable_mutators`). `ignore_source_lines: []`. |
| `mutarust-baseline.json` | A committed accepted-escapes baseline. `version: 1`. Exactly 1 mutant: `src/strsim.rs:15`, mutator `numbers/incrementer`. |
| `report.json` (gitignored) | The last recorded run was a dry-run: `totalMutantsCount: 0, msi: 0.0` (`.gitignore:29`). |
| `docs/refs/mutation_testing.md:141-147` | CI integration is labelled "**aspirational — no CI job today**". The suggested future job is `test:mutants --git-diff --min-msi 60 --fail-on-escaped`. |
| `mise.toml:589-592` | `[task_templates.mutants]` sets `timeout = "16h"` and disables cache. |

Net: mutation testing is fully wired as a local, opt-in, ungated tool. No MSI
threshold is active anywhere. No CI job exists. The baseline holds one
accepted escape. The only default-on behavior is `depends=["test"]`.

### 1.4 Coverage (llvm-cov) and CRAP

| Source | Finding |
| --- | --- |
| `mise.toml:68` | `COVERAGE_IGNORE_REGEX = 'src/main\.rs$\|src/cli($\|/)\|src/(.*/)?error\.rs$'`. cargo-llvm-cov consumes this as `--ignore-filename-regex`. A comment warns that an invalid POSIX ERE silently disables ALL exclusions. |
| `mise.toml:416-424` | `coverage:lcov` = `cargo llvm-cov nextest --workspace --all-features --lcov --output-path lcov.info --ignore-filename-regex "$COVERAGE_IGNORE_REGEX"`. `--lcov` conflicts with `--html`, so two tasks exist. The task sets no `--no-clean`. A stale `*.profraw` would merge. |
| `mise.toml:426-484` | `coverage:html` = `cargo llvm-cov nextest --workspace --all-features --html`, with an optional `-m <module>` scope. The scope excludes every sibling `src/<dir>` and every `src/*.rs`, because llvm-cov has no include-file flag. The task rejects modules that are already globally excluded (lines 452-477). |
| `mise.toml:448-449` | A comment states: "cargo-llvm-cov appends its default ignores (rustc/registry/target/tests...) to whatever we pass here". |
| `mise.toml:486-488` | `coverage` depends on `coverage:html`. |
| `mise.toml:490-497` | `crap` depends on `coverage:lcov`, then runs `cargo crap --lcov lcov.info`. `.cargo-crap.toml` is a declared cache input. |
| `.cargo-crap.toml:9-14` | `threshold = 15.0`. `fail-above = false`. `missing = "pessimistic"`. `exclude = ["tests/**", "benches/**"]`. |
| `docs/refs/crap_metric.md:51-62` | **Contradiction.** The doc presents the project config as `threshold = 20.0`. The live file says `15.0`. The doc also says "Set to true to enforce gating in CI" for `fail-above`, which is `false`. |
| `ci.yml` and `hk.pkl` | No coverage step exists. No CRAP step exists. Neither a job nor a hook. |
| Coverage thresholds | **None.** A search for `fail-under`, `fail_under`, or `--threshold` in `mise.toml`, `.mise/` and `docs/refs/*.md` returns only `.mise/scripts/size_gate.sh:102`. That is a disk-size gate for `clean`/`sweep`. It is unrelated. |
| `lcov.info` (gitignored, `.gitignore:25`) | Last written 2026-10-08. 55,435 lines over 97 source files (`rg -c '^SF:' lcov.info`). The task has run, but the artifact is stale. It is not a gate. |

Which tests coverage includes: `--workspace --all-features` makes every
nextest target run with all features on. The `integration` target builds,
because its required-features is satisfied. The `e2e` target also runs **with
`test-utils` enabled**. That is not the shipped configuration that
`mise run test` uses for E2E (`_default:106-109`). No binary or phase filter
is applied. The only exclusions are filename-based. No line-level or
test-level exclusion exists.

Nextest coupling: coverage does run under nextest (`cargo llvm-cov nextest`,
`mise.toml:424` and `:481`). It inherits `.config/nextest.toml`'s 20 m global
timeout and 30 s slow-timeout.

### 1.5 Test inventory

All counts come from `rg` over the working tree on 2026-10-29, excluding
`target/`.

| Command | Result |
| --- | --- |
| `rg -c '^\s*#\[test\]' src` (sum) | **2,398** `#[test]` functions in `src/`, across **128 files** |
| `rg -c '#\[cfg\(test\)\]' src` (sum) | 197 `#[cfg(test)]` markers |
| `rg -c '^\s*#\[rstest' src` (sum) | 290 `#[rstest…]` attributes in `src/` |
| `rg -c '^\s*#\[case' src` (sum) | **1,885** `#[case]` rows. The bulk of the suite is parametrized. |
| `tests/integration/` | 37 `#[test]` and 1 `#[rstest]`, in 4 files: `persistence.rs` (3), `query.rs` (18), `refresh.rs` (2), `template_render.rs` (14) |
| `tests/e2e/` | 64 `#[test]` in 13 files. `support.rs` has 0 tests. `config_fixture.rs` has 5 tests. |
| `tests/*.rs` roots | `tests/integration.rs` is 13 lines of module wiring. `tests/e2e.rs` is 31 lines of module wiring. |
| Doctests | 76 at audit HEAD (`test-arch-audit.md:48-49`), 83 at ledger time (`issues/15:39`) |
| Full-suite snapshot | 3,127 nextest tests in about 11 s (`test-arch-audit.md:48-49,95,98`: 3,070 lib+bin, 30 integration, 27 e2e) |

Per-file `#[test]` distribution inside `src/`, top modules: `template` 373,
`query` 318, `schema` 311, `note` 304, `cli` 224, `index` 208, `temporal`
194, `config` 116.

#### Proptests

| Source | Finding |
| --- | --- |
| `Cargo.toml:158` | `proptest = "1.12.0"` is a dev-dependency. |
| `rg -n 'proptest!' src` | Exactly **6** `proptest!` blocks exist. All are in `src/temporal/`: `point.rs:3222`, `duration.rs:1614`, `fmt.rs:247`, `date.rs:1220`, `datetime.rs:1968`, `datetime.rs:1990`. |
| `rg -c 'proptest!' tests` | 0. No proptest exists in `tests/integration/` or `tests/e2e/`. |
| `rg -n '#\[proptest\]'` | 0. The attribute-macro form is unused. Only the `proptest! { … }` block form is in use. |
| `rg -n 'cargo-proptest'` | 0 hits. No `cargo-proptest` binary, mise tool, or task exists. In ticket 08, "proptest" maps to the `proptest` crate and inline `proptest!` blocks. |
| `find . -name proptest-regressions` | **No `proptest-regressions/` directory exists** in the repo. One copy exists inside `.rust-docs/cache/`. Either no regressions have been recorded, or none exist. |
| `issues/08:80-83` | Decision (c): "Proptests stay inline in `src/`. Commit `proptest-regressions/` files. No CI rule for them." Growth candidates wait for ticket 10: path codec, note wire format, `NoteFieldValue` order, table escaping, tag containment. |

#### The `Proves`, `Defect class:` and `Executes vs asserts:` conventions

| Command | Result |
| --- | --- |
| `rg -c 'Proves' tests` and `src` | 34 lines in `tests/`, 4 in `src/`. Total: 38. |
| Breakdown in `tests/` | 5 file-header statements (`//! Proves`, for example `tests/e2e/trust.rs:1`) and 28 per-test statements (`/// Proves` at column 0). |
| `src/` occurrences | `src/index/store.rs:2532`, `src/template/engine/schema.rs:912`, `src/template/engine/ui.rs:289`, `src/template/service.rs:1144` |
| `rg -c 'Defect class:' tests` and `src` | 33 in `tests/`, 1 in `src/`. Total: 34. |
| Distinct tags | **15**. The top five: `unwitnessed-contract` ×6, `assumed-hermeticity` ×5, `stale-secondary-index` ×4, `disk-config-untested` ×4, `trust-before-parse confusion` ×3. Ten more tags appear once each. |
| `rg -c 'Executes vs asserts' tests` | 33, all in `tests/`. The files: `config.rs` 9, `env_policy.rs` 5, `index.rs` 5, `template.rs` 3, `process_contract.rs` 2, then 1-2 each in 7 more files. |

**Where documented:** nowhere in `docs/`. A search for `Proves|Defect class`
over `docs` returns 0. The only written description is
`research/05:158-177` (§G9) and `:332-341` (§4). It calls the taxonomy a
"de facto" contract, "free-form and already drifting". It states that no
taxonomy or rule is documented anywhere. `map.md:49-50` lists both
conventions as an evidence source for the BDD effort. `map.md` does not
record their fate. Ticket 14 owns that decision (`research/05:396`).

A counting note: `research/05` G9 says "34 uses … 16 distinct tags". A direct
grep gives 38 `Proves` lines and **15** distinct `Defect class:` tags. The
research's own tag list enumerates 15 names while asserting 16. The
off-by-one is inside the research note, not in the code.

### 1.6 `test-utils` and the three baselines

**No `test-utils` crate exists.** `test-utils` is a Cargo feature
(`Cargo.toml:170`). It gates two things:

| Source | Finding |
| --- | --- |
| `Cargo.toml:49-51` | `[[test]] name = "integration"` sets `required-features = ["test-utils"]`. Without the feature, `cargo test --test integration` hard-errors instead of silently running 0 tests. This is the silent-green guard (`integration-e2e-suite/issues/03`). |
| `src/lib.rs:178-214` | `#[cfg(any(test, feature = "test-utils"))] pub mod testing` is the sole test-only export surface. Its doc comment says: "The membership of this module is frozen: exactly the **46** identifiers…". |
| `src/lib.rs:185-213` | The actual count is **47** (`research/05` §2; `08-verify-suite-decisions.md:96-124`). The commit `728696d0` added `DriftCause`. **The lib doc at `src/lib.rs:181` still says 46.** This is a live doc/code contradiction. |
| `tests/baselines/public-api-test-utils.txt` | 4,451 lines. This is the machine-authoritative list of the 47 `traces::testing::*` names. |

The "three baselines":

| Baseline | Lines | Produced by | Enforced by |
| --- | --- | --- | --- |
| `tests/baselines/public-api-default.txt` | 2,560 | `cargo public-api --color=never` | `mise.toml:137` |
| `tests/baselines/public-api-test-utils.txt` | 4,451 | `cargo public-api --features test-utils` | `mise.toml:140` |
| `tests/baselines/pub-tokens.txt` | 1,568 | `git grep -E '^\s*pub(\([^)]*\))?\s+' src/`, normalized and sorted | `mise.toml:143` |

Total: 8,579 lines. Ticket 07 sets the hard gate: "BDD adoption changes none
of the three baselines" (`map.md:111-113`; `issues/09:18`;
`research/08-integration-e2e-suite-disposition.md:24`).

Two other things are called "baseline" in this repo. Do not confuse them.
`mutarust-baseline.json` holds accepted mutation escapes (section 1.3).
Criterion baselines live under `.criterion/`, managed by `bench`/`selftest`
(`.mise/tasks/selftest:341-397`).

Fixture support behind the feature lives in `src/lib.rs`'s `test_support`
module (`src/lib.rs:213-230`): `TestProject`, `fixture_service`,
`create_trusted_project`, `write_note`, `build_test_index`, and more. The
`traces::testing` facade re-exports them.

### 1.7 Existing guarantee-to-test mapping and ledger artifacts

#### What exists

| Artifact | What it maps | Source |
| --- | --- | --- |
| **Duplication ledger**, the real "ledger" | behavior → former layer → defect class → surviving test. D1-D12 rows. | `.scratch/integration-e2e-suite/issues/15-recompute-duplication-ledger.md` (118 lines; row shape at lines 3-6 and 20-24; table at 45-58) |
| Pruning execution of that ledger | a per-deletion checklist | `integration-e2e-suite/issues/16-pruning-per-ledger.md` (121 lines) |
| The spec requirement | "Every deletion carries a ledger entry: behavior, former layer, defect class, surviving test. Test-count deltas are bookkeeping, not a gate." | `integration-e2e-suite/spec.md:334` (also `:49`, `:156`, `:345`, `:567`) |
| The binding decision record | "Binding: layer invariants, hermetic sandbox, baselines and gates, four-phase runner, **ledger method** (its issues 15 and 16). Not binding: its per-test classification" | `issues/08:96-99` |
| **Invariance map**, the closest thing to a guarantee ledger | 88 guarantees that pass ticket 07 conditions 1-2. Each has an ID, a prose statement, and an arch-ticket-16 tier (F / M / F+M / D / Free). Grouped per inventory. | `research/10-invariance-map.md` (211 lines; tiers at lines 5-26; list starts line 30) |
| Per-test provenance tables, e2e | File, Test Function, **Proves**, Process-Boundary?, Contract vs Incidental, Twin, Fragility, **Class** (`KEEP-as-e2e`, `CANDIDATE-scenario`, `MISPLACED`, `WEAK`, `DUPLICATE`) | `research/08-audit-e2e.md:45` |
| Per-test provenance tables, integration | Test Function, What It Proves, Nature, Twins & Parity, Refactor Stability, Assertions & Time/Env, Classification (`KEEP-as-integration`, `MISPLACED-unit`, `DUPLICATE`) | `research/08-audit-integration.md:48` |
| Ticket 13 mandate | "Ticket 13 builds a fresh ledger from the audits": 5 misplaced e2e units, 10 misplaced integration units, 6 duplicates, 7 weak assertions, 1 test on a condemned seam, **27 e2e tests that are scenario candidates** | `issues/08:101-111` |
| Guarantee inventories | the five research inventories 01-05 with pinned/unpinned status. Their unpinned flags feed ticket 12. | `research/{01..05}-*.md`; ticket 12 inputs at `issues/12:13-24` |

Two things are called "ledger" and they are unrelated. The tactical
findings ledger at `.scratch/arch-design/findings-ledger.md` holds local
defects. It is not on this map's route. Three review ledgers live under
`.design/`. They hold hypothesis EAV scores for design reviews.

#### What does not exist

| Command | Result |
| --- | --- |
| `find . -iname '*ledger*'` | Only the files listed above. **No guarantee→test mapping table exists anywhere.** |
| `rg -n -i 'guarantee' tests/` | 2 hits, both incidental prose: `tests/e2e.rs:3` and `tests/e2e/query.rs:145`. |
| `rg -c -i 'guarantee' src/` | 5 files, all prose in module or impl docs. |
| `ls src/tests` | No such directory. All unit tests are inline `#[cfg(test)] mod …` (197 markers). |
| `rg -ln 'Proves\|Defect class' docs/` | 0. No doc references either convention. |

Implication for ticket 12: the *method* for a guarantee↔test ledger exists
(issues 15/16 four-column rows) and is declared binding. No ledger of
guarantees→tests has been produced. The nearest inputs are the invariance map
(88 guarantees, tiered) and the two audit per-test tables. Neither is a
guarantee-keyed mapping. Neither is machine-checkable.

### 1.8 Standing constraints, as recorded

| Source | Finding |
| --- | --- |
| `map.md:29-30` | "Nextest compatibility is decisive. All four test phases run under cargo-nextest." |
| `map.md:31` | "Specs define *what the system guarantees*, never *how it works*." |
| `map.md:33` and `:169` | "Build no custom infrastructure beyond adopting an existing crate." The map repeats this as an out-of-scope item. |
| `map.md:34-35` | "Obey the deny-lints (`panic`, `exit`, `print_stdout` and `unwrap` outside tests)." |
| `Cargo.toml:192,199,201,211` | `exit = "deny"`, `panic = "deny"`, `print_stdout = "deny"`, `unwrap_used = "deny"`. Also `expect_used = "deny"` (line 193), `todo`/`unimplemented`/`unreachable` denied (205, 208, 209), `unsafe_code = "deny"` (256). `print_stderr = "allow"` (241). |
| `research/05:177-182` (G10) | Fixture-only `.expect()` is scoped by `#![expect(clippy::expect_used, reason = "fixture-only code")]` at `src/lib.rs:233-238`. |
| `map.md:36-37` | "Obey the machine-enforced layer gates (`check:layers`, `check:visibility`). Do not route around them." |
| `map.md:115-124` | Ticket 08's decision: `check:layers` must fail on `git grep` exit ≥ 2 (section 1.2 shows this is not implemented). Proptests stay inline. The `integration-e2e-suite` is partially adopted: invariants, gates and the ledger method bind; the per-test classification does not. |
| `map.md:156-160` | "Policy for BDD and quality gates beyond layer placement. The tools exist: mutarust (`mise run test:mutants`) and `llvm-cov nextest`. **Ticket 12 sets the thresholds and the scope.**" No thresholds are pre-decided. Sections 1.3 and 1.4 confirm that none is active. |
| `issues/08:128-134` | Ticket 08's constraints for other tickets: "Ticket 12: set mutation thresholds. Unit tests are guarded only after that." |

### 1.9 Surprises and contradictions found

| # | Finding |
| --- | --- |
| 1 | **No quality threshold is active anywhere.** `min_msi` is commented out. CRAP has `fail-above = false`. No llvm-cov fail-under exists. `mess` is advisory-only. Mutation and coverage are absent from CI and from `verify`. |
| 2 | CRAP threshold contradiction: the doc says 20.0, the config says 15.0. |
| 3 | **`check:layers` does not meet ticket 08's own requirement.** Three greps are `\|\| true`, so a `git grep` exit of 2 or 128 is still swallowed. This is the exact failure that kept the gate dead for 224 commits. |
| 4 | `check:layers` cannot see untracked files. The ticket-09 prototype "passed" without scanning its own new files. |
| 5 | A dead pathspec fallback remains: `mise.toml:157-160` still probes `tests/common/`, which was deleted in `7b71ed06`. |
| 6 | `test:unit` lacks `--no-tests fail`. The canonical `test` and all four CI phases have it. An ad-hoc filtered run can be silently green. |
| 7 | A stale lib doc: `src/lib.rs:181` claims "exactly the 46 identifiers". The frozen baseline holds 47. |
| 8 | Coverage runs e2e with `--all-features`, that is, with `test-utils` on. That is not the shipped configuration that `mise run test` and CI use for the e2e phase. |
| 9 | `Proves`, `Defect class:` and `Executes vs asserts:` are undocumented and drifting. No taxonomy exists in `docs/` or `docs/agents/`. 15 tags include near-duplicates. Ticket 14 owns their fate. |
| 10 | **No guarantee→test mapping exists.** The only ledger is a *deletion* ledger (behavior→former layer→defect class→surviving test). The 88-guarantee invariance map has no test column. |
| 11 | `proptest-regressions/` does not exist, despite ticket 08(c) saying "Commit `proptest-regressions/` files". All 6 proptests are in `src/temporal/` only. |
| 12 | No `cargo-proptest` tool or task exists anywhere. Ticket 08's phrase maps to the `proptest` dev-dependency and inline `proptest!` blocks. |
| 13 | `check:layers` and `check:visibility` run twice per `verify`. Once directly, once inside `hk check`, which does not skip them. |
| 14 | `mise run check` invokes `hk check` with no `--all` and no file list. CI's hygiene job passes `--all`. The two do not select the same step set. |
| 15 | `docs/refs/mutation_testing.md:29` pins `"cargo:mutarust" = "0.1.10"` in prose. `mise.toml:36` installs `latest`. |
| 16 | The root `report.json` shows `totalMutantsCount: 0`, because every run clobbers it, including `--dry-run` (`mutarust.yml:8-11`). Reading it gives no signal about the last scored run. |
| 17 | Test-count bookkeeping drifts across snapshots: lib+bin 3,070 → 3,151; integration 30 → 58 → 61; e2e 27 → 61; doctests 76 → 83. Use these as a baseline for gap work. They are explicitly not a gate. |

---

## 2. Detecting gaps in behavioral and specification coverage (external)

This section records what primary BDD and SBE sources say about defining
"specification coverage" and finding GAPS in it. The emphasis is on
lightweight, session-runnable methods. Scenario-quality rubrics and mutation
tooling are out of scope here. Sections 3 and 4 cover them.

Research date: 2026-10-28. Sources are official docs, first-party posts by
the named authors, Manning sample chapters, and crate or registry first-party
material. Marketing and SEO blogspam were excluded.

Two labels apply in this section:

- **Source:** the source states the claim.
- **Inference:** the claim is my reading. It is flagged where used.

An attribution correction to the ticket brief: primary sources credit
**Feature Injection to Chris Matts**. Andy Pols, Sanela Hodzic and David
Anderson did later work. Kent McDonald gave the three-step framing.
**Liz Keogh** championed the approach. No source mentions a "Chris George"
(section 2.2).

### 2.1 What "covered" means in BDD and SBE

#### The identity that grounds coverage

| Source | Finding |
|---|---|
| [Adzic, *Specification by Example*, ch. 2 sample (Manning)](https://manning-content.s3.amazonaws.com/download/d/5a22dfb-908c-408d-aaa1-46873b6b18c9/adzic_ch02.pdf) | Source: "With Specification by Example, functional requirements, specifications, and acceptance tests are the same thing." The seven key process patterns are: derive scope from goals, specify collaboratively, illustrate using examples, refine, automate validation *without changing the specifications*, validate frequently, and evolve living documentation. |
| Same chapter | Source: living documentation is "a reliable and authoritative source of information on system functionality that anyone can access. It's as reliable as the code but much easier to read and understand." The "specification with examples" lives in a repository: a FitNesse wiki or "a directory structure of Cucumber feature files". |
| [Cucumber: Introduction](https://cucumber.io/docs) | Source: Gherkin serves three purposes: "Unambiguous executable specification", "Automated testing using Cucumber", "Document how the system _actually_ behaves". Feature files are versioned in source control next to the software. This is the "Single source of truth". |
| [Cucumber: Gherkin reference](https://cucumber.io/docs/gherkin/reference) | Source: "In addition to being a specification and documentation, an example is also a _test_. As a whole, your examples are an _executable specification_ of the system." |
| [North, "Introducing BDD" (2006)](https://dannorth.net/introducing-bdd) | Source: BDD's core template is "Given some initial context, When an event occurs, Then ensure some outcomes". Behavior is specified as observable outcomes, not implementation. |
| [North, "What's in a Story?" (2007)](https://dannorth.net/whats-in-a-story/) | Source: BDD is outside-in: business outcome → feature set → stories. Each story defines scope plus its acceptance criteria (the ATM withdrawal example). |

**Inference**, strongly grounded: the sources imply this definition. A
behavior is *specification-covered* if and only if an executable example for
it both documents intent and enforces it. The example passes or fails with
meaning. Merely *exercising* code does not add specification coverage. A unit
test that hits a path without stating the business rule does not count.
SBE equates requirements, specifications and acceptance tests. Coverage
counts live in the specification corpus, that is, the feature files. They do
not live in line coverage.

#### Rules versus examples: not every rule needs a green card

| Source | Finding |
|---|---|
| [Keogh, "Acceptance Criteria vs. Scenarios" (2011)](https://lizkeogh.com/2011/06/20/acceptance-criteria-vs-scenarios/) | Source: a scenario is a concrete example of behavior. Acceptance criteria are rules from which scenarios are derived. "It isn't always necessary to write scenarios for all the acceptance criteria. As long as the acceptance criteria are clear enough to derive the relevant scenarios easily, I recommend leaving it as acceptance criteria until the point where it needs to be automated." |
| [Wynne, "Introducing Example Mapping" (2015)](https://cucumber.io/blog/bdd/example-mapping-introduction) | Source: "some rules are so obvious you don't need examples at all... You can all move on with your lives without forcing yourselves to grind out examples like BDD-brainwashed automatons." |

**Inference** for gap detection: a rule list with un-automated rules is not
automatically a gap list. Trivially-derivable rules can stay as prose rules.
The gap signal is a rule that is *non-obvious, risky, or disputed* and still
lacks a concrete example. Every red "question" card from discovery is also a
gap.

#### What "living documentation" implies about gap detection

Source: living documentation is trustworthy only because automatic validation
with the code keeps it honest (SBE ch. 2; the Cucumber docs: examples
document how the system *actually* behaves).

**Inference**: the gap check becomes a read of the specification corpus
itself. If a behavior a stakeholder cares about has no feature or scenario
text, it is simultaneously missing documentation, missing specification, and
missing acceptance test. One gap, three consequences, one artifact to audit.
This is why living documentation removes the need for a separate
traceability document in the *forward* direction, that is, rule → example.
The feature tree *is* the documentation index. The *backward* direction, that
is, example → rule or goal, is where tags or naming conventions return
(section 3.1).

### 2.2 Lightweight, session-runnable gap-detection practices

#### Example Mapping (Matt Wynne)

Primary sources: [Wynne, "Introducing Example Mapping", Cucumber blog,
2015-12-08](https://cucumber.io/blog/bdd/example-mapping-introduction);
[Cucumber docs: Example Mapping](https://cucumber.io/docs/bdd/example-mapping);
[Tooke, "Your first Example Mapping session"
(2018)](https://cucumber.io/blog/bdd/your-first-example-mapping-session).

| Source | Finding |
|---|---|
| Wynne 2015 | Source: four cards: yellow story, blue rules (acceptance criteria), green examples under the rule they illustrate, red questions that nobody in the room can answer. Keep going until the scope is clear or time runs out. |
| Wynne 2015 | Source: the timebox is "about 25 minutes" for a well-understood, well-sized story. Not finishing means learning, an oversized story, or residual uncertainty: "Listen to that, and either try and slice the story, or let the product person go off and do some homework." |
| Wynne 2015 | Source: the readiness gate is a quick thumb-vote after 25 minutes. Minor remaining questions can be accepted by group decision. |
| Wynne 2015 | Source, the built-in gap readout: "A table covered in red (question) cards tells us that we still have a lot to learn". A blue-heavy table means the story is big or complicated, so slice it. "A rule with many examples might be over-complex. Are there multiple rules at play?" |
| Wynne 2015 | Source, the core gap move: "When the outcome (Then) is unclear, you don't have an example, you have a question." Capture the question instead of debating opinions: "Congratulations! You've just turned an *unknown unknown* into a *known unknown*. That's great progress." |
| Wynne 2015 | Source: do NOT write Gherkin during the session. Stay low-tech. Use "friends episode" naming ("The one where the customer forgot his receipt"). The goal is shared understanding, not test authoring. The other amigos write the Gherkin afterwards. |
| Wynne 2015 | Source, the cadence: "run them frequently: every other day is often a good rhythm. Just pick one story and give it 25 minutes of attention... Trying to do more in a big batch will just drain your energy." Minimum attendees: developer, tester, product person. Distributed variants: a bullet list in a shared doc, a spreadsheet with coloured cells, a mind-map. "keep it quick and easy." |
| Tooke 2018 | Source: thumb-vote at the end. If "no", schedule another session after the questions are answered. Have the person with least understanding write the Gherkin "minutes" and share back. The facilitator watches for scope drift and detail-sweating. Mini retro at the end. |
| [Wynne, bdd-discovery skill (first-party)](https://github.com/mattwynne/skills/blob/main/skills/bdd-discovery/SKILL.md) | Source, reading the map: many red cards mean too much uncertainty. Many blue cards mean the story is too broad. Many green cards under one rule mean the rule hides smaller rules. New-story cards are scope control, not failure. Treat red cards as progress. |

#### Feature Injection (Chris Matts; framing by Kent McDonald)

Primary sources: [Matts & Adzic, "Feature Injection: three steps to
success", InfoQ, 2011-12-14](https://www.infoq.com/articles/feature-injection-success);
[Smart, *BDD in Action* ch. 3 (Manning
livebook)](https://livebook.manning.com/book/bdd-in-action/chapter-3/ch03fig02).

| Source | Finding |
|---|---|
| InfoQ 2011 | Source: three steps (Kent McDonald's framing): 1. Hunt the value. 2. Inject the features. 3. Spot the examples. Chris Matts invented the practice in the 2000s. He extended it with examples to "validate or break models" (Matts & Hodzic, 2004). |
| InfoQ 2011 | Source, gap detection by counter-example: "we asked the business experts whether there were any examples that did not fit into the example we had constructed." Oblique examples (a bankrupt gas supplier feeding a power station that it could not legally cut off) exposed missing contract attributes, that is, new rules the model lacked. |
| InfoQ 2011 | Source: "Once we have identified the values we need to source from outside the system, we can stop. Feature Injection has a defined end point and as a result does not suffer from Analysis Paralysis." The method is bounded and session-sized by design. |
| InfoQ 2011 | Source: high-level examples "provide enough context to focus related development processes such as test-driven-development and behaviour-driven-development". If detailed enough, they can be automatically validated, "providing an objective measure of progress and a foundation for a living documentation system." |
| *BDD in Action* ch. 3 | Source: Feature Injection's aim is "the minimum set of features that will provide the most benefit to stakeholders in terms of achieving their business goals". Impact Mapping visualizes goal ↔ feature relationships, that is, "which features map to which goals". |

#### Specification workshops and cadence (Gojko Adzic)

Primary source: [Adzic, "Specification workshops - an agile way to get
better requirements"
(2008-11-12)](https://gojko.net/2008/11/12/specification-workshops-an-agile-way-to-get-better-requirements/).

| Source | Finding |
|---|---|
| Adzic 2008 | Source: "organise a workshop at the start of each iteration". The goal is "to discuss examples of stories that are planned for the iteration. The discussion leads to investigating edge cases, identifying functional gaps and inconsistencies before the development starts." |
| Adzic 2008 | Source: participants must include domain experts, who know how the system should work, and someone who can "say yes or no to edge cases, ideas and identified functional gaps", who controls scope. Without business presence, engineers alone define the rules and examples, and the result misses the point. |
| Adzic 2008 | Source: collaborative specs are "more complete, easier to implement and more consistent with the technical infrastructure" than single-author specs. Completeness is a *collaboration outcome*, not a document-review outcome. |

#### Adzic's example techniques for finding missing examples

| Source | Finding |
|---|---|
| [Adzic, "Focus on key examples" (2014-05-05)](https://gojko.net/2014/05/05/focus-on-key-examples) | Source: "Overly complex examples, or too many examples, are often a sign that some important business concepts are not explicitly described." Four starting strategies: look for missing concepts; group by commonality and focus only on variations; split validation and processing; "summarise and explore important boundaries". Boundary examples (49.99 vs 50 USD) "will probably give the team 90% of the value they could get from twenty pages of similar examples." |
| [Adzic, "Examples make it easy to spot inconsistencies" (2009-05-12)](https://gojko.net/2009/05/12/examples-make-it-easy-to-spot-inconsistencies) | Source: the blackjack workshop experiment. Teams got a requirements doc plus realistic examples. Each team presented and answered three questions: "was there any functionality missing from the requirements that you discovered? were there any conflicting rules that you discovered? was there something that you scrapped and decided not to do?" Result: "All the teams discovered almost all the inconsistencies and functional gaps, and one team even noticed something that I missed while preparing the workshop." The three questions are a documented, session-runnable gap checklist. |
| [Adzic, *Impact Mapping* book page (2012)](https://gojko.net/books/impact-mapping/) | Source: impact mapping is "collaborative strategic planning... better plans and roadmaps that ensure alignment of business and delivery, and are easily adaptable to change." **Inference**, gap reading: goals on an impact map with no supporting deliverables or features are *portfolio-level* gaps. Features with no path to a goal are candidates for cutting. The book site was offline during research (section 6). This reading also follows *BDD in Action* ch. 3: "visualize the relationships between stakeholders, capabilities, features, and business goals". |

#### Cadence summary: what is actually documented

| Cadence | Source | Scope |
|---|---|---|
| Every other day, one story, 25 min, thumb-vote. Do not batch. | Wynne 2015 (Source) | Story-level readiness and gap check |
| A workshop at the start of each iteration over the iteration's stories | Adzic 2008 (Source) | Iteration-level gap check: edge cases, gaps, inconsistencies |
| Questions-as-cards, resolved by homework or spike between sessions | Wynne 2015 (Source) | Deferred-gap loop |

No source in the corpus describes a heavier periodic "specification backlog
review" cadence beyond the above.

**Inference**: for a solo session, the Wynne cadence compresses to one
25-minute mapping. Run it even alone. Red cards hold questions to ask
someone later. Ask them before you write scenarios for a behavior area.

### 2.3 Requirements-to-test traceability: lightweight forms

#### The tag convention, the documented lightweight form

| Source | Finding |
|---|---|
| [Cucumber reference, tags section](https://cucumber.io/docs/cucumber/api) | Source: "Using tags for documentation... Tags can refer to IDs in external systems such as requirement management tools, issue trackers or test management tools: `@BJ-x98.77 @BJ-z12.33 Feature: Convert transaction`". Tags inherit: Feature tags → Rule → Scenario → Scenario Outline → Examples. Tag expressions (`and`/`or`/`not`, parens) run subsets. |
| [cucumber/tag-expressions README](https://github.com/cucumber/tag-expressions/blob/main/README.md) | Source: tag expressions serve exactly two purposes: run a subset of scenarios (`--tags`) and restrict hooks. The expressions are boolean, with `and`, `or`, `not`. |
| [Hindsight/Behave Pro: Organising scenarios with tags](http://help.hindsightsoftware.com/en/articles/2376758-organising-scenarios-with-tags) (vendor docs; documents the community convention) | Source: tags serve execution control (`@ui`, `@api`, `@unit`) and traceability metadata that is not used in execution: "we can add tags such as `@JIRAID-101` to make it clear which user story the team was working on". "If you are working in a regulated industry we can also use tags to trace a scenario back to a specific legal rule or requirement, such as `@1.2.0`." Behave Pro auto-tags story id and status. |

#### The BDD community's official stance on traceability

Primary sources: the Cucumber project blog (first-party), 2020 series.

| Source | Finding |
|---|---|
| [Cucumber blog: "Better requirements by harnessing the power of examples" (2020-09-17)](https://cucumber.io/blog/bdd/better-requirements-by-harnessing-the-power-of-exa) | Source: traceability is "the ability to describe and follow the life of a requirement in both a forwards and backwards direction". It must answer "Has this requirement been satisfied?". Example Mapping plus automated executable specifications extend traceability to test executions and product versions: "This is a significant advance when compared to the prevalent forms of requirements traceability in the industry today, such as spreadsheets or links in ALM tools. Even better, there is no overhead incurred. Every activity in the BDD approach delivers intrinsic value. Traceability comes as a secondary outcome." |
| [Cucumber blog: "How does BDD affect traceability?" (2020-09-24)](https://cucumber.io/blog/bdd/how-does-bdd-affect-traceability) | Source: "Traceability is dynamic". Any pre-Discovery sign-off must be revisited, because understanding arrives during Discovery. Useful traceability must be traversable and queryable: "every requirement has a sponsor", "every test has a successful execution". SCM versioning (feature files in the repo) provides snapshots. Cards-on-a-board links are "valuable, but difficult to search, traverse, or persist". Cross-lifecycle traceability requires tool integration. |
| [Cucumber blog: "User stories and BDD (part 4): features are not stories" (2020-01-30)](https://cucumber.io/blog/bdd/user-stories-and-bdd-features-are-not-stories) | Source, the documented critique of tracing through story artifacts: "stories are neither definitive nor independent... If you use stories for traceability, one of these days you're sure of a big surprise." Stories are "**transient artefacts** that facilitate delivery, not persistent artefacts that document behaviour". The feature files and ancillary docs capture requirements. |

#### When is a full RTM overhead?

Source, composite: the Cucumber sources never reject traceability as a goal.
Regulated needs are acknowledged. They reject *manually maintained
spreadsheet or ALM-link matrices*. They reject any links whose source
artifact is transient, that is, stories. Traceability should be a by-product
of activities that already carry intrinsic value: Discovery conversations and
executable specs in SCM. It must be queryable.

**Inference**: an RTM is overhead when three conditions hold:
1. The mapping lives outside the executable corpus and must be updated by
   hand.
2. It is anchored to transient artifacts.
3. Maintaining it is an activity with no other value.

A tag on a feature or scenario in the same repo is the lightest form
consistent with every source here. It is versioned, greppable, filterable at
runtime, and maintained by the act of writing the scenario.

**Inference**, limitation: tags give only the forward direction, that is,
scenario → requirement id. Detecting "requirement id with zero scenarios"
needs an external requirement inventory to diff against. That inventory can
be a rules list from Example Mapping, a glossary, or a tracked issue per
rule. The sources say traceability must answer "has this requirement been
satisfied?", which is backward from the requirement. Nothing in the corpus
automates that direction. It stays a review question.

### 2.4 Measuring specification coverage without vanity metrics

What sources state:

| Source | Finding |
|---|---|
| Cucumber blog 2020 (above) | Source: the operative questions are binary and per-requirement ("Has this requirement been satisfied?"). They are not percentages. |
| Adzic 2009 (blackjack) | Source: the gap check is three review questions (missing functionality? conflicting rules? scrapped items?). It is a facilitated review, not a metric. |
| Adzic 2014 | Source: a handful of boundary examples "give 90% of the value" of twenty pages. Example *count* is explicitly a bad proxy for coverage quality. |
| Wynne 2015 | Source: rules without examples are often fine. Red cards are the real gap inventory. Rule and example *counts* are diagnostic (blue-heavy = too big). They are never scored. |

What sources do NOT say, searched and not found: no primary source defines a
"specification coverage ratio", a target percentage, or any formula. The
closest thing to a coverage artifact in the literature is the *rule
inventory itself*. It covers Example Mapping blue cards, business rules in
feature descriptions, and domain glossary terms, reviewed against the
scenario corpus.

**Inference**: the lightweight method the sources support. This is a
candidate for the ticket-12 deliverable.

1. Maintain a **rule/behavior list** per module. Sources: Example Mapping
   blue cards, feature-description rule paragraphs, glossary terms, incident
   write-ups. This list is the coverage denominator. It is text, not a
   spreadsheet.
2. For each rule, point at its enforcing scenario or scenarios. The cheapest
   form is a **requirement/rule tag** on the scenario (section 2.3). A
   name/location convention also works.
3. The **gap list** is: rules with no enforcing scenario, plus open questions
   (red cards). Review it in a 25-minute session (Wynne cadence). Use Adzic's
   three questions as the review script: missing functionality? conflicting
   rules? scrapped?
4. Do not score a percentage. Do not count scenarios. Count *unmapped rules
   and unresolved questions*. Those are the only numbers these sources would
   recognize.

This method deliberately excludes code/line coverage and mutation scores as
the coverage story. Both are section 4's scope. The sources surveyed here
never conflate behavioral coverage with code coverage.

### 2.5 The Rust ecosystem

#### cucumber-rs: organize and filter scenarios by tag

| Source | Finding |
|---|---|
| [Cucumber Rust Book: Tags](https://cucumber-rs.github.io/cucumber/main/writing/tags.html) | Source: tags are meta-information. Typical purposes: run a subset of scenarios by tag, `@serial` for isolation, `@allow.skipped` to permit skips. A scenario may carry many tags. Filtering uses the `--tags` CLI with [tag expressions](https://cucumber.io/docs/cucumber/api), the same boolean language as upstream Cucumber. Methods such as `filter_run` also filter. Tag inheritance: Feature → Rule → Scenario → Examples. |
| [cucumber-rs docs.rs](https://docs.rs/cucumber/latest/cucumber/) | Source: the `tag` module extends `TagOperation` from the `gherkin` crate's tagexpr. The tag-expression evaluator is shared with the gherkin parser family. |

#### rstest-bdd: tag filtering, including compile-time empty-filter detection

| Source | Finding |
|---|---|
| [rstest-bdd roadmap §3.4 "Tag filtering" (all items checked)](https://github.com/leynos/rstest-bdd/blob/main/docs/roadmap.md) | Source: `#[scenario]` and `scenarios!` accept a tag-expression filter. Macro expansion evaluates the filter. An invalid expression produces `compile_error!` diagnostics with byte offsets. Tag scope and inheritance are defined: scenarios inherit Feature tags; outline cases inherit from the outline plus the originating Examples block. `and` and `or` are left-associative; `not` is unary. Identifiers match `[A-Za-z_][A-Za-z0-9_]*` and are case-sensitive. `@allow_skipped` is supported with the `fail_on_skipped` config. |
| [rstest-bdd user's guide (docs in leynos/statelet)](https://github.com/leynos/statelet/blob/main/docs/rstest-bdd-users-guide.md) | Source: `tags: &str` is an "Optional tag-expression filter applied at expansion... filters scenarios and outline example rows; **errors when nothing matches**." The runtime skip listing (`steps --skipped`, `skipped` in cargo-bdd) emits JSON with `feature`, `scenario`, `line`, `tags` and `reason` fields. |
| [rstest-bdd on crates.io](https://crates.io/crates/rstest-bdd) | Source: "Zero new runners: keep CI/CD and IDE behaviour unchanged". Feature files are processed at compile time. Scenarios are static. The comparison table positions it as BDD-as-rstest-extension, against cucumber-rs's custom runner. |

**Inference**, a tooling idea grounded in the above: rstest-bdd errors at
compile time when a `tags` filter matches nothing. A session-runnable gap
check could therefore emit one `scenarios!(..., tags = "@rule-xyz")` probe
per rule in the inventory. An unproven rule then becomes a *compile
failure*. The Rust compiler does the gap detection. This is my inference
from documented behavior. It is not a documented practice.

### 2.6 Guarantee→test mapping tooling in Rust: no tool found

Searched crates.io and the web:

| Source | Finding |
|---|---|
| [rubric-attr](https://crates.io/crates/rubric-attr) | Experimental: "Attribute macros for requirements traceability" with `#[satisfies(...)]` and `#[verifies(...)]`. v0.1.1 is **yanked**. Its readme notes that the matrix "doesn't yet surface the inverse: requirements with no satisfying code or no verifying tests". Even the experiment lacked the gap view. |
| [trace-doc](https://crates.io/crates/trace-doc) | Experimental: v0.1.0, "compile-time, zero-cost traceability" via traits (`Realizes`, `Tests`). Roadmap items (graph extraction, rustdoc integration) are still future. |
| trace-doc README → [mantra](https://crates.io/crates/mantra) | "an alternative approach to requirement tracing that uses string-based requirement IDs with automatic source code scanning and database-backed reporting". This is a source-scanning style, not a BDD-scenario style. |
| [agent-spec](https://crates.io/crates/agent-spec) | Experimental "intent compiler" with a requirement-traceability projection. Agent-workflow niche, not BDD scenario mapping. |
| [DavidValin/aloo docs/TESTING.md](https://github.com/DavidValin/aloo/blob/main/docs/TESTING.md) | One third-party repo built a bespoke gate: `cargo trace` cross-checks `requirements/requirements.toml`, `@AC-024`-style tags on Gherkin scenarios, and `/// @requirement AC-024` doc markers on Rust tests. It "fails if a requirement has no test, a test names an id that does not exist, or an id is defined twice". It generates matrix reports under `target/traceability/`. Tag-filtered runs are "run everything that proves this requirement" (the `cargo bdd` runner with `-t "@AC-030"`). This is a project-local harness, not a published tool. |
| cucumber-rs / rstest-bdd first-party docs (above) | Neither framework documents any guarantee→test or requirement→scenario *completeness* checking. Only filtering and skipping are documented. |

**Conclusion**, a confirmed absence: no established Rust tool exists for
guarantee→test mapping integrated with cucumber-rs or rstest-bdd. A handful
of 0.1.x experimental crates target safety-critical traceability with
attribute or type-system approaches. One is yanked. The only complete
gap-detecting harness found is bespoke inside one application repo. Any
method adopted here will be convention plus a small script or gate. It will
not be a crate.

### 2.7 Things not found and access limits

These facts belong in the ticket record:

1. **No formal definition of "specification coverage"** as a metric exists
   anywhere in the primary corpus (North, Keogh, Wynne, Hellesøy/Cucumber,
   Rose, Adzic). The term "behavioral coverage" as a measurable quantity does
   not appear. The operational definition in section 2.1 is an inference from
   the requirements = specs = tests identity.
2. **No documented minimal cadence** exists for "reviewing a specification
   backlog" beyond Wynne's every-other-day Example Mapping and Adzic's
   per-iteration specification workshop.
3. **No standalone academic or heavyweight-RTM critique paper** was found in
   the BDD literature. The documented critiques are the Cucumber blog posts
   (section 2.3). Transient stories are bad trace anchors. Manually
   maintained matrices are overhead. Traceability should be a queryable
   by-product.
4. **specificationbyexample.com is offline** (HTTP 301 →
   gojko.net/books/specification-by-example, per [Wayback
   2023-12-31](https://web.archive.org/web/20231231094811/https://www.specificationbyexample.com/?src=/)).
   SBE chapter 3 ("Living documentation") full text is paywalled. Findings
   for SBE rest on the free Manning sample chapters 1 and 2 and the book's
   own summaries.
5. **impactmapping.org was unreachable** during research (transport error).
   Impact Mapping claims rest on gojko.net's book page and *BDD in Action*
   ch. 3 excerpts.
6. **"Chris George" does not exist in the Feature Injection sources.** The
   inventor is Chris Matts (section 2.2). The ticket brief needs the
   correction.
7. **No first-party rstest-bdd documentation page dedicated to tags** was
   reachable. The docs live in repo markdown: the roadmap, plus a user's
   guide hosted under the maintainer's `statelet` repo. The roadmap is
   first-party and complete.
8. **No ecosystem-standard Rust requirement-traceability tool** exists. All
   candidates are 0.1.x or yanked (section 2.5).

---

## 3. Scenario-quality criteria and anti-metrics (external)

Two labels apply in this section:

- **Source:** the claim is directly traceable to a primary source cited
  below.
- **Inference:** the claim is my synthesis from the cited material.

### 3.1 Scenario-quality criteria

#### Behaviour, not implementation: the what-not-how rule

| Source | Finding |
| --- | --- |
| Cucumber, *BDD: Better Gherkin* (cucumber.io/docs/bdd/better-gherkin) | Source: scenarios describe the behaviour of the system, not implementation steps. The contrast: `When 'Bob' logs in` states a functional requirement. Imperative steps ("I fill in the username field…") describe UI mechanics. |
| Same page | Source, the page's central test: a scenario should be worded so that changing the implementation does not force a rewording. If an implementation change breaks the scenario's *wording*, not its assertion, the scenario was written at the wrong level. |
| Same page, declarative style | Source: "Declarative style describes the behaviour of the application, rather than the implementation details." Declarative scenarios read better as living documentation. |
| Same page | Source: what-not-how maps directly to this repo's rule. Specs define what the system guarantees, never how. A scenario that names an internal component, module, or wrapped error string is an implementation coupling. |

**Rubric candidate** (**Inference**): for each scenario, ask: *would this
wording need to change if the implementation changed?* If yes, rewrite the
scenario at behaviour level.

#### Scenario structure: brevity, focus, illustrative scope

| Source | Finding |
| --- | --- |
| Cucumber, *Gherkin Reference* (cucumber.io/docs/gherkin/reference) | Source: "We recommend 3-5 steps per example." Scenario is a synonym of Example. `Rule` groups the scenarios of one business rule. |
| Seb Rose, *Keep your scenarios brief* (cucumber.io/blog/bdd/keep-your-scenarios-brief, 2019) | Source: the BRIEF acronym for scenario design (Brief, Repeatable, Independent, Fast, Examples) targets documentation-readability first. Scenarios are documentation that happens to be executable, not tests with prose attached. |
| Same article | Source: it distinguishes *illustrative* scenarios (a few examples that illuminate a rule) from full user-journey scenarios. Illustrative examples are the default for documentation quality. |
| Cucumber, *Introduction to Formulation* (cucumber.io/blog/bdd/introduction-to-formulation) | Source: it references BRIEF and the illustrative-scenario distinction. Formulation (choosing the right examples) precedes automation. |
| Gojko Adzic, *Specification by Example* via [InfoQ interview](https://infoq.com/articles/specification-by-example-book) | Source, quoting the book: "whenever you see too many examples or very complicated examples in a specification, try to raise the level of abstraction for those descriptions." |
| Adzic via InfoQ | Source: feature examples should be **precise** (concrete data, no yes/no answers), **realistic** (real data from customers), **complete** (experiment with data combinations), and **easy to understand** (do not explore every combination). |
| Adzic via InfoQ | Source: good specifications "should be precise and testable, not written as a script, not written as a flow." |
| Adzic via InfoQ | Source: over-prescribing hinders discussion. The right level of detail is a card that prompts "I'm not quite sure". The scenario's job is to trigger the conversation (Three Amigos). |

**Rubric candidates** (**Inference**): 3-5 steps; one business rule per
scenario; concrete example data over yes/no; raise abstraction rather than
enumerate combinations.

#### Refactor survival: alignment with the repo rule

The repo rule: scenarios must survive refactors.

| Source | Finding |
| --- | --- |
| Adzic via InfoQ, living documentation | Source: living documentation is a source of information about functionality "as reliable as the programming language code but much easier to access and understand". Validation automation keeps the documents honest. A scenario that dies on refactor fails the reliability half of this contract. |
| Adzic via InfoQ | Source: specifications should not be about software design: "not tightly coupled with code, work around technical difficulties, trapped in user interface details." Also: "watch out for descriptions of how the system should work, think about what the system should do." |
| Cucumber, *Better Gherkin* | Source: the implementation-change test (the refactor-survival rubric above) is the operational form of refactor survival. |
| Adzic via InfoQ | Source: ubiquitous language and symmetric change. A small change in business functionality should be a small change in software *and* documentation. Drift is legacy. Scenario wording therefore tracks domain language, not code structure. |

**Rubric candidate** (**Inference**): a scenario asserts on externally
observable outcomes and diagnostic codes. The repo rule is: assert codes,
never wrapped human text. Refactors of messages and formatting then cannot
invalidate the scenario.

#### Failure attribution: a failing scenario must localize the cause

| Source | Finding |
| --- | --- |
| Seb Rose, *How can Cucumber help us understand the root cause of a problem* (cucumber.io/blog/bdd/how-can-cucumber-help-us-understand-the-root-cause, 2021) | Source: the article's premise. Well-formed scenarios (Given/When/Then separating setup, action, outcome) make it possible to localize *where* in the behaviour a failure occurred, not only that "the test failed". |
| Gojko Adzic via InfoQ | Source: one check on requirements completeness is whether black-box tests can be designed against them. If you lack information for good test cases, you lack information to build the system. Corollary (**Inference**): a scenario whose failure cannot be attributed to a specific rule is an underspecified scenario. |

**Rubric candidate** (**Inference**): on failure, a reader can name the
broken business rule without reading source code. This ties to this repo's
requirement: assertions target diagnostic codes, which are stable and
localizable, not prose.

#### Review process: Three Amigos, Example Mapping, quality checklists

| Source | Finding |
| --- | --- |
| Cucumber, *Example Mapping* (cucumber.io/docs/bdd/example-mapping) | Source: yellow = story/question, blue = rule, green = example, red = question that blocks. The session is timeboxed. Quality is produced in the conversation, not in later editing. |
| Oliveira & Marczak, *How to Evaluate BDD Scenarios Quality?* (SBES '19, dl.acm.org/doi/10.1145/3350768.3351301) | Source: an interview study of 18 BDD practitioners. It extracted scenario-quality attributes and a question-based review checklist. This is the only peer-reviewed quality-checklist source found. It is paywalled. Only the abstract level is available. The checklist contents were not retrieved. |
| Adzic via InfoQ | Source: collaboration practices. Three Amigos sessions, developers reviewing tests, prepare only basic examples ahead of refinement. |

**Rubric candidate** (**Inference**): scenario quality is a review-time
property with a checklist. It is not an authoring-time property. Adopt a
short question-based checklist (from the rubric candidates in section 3.1)
as the review gate.

### 3.2 Anti-metrics: code coverage as a quality proxy

| Source | Finding |
| --- | --- |
| Cem Kaner, *The Ongoing Revolution in Software Testing* (kaner.com/pdfs/TheOngoingRevolution.pdf, 2004) | Source: "Monitoring our coverage of lines of code is simplistic." Plenty of other things exist to count: dataflows, anticipated failure modes. Focusing on executing statements misses problems, for example executing a line that contains A and B. |
| Kaner, *Ongoing Revolution* (2007 edition, testingRevolution2007.pdf) | Source: "We cannot measure the thoroughness of testing by computing simple coverage metrics or by creating at least one test per requirement or specification assertion. Thoroughness of testing means thoroughness of mitigation of risk." |
| Kaner, *Ongoing Revolution* (2004) | Source: code-coverage measurement makes sense in the unit-testing context. For black-box and system testers it is a practical mismatch. Also: managers pressing staff to hit coverage requirements produce predictable gaming (citing Marick 1997). |
| Brian Marick, *How to Misuse Code Coverage* (1997/1999/2000; mirror: examper.com/testing-com/writings/coverage.pdf; the original testing.com URL is dead) | Source: "Code coverage tools measure how thoroughly tests exercise programs. I believe they are misused more often than they're used well." |
| Same article | Source: it works through the canonical misuse: "Suppose a manager requires some level of coverage, perhaps 85%, as a 'shipping gate'". A threshold used as a goal produces tests chosen to satisfy the number, not the risk. |
| Same article, as summarized by Kaner's site and BBST | Source: Marick "describes the measurement dysfunction that he saw in organizations that pushed their testing to achieve very high statement/branch coverage." |
| Google Testing Blog, *Code Coverage Best Practices* (testing.googleblog.com, 2020-08-07) | Source: coverage is a "lossy and indirect metric" and "should not be your only source of truth". The best practice is to use it to find *untested* code, that is, a gap-finder. Do not use it as a score. |
| BBST Foundations slides (bbst.courses, 2022; Kaner/Pettichord lineage) | Source: "Driving testing to achieve 'high' coverage is likely to yield a mass of low-power tests." |
| BBST Foundations slides | Source: "People optimize what we measure them against, at the expense of what we don't measure." |

**Critique summary** (**Inference**): coverage measures *executed*
structure, not *risk mitigation*. As a target it does three harmful things:

- (a) It optimizes for cheap-to-execute assertions.
- (b) It crowds out non-structural coverage (failure modes, dataflows,
  requirements).
- (c) It is meaningless at system and black-box level.

Keep it only as a diagnostic for gaps (Google's framing). Never use it as a
gate or a score.

### 3.3 Anti-metrics: Goodhart and Campbell

| Source | Finding |
| --- | --- |
| DORA, *DORA Metrics guide* (dora.dev/guides/dora-metrics) | Source, the listed pitfall: "Setting metrics as a goal. Ignoring Goodhart's law…". When a measure becomes a target, it ceases to be a good measure. |
| Goodhart's law origin | Source (via a Jellyfish summary and Manheim 2018): the adage is attributed to Charles Goodhart. Campbell's law is the independent parallel formulation. Manheim, *Building Less Flawed Metrics: Dodging Goodhart and Campbell's Laws* (MPRA 90649, 2018): naive application of metrics "can distort the system in ways that undermine the original goal". In some forms the distortion is unavoidable. |
| Kaner & Bond, *Software Engineering Metrics: What Do They Measure and How Do We Know?* (METRICS 2004, kaner.com/pdfs/metrics2004.pdf) | Source: the construct-validity framing. Before you use a metric, ask what attribute it actually measures. Their evaluation checklist includes the side-effect question: what are "the natural and foreseeable side effects of using this instrument?". A metric whose foreseeable side effect is gaming should be rejected before adoption. |
| Kaner & Bond | Source, the paper's thesis: metrics in software engineering are widely used without established validity. The paper is an anti-metrics-adoption checklist with 10 questions on purpose, scope, attribute, scale, error and side-effects. |

**Critique summary** (**Inference**): pre-reject any BDD metric where the
cheapest way to improve the number diverges from improving the guarantee.
Apply Kaner & Bond's side-effect question as the admission test for every
candidate metric.

### 3.4 Anti-metrics: Dan North on test metrics

| Source | Finding |
| --- | --- |
| Dan North, *We need to talk about testing* (dannorth.net/blog/we-need-to-talk-about-testing, 2021) | Source: the section "Is test coverage a useful metric? No. Yes. Sometimes. It depends." Coverage is not flatly useless. It works as a *conversation starter* and a gap signal. It never works as a universal quality score. This section was fetched. Treat the heading and framing as the citable claim. |

**Critique summary** (**Inference**): the anti-metric is not coverage alone.
It is *any proxy adopted as a score*: scenario counts, Gherkin line counts,
pass-rate as quality, automation percentage.

### 3.5 Untraceable quotes and folk rules: findings in themselves

| Claim | Result of search |
| --- | --- |
| "6-word scenario title" rule | **UNTRACEABLE.** It is not in the cucumber.io docs (Gherkin reference, Better Gherkin, Example Mapping, Formulation, FAQ), Rose's blog, or Adzic's material. It appears to be a folk or team rule with no primary source. Do not cite it as Cucumber guidance. |
| "Code coverage is a floor, not a ceiling" | **UNTRACEABLE as a quote.** The search returned no authoritative origin. The *idea* is sourced in weaker form (Google: gap-finder not score; North: "sometimes"). The aphorism itself has no traceable author. Flag this before reuse. |
| Marick original PDF | **DEAD LINK.** testing.com/writings/coverage.pdf is gone (the domain was repurposed). Working mirrors: examper.com/testing-com/writings/coverage.pdf, testingeducation.org/BBST/foundations/Marick_coverage.pdf, archive.org item manualzilla-id-5972971. Cite the mirror. |
| SBES '19 checklist contents | **UNRETRIEVED.** The paper is paywalled. Only the abstract and secondary descriptions are available. The 18-practitioner checklist itself must be obtained before its questions can be quoted. |
| Adjacent: test-induced design damage (DHH / Fowler / Beck; Anand Bagmar, ThoughtWorks) | **Sourced but OUT OF SCOPE** unless tied to metric-gaming. It is a critique of *how testing*, not which metric, warps design. One-line mention only. Do not build the anti-metrics section on it. |

### 3.6 Synthesis for ticket 12

**Scenario-quality rubric candidates** (all Source-derived, **Inference**
assembled):

1. Behaviour-level wording that survives implementation change (Cucumber).
2. 3-5 steps; one rule per scenario; illustrative not exhaustive (Cucumber,
   Rose, Adzic).
3. Concrete, realistic example data; no yes/no; raise abstraction instead of
   enumerating (Adzic).
4. Asserts observable outcomes and diagnostic codes, never implementation or
   prose (Adzic plus repo rules).
5. Failure localizes to a business rule without reading source (Rose,
   Adzic).
6. Reviewed in Three Amigos or Example Mapping with a question-based
   checklist (Cucumber, SBES '19).

**Anti-metrics to pre-reject:**

1. Code coverage percentage as a quality gate (Kaner, Marick, Google, BBST,
   North).
2. Scenario counts, test counts, pass-rate, automation percentage as KPIs
   (Goodhart through DORA, Manheim).
3. Any metric that fails Kaner & Bond's side-effect question (metrics2004).
4. Do not adopt the untraceable "6-word title" or "floor not ceiling"
   aphorisms as sourced rules.

---

## 4. Mutation testing and coverage tooling versus BDD scenarios (Rust)

Research date: 2026-10-28. Primary sources only: official docs and repos,
plus this repo's committed research notes (cited as local docs). Claims
marked **Inference** are reasoned from sourced facts but are not stated
directly by a primary source.

Standing repo facts, from local docs, not re-researched: mutation runs
through `mise run test:mutants` (mutarust, a tool-agnostic task entrypoint).
Coverage comes from cargo-llvm-cov plus nextest. Mutation is deliberately outside
`verify`/hk/CI. Scenarios are rstest-bdd tests folded into `tests/e2e.rs`.
They spawn the CLI binary. Sources:
[`docs/refs/mutation_testing.md`](../../../docs/refs/mutation_testing.md),
[`docs/refs/quality_gates_mise_adoption.md`](../../../docs/refs/quality_gates_mise_adoption.md).

### 4.1 cargo-mutants: it runs tests, it is coverage-blind, it is nextest-native

| Source | Finding |
|---|---|
| [how-it-works.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/how-it-works.md) | For each mutant: apply the patch, then `cargo test --no-run` (a viability check), then **run `cargo test`**. A failure means caught. It genuinely *runs* the suite per mutant. Compile-only is only the viability pre-step. |
| [using-results.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/using-results.md) | Outcomes are exactly **caught / missed / unviable / timeout**. There is **no "covered" state and no coverage integration**. A missed mutant conflates "never executed" with "executed but not asserted". Confirmed by local research: "Coverage awareness: none" (`quality_gates_mutarust.md` §3). |
| [how-it-works.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/how-it-works.md) | It skips functions "because they're tests, because they have `#[mutants::skip]`". Test functions are never mutated. The tree is copied to a scratch dir (reused across mutants for incremental builds). A baseline clean test run happens first. The mutation is reverted after each run. |
| [in-diff.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/in-diff.md) | "The diff is only matched against the code under test, **not the test code**". Test-code edits never trigger mutants. |
| [nextest.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/nextest.md) | First-class `test_tool = "nextest"`. The rationale: nextest runs each test in a separate process and fails fast, so a caught mutant stops sooner. Two caveats: (a) nextest allows straggling tests to completion, so it can be *slower* on fast-unit, slow-integration trees; (b) **nextest does not run doctests, so doctest-only behaviors show as missed mutants**. Fail-fast should stay on for mutation. |
| [shards.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/shards.md) | Cost controls: `--shard k/n` (no runtime coordination; same args and same `n` on every shard), `slice` and `round-robin` algorithms, a rule of thumb of at least 10 mutants per shard, 8-32 shards typical. `--shard 1/100` doubles as about a 1% sample. `--baseline=skip` avoids per-shard baseline cost. |
| [timeouts.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/timeouts.md) | The default test timeout is 5x the baseline test time, minimum 20 s. Flags: `--timeout`, `--timeout-multiplier`, `--minimum-test-timeout`. Build timeouts are off by default (const-eval hangs). |
| [baseline.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/baseline.md) | "Baseline" in cargo-mutants means the **clean-suite run** in the scratch copy. It also calibrates timeouts. It is *not* an accepted-escapes file. Use `--baseline=skip` only if the tests are known green. |
| [skip_files.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/skip_files.md), [filter_mutants.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/filter_mutants.md) | Native path and name scoping: `-f`/`-e` globs, `examine_globs`/`exclude_globs`, `--re`/`--exclude-re` on mutant names, `--package`. Excluded files are still parsed for `mod` traversal. |
| [ci.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/ci.md), [performance.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/performance.md) | CI guidance: `--in-place`, diff-scoped runs preferred. Performance levers: `[profile.mutants]` (debug=none), ramdisk TMPDIR, mold or wild linkers. **No `--no-shuffle` flag was found in the book.** A mutant *shuffling* concept exists (shards.md mentions combining with shuffling) but the exact flag surface was not verified. |

The answer to "how does it measure covered?": it does not. cargo-mutants has
no coverage measurement. "Killed versus escaped" is the only strength
signal. mutarust is the tool in this repo that added coverage-awareness
(`--coverage` produces a `not covered` state, and `--min-covered-msi`
exists). Source: `quality_gates_mutarust.md` §2.5 and §9.7.

### 4.2 mutarust, this repo's engine: what it adds over cargo-mutants

Vendored upstream docs at [`docs/refs/mutarust/`](../../../docs/refs/mutarust/)
are authoritative.

| Source | Finding |
|---|---|
| `mutarust/docs/cli.md` | It always **runs the full test suite** per mutant (plain `cargo test`; the clean suite first; it aborts on failure). The default runner is cargo test. **Nextest appears nowhere in the repo.** It appears only via `--exec`, which is serial (one worker) and drops Cargo type-proof mutators to Skipped (`quality_gates_mutarust.md` §2.4, §9). |
| `mutarust/docs/cli.md`, `config.md` | It adds what cargo-mutants lacks: **native `--min-msi` and `--min-covered-msi` gates (exit 4)**, coverage-aware scoring (`--coverage` via cargo-llvm-cov; uncovered mutants are not run), an **accepted-escape baseline** (`mutarust-baseline.json` plus `--fail-on-escaped`, so only new escapes fail), `--blacklist` (checksum skip), `--run-mutant-id` (single-mutant rerun), `--git-diff-lines` changed-line scoping, and agentic or summary JSON reports. |
| `mutarust/docs/cli.md` | The score is (killed + errored + skipped) / total. **Errored and skipped count toward the score.** MSI numbers are therefore not comparable to cargo-mutants' outcome taxonomy or to textbook MSI (`quality_gates_mutarust.md` §6 risk 2). |
| `mutarust/docs/cli.md` | It never mutates `#[cfg(test)]` items. Recursive discovery excludes tests, benches, examples, fixtures, vendor, generated code, `build.rs` and `*_test.rs`. An **explicit file target can still select test source**. `--timeout-coefficient` multiplies the **longest single clean test** (minimum 1 s). It is mutually exclusive with fixed `--timeout`. |

The constraint for decision 12(e): the mutation engine this repo uses runs
`cargo test`, not nextest, per mutant. The "nextest is decisive" constraint
applies to the *test phase* (`mise run test`) and to coverage
(`cargo llvm-cov nextest`). It does **not** apply to the mutation runner.
Mutants therefore also execute doctests (cargo test behavior). A nextest-based
mutation run would not.

### 4.3 cargo-llvm-cov plus nextest: what is measured

| Source | Finding |
|---|---|
| [cargo-llvm-cov README](https://github.com/taiki-e/cargo-llvm-cov) | It wraps rustc `-C instrument-coverage`: **line, region and branch coverage** (branch is optional and nightly: `--branch`, issue #8). Doctests are optional and nightly (`--doctests`, issue #2). A first-class `cargo llvm-cov nextest` subcommand exists. |
| [nextest test-coverage.md](https://nexte.st/book/test-coverage.html) | Nextest delegates to cargo-llvm-cov. Doctest coverage requires a merge: `--no-report nextest`, then `--no-report --doc`, then `report --doctests`. The integration is env-var based (RUSTFLAGS, RUSTC_WRAPPER). Nextest runs Cargo for the build, so instrumentation flows through normally. |
| [cargo-llvm-cov README, "Get coverage of external tests"](https://github.com/taiki-e/cargo-llvm-cov) | The documented recipe for binaries run outside `cargo test`: `source <(cargo llvm-cov show-env --sh)`, then `clean --workspace`, then `cargo build` (instrumented), then run your processes, then `cargo llvm-cov report`. This is upstream's explicit acknowledgment: arbitrarily-spawned binaries can contribute coverage if they build with the env set. |
| [LLVM SourceBasedCodeCoverage](https://releases.llvm.org/22.1.0/tools/clang/docs/SourceBasedCodeCoverage.html), [rustc instrument-coverage book](https://doc.rust-lang.org/beta/rustc/instrument-coverage.html) | An instrumented binary writes a `.profraw` to `$LLVM_PROFILE_FILE` **on exit**. The `%p`, `%m` and `%Nm` patterns make per-process unique files. The LLVM docs explicitly discuss "a program such as the Lit testing tool, which invokes other programs": "it may be necessary to set `LLVM_PROFILE_FILE` for each invocation". Spawned child processes inherit the env var and write their own profraw. `llvm-profdata merge` then folds them in. |
| [cargo-llvm-cov README](https://github.com/taiki-e/cargo-llvm-cov) | The `LLVM_PROFILE_FILE_NAME` env var overrides the final path component of `LLVM_PROFILE_FILE`. This is the collision-avoidance knob. `--no-cfg-coverage` and `--no-cfg-coverage-nightly` unset the `cfg(coverage)` and `cfg(coverage_nightly)` cfgs that cargo-llvm-cov always sets during its builds. Use them only if code gates on them or lints fire. The recommended mitigation is a `check-cfg` lints table in Cargo.toml. In multi-binary workspaces, profiles from multiple test binaries merge automatically. `--failure-mode any|all` controls merge failures. The default `--ignore-filename-regex` excludes `tests/`, `examples/` and `benches/` **from the report**. |

### 4.4 The spawned-binary question: does llvm-cov see scenario-executed src?

The mechanism says yes, conditionally. This claim is an **Inference**, well
grounded:

1. Under `cargo llvm-cov nextest`, the CLI bin target is built by the same
   instrumented Cargo invocation.
2. `tests/e2e.rs` processes inherit `LLVM_PROFILE_FILE` and pass it to the
   spawned CLI process.
3. The spawned process writes its own profraw on exit. `llvm-profdata merge`
   combines the files.
4. src/ lines executed *by the spawned binary* are inside the report scope.
   Only `tests/` directories are excluded from the report, not src.

The failure modes:

- If the spawned binary does not build under the llvm-cov env (for example a
  stale pre-built `target/debug` binary, or a build step that clears
  RUSTFLAGS), it contributes zero coverage. The report then silently shows
  src lines uncovered.
- Concurrent profraw writes are the documented corruption risk. The exact
  cargo-llvm-cov pattern choice was **not verified from source**. One
  empirical run settles it: spawn N scenarios, then count profraw files.

The honest statement for decision 12(e): upstream docs support
spawned-process coverage in principle (the external-tests recipe plus the
LLVM Lit note). **No primary source documents the exact
`cargo llvm-cov nextest` + CARGO_BIN_EXE + rstest-bdd path this repo
uses**. Treat "e2e scenarios contribute line coverage of src with the
spawned binary" as check-by-experiment, not established fact.

### 4.5 Threshold practice: the honest answer is "no fixed thresholds upstream"

| Source | Finding |
|---|---|
| The cargo-mutants book, grepped over `book/src` (per `quality_gates_mutarust.md` §3) | **No mutation-score or MSI concept exists at all.** CI fails via exit code 2 (some mutants missed) or per-missed-mutant triage. Accepted survivors are handled by `#[mutants::skip]` or `exclude_re`/`exclude_globs`. This is policy-by-exemption, not by number. "What to do about missed mutants" is explicitly a human-judgment triage loop. |
| `mutarust/docs/ci.md` (vendored) | The only concrete Rust-tooling threshold recommendation found: tiers **baseline-only (legacy), 60-75 (active), 75-80 (stable library), 90-95 (high assurance)** for `--min-msi` and `--min-covered-msi`. This is single-vendor guidance. The score definition counts errored and skipped (section 4.2). |
| [Papadakis et al., "Practical Mutation Testing at Scale" (Google, TR 2021)](https://homes.cs.washington.edu/~rjust/publ/practical_mutation_testing_tr_2021.pdf) | Google's production mutation service **deliberately avoids absolute mutation scores** ("not the same as the traditional mutation score…"; an absolute score is a function of repo size). The practice: probabilistically select high-value surviving mutants per changelist and surface them in code review as "please fix". This is **per-change survivor triage, not a global gate**. Survivability is about 9-13% by operator. |
| [Just et al., "Are Mutants a Valid Substitute for Real Faults?" (FSE 2014)](https://homes.cs.washington.edu/~mernst/pubs/mutation-effectiveness-fse2014.pdf) | 73% of real faults are coupled to mutants, so mutants are a valid proxy. Also: "if a test suite covers only 20% of the source code, developers are likely to focus on improving code coverage before… the mutation score". Score-first is the wrong order. |
| [Papadakis, Papadoukas, Just, "Prioritizing Mutants…" (ICSE 2022)](https://homes.cs.washington.edu/~rjust/publ/prioritizing_mutants_tcap_icse_2022.pdf) | Directly against fixed score thresholds: "the traditional mutation adequacy score… rises rapidly with the first tests due to **redundant mutants**, and hence is a poor measure of test completeness." "When to stop" is a prioritization and ranking problem (TCAP), not a number. |
| [CircleCI, "What is mutation testing?" (2026, vendor blog, secondary)](https://circleci.com/blog/what-is-mutation-testing) | Industry-practice echo: "**Diagnostic first, gate later.** Run mutation testing without a threshold for the first few weeks". The Stryker `break: 70` value is shown as a *chosen* floor, not a universal one. This is secondary (vendor content). It is included because it matches the primary-source posture. |

The synthesis for decision 12(e): no primary source recommends a universal
survivor-rate threshold. Two honest practices exist:

1. cargo-mutants-style per-mutant triage plus explicit exemptions.
2. mutarust's staged tiers as an *adoption* ladder. Its baseline-first step
   is "accept reality, gate only on new escapes". This is exactly what this
   repo's [`mutation_testing.md` §4 baseline
   workflow](../../../docs/refs/mutation_testing.md) already does.

Google's scale practice independently validates baseline and
new-escapes-only over absolute scores.

### 4.6 Does mutation testing add signal BEYOND BDD scenarios on the same code?

| Source | Finding |
|---|---|
| [vs-coverage.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/vs-coverage.md) | The canonical argument: coverage says a line was *executed*. Mutation says a test *checks* it. The worked example: a function returns `Result` after writing a file, the return value is asserted, the file write is not. The mutant `-> Ok(())` survives. This is precisely the class of gap that "exit code 0 and stdout contains X" scenarios are prone to. |
| [using-results.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/using-results.md) | Explicit composition guidance: "avoid writing tests that are too tightly targeted to the mutant… write tests that assert the **correct behavior at the right level of abstraction, preferably through a public interface**." BDD scenarios that assert CLI stdout, stderr and exit codes *are* the public-interface tests this guidance prescribes. The intended response to an escape under BDD is therefore a stronger scenario, not a unit test. Mutation testing and BDD are a loop, not competitors. |
| [limitations.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/limitations.md) | "If you rely on testing the program's behavior by manual testing, or by an **integration test not run by `cargo test`**, then cargo-mutants can't know this." The key fact for this repo: scenarios folded into `tests/e2e.rs` ARE run by `cargo test` and nextest. The mutation runner sees them fully. Had BDD lived in an external runner, mutation would under-credit scenario coverage entirely. |
| `mutarust/docs/cli.md` plus `mutation_testing.md` | The cost corollary: every mutant re-runs the **whole suite including e2e scenarios that spawn the CLI binary**. `--timeout-coefficient 5` is sized off the *longest single clean test*. An e2e scenario is a plausible candidate for that slot, so BDD additions directly inflate per-mutant cost. The mitigations are already wired: `--coverage` (skip uncovered mutants) and `--git-diff`/`-m`/`-f` scoping. |
| [Just et al. FSE 2014](https://homes.cs.washington.edu/~mernst/pubs/mutation-effectiveness-fse2014.pdf) | Academic grounding for the signal claim: mutation score correlates with real-fault detection. Adding fault-revealing tests raises mutation score even without a coverage change (94% of coverage-increasing augmentations). Mutation detects *assertion strength* changes that coverage metrics cannot. |

The net answer for decision 12(e): mutation testing does not test
*different code* than BDD scenarios. It grades whether the scenarios'
**assertions**, and the unit tests' assertions, would notice behavior
changes. The upstream tools' own docs treat it as a feedback loop *for*
public-interface tests, which BDD scenarios are. It also covers the
assertion strength of non-scenario tests, that is, the inline `src/` unit
tests that scenarios never touch. It is therefore not strictly subsumed by
BDD even where scenarios exist.

### 4.7 Selected-module mutation as a cost control

Both tools support it natively:

| Source | Finding |
|---|---|
| [skip_files.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/skip_files.md), [filter_mutants.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/filter_mutants.md), [in-diff.md](https://raw.githubusercontent.com/sourcefrog/cargo-mutants/main/book/src/in-diff.md) | cargo-mutants: `-f` and `-e` globs (path-aware), `--re` and `--exclude-re` mutant-name regex, `--package`, `--in-diff`, `--shard` sampling. Preview with `--list` or `--list-files`. |
| `mutarust/docs/cli.md`, `config.md` | mutarust: positional targets are file, directory, package or workspace. Also `exclude_dirs` (path prefixes), `--match` (fn-name regex), `--git-diff-lines` plus `--git-diff-base`, in-source `// mutator-disable-*` annotations, `--blacklist` (per-mutant checksums), `--run-mutant-id`. Preview with `--dry-run` or `--list-files`. |
| [Papadakis et al. TR 2021](https://homes.cs.washington.edu/~rjust/publ/practical_mutation_testing_tr_2021.pdf) | Independent validation at scale: Google mutates only probabilistically selected mutants per changelist. Path and diff-scoped mutation is the established large-org cost model, not a local hack. |
| `mutation_testing.md` §3 (local) | This repo already wraps the scoping: `mise run test:mutants -m <module>`, `-f <file>`, `--git-diff` (auto-pairs `--ignore-msi-with-no-mutations`). Workers are capped via `--workers`. No `--iterate` exists. Rerun cost equals scoping. |

The conclusion: both engines support module, file and diff-scoped mutation
natively. BDD adoption changes nothing here. If BDD scenarios raise
per-mutant suite runtime, the documented lever is narrower scoping, plus
mutarust's `--coverage` skipping of uncovered mutants. It is not a change of
tool.

### 4.8 Claims we could not check

1. **The cargo-llvm-cov profraw collision behavior for spawned processes.**
   The exact `LLVM_PROFILE_FILE` pattern cargo-llvm-cov sets (and whether it
   uses `%p` or `%m`) was not read from source. One empirical run settles it:
   count profraw files after a scenario-heavy `cargo llvm-cov nextest`.
2. **`--no-shuffle` (cargo-mutants).** It was not found in the book. Only a
   shuffling concept, referenced from shards.md, exists. The flag may not
   exist. Flag it if any plan assumed it.
3. **Spawned-binary coverage end-to-end in this repo** (e2e scenario →
   instrumented CLI binary → profraw → merged report). The mechanism is
   documented upstream (the external-tests recipe plus the LLVM Lit note).
   The exact CARGO_BIN_EXE plus rstest-bdd path is an **Inference** until
   the experiment runs (section 4.4).
4. **mutants.dev** was unreachable during research (transport error). No
   claim rests on it. The threshold-practice synthesis instead rests on the
   cargo-mutants book, mutarust ci.md, and the three academic and Google
   primary sources.
5. **Whether `cargo llvm-cov nextest` instruments the bin target** in this
   repo's single-package workspace (versus only lib and test targets): highly
   likely. This is an **Inference**. A `cargo llvm-cov nextest --no-report`
   then `ls target/llvm-cov-target/debug/` check would settle it.

---

## 5. Contradiction triage dossier, the input for decision (f)

This section verifies every docs-versus-code contradiction that the four
inventories list against the **current** tree (2026-10-29). Method: We read
ADRs and glossaries verbatim. We checked the code with CodeGraph and Read at
the cited lines.

The rule being served (ticket 07): a BDD scenario may only pin a guarantee
whose intent evidence agrees with the code. A contradiction needs a recorded
disposition before any scenario uses the affected guarantee. **No dispositions
are proposed here.** Each item ends with the realistic options and the facts
a human needs to choose.

Two terms define scope:

- "Invariance list" = research/10's 88 guarantees.
- "Class-level map" = research/10's exclusions.

### Family 1: Exit codes and command outcome

#### 1.1 The `CommandOutcome` doc claims exit 0 for both variants

- **Doc claim**: `src/cli/mod.rs:51-55` says: "Top-level result of a
  successful CLI command. … The process exit code is `0` for both variants;
  callers that need different behavior can match on the variant."
- **Type shape**: `CommandOutcome` has exactly two variants: `Completed` and
  `Aborted(UserAbort)` (`src/cli/mod.rs:57-63`). `UserAbort` splits into
  `Cancelled` and `Interrupted` (`src/cli/mod.rs:70-75`).
- **Code behavior**: `src/main.rs:22-29` maps `Completed | Aborted(Cancelled)`
  to SUCCESS (0). It maps `Aborted(Interrupted)` to `ExitCode::from(130)`. A
  unit pin exists: `ctrl_c_abort_exits_130` (`src/main.rs:58-63`). ADR-0004:41
  says: "Escape aborts the whole Command with … exit 0; Ctrl-C aborts the
  whole Command with terminal-conventional exit 130". ADR-0004:63 agrees
  with the code.
- **Still exists**: YES. The doc sentence is false for
  `Aborted(Interrupted)`. Nothing has changed since the inventory wrote it.
  ADR-0004 and main.rs agree with each other. Only the type-level doc lags.
- **Scenario-candidate**: YES. Research/10 invariance list: "Process and
  surface G1: Exit codes 0, 1, 2; never 101" (tier F). The 130 leg is a
  class-level one-off spawned-signal case. The contradiction directly touches
  the intent evidence for that guarantee.
- **Options**:
  - (a) Treat ADR-0004 and `src/main.rs` as intent. Change the
    `CommandOutcome` doc sentence (a doc-chase).
  - (b) Keep the doc and accept a three-way split that no ADR records. This
    is unlikely. It would re-legislate ADR-0004.
  - (c) Rewrite the sentence to: "exit 0 for `Completed` and
    `Aborted(Cancelled)`; 130 for `Aborted(Interrupted)`".

#### 1.2 ADR-0004's confirmation clause is partially unmet: no process-level exit-130 test

- **Doc claim**: ADR-0004:67 promises confirmation will "add contract tests
  for each stable diagnostic code, each User Abort origin, **Ctrl-C exit
  130**, and the absence of a Note or init scaffold after User Abort."
- **Code behavior**: the only 130 pin is the in-process unit test
  `ctrl_c_abort_exits_130` (`src/main.rs:58-63`). A search for `130|SIGINT`
  over `tests/` returns nothing. No spawned-signal e2e exists. The
  escape/no-side-effect pins are likewise in-process with injected providers
  (the 01 G5 evidence is unchanged).
- **Still exists**: YES. The confirmation clause is still partially unmet.
- **Scenario-candidate**: YES, narrowly. Research/10 class-level map: "Exit
  130: one spawned-signal case on Unix. The unit test stays." Exactly one
  scenario is planned for this leg. The gap is the reason it exists.
- **Options**:
  - (a) Spec the one Unix SIGINT scenario. ADR-0004's confirmation.
  - (b) Amend ADR-0004's confirmation. Let it accept the in-process pin.
  - (c) Leave the gap as recorded.

### Family 2: Config trust (ADR-0002)

Background: trust is keyed by `Blake3PathHash` of the canonical path
(`src/file_tracker.rs:37-49,89-101`). `ConfigTrustStatus` is
`Trusted | Untrusted | MissingBaseline | Stale`. `Stale` means a
companion-hash mismatch (`src/config/trust.rs:84-95,93-94`).

#### 2.1 A moved directory leaves the trust entry "stale" (priority case)

- **Doc claim**: ADR-0002:82-83 says: "trust entry becomes **stale**, tool
  warns and suggests re-trust." ADR-0002:98-99 (Bad) contradicts it:
  "renaming a directory invalidates trust".
- **Code behavior**: `config_file_trust_check` (`trust.rs:240-259`) returns
  `Untrusted` when `!contains(root)` (`:245-246`). It returns `Stale` only
  on digest mismatch (`:253-258`). A moved root has a new canonical path and
  a new hash. The result is **`Untrusted`**, never `Stale`. The config
  glossary sides with the code (`src/config/GLOSSARY.md:31-37`).
- **Still exists**: YES. The ADR is internally inconsistent. Code and
  glossary agree on `Untrusted`.
- **Scenario-candidate**: YES. Invariance "G23: Four trust statuses with
  distinct meaning" (F) plus G24/G26.
- **Options**:
  - (a) The doc wins. Rewrite ADR-0002:82-83 to `Untrusted` plus
    `trust clean`.
  - (b) The code wins. Record the "stale" wording as superseded by the
    companion-hash design. The ADR's own More Information (:134-139)
    narrates that evolution. G23's table is authoritative.
  - (c) Drop the guarantee. This is the weakest option. G23 is F.

#### 2.2 ADR-0002 "existence check only" hot path

- **Doc claim**: ADR-0002:84-85,107-109 says the trust check is "just a
  file existence check" that runs "before … is read".
- **Code behavior**: the check reads and hashes the config
  (`trust.rs:253-254`). Only *TOML parsing* is deferred. The stage order
  (trust before parse) is intact.
- **Still exists**: YES. The cheap-hot-path consequence is stale.
- **Scenario-candidate**: PARTIAL. G22 "Four ordered config failure stages"
  (F stages / M codes) is the observable half. The performance claim is
  unobservable. No scenario may encode "the check does not read the file".
- **Options**:
  - (a) Amend the ADR to "reads and hashes without parsing".
  - (b) Treat the sentence as superseded consequence prose. Rely on G22's
    table.
  - (c) Leave it. Forbid scenarios that assert read-avoidance.

### Family 3: Dialog ADR-0003 confirmation citations

- **Doc claim**: ADR-0003:119-131 cites test names (`select_index_*` etc.)
  and locations that do not exist. The behavior it describes matches the
  code. The real tests are `recovers_the_object_by_position`
  (`preset.rs:373`), `disambiguates_duplicate_labels` (`:385`),
  empty-selection guards, and `is_usable_as_dyn_dialog_provider` (`:308`).
- **Still exists**: YES. Citation drift only.
- **Scenario-candidate**: NO. Dialog selection is class-level (unit
  disposition). This is ADR hygiene, not a blocker.
- **Options**:
  - (a) Update the Confirmation to the real names.
  - (b) Leave it as a historical record.

### Family 4: Misleading sort test names

- **Claim**: `sort_orders_by_the_named_field_ascending_by_default`
  (`src/cli/list.rs:139`) and `…_rows_…_ascending_by_default`
  (`src/cli/table.rs:148`) both set `asc: true` explicitly
  (`list.rs:148-152`). The true default is **descending**. The flag doc
  (`src/cli/mod.rs:287-289`), `SortDirection::default()` (`sort.rs:269-271`),
  and the real pin `defaults_to_descending_without_flags` (`mod.rs:843`)
  all agree.
- **Still exists**: YES.
- **Scenario-candidate**: YES **as a hazard**. Invariance "G6: Sort default
  direction and CLI sort flags" (D/M). These names are evidence *against*
  the guarantee. Do not mine them.
- **Options**:
  - (a) Rename the two tests before you mine.
  - (b) Record in the scenario backlog that G6's intent evidence is the flag
    doc plus `:843` plus the glossary.

### Family 5: ADR-0008 names nonexistent `SourceAtom` variants (priority case)

- **Doc claim**: ADR-0008:75-77 says `SourceAtom` is `Tag`, `Folder`,
  `Class`, or `Glob`.
- **Code behavior**: the enum is
  `enum SourceAtom { Tag(String), Path(GlobPattern), Class {…} }`
  (`src/query/grammar/source.rs:190-202`). `Folder` and `Glob`
  unified into `Path` (`:210-212`). The user-visible grammar matches the ADR
  prose (`#tag`, `folder/`, `@Class`, quoted globs).
- **Still exists**: YES. The type names are wrong. The behavior is right.
- **Scenario-candidate**: YES. Invariance "G1: Source-expression grammar"
  (F+M). Research/10 ties this item to 02 G1. Scenarios may pin grammar
  behavior. They must not cite the ADR's variant names as the type
  vocabulary.
- **Options**:
  - (a) Fix ADR-0008:75-77 to `Tag | Path | Class`.
  - (b) Record the ADR as prose-level authority only. The enum spelling
    belongs to code.
  - (c) Leave it. Instruct scenario authors to cite `source.rs`.

### Family 6: ADR-0005 "clear runtime error" for terminal-then-filter

- **Doc claim**: ADR-0005:71-74 promises "a clear runtime error with
  template name, line, and column."
- **Code behavior**: terminal filters return `String`/`usize`
  (`query.rs:447-472`). There is no dedicated error variant. A chained
  pipeline degrades to a generic minijinja error. No test asserts the ADR's
  claim anywhere.
- **Still exists**: YES. Research 02 classified it `uncertain`.
- **Scenario-candidate**: NO. It is not in the invariance list. Pinning "a
  clear runtime error" would pin an unspecified generic message.
- **Options**:
  - (a) Demote the ADR sentence to "misuse surfaces whatever minijinja
    reports".
  - (b) Implement a dedicated error plus a test.
  - (c) Leave it unverified and exclude it from scenarios (the current
    de-facto).

### Family 7: Template-resolution precedence glossary vs loader

- **Doc claim**: `src/template/GLOSSARY.md:28-32` gives the resolution
  order: "first as exact paths, then in local template directories, then in
  global directories."
- **Code behavior**: `TemplateLoader::find` iterates directories
  local-first. Per directory it tries the exact path, then the extensionless
  stem (`loader.rs:94-108`, order `:98-105`). A local *stem* match beats a
  *global exact-path* match. This cross-dimension ordering is one the
  glossary does not allow.
- **Still exists**: YES.
- **Scenario-candidate**: YES. Invariance "G10: Template resolution order"
  (D). The local-stem-vs-global-exact scenario is precisely the
  misdescribed case.
- **Options**:
  - (a) Reword the glossary to the per-directory order. The loader docs win.
  - (b) Change the loader to the glossary's cross-dimension order. This is
    a behavior change and needs an ADR touch.
  - (c) Pin only the uncontroversial halves: local overrides global, and
    `../evil` is rejected.

### Family 8: Duplicate ADR number 9, both `proposed`, both landed

- **Doc claim**: `docs/adr/0009-index-refresh.md` and
  `0009-filter-date-literals.md` both carry `number: 9` and
  `status: proposed`.
- **Code behavior**: both decisions are implemented and pinned. Date
  literals pin at `filter.rs:1462,1494` and
  `tests/integration/query.rs:486`. Index-refresh fingerprint drift pins at
  `src/index/fingerprint.rs` (arch ticket 25 landed).
- **Still exists**: YES.
- **Scenario-candidate**: YES through the date-literals half. Invariance
  "B4: Temporal literals in filters" (M) and "G5: Filter semantics"
  (F+M). Research/02 G5 notes that `proposed` status weakens intent
  evidence. The duplicate number is a citation hazard: "ADR-0009" alone is
  ambiguous.
- **Options**:
  - (a) Renumber one and/or promote both to `accepted`. ADR policy needs
    human review.
  - (b) Record which file each citation means. Accept `proposed` as
    sufficient for B4.
  - (c) Substitute the filter module spec (`filter.rs:1-18`) plus the
    integration pin as B4's intent evidence.

### Family 9: ADR-0009 (index refresh) format version as `Drifted` vs wipe-at-open

- **Doc claim**: ADR-0009:33-41 says freshness includes the store format
  version: "Drifted: the stored config fingerprint **or format version** no
  longer matches."
- **Code behavior**: a version mismatch triggers **recreate-at-open**
  (`store.rs:927-979`) before any drift state exists. The index glossary
  scopes `Drifted` to config only (`src/index/GLOSSARY.md:34-41`). Arch
  ticket 25 item 2 flags the same gap.
- **Still exists**: YES. The ADR is the outlier. Glossary and code agree.
- **Scenario-candidate**: NO. It is class-level (Free strategy). The
  planned seam disposition says "results stay identical after the index
  file is deleted; no scenario for the wipe".
- **Options**:
  - (a) Amend the ADR to scope `Drifted` to config and describe
    wipe-at-open.
  - (b) Implement format-version drift reporting (arch ticket 25 item 2)
    and keep the ADR.
  - (c) Record the ADR sentence as superseded.

### Family 10: Glossary vocabulary with no code: slug, callout, footnotes (priority case)

- **Slugs**: `src/note/GLOSSARY.md:45-52` defines GitHub-style anchors and
  parse-time dedup with `slug-1` suffixes. Code: **none**
  (`rg -i slug src/ tests/` hits only unrelated schema-select fixtures).
- **Callouts**: `src/note/GLOSSARY.md:98-105` defines 13 canonical blockquote
  types. Code: zero references anywhere.
- **Footnotes**: `src/note/GLOSSARY.md:84-96` says "the parser emits them
  independently and the index cross-references them." Code: a footnote
  reference event only re-injects source text (`parser.rs:252-255`). There
  is no definition/reference record type. `src/index/` handles zero
  footnotes. The whole root-glossary "Navigation & Resolution" section
  (`src/GLOSSARY.md:70-115`: `NavTarget`, `Resolution`, `Ambiguity`, "LSP
  Locations", `Comparison Key`) is aspirational vocabulary with no code.
- **Still exists**: YES.
- **Scenario-candidate**: NO. Research/10 states outright: "The slug,
  callout and footnote glossary claims have no code. They get no
  scenario." Arch ticket 23 admits the gap.
- **Options**:
  - (a) Trim the glossary entries or mark them "planned (arch ticket 23)".
  - (b) Implement the features.
  - (c) Leave it and rely on the research/10 exclusion. Any future
    anchor-targeting scenario (P21/N17) must not assume slug-dedup
    behavior.

### Family 11: ADR-0004 "domain modules do not implement miette" vs `src/query/error.rs`

- **Doc claim**: ADR-0004:67 says: "Review confirms domain modules do not
  implement miette diagnostics."
- **Code behavior**: `impl Diagnostic for QueryError` (`query/error.rs:66-84`).
  `#[derive(Diagnostic)]` sits on `QuerySyntaxError` (`:111-125`) with
  `#[source_code]`/`#[label]` spans. The 14 root modules remain clean. An
  arguable reading: spans-as-data are not terminal *presentation*.
- **Still exists**: YES.
- **Scenario-candidate**: NO. It is gate-owned (structural). It belongs to
  `check:visibility`/lints.
- **Options**:
  - (a) Amend the confirmation to exempt span-carrying syntax errors in
    query.
  - (b) Move the impls to the CLI edge. This is a large refactor and
    contradicts ADR-0008:97-98.
  - (c) Record reading (a) as the standing interpretation.

### Family 12: `test-utils` surface: "exactly the 46 identifiers"

- **Doc claim**: `src/lib.rs:181-182` says 46.
- **Code behavior**: 47 re-exports. The machine baseline is authoritative
  (see §1.6).
- **Still exists**: YES.
- **Scenario-candidate**: NO. It is gate-owned. The seam is already frozen
  better than any scenario could freeze it.
- **Options**:
  - (a) Fix the comment to 47.
  - (b) Leave it. The baseline wins.

### Family 13: `src/yaml.rs` doc-claims with zero pinning

- **Doc claim**: `src/yaml.rs:7-15` lists four quirks under
  `serde_yaml_compat`: YAML 1.1 binary literals as integers, overflowing
  float stays string, integer past `u64::MAX` errors, non-scalar mapping key
  errors, `<<` merge keys ordinary.
- **Code behavior**: the config is real (`yaml.rs:18-19`). The quirks that
  ARE pinned (leading zero, merge key, duplicate keys, yes/no) live in
  `src/note/metadata.rs:317-365`. **None** of the four named claims has a
  test. `src/yaml.rs` itself has 0 tests.
- **Still exists**: YES.
- **Scenario-candidate**: PARTIAL. Invariance "G8: One YAML config for
  frontmatter and template filters (parity rule)" (D) is the candidate. The
  four quirks are unit material and do not block the parity scenario.
- **Options**:
  - (a) Add unit pins for the four quirks (move claims or tests).
  - (b) Trim the doc to the pinned quirks.
  - (c) Spec only the parity rule.

### Batch: stale-citation / hygiene items (not scenario blockers)

| # | Item | Still exists | Notes |
| --- | --- | --- | --- |
| B1 | ADR-0001/0008 line drift (`:76/83/86/91`, `:52` vs actual locations) | YES | citation fix; G12's pins remain valid |
| B2 | ADR-0005 filename vs title drift (`0005-template-query.md`) | YES | resolved by title in `0009-index-refresh.md:21` |
| B3 | ADR-0005 names table `file_classes_by_path`; code string is `classes_by_path` (`tables.rs:66-70`) | YES | store internals; seam 4 owns user-visible halves |
| B4 | `path.rs:21-24` TOCTOU caveat: a documented limitation, not a contradiction | YES | later tickets must not spec "race-free writes" from G7 |
| B5 | `Defect class:` taxonomy drift: ~15 tags, no taxonomy in docs; `trust-before-parse` pair collapsed to one spelling | YES | research/10 routes to ticket 12/13 |
| B6 | Ticket-figure drift: "~269 tests in root modules" is actually 278 | YES | survey correction only |

### Missed by the inventories (noticed in passing)

1. Root glossary "Navigation & Resolution" is broader than the footnote item
   (see Family 10). The whole section is code-free.
2. ADR-0005 eighth-table name drift (B3 above).
3. ADR-0002:60 "directory's canonical path" wording looseness. This is
   minor. Read the grant path end-to-end when you triage 2.1.

### Cross-check: verified still consistent

- 03 §D "Checked and consistent" list re-verified clean
  (`TaskStatusType`/`NonTask`, malformed-YAML handling).
- research 05 G4's "0 miette in the 14 root modules" still holds.
- The config glossary trust vocabulary matches `trust.rs` exactly. The
  glossary is the doc that sides with code in family 2.

---

*End of dossier. Prepared for the ticket-12 grilling session; no decisions
have been made.*
# Framework Currency and Upstream Maintenance Research

Layout note (2026-11-05): this record is older than the move of the scenarios to `tests/features.rs`. Where it names `mod bdd` or `tests/e2e/bdd/`, read `mod steps` and `tests/features/steps/`. Where it names a flat feature file, read the capability folder. Section 5 of `.scratch/bdd-testing/spec.md` is current.

- Date: 2026-11-03
- Target ticket: Ticket 14 (Adoption roadmap)
- Tag prefix: `G`
- Upstream target: `rstest-bdd` (=0.6.0)

## 1. Question

Ticket 06 selected `rstest-bdd` 0.6.0 for BDD adoption. What maintenance and
currency facts changed between initial verification and 2026-11-03? What
upstream risks exist? What is the cost of an exit strategy?

## 2. Method and Source List

This research inspected primary package registries, repositories, documentation,
and compiler releases. The research opened and read 30 distinct primary sources.

Primary sources:

1. Crates.io API for `rstest-bdd`: `https://crates.io/api/v1/crates/rstest-bdd`
2. GitHub repository metadata: `https://api.github.com/repos/leynos/rstest-bdd`
3. GitHub commit log:
   `https://api.github.com/repos/leynos/rstest-bdd/commits?per_page=20`
4. Upstream repository manifest:
   `https://raw.githubusercontent.com/leynos/rstest-bdd/main/Cargo.toml`
5. Upstream change log:
   `https://raw.githubusercontent.com/leynos/rstest-bdd/main/docs/CHANGELOG.md`
6. Upstream GitHub releases:
   `https://api.github.com/repos/leynos/rstest-bdd/releases`
7. Upstream issue search for `catch_unwind`:
   `https://api.github.com/search/issues?q=repo:leynos/rstest-bdd+catch_unwind`
8. Upstream issue search for `disallowed_methods`:
   `https://api.github.com/search/issues?q=repo:leynos/rstest-bdd+disallowed_methods`
9. Upstream macro codegen `assembly/mod.rs`:
   `https://raw.githubusercontent.com/leynos/rstest-bdd/main/crates/rstest-bdd-macros/src/codegen/wrapper/emit/assembly/mod.rs`
10. Upstream macro codegen `assembly/lint_config.rs`:
    `https://raw.githubusercontent.com/leynos/rstest-bdd/main/crates/rstest-bdd-macros/src/codegen/wrapper/emit/assembly/lint_config.rs`
11. Upstream pattern parsing `hint.rs`:
    `https://raw.githubusercontent.com/leynos/rstest-bdd/main/crates/rstest-bdd-patterns/src/hint.rs`
12. Upstream pattern parsing `pattern/mod.rs`:
    `https://raw.githubusercontent.com/leynos/rstest-bdd/main/crates/rstest-bdd-patterns/src/pattern/mod.rs`
13. Upstream pattern library `lib.rs`:
    `https://raw.githubusercontent.com/leynos/rstest-bdd/main/crates/rstest-bdd-patterns/src/lib.rs`
14. Upstream change detection ADR-010:
    `https://raw.githubusercontent.com/leynos/rstest-bdd/main/docs/adr-010-feature-file-change-detection.md`
15. Upstream open issues list:
    `https://api.github.com/repos/leynos/rstest-bdd/issues?state=open`
16. Crates.io API for `cargo-nextest`:
    `https://crates.io/api/v1/crates/cargo-nextest`
17. GitHub releases for `cargo-nextest`:
    `https://api.github.com/repos/nextest-rs/nextest/releases`
18. Nextest integration documentation: `https://nexte.st/docs/integrations/`
19. Nextest change log:
    `https://raw.githubusercontent.com/nextest-rs/nextest/main/site/src/changelog.md`
20. Rust 1.99.0 release announcement:
    `https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/`
21. Rust 1.99.1 release announcement:
    `https://blog.rust-lang.org/2026/10/15/Rust-1.99.1/`
22. Rust issue tracker on `#[expect]` lint expectations:
    `https://github.com/rust-lang/rust/issues/157093`
23. Crates.io API for `cucumber`: `https://crates.io/api/v1/crates/cucumber`
24. Cucumber-rs issue 370 on cargo-nextest:
    `https://api.github.com/repos/cucumber-rs/cucumber/issues/370`
25. Cucumber-rs issue 370 discussion:
    `https://api.github.com/repos/cucumber-rs/cucumber/issues/370/comments`
26. Crates.io API for `foldhash`: `https://crates.io/api/v1/crates/foldhash`
27. Local comparison note:
    `.scratch/bdd-strategy/research/rust-bdd-crate-comparison.md`
28. Local crate note: `.scratch/bdd-strategy/research/rstest-bdd.md`
29. Framework decision ticket:
    `.scratch/bdd-strategy/issues/06-framework-revalidation.md`
30. Cucumber Gherkin reference specification:
    `https://cucumber.io/docs/gherkin/reference/`

Sources tried and found useless:

- `https://raw.githubusercontent.com/nextest-rs/nextest/main/CHANGELOG.md` (HTTP
  404; changelog lives at `site/src/changelog.md`).
- `https://api.github.com/repos/nextest-rs/nextest/contents/site/src/changelog`
  (HTTP 404; path is a file, not a directory).
- Web query `site:github.com/nextest-rs/nextest/releases "rstest"` (0 relevant
  release notes).

## 3. Findings

- `[G01]` `rstest-bdd` version 0.6.0 remains the latest release on crates.io.
  The author published it on 2026-09-14. No version 0.7 or patch exists. Source:
  `https://crates.io/api/v1/crates/rstest-bdd` (accessed 2026-11-03, primary,
  confidence: high).

- `[G02]` Total downloads grew from 74,901 to 83,881 (+8,980 downloads). Version
  0.6.0 downloads reached 10,970. Recent 90-day downloads are 45,356. Source:
  `https://crates.io/api/v1/crates/rstest-bdd` (accessed 2026-11-03, primary,
  confidence: high).

- `[G03]` The upstream git repository is active. Commit 4d585b7 landed on
  2026-10-31 and pushed on 2026-11-02. Recent commits update CI runners, Dylint,
  mold, and dependencies. Source:
  `https://api.github.com/repos/leynos/rstest-bdd/commits?per_page=20` (accessed
  2026-11-03, primary, confidence: high).

- `[G04]` Upstream bus factor remains 1. Leynos (Payton McIntosh) authored all
  commits. The repository has 13 stars, 0 forks, 1 subscriber, and 42 open
  issues. Source: `https://api.github.com/repos/leynos/rstest-bdd` (accessed
  2026-11-03, primary, confidence: high).

- `[G05]` Upstream issue triage is active. Leynos processes pull requests
  quickly. Dependabot PRs run on Ubuntu runners via workflow rules. Source:
  `https://api.github.com/repos/leynos/rstest-bdd/commits` (accessed 2026-11-03,
  primary, confidence: high).

- `[G06]` No breaking API changes exist after 0.6.0. The repository contains
  unreleased development commits, but no new release has been tagged. Source:
  `https://raw.githubusercontent.com/leynos/rstest-bdd/main/docs/CHANGELOG.md`
  (accessed 2026-11-03, primary, confidence: high).

- `[G07]` The upstream maintainer has not filed or fixed an issue for
  `clippy::disallowed_methods`. Upstream macro file `lint_config.rs` suppresses
  7 Clippy lints, but omits `disallowed_methods`. Source:
  `https://raw.githubusercontent.com/leynos/rstest-bdd/main/crates/rstest-bdd-macros/src/codegen/wrapper/emit/assembly/lint_config.rs`
  (accessed 2026-11-03, primary, confidence: high).

- `[G08]` Step wrappers still invoke `std::panic::catch_unwind` in macro output
  `assembly/mod.rs:188-212`. The module attribute
  `#[expect(clippy::disallowed_methods)]` remains required on `mod bdd;`.
  Source:
  `https://raw.githubusercontent.com/leynos/rstest-bdd/main/crates/rstest-bdd-macros/src/codegen/wrapper/emit/assembly/mod.rs`
  (accessed 2026-11-03, primary, confidence: high).

- `[G09]` cargo-nextest reached version 0.9.150 on 2026-10-26. Nextest runs each
  libtest test as an isolated process. Nextest compatibility with `rstest-bdd`
  remains complete. Source:
  `https://raw.githubusercontent.com/nextest-rs/nextest/main/site/src/changelog.md`
  (accessed 2026-11-03, primary, confidence: high).

- `[G10]` Upstream ADR-010 confirmed macro-emitted `include_bytes!` tracking for
  `.feature` files. Cargo rebuilds test binaries when developers edit existing
  `.feature` files. Adding a new `.feature` file still requires touching a Rust
  source file or adding a build script. Source:
  `https://raw.githubusercontent.com/leynos/rstest-bdd/main/docs/adr-010-feature-file-change-detection.md`
  (accessed 2026-11-03, primary, confidence: high).

- `[G11]` Bare `{name}` placeholders without format specifiers compile into
  regex pattern `^Given (.+?)$` with `type_hint: None`. They avoid Clippy lint
  `literal_string_with_formatting_args`. Source:
  `https://raw.githubusercontent.com/leynos/rstest-bdd/main/crates/rstest-bdd-patterns/src/hint.rs`
  (accessed 2026-11-03, primary, confidence: high).

- `[G12]` Upstream MSRV remains Rust 1.88 in `Cargo.toml`. Source:
  `https://raw.githubusercontent.com/leynos/rstest-bdd/main/Cargo.toml`
  (accessed 2026-11-03, primary, confidence: high).

- `[G13]` Transitive dependency `foldhash` 0.2.0 uses the Zlib license.
  Workspace `deny.toml` must allow `"Zlib"` for compliance. Source:
  `https://crates.io/api/v1/crates/foldhash` (accessed 2026-11-03, primary,
  confidence: high).

- `[G14]` Upstream added mold linker configuration for Linux development in PR
  907. This change does not affect downstream crate consumers. Source:
  `https://api.github.com/repos/leynos/rstest-bdd/commits/4d585b789dcc8f070c3c7b7cb141388a65afcbe5`
  (accessed 2026-11-03, primary, confidence: high).

- `[G15]` Cucumber-rs remains at version 0.23.0 (published 2026-04-23). Issue
  370 for cargo-nextest remains open and blocked by architectural differences.
  Source: `https://api.github.com/repos/cucumber-rs/cucumber/issues/370`
  (accessed 2026-11-03, primary, confidence: high).

- `[G16]` Current stable Rust is 1.99.1, released on 2026-10-15 following 1.99.0
  on 2026-10-01. Source: `https://blog.rust-lang.org/2026/10/15/Rust-1.99.1/`
  (accessed 2026-11-03, primary, confidence: high).

- `[G17]` Rust 1.99.1 does not change `#[expect(...)]` behavior on module
  declarations. The attribute silences inner macro lints as documented in ticket
  06. Source: `https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/` (accessed
  2026-11-03, primary, confidence: high).

- `[G18]` Feature files use standard Gherkin syntax. Feature files remain 100
  percent portable to cucumber-rs or other test runners. Source:
  `https://cucumber.io/docs/gherkin/reference/` (accessed 2026-11-03, primary,
  confidence: high).

- `[G19]` Step function definitions differ across frameworks. `rstest-bdd`
  injects arguments directly into step functions. `cucumber-rs` requires a
  shared `&mut World` argument. Plain tests call helper functions directly.
  Source: `.scratch/bdd-strategy/research/rust-bdd-crate-comparison.md` (accessed
  2026-11-03, primary, confidence: high).

## 4. Disagreements

The primary disagreement concerns issue filing for `clippy::disallowed_methods`.
The ticket 06 notes suggested filing an upstream issue for `catch_unwind` lint
suppression. However, upstream enforces `allow_attributes = "deny"` across its
codebase. If upstream emitted `#[expect(clippy::disallowed_methods)]`,
downstream crates without that disallowed method would fail compilation with
`unfulfilled_lint_expectations`. Therefore, downstream module-level suppression
is the correct design. The evidence favors maintaining the consumer workaround
rather than expecting an upstream macro change.

## 5. Does Not Transfer to This Repo

1. Upstream mold linker settings: Upstream enabled mold for Linux in PR 907. Our
   repository manages toolchains and linkers through mise tasks and local cargo
   configs. We do not adopt upstream internal development flags.

2. Upstream Dylint snapshot pins: Upstream runs specialized Dylint lint drivers.
   Our repository uses standard Clippy and machine-enforced layer checks.

3. Ubicloud CI runners: Upstream uses Ubicloud runners for Linux workflows. Our
   project uses GitHub-hosted runners.

## 6. Implications for Ticket 14

- Option 1: Maintain the exact pin `rstest-bdd =0.6.0` in the adoption roadmap
  `[G01]`. No newer release exists. The current release is stable and tested.

- Option 2: Document the `deny.toml` license update as a prerequisite slice task
  `[G13]`. Adopting `rstest-bdd` pulls `foldhash` with the Zlib license.

- Option 3: Document the `mod bdd;` lint suppression requirement in the pilot
  dossier `[G07, G08]`. The pilot slice must apply
  `#[expect(clippy::disallowed_methods)]` on the module declaration.

- Option 4: Rely on standard `include_bytes!` tracking for feature edits, and
  document touching a source file when adding a new `.feature` file `[G10]`.
  This avoids introducing a mandatory `build.rs` during initial adoption.

## 7. Open Gaps

1. Upstream has not published a release since September 2026. No breaking
   changes exist, but upstream release cadence depends entirely on one
   maintainer.

2. Adding a new `.feature` file does not automatically trigger compilation
   without a build script or file modification.

## 8. Acceptance Items

### Delta Table

| Ticket 06 Claim | Still True? | Evidence Tag |
|---|---|---|
| Pin `rstest-bdd =0.6.0` | Still True | `[G01]` |
| Single maintainer bus factor | Still True | `[G04]` |
| Compatible with cargo-nextest | Still True | `[G09]` |
| `catch_unwind` requires module lint suppression | Still True | `[G07, G08]` |
| Bare `{name}` placeholders avoid formatting lints | Still True | `[G11]` |
| Feature file edits trigger rebuilds via `include_bytes!` | Still True | `[G10]` |
| Adding new `.feature` file requires touch or build.rs | Still True | `[G10]` |
| MSRV is Rust 1.88 | Still True | `[G12]` |
| Foldhash requires Zlib license in `deny.toml` | Still True | `[G13]` |
| Cucumber-rs lacks process-per-scenario nextest support | Still True | `[G15]` |

### New Risks

1. Low bus factor: Leynos remains the sole committer. If the maintainer becomes
   inactive, our project must vendor or maintain the crate.

2. Unreleased commits: Upstream has accumulated 15+ commits since 0.6.0 without
   a release. Future releases may bundle multiple dependency upgrades.

### Exit Strategy

If our team decides to replace `rstest-bdd`, two exit routes exist:

1. Migration to Cucumber-rs:
   - Feature files: 100 percent portable without syntax changes `[G18]`.
   - Step definitions: High rewrite cost. Must rewrite step signatures from
     typed parameters to mutable World references `[G19]`.
   - Test runner: High architectural cost. Must abandon process-per-scenario
     isolation under cargo-nextest `[G15]`.

2. Migration to Plain Rust Integration Tests:
   - Feature files: Abandoned or converted to test doc comments `[G18]`.
   - Step definitions: Low to moderate rewrite cost. Step logic converts into
     standard Rust helper functions `[G19]`.
   - Test runner: Zero cost. Plain tests run natively under cargo-nextest with
     complete isolation `[G09]`.

Migration cost estimate: Migrating 5 to 10 scenarios to plain integration tests
takes 2 to 4 developer hours. Migrating to cucumber-rs takes longer due to
runner reconfiguration and World management.

### Explicit Verdict Statement

No finding contradicts or invalidates the ticket 06 decision. `rstest-bdd
=0.6.0` remains the correct choice for BDD adoption under cargo-nextest. Ticket
06 verdict MUST NOT be reopened.

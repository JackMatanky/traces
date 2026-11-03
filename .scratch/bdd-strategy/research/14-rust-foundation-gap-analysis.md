# Research 14: Rust Foundation Gap Analysis

This note supports ticket 14 of the BDD adoption map. It evaluates the codebase
foundation against evidence-backed Rust best practices.

## 1. Question

What is the evidence-backed best-practice baseline for a Rust library and CLI
codebase that prepares for a multi-crate workspace (`core`, `cli`, `lsp`,
`mcp`), and how does this repository compare?

## 2. Method and Source List

Date: 2026-11-03. The review searched and opened 33 primary and secondary
sources. It inspected repository manifests, configurations, tasks, CI workflows,
and ADRs.

### Primary Sources

- `[E01]` Rust API Guidelines Checklist:
  <https://rust-lang.github.io/api-guidelines/checklist.html> (accessed
  2026-11-03, primary, high).
- `[E02]` Rust Compiler Dev Guide - Testing:
  <https://rustc-dev-guide.rust-lang.org/tests/intro.html> (accessed 2026-11-03,
  primary, high).
- `[E03]` Clippy Lints Guide: <https://doc.rust-lang.org/clippy/lints.html>
  (accessed 2026-11-03, primary, high).
- `[E04]` The Cargo Book - Lints & Workspace Inheritance:
  <https://doc.rust-lang.org/cargo/reference/lints.html> (accessed 2026-11-03,
  primary, high).
- `[E05]` cargo-deny Checks Overview:
  <https://embarkstudios.github.io/cargo-deny/checks/index.html> (accessed
  2026-11-03, primary, high).
- `[E06]` cargo-deny Bans Configuration:
  <https://embarkstudios.github.io/cargo-deny/checks/bans/cfg.html> (accessed
  2026-11-03, primary, high).
- `[E07]` RustSec Advisory Database & cargo-audit: <https://rustsec.org/>
  (accessed 2026-11-03, primary, high).
- `[E08]` cargo-vet Documentation:
  <https://mozilla.github.io/cargo-vet/how-it-works.html> (accessed 2026-11-03,
  primary, high).
- `[E09]` cargo-semver-checks README:
  <https://raw.githubusercontent.com/obi1kenobi/cargo-semver-checks/main/README.md>
  (accessed 2026-11-03, primary, high).
- `[E10]` cargo-public-api README:
  <https://raw.githubusercontent.com/cargo-public-api/cargo-public-api/main/README.md>
  (accessed 2026-11-03, primary, high).
- `[E11]` cargo-mutants CI Guide: <https://mutants.rs/ci.html> (accessed
  2026-11-03, primary, high).
- `[E12]` cargo-mutants In-Diff Guide: <https://mutants.rs/in-diff.html>
  (accessed 2026-11-03, primary, high).
- `[E13]` cargo-llvm-cov README:
  <https://raw.githubusercontent.com/taiki-e/cargo-llvm-cov/main/README.md>
  (accessed 2026-11-03, primary, high).
- `[E14]` cargo-hack README:
  <https://raw.githubusercontent.com/taiki-e/cargo-hack/main/README.md>
  (accessed 2026-11-03, primary, high).
- `[E16]` Miri Documentation:
  <https://raw.githubusercontent.com/rust-lang/miri/master/README.md> (accessed
  2026-11-03, primary, high).
- `[E17]` Rust Fuzz Book Guide:
  <https://rust-fuzz.github.io/book/cargo-fuzz/guide.html> (accessed 2026-11-03,
  primary, high).
- `[E18]` Rust Fuzz Book CI:
  <https://rust-fuzz.github.io/book/cargo-fuzz/ci.html> (accessed 2026-11-03,
  primary, high).
- `[E19]` Proptest Tips & Best Practices:
  <https://proptest-rs.github.io/proptest/proptest/tips-and-best-practices.html>
  (accessed 2026-11-03, primary, high).
- `[E20]` Proptest Failure Persistence:
  <https://proptest-rs.github.io/proptest/proptest/failure-persistence.html>
  (accessed 2026-11-03, primary, high).
- `[E21]` Nextest Repository Configuration:
  <https://nexte.st/docs/configuration/> (accessed 2026-11-03, primary, high).
- `[E22]` RFC 2495 MSRV Specification:
  <https://rust-lang.github.io/rfcs/2495-min-rust-version.html> (accessed
  2026-11-03, primary, high).
- `[E23]` cargo-modules README:
  <https://raw.githubusercontent.com/regexident/cargo-modules/master/README.md>
  (accessed 2026-11-03, primary, high).
- `[E24]` release-plz Documentation: <https://release-plz.dev/docs> (accessed
  2026-11-03, primary, high).
- `[E25]` cargo-dist README:
  <https://raw.githubusercontent.com/axodotdev/cargo-dist/main/README.md>
  (accessed 2026-11-03, primary, high).
- `[E26]` OpenSSF Scorecard Checks:
  <https://raw.githubusercontent.com/ossf/scorecard/main/docs/checks.md>
  (accessed 2026-11-03, primary, high).
- `[E27]` The Cargo Book FAQ on Cargo.lock:
  <https://doc.rust-lang.org/cargo/faq.html> (accessed 2026-11-03, primary,
  high).
- `[E28]` The rustc Book - Remap Source Paths:
  <https://doc.rust-lang.org/rustc/remap-source-paths.html> (accessed
  2026-11-03, primary, high).
- `[E29]` Astral uv Workspace Manifest:
  <https://raw.githubusercontent.com/astral-sh/uv/main/Cargo.toml> (accessed
  2026-11-03, primary, high).
- `[E30]` BurntSushi ripgrep Manifest:
  <https://raw.githubusercontent.com/BurntSushi/ripgrep/master/Cargo.toml>
  (accessed 2026-11-03, primary, high).
- `[E31]` rust-analyzer Workspace Manifest:
  <https://raw.githubusercontent.com/rust-lang/rust-analyzer/master/Cargo.toml>
  (accessed 2026-11-03, primary, high).
- `[E32]` Rust Arkitect README:
  <https://raw.githubusercontent.com/pfazzi/rust_arkitect/main/README.md>
  (accessed 2026-11-03, primary, medium).
- `[E33]` GitHub Dependabot Options Reference:
  <https://docs.github.com/en/code-security/reference/supply-chain-security/dependabot-options-reference>
  (accessed 2026-11-03, primary, high).

### Secondary Sources

- `[E15]` Rust Project Primer - Unused Dependencies:
  <https://www.rustprojectprimer.com/checks/unused.html> (accessed 2026-11-03,
  secondary, medium).

### Sources tried and discarded

- ArchUnitTS / Medium notes: discarded because they address TypeScript, not
  Rust.
- generic README templates: discarded because they do not contain Rust-specific
  testing rules.

## 3. Findings

- `[E01]` Rust API Guidelines require explicit public types to implement
  `Debug`, `Clone`, and common traits eagerly.
- `[E02]` The compiler test guide separates unit tests, integration tests, and
  documentation tests. Unit tests live in separate files to prevent full
  recompilation.
- `[E03]` Clippy forbids enabling the `restriction` group as a whole. Teams must
  cherry-pick restriction lints. Power users cherry-pick `pedantic` lints or
  allow them per module.
- `[E04]` Cargo workspace manifests support `[workspace.lints]`. Member crates
  inherit them with `[lints] workspace = true`. Cargo warns on
  `missing_lints_inheritance`.
- `[E05]` `cargo-deny` checks four areas: licenses, bans, advisories, and
  sources.
- `[E06]` `cargo-deny` bans duplicate crate versions with `multiple-versions =
  "deny"`. It enforces workspace dependency usage via
  `[bans.workspace-dependencies]`.
- `[E07]` `cargo-audit` checks `Cargo.lock` against RustSec security advisories
  and yanked releases.
- `[E08]` `cargo-vet` validates supply chain dependencies via shared trusted
  audits. It allows exemption baselines for existing crates.
- `[E09]` `cargo-semver-checks` scans public Rust library APIs for breaking
  semantic version changes against crates.io or git baselines.
- `[E10]` `cargo-public-api` lists and diffs public APIs using rustdoc JSON. It
  supports snapshot assertions in test suites.
- `[E11]` `cargo-mutants` detects untested code by mutating expressions. CI runs
  use `--in-place` to avoid file copying.
- `[E12]` `cargo-mutants --in-diff` tests only mutants in modified lines. A diff
  changing only test code runs zero mutants.
- `[E13]` `cargo-llvm-cov` uses LLVM source-based coverage. It generates LCOV
  and HTML reports. It integrates with `cargo-nextest`.
- `[E14]` `cargo-hack` tests feature combinations with `--feature-powerset` and
  `--each-feature --no-dev-deps`. This detects accidental feature unification
  bugs.
- `[E15]` `cargo-machete` detects unused dependencies fast via text search.
  `cargo-udeps` detects them accurately via compiler analysis on nightly.
  `cargo-shear` cleans unused manifest dependencies.
- `[E16]` Miri detects undefined behavior and memory leaks in unsafe Rust. Miri
  requires nightly. Miri does not run across arbitrary system calls or external
  processes.
- `[E17]` `cargo-fuzz` uses libFuzzer and LLVM sanitizers. Parsers for queries,
  templates, and configurations need fuzz targets.
- `[E18]` CI workflows run short fuzz smoke tests (`-max_total_time=300`) to
  catch regressions.
- `[E19]` Proptest recommends setting `opt-level = 3` for `proptest` and
  `rand_chacha` in test profiles to speed up case generation.
- `[E20]` Proptest persists failure seeds under `proptest-regressions/`. Teams
  must check these files into version control.
- `[E21]` Nextest reads `.config/nextest.toml` to configure profiles, timeouts,
  and retries. Nextest runs process-isolated tests. Nextest does not execute
  doctests.
- `[E22]` RFC 2495 defines `rust-version`. Cargo resolves dependency versions
  respecting the specified MSRV.
- `[E23]` `cargo-modules` validates tree hierarchy, checks acyclic module
  dependencies, and denies orphan files with `cargo modules orphans --deny`.
- `[E24]` `release-plz` automates release PRs, changelog generation with
  git-cliff, semver checks, and crates.io publishing from CI.
- `[E25]` `cargo-dist` builds multi-platform shippable binary releases and
  installers from git tags in GitHub Actions.
- `[E26]` OpenSSF Scorecard scores repository supply chain posture: branch
  protection, pinned actions, security policy, and dependency update tools.
- `[E27]` Cargo requires checking `Cargo.lock` into git for deterministic builds
  and `git bisect` reproducibility in binary and library crates.
- `[E28]` rustc flag `--remap-path-prefix` strips absolute local host paths from
  debug information and diagnostics for reproducible builds.
- `[E29]` Astral uv manages multi-crate workspaces with
  `[workspace.dependencies]`, centralized `[workspace.lints]`, and specialized
  compilation profiles.
- `[E30]` Ripgrep isolates CLI and library crates within a workspace, using
  release LTO and deb/binary packaging metadata.
- `[E31]` Rust-analyzer enforces encapsulation in multi-crate workspaces by
  configuring `unreachable_pub = "warn"`.
- `[E32]` Architectural fitness functions enforce unidirectional dependencies
  between layers using automated tests.
- `[E33]` Dependabot automates Cargo and GitHub Action version updates on
  scheduled intervals with pull request grouping.

## 4. Disagreements

- **Advisory auditing**: Embark `cargo-deny advisories` integrates advisories
  into one configuration file `[E05]`. RustSec `cargo-audit` offers a standalone
  CLI with OSV export `[E07]`. Both use the RustSec database. Running both
  creates duplicate work.
- **Unused dependency detection**: `cargo-machete` is fast but causes false
  positives on macro-only crates `[E15]`. `cargo-udeps` is precise but slow and
  requires nightly `[E15]`. `cargo-shear` modifies manifests directly.
- **MSRV strategy**: Open source libraries often test against minimum stable
  versions `[E22]`. Application codebases frequently pin specific toolchains
  `[E29]`. This repository specifies `rust-version = "1.96"` but develops on
  `nightly-2026-10-01`.
- **Public API regression**: `cargo-public-api` diffs text signatures `[E10]`.
  `cargo-semver-checks` validates semver semantics `[E09]`. Signature diffs
  catch all changes, including non-breaking changes. Semver checks evaluate
  breakage rules.

## 5. Does Not Transfer to This Repo

- **`cargo-vet` organizational registry**: Mozilla and Google use `cargo-vet`
  with dedicated security auditing teams `[E08]`. For a single maintainer, audit
  overhead exceeds value.
- **SLSA Level 3 builder**: Complex container provenance workflows `[E26]` add
  unnecessary maintenance for a pre-1.0 CLI.
- **Miri test suite execution**: Miri checks unsafe pointer arithmetic and
  aliasing `[E16]`. This codebase specifies `#![deny(unsafe_code)]`
  (`Cargo.toml:256`). Miri cannot execute spawned CLI tests.
- **Continuous OSS-Fuzz cluster**: Dedicated cluster fuzzing `[E17]` is
  premature before public distribution.

## 6. Gap Table

| Control | Evidence Tag | Repo Status | File Path Evidence | Cost to Add | Blocks BDD Slice |
|---|---|---|---|---|---|
| Workspace Lints & Deps Table | `[E04]`, `[E29]`, `[E31]` | Absent | `Cargo.toml:11` (single package) | Low | Blocks workspace split |
| Clippy Strict Lint Set | `[E03]` | Present | `Cargo.toml:175-251` | None | None |
| cargo-deny Licenses | `[E05]` | Present | `deny.toml:8-15` | None | Blocks Slice 1 (needs Zlib) |
| cargo-deny Bans Policy | `[E06]` | Partial | `deny.toml:17-28` (missing `multiple-versions = "deny"`) | Low | None |
| cargo-deny Advisories | `[E05]`, `[E07]` | Partial (checked in CI with defaults; no explicit table) | `deny.toml` (no advisories section); `.github/workflows/ci.yml:209-226` runs `cargo-deny` | Low | None |
| cargo-deny Sources | `[E05]` | Partial (checked in CI with defaults; no explicit table) | `deny.toml` (no sources section); `.github/workflows/ci.yml:209-226` | Low | None |
| cargo-audit in CI | `[E07]` | Present | `.github/workflows/ci.yml:209-226` | None | None |
| cargo-semver-checks | `[E09]` | Absent | `.github/workflows/ci.yml` | Low | Blocks library release |
| cargo-public-api Freeze | `[E10]` | Present | `mise.toml:123`, `tests/baselines/` | None | None |
| cargo-mutants in CI | `[E11]`, `[E12]` | Partial | `mise.toml:36` (mutarust in mise, missing in CI) | Medium | None |
| cargo-llvm-cov in CI | `[E13]` | Partial | `mise.toml:416` (mise task exists, missing in CI) | Low | None |
| cargo-hack Powerset | `[E14]` | Absent | `Cargo.toml`, `mise.toml` | Low | None |
| cargo-shear Unused Deps | `[E15]` | Present | `mise.toml:396-405` | None | None |
| Miri Test Gate | `[E16]` | Absent (Justified) | `Cargo.toml:256` (`unsafe_code = "deny"`) | High | None |
| cargo-fuzz Parser Targets | `[E17]`, `[E18]` | Absent | `src/query/`, `src/template/` | Medium | None |
| Proptest Failure Persistence | `[E19]`, `[E20]` | Partial | `Cargo.toml:158` (proptest used, no `proptest-regressions/`) | Low | None |
| Nextest Configuration | `[E21]` | Present | `.config/nextest.toml:1-19` | None | None |
| MSRV Validation in CI | `[E22]` | Present | `.github/workflows/ci.yml:238-259` | None | None |
| cargo-modules Gates | `[E23]` | Partial | `mise.toml:218-351` (tasks exist, not in `check` or CI) | Low | None |
| rustdoc Intra-Doc Links | `[E01]` | Present | `Cargo.toml:259` (`broken_intra_doc_links = "deny"`) | None | None |
| Doctests Enforcement | `[E02]` | Present | `.github/workflows/ci.yml:143-165` | None | None |
| Release Automation | `[E24]`, `[E25]` | Absent | `.github/workflows/` (no release flow) | Medium | None |
| OpenSSF Security Scorecard | `[E26]` | Absent | `.github/workflows/` (no scorecard flow) | Low | None |
| Cargo.lock Committed | `[E27]` | Present | `Cargo.lock` | None | None |
| Path Remapping (Reproducible) | `[E28]` | Absent | `Cargo.toml:272-278` (no `--remap-path-prefix`) | Low | None |
| SECURITY.md Policy | `[E26]` | Absent | Repo root (file does not exist) | Low | None |
| CHANGELOG.md | `[E01]`, `[E24]` | Absent | Repo root (file does not exist) | Low | None |
| Dependency Updates Bot | `[E26]`, `[E33]` | Absent | `.github/dependabot.yml` does not exist | Low | None |
| Layer Architecture Gates | `[E32]` | Present | `mise.toml:148-190` (`check:layers`) | None | Blocks Slice 1 (fix hole) |
| Visibility Architecture Gates | `[E31]` | Present | `mise.toml:123-146` (`check:visibility`) | None | None |

## 7. Recommended Controls Ranked by (Evidence Strength, Cost)

1. **Fix `check:layers` error suppression**: Evidence `[E32]`, Cost: Low. Remove
   `|| true` on `git grep`. Blocks Slice 1.
2. **Add "Zlib" to `deny.toml`**: Evidence `[E05]`, Cost: Low. Prevents license
   check failure in Slice 1.
3. **Configure Dependabot (`.github/dependabot.yml`)**: Evidence `[E26]`,
   `[E33]`, Cost: Low. Automates dependency maintenance with grouped PRs.
4. **Wire `cargo modules orphans --deny` into `mise run check`**: Evidence
   `[E23]`, Cost: Low. Prevents dead unlinked files.
5. **Add `SECURITY.md` and `CHANGELOG.md`**: Evidence `[E01]`, `[E26]`, Cost:
   Low. Fulfills open-source and security baselines.
6. **Add `cargo-semver-checks` to CI**: Evidence `[E09]`, Cost: Low. Protects
   public API contract stability when releasing.
7. **Add `[bans]` and `[sources]` to `deny.toml`**: Evidence `[E05]`, `[E06]`,
   Cost: Low. Denies accidental multiple versions and untrusted registries.
8. **Add `cargo-llvm-cov` to CI**: Evidence `[E13]`, Cost: Low. Ensures test
   suite code coverage does not drop silently.
9. **Add `cargo-fuzz` parser targets**: Evidence `[E17]`, `[E18]`, Cost: Medium.
   Protects query and template parsers against malicious inputs.
10. **Add `release-plz` workflow**: Evidence `[E24]`, Cost: Medium. Automates
    changelog generation, tagging, and package publishing.

## 8. Overkill Controls for This Repo's Size

- **`cargo-vet` audit tracking**: Requires maintaining manual cryptographic
  audit proofs `[E08]`. For a single developer, `cargo-audit` and `cargo-deny`
  are sufficient.
- **SLSA Level 3 builder workflows**: Requires multi-stage isolated build
  runners `[E26]`. Unnecessary overhead for a pre-1.0 CLI.
- **Full mutation testing on all PRs**: Running full mutation test suites takes
  hours `[E11]`. Running only on diffs (`--in-diff`) or scheduled runs is
  appropriate.
- **Miri in CI**: Miri cannot execute child processes in E2E tests `[E16]`. The
  repository already denies unsafe code (`Cargo.toml:256`).

## 9. Conflicts: Current Controls vs Best Practice

1. **`clippy.toml` catch_unwind ban vs `rstest-bdd`**:
   - `clippy.toml:97` bans `std::panic::catch_unwind`.
   - `rstest-bdd` generated step code calls `catch_unwind` `[E03]`.
   - Ticket 06 resolves this by adding `#[expect(clippy::disallowed_methods)]`
     on `mod bdd`.
2. **`multiple_crate_versions = "allow"` vs duplicate prevention**:
   - `Cargo.toml:240` allows `multiple_crate_versions` in Clippy.
   - `deny.toml` omits `multiple-versions = "deny"` `[E06]`.
   - This allows duplicate dependency versions to accumulate unnoticed.
3. **`deny.toml` licenses check vs `rstest-bdd` dependencies**:
   - `deny.toml:8-15` allows MIT, Apache-2.0, ISC, Unicode-3.0, CC0-1.0.
   - Adopting `rstest-bdd` pulls in Zlib-licensed crates.
   - `deny.toml` will fail until `"Zlib"` is explicitly added.
4. **Nightly toolchain vs stable MSRV**:
   - `rust-toolchain.toml:10` pins `nightly-2026-10-01`.
   - `Cargo.toml:15` sets `rust-version = "1.96"`.
   - Developing on nightly masks APIs unavailable on stable 1.96 `[E22]`.
5. **`check:layers` error mask**:
   - `mise.toml:168, 175, 182` appends `|| true` to `git grep`.
   - This masks script errors and ignores untracked files `[E32]`.

## 10. Implications for Ticket 14

- **Option A (Pre-flight hardening before Slice 1)**:
  - Add `"Zlib"` to `deny.toml` `[E05]`.
  - Fix `check:layers` `|| true` hole `[E32]`.
  - Add `#[expect(clippy::disallowed_methods)]` to `tests/e2e/bdd/mod.rs`
    `[E03]`.
  - Evidence: Ticket 06, Ticket 09, Ticket 13.
- **Option B (Quality tooling expansion in Slice 1 CI)**:
  - Add `cargo-llvm-cov` step to `.github/workflows/ci.yml` `[E13]`.
  - Add `cargo modules orphans --deny` to `mise run check` `[E23]`.
  - Evidence: `[E13]`, `[E23]`.
- **Option C (Defer supply-chain and release automation to workspace split)**:
  - Place `cargo-semver-checks`, `release-plz`, and `[workspace.lints]` into the
    post-BDD workspace split milestone `[E04]`, `[E09]`, `[E24]`.
  - Evidence: `[E04]`, `[E09]`, `[E24]`.

## 11. Open Gaps

1. Windows and Linux CI behavior for `tests/e2e.rs` with `rstest-bdd` remains
   unverified until Slice 1 executes on CI runners.
2. Property test regression seeds (`proptest-regressions/`) have no committed
   examples yet; directory tracking policy needs confirmation.
3. Feature flag matrix (`cargo-hack`) is not tested because the repository
   currently has only one feature (`test-utils`). Multi-crate workspace split
   will require feature matrix checks.

# Research 14: Cross-Platform CI and Flake Prevention

This document supports [ticket 14](../issues/14-adoption-roadmap.md). It gathers
external evidence on cross-platform test reliability, runner configuration,
operating system pitfalls, and flake prevention for Rust workspaces using
`cargo-nextest`.

## 1. Question

What are the best practices to get and keep a reliable green test run on Linux,
macOS, and Windows for a Rust workspace using `cargo-nextest`, and how do teams
prevent and manage flaky tests?

## 2. Method and Source List

Date of research: 2026-11-03. The session opened and read 27 primary and
authoritative secondary sources.

Sources tried and found useless:

- `wiki.jenkins-ci.org/Flaky+Test+Handler+Plugin` (retired page);
- `crates.io/crates/junction-verbatim` (deprecated stub);
- `blog.testproject.io` (offline domain).

### Source Catalog

- `[C1]` Nextest Configuration: `https://nexte.st/docs/configuration/`
  (2026-11-03, Primary, High)
- `[C2]` Nextest Retries: `https://nexte.st/docs/features/retries/` (2026-11-03,
  Primary, High)
- `[C3]` Nextest Flaky Results:
  `https://nexte.st/docs/features/retries/#failing-flaky-tests` (2026-11-03,
  Primary, High)
- `[C4]` Nextest Test Groups: `https://nexte.st/docs/configuration/test-groups/`
  (2026-11-03, Primary, High)
- `[C5]` Nextest Threads Required:
  `https://nexte.st/docs/configuration/threads-required/` (2026-11-03, Primary,
  High)
- `[C6]` Nextest JUnit XML: `https://nexte.st/docs/machine-readable/junit/`
  (2026-11-03, Primary, High)
- `[C7]` Nextest Partitioning: `https://nexte.st/docs/ci-features/partitioning/`
  (2026-11-03, Primary, High)
- `[C8]` Nextest Archiving: `https://nexte.st/docs/ci-features/archiving/`
  (2026-11-03, Primary, High)
- `[C9]` Nextest Environment Variables:
  `https://nexte.st/docs/configuration/env-vars/` (2026-11-03, Primary, High)
- `[C10]` GitHub Actions Runner Images:
  `https://raw.githubusercontent.com/actions/runner-images/main/README.md`
  (2026-11-03, Primary, High)
- `[C11]` Swatinem/rust-cache:
  `https://raw.githubusercontent.com/Swatinem/rust-cache/master/README.md`
  (2026-11-03, Primary, High)
- `[C12]` taiki-e/install-action:
  `https://raw.githubusercontent.com/taiki-e/install-action/main/README.md`
  (2026-11-03, Primary, High)
- `[C13]` zizmor Audit Rules: `https://docs.zizmor.sh/audits/` (2026-11-03,
  Primary, High)
- `[C14]` OpenSSF Scorecard Pinned Dependencies:
  `https://raw.githubusercontent.com/ossf/scorecard/main/checks/pinned_dependencies.go`
  (2026-11-03, Primary, High)
- `[C15]` cargo-hack:
  `https://raw.githubusercontent.com/taiki-e/cargo-hack/main/README.md`
  (2026-11-03, Primary, High)
- `[C16]` cargo-minimal-versions:
  `https://raw.githubusercontent.com/taiki-e/cargo-minimal-versions/main/README.md`
  (2026-11-03, Primary, High)
- `[C17]` Rust std::fs::canonicalize:
  `https://doc.rust-lang.org/stable/std/fs/fn.canonicalize.html` (2026-11-03,
  Primary, High)
- `[C18]` Dunce Crate: `https://docs.rs/dunce/latest/dunce/index.html`
  (2026-11-03, Primary, High)
- `[C19]` Win32 CreateFileW:
  `https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew`
  (2026-11-03, Primary, High)
- `[C20]` Win32 Symbolic Links:
  `https://learn.microsoft.com/en-us/windows/win32/fileio/creating-symbolic-links`
  (2026-11-03, Primary, High)
- `[C21]` Win32 CreateSymbolicLinkW:
  `https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createsymboliclinkw`
  (2026-11-03, Primary, High)
- `[C22]` Rust tempfile::TempDir:
  `https://docs.rs/tempfile/latest/tempfile/struct.TempDir.html` (2026-11-03,
  Primary, High)
- `[C23]` Git Racy Git Documentation: `https://git-scm.com/docs/racy-git`
  (2026-11-03, Primary, High)
- `[C24]` Martin Fowler, Eradicating Non-Determinism in Tests:
  `https://martinfowler.com/articles/nonDeterminism.html` (2026-11-03, Primary,
  High)
- `[C25]` John Micco, Google Testing Blog, Flaky Tests at Google:
  `https://googledata.org/google-testing/flaky-tests-at-google-and-how-we-mitigate-them/`
  (2026-11-03, Primary, High)
- `[C26]` Software Engineering at Google, Chapter 11 (Testing Overview):
  `https://abseil.io/resources/swe-book/html/ch11.html` (2026-11-03, Primary,
  High)
- `[C27]` Software Engineering at Google, Chapter 14 (Larger Tests):
  `https://abseil.io/resources/swe-book/html/ch14.html` (2026-11-03, Primary,
  High)

## 3. Findings

### 3.1 cargo-nextest Execution Controls

- Nextest executes each test in a dedicated process `[C1]`. This design isolates
  crashes and environment mutations `[C1]`.
- Retries rerun failed tests up to $N$ times `[C2]`. By default, nextest treats
  a test that passes on retry as successful `[C2]`.
- Nextest supports `flaky-result = "fail"` `[C3]`. This setting runs configured
  retries to collect diagnostics, but fails the suite if any attempt failed
  `[C3]`.
- Test groups define named concurrency limits with `max-threads` `[C4]`. Groups
  act as semaphores for tests accessing shared resources `[C4]`.
- Nextest provides `threads-required = N` for heavy tests `[C5]`. This option
  allocates multiple execution permits to one test `[C5]`.
- Nextest supports JUnit XML export via `[profile.<name>.junit]` `[C6]`. The
  report captures test duration, stdout, stderr, and retry failures `[C6]`.
- Partitioning divides test execution across runners using `--partition
  slice:m/n` or `--partition hash:m/n` `[C7]`. Hashed sharding provides stable
  buckets `[C7]`.
- Nextest supports binary archiving via `cargo nextest archive` `[C8]`. The
  target machine runs tests with `cargo nextest run --archive-file` without
  `cargo` `[C8]`.
- Nextest injects runtime environment variables: `NEXTEST_RUN_ID`,
  `NEXTEST_ATTEMPT`, `NEXTEST_TEST_GROUP`, and `NEXTEST_BIN_EXE_<name>` `[C9]`.
- Nextest provides `--no-fail-fast` to run all tests despite earlier failures
  `[C1]`.
- Nextest terminates hanging tests via `slow-timeout = { period = "...",
  terminate-after = N }` and bounds run duration via `global-timeout` `[C1]`.

### 3.2 CI Runner Images, Caching, and Supply-Chain Hardening

- Standard GitHub Actions runner images differ by CPU architecture `[C10]`.
  `ubuntu-latest` and `windows-latest` run on x86_64 `[C10]`. `macos-latest`
  runs on arm64 (Apple Silicon) `[C10]`.
- `Swatinem/rust-cache` automatically sets `CARGO_INCREMENTAL=0` `[C11]`. It
  keys caches by toolchain hash, lockfile hash, and environment variables
  `[C11]`.
- `Swatinem/rust-cache` purges build artifacts with an `mtime` older than seven
  days before saving `[C11]`. It works around macOS cache timestamp bugs
  `[C11]`.
- `taiki-e/install-action` installs pre-built binary tools directly from GitHub
  Releases `[C12]`. It validates SHA256 checksums and artifact attestations
  `[C12]`.
- `zizmor` and OpenSSF Scorecard enforce commit SHA pinning for third-party
  actions `[C13, C14]`. Mutable tags (such as `@v2` or `@main`) permit upstream
  tampering `[C13, C14]`.
- Workflows must declare minimal permissions (`permissions: contents: read`) and
  set `persist-credentials: false` on `actions/checkout` `[C13]`.

### 3.3 Matrix Design and Build Verification

- Matrix jobs must set `fail-fast: false` `[C10]`. This setting ensures all
  platforms run to completion so engineers observe complete failure sets
  `[C10]`.
- MSRV jobs verify that dependencies and language features compile against the
  minimum supported Rust version `[C15]`.
- `cargo-hack --each-feature` verifies that all feature combinations compile
  `[C15]`. This catches missing imports in optional dependencies `[C15]`.
- `cargo-minimal-versions check --workspace` resolves dependencies to their
  lowest declared versions `[C16]`. This catches undeclared transitive
  requirements `[C16]`.

### 3.4 Cross-Platform Operating System Pitfalls

| Operating System | Pitfall / Behavior | Root Cause and Impact | Mitigation | Source |
|---|---|---|---|---|
| Windows | Path separators | Code assumes `/`. Path display and joining use `\`. | Use `Path::join` and standard path abstractions. | `[C17]` |
| Windows | Line endings | Git checkout or file writes use `\r\n`. Assertions expect `\n`. | Normalize `\r\n` to `\n` in text assertions. | `[C17]` |
| Windows | Extended length `\\?\` prefix | `std::fs::canonicalize` prepends `\\?\` UNC syntax. External tools fail. | Use `dunce::canonicalize` or strip prefix. | `[C17, C18]` |
| Windows | File locking (`ERROR_SHARING_VIOLATION`) | Open file handles block delete or overwrite operations. | Close files before deletion. Use `TempDir::close`. | `[C19, C22]` |
| Windows | Symlink privilege | `CreateSymbolicLinkW` requires Developer Mode or Admin rights. | Avoid symlinks in tests; copy files or use junctions. | `[C20, C21]` |
| Windows | ConPTY vs Headless | No pseudo-console exists in CI. `is_terminal()` returns false. | Pass explicit `--color` and headless test flags. | `[C9, C19]` |
| Windows | Exit code semantics | Unhandled aborts exit with 32-bit NTSTATUS (e.g. `0xC000013A`). | Check exit code ranges, not single POSIX bytes. | `[C19]` |
| Windows | Antivirus interference | Defender locks newly written executables during scans. | Retry transient disk access; exclude target in CI. | `[C19]` |
| macOS | Case-insensitive APFS | APFS preserves case but ignores case distinctions. | Enforce lower-case file naming in layer linters. | `[C10]` |
| macOS | `/private/var` symlink | `/var` resolves to `/private/var`. Path comparisons fail. | Canonicalize both expected and actual test paths. | `[C17, C22]` |
| macOS | CPU Architecture | `macos-latest` runs on arm64; Linux/Windows run on x86_64. | Do not assume x86 memory models or pointer sizes. | `[C10]` |
| Linux | Case-sensitive ext4 | Filenames differing only in case exist as separate files. | Keep cross-platform filenames strictly distinct. | `[C10]` |
| Linux | Resource limits | Container runners restrict open file descriptors (`ulimit -n`). | Configure nextest thread limits or raise limits. | `[C4, C5]` |

### 3.5 Filesystem Timestamps, Racy Timestamps, and Determinism

- Fast file writes can occur within the same filesystem timestamp tick `[C23]`.
  Filesystem caching assumes identical mtime means unmodified content `[C23]`.
- This condition ("racy git") causes build tools and file watchers to miss edits
  `[C23]`.
- Tests modifying files to trigger re-indexing must advance file timestamps
  explicitly or use content hashes `[C23]`.
- Unset timezones cause timestamp parsing tests to fail on local machines
  `[C24]`. CI must set `TZ=UTC` `[C24]`.
- Locale differences alter string sorting and numeric formatting `[C24]`.
  Workflows must set `LC_ALL=C` or `LANG=en_US.UTF-8` `[C24]`.
- Tests must never rely on wall-clock time `SystemTime::now()` for assertions
  `[C24, C26]`. Tests must use frozen virtual clocks or event synchronization
  `[C26, C27]`.

### 3.6 Flaky Test Management, Quarantine, and Retry Policies

- Flaky tests produce both passing and failing results on identical code
  `[C25]`.
- Google measured that 1.5% of all test executions report a flaky result
  `[C25]`. Almost 16% of Google's test base exhibits some level of flakiness
  `[C25]`.
- 84% of observed pass-to-fail transitions in CI involve a flaky test `[C25]`.
- Automatic retries hide real concurrency and architectural defects `[C24,
  C25]`. Retries multiply CI execution time and delay true breakage discovery
  `[C25]`.
- Fowler identifies two primary root causes: lack of test isolation and
  asynchronous races `[C24]`. Arbitrary `sleep()` statements cause high
  flakiness `[C24, C27]`.
- Quarantine policy: Non-deterministic tests must be isolated immediately into a
  quarantine suite `[C24, C25]`. Quarantined tests must not block CI `[C24,
  C25]`.
- Teams must enforce a quarantine budget (e.g. maximum 5 tests or 7-day limit)
  to prevent test accumulation `[C24]`.
- Event-driven polling with bounded timeouts beats static sleeps `[C27]`.
- Nextest provides slow-test monitoring: `slow-timeout = { period = "...",
  terminate-after = N }` `[C1]`. This kills hung processes before exhausting CI
  runner deadlines `[C1]`.

### 3.7 Repository Audit and Gap List

An audit of this repository (`traces`) revealed the following state:

1. **GitHub Actions Workflow (`.github/workflows/ci.yml`)**:
   - `check` and `test-e2e` run on a 3-OS matrix `[ubuntu-latest, macos-latest,
     windows-latest]`.
   - `test-unit`, `test-integration`, `test-doctests`, `clippy`, `visibility`,
     and `hygiene` run only on `ubuntu-latest`.
   - Actions use unpinned mutable tags: `actions/checkout@v7`,
     `jdx/mise-action@v4`, `Swatinem/rust-cache@v2`. This violates zizmor and
     Scorecard pinning rules `[C13, C14]`.
   - The workflow lacks a `cargo-hack --each-feature` powerset check `[C15]`.
   - The workflow lacks a `cargo-minimal-versions` check `[C16]`.
   - The workflow does not set `TZ: UTC` or `LC_ALL: C` globally.

2. **Nextest Configuration (`.config/nextest.toml`)**:
   - Contains only `[profile.default]` with `slow-timeout = { period = "30s",
     terminate-after = 2 }` and `global-timeout = "20m"`.
   - Lacks a dedicated `[profile.ci]` profile.
   - Lacks retry configuration (`retries = { backoff = "exponential", ... }`).
   - Lacks `flaky-result = "fail"` or quarantine filterset rules.
   - Lacks JUnit XML export configuration (`[profile.ci.junit]`).
   - Lacks `test-groups` definitions for mutual exclusion.

3. **Local Mise Tasks (`.mise/tasks/test/`)**:
   - Tasks are written as bash scripts. Bash scripts do not execute natively on
     Windows cmd/powershell unless Git Bash or MSYS2 is on the PATH.
   - Phase tasks invoke raw nextest commands without exporting JUnit or CI
     flags.
   - Doctests run via `cargo test --doc --all-features`, bypassing nextest.

### 3.8 Checklist of Evidence-Backed CI Controls

- [ ] **[Control 1]** Matrix test execution with `fail-fast: false` across
  `ubuntu-latest`, `macos-latest`, and `windows-latest` `[C10]`.
- [ ] **[Control 2]** SHA-pinned GitHub Actions with `persist-credentials:
  false` and explicit `permissions: contents: read` `[C13, C14]`.
- [ ] **[Control 3]** Nextest CI profile with JUnit XML output enabled for build
  analytics `[C6]`.
- [ ] **[Control 4]** Bounded test execution timeouts (`slow-timeout` and
  `global-timeout`) configured in `.config/nextest.toml` `[C1]`.
- [ ] **[Control 5]** Strict flake detection: configure `retries` with
  `flaky-result = "fail"` on CI to capture failure logs without hiding flakes
  `[C2, C3]`.
- [ ] **[Control 6]** Explicit quarantine workflow or filterset (`not
  group(quarantine)`) with a 7-day or 5-test ceiling `[C24, C25]`.
- [ ] **[Control 7]** Deterministic test environment variables: `TZ: UTC`,
  `LC_ALL: C`, `CARGO_TERM_COLOR: always` `[C9, C24]`.
- [ ] **[Control 8]** Windows path safety: normalize Windows paths with
  `dunce::canonicalize` to eliminate `\\?\` prefix incompatibilities `[C17,
  C18]`.
- [ ] **[Control 9]** Windows handle safety: explicitly close files and call
  `TempDir::close` to prevent file locking sharing violations `[C19, C22]`.
- [ ] **[Control 10]** MSRV and feature powerset verification via `cargo-hack`
  and `cargo-minimal-versions` `[C15, C16]`.

## 4. Disagreements

1. **Automatic Retries vs Fail-on-Flaky**:
   - *Pragmatic CI view (Jenkins plugin, pytest-rerunfailures)*: Retries keep
     deployment pipelines flowing and reduce noise for developers.
   - *Rigorous architecture view (Fowler, Google)*: Retries hide real race
     conditions and multiply resource costs. A test passing on retry remains
     broken.
   - *Evidence weight*: Measured data from Google shows 84% of transition
     failures involve flakiness `[C25]`. The better evidenced solution is
     nextest's `flaky-result = "fail"`: run retries to capture stack traces, but
     fail the build `[C3]`.

2. **Action Pinning by Tag vs SHA**:
   - *Author view (Action maintainers)*: Major tags (e.g. `@v2`) automatically
     provide security and bug patches.
   - *Supply-chain security view (zizmor, OpenSSF Scorecard)*: Tags are mutable
     pointers. Upstream account compromise allows immediate code injection.
   - *Evidence weight*: OpenSSF Scorecard awards zero points to unpinned
     dependencies `[C14]`. SHA pinning with automated Dependabot updates
     provides the strongest security `[C13, C14]`.

3. **Sliced vs Hashed Test Partitioning**:
   - *Sliced partitioning*: Even test distribution across buckets, but changing
     test counts shuffles tests across buckets between runs `[C7]`.
   - *Hashed sharding*: Deterministic bucket assignment, but small test suites
     suffer uneven bucket sizes `[C7]`.
   - *Evidence weight*: Nextest documentation explicitly recommends `hash:` for
     large stable suites and `slice:` when exact bucket balancing matters
     `[C7]`.

4. **Anecdote vs Measured Data**:
   - Practitioner claims regarding flake rates ("10% of CI runs fail") are
     anecdotes.
   - Google's metrics (1.5% run flake rate, 16% flaky test rate, 84% transition
     rate) are empirical measurements over millions of test executions `[C25]`.

## 5. Does Not Transfer to This Repo

1. **Distributed Test Partitioning**:
   - traces compiles and runs its test suite in under two minutes locally.
   - Multi-machine sharding (`--partition slice:1/10`) adds network and runner
     overhead that exceeds any execution gain for this single-crate repository.
2. **Build-Run Archive Decoupling**:
   - Creating tar.zst build archives on one VM to run on GPU or separate test
     nodes `[C8]` is unnecessary when GitHub-hosted runners can compile and test
     directly in the same job.
3. **Complex Automated Quarantine Daemons**:
   - Google's automatic bug-filing and quarantine bots require massive
     infrastructure.
   - For a solo/small team, a simple nextest filterset or `@quarantine` tag in
     the repository satisfies quarantine needs without custom bots.
4. **Pseudo-Terminal Emulation (ConPTY)**:
   - Unless traces tests interactive terminal escape controls or curses
     interfaces, spawning full Windows ConPTY instances adds unnecessary
     complexity. Standard headless process pipes suffice.

## 6. Implications for Ticket 14

These are options for ticket 14 and `spec.md`, not final decisions:

- **Option A (CI Profile and JUnit Artifacts)**: Add `[profile.ci]` in
  `.config/nextest.toml` with `junit.path = "junit.xml"` and `flaky-result =
  "fail"`. Upload test reports as GitHub Actions artifacts. Tagged evidence:
  `[C3, C6]`.
- **Option B (Three-OS Matrix Expansion)**: Expand `test-unit` and
  `test-integration` from `ubuntu-latest` only to the three-OS matrix
  `[ubuntu-latest, macos-latest, windows-latest]`, matching `test-e2e`. Tagged
  evidence: `[C10, C17, C19]`.
- **Option C (Action Pinning and Security Hardening)**: Pin all GitHub Actions
  in `.github/workflows/ci.yml` to full 40-character commit SHAs with inline
  version comments, and run `zizmor` in the hygiene job. Tagged evidence: `[C13,
  C14]`.
- **Option D (Windows Path and Handle Sanitation)**: Adopt `dunce::canonicalize`
  across test helpers to avoid `\\?\` path assertion failures on Windows, and
  require explicit `TempDir::close` calls. Tagged evidence: `[C17, C18, C22]`.
- **Option E (Determinism Environment Invariant)**: Add `TZ=UTC` and `LC_ALL=C`
  to top-level workflow environment variables and `mise.toml`. Tagged evidence:
  `[C9, C24]`.
- **Option F (Quarantine Policy Enforcement)**: Define a formal quarantine rule
  in ticket 14: flaky tests are tagged `#[ignore = "quarantine: <issue>"]`, with
  a hard limit of 5 quarantined tests permitted across the workspace. Tagged
  evidence: `[C24, C25]`.

## 7. Open Gaps

1. **Windows E2E Verification**:
   - Ticket 09 noted that Linux and Windows CI for the e2e scenario placement
     remain unverified.
   - Spawning `traces` on Windows runners needs verification for path formatting
     and exit code handling.
2. **`check:layers` Gate Reliability**:
   - Ticket 09 noted `check:layers` contains a `git grep || true` hole and
     misses untracked files.
   - This gate must be patched before slice 1 adoption.
3. **Windows Symlink Fixtures**:
   - If future BDD scenarios touch symlinks or git repository fixtures with
     symlinks, unprivileged Windows runners will fail without Developer Mode.

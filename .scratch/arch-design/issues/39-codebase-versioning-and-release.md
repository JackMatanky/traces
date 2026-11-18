# How does Traces version and release the codebase?

Type: decision
Status: open
Blocked by: 16, 22, 25, 32

## Question

Define how Traces bumps its version and cuts a release.

The project has no release process today. The version is `0.1.0`. The repository
has no release tags.

[Does the landed index freshness model
stand?](./25-index-freshness-and-invalidation.md) does not depend on this
ticket. The store identity uses an epoch and fingerprints, not the binary
version. Notices use the binary version as a label.

Tickets 25, 28, 29 and 32 use the words "pre-release" and "before release".
They treat a wipe or an epoch bump as free until a release. This ticket must
define that boundary.

Answer each numbered question separately.
Use [ticket 36's decision-record
rules](./36-reconcile-decisions-and-audit-coverage.md#decision-records-and-evidence-gates).
Keep decision evidence separate from implementation gates.

1. Define when "pre-release" stops.
   Choose the first release tag, the first `cargo publish`, or no boundary.
   The user is the only consumer.
2. Choose where the version lives. `Cargo.toml` is the proposed single source.
   For a workspace, that source is `workspace.package.version`.
3. Define when a version bump is needed.
   Use ticket 16's specification change table, or choose another trigger.
4. Define how a release checks that its version is newer than the last release
   version. Use a `mise` task. Name the task.
   The name `release` is taken (see Facts).
5. Define the release tag pattern and the first-release rule.
   Ignore unrelated tags such as `merge-safety/*`.
6. Define what the check does when tag history is absent or incomplete.
7. Define what the release check says about the format epoch.
   State if the check reports a change since the last release tag.
8. Decide if the project uses `git-cliff` for the changelog.
   The `hk` commit hook enforces Conventional Commits on a local machine.
   CI does not.
9. Decide how a developer build names itself between releases.
   `build.rs` can add `git describe` to `--version` only.
   This changes the exact `--version` output that an e2e test asserts.
10. Decide which string the store `META` field "written-by version" holds.
    Ticket 25 says "the binary version".
    Choose the plain `Cargo.toml` version or the `git describe` string for
    developer builds.
11. Define the install instructions. They must say `cargo install --locked`.
    Choose the channel (`--git`, `--path` or crates.io).
    Name the file that holds the install line, because `README.md` does not
    exist.
12. Decide whether `cargo-semver-checks` serves a real consumer.
    The library surface is Free under ticket 16.
    `check:visibility` detects changes but does not prove semantic
    compatibility.

Three points depend on the open tickets 35 and 33. They are the version source,
the package name in the install line and the place of `build.rs`.
Ticket 35 decides one crate or a workspace. State a layout-independent rule
here, then let ticket 35 supply the package paths.
Alternatively, wait for ticket 35 before choosing those paths.

Facts, all checked on `cf6d79bd` unless marked:

- [V] `Cargo.toml` has `version = "0.1.0"` under `[package]`. It has no
  `[workspace]` table. Its header comment says "Workspace manifest". The comment
  is wrong today.
- [V] `git tag` lists two tags: `merge-safety/cargo-modules-mise-before-merge`
  and `merge-safety/main-before-cargo-modules-mise`. Neither tag marks a
  release.
- [V] The repository has no `CHANGELOG*`, no `build.rs`, no `cliff.toml`, no
  `README.md` and no `LICENSE`. `Cargo.toml` declares `readme = "README.md"` and
  `documentation = "https://docs.rs/traces"`.
- The package check remains incomplete. The declared `README.md` is absent.
- Crates.io name availability remains unchecked. Do not assume that `traces` is
  available.
- [V] `.github/workflows/` holds `bench.yml`, `ci.yml`, `fuzz.yml`,
  `mutation.yml` and `soak.yml`. No workflow reacts to a tag or builds a
  release.
- [V] `mise.toml` already defines the task `release` (alias `r`). It runs `cargo
  build --release`. A new release-check task needs another name.
- [V] `mise.toml` and `mise.lock` do not list `git-cliff`, `cargo-release`,
  `release-plz` or `cargo-semver-checks`. CI installs tools with `mise install
  --locked`. A new tool needs an entry in both files.
- [V] `mise run check:visibility` bans glob re-exports. It diffs `cargo
  public-api` output against `tests/baselines/public-api-default.txt` and
  `tests/baselines/public-api-test-utils.txt`. It diffs the `pub` tokens against
  `tests/baselines/pub-tokens.txt`. It runs in `mise run check`, in the `hk`
  step `visibility` and in the CI job `visibility`.
- [V] `check:visibility` finds a change. It does not tell a breaking change from
  an addition. Ticket 16 says the baselines are "change detectors, not a
  contract". Ticket 35 may replace them.
- [V] The `hk` `commit-msg` step `conventional-commit` runs `hk util
  check-conventional-commit`. The `postinstall` hook in `mise.toml` installs it
  with `hk install --mise`. The hook runs only on a machine that installed it.
  No CI step checks commit messages.
- [V] 46 of 2,620 non-merge commits do not match a Conventional Commits type.
  Many are `Revert` subjects or long prose subjects. A changelog tool must
  handle them.
- [V] `src/cli/mod.rs` sets `#[command(name = "traces", version, ...)]` on
  `Cli`. `traces --version` prints `traces 0.1.0` from `CARGO_PKG_VERSION`.
- [V] `tests/e2e/process_contract.rs` (`exit_code_zero_for_help_and_version`)
  asserts the exact `--version` stdout `traces <CARGO_PKG_VERSION>` and an empty
  stderr. `tests/e2e/help.rs` checks the `-V, --version` option signature.
- [V] `src/` and `benches/` have no other use of `CARGO_PKG_VERSION`.
- [V] `Cargo.lock` is tracked in git. `[profile.release]` sets `opt-level =
  "z"`, `strip = true`, `lto = true`, `panic = "abort"` and `codegen-units = 1`.
- [V] The current store marker is `META_VERSION = b"1"` in `src/index/store.rs`.
  It does not use the binary version. Ticket 25 removes it.
- [H] A `cargo install` without `--locked` ignores the packaged `Cargo.lock` and
  resolves the dependencies again. The planned `build.rs` of ticket 25 may then
  read the resolved lock. `--locked` then controls the tested dependency set. It
  does not decide if the fingerprint is correct. No one tested this.

Alternatives:

- Keep as is. Keep `0.1.0`, add no tag, add no check. Ticket 25 does not need a
  release process.
- Tag only. Bump `Cargo.toml` by hand, tag, and install with `cargo install
  --locked --git <repository>`. Add no release tool.
- Tag plus a `mise` release check. The check compares release versions. Evaluate
  `git-cliff` separately from the version check.
- Full tooling. Use `cargo-release` or `release-plz`, `cargo-semver-checks` and
  a release workflow. The library surface is Free (ticket 16), so this fits a
  sole consumer poorly.
- For the version label, compare three options. Use `CARGO_PKG_VERSION` only.
  Use `build.rs` with `git describe`. Or set a label from an environment
  variable in the `mise` build task, with no `build.rs`.

Include "keep as is" in the comparison for every bullet.

## Inputs from the freshness decision

See [ticket 25's Answer](./25-index-freshness-and-invalidation.md).

- Ticket 25 hands this ticket "the release process" and "the `--locked` install
  rule". The Answer says this ticket no longer gates ticket 25. The maintainer
  asked for this ticket. A reviewer called it dead weight [V: "Audit of all
  tickets"].
- Ticket 25 lists the engine tag, platform family, format epoch and two
  fingerprints. Ticket 32 supersedes the platform-family rule. Keep the
  canonical root and remove the platform family.
- The "written-by version" is the binary version. Notices use it. No identity
  comparison uses it [V].
- The format epoch is an ordered `u32`. Only `mise run index:bless` changes it.
  The task is planned, not in the repository. It rewrites a committed identity
  file and adds 1 to the epoch [V]. A version bump never changes the epoch.
- The change table for ticket 16 says: a store from an older epoch rebuilds once
  on upgrade. A patch release that changes no extractor output does not rebuild
  [V]. The release rule must keep this true.
- "Why the version floor is gone" gives four reasons. The repository has no
  version tags. For `0.x`, the minor version is the breaking one. Developer
  builds and `cargo install --git` never change the version. The crate list in
  the fingerprint covers the gap [V]. This ticket must not bring a version floor
  back.
- The "Known gaps" list says a `cargo install` without `--locked` can resolve
  other crate versions than the packaged `Cargo.lock`. This ticket owns the
  `--locked` rule [V].
- Handoff to ticket 35: `build.rs` reads `Cargo.lock` to hash six crate
  versions. After a workspace split the lock file is at the workspace root. A
  second `build.rs` for `git describe` adds a second build script, or one script
  does both jobs [V].
- The identity file and the `index:bless` task are release-adjacent. Decide if
  the release check also runs the identity test [P].

## Inputs from the freshness research

See [research 25, section H](../research/25-index-freshness-prep.md).

- Rejected: a `build.rs` git hash inside the store identity. A changed hash
  invalidates the store after every commit [V]. This does not reject a developer
  label in `--version`. Measure any build-script recompilation cost separately
  [H].
- Verdict "Major or minor fails for `0.x`, and no release process exists":
  accepted [V]. Two tags exist. Both are `merge-safety/*`.
- Row "Cancel ticket 26. Close ticket 39": a judgment call for the maintainer
  [V]. The ticket stays open on the maintainer's request.

## Inputs from the compatibility envelope

See [ticket 16's Answer](./16-grilling-compatibility-envelope.md).

- The user is the only consumer. There is no published release. No external Rust
  crate depends on the library [V].
- The Rust library surface is Free [V]. A semver promise for the library has no
  consumer. A version bump can only tell the user about the binary.
- Ticket 16 freezes exit codes, the stdout and stderr split, and the named
  payload shapes [V]. It does not name the `--version` string as Frozen. An e2e
  test asserts that string today. Classify a proposed label change before
  changing the test [P]. A test assertion alone does not set its compatibility
  tier.
- The redesign specification has its own "behaviour changes" table. An
  observable change that is absent from it is a regression [V]. This table can
  decide when a bump is needed [P].
- `index.redb` layout and versioning are Free [V].

## Inputs from the config decision

See [ticket 22's Answer](./22-config-ownership-and-lifecycle.md).

- Keep `deny_unknown_fields`. Add no `version` key to the config until the first
  breaking change [V]. This ticket must define "breaking change" for the config
  file, or state that the release rule does not.

## Inputs from the pre-release statements

See [ticket 28](./28-live-buffer-and-persisted-index-consistency.md), [ticket
29](./29-query-core-and-plan.md) and [ticket
32](./32-path-and-capability-model.md).

- Tickets 28 and 29: "The project is pre-release. The `.redb` layout is Free"
  [V].
- Ticket 32: "An epoch bump before release is free". Ticket 32 drops the escape
  for U+FDD0 on this basis [V].
- The first point of the Question settles when these statements stop being true.
  After that point, an epoch bump rebuilds the store for every user.

## Inputs from the crate topology and host decisions

See [the map's user steer on workspace layout](../map.md), [ticket
35](./35-crate-topology-and-public-surface.md) and [ticket
33](./33-workspace-host-and-composition.md). Both tickets are open.

- The steer asks to evaluate a workspace that starts with a separate CLI crate.
  This is a direction to evaluate, not a decision [V].
- [P] In a workspace, select the CLI package for `cargo install --locked`. Use
  `workspace.package.version` as one version-source option. The CLI crate
  supplies `--version`.
- A `build.rs` in the CLI crate recompiles less code on each commit than one in
  a 94k-line crate [H]. No one measured this.
- Ticket 35 may replace the `check:visibility` baselines. Evaluate semver
  tooling against actual consumers, not those baselines.

## Inputs from the specification hand-off

See [ticket 36](./36-reconcile-decisions-and-audit-coverage.md).

- Ticket 36 orders the "Code delta" sections into the refactor sequence. Its
  `Blocked by:` line does not list this ticket [V]. This ticket changes
  `mise.toml`, `Cargo.toml` and maybe `build.rs`. The specification must list
  those changes as a delta, or state that the release process is outside the
  specification.
- Ticket 25 lists "Not in code: the release process and the `--locked` install
  rule (ticket 39)" [V]. The Answer must state which file and which task carry
  them.

## Inputs from the findings ledger and the ADRs

See [the findings ledger](../findings-ledger.md) and `docs/adr/`.

- The ledger holds tactical findings outside the map's destination. A release
  process is a tactical item for the core design. It stays in the map only for
  the points that tickets 25, 28, 29 and 32 use [P].
- `docs/adr/` has ten ADRs and none covers versioning or release [V]. An ADR is
  optional here. AI-written ADRs stay `proposed`.

## What the Answer must name

The Answer must be testable. Name these items:

- The `mise` task and its exact failure condition.
- The file and the key that hold the version.
- The install line, with `--locked`, and the file that holds it.
- The `--version` output, and the edit to `exit_code_zero_for_help_and_version`
  if the output changes.
- The line in the change table for ticket 16, if any.
- The release tag pattern, the first-release rule and the missing-history rule.
- The rule for a source archive without `.git`.
- The rule for a package that reads the workspace `Cargo.lock` after a split.
- The effect of the parsing-crate list changes from tickets 30 and 35.
- The invariant owner, removable code, unresolved items and downstream tickets.

Check the chosen process against a first release, an existing release and an
unrelated safety tag. Exercise equal and lower version numbers.
Keep binary versions, format epochs and store identity as separate concepts.
Do not add a version floor to the store.

Ticket 36 must either include this Answer in the specification hand-off or state
that release tooling stays outside it. Its current blocker list omits ticket 39.
This review records the missing scope decision.
It does not change that list.

## Answer

Open.

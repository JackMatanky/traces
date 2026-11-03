# 09: Prototype BDD inside the e2e binary

Type: research (empirical: runs a throwaway spike in the repo; results only)
Status: resolved
Blocked by: 08

## Question

Does ticket 08's placement (scenarios folded into `tests/e2e.rs`) work in this
repo? Ticket 06 left these items unverified:

- `mise run verify` (including `check:layers` and `check:visibility`) passes
  with a `mod bdd` and one scenario in the e2e binary.
- The e2e binary builds with `--no-default-features` and needs no `test-utils`.
- Linux and Windows CI runs pass (`test-e2e` runs on three OSes).
- A `build.rs` for new `.feature` files: measure the rebuild cost of adding and
  editing a feature file in `traces`.
- The three public-API and token baselines do not change.

Ticket 08 adds two requirements for `check:layers`:

- It scans every path that holds feature files and steps.
- It fails when `git grep` exits with code 2 or higher.

## Decision to reach

- Placement holds, or the fallback is a dedicated `bdd` binary with a new
  `[[test]]`, phase and CI job.
- The `build.rs` cost: accept it, or document `touch`.

## Rules

- Use one pilot scenario from the process-contract class. Spawn the scenario
  through the `Sandbox` helper only.
- Do not merge the spike code. Record the results and any proposed `mise.toml`
  diff in `research/09-prototype.md`.
- Ticket 06 sets the crate pins. Put one `#[expect]` on each `mod bdd`
  declaration.

## Answer

Empirical findings live in
[research/09-prototype.md](../research/09-prototype.md).

### Decisions

1. **Local placement works (pilot in `tests/e2e.rs`):**
   - The task `mise run verify` passed on macOS.
   - The `e2e` phase found the scenario. You do not need to edit
     `.mise/tasks/test/_default`, `nextest.toml`, or `ci.yml`.
   - The `e2e` target compiles with `--no-default-features`. It does not need
     `test-utils`.
   - No public API or token baseline changed.
   - Linux and Windows CI did not run. Ticket 08's three-OS condition stays
     open. The first adoption CI run must check it.
   - You do not need a separate `bdd` test binary.

2. **Reject `build.rs` (document `touch`):**
   - A `build.rs` script marks the crate dirty on each `.feature` edit. This
     caused a full package rebuild (6.94s, one warm run).
   - Without `build.rs`, rustc tracks edits to existing `.feature` files
     through `include_str!`. Only `tests/e2e.rs` recompiled (0.87s).
   - Without `build.rs`, Cargo did not find a new `.feature` file. Run
     `touch tests/e2e/bdd.rs` after you add one (1.26s). Clean builds and CI
     find all files.

3. **New caveat (license policy):**
   - The crate `rstest-bdd 0.6.0` pulls in `foldhash 0.2.0` through `hashbrown
     0.17.1`.
   - `foldhash` uses the Zlib license.
   - The task `cargo deny check licenses` failed. Adoption must add `"Zlib"` to
     `[licenses].allow` in `deny.toml`.

4. **`check:layers` is not proved:**
   - The task passed. The spike files were untracked, so `git grep` did not
     read them.
   - The `|| true` calls still hide `git grep` errors. Ticket 08's
     exit-code requirement is not implemented.
   - No `mise.toml` change was made. See section 4 of
     `research/09-prototype.md`. The adoption slice must write and test the
     change.

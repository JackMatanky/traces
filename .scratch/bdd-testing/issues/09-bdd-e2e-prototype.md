# 14: Prototype BDD inside the e2e binary

Type: prototype
Status: open
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

- Use one pilot scenario from the process-contract class (exit codes or stream
  split). Spawn through `Sandbox` only.
- Do not merge the spike. Record the results and any proposed `mise.toml` diff
  in `research/14-prototype.md`.
- Ticket 06 sets the pins. It also sets one `#[expect]` on each `mod bdd`
  declaration.

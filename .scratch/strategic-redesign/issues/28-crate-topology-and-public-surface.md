# Single crate or workspace, and what is the public surface?

Type: grilling
Status: open
Blocked by: 8, 12, 18, 25, 26, 27

## Question
Decide the crate topology and visibility story: remain a single crate (what replaces the `traces::testing` 45-type backdoor and the frozen 1,528-token visibility baseline?) vs workspace split along the 10-tier DAG (which seams are crate-worthy by dependency isolation / API stability / second consumer, not by size); re-homing of root-level modules (task/field/tag/file_tracker/dirs/env_vars/delimiter/file_class_expander); `lib.rs` carrying 662 lines of test fixtures; the bench/integration/e2e access story; module-per-domain layout and dependency direction rules; dependency set pruning that follows from earlier decisions (speedate, winnow, indextree, noyalib roles, syn dev-dep).

Facts: `survey/cross-cutting.md` (edge table, H1/H3/H6), `survey/root-primitives.md` H3/H4, `mise.toml` check tasks, `tests/baselines/*`.
Alternatives: single crate with `#[doc(hidden)] pub` internals; workspace with `traces-test-support`; two crates (lib + cli); keep as is. Measure actual crate compile-time structure if it will decide the question (note as a prerequisite task if so).
Inputs: all prior boundary decisions, `research/08`, compat envelope (public Rust API frozen or not).

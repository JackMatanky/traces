# Single crate or workspace, and what is the public surface?

Type: grilling
Status: open
Blocked by: 8, 12, 18, 25, 26, 27

## Question
Decide the crate topology and visibility story: remain a single crate (what replaces the `traces::testing` 46-identifier backdoor (45 at charting) and the frozen 1,550-line visibility baseline (1,528 at charting; the baselines were edited by `207cc3fb`)?) vs workspace split along the 10-tier DAG (which seams are crate-worthy by dependency isolation / API stability / second consumer, not by size); re-homing of root-level modules (task/field/tag/file_tracker/dirs/env_vars/delimiter/file_class_expander); `lib.rs` carrying 662 lines of test fixtures; the bench/integration/e2e access story; module-per-domain layout and dependency direction rules; dependency set pruning that follows from earlier decisions (speedate, winnow, indextree, noyalib roles, syn dev-dep).

Facts: `survey/cross-cutting.md` (edge table, H1/H3/H6), `survey/root-primitives.md` H3/H4, `mise.toml` check tasks, `tests/baselines/*`.
Alternatives: single crate with `#[doc(hidden)] pub` internals; workspace with `traces-test-support`; two crates (lib + cli); keep as is. Measure actual crate compile-time structure if it will decide the question (note as a prerequisite task if so).
Inputs: all prior boundary decisions, `research/08`, compat envelope (public Rust API frozen or not).

Rule from the compatibility-envelope ruling: the library surface is Free (sole consumer). The primary visibility rule is to reduce visibility whenever possible and limit `pub` to outward-facing components. Widening is not banned, but each `pub` item (and each crate boundary's surface) needs an outward-facing justification; the default direction is narrowing. Evaluate every alternative, including the `#[doc(hidden)] pub` backdoor and the workspace split, against it.

User steer (after the compatibility-envelope ruling): consider a workspace layout, **starting with the CLI as a separate crate**. Treat this as the leading candidate to test, not a conclusion: evaluate it against keep-as-is and against the 2-3 crate shape in `research/08` (core / test-support / cli). Questions it adds: (a) what the CLI crate may depend on, i.e. the minimal core API the CLI needs, which becomes the first deliberate outward-facing `pub` surface under the visibility rule; (b) whether `tests/e2e` (already binary-only) needs anything beyond the split; (c) how `traces::testing` benches and integration tests reach internals once the CLI is out (a test-support crate vs narrow re-exports); (d) the order in which later splits could follow once the CLI boundary is proven. The split is blocked in practice by the workspace-host ticket, since composition currently lives in the CLI.

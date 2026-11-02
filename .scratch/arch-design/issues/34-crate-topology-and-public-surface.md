# Single crate or workspace, and what is the public surface?

Type: grilling
Status: open
Blocked by: 8, 16, 22, 31, 32, 33, 12

## Question

Decide the crate topology and the visibility story. Make one decision for each
bullet below:

- Choose between one crate and a workspace split along the 10-tier DAG.
- For one crate, decide what replaces the `traces::testing` 46-identifier
  backdoor (45 at charting).
- For one crate, decide what replaces the frozen 1,550-line visibility baseline
  (1,528 at charting). `207cc3fb` edited the baselines.
- For the workspace split, decide which seams are crate-worthy. Judge each seam
  by dependency isolation / API stability / second consumer, not by size.
- Decide the re-homing of the root-level modules
  (task/field/tag/file_tracker/dirs/env_vars/delimiter/file_class_expander).
- Decide the handling of the 662 lines of test fixtures in `lib.rs`.
- Decide the bench/integration/e2e access story.
- Decide the module-per-domain layout and the dependency direction rules.
- Decide the dependency set pruning that follows from earlier decisions
  (speedate, winnow, indextree, noyalib roles, syn dev-dep).

Facts: `survey/cross-cutting.md` (edge table, H1/H3/H6),
`survey/root-primitives.md` H3/H4, `mise.toml` check tasks, `tests/baselines/*`.

Alternatives:

- Remain a single crate with `#[doc(hidden)] pub` internals.
- Use a workspace with `traces-test-support`.
- Use two crates (a library crate plus a CLI crate).
- Keep as is.

Measure the actual crate compile-time structure if this measurement decides the
question. Note it as a prerequisite task if so.

Inputs: all prior boundary decisions, `research/08`, compat envelope (public
Rust API frozen or not).

Rule from the compatibility-envelope ruling: the library surface is Free (sole
consumer). The primary visibility rule reduces visibility whenever possible. The
rule also limits `pub` to outward-facing components.

Widening is not banned. Each `pub` item needs an outward-facing justification.
Each crate boundary's surface needs an outward-facing justification. The default
direction is narrowing.

Evaluate every alternative, including the `#[doc(hidden)] pub` backdoor and the
workspace split, against this rule.

User direction (after the compatibility-envelope ruling): evaluate a workspace
layout, **starting with the CLI as a separate crate**. Treat this layout as the
leading option, not as a conclusion. Evaluate it against keep-as-is and against
the 2-3 crate shape in `research/08` (core / test-support / CLI). This layout
adds these decisions:

- Decide what the CLI crate is allowed to depend on. The minimal core API that
  the CLI needs becomes the first deliberate outward-facing `pub` surface under
  the visibility rule.
- Decide whether `tests/e2e` (already binary-only) needs anything beyond the
  split.
- Decide how `traces::testing` benches and integration tests reach internals
  after the CLI is out. Choose a test-support crate or narrow re-exports.
- Decide the order in which later splits follow, after the CLI boundary is
  proven. Open: the order.

The workspace-host ticket blocks the split in practice, because composition
currently lives in the CLI.

## Inputs from the value model decision

See [ticket 18's Answer](./18-value-model.md). `NoteFieldValue` and `Link` are
`pub` today but externally nearly opaque. No default-feature public function
returns a value. `Note::outlinks()` accessors are `pub(crate)`. Decide their
visibility under the reduce-`pub` rule. The root `Link`'s module name is also
chosen here.

## Inputs from the coercion and comparison decision

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md). Decide the
visibility of the new value-semantics module (loose equality, bracketed order,
sort key). Sorting adds a collation dependency (`icu_collator`). Its binary
size and its presence in the tree are unverified. This counts in the
dependency pruning.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). The runtime
value type must sit below both `schema` and `note`. Schema literals stay
separate. The class hierarchy type needs an owner. The app layer of core holds
use cases composed from domain modules.

A one-implementor trait in a crate-root glue file fails the orphan rule once the
modules are separate crates. Module-to-module wiring is therefore plain data.
The checker, the violation type and field descriptors are outward-facing for lsp
and mcp consumers.

## Inputs from the IWE research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md). IWE is a
five-crate workspace [V]. It contains `liwe` (zero-I/O domain: graph, model,
parsing, operations, query) and `diwe` (engine: filesystem, loader, watcher,
config, search, `find`, `retrieve`). It also contains three hosts (`iwe` CLI,
`iwes` LSP, `iwec` MCP). Each host depends on both libraries.

This is a data point beyond the 2-3 crate shape of research 8. The app layer is
a separate library crate from the domain crate. Whether that split earns its
cost for Traces versus one core crate (as ticket 20 states it) remains open and
untested here. Each crate
bundles its integration tests into one `tests/main.rs` binary to avoid per-test
link cost [V]. Not examined: how strictly `liwe` limits `pub`.

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md).
CLI, LSP and MCP are thin consumers of an app layer inside core. Only adapters
acquire cwd/env, time, local zone and prompt mode. Keep workspace policy/config
generation and operation-scoped temporal frames distinct. Do not add a public
god-context or a process-global registry merely to prepare for multi-root.

Provide narrow, justified outward APIs for config admission, query execution and
fallible results. If enforcing ambient-free core with Clippy, enumerate exact
disallowed paths rather than wildcard method patterns. Inventory-only scans
cannot establish the dependency boundary.

## Inputs from the freshness decision

See [ticket 25's Answer](./25-index-freshness-and-invalidation.md).

- `build.rs` reads `Cargo.lock` to hash the resolved versions of `pulldown-cmark`, `logos`, `noyalib`, `jiff`, `speedate` and `postcard`. After a workspace split, the lock file is at the workspace root. Decide how `build.rs` finds it.
- The `check:boundaries` mise task confines `pulldown-cmark` to `src/note/parser.rs` and `src/note/parser/`. It confines `redb` and `postcard` to `src/index/`. It is provisional. Replace it with crate boundaries if this ticket splits the crates.
- Remove `META_VERSION` and `META_VERSION_KEY` from `src/index/store.rs`. The store identity replaces them.

## Inputs from the note document and index read model decisions

See [ticket 23's Answer](./23-note-document-model.md) and [ticket 24's Answer](./24-index-read-model-and-query-execution.md).

- Decide the visibility of `Link`, `NoteLink`, `NoteDoc`, the status map and the buffer parser. Ticket 23 puts `Link` at the crate root as shared vocabulary. `NoteLink` stays in `note`.
- Decide where `NoteDoc` lives. Ticket 23 keeps `Note` as the facts type.
- Decide the visibility of the index and the store port. No engine type crosses the port.
- Ticket 24 asks for a domain-level store port with an adapter boundary.

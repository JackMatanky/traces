# Query core: predicate algebra, row model, and execution against the index

Type: grilling
Blocked by: 02, 10, 12, 15, 07, 21

## Question

The query subsystem has separate lexers, ASTs, and evaluators for *source selection* (`--from`: tags, folders, classes, paths, boolean combinations; `grammar/source.rs`, ~1.2K lines) and *row filtering* (`--where`: field expressions, arithmetic, date literals; `grammar/filter.rs`, ~2.1K lines), with overlapping capabilities (tag membership is expressible in both). Rows are `QueryRow`s reportedly carrying an `Arc<WorkspaceIndex>` each plus flattened field vectors; execution is lazy-memoized (`QuerySet`/plan), with top-K fusion; class expansion crosses to schema through a one-method `FileClassExpander` trait implemented by a glue module that both `cli` and `template` must construct; a suspected document-order comparison defect ignores list-item position. Decide the **core query model**: whether source selection and filtering are one predicate algebra or two deliberately separate stages (index-level pruning vs row-level evaluation), the row/result representation and ownership, how execution binds to the index read model chosen in [Index read model](15-index-read-model.md), how class (schema) knowledge enters (trait seam, resolved membership facet, or injected resolver value), and how typing/zone context from [Coercion policy](10-coercion-policy.md) and [Ambient environment](12-ambient-environment-and-execution-context.md) reach evaluation.

## Evidence status

Survey leads (UNVERIFIED until ticket [Verify system findings](02-research-verify-system-findings.md) §2–4 lands): [query §2–§11](../survey/query.md), [cross-cutting §6](../survey/cross-cutting.md). ADR-0008 (source grammar) and ADR-0009 (filter date literals) are claims; the `FileClassExpander` "keep" verdict is a hypothesis.

**Verified (ledgers landed 2026-10-18)** — [02 §2–4](../research/02-verify-system-findings.md): `QueryRow` is **48 bytes** (not 112) and clones an `Arc<WorkspaceIndex>` per row (`results.rs:44-50,57`); `cmp_document_order` (`results.rs:426-432`) compares only note `position`, so list/task rows from the same note compare `Equal` — a real, untested defect exposed to templates via `custom_cmp` (`template/engine/query.rs:539`). Sources (`--from`: `SourceLexer`/`SourceSelector`, `source.rs`) and filters (`--where`: `FilterLexer`/`FilterExpr`, `filter.rs`) are separate lexers and ASTs expressing overlapping tag/class/folder predicates at different pipeline stages. `TextShape::classify` is called from sort (`sort.rs:394`) and filter (`filter.rs:85,242`). `FileClassExpander`: one production impl (`SchemaService`, `file_class_expander.rs:15`), one test mock, built at `cli/mod.rs:479-480` and `template/engine/query.rs:132-133`; `query` has zero `schema` references. [03 §5–7](../research/03-performance-evidence.md): in-memory `run` is 48 B/row; sorted 184 B/row; `QuerySet::clone` is 12 ns (Arc); materializing a chained plan with transforms does `base.to_vec()` (20K rows → 20K `Arc` increments + ~960 KB) — a shallow-clone design that still copies on materialization. Evaluation costs are small at 20K rows (sort 0.57–1.2 ms; date filters 0.83–2.7 ms).

**Verified (research landed 2026-10-18)** — [Carry-forward evidence gaps](21-research-carry-forward-evidence-gaps.md) ([ledger](../research/21-carry-forward-evidence-gaps.md) Gap A): the filter comparison operators (`filter.rs:807,832`) call `NoteFieldValueRef::compare`, whose `Date`/`DateTime` cross-kind arms read the ambient zone on every call and map a lookup failure to a fixed ordering (`note/field.rs:297-300`). Cross-kind ordering is therefore a semantic the query core must either fix explicitly (zone supplied by the evaluation context) or define as an error.

## Alternatives (not exhaustive)

- Keep two stages, share a common expression core; make source selection compile to an index-pruning plan.
- Unify into one predicate algebra with a planner that pushes index-capable predicates down.
- Lightweight indexed row handles borrowing from the result set vs owned/`Arc`-carrying rows.
- Class membership as an index-time facet (expansion resolved once) vs query-time expansion through a resolver.

## Done when

The answer states the grammar(s)/AST(s) and evaluator ownership, the plan and result types and their lifetimes, execution against the [Index read model](15-index-read-model.md) read interface, the class-knowledge mechanism, the typing/zone inputs, the stable-order guarantee, what is deleted, and which fog items (presentation, CLI query construction) become specifiable.

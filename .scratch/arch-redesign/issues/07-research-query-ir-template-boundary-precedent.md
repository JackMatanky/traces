# Research: precedent for query IR and template/capability boundaries

Type: research
Status: resolved

## Question

Gather transferable lessons for tickets [Query core](16-query-core.md) (one predicate algebra or two stages; row model; execution against an index) and [Template engine boundary](18-template-engine-boundary.md) (what a template engine owns vs receives; prompts; compile/analysis structure).

**General/Rust comparables** (read source/docs; rust-docs-mcp for crate APIs): DataFusion/Polars logical plans, expression IR and predicate pushdown (pruning stage vs row filter); jq/jaq evaluation model; gitoxide pathspecs and `ignore` globs (selection DSL separate from filtering); minijinja's own guidance on objects, state, globals and static analysis (`machinery`); cargo-generate (prompts, placeholders, non-interactive mode), Copier and Cookiecutter (declared questions vs mid-render prompts — a direct test of ADR-0001's no-declaration format), Zola/Tera and Helm (functions injected by a host; library vs engine boundary).

**Local digests** (`docs/digests/`; very large — **search/grep first, read only matched ranges**; `*-src-digest.txt` = source, `*-docs-digest.txt` = docs):

- `obsidian_blacksmithgu-obsidian-dataview-src-digest.txt`: query AST, expression evaluation, source selection (`FROM`) vs row filtering (`WHERE`) split, row/result representation, and how results are rendered (presentation vs query). Use `-docs-` only for the intended semantics of source vs where.
- `obsidian_silentvoid13-obsidian-templater-src-digest.txt`: how its runtime namespaces (`tp.*`) are assembled, what the host injects, how prompts/interactivity run during evaluation, and error reporting; `-docs-` for declared behavior.
- `obsidian_obsidian-tasks-src-digest.txt`: its filter/sort/group pipeline over tasks, as a second example of a predicate stage and its reuse across query blocks.

## Deliverable

`.scratch/arch-redesign/research/07-query-ir-template-boundary-precedent.md`: at most ~8 lessons tagged to [Query core](16-query-core.md)/[Template engine boundary](18-template-engine-boundary.md), graded `VERIFIED` (source, cite), `ECOSYSTEM-CONSENSUS`, or `INFERENCE`, each with a **transferability test** (single-user synchronous CLI; vaults ≤ ~20K notes where in-memory scans cost µs; DSL strings in both CLI flags and templates; interactive prompts mid-render) and **negative lessons** (e.g. planner machinery that only pays off at scales Traces lacks). Do not recommend a design; do not modify production code or docs. Append an `## Answer` (5–8 lines) to THIS ticket and set `Status: resolved`. Do NOT edit `map.md`.

## Exact digest file names (`docs/digests/`)

`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt` and `-docs-digest.txt`, `obsidian_silentvoid13-templater-src-digest.txt` and `-docs-digest.txt` (note: `templater`, not `obsidian-templater`), `obsidian_obsidian-tasks-src-digest.txt` and `-docs-digest.txt`.

## Answer

Eight verified precedent lessons across query IR and template boundaries are distilled in `.scratch/arch-redesign/research/07-query-ir-template-boundary-precedent.md`.
For [Query core](16-query-core.md) (Query Core), DataFusion/Polars and Dataview prove that two execution stages (index set pruning vs in-memory row filtering) do not require two separate grammars or lexers; Dataview resolves set-membership sources to candidate paths before row materialization, while row handles in jaq/Dataview never embed database handles (refuting Traces' 20K `Arc<WorkspaceIndex>` clones).
Minijinja's `custom_cmp` contract mandates total ordering, confirming Traces' note-only `cmp_document_order` defect for intra-note entities.
For [Template engine boundary](18-template-engine-boundary.md) (Template Engine), cargo-generate, Copier, and Cookiecutter demonstrate that CLI template engines universally resolve declared question schemas upfront, proving that ADR-0001's mid-render prompts are fundamentally hostile to non-interactive scripting without a two-phase prompt extraction pass.
Helm and Zola/Tera demonstrate that template engines must remain thin capability adapters rather than domain owners (moving Moment translation, date math, schema reflection, and query wrappers out of `template`), while upstream Minijinja 3.0 provides type-safe render extensions (`State::get_or_insert_extension`) and AST static analysis (`Template::undeclared_variables`) without sandboxed execution. Checked later ([Carry-forward evidence gaps](21-research-carry-forward-evidence-gaps.md)): the pinned 2.24.0 already has `undeclared_variables` but lacks the extension API (3.0-only).

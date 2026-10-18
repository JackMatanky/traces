# Precedent for Query IR and Template/Capability Boundaries

**Ticket:** `.scratch/arch-redesign/issues/07-research-query-ir-template-boundary-precedent.md`  
**Date:** 2026-10-18  
**Caveat:** lessons citing minijinja 3.0 (`State::get_or_insert_extension`, `Template::undeclared_variables`, `state.rs:537-542`) come from upstream 3.0 sources; `Cargo.toml` pins `minijinja 2.24.0`, so their availability had to be checked against 2.24.0. **Checked in [research 21](21-carry-forward-evidence-gaps.md) Gap B:** `undeclared_variables` exists in 2.24.0; `get_or_insert_extension` is 3.0-only. Lesson 7's "extensions replace the downcasting caches" therefore needs a crate bump (cost unchecked).
**Scope:** Precedent research for query core (Ticket 16) and template engine boundary (Ticket 18). Evaluates comparables across database engines (DataFusion, Polars), stream processors (jq, jaq), pathspec engines (gitoxide, ignore), template engines (Minijinja, Zola/Tera, Helm), scaffolding generators (cargo-generate, Copier, Cookiecutter), and Obsidian plugins (Dataview, Templater, Tasks). Planning-only; zero modifications to production code, tests, or documentation.

---

## Precedent Lessons Ledger

### Lesson 1: Two-Stage Execution vs Single Predicate Algebra (DataFusion/Polars vs Dataview)

- **Tag:** Ticket 16 (Query Core)
- **Grade:** `VERIFIED`
- **Precedent & Provenance:**
  - *Obsidian Dataview:* Dataview separates query parsing into source selection (`Source`: tags, folders, links; `src/data-index/source.ts:4742-4799`) and row filtering (`QueryOperation::Where`: arbitrary expression trees; `src/query/query.ts:8245-8255, 9050-9062`). `resolveSource` executes source selection against in-memory index sets via `matchingSourcePaths` (`src/data-index/source.ts:4601-4679`), using `Set` intersections and unions (`index.tags.getInverse`, `index.prefix.get`) to produce candidate file paths, and only then calls `resolveMarkdownData` (`src/data-index/source.ts:4703-4713`) to materialize rows before evaluating `executeCore` row-by-row expressions.
  - *DataFusion / Polars:* Both systems maintain a *single* expression AST (`datafusion_expr::Expr`, Polars `Expr`). Rather than maintaining two separate grammars, optimizer passes (`FilterPushDown` in DataFusion, `predicate_pushdown` in Polars) push candidate expressions into index/partition pruning (`PruningPredicate`), leaving residual or unsupported expressions to run as row filters (`FilterExec`).
- **Transferability Test:**
  - *Traces Constraints:* Single-user synchronous CLI; vaults ≤ 20K notes; in-memory scans take ~100–300 µs; redb disk index lookups have fixed overhead ~16–37 ms (`03-performance-evidence.md` §2, §6).
  - Traces currently maintains *two separate lexers, ASTs, and evaluators* (`grammar/source.rs` ~1.2K lines, `grammar/filter.rs` ~2.1K lines) that duplicate tag/folder/class filtering in incompatible syntaxes (`#tag` vs `contains(tags, "#tag")`).
  - A unified expression algebra with a compiler pass that maps index-capable terms (tag/folder/class sets) to candidate set lookups eliminates syntax fragmentation and duplicate parser code.
- **Negative Lesson:**
  - Full cost-based query planners with expression rewrite trees, statistics estimation, and physical operator planning (DataFusion style) are massive deadweight for vaults ≤ 20K notes where unindexed in-memory scans execute in microseconds. Index pruning pays off only for coarse inverted-index lookups.

---

### Lesson 2: Selection DSL as Hierarchical Set Pruning vs General Expression Filtering (gitoxide & ignore vs Obsidian Tasks)

- **Tag:** Ticket 16 (Query Core)
- **Grade:** `VERIFIED`
- **Precedent & Provenance:**
  - *Gitoxide (`gix-pathspec`) & BurntSushi (`ignore`/`globset`):* Selection DSLs (`gix_pathspec::Pattern`, `gix_pathspec::Search`) are strictly scoped to hierarchical path and pattern matching (prefixes, wildcards, exclusions). They operate directly on repository index trees to prune directory traversal. They do not evaluate file content or commit metadata; content matching (`grep`) and metadata filtering (`log`) are separate stages.
  - *Obsidian Tasks:* Line-by-line task query execution pipeline strictly separates task filtering (`tasks.filter(fn)`), sorting (`Sort.by`), top-K slicing (`slice(0, limit)`), and grouping (`TaskGroups`) (`src/Query/Query.ts:12105-12135`).
- **Transferability Test:**
  - *Traces Constraints:* Selection strings are passed via CLI flags (`--from`) and template helper arguments (`traces.query(from=...)`).
  - When selection is kept strictly as a set-membership DSL (folders, path globs, tag sets), it maps directly to inverted index lookups without parsing ambiguity.
  - Overloading selection syntax with relational operators, field arithmetic, or date comparisons blurs the conceptual boundary with `--where` and necessitates a second parser.
- **Negative Lesson:**
  - Expanding a selection DSL into a general-purpose expression language creates grammar collisions and duplicate feature implementations. Selection DSLs must remain strictly focused on candidate set pruning.

---

### Lesson 3: Row Model Decoupling — Materialized Dictionaries vs Index References (Dataview & jaq vs Traces `QueryRow`)

- **Tag:** Ticket 16 (Query Core)
- **Grade:** `VERIFIED`
- **Precedent & Provenance:**
  - *Obsidian Dataview:* Row model is `Pagerow = Datarow<DataObject>` where `id` is a `Literal` and `data` is a flat key-value dictionary (`Record<string, Literal>`) (`src/data-index/source.ts:4598`), completely decoupled from the index once materialized via `resolveMarkdownData` (`src/data-index/source.ts:4703-4713`).
  - *jaq:* Evaluation produces immutable values (`Rc<Val>` / `Arc<Val>`), executed as a streaming iterator without maintaining back-references to source documents or index tables.
  - *Traces Current State:* `QueryRow` is 48 bytes and clones an `Arc<WorkspaceIndex>` per row (`src/query/results.rs:44-50, 57`; verified in `02-verify-system-findings.md` §2).
- **Transferability Test:**
  - *Traces Constraints:* Vaults up to 20K notes; in-memory scans cost microseconds.
  - Cloning `Arc<WorkspaceIndex>` across 20,000 rows generates 20,000 atomic refcount increments/decrements, increases memory footprint, and pins the entire workspace index in memory as long as result sets or template objects are held.
  - Query result sets should borrow from an index or hold lightweight projected row handles, never owned `Arc` references to the indexing engine.
- **Negative Lesson:**
  - Lesson for Ticket 16: individual query rows should not embed handles to the indexing engine or database instance; tying row lifetimes to index instances prevents index teardown and bloats row storage.

---

### Lesson 4: Total Ordering Contract in Row Comparison and Presentation Projection (Minijinja `custom_cmp` vs Traces `cmp_document_order`)

- **Tag:** Ticket 16 (Query Core) & Ticket 18 (Template Engine Boundary)
- **Grade:** `VERIFIED`
- **Precedent & Provenance:**
  - *Minijinja:* `minijinja::value::Object::custom_cmp` mandates a total ordering contract (`Option<Ordering>` returning `Some` for objects of the same type; `minijinja/src/value/object.rs:278-295`). Inconsistent or partial comparators lead to panics or nondeterministic sorting during template `|sort`.
  - *Dataview & Obsidian Tasks:* Presentation projection is a distinct post-sort extraction phase (`executeCoreExtract` in Dataview `src/query/query.ts:8538-8551`; `TaskGroups` in Tasks `src/Query/Query.ts:12129`). Tasks preserves stable document order using note path and task line number.
  - *Traces Defect:* `cmp_document_order` (`src/query/results.rs:426-432`) compares only note `position`. List items and tasks within the same note compare `Equal`, directly exposing an invalid total ordering to templates via `custom_cmp` (`src/template/engine/query.rs:539`; verified in `02-verify-system-findings.md` §2).
- **Transferability Test:**
  - *Traces Constraints:* CLI queries and templates sort mixed entities (notes, tasks, list items).
  - Document order across sub-note entities requires a composite key: `(note_position, line_number, byte_offset)`. Comparing note position alone causes tie-breaking collisions for all intra-note items.
- **Negative Lesson:**
  - Treating document order as note-level position alone causes non-deterministic sorting and breaks Minijinja's `custom_cmp` total ordering guarantee for sub-document entities.

---

### Lesson 5: Declared Question Schema vs Imperative Mid-Render Prompts (cargo-generate, Copier, Cookiecutter vs Obsidian Templater)

- **Tag:** Ticket 18 (Template Engine Boundary)
- **Grade:** `VERIFIED`
- **Precedent & Provenance:**
  - *cargo-generate, Copier, Cookiecutter:* CLI template generators universally mandate upfront parameter declarations (`cargo-generate.toml` `[placeholders]`, `copier.yml` questions, `cookiecutter.json`). All prompts are resolved *before* template evaluation begins (via CLI flags, config files, or pre-render interactive wizards). The template rendering phase is 100% pure, functional, reproducible, and side-effect-free.
  - *Obsidian Templater:* Templates declare no parameters (ADR-0001 style). Prompts (`tp.system.prompt()`, `tp.system.suggester()`) execute imperatively *mid-render* via async modal dialogs awaiting user input in the Obsidian GUI (`src/core/functions/internal_functions/system/InternalModuleSystem.ts:2986-3059`).
- **Transferability Test:**
  - *Traces Constraints:* Single-user synchronous CLI; interactive terminal prompts; automated CLI scripting (`--no-input`); template re-use across batch operations.
  - Mid-render prompts break non-interactive CLI workflows (pipes, batch generation, CI), discard user answers when later syntax/render errors occur, and prevent static analysis (LSP) from discovering needed variables.
  - However, Templater ecosystem parity requires supporting templates with inline prompt functions.
  - A two-phase execution model (statically analyzing or dry-running templates to collect prompts upfront, then executing with an answers context) bridges CLI automation with Templater compatibility.
- **Negative Lesson:**
  - Relying exclusively on mid-render interactive execution destroys headless automation and idempotence. Templater functions in Obsidian only because it runs inside an interactive desktop GUI.

---

### Lesson 6: Host Capability Injection & Engine Boundaries (Helm, Zola/Tera vs Templater)

- **Tag:** Ticket 18 (Template Engine Boundary)
- **Grade:** `VERIFIED`
- **Precedent & Provenance:**
  - *Helm:* Wraps Go `text/template` and the Sprig library with host capabilities (`include`, `required`, `lookup`). In local/dry-run rendering (`helm template`), external cluster queries (`lookup`) are mocked or disabled to ensure offline idempotence and sandboxing.
  - *Zola / Tera:* Tera provides the template expression and layout engine; Zola registers host callbacks (`load_data`, `get_url`) querying Zola's site index. Tera owns zero Markdown parsing, taxonomy logic, or path resolution.
  - *Templater:* `Templater` wraps the engine (`rusty_engine_bg.wasm`), while `InternalFunctions` injects modular host namespaces (`InternalModuleDate`, `InternalModuleFile`, `InternalModuleSystem`; `src/core/functions/internal_functions/InternalFunctions.ts:2344-2381`).
- **Transferability Test:**
  - *Traces Constraints:* Production module graph is a strict DAG; template subsystem currently spans ~13K lines.
  - Traces' template engine currently owns substantive domain logic: Moment-token date format translator (~140 lines) and anchor date math (`engine/date.rs`, 2.5K lines), schema reflection via downcasting (`engine/schema.rs`), prompt option parsing (`engine/ui.rs`), and query wrappers (`engine/query.rs`, 2K lines).
  - This duplicates domain behavior that belongs in `temporal`, `schema`, and `query`.
- **Negative Lesson:**
  - Hypothesis for Ticket 18: template engines should not become secondary domain owners. Date formatting dialects and anchor math belong in `temporal`; schema reflection belongs in `schema`; query planning belongs in `query`. The engine must remain a thin capability adapter.

---

### Lesson 7: Render-Scoped State and Static Analysis in Minijinja (Minijinja 3.0 `State::extensions` & `undeclared_variables`)

- **Tag:** Ticket 18 (Template Engine Boundary)
- **Grade:** `VERIFIED`
- **Precedent & Provenance:**
  - *Minijinja 3.0 Extensions:* `State::get_or_insert_extension::<T: Send + 'static>(value)` (`minijinja/src/vm/state.rs:537-542`) provides first-class, type-safe, render-local extension storage, eliminating string-keyed downcasting over temporary dictionaries.
  - *Minijinja Static Analysis:* `Template::undeclared_variables(nested: bool)` and `Expression::undeclared_variables(nested: bool)` (`minijinja/src/template.rs:317-326`, `minijinja/src/expression.rs:93-104`) parse the AST via `minijinja::machinery::parse` and return all variable references and nested attribute paths (`foo.bar`) without executing templates, filters, or control flow.
- **Transferability Test:**
  - *Traces Constraints:* md-pkm-lsp (Ticket 22) requires static template analysis that never executes templates; Traces currently downcasts string-keyed `State` temp-storage (`query.index_cache`, `schema.service_cache`) and re-parses root templates on every render (`src/template/engine.rs:190-193`).
  - Minijinja's native `State::extensions` replaces custom downcasting caches, while `undeclared_variables(nested: true)` provides variable and namespace extraction for LSP without sandboxing or mock execution. *Version note (research 21 Gap B):* `undeclared_variables` is available in the pinned 2.24.0; `State::get_or_insert_extension` is not (3.0-only).
- **Negative Lesson:**
  - Do NOT implement custom AST walkers or sandboxed mock evaluators to find template dependencies; Minijinja's built-in AST analysis (`undeclared_variables`) inspects all identifiers and attribute paths natively.

---

### Lesson 8: Dynamic Context Projections vs Eager Serialization (Minijinja `Object` as Map vs Deep JSON Serialization)

- **Tag:** Ticket 16 (Query Core) & Ticket 18 (Template Engine Boundary)
- **Grade:** `VERIFIED`
- **Precedent & Provenance:**
  - *Minijinja "Map As Context":* Minijinja's `Object` trait allows implementing dynamic map context via `get_value` / `get_value_by_str` (`minijinja/src/value/object.rs:174-301`). The engine evaluates field lookups lazily on demand without converting entire Rust data structures into serialized `minijinja::Value` or `serde_json::Value` objects upfront.
  - *Obsidian Dataview:* Data rows lazily serialize metadata (`page.serialize(index)`) only when resolved by `resolvePathData` or accessed in evaluation contexts (`src/data-index/source.ts:4710`).
- **Transferability Test:**
  - *Traces Constraints:* Vaults up to 20K notes; synchronous CLI responsiveness under 50 ms.
  - Eagerly serializing 20,000 note objects into template value trees consumes tens of megabytes of heap allocations and tens of milliseconds of CPU time.
  - Exposing note fields and query rows through lightweight `Object` implementations defers field extraction to the specific properties referenced by the template.
- **Negative Lesson:**
  - Never eagerly serialize entire vault read models or query result sets into template values ahead of time; lazy accessor objects (`Object`) prevent quadratic serialization overhead.

---

## Provenance and Verification Summary

| Lesson | Ticket | Area | Grade | Key Sources |
|---|---|---|---|---|
| **1. Two-Stage Execution vs Single Predicate Algebra** | 16 | Query Core | `VERIFIED` | Dataview `source.ts:4601-4733, 8234-8255`; DataFusion `datafusion_expr::Expr`, `FilterPushDown` |
| **2. Selection DSL as Hierarchical Set Pruning** | 16 | Query Core | `VERIFIED` | `gix-pathspec::Search`, `gix-pathspec::Pattern`; Tasks `Query.ts:12105-12135` |
| **3. Row Model Decoupling & Lifetime Management** | 16 | Query Core | `VERIFIED` | Dataview `source.ts:4598`; jaq `val.rs`; Traces `results.rs:44-50, 57` |
| **4. Total Ordering Contract & Intra-Doc Order** | 16 / 18 | Query / Template | `VERIFIED` | Minijinja `Object::custom_cmp`; Tasks `Query.ts:12129`; Traces `results.rs:426-432` |
| **5. Declared Question Schema vs Mid-Render Prompts** | 18 | Template Engine | `VERIFIED` | cargo-generate guide; copier docs; Templater `InternalModuleSystem.ts:2986-3059` |
| **6. Host Capability Injection & Engine Boundaries** | 18 | Template Engine | `VERIFIED` | Helm `helm template` docs; Zola/Tera source; Templater `InternalFunctions.ts:2344-2381` |
| **7. Render-Scoped State & Static Analysis** | 18 | Template Engine | `VERIFIED` | Minijinja 3.0 `state.rs:537-542`, `template.rs:317-326` |
| **8. Dynamic Context Projections vs Eager Serialization** | 16 / 18 | Query / Template | `VERIFIED` | Minijinja `src/value/object.rs`; Dataview `source.ts:4710` |

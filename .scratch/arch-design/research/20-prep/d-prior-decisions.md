# Prior Decisions, Constraints, and Handoffs Bearing on Ticket 16

- **Topic:** Architectural audit of settled decisions, constraints, downstream couplings, and unresolved loops for Wayfinder Ticket 16 (Schema Note Typing and Validation).
- **Status:** Complete research synthesis.
- **Deliverable:** `.scratch/arch-design/research/20-prep/d-prior-decisions.md`.

---

## 1. Constraint Table (Decisions Bearing on Ticket 16)

Ticket 16 must honour all settled invariant and architectural decisions from tickets 01–15, ADRs, and map directives:

| ID | Source | Settled Rule / Quote | Direct Invariant Consequence for Ticket 16 |
|---|---|---|---|
| **C1** | `issues/12:24-25` | *"The user is the only consumer. No published release, no external Rust crate depends on the library. Every surface is therefore at most Migratable unless it would break the user's own vault files."* | [V] Schema TOML formats, error codes, and field typing rules are **Migratable**; breaking user Markdown vault files is forbidden. |
| **C2** | `issues/12:30,32` | **Frozen Surfaces:** Vault-format parsing and index query result equivalence with fresh parse are strictly **Frozen**. | [V] Schema validation or typing cannot silently reinterpret or mutate raw Markdown note parsing or produce divergence between fresh and cached query results. |
| **C3** | `issues/12:39` | **Schema TOML Format (Migratable):** `config.toml` and schema TOML formats (incl. external select-values files) are Migratable. | [V] Ticket 16 is free to simplify schema TOML syntax, eliminate external select files, or restructure field declarations. |
| **C4** | `issues/12:47` | **Visibility Rule:** *"Reduce visibility wherever possible and limit `pub` to outward-facing components; widening requires outward-facing justification."* | [V] Types in `src/schema/` must remain `pub(crate)` unless directly consumed by the CLI or future LSP/MCP boundary. |
| **C5** | `issues/13:37-44` | **LSP Committed, Source-Aware Diagnostics:** Shared core must support LSP; editor-facing behavior needs source-aware diagnostics; general note write-back is **out of scope**. | [V] Note validation must produce byte-accurate diagnostics for LSP consumption; schema never performs automated write-back formatting. |
| **C6** | `issues/13:57-58` | **Synchronous Core Invariant:** Domain analysis and query execution remain synchronous (`Send + Sync`). | [V] Schema loading, inheritance DAG resolution, and note validation passes must be purely synchronous. |
| **C7** | `issues/14:447-450` | **Two Owned Value Enums:** Runtime note data uses `note::NoteFieldValue`; schema declarations use schema-local `SchemaLiteral`. | [V] `src/schema/` never models or owns note values; schema literals hold no `Date`, `DateTime`, `Duration`, or `Link` variants. Native TOML dates read as strings [`14:479-480`]. |
| **C8** | `issues/14:481-482` | **Select Values as Strings:** *"Select values that name links stay strings; ticket 16 interprets them at the consumer boundary with the shared wikilink parser."* | [V] Schema select options store literal strings; conversion to `Link` occurs only when matched against notes. |
| **C9** | `issues/14:501-505` | **Schema-Free Parse Classification:** Frontmatter classification is quote-insensitive and shares a unified parser with inline fields. | [V] Raw note parsing does not consult schemas to classify lexical tokens; classification is strictly syntactic at the note boundary. |
| **C10** | `issues/15:55-59` | **Single Classification at Parse:** *"Note values are classified once at parse and never sniffed again; the only late typing is an operand-adaptive string literal in queries."* | [V] Notes do not undergo late re-sniffing; schema typing cannot invisibly re-classify stored note fields during query evaluation. |
| **C11** | `issues/15:104-106` | **Frontmatter String Override Handoff:** *"A frontmatter value that must stay text uses the schema override (ticket 16)."* | [V] Ticket 15 explicitly handed the mechanism for forcing a date-shaped frontmatter value to remain text to Ticket 16. |
| **C12** | `issues/15:122-124` | **Adaptive Literal Settles Select Matching:** *"A `Link` row adopts a literal as its folded target... This settles schema select string vs `Link` matching for ticket 16."* | [V] Ticket 15 resolved select-string-to-link matching via query literal adoption; no separate matching engine is needed in schema. |
| **C13** | `issues/15:159-167` | **Error Mode Principle:** *"classification failure keeps the string; explicit casts fail to Null; filters never error at evaluation... Advisory stderr notices"*. | [V] Schema validation failures must be advisory diagnostics, never query-aborting or index-gating errors. |
| **C14** | `research/03:79-80` | **Document Open-World Tolerance:** Markdown notes MUST parse and index regardless of schema conformance; zero schema-gating at ingestion. | [V] Validating notes against schemas is an on-demand analysis pass (Taplo/jsonschema pattern), not an index gate. |
| **C15** | `docs/adr/0006:75-77` | **Schema Supplies Values Only:** *"The schema supplies values only; the template author picks the interactive `ui.*` function."* | [V] Schemas do not declare interactive UI widgets; templates bind values to `ui.select` explicitly. `required`/`multi` are declared inert [`0006:110-111`]. |
| **C16** | `docs/adr/0007:47-53` | **Linearized Inheritance DAG:** Multiple inheritance via `extends` (is-a DAG) and `excludes`. Kahn's topological sort linearizes resolution; first-listed parent wins. | [V] Inherited field resolution is pure, acyclic, and ancestor-bounded (`$ref` to `global` or ancestor only [`0007:54-61`]). |
| **C17** | `map.md:29-32` | **Standing Preferences:** Adversarial evaluation; test "keep as is" for every candidate; user steer towards workspace split starting with CLI crate [`map.md:33-35`]. | [V] Schema subsystem must justify its 12k LoC against "keep as is"; evaluate decoupling without leaky cross-crate dependencies. |

---

## 2. Handoff Table: Downstream Open Tickets (17–30)

Ticket 16 must resolve what it produces and hands off to all downstream tickets:

| Ticket | What Downstream Ticket Assumes / Defers Regarding Schema & Validation | Hidden Coupling / What Ticket 16's Choice Changes |
|---|---|---|
| **17: Ambient Context** | Assumes context supplies `TimeZone` and `now` for evaluation [`17:40-42`]. Defers temporal validation to execution context. | **Low.** If schema validates date formats against local dates, it must consume `ExecutionContext` rather than ambient clock/zone. |
| **18: Config Lifecycle** | Assumes `[schemas]` config table defines `class_field` and `directory` [`18:facts; survey/config-dialog.md:11`]; notes digest asymmetry (`[schemas].directory` defaults to `.traces/schemas`, `[templates]` has no default [`18:19-20; findings-ledger:19`]). | **Medium.** If Ticket 16 introduces validation modes (e.g. strict vs off) or retires external select files, `[schemas]` config schema changes. |
| **19: Note Document Model** | Assumes `Note` may split into indexing vs editor shapes [`19:33-35`]; byte-span policy for frontmatter/inline fields is open [`19:10-11`]. | **CRITICAL COUPLING (Pre-supposition).** Ticket 19 cannot know whether `Note` AST must retain source `ByteSpan`s for frontmatter keys/values unless Ticket 16 decides whether advisory diagnostics are emitted with AST source spans (Taplo pattern [`research/03:110-111`]). |
| **20: Index Read Model** | Assumes Redb indexes `class_field` as a raw string into `paths_by_file_class` [`20:facts; survey/index.md:49`]. | **CRITICAL COUPLING.** If Ticket 16 chooses *Index-Time Schema Typing*, Redb must store schema-coerced typed values and `IndexStore` must depend on `SchemaService`. If Ticket 16 chooses *Advisory Validation*, Redb remains schema-free. |
| **21: Freshness & Invalidation** | Assumes `IndexFingerprint` covers 10 `config.toml` settings [`21:13-16`]. Notes: *"Decide whether a parser/schema identity enters the fingerprint or the version is derived"* [`21:30-31`]. | **CRITICAL COUPLING (Hidden Trap).** If Ticket 16 chooses *Index-Time Schema Typing*, modifying any `.traces/schemas/*.toml` file **must** invalidate the index and trigger full rebuilds. If Ticket 16 keeps schema passive or advisory, schema changes never invalidate Redb. |
| **22: Query Core & Plan** | Assumes `FileClassExpander` seam (`src/file_class_expander.rs`) expands `@class*` hierarchies [`22:facts; survey/query.md:93`]; filters use adaptive literals [`22:46-48`]. | **High.** If Ticket 16 decides schemas provide typed field projections to queries, 22's query planner needs schema-informed typing. If schemas are passive, query planning relies strictly on Ticket 15's adaptive literals and explicit casts. |
| **23: Temporal Dialects** | Notes that schema `date` fields define an unvalidated `format` string (`src/schema/fields/date.rs:12-14` [`23:20-21`]). | **Medium.** If schema validates note dates against this declared `format`, 16 couples schema validation directly to 23's date formatting engine. |
| **24: Template Boundary** | Assumes ADR-0006: `schema.get("book").field("status")` returns values only [`24:23; ADR-0006:70-77`]; identifies Minijinja `State` temp-cache hack for `Schema` hierarchy access (`src/template/engine/schema.rs:82-90` [`24:15-16`]). | **CRITICAL COUPLING.** Ticket 16 explicitly asks: *"which module owns... the typed-field representation shared with templates (`ui` prompts) and query?"* [`16:13-14`]. If schema fields declare `ui` prompt types, 24 must bind prompt dispatchers; if ADR-0006 stands, schema supplies values only and 24 is untouched. |
| **25: Path & Capability** | Assumes `SafeRelativePath` in `src/schema/fields/select.rs:22` confines external select files to schemas directory [`25:17-19; survey/schema.md:10`]. | **Medium.** If Ticket 16 retires external select files (`fields/select.rs`), the path confinement and caching code in `select.rs` is deleted. |
| **26: Workspace Host** | Assumes CLI composition loads `SchemaService::load_verbose` (`src/cli/mod.rs:490-502` [`26:9-10`]). | **High.** Who constructs and owns `SchemaService`? If validation is a standalone command (`traces check`), `WorkspaceHost` must orchestrate `SchemaService` + `Note` + validator. |
| **27: Diagnostic Architecture** | **EXPLICITLY BLOCKED BY 16.** Asks: *"where schema validation diagnostics (if any) land"* [`27:5, 13-14`]. Notes 5 nested error enums in `schema/fields/error.rs` [`27:facts`]. | **CRITICAL COUPLING.** Ticket 16 dictates whether `SchemaDiagnostic` exists, what fields it carries (code, severity, span, path, message), and whether 27 must accommodate note-validation diagnostics alongside query syntax errors. |
| **28: Crate Topology** | Assumes `schema` is Tier 4 in the 10-tier DAG, importing Tier 0 (`path`, `strsim`) and Tier 3 (`field`, `file`) [`28:facts; survey/cross-cutting.md:35, 117`]. | **CRITICAL COUPLING (Topological Trap).** If `schema` validates `Note` (Tier 5), `schema` must import `note`, creating an upward dependency cycle in the DAG. Ticket 16's placement choice dictates whether a new `analysis` crate or module is needed. |
| **29: Reconcile Decisions** | **BLOCKED BY 16.** Audits schema subsystem and checks whether "Schema internals" fog graduated [`29:5, 13, 16`]. | **High.** 16 must provide clean boundaries and resolved fog so 29 can close the strategic redesign. |
| **30: Buffer Consistency** | Assumes LSP edits exist in unsaved buffers separate from persisted index [`30:9-13`]. | **High.** Advisory schema validation must be capable of running against transient in-memory `Note` ASTs without requiring index persistence. |

---

## 3. Cross-Ticket Inconsistencies & Open Loops

1. **The "Frontmatter String Override" Illusion:**
   - *Tension:* Ticket 15 ruled: *"Frontmatter classification stays schema-free and quote-insensitive... A frontmatter value that must stay text uses the schema override (ticket 16)"* [V: `issues/15:102-106`]. But Ticket 14 mandated: *"note-boundary classification rules owned by note parser, one precedence order"* [V: `issues/14:522-523`].
   - *The Trap:* If note frontmatter parsing is strictly schema-free, the parser eagerly converts `title: 2026-07-29` into `NoteFieldValue::Date`. How can a schema "override" this to text without: (a) passing `SchemaService` into `NoteParser` (violating schema-free note parsing and Tier 4/5 DAG decoupling), (b) mutating stored note values post-parse, or (c) applying a late query-time projection? Ticket 16 must reconcile this contradiction.
2. **Select String vs Link Matching Redundancy:**
   - *Tension:* Ticket 14 recorded: *"Select values that name links stay strings; ticket 16 interprets them at the consumer boundary with the shared wikilink parser"* [V: `issues/14:481-482`]. But Ticket 15 recorded: *"A `Link` row adopts a literal as its folded target... This settles schema select string vs `Link` matching for ticket 16"* [V: `issues/15:122-124`].
   - *The Loop:* Ticket 15 already solved select-string matching via adaptive query literals (`due == "[[Project A]]"` matches `Link("Project A")`). Does Ticket 16 need to do anything further, or is Ticket 14's handoff to 16 completely moot?
3. **ADR-0006 "Inert" Flags vs Active Validation:**
   - *Tension:* ADR-0006 explicitly documented: *"required/multi are declared now but inert until the deferred MCP guardrail stage"* [V: `docs/adr/0006:110-111`] and *"The schema supplies values only; the template author picks the interactive ui.* function"* [V: `docs/adr/0006:76-77`].
   - *The Loop:* Ticket 16 reopens this: should `required` and `multi` be validated against notes today (advisory diagnostics), or do they remain inert until an editor/LSP/MCP milestone?
4. **Index Invalidation Cliff vs Incremental Watchers:**
   - *Tension:* If Ticket 16 chooses index-time schema typing (e.g. coercing note fields into schema-typed variants in Redb), any edit to a schema file invalidates all notes of that class in Redb.
   - *The Loop:* Landed drift detection (`207cc3fb`) invalidates the *entire* index on config drift [`issues/21:18-20`]. Adding schema hashes to `IndexFingerprint` would turn schema edits into full-vault rebuild cliffs.
5. **Topological Tier Inversion for Note Validation:**
   - *Tension:* `src/schema` (Tier 4) has zero imports of `src/note` (Tier 5) [V: `survey/schema.md:37; survey/cross-cutting.md:117-118`].
   - *The Loop:* If `src/schema` owns `validate(&Note, &Schema)`, Tier 4 must import Tier 5, inverting the architectural DAG. If `src/note` owns validation, Tier 5 imports Tier 4. If neither, where does the validator live?

---

## 4. The 'Graduates Fog' List (Map: Schema Internals)

Map `map.md:234-240` lists 6 unsharpened items under "Schema internals" that cannot be settled until Ticket 16 fixes what schema produces:

| Fog Item | Size / Location | What Ticket 16 MUST Settle | What Can Wait Until After Ticket 16 |
|---|---|---|---|
| **1. External Select-Values** | 2,032 LoC (`fields/select.rs:1-2032`) | **Decide in 16:** Whether external `.toml`/`.json` select dictionaries remain supported or are retired in favour of inline lists + index queries (`from_class`). | Internal caching, file watcher invalidation, and data-loading refactoring in `select.rs`. |
| **2. Imperative Option Parsing vs Serde** | 918 LoC (`fields/parser.rs:28-143`) | **Wait:** Internal implementation detail. Option parsing into untyped `IndexMap<String, SchemaLiteral>` can migrate to Serde internally without changing schema capabilities. | Refactoring `SchemaFieldParser` to strongly-typed Serde enum tables. |
| **3. Five Nested Field-Error Enums** | 655 LoC (`fields/error.rs:32-232`) | **Decide in 16:** Define the boundary between *schema authoring errors* (malformed TOML, cycle) and *note validation diagnostics* (type mismatch, missing required). | Collapsing the 5 internal error enums into unified structs (handled by Ticket 27). |
| **4. CSR Adjacency Scale-Fit** | 734 LoC (`graph/adjacency.rs:42-47`) | **Wait:** Verified sound and allocation-free [V: `survey/schema.md:106-108`]. Graph representation does not affect note typing or validation contracts. | Internal simplification to `IndexMap` if desired during specification. |
| **5. Dead `SchemaService::new` Constructor** | 9 LoC (`service.rs:52-60`) | **Wait:** Pure dead-code cleanup (`expect(dead_code)`). | Trivial removal during specification. |
| **6. Template `Schema` Handle Hack** | ~30 LoC (`template/engine/schema.rs:82-90`) | **Decide in 16:** The API contract exposed by `Schema` to consumers (does it carry an internal hierarchy reference or remain a detached value object?). | Minijinja `State` temp-cache elimination (handled by Ticket 24). |

---

## 5. The User's Likely Preferences (Inferred from Earlier Steers)

The user's decisions across tickets 12–15 establish clear, consistent architectural instincts:

1. **Advisory over Gating / Lenient Open-World Vaults:**
   - *Actual words:* In Ticket 14 (out-of-range integers), agreed to *"frontmatter-only config copy with `integer_overflow_errors` off, so an integer above `u64::MAX` becomes an approximate `Float` and does not discard the whole frontmatter block"* [V: `issues/14:436, 494-496`].
   - *Actual words:* In Ticket 15 (filter error handling), agreed to the error-mode principle: *"classification failure keeps the string; explicit casts (`date()`, `dur()`) fail to `Null`; filters never error at evaluation; arithmetic on mismatched kinds is `Null`. Advisory stderr notices"* [V: `issues/15:159-163`]. Explicitly rejected *"erroring filters (hostile to open-world vaults)"* [V: `issues/15:68-69`].
   - *Inference for 16 [H]:* The user will firmly **reject** ingestion-time gating or index-blocking schema validation. They will strongly favour **advisory diagnostics** (warnings on stderr or LSP diagnostics) that never prevent notes from being indexed or queried.
2. **Delete / Simplify over Speculative Keep:**
   - *Actual words:* In Ticket 14, eliminated `FieldValueRef` and duplicate visitor hierarchies: *"FieldValueRef is always immediately owned (zero-copy gain is nil), the chain costs about 41 percent of frontmatter parse time... rejected"* [V: `issues/14:456-458`].
   - *Actual words:* In Ticket 15, deleted sort-time sniffing: *"The sort-time sniffing (`TextShape::classify` in `SortKey`) is deleted, and `List`/`Object` stop being mapped to `Null`"* [V: `issues/15:85-87`].
   - *Actual words:* In Ticket 14, rejected caller counts as the sole metric for deleting end-user features: *"Caller counts alone were rejected as evidence for deleting capabilities, since the crate has end users and a public surface"* [V: `issues/14:332-333`].
   - *Inference for 16 [H]:* The user will favor eliminating the 2.0k-line `fields/select.rs` external file cache complexity if it is shown to be accidental complexity, but will protect user-visible schema capabilities.
3. **Compatibility Freedom & Sole Consumer:**
   - *Actual words:* In Ticket 12: *"The user is the only consumer. No published release, no external Rust crate depends on the library. Every surface is therefore at most Migratable unless it would break the user's own vault files"* [V: `issues/12:24-25`]. Rust library surface is Free [`12:45`].
   - *Inference for 16 [H]:* The user is completely unconcerned with preserving internal Rust APIs in `src/schema/`. They care about not breaking existing Markdown files and keeping `.traces/schemas/*.toml` files clean.
4. **Workspace Split & Decoupling:**
   - *Actual words:* Steer recorded on 2026-10-18: *"consider evolving the codebase toward a workspace layout, starting by separating the CLI into its own crate... reduce visibility wherever possible and limit `pub` to outward-facing components"* [V: `map.md:33-39; issues/12:47`].
   - *Actual words:* In Ticket 14 (domain boundaries): *"Keep schema declarations, note source occurrences, query projections, and template-facing representations separate where their responsibilities differ... schema field types do not silently reinterpret literals. Any such interpretation must be explicit at the consumer boundary"* [V: `issues/14:63-65, 159-161`].
   - *Inference for 16 [H]:* The user will resist any design that tightly couples `schema` to `note` or `index`. They prefer clean, layered interfaces (like `FileClassExpander`) that facilitate a multi-crate workspace split.

---

## 6. Draft Questions Ticket 16 Must Settle (Ordered by Dependency)

### Spine Question 1: What is the operational remit of Schemas regarding notes?

- **Option A (Keep as is / Passive Metadata Provider):** Schemas define field definitions used strictly as passive metadata for templates (supplying select value lists via `schema.get("class").field("prop")`), template reflection (`ui.select`), and query class hierarchy expansion (`@class*` via `FileClassExpander`). Zero note validation; note frontmatter and inline fields remain 100% open-world and schemaless.
- **Option B (On-Demand Advisory Validation Pass — Recommended):** Notes parse and index unconditionally. An independent validator pass (`validate(note, schema) -> Vec<SchemaDiagnostic>`) evaluates note frontmatter/inline fields against resolved schema definitions on demand (for CLI `traces check` / `traces lint` and LSP `textDocument/publishDiagnostics`). Emits structured diagnostics with source byte spans. Never blocks indexing or query execution.
- **Option C (Ingestion-Time Gating / Typed Store):** Notes are validated against schemas during indexing. Notes with schema violations are rejected from Redb or have non-conforming fields nulled. (Breaks open-world tolerance, tightly couples `index` to `schema`, turns schema edits into index wipe cliffs).
- **Option D (Query-Time Schema Projection):** Stored notes remain raw; queries use schema definitions to dynamically project and cast fields (e.g. coercing strings to schema-declared types during query execution).
- *Downstream Blockers:* **Blocks Tickets 19, 20, 21, 22, 26, 27, 28, 30.**

### Question 2: How does the "Frontmatter String Override" operate, and who owns it?

*(Ticket 15:104-106 promised: "A frontmatter value that must stay text uses the schema override (ticket 16)")*

- **Option A (No Runtime Override; Quote-Insensitive Frontmatter Stands):** Frontmatter dates always classify to `Date`/`DateTime`. If an author wants a string, they write an inline field with quotes (`title:: "2026-07-29"`), or write a non-date string. Schema field definitions do not mutate runtime values. (Keeps note parser 100% schema-free).
- **Option B (Query-Time Projection Override):** Note parsing remains schema-free and stores `Date`. During query plan compilation, if a note's schema declares the field as `type = "input"` (string), the query binder projects the value as `String` (or compares it using string semantics).
- **Option C (Parse-Time Schema Sniffing — Rejected Direction):** Pass `SchemaService` into `NoteParser`. If a schema declares the field as string, lexical date sniffing is skipped. (Violates 14 & 15 schema-free note parsing; couples note parser to schema).
- **Option D (Advisory Diagnostic Only):** The value remains a `Date` at runtime; the advisory validator warns: `"Field 'title' has schema type 'input' but contains Date '2026-07-29'"`.
- *Downstream Blockers:* **Blocks Tickets 19, 20, 22.**

### Question 3: Which module owns the validator and where does the diagnostic type live?

- **Option A (`schema::validate`):** `src/schema/` owns `validate(&Note, &Schema) -> Vec<SchemaDiagnostic>`. Inverts the DAG (Tier 4 `schema` must import Tier 5 `note`).
- **Option B (Independent Analysis / Lint Module):** An independent module (e.g. `src/analysis/` or `src/lint/`) sitting at Tier 8+ above `note` and `schema` imports both and implements validation. Keeps `schema` and `note` decoupled.
- **Option C (`note::validate`):** `src/note/` imports `Schema` and validates itself.
- **Option D (No Validator):** If Option A of Q1 is selected, no validator or diagnostic type exists.
- *Downstream Blockers:* **Blocks Tickets 27 (Diagnostic Architecture), 28 (Crate Topology).**

### Question 4: What is the boundary between Schema Field Types and Template `ui` Prompts?

*(Ticket 16 asks: "typed-field representation shared with templates (`ui` prompts) and query?")*

- **Option A (Strictly Decoupled Values-Only — ADR-0006 Baseline):** Schema fields supply values/options only (`field.values()`). The template author explicitly chooses the interactive function (`ui.select(...)`, `ui.text(...)`). Schemas have zero knowledge of `ui` prompt types.
- **Option B (Schema Prompt Helper):** Schema fields declare an optional prompt preference; templates can call `field.prompt()` as a convenience wrapper dispatching to `ui.*`.
- **Option C (Unified FieldDef Enum Across Subsystems):** A heavy, shared enum merging schema types, template prompt widgets, and query filter affinities.
- *Downstream Blockers:* **Blocks Ticket 24 (Template Boundary).**

### Question 5: What is the fate of external select-values files and `fields/select.rs`?

*(Map "Graduates fog" item 1)*

- **Option A (Keep as-is):** Retain external `.toml`/`.json` file caching, path confinement, and 5 nested error types in `fields/select.rs` (~2.0k LoC) for large external taxonomies.
- **Option B (Retire External Select Files — Simplify):** Select fields support only inline string/object lists in `.traces/schemas/*.toml`. Large option datasets are queried via `file` fields (`from_class` / index queries). Deletes ~1,500 LoC and 3 error enums.
- **Option C (Simplify External Loading):** Retain external file references, but strip custom caching and path-confinement gymnastics; read them via standard Serde at `SchemaService::load` time.
- *Downstream Blockers:* **Blocks Ticket 25 (Path & Capability).**

---

## 7. Not Verified

1. **Impact of removing external select files on real user vaults:** [H] It is hypothesized that few vaults use external `.json`/`.toml` select dictionaries, but this has not been surveyed across user vault repositories.
2. **Performance of on-demand validation over 50k notes:** [H] Running `validate(&Note, &Schema)` across 50,000 notes in memory using Rayon is estimated at <50ms, but has not been benchmarked.
3. **Template authors' reliance on ADR-0006 `required`/`multi`:** [H] ADR-0006 documented these as inert, but whether any existing templates inspect `field.required` or `field.multi` reflects unverified template usage.

---

## 8. Questions This Raises for the Grilling

1. *The String Override Paradox:* In Ticket 15 you decided that frontmatter classification is quote-insensitive and schema-free, but noted that a frontmatter value staying text would use the "schema override (ticket 16)". Since note parsing cannot depend on schemas without inverting the module DAG and slowing down ingest, how should this override behave? Should it simply be an advisory diagnostic, a query-time string projection, or do we accept that frontmatter dates are always dates?
2. *Advisory vs Passive:* `src/schema` is 12,000 lines of code, but currently does zero validation of notes. Are you committed to introducing an on-demand advisory validation pass (for `traces check` and LSP), or would you rather keep schemas as passive template/query metadata providers and avoid building a note validation engine altogether?
3. *Select Files Pruning:* `src/schema/fields/select.rs` accounts for 2,032 lines (>16% of the schema subsystem) just to support external `.toml` and `.json` option dictionaries with custom caching and path confinement. Can we delete external select files and require select options to be inline lists in the schema TOML?
4. *Prompt Coupling:* ADR-0006 ruled that schemas supply values only, leaving template authors to pick `ui.*` functions. Ticket 16 raises whether schemas should own a typed-field representation shared with `ui` prompts. Do you want schemas to dictate UI prompt behavior, or keep schemas strictly decoupled from UI dialogs?

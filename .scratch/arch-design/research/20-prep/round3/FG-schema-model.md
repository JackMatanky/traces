# Scout FG: Multi-Class Schema Resolution and External Value Sources

## 1. Sub-Problems Restated

Ticket 20 must determine how Traces resolves schema definitions and provides
them to validation, the LSP, templates, and the MCP agent. This scout
investigates two tightly coupled core engine sub-problems:

- **Sub-Problem F: Multi-Class Effective Field Resolution and Schema Model
  Extensions**:
  A note in Traces may declare multiple classes in frontmatter, e.g.
  `class: [task_project, pkm_zettel]`. Each individual class participates in a
  multi-level `extends` DAG, applies `excludes`, and resolves bounded `$ref`
  overrides (the author's vault contains 31 `extends`, 7 `excludes` statements
  affecting 26 fields, and 43 `$ref` directives across 40 schema files).
  Furthermore, downstream consumers (LSP hover, completion, diagnostics, and
  MCP tool input schemas) require metadata absent from the current model:
  field provenance (`origin`), author documentation (`description`),
  inheritance hierarchy depth (`parent_order`), and a neutral field descriptor
  capable of driving both in-memory checking and a JSON Schema projection.
  When two classes on the same note define a field with the same canonical
  name, what is the conflict policy (first wins, last wins, error, or constraint
  intersection), and how is the effective field map represented?

- **Sub-Problem G: Large and External Value Sources for Select Fields**:
  Select fields in Traces can source selectable options from external JSON/TOML
  files (e.g. `examples/.traces/schemas/values/job_titles.json` containing 3,734
  entries, `industries.json` with 2,108 entries, `cities.json` with 1,675
  entries). Entries map structured attributes (`value`, `label`, `link`, `key`).
  Crucially, several fields configure `value = "link"`, meaning the stored note
  value is a wikilink (e.g. `[[account_collector|Account Collector]]`),
  coupling select validation to Ticket 18's `NoteFieldValue::Link` and Ticket
  19's adaptive literal coercion. Today, loading and cloning these large lists
  costs significant memory and parsing time. How should large vocabularies be
  loaded, cached, invalidated on file edits, validated without runaway memory,
  presented in LSP completion without latency spikes, and bounded in diagnostic
  messages?

---

## 2. What Traces Does Today

### 2.1 Single-Schema Resolution Pipeline [V]

`src/schema/builder.rs:125-228` implements whole-directory schema resolution at
startup via `SchemaService::load_verbose` (`src/schema/service.rs:83-90`).
Resolution proceeds as follows:

1. `resolve_global()` (`src/schema/builder.rs:144-156`): Parses `global.toml`
   as an isolated `$ref` target pool outside the DAG.
2. `SchemaGraphBuilder` (`src/schema/graph/builder.rs:27-71`): Builds a CSR
   adjacency graph of declared `extends` relationships, verifies acyclicity via
   iterative Tarjan DFS (`src/schema/graph/cycle.rs:18-80`), and linearizes
   schemas using Kahn's topological sort (`src/schema/graph/builder.rs:88-125`).
3. `merge_in_topological_order` (`src/schema/builder.rs:167-195`): For each
   schema in topological order, calls `merge()`:
   - `inherit_fields` (`src/schema/builder.rs:233-258`): Copies fields from
     declared parents using **first-listed parent wins**
(`IndexMap::entry().or_insert_with()`).
   - `exclude_fields` (`src/schema/builder.rs:261-268`): Removes field names
     listed in the child's `excludes = [...]`.
   - `resolve_own_fields` (`src/schema/builder.rs:274-295`): Resolves child
     fields and `$ref` pointers (bounded to `global` or transitive ancestors),
     overwriting inherited fields via `IndexMap::insert`.
   - `reject_ambiguous_field_names` (`src/schema/builder.rs:306-323`): Checks
     that no two fields produce identical `FieldKey::canonicalize()` strings.
4. `compute_hierarchy` (`src/schema/builder.rs:325-349`): Populates each
   `Schema`'s `children` and `descendants` sets.

### 2.2 Absence of Multi-Class Note Resolution [V]

While a schema can extend multiple parent schemas, **Traces has zero code to
resolve effective fields for a note with multiple classes**.

- `src/schema/service.rs:36-38` stores schemas keyed strictly by individual
  name: `schemas: IndexMap<SchemaName, Arc<Schema>>`.
- `SchemaService::get(&self, name: &str)` (`src/schema/service.rs:109-111`)
  fetches only a single named `Schema`.
- `SchemaService::matches(&self, classes: &[String])`
  (`src/schema/service.rs:171-185`)
  only expands class names to their transitive descendants for query evaluation
  (`@class(book)` matching `sci_fi`). It performs no field merging.
- Notes record their class as a raw string or string list in frontmatter
  (`src/index/service.rs:1732-1780`), which is indexed directly into Redb table
  `paths_by_file_class` (`src/index/tables.rs:51-52`) without ever consulting
  `SchemaService`.
- If a note has `class: [task, pkm]`, any consumer today must either look at
  only the first class or perform its own ad-hoc merging.

### 2.3 The Schema Field Model Today [V]

`src/schema/fields.rs:52-57` defines:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct SchemaFieldDef {
    kind: SchemaFieldType,
    required: bool,
    multi: bool,
}
```

`SchemaFieldDef` stores only type and boolean flags.

- **No `origin`**: Does not record which schema in the `extends` chain
  originally declared the field.
- **No `description`**: Does not support user documentation strings.
  `ALLOWED_OPTION_KEYS` in `src/schema/raw.rs:18-21` rejects `"description"`.
- **No `parent_order`**: Does not preserve the linearization path of parents.
- **No neutral descriptor**: `SchemaFieldDef` is tightly bound to internal
  `SchemaFieldType` and carries no methods for JSON Schema conversion.

### 2.4 External Select Values Loading and Memory Cost [V]

External select values are handled in `src/schema/fields/select.rs:202-289`:

- `SelectValuesFileCache` loads `.json` or `.toml` files during
  `SchemaService::new`.
- Parsing is transient: raw files are parsed via `serde_json` or `toml` into
  `Vec<FieldValue>`, converted into `Vec<SchemaSelectFieldEntry>`, and embedded
  directly inside `SchemaSelectField { values: Vec<SchemaSelectFieldEntry> }`
  (`src/schema/fields/select.rs:41-47`).
- In `examples/.traces/schemas/values/`, `job_titles.json` has 3,734 entries,
  `industries.json` has 2,108 entries, and `cities.json` has 1,675 entries.
- Each `SchemaSelectFieldEntry` (`src/schema/fields/select.rs:134-139`) contains
  two `FieldValue`s (`value` and `label`) and an `IndexMap<String, FieldValue>`
  for extra attributes (e.g. `key`).
- Because `SchemaFieldDef` is `#[derive(Clone)]` and `SchemaSelectField`
  contains
  a plain `Vec<SchemaSelectFieldEntry>`, **every time a child schema inherits
  a select field, the entire 3,734-entry vector and all its heap allocations are
  deep-cloned** (`src/schema/builder.rs:252`).
- In `src/template/engine/schema.rs:254-266`, invoking `.field("job_title")`
  iterates across all 3,734 entries, wrapping each in a Minijinja `Value`.

### 2.5 Real-Vault Ground Truth [V]

Inspection of `examples/.traces/schemas/` reveals:

- **Total schemas**: 40 TOML files.
- **Extends count**: 31 inheritance links.
- **Multi-parent schemas**: **0**. Every schema in the vault extends at most
  one parent schema (`extends = ["lib"]`, `extends = ["cal"]`). Sibling
  multiple inheritance occurs *only at the note level* when notes assign
  multiple classes.
- **Excludes count**: 7 schemas use `excludes`, eliminating 26 fields total
  (e.g. `cal_day.toml` excludes `date_start` and `date_end`; `lib_webpage.toml`
  excludes 12 fields from `lib`).
- **$ref count**: 43 references to `#global/<field>`.
- **Values files referenced**:
  - `global.city`: `values/cities.json` (1,675 entries, `value="value"`,
    `label="label"`)
  - `dir_contact.job_title`: `values/job_titles.json` (3,733 entries,
    `value="link"`, `label="value"`)
  - `dir_contact.country`: `values/countries.json` (811 entries,
    `value="value"`, `label="label"`)
  - `dir_organization.industry`: `values/industries.json` (2,108 entries,
    `value="link"`, `label="value"`)
  - `dir_organization.country`: `values/countries.json` (811 entries,
    `value="value"`, `label="label"`)
- **Wikilinks in Select Values**: `job_titles.json` and `industries.json` store
  wikilinks as values: `[[account_collector|Account Collector]]`. In
frontmatter,
  these parse as `NoteFieldValue::Link`.

---

## 3. Precedents and Comparative Analysis

### 3.1 Obsidian Metadata Menu: Multi-fileClass Priority & ID Deduplication [V]

- **Mechanism**: Metadata Menu (`mdelobelle/metadatamenu`) allows notes to
  inherit multiple `fileClass` definitions via frontmatter (`fileClass: [a,
b]`),
  tags, folders, or bookmark groups.
- **Conflict Policy**: Frontmatter array order dictates priority: **first
  listed wins**. If `a` and `b` both define field `status`, the definition from
  `a` completely replaces `b`.
- **Failure / Post-Mortem**: Metadata Menu assigned internal random UUIDs
  (`id: "ab3f..."`) to every field in a fileClass note. When users copied
  fields across fileClasses or edited YAML manually, UUID collisions or drift
  caused fields with identical names to overwrite each other unpredictably, or
  disappear from table views. Users reported widespread data loss and broken
  table columns (GitHub issue #656, #550).
- **Lesson**: Field identity must be based on canonical name (`FieldKey`), never
  synthetic IDs. First-listed wins is intuitive to markdown authors but requires
  transparent diagnostic warnings when types conflict.

### 3.2 Dendron: Dot-Notation Path Schemas and Templates [V]

- **Mechanism**: Dendron schemas (`*.schema.yml`) use dot-notation hierarchy
  matching (e.g. `proj.*` or `daily.*.*`). A schema node defines `is-a` parent
  links, child pattern matching, and frontmatter templates.
- **Conflict Policy**: Dendron avoids multiple inheritance entirely. Matching
  is strictly tree-based with prefix specificity: the most specific schema
  pattern matches.
- **Failure / Post-Mortem**: Users frequently needed orthogonal traits (e.g.
  a note that is both a `meeting` and part of `project-alpha`). Dendron's
  inability to compose multiple schemas forced users to create deep, brittle
  combinatorial hierarchies (`project.meeting.standup`), leading to schema
  sprawl and migration pain before Dendron was deprecated.
- **Lesson**: Real PKM vaults demand multi-class composition on notes; pure
  single-inheritance path trees fail real user workflows.

### 3.3 JSON Schema: `allOf`, `$ref`, and `unevaluatedProperties` [V]

- **Mechanism**: JSON Schema (Draft 2020-12) composes schemas using `allOf:
  [...]`.
  Each subschema in `allOf` is evaluated independently as a logical conjunction
  ($A \land B$).
- **Conflict Policy**: **Intersection / Pure Conjunction**. There is no "first
  wins" override. If schema A requires `type: string` and schema B requires
  `type: integer`, `allOf: [A, B]` can never be satisfied; any input is
  rejected.
- **Composition Hazards**: Sibling keywords with `$ref` were ignored in older
  drafts (Draft 7), leading to silent validation drops. In Draft 2020-12,
  `unevaluatedProperties: false` fails unless every property was explicitly
  seen by an in-scope applicator. When combining schemas via `allOf`,
  `unevaluatedProperties` cannot "see" through dynamic references without full
  applicator state tracking.
- **Lesson**: A JSON Schema projection for Traces cannot simply emit
  `allOf: [ClassA, ClassB]`, because Traces semantics allow field overrides and
  first-wins shadowing. The projection must emit the **linearized effective
  fields**.

### 3.4 CUE Unification and Disjunction Lattices [V]

- **Mechanism**: In CUE, types and values belong to the same partial order
  (lattice). Schema composition uses unification (`&`), which is strictly
  commutative and associative ($A \& B == B \& A$).
- **Conflict Policy**: Order does not matter. If `A = { x: string }` and
  `B = { x: "prod" }`, then $A \& B = \{ x: \text{"prod"} \}$. If $A$ and $B$
  specify incompatible types (`int` and `string`), the result is bottom (`_|_`),
  a fatal compilation error.
- **Large Enum Handling**: CUE expresses enums as disjunctions
  (`status: "open" | "closed" | ...`). For large sets (3,000+ items), AST
  traversal becomes slow unless the compiler recognizes the disjunction as a
  closed set and compiles it into an internal hash table.
- **Lesson**: Commutative unification is mathematically elegant for
  configuration
  compilers, but ill-suited for markdown frontmatter where users expect
  one class to take precedence over another without schema-level compile errors.

### 3.5 Python MRO (C3 Linearization) in Dataclasses and Pydantic [V]

- **Mechanism**: Python resolves multiple inheritance via C3 Linearization.
  When a class inherits `class C(A, B)`, attributes are searched in MRO order:
  `C`, then `A`, then `B`, then `object`.
- **Conflict Policy**: **First wins in MRO order**. A's field definition
  overrides B's field definition.
- **Failure / Post-Mortem**: In standard Python `dataclasses`, ordering bugs
  frequently occur when a base class defines a field with a default value and a
  subsequent base class defines a field without a default (`TypeError:
  non-default argument follows default argument`). Pydantic v2 had to rewrite
  its MRO field collector to reconcile conflicting field constraints, validator
  chains, and generic parameters.
- **Lesson**: Multi-inheritance field resolution requires deterministic
  linearization, but inheriting fields without sensible defaults creates
  friction.

### 3.6 GraphQL Interface & Union Merging vs Protobuf Composition [V]

- **GraphQL**: An object type can implement multiple interfaces (`type Task
  implements Node & Described`).
  - *Conflict Policy*: **Strict Covariance / Identical Error**. If two
    interfaces define field `title`, the object must declare `title` with a
    type that is a valid subtype of both. If Interface A specifies `title:
    String!` and Interface B specifies `title: Int!`, GraphQL rejects the schema
    at build time.
- **Protobuf**: Protocol Buffers explicitly **forbids inheritance**. Messages
  compose through sub-messages (`optional SubMessage foo = 1;`) or `oneof`.
  Protobuf v3 eliminated required fields entirely because field evolution
  across multiple teams caused breaking changes and un-parseable payloads.
- **Lesson**: Strict interface merging prevents runtime type surprises, but is
  too rigid for personal notes where authors casually tag a note with two
  loosely related classes.

### 3.7 TypeScript: Interface Extends vs Type Intersection [V]

- `interface C extends A, B`: If `A.x: string` and `B.x: number`, TypeScript
  emits compile error `Interface 'C' cannot simultaneously extend types 'A'
  and 'B'`.
- `type C = A & B`: Compiles successfully, but `C.x` becomes `string & number`,
  which evaluates to `never`. Any assignment to `c.x` produces a type error.
- **Lesson**: Intersection types in programming languages result in `never`
  when primitive types clash. Traces must not produce uninhabitable fields; it
  must choose a winning definition and emit an advisory warning.

### 3.8 PostgreSQL: Table Inheritance (`INHERITS`) [V]

- **Mechanism**: `CREATE TABLE c () INHERITS (a, b);`.
- **Conflict Policy**: Columns with the same name are merged into a single
  column. Data types must match identically (`ERROR: table "c" has conflicting
  inherited column "x"`). `NOT NULL` constraints are combined (`OR` logic: if
  either parent is `NOT NULL`, the child column is `NOT NULL`).
- **Post-Mortem**: Postgres table inheritance proved so troublesome (duplicate
  indexes, foreign key caveats, check constraint merging) that PostgreSQL
  introduced declarative partitioning in version 10 as a replacement.
- **Lesson**: Physical inheritance of schemas across multiple parents is a
  historic source of bugs. A clean runtime projection is superior to complex
  relational inheritance.

### 3.9 Summary of Conflict Policies

| System | Conflict Policy | Type Mismatch Action | Rationale |
|---|---|---|---|
| **Metadata Menu** | First wins in frontmatter array | Silent replacement | Markdown author convenience |
| **Python MRO** | First wins in C3 order | First type wins | Deterministic class hierarchy |
| **Postgres INHERITS** | Column merge | Fatal SQL error | Relational type safety |
| **TypeScript extends** | Error on conflict | Compile error | Static soundness |
| **TypeScript intersection**| Constraint intersection | Produces `never` | Set-theoretic consistency |
| **GraphQL** | Interface overlap | Build-time validation error | API contract safety |
| **CUE** | Lattice unification | Produces bottom (`_|_`) | Commutative determinism |
| **JSON Schema `allOf`** | Conjunction | Uninhabitable schema | Logical AND validation |

---

## 4. Precedents for Large External Value Lists

### 4.1 LSP: `CompletionList.isIncomplete` and `completionItem/resolve` [V]

- **Mechanism**: The Language Server Protocol specification provides two
  mechanisms for large completion payloads:
  1. `CompletionList { isIncomplete: true, items: [...] }`: When returning
     a truncated set (e.g. 50 items out of 3,734), setting `isIncomplete: true`
     tells the client (VS Code, Neovim) that filtering locally on further
     keystrokes is insufficient; the client MUST re-query the LSP server with
     the updated prefix.
  2. `completionItem/resolve`: The initial completion response sends only
     lightweight labels and insert text. When the user scrolls through the
     menu, the client queries `completionItem/resolve` to fetch heavy markdown
     `documentation` and details on demand.
- **Lesson**: Traces LSP must never send 3,734 full completion items on an
  empty trigger. It must return a bounded prefix slice (`isIncomplete = true`)
  and resolve documentation lazily.

### 4.2 VS Code Settings: `enum`, `enumItemLabels`, and `$ref` [V]

- **Mechanism**: In `package.json` configuration schemas, VS Code supports
  `enum` (actual serialized values) and `enumItemLabels` (matching
human-readable
  labels in the UI dropdown).
- **Failure**: When schemas embed multi-thousand-item enums directly inside
  `settings.json` validation schemas, VS Code's schema validator allocates
  megabytes of AST nodes and keystroke latency in the editor increases
  measurably. VS Code documentation recommends splitting large schemas across
  `$ref` files and caching them.
- **Lesson**: Do not inline thousands of enum strings into generated JSON
  Schema files.

### 4.3 Notion and Airtable: Select vs Relation / Linked Records [V]

- **Notion**: Offers `Select` (closed local list of tags) and `Relation`
  (links to pages in another database). Notion degrades and warns when a
  `Select` property exceeds ~100 options; users are directed to use `Relation`.
- **Airtable**: Single Select documentation explicitly advises: *"If you have
  more than 50 options or options that change frequently, use a Linked Record
  field to another table instead of a Single Select field."*
- **Lesson**: A 3,734-entry list of job titles where values are wikilinks
  (`[[account_collector|Account Collector]]`) is **not a select field in domain
  reality; it is a Relation to uncreated entities**. Traces supports this today
  via external select files, but architecturally it sits at the boundary
  between controlled vocabularies and graph relations.

### 4.4 PostgreSQL: Enum Types vs Foreign Key Lookup Tables [V]

- **`CREATE TYPE mood AS ENUM (...)`**: Enums are stored as 4-byte OIDs. Fast,
  compact. But adding values requires transaction locks; removing values
  requires dropping and recreating the type. Hard limit of 64KB per enum.
- **Lookup Tables (`REFERENCES lookup_table(code)`)**: Scalable to millions
  of entries, dynamic additions via simple `INSERT`, supports aliases, labels,
  and localized names.
- **Lesson**: Large static dictionaries belong in indexed lookup tables, not
  hardcoded type enums.

---

## 5. Menu of Candidate Solutions

### Candidate 1: Keep-as-Is (Single-Schema Resolution Only, Ad-Hoc Callers)

- **Model**: `SchemaService` continues storing only single `Schema`s.
  `SchemaFieldDef` remains `{ kind, required, multi }`.
- **Multi-Class**: Consumers (LSP, validator) inspect only the first class in
  `note.classes()`, or each consumer writes its own ad-hoc loop merging
  `schema.fields()`.
- **Values**: External values files are parsed into
  `Vec<SchemaSelectFieldEntry>`
  at construction and deep-cloned into every inheriting schema.
- **Input to Checker**: `fn validate(note: &Note, schema: &Schema)`. Multi-class
  notes are either unvalidated or repeatedly validated per-class.

### Candidate 2: Immutable Cached `ResolvedClassSet` with Enriched Field

Descriptors

- **Model**: Introduce `ResolvedClassSet` (or `ResolvedFields`) representing
  the effective field definitions for a specific combination of classes.
  `SchemaService` maintains a memoized cache `IndexMap<ClassSetKey,
Arc<ResolvedClassSet>>`,
  where `ClassSetKey` is a sorted, deduplicated tuple of class names.
- **Multi-Class Resolution**: Computed on demand or at startup for all class
  combinations observed in the vault index:
  1. Iterate classes in declared note order (**first-listed class wins**).
  2. For each class, traverse its linearized `extends` ancestors.
  3. Combine fields into an `IndexMap<FieldKey, FieldDescriptor>`.
  4. If a field name is defined by two classes with conflicting types, first
     class's type wins, and a `SchemaWarning::ConflictingClassField` advisory
     diagnostic is recorded.
- **Field Model Additions**: `FieldDescriptor` (or enriched `SchemaFieldDef`)
  carries:

  ```rust
  pub struct FieldDescriptor {
      pub key: FieldKey,
      pub canonical_name: FieldName,
      pub kind: SchemaFieldType,
      pub required: bool,
      pub multi: bool,
      pub origin: SchemaName,
      pub description: Option<String>,
      pub parent_order: Vec<SchemaName>,
      pub select_lookup: Option<Arc<SelectLookupTable>>,
  }
  ```

- **External Values (`SelectLookupTable`)**: Wrap external select values in an
  `Arc<SelectLookupTable>` backed by an `IndexSet<String>` (or hash table) for
  $O(1)$ membership checking, an index of display labels, and a folded wikilink
  lookup map. When schemas inherit the field, they clone only the `Arc`.
- **Input to Checker**: `fn validate(fields: &FrontmatterMap, defs:
  &ResolvedClassSet)`.
- **JSON Schema Projection**: Directly projects the flattened `defs` into
  `properties: { ... }` with `description`, `type`, and bounded enums.

### Candidate 3: Per-Field Lazy Resolution on Demand

- **Model**: No class-set merging. `SchemaService` exposes a querying method:
  `fn resolve_field(&self, classes: &[String], field: &FieldKey) ->
Option<FieldDescriptor>`.
- **Resolution**: When checking frontmatter, the validator iterates note
  keys, querying `resolve_field` for each key. `resolve_field` checks class 0
  (walking its ancestors), then class 1, stopping at the first match.
- **Values**: External values files are loaded lazily on the first field query
  and cached in an internal `RwLock<HashMap<PathBuf, Arc<SelectLookupTable>>>`.
- **Input to Checker**: `fn validate<R: FieldResolver>(fields: &FrontmatterMap,
  resolver: &R)`.

### Candidate 4: Unconventional: External Select Lists as Tabular Query Sources

- **Model**: Deconstruct `select` fields with external files into virtual
  data tables. Instead of parsing JSON/TOML into the schema subsystem, treat
  `values/job_titles.json` as a registered Read-Only Table in the query/index
  subsystem.
- **Schema Declaration**: The schema field stores:
  `values = { source = "table", path = "values/job_titles.json", value = "link",
label = "value" }`.
- **Resolution & Validation**: The schema compiler does not load entries into
  memory. Validation checks membership by querying the table index (or a bloom
  filter in redb). LSP completion queries the table using prefix search and
  `isIncomplete: true`.
- **Values vs Link**: If `value = "link"`, the field is automatically treated
  as a virtual relation to the entity namespace.

### Candidate 5: Virtual Synthesized Schemas in the DAG

- **Model**: When a note declares multiple classes `[task, pkm]`,
  `SchemaService` synthesizes an anonymous virtual schema node
  `_class_task+pkm` with `extends = ["task", "pkm"]`.
- **Resolution**: The synthesized schema passes through the exact same
  `SchemaBuilder::merge` and `inherit_fields` logic already implemented in
  `src/schema/builder.rs:211-258`. First-listed parent wins, `excludes` apply,
  and canonical ambiguity detection runs automatically.
- **Caching**: The synthesized `Schema` is stored in `SchemaService.schemas`
  keyed by its compound name.
- **Input to Checker**: `fn validate(note: &Note, schema: &Schema)`. Reuses
  the exact single-schema validation signature without changing checker types.

---

## 6. Strongest Argument Against Each Candidate

### Against Candidate 1 (Keep-as-Is)

- **Verdict**: Completely unviable for ticket 20's committed goals.
- **Fatal Flaw**: Multi-class notes are pervasive in real vaults
  (`examples/.traces/schemas`
  demonstrates notes that combine organizational and domain schemas). Forcing
  every consumer to either ignore secondary classes or write ad-hoc merging
loops
  duplicates graph traversal logic across the LSP, template engine, and MCP.
  Furthermore, deep-cloning 3,734-entry vectors across schema inheritance wastes
  megabytes of RAM on every startup.

### Against Candidate 2 (Immutable Cached `ResolvedClassSet`)

- **Verdict**: Strongest general candidate, but requires careful cache bounding.
- **Objection**: Introducing `ResolvedClassSet` creates a secondary data
  structure
  alongside `Schema`. If a vault has notes with dozens of unusual class
  combinations, the class-set cache could grow unbounded unless populated only
  from classes observed during indexing. Cache invalidation on schema file
  edits must invalidate both individual schemas and all cached class sets.

### Against Candidate 3 (Per-Field Lazy Resolution)

- **Verdict**: Poor fit for validation, completion, and JSON Schema generation.
- **Objection**: Lazy resolution excels when inspecting one field, but fails
  catastrophically for operations that must know the *whole* schema:
  1. `missing-required` validation must enumerate all required fields across all
     classes; lazy lookup cannot do this without traversing all parent graphs.
  2. Suggested-but-unset completion (`Schema::fields() - Note::fields()`)
     requires
     full enumeration.
  3. Generating a JSON Schema projection for MCP `inputSchema` requires an
     exhaustive property map.
  Lazy resolution would re-traverse the DAG dozens of times per keystroke.

### Against Candidate 4 (Select Lists as Tabular Query Sources)

- **Verdict**: Violates architectural boundaries and introduces heavy machinery.
- **Objection**: Coupling schema validation to the query engine or database
  tables violates the principle that `schema` is an isolated, synchronous
  metadata compiler. Loading a 3,734-item JSON list into Redb or a database
  table engine adds transaction overhead, lock contention, and cross-crate
  dependencies for what is essentially a static 100KB lookup dictionary.

### Against Candidate 5 (Virtual Synthesized Schemas in the DAG)

- **Verdict**: Elegant reuse of `inherit_fields`, but corrupts the schema model.
- **Objection**: Synthesizing fake schemas in the DAG pollutes `SchemaService`:
  1. `Schema::name()` now returns synthetic strings like `_class_task+pkm`.
  2. Hierarchy queries (`children()`, `descendants()`) become confused by
     ephemeral synthetic nodes.
  3. Schema TOML author diagnostics and go-to-definition cannot map a synthetic
     compound schema back to a single source file on disk.

---

## 7. Deep Dive: The Schema Model Additions

The LSP (Tickets 23, 24, 30) and MCP require specific extensions to the schema
model. Below is the technical specification for each addition:

### 7.1 Provenance (`origin: SchemaName`) [P]

- **Need**: When hovering a field in frontmatter, or invoking go-to-definition,
  the user must jump to the TOML file that *defined* the field, not the leaf
  class of the note. For example, in a note with `class: task_project`, field
  `due_do` originated in `global.toml` and was inherited via `task.toml`.
- **Design**:
  - `FieldDescriptor` stores `origin: SchemaName`.
  - When `resolve_own_fields` builds a direct field, it stamps
    `origin = self.name`.
  - When `inherit_fields` copies a field from a parent, it preserves the
    parent's
    `origin`.
  - When a field overrides a parent field via `$ref`, `origin` records the
    schema declaring the override, while an optional `ref_target:
Option<FieldAddress>`
    records the upstream definition.
  - In `PartialEq`, `origin` is ignored so structural tests remain clean.

### 7.2 Author Documentation (`description: Option<String>`) [P]

- **Need**: LSP hover cards and MCP tool parameter descriptions must display
  human-readable explanation of field semantics.
- **Design**:
  - Add `description = "..."` to `RawSchemaFieldDef` and `ALLOWED_OPTION_KEYS`.
  - Deserialized into `Option<String>` on `RawSchemaFieldDef`.
  - Propagated to `FieldDescriptor.description`.
  - Inheritance rule: If a child schema declares a description, it overrides
    the parent; if omitted, it inherits the parent's description.

### 7.3 Linearization Trace (`parent_order: Vec<SchemaName>`) [P]

- **Need**: Hover cards display the resolution chain:
  `due_do (from task -> extends global)`.
  When a multi-class conflict occurs, diagnostics report:
  `Field 'status' from class 'task' shadows definition in class 'project'`.
- **Design**:
  - Recorded during class-set linearization. Stores the exact sequence of
    schemas traversed to resolve the field.

### 7.4 Neutral Field Descriptor (`FieldDescriptor`) [P]

- **Need**: Bridge the gap between internal TOML AST representations and
  external consumers (LSP hover markdown, completion items, JSON Schema
  projection).
- **Interface**:

  ```rust
  pub struct FieldDescriptor {
      pub key: FieldKey,
      pub canonical_name: FieldName,
      pub kind: SchemaFieldType,
      pub required: bool,
      pub multi: bool,
      pub origin: SchemaName,
      pub description: Option<String>,
      pub parent_order: Vec<SchemaName>,
      pub select_lookup: Option<Arc<SelectLookupTable>>,
  }

  impl FieldDescriptor {
      /// Formats markdown text for LSP hover documentation.
      pub fn to_hover_markdown(&self) -> String;

      /// Projects the field into a JSON Schema property object.
      pub fn to_json_schema_property(&self) -> serde_json::Value;
  }
  ```

---

## 8. Deep Dive: External Value Lists Architecture

### 8.1 The `SelectLookupTable` Structure [P]

To eliminate the 3,734-entry vector cloning and linear search performance
penalties, external select values should be compiled once into an immutable,
shared `SelectLookupTable`:

```rust
#[derive(Debug)]
pub struct SelectLookupTable {
    /// Confined relative path to the values file.
    source_path: PathBuf,
    /// Fast O(1) set of raw stored values for validation.
    values_set: HashSet<String>,
    /// Case-folded wikilink targets for Ticket 19 adaptive literal matching.
    /// Maps "account_collector" -> index in entries.
    folded_targets: HashMap<String, usize>,
    /// Preserved display entries for completion and UI rendering.
    entries: Vec<SelectEntry>,
}

#[derive(Clone, Debug)]
pub struct SelectEntry {
    pub value: String,
    pub label: String,
    pub is_link: bool,
    pub extra: IndexMap<String, FieldValue>,
}
```

### 8.2 Memory & Allocation Impact [V]

- **Current cost**: 5 schemas inheriting `job_titles.json` allocate 5 separate
  `Vec<SchemaSelectFieldEntry>` containing 3,734 heap-allocated items each
  ($5 \times 3,734 \approx 18,670$ entry allocations).
- **With `Arc<SelectLookupTable>`**: Exactly **1** allocation of the table
  loaded into `SchemaService`. Schemas and `ResolvedClassSet`s hold only an
  `Arc<SelectLookupTable>` (8 bytes). Cloning a schema during graph construction
  costs an atomic increment instead of 3,734 heap clones.

### 8.3 Staleness and LSP File-Watching [H]

- In an LSP session, a user might edit `values/job_titles.json`.
- The LSP server registers `workspace/didChangeWatchedFiles` on
  `**/*.{toml,json}`
  in the schemas directory.
- When an external values file changes, `SchemaService` reloads only the
  affected `SelectLookupTable` in its cache, or triggers a lightweight full
  service reload (<5ms for 40 schemas).

### 8.4 Bounded Diagnostics for Large Enums [P]

- If a note has `job_title: "Astronaut"` (invalid value):
  - Emitting all 3,734 valid options in the diagnostic message blows up LSP
    JSON-RPC payloads (hundreds of kilobytes) and exhausts LLM token windows in
    MCP results.
  - **Bounded rule**: The diagnostic message reports:
    `Value 'Astronaut' is not a valid option for 'job_title'. Expected one of
3,734 options (e.g. 'Accountant', 'Actor', 'Actuary'...).`
  - For lists with $\le 10$ items: list all items.
  - For lists with $> 10$ items: list the top 3 closest Levenshtein matches or
    the first 3 alphabetical items, plus total count.

### 8.5 The 'Value vs Link' Split (Ticket 18/19 Integration) [V]

- In `dir_contact.toml`:
  `values = { path = "values/job_titles.json", value = "link", label = "value"
}`.
- An entry in `job_titles.json`:
  `{ "key": "accountant", "value": "Accountant", "link":
"[[accountant|Accountant]]" }`.
- When frontmatter contains `job_title: [[accountant|Accountant]]`:
  1. Frontmatter parser produces `NoteFieldValue::Link(Link)`.
  2. Validator checks membership. It extracts the link target (`accountant`),
     folds case/whitespace per Ticket 19, and probes
`SelectLookupTable::folded_targets`.
  3. Match succeeds in $O(1)$ time without string concatenation or re-parsing.

---

## 9. Evaluation of Checker Input Shape (`defs` Parameter)

The validation kernel's API shape depends directly on how effective fields are
resolved:

| Candidate | Checker Signature | Pros | Cons |
|---|---|---|---|
| **Candidate 1 (Keep-as-is)** | `validate(note, schema)` | Simple, exists | Fails multi-class notes; forces callers to loop |
| **Candidate 2 (ResolvedClassSet)** | `validate(frontmatter, &ResolvedClassSet)` | Pure, zero-copy, fast, decouples note AST from schema | Needs class-set cache in `SchemaService` |
| **Candidate 3 (Lazy Resolver)** | `validate(frontmatter, &impl FieldResolver)` | Minimal upfront memory | Inefficient for `missing-required` & full scans |
| **Candidate 4 (Table Query)** | `validate(frontmatter, &defs, &TableContext)` | Scales to millions | Breaks pure function model; async/IO contagion |
| **Candidate 5 (Virtual Schema)** | `validate(note, &Schema)` | Signature unchanged | Pollutes schema registry with synthetic names |

**Finding**: `defs: &ResolvedClassSet` (Candidate 2) provides the cleanest,
fastest interface. The validation kernel takes only the note's parsed
frontmatter map and the pre-resolved `ResolvedClassSet`, executing as a pure,
allocation-free in-memory check.

---

## 10. Evaluation of JSON Schema Projection

How does each model project to standard JSON Schema (Draft 2020-12) for MCP
`inputSchema` and VS Code frontmatter validation?

### 10.1 Why Raw `allOf` Fails [V]

If a note has `class: [task_project, pkm_zettel]`, projecting raw schemas as:

```json
{
  "allOf": [
    { "$ref": "schemas/task_project.json" },
    { "$ref": "schemas/pkm_zettel.json" }
  ]
}
```

fails whenever:

1. `task_project` excludes a field that `pkm_zettel` or an ancestor defines:
   JSON Schema `allOf` has no mechanism to exclude fields from a referenced
   schema.
2. Two classes define the same field with different constraint sets:
   `allOf` enforces strict conjunction. If class A has `type: string` and
   class B has `type: integer`, the JSON Schema is completely uninhabitable.
3. `unevaluatedProperties: false` is desired: `unevaluatedProperties` cannot
   inspect through complex `$ref` chains across dynamic boundaries in many
   standard validators.

### 10.2 The Flattened Projection Model [P]

The JSON Schema projection must be generated from the **linearized
`ResolvedClassSet`**:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Frontmatter for [task_project, pkm_zettel]",
  "type": "object",
  "properties": {
    "title": {
      "type": "string",
      "description": "Note title (inherited from pkm.toml)"
    },
    "due_do": {
      "type": "string",
      "enum": ["do", "due"],
      "description": "Task urgency mode (inherited from task.toml)"
    },
    "job_title": {
      "type": "string",
      "description": "Job title wikilink (3,734 allowed options)",
      "$comment": "Select values from values/job_titles.json"
    }
  },
  "required": ["title"]
}
```

- For small select lists ($\le 100$ items): emit standard `enum: [...]`.
- For large external select lists ($> 100$ items, like `job_titles.json`):
  do NOT emit 3,734 enum strings into the JSON Schema. Instead, emit
  `"type": "string"` with a `"$comment"` or custom `"x-traces-values-source":
  "values/job_titles.json"`. This keeps the JSON Schema lightweight (<5KB)
  and prevents VS Code / LLM context window exhaustion.

---

## 11. Ranked Recommendation and Evidence That Would Change It

### 11.1 Ranked Recommendation

1. **Rank 1: Candidate 2 (Immutable Cached `ResolvedClassSet` with Enriched
   `FieldDescriptor` and `SelectLookupTable`)**
   - *Why*: It completely solves multi-class resolution with deterministic
     first-listed-wins semantics; eliminates the 3,734-entry vector cloning
     via `Arc<SelectLookupTable>`; equips the LSP with `origin`, `description`,
     and `parent_order`; gives the checker an allocation-free `defs` input; and
     cleanly drives lightweight JSON Schema projections.
2. **Rank 2: Candidate 5 (Virtual Synthesized Schemas in DAG)**
   - *Why*: Reuses existing `src/schema/builder.rs` logic without introducing
     new resolution structs, but rejected because synthetic names pollute the
     schema registry and break file-level go-to-definition.
3. **Rank 3: Candidate 3 (Per-Field Lazy Resolution)**
   - *Why*: Saves memory if only single fields are queried, but severely
     degrades LSP completion and full-file validation performance.
4. **Rank 4: Candidate 4 (External Lists as Tabular Query Sources)**
   - *Why*: Over-engineered; violates module decoupling between schema and
     query.
5. **Rank 5: Candidate 1 (Keep-as-Is)**
   - *Why*: Fails to support multi-class notes, leaves LSP requirements
     unmet, and wastes RAM.

### 11.2 Conflict Resolution Policy Recommendation [P]

For same-named fields across multiple classes on a note:

1. **First-Listed Class Wins**: The field definition from the class appearing
   earlier in the note's `class: [...]` list completely takes precedence for
   validation, completion, and hover.
2. **Advisory Diagnostic on Incompatible Types**: If class A defines `status` as
   `select` and class B defines `status` as `number`, the checker treats it as
   `select` (A wins) but emits an advisory `schema.conflicting-class-field`
   Information/Hint diagnostic to notify the author of the collision.
3. **Additive Required and Multi Flags**: If *either* class declares
   `required = true`, the effective field is required. If either declares
   `multi = true`, multiple values are permitted.

### 11.3 What Evidence Would Change This Recommendation

- If benchmarking in a 50,000-note vault demonstrates that class-set memoization
  consumes significant RAM, switch to lazy on-demand caching with a LRU cap.
- If real users demand commutative union where class array order does not
  matter,
  evaluate CUE-style lattice intersection (though this would break existing
  field overrides).
- If external JSON values files grow to $>100,000$ entries, Candidate 4
  (indexing
  in Redb / SQLite) becomes mandatory.

---

## 12. Transferable Lessons for Other Sub-Problems

1. **For Sub-Problem E (Violation Model & Checker API)**:
   The checker should take `defs: &ResolvedClassSet` rather than `&Schema`.
   Violations for invalid select values must truncate large expected sets to
   prevent unbounded diagnostic payloads.
2. **For Ticket 23 (Byte Spans & AST Split)**:
   The validation kernel needs only frontmatter values and `ResolvedClassSet`.
   It does not need byte spans. Byte spans are attached at the diagnostic
   envelope adapter boundary.
3. **For Sub-Problem A/B (Module Seam & Crate Split)**:
   `ResolvedClassSet` and `FieldDescriptor` belong inside `src/schema/`.
   They require zero dependencies on `src/note/` or `src/index/`. The validator
   lives in a consumer layer (`src/analysis/` or `src/check/`) that imports both
   `schema` and `note`.
4. **For Ticket 19 (Coercion & Adaptive Literals)**:
   `SelectLookupTable`'s folded target map allows instant $O(1)$ verification
   of wikilink note values against string select options, cleanly fulfilling
   Ticket 19's adaptive literal mandate.

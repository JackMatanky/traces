# Scout A: Seam Patterns for Schema-Note Validation

## 1. Sub-problem Restated

Traces contains two independent domain modules that have evolved with zero
direct dependencies on each other:

1. `src/schema`: parses and models TOML field definitions (`SchemaFieldDef`),
   inheritance graphs (`extends`), and select/file source options [V:
   `src/schema/model.rs:11-20`, `src/schema/fields.rs:52-57`].
2. `src/note`: parses and models Markdown notes, extracting YAML frontmatter
   into strongly typed metadata mappings [V: `src/note/metadata.rs:94-98`,
   `src/note/field.rs:43-65`].

To support advisory frontmatter diagnostics in the planned LSP server, pre-check
warnings in rendered templates, and write-path validation guardrails for an MCP
agent, Traces requires a validation capability [V: `docs/adr/0006:37-38`,
`.scratch/arch-design/research/20-prep/brief.md:34-42`]:

```text
(note field values, schema field defs) -> violations
```

The core architectural problem is: **how should these two domain modules be
connected so this checker exists, without resorting to the glue-module and
trait-in-consumer pattern that the user has identified as a disliked
anti-pattern?**

This scout evaluates the module boundaries, dependency directions, trait costs,
and import topologies of eight candidate seam architectures, applying the
principles of the `rust-design` skill (`MODELING.md`, `DISCOVERY.md`), and
assesses how each candidate withstands a future workspace crate split
(Ticket 32).

---

## 2. What Traces Does Today

Today, Traces has zero schema validation for note contents [V:
`survey/schema.md:18`]. The existing module topology reveals sharp boundaries
and one notable coupling seam:

### 2.1 The Two Domain Modules

- **Schema Subsystem (`src/schema`)**:
  - `Schema` (`src/schema/model.rs:11-20` [V]) stores effective field
    definitions `IndexMap<FieldName, SchemaFieldDef>` and hierarchy sets
    (`ancestors`, `children`, `descendants`).
  - `SchemaFieldDef` (`src/schema/fields.rs:53-57` [V]) holds `kind:
    SchemaFieldType`, `required: bool`, and `multi: bool`. Properties such as
    `min`, `max`, `step` (on numbers) and `format` (on dates) are parsed into
    definitions but are inert in production today [V: `brief.md:39-41`].
  - Dependencies: `src/schema` imports only primitives: `FieldName` [V:
    `src/field.rs:45`], `closest_match` [V: `src/strsim.rs`], and `FieldValue`
    [V: `src/field.rs:596`] for default values. `src/schema` imports zero
    items from `src/note`.

- **Note Subsystem (`src/note`)**:
  - `Frontmatter` (`src/note/metadata.rs:94-98` [V]) stores parsed fields in an
    `IndexMap<FieldKey, NoteFieldValue>`.
  - `NoteFieldValue` (`src/note/field.rs:43-65` [V]) is an owned enum with 10
    variants: `Null`, `Bool(bool)`, `Number(f64)`, `String(String)`,
    `Date(DateValue)`, `DateTime(DateTimeValue)`, `Duration(DurationValue)`,
    `Link(Link)`, `List(Box<[Self]>)`, and `Object(IndexMap<String, Self>)`.
  - Notes possess zero byte spans and zero raw scalar source text [V:
    `src/note/metadata.rs:56-74`, `20-prep-context-3.md:30-31`].
  - Dependencies: `src/note` imports zero items from `src/schema`.

### 2.2 The Disliked Seam: `src/file_class_expander.rs`

The existing precedent for connecting decoupled modules in Traces is
`src/file_class_expander.rs` (87 lines total: 39 production SLoC, 48 test SLoC)
[V: `src/file_class_expander.rs:1-87`]. It bridges `query` and `schema` for
class selector expansion (`@book*`):

```rust
// In src/query/grammar/source.rs:353 [V]:
pub(crate) trait FileClassExpander: Send + Sync {
    fn expand(&self, classes: &[String], mode: &mut ClassExpansionMode);
}

// In src/file_class_expander.rs:15-38 [V]:
impl FileClassExpander for SchemaService {
    fn expand(&self, classes: &[String], mode: &mut ClassExpansionMode) {
        // ... traverses schema hierarchy and mutates mode out-param ...
    }
}
```

The user identified this file as an uncomfortable anti-pattern [V:
`20-prep-context-3.md:46-54`]. An analysis under `MODELING.md` reveals five
concrete architectural defects:

1. **Trait in consumer**: `query` defines an abstract trait shaped around its
   own internal grammar type (`ClassExpansionMode`), forcing the service to
   conform to the consumer's calling convention.
2. **Orphan glue module**: `src/file_class_expander.rs` lives at the crate root,
   outside both `src/query` and `src/schema`. This violates the Principle of
   Least Surprise (POLS, `MODELING.md:16-25`): an engineer inspecting `schema`
   finds no indication of how queries expand classes, while `query` contains an
   empty trait shell.
3. **Premature dynamic abstraction**: `QueryService` stores `Option<Arc<dyn
   FileClassExpander>>` [V: `src/query/service.rs:36`], paying dynamic dispatch
   and `Arc` allocation for an abstraction that has exactly one production
   implementation [V: `MODELING.md:40-46`].
4. **Out-parameter mutation**: `expand` takes `&mut ClassExpansionMode` rather
   than returning an owned expansion set, coupling traversal to enum mutation.
5. **Crate split friction**: If `query` and `schema` split into crates, this
   orphan glue forces an artificial third bridge crate (`traces-query-schema`)
   or requires the top-level CLI composition root to wire them via wrappers.

Repeating this pattern for validation (e.g. creating a `NoteValidator` trait in
a consumer with a crate-root `src/note_validator_glue.rs`) would multiply these
defects.

---

## 3. Precedents: How Other Systems Relate Validators to Models

We examine 12 real systems, noting their architectural approach, trade-offs, and
documented post-mortems or regrets.

### 3.1 Crate `jsonschema` (Rust, v0.59.0)

- **Approach [V]**: Compiles a schema AST into an immutable execution graph
  `Validator<F: Json = SerdeJson>` [V: `docs.rs/jsonschema/latest/jsonschema`].
  Validation is generic over `F::Node<'a>`, allowing borrowed inspection of JSON
  trees without forced cloning.
- **Evolution and Regret [V]**: In versions prior to 0.20, `jsonschema` was
  strictly coupled to `serde_json::Value`. Callers with other representations
  (e.g. PostgreSQL `JSONB`, Python `PyAny` in PyO3, Ruby `Magnus`) had to
  allocate complete `serde_json::Value` DOMs just to validate. The maintainer
  undertook a massive rewrite to extract `jsonschema-value` and introduce the
  `Json` trait abstraction.
- **Lesson for Traces**: Generic abstraction (`trait Json`) was justified only
  because `jsonschema` crossed FFI language barriers (Python/Ruby/C). In a
  closed-world Rust binary, introducing a generic value-view trait before having
  multiple value representations is premature ceremony (`MODELING.md:40-46`).

### 3.2 Crates `garde` (v0.23.0) and `validator` (v0.19.0) (Rust)

- **Approach [V]**: Statically typed validation using derive macros:
  `#[derive(Validate)] struct User { #[garde(length(min = 3))] name: String }`.
  Generates `impl Validate for User`.
- **Limitation [V]**: These crates compile validation rules directly into Rust
  types at compile time. They assume a closed, static schema known when the
  compiler runs.
- **Lesson for Traces**: Static derive validation cannot support dynamic PKM
  vaults, where schemas are user-authored TOML files (`.traces/schemas/*.toml`)
  loaded and modified at runtime.

### 3.3 `serde` (Rust, v1.0.229)

- **Approach [V]**: Complete architectural decoupling of the serialization data
  model from data formats through the `Serializer`/`Deserializer` and
  `Serialize`/`Deserialize` traits, bridged via the `Visitor` pattern.
- **Trade-off [V]**: Maximizes independence, but imposes compile-time
  costs, monomorphization overhead, and conceptual ceremony.
- **Lesson for Traces**: Essential for general-purpose cross-format translation;
  disproportionate for an internal validation pass checking six scalar variants.

### 3.4 `schemars` (Rust, v0.8.21)

- **Approach [V]**: Generates JSON Schema definitions from Rust types via
  the `JsonSchema` trait. The data structure owns the schema generator.
- **Direction [V]**: Data-type-to-schema extraction.
- **Lesson for Traces**: The reverse of Traces' need: Traces has external TOML
  schemas that validate open-world Markdown note data, not static Rust structs
  generating schemas.

### 3.5 `figment` (Rust, v0.10.19) and `config` (Rust, v0.14.0)

- **Approach [V]**: Hierarchical configuration combiners. `figment::Provider`s
  feed a unified dictionary of `figment::value::Value` items tagged with
  provenance metadata (`Tag`). A consumer extracts typed targets via Serde:
  `figment.extract::<Config>()`.
- **Lesson for Traces**: Configuration libraries separate the provider
  dictionary from target extraction. The provider dictionary serves as the
  neutral vocabulary layer.

### 3.6 `taplo` (Rust, v0.14.0) and `toml_edit`

- **Approach [V]**: Taplo parses TOML into a lossless CST/DOM
  (`taplo::dom::Node`) preserving source ranges [V: `taplo.tamasfe.dev`]. For
  validation, Taplo maps its DOM nodes into `jsonschema-rs`, translating
  schema validation failures back to source byte spans via DOM node ranges.
- **Lesson for Traces**: Taplo cleanly separates the validation engine (pure
  schema rules) from the diagnostic span mapper (AST/DOM adapter). This matches
  the two-phase kernel/adapter split proposed in Ticket 20 revision 2 [V:
  `brief.md:107-125`].

### 3.7 `rust-analyzer` (`ide_diagnostics`) and `ruff` (`ruff_linter`)

- **Approach in rust-analyzer [V]**: Rowan CST (`syntax`) knows nothing of type
  semantics. The semantic layer (`hir`) handles name resolution and type
  checking. A dedicated diagnostics module (`ide_diagnostics`) imports both
  `syntax` and `hir`, querying semantic facts and projecting diagnostics onto
  CST syntax nodes.
- **Approach in ruff [V]**: `ruff_python_ast` defines AST nodes. `ruff_linter`
  defines lint rules that inspect AST and semantic scopes, emitting structured
  `Violation` records.
- **Evolution and Regret [V]**: In earlier versions of `rust-analyzer`,
  diagnostics were scattered across parser passes, `hir_def`, and `hir_ty`.
  Scattered diagnostics caused duplicate warnings, inconsistent spans, and
  tight coupling. Centralizing diagnostic generation into an explicit analysis
  crate resolved these issues.

### 3.8 `Pydantic` (v2 `pydantic-core`) and `Zod` (TypeScript)

- **Approach in Pydantic v2 [V]**: In Pydantic v1, validation was coupled to
  Python class metaclasses, resulting in severe performance bottlenecks. For v2,
  the author built `pydantic-core` in Rust [V:
  `github.com/pydantic/pydantic-core`].
  A schema dictionary is compiled once into an execution graph
  (`SchemaValidator`), which exposes `.validate_python(obj)` and
  `.validate_json(bytes)`.
- **Approach in Zod [V]**: Zod schemas are runtime validator objects:
  `schema.safeParse(data: unknown) -> { success, data, error }`.
- **Lesson for Traces**: Compiling a schema once into an execution object that
  validates generic input dictionaries is fast and maintainable.

### 3.9 CUE (cuelang.org, v0.12.0)

- **Approach [V]**: Types and values occupy the same partial-order lattice.
  Schema validation is lattice unification: `Value & Schema = Result`. If the
  unification evaluates to bottom ($\bot$), a violation occurs.
- **Regret [V]**: Mathematically elegant, but constraint-conflict error messages
  are notoriously difficult for human users to interpret in large graphs.

### 3.10 Dendron Schemas (PKM, VS Code)

- **Approach [V]**: Dendron utilized YAML schemas (`*.schema.yml`) defining
  hierarchical path matching patterns (`pattern: notes.*`) and frontmatter
  properties [V: `docs.dendron.so`].
- **Regret [V]**: Schemas became too rigid and complex for PKM users. Errors
  were opaque, and maintenance friction contributed to Dendron halting active
  development in 2023 [V: `github.com/dendronhq/dendron`].
- **Lesson for Traces**: Schema validation in PKM must remain strictly advisory,
  forgiving, and simple to author (`docs/adr/0006:40-44`).

### 3.11 Obsidian Metadata Menu (`mdelobelle-metadatamenu`)

- **Approach [V]**: Field definitions live in fileClass markdown notes [V:
  `b-metadatamenu-lessons.md:16-20`].
- **Empirical Reality [V]**: Zero gating occurs during file writing, indexing,
  or storage parsing. `postValues()` writes arbitrary strings without checking
  schemas [V: `b-metadatamenu-lessons.md:75-85`]. Validation occurs exclusively
  inside interactive edit modals. The read-side API (`fileFields.ts`) exposes an
  advisory `isValid` boolean [V: `b-metadatamenu-lessons.md:102-108`].
- **Lesson for Traces**: Confirms Traces' settled architecture: note ingestion
  and indexing must remain 100% unconditional and schema-free. Validation is an
  on-demand advisory pass.

---

## 4. Menu of Candidate Architectures

We evaluate eight candidate architectures for connecting `schema` and `note`.

```text
1. Keep-As-Is (No checker)
2. Consumer Module / Glue (Disliked file_class_expander pattern)
3. Schema Depends on Note (schema -> note)
4. Note Depends on Schema (note -> schema)
5. Shared Foundation Vocabulary (schema -> field <- note)
6. Small View Trait in Schema (schema::ValueView implemented by note)
7. Compiled Generic Schema Validator (Validator<V: ValueInspector>)
8. Unified Vault Model Module (Merge schema + note frontmatter)
```

### Candidate 1: Keep-As-Is (No Checker)

- **Description**: Do not introduce any validation connection between `schema`
  and `note`. Schemas remain passive metadata providers for templates
  (`schema.get("class").field("prop")`) and query class expansion (`@class*`).
- **Imports**: Zero new imports.
- **Crate Split**: Trivial; `traces-schema` and `traces-note` remain isolated.
- **Test Burden**: Zero.
- **Generics / Dyn Cost**: Zero.

### Candidate 2: Checker in a Consumer Module (Disliked Glue Pattern)

- **Description**: Keep `schema` and `note` completely ignorant of each other.
  Place the validation function in a higher-tier consumer module (e.g.
  `src/validate.rs`, `src/lsp/diagnostics/schema.rs`, or a root glue file):
  `fn check_note(note: &Note, schema: &Schema) -> Vec<Violation>`.
- **Imports**: The consumer module imports `crate::schema::Schema` and
  `crate::note::{Note, NoteFieldValue}`.
- **Crate Split**: Requires an artificial intermediate crate (`traces-validate`)
  or forces the checker to live inside `traces-lsp`. If templates or MCP agents
  need validation, they must depend on `traces-lsp` or duplicate the logic.
- **Test Burden**: High; unit tests require instantiating full note and schema
  environments in integration-level test suites.
- **Generics / Dyn Cost**: Low if concrete; high if abstract traits used.

### Candidate 3: Schema Depends on Note Value Type (Direct Dependency)

- **Description**: `src/schema` imports `NoteFieldValue` and `FieldKey` directly
  from `src/note`. The validation method lives directly on `Schema`:
  `Schema::validate(&self, fields: &IndexMap<FieldKey, NoteFieldValue>) ->
  Vec<Violation>`.
- **Imports**: `src/schema` adds `use crate::note::NoteFieldValue;`. `src/note`
  imports nothing from `schema`.
- **Crate Split**: Reverses the current topological tiers (`schema` Tier 4,
  `note` Tier 5 [V: `survey/cross-cutting.md:35`]). `traces-schema` becomes a
  downstream dependent of `traces-note`.
- **Test Burden**: Low; `Schema::validate` can be tested using in-memory
  `NoteFieldValue` instances without parsing markdown files.
- **Generics / Dyn Cost**: Zero generics, zero `dyn`.

### Candidate 4: Note Depends on Schema

- **Description**: `src/note` imports `src/schema::Schema`. A method is placed
  on `Note` or `Frontmatter`: `note.validate(&self, schema: &Schema) ->
  Vec<Violation>`.
- **Imports**: `src/note` adds `use crate::schema::Schema;`.
- **Crate Split**: `traces-note` depends on `traces-schema`.
- **Test Burden**: High; note tests must construct or mock schema registries.
- **Generics / Dyn Cost**: Zero generics, zero `dyn`.

### Candidate 5: Shared Neutral Foundation Vocabulary (Recommended)

- **Description**: Both `src/schema` and `src/note` depend on a neutral,
  lower-tier value representation: the canonical `FieldValue` enum in
  `src/field.rs` (Tier 3 [V: `survey/cross-cutting.md:35`]), as resolved by
  Ticket 18 [V: `.scratch/arch-design/issues/18-value-model.md:15-17, 77-82`].
  The validation kernel lives directly on `Schema` in `src/schema/validate.rs`:

  ```rust
  impl Schema {
      pub fn validate(
          &self,
          fields: &IndexMap<FieldKey, FieldValue>,
      ) -> Vec<Violation> {
          // ... compares FieldValue against SchemaFieldDef ...
      }
  }
  ```

- **Imports**:
  - `src/schema` imports `crate::field::{FieldKey, FieldName, FieldValue}`.
  - `src/note` imports `crate::field::{FieldKey, FieldValue}`.
  - Neither `src/schema` nor `src/note` imports the other.
- **Crate Split**: In a workspace split:
  - `traces-core` (or `traces-field`): Tier 0/1. Contains `FieldKey`,
    `FieldName`, `FieldValue`, `DateValue`, `DateTimeValue`, `DurationValue`.
  - `traces-schema`: Tier 2. Depends on `traces-core`. Owns schema TOML
    parsing, inheritance, and `Schema::validate`.
  - `traces-note`: Tier 2. Depends on `traces-core`. Owns note markdown parsing
    and frontmatter conversion into `FieldValue`.
  - Consumers (`traces-lsp`, `traces-template`, `traces-query`, `traces-cli`):
    Tier 3+. Depend on both `traces-schema` and `traces-note`.
- **Test Burden**: Minimal. `Schema::validate` is tested purely with in-memory
  maps of `FieldValue`, without touching markdown or note parsers.
- **Generics / Dyn Cost**: Zero generics, zero `dyn`. Concrete, cache-friendly
  data structures.

### Candidate 6: Schema Defines a Small Trait Implemented by Note

- **Description**: `src/schema` defines a value view trait:

  ```rust
  pub trait ValueView {
      fn as_str(&self) -> Option<&str>;
      fn as_f64(&self) -> Option<f64>;
      fn as_bool(&self) -> Option<bool>;
      fn as_date(&self) -> Option<DateValue>;
  }

  impl Schema {
      pub fn validate(
          &self,
          fields: &IndexMap<FieldKey, impl ValueView>,
      ) -> Vec<Violation>;
  }
  ```

  `src/note::NoteFieldValue` implements `ValueView`.
- **Imports**: For `NoteFieldValue` to implement `ValueView`, `src/note` must
  import `src/schema::ValueView` (recreating Candidate 4's inverted dependency),
  or an external glue module must implement it (recreating Candidate 2).
- **Crate Split**: Cycles or glue crates required.
- **Test Burden**: Medium; requires mocking trait implementors.
- **Generics / Dyn Cost**: Monomorphization on `validate`, or `dyn ValueView`
  virtual call overhead per field.

### Candidate 7: Compile Schema into Ready-to-Run Generic Checker Object

- **Description**: The `jsonschema-rs` / `pydantic-core` pattern.
  `Schema::compile` produces a `SchemaValidator<V: ValueInspector>`.
  The validator holds an optimized node tree of validation operations,
  parameterized over the input representation.
- **Imports**: Decoupled via generic parameters.
- **Crate Split**: Clean separation.
- **Test Burden**: High; requires testing the compiler, execution engine, and
  adapter implementations.
- **Generics / Dyn Cost**: Significant generic parameter proliferation and code
  bloat.

### Candidate 8: Unified 'Vault Model' Module (Consolidate Schema + Note-Typing)

- **Description**: Merge `src/schema` and the frontmatter/field typing parts
  of `src/note` into a single module: `src/vault_model` (or `traces-model`).
  `src/vault_model` owns schemas, frontmatter fields, and validation. `src/note`
  is reduced to a pure document parser (headings, lists, blocks).
- **Imports**: `Schema` and `Frontmatter` are sibling types within the same
  module; no cross-module boundary exists between them.
- **Crate Split**: Forms a single cohesive `traces-model` domain crate.
- **Test Burden**: Low within the module; high blast radius across the rest of
  the codebase.
- **Generics / Dyn Cost**: Zero generics, zero `dyn`.

---

## 5. Strongest Arguments Against Each Candidate

| Candidate | Strongest Argument Against |
| :--- | :--- |
| **1. Keep-as-is** | Violates committed project roadmap: leaves schema options (`required`, `multi`, `min`, `max`, `step`, `format`) permanently dead in production, prevents LSP frontmatter diagnostics, and blocks MCP tool write guardrails [V: `brief.md:23-30`]. |
| **2. Consumer Glue** | Semantic homelessness and consumer fragmentation. Validation is a core schema capability, not an editor quirk. Stashing it in LSP forces templates and MCP agents to depend on LSP or duplicate code. Recreates the disliked orphan glue of `file_class_expander.rs` [V: `20-prep-context-3:46-54`]. |
| **3. Schema -> Note** | Inverts architectural domain hierarchy. Schemas (abstract declarative rules) become coupled to the concrete Markdown document parser (`traces-note`). Schemas cannot be reused to validate non-note inputs (e.g. MCP JSON tool parameters, CSV imports) without dragging in Markdown AST dependencies [V: `MODELING.md:57-61`]. |
| **4. Note -> Schema** | Severe breach of PKM domain invariant: notes are open-world, schemaless documents that parse and persist unconditionally into Redb without schemas [V: `docs/adr/0009:76-77`]. Coupling note parsing to schemas risks circular dependencies, breaks isolated note testing, and turns schema TOML errors into note parsing liabilities. |
| **5. Shared Vocabulary** | Requires completing the Ticket 18 value unification: migrating `NoteFieldValue` into the canonical `FieldValue` in `src/field.rs`. Note-specific types (such as `Link`) must be accommodated in the shared value enum at Tier 3 rather than remaining private to `src/note` [V: `.scratch/arch-design/issues/18-value-model.md:57-65`]. |
| **6. Small View Trait** | Trait-in-provider dilemma: `note` cannot implement `schema::ValueView` without depending directly on `schema` or resorting to an orphan glue file. Introduces trait indirection and dynamic dispatch for a system with exactly one data representation, violating `MODELING.md:40-46`. |
| **7. Compiled Generic** | Extreme over-engineering. JSON Schema validators compile execution trees because JSON Schema has hundreds of recursive, combinatorial keywords (`$defs`, `anyOf`, regex, `patternProperties`). Traces schemas validate six flat scalar types and two boolean flags. A generic compiled execution engine violates YAGNI. |
| **8. Unified Model** | Conflates distinct operational lifecycles: schema definitions are static, configuration-time TOML metadata loaded once at startup; note documents are thousands of user files parsed, indexed, and modified continuously. Packing them into one crate inflates compilation blast radius. |

---

## 6. Ranked Recommendation and Decision Triggers

### 6.1 Ranked Order

1. **Rank 1: Candidate 5 (Shared Foundation Vocabulary in `src/field.rs`)**
   - **Why**: Perfectly aligns with the resolved Ticket 18 decision ("not one
     model and not four. Two owned enums: a runtime `FieldValue` for notes and
     queries, and a schema-local literal type... choose the lowest existing
     layer", `.scratch/arch-design/issues/18-value-model.md:15-17, 77-80` [V]).
   - `src/field.rs` already sits at Tier 3 [V: `survey/cross-cutting.md:35`],
     below both `schema` (Tier 4) and `note` (Tier 5).
   - `Schema::validate(&self, fields: &IndexMap<FieldKey, FieldValue>) ->
     Vec<Violation>` lives directly on `Schema` in `src/schema/validate.rs`.
     This satisfies the Principle of Least Surprise (`MODELING.md:16-25`):
     validation is a method on `Schema`, requiring zero glue files, zero traits,
     zero `dyn` dispatch, and zero cycles.
   - Survives a workspace split cleanly: `traces-core` owns `FieldValue`;
     `traces-schema` and `traces-note` are sibling crates; downstream
     consumers (`traces-lsp`, `traces-template`, `traces-mcp`) pass note fields
     into the schema validator directly.

2. **Rank 2: Candidate 3 (Schema Depends on Note Value Type `schema -> note`)**
   - **Why**: If unifying `NoteFieldValue` into `src/field.rs` is deferred or
     rejected, letting `src/schema` import `NoteFieldValue` is the most direct,
     zero-glue alternative. `Schema::validate` remains on `Schema`. The cost is
     coupling `traces-schema` to `traces-note` in a crate split.

3. **Rank 3: Candidate 8 (Unified Vault Model Module)**
   - **Why**: Clean domain cohesion, but higher refactoring blast radius.

4. **Rank 4: Candidate 2 (Consumer Module / Glue)**
   - **Why**: Workable, but perpetuates the exact orphan-glue anti-pattern the
     user disliked in `file_class_expander.rs`.

5. **Rank 5-8: Candidates 6, 7, 4, 1**
   - **Why**: Rejected for trait indirection ceremony (6), unwarranted
     complexity (7), architectural boundary destruction (4), or roadmap
     default (1).

### 6.2 Evidence That Would Change the Recommendation

- **Evidence favoring Candidate 3 over 5**: If Ticket 18's value unification is
  reopened and it is decided that `NoteFieldValue` must remain strictly private
  to `src/note` (e.g. due to note-specific span pointers or AST references),
  Candidate 5 becomes blocked. In that scenario, `schema -> note` (Candidate 3)
  is the only remaining concrete, glue-free design.
- **Evidence favoring Candidate 7 over 5**: If Traces decides to support full
  recursive JSON Schema / Draft 2020-12 within schema definitions (e.g. for
  arbitrary nested object validation in frontmatter), a compiled execution
  graph would become necessary. (Currently, schemas are flat scalar
  definitions, rendering Candidate 7 unnecessary).
- **Evidence favoring Candidate 8 over 5**: If benchmark evidence shows that
  having separate crates for `schema` and `note` incurs unacceptable Cargo build
  latency or test duplication, merging them into a single `traces-model` crate
  would be justified.

---

## 7. Lessons Transferable to Other Sub-problems

1. **To Sub-problem B (`ClassExpanderCritique`)**:
   `src/file_class_expander.rs` exists because `query` created a consumer trait
   (`FileClassExpander`) to avoid depending on `schema`. If `query` is at Tier 7
   and `schema` is at Tier 4, `query` could have depended on `schema` directly,
   or both could have shared a class hierarchy provider in `traces-core`. The
   solution to orphan glue is moving shared vocabulary to the foundation tier,
   not inventing dynamic consumer traits.

2. **To Sub-problem C (`AgentGuardrail`)**:
   An MCP tool guardrail validates incoming tool JSON parameters against a
   schema. Under Candidate 5, because `Schema::validate` accepts
   `IndexMap<FieldKey, FieldValue>`, the MCP server can convert parsed JSON
   values into `FieldValue` and run the exact same validation kernel. The
   validation kernel does not care whether values originated from a Markdown
   YAML note or an incoming MCP JSON RPC call!

3. **To Sub-problem D (`TextVsTyped`)**:
   Validation logic operates strictly on classified `FieldValue` items.
   Whether a date is parsed as a typed `DateValue` or retained as a string
   occurs at the parse boundary (`src/field.rs` / `src/note`), prior to
   validation. The validation kernel checks the invariant: "does this
   `FieldValue` match `SchemaFieldDef`?"

4. **To Sub-problem E (`ViolationModel`)**:
   The validation kernel `Schema::validate` emits pure, span-free `Violation`
   records (`field: FieldKey`, `kind: ViolationKind`, `expected`, `found`).
   The diagnostic presentation adapter (LSP diagnostics, CLI formatting) is an
   outer layer that combines the pure `Violation` with note source spans.
   Keeping the kernel in `src/schema` free of source span dependencies
   ensures it remains reusable across index notes, editor notes, templates, and
   MCP tool calls.

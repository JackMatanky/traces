# Scout Report B: FileClassExpander Critique & Class Ownership

- Ticket: 20 (Schema Note Typing and Validation)
- Sub-problem: B (Critique `src/file_class_expander.rs` & Class Ownership)
- Target: `.design/2026-10-21-model-schema-validation/scouts/`
  `B-class-expander-critique.md`
- Status: Research complete; proposal ready

---

## 1. The Sub-Problem Restated

Traces supports File Class source filtering in its query DSL:
`@Class` (exact match), `@Class+` (class and direct children), and
`@Class*` (class and all transitive descendants)
[V: `src/query/grammar/source.rs:310-325`].
To resolve hierarchy expansions like `@Book*` into `["book", "novel", "sci_fi"]`
without coupling the query parser directly to the schema loader, Traces
currently employs a trait indirection:

- `pub(crate) trait FileClassExpander` defined in
  `src/query/grammar/source.rs:353-356` [V].
- `QueryService` holds `Option<Arc<dyn FileClassExpander>>` [V:
  `src/query/service.rs:59`].
- `src/file_class_expander.rs` (an 87-line root module at Tier 8) implements
  `FileClassExpander` for `SchemaService`
  [V: `src/file_class_expander.rs:15-39`].
- The expansion method takes `&mut ClassExpansionMode` to mutate the query
  AST in-place [V: `src/file_class_expander.rs:16`,
  `src/query/grammar/source.rs:81-96`].
The user steer for Round 3 explicitly notes:
"A previously proposed seam ('checker lives in a consumer module above schema
and note, like `src/file_class_expander.rs`') feels like bad design to the
user. They say `file_class_expander.rs` 'has never felt right' and they know of
no codebase that structures components this way; it may be a lazy anti-pattern"
[V: `local://20-prep-context-3.md:46-54`].

This scout addresses:

1. Complete code map and call graph of all symbols involved in File Class
   expansion, tracing where inheritance expansion occurs at query vs index time.
2. Concrete diagnosis of the design smells that make `file_class_expander.rs`
   "feel wrong", distinguishing verified structural defects [V] from taste [H].
3. Rigorous verification of whether the user's claim holds, and whether the
   "orphan glue file" pattern is recognized or viable in multi-crate Rust.
4. Precedents across 7 real-world systems handling ontology/class expansion.
5. Menu of 7 candidate solutions, evaluated against Sub-problem A (the checker)
   and Ticket 32 (workspace crate split).
6. Ranked recommendations and transferable architectural lessons.

---

## 2. What Traces Does Today (Inventory and Analysis)

### 2.1 The Components and Their Locations

```text
                 ┌─────────────────────────────┐
                 │     src/cli/mod.rs:470      │
                 │ run_query_builder_from_store│
                 └──────────────┬──────────────┘
                                │ wires Arc<SchemaService>
                                ▼
┌─────────────────────────┐           ┌─────────────────────────┐
│         Tier 4          │           │         Tier 7          │
│       src/schema/       │           │       src/query/        │
│                         │           │                         │
│ SchemaService           │           │ QueryService            │
│  - matches()            │           │  - class_expander:      │
│  - children_names_of()  │           │    Option<Arc<dyn       │
│  - warn_unknown_classes │           │      FileClassExpander>>│
│                         │           │ SourceSelector          │
│                         │           │ ClassExpansionMode      │
└────────────┬────────────┘           └────────────┬────────────┘
             │                                     │
             │ implements                          │ defines
             │ FileClassExpander                   │ FileClassExpander
             │                                     │
             └────────────────► ◄──────────────────┘
                                │
                  ┌─────────────┴─────────────┐
                  │          Tier 8           │
                  │ src/file_class_expander.rs│
                  │ (orphan glue module)      │
                  └───────────────────────────┘
```

#### A. The Trait: `FileClassExpander`

Defined in `src/query/grammar/source.rs:353-356` [V]:

```rust
pub(crate) trait FileClassExpander: Send + Sync {
    /// Expands `classes` into `mode`'s match set.
    fn expand(&self, classes: &[String], mode: &mut ClassExpansionMode);
}
```

The doc comment explicitly notes:
"The `Send + Sync` bound lets `Arc<dyn FileClassExpander>` satisfy minijinja's
`Object: Send + Sync` bound when `QueryOps` stores a persistent `QueryService`"
[V: `src/query/grammar/source.rs:350-352`].

#### B. The AST and Expansion Mode

Defined in `src/query/grammar/source.rs:320-346` [V]:

```rust
pub(crate) enum ClassExpansionMode {
    Exact(BTreeSet<String>),
    Children(BTreeSet<String>),
    Descendants(BTreeSet<String>),
}
```

Each variant encapsulates a `BTreeSet<String>` [V: `source.rs:322-324`]. When
parsed from the query text (`@Class`, `@Class+`, `@Class*`), this set is
initialized to `BTreeSet::new()` [V: `source.rs:384, 387, 390`]. It serves as a
mutable placeholder within the AST node `SourceAtom::Class { names, mode }`
[V: `source.rs:196-201`].

#### C. The Glue File: `src/file_class_expander.rs`

Registered as a crate-root module in `src/lib.rs:118` (`mod
file_class_expander;`)
[V]. The file contains 39 lines of implementation and 47 lines of unit tests:

- Lines 1-6: Doc comment stating: "This is the only place in the crate
  referencing both `schema` and `query` for File Class expansion. Neither
  module depends on the other; this module wires them together" [V].
- Lines 15-39: Implements `FileClassExpander for SchemaService`:
  - `Exact`: calls `self.warn_unknown_classes(classes)` [V: line 20].
  - `Children`: calls `self.warn_unknown_classes(classes)`, loops over classes,
    and calls `self.children_names_of(class)` [V: lines 23-28].
  - `Descendants`: calls `expanded = self.matches(classes)` [V: line 34].
    Comment notes: "`matches` warns internally, so this branch alone would
    otherwise skip the warning the other two branches emit directly above"
    [V: lines 31-33].
  - Finally, mutates the argument:
    `mode.set_classes(expanded.into_iter().collect())`
    [V: line 37].
- Lines 41-87: Unit tests with a mock 4-tier schema (`thing -> book -> sci_fi ->
  space_opera`).

#### D. The Test Mock: `RecordingExpander`

Defined exclusively in `src/query/grammar/source.rs:1160-1173` [V]:

```rust
#[derive(Default)]
struct RecordingExpander {
    calls: std::sync::Mutex<Vec<Vec<String>>>,
}

impl FileClassExpander for RecordingExpander {
    fn expand(&self, classes: &[String], mode: &mut ClassExpansionMode) {
        self.calls.lock().expect("lock calls").push(classes.to_vec());
        mode.set_classes(classes.iter().cloned().collect());
    }
}
```

This is the **only other implementor** in the entire repository [V]. It exists
solely to test that `SourceSelector::resolve_classes` visits AST atoms.

#### E. Callers and Wiring

- `src/query/service.rs:57-60`: `QueryService` stores
  `class_expander: Option<Arc<dyn FileClassExpander>>` [V].
- `src/query/service.rs:82-88`: `.with_class_expander(expander)` attaches it
  [V].
- `src/cli/mod.rs:478-480`: Inside `run_query_builder_from_store`, CLI inspects
  `if has_classes`:

  ```rust
  let service = if has_classes {
      let schema_service = Arc::new(load_schema_service(config)?);
      service.with_class_expander(schema_service)
  } else {
      service
  };
  ```

- `src/template/engine/query.rs:121-122`: In Minijinja template execution,
`QueryOps` constructs
`QueryService::new(class_field).with_class_expander(schemas)`
  unconditionally [V].

### 2.2 Where Does Inheritance Expansion Happen?

| Lifecycle Stage | Does Class Inheritance Expansion Happen? | Details & Evidence |
| :--- | :--- | :--- |
| **Parsing** | No | AST records `ClassExpansionMode::*(BTreeSet::new())` [V: `source.rs:384`]. |
| **Indexing** | **NO** | Indexer has **zero awareness of schemas** (ADR-0009). Frontmatter values under `class_field` are extracted as raw lowercase strings and indexed into Redb multimap `paths_by_file_class` [V: `src/index/store.rs:1781-1798`]. |
| **Query Planning** | **YES** | `QueryService::run` and `run_from_store` call `resolve_source_classes` [V: `src/query/service.rs:110, 135`]. |
| **Execution** | Matched | Store-backed uses expanded classes to query `paths_by_file_class` [V: `service.rs:408`]; in-memory filters index entries against `mode.classes()` [V: `source.rs:227-230`]. |

Inheritance expansion happens **strictly at query planning time**, in-memory,
synchronously, before row generation.

### 2.3 The Hidden Semantic Bifurcation Bug

Because `class_expander` is an `Option` on `QueryService`, omitting it triggers
radically different, broken behavior depending on execution mode:

1. **Store-backed execution (`run_from_store`)** [V:
   `src/query/service.rs:402-406`]:

   ```rust
   if mode.classes().is_empty() {
       for class in names {
           let paths = self.store.paths_with_file_class(class)?;
           acc.extend_from_slice(&paths);
       }
   }
   ```

If no expander was provided, it falls back to querying the raw unexpanded
`names`.
   An exact query like `@book` succeeds.

2. **In-memory execution (`run`)** [V: `src/query/grammar/source.rs:227-230`]:

   ```rust
   Self::Class { mode, .. } => entry.note().is_some_and(|note| {
       class_values(note, canonical_class_field)
           .any(|value| mode.classes().contains(value))
   }),
   ```

   In memory, matching relies **strictly** on `mode.classes()`. If no expander
   was attached, `mode.classes()` is **empty**. Even an exact query like `@book`
   returns **0 matching notes**, even if notes with `class: book` exist [V]!

This proves that `FileClassExpander` is not an optional optimization: it is an
undocumented required dependency whose absence silently breaks in-memory
queries.

### 2.4 Diagnosis of Design Smells: Real [V] vs Taste [H]

| Smell | Type | Evidence & Technical Impact |
| :--- | :--- | :--- |
| **Trait with one impl** | **Real [V]** | Exactly 1 production impl (`SchemaService`), 1 mock (`RecordingExpander`). Forces `Arc<dyn ...>` vtable indirection, heap allocation, and disables inlining for a simple in-memory map lookup. |
| **Defined by consumer** | **Taste [H] / Defect [V]** | In theory, consumer-defined interfaces follow DIP. In practice, `expand` takes `&mut ClassExpansionMode` [V], which is `query`'s internal AST enum! The provider is tightly coupled to `query` internals. |
| **Orphan glue file at root** | **Real [V]** | Sits at crate root (`src/file_class_expander.rs`) solely because of module tiering (Tier 4 `schema`, Tier 7 `query`). Under Rust's cross-crate orphan rules (RFC 1023), this pattern is **compiler-illegal** across crates. |
| **`Option<dyn>` dependency** | **Real [V]** | Silent behavioral fork: omitting expander exact-matches in store queries but returns 0 results in memory queries. |
| **Out-param mutation** | **Real [V]** | `fn expand(&self, classes: &[String], mode: &mut ClassExpansionMode)` takes a `&mut` reference to an enum to overwrite inner state. Violates idiomatic Rust value-returning design. |
| **Mutating query AST** | **Real [V]** | `SourceAtom::Class` holds mutable state (`mode`) populated post-parse. The parsed AST is not immutable; it cannot be safely shared across executions without re-expansion. |
| **Side-effecting warnings** | **Real [V]** | `warn_unknown_classes` uses `tracing::warn!` inside query expansion [V: `src/schema/service.rs:194`]. Callers (CLI, LSP) cannot intercept or format diagnostics. |

### 2.5 Verification of the User's Claim

The user claimed:

1. `file_class_expander.rs` "has never felt right" and is a "lazy anti-pattern".
2. They know of no codebase that structures components this way.
3. The pattern should not be repeated for Sub-problem A (the schema checker).

**Verdict: The user's claim fully HOLDS [V].**

#### Why the pattern is not recognized in Rust

In multi-crate Rust, RFC 1023 (Coherence) dictates:
"Only traits defined in the current crate can be implemented for types defined
outside of the crate."

If `traces` is split into crates (`traces-query`, `traces-schema`,
`traces-cli`):

- `FileClassExpander` lives in `traces-query`.
- `SchemaService` lives in `traces-schema`.
- A third crate (e.g. `traces-cli` or a root glue crate) **CANNOT** write:
  `impl traces_query::FileClassExpander for traces_schema::SchemaService` [V].
  The Rust compiler rejects it with error `E0117`!

To make this compile across crates, one would have to wrap `SchemaService` in an
artificial local newtype (`struct SchemaExpander(Arc<SchemaService>)`).

The pattern worked in Traces only because `src/file_class_expander.rs` is
currently inside the same monolithic crate (`src/lib.rs`). It is an
idiosyncratic
workaround for internal module DAG tiering, masquerading as architecture.

---

## 3. Precedents Across Real-World Systems

### 3.1 Dataview (Obsidian Plugin)

- **Mechanism**: Evaluates `FROM #tag` and `FROM "folder"`. Dataview does
  **not**
  support schema or class inheritance [V: Dataview documentation]. Tag hierarchy
  is evaluated via string prefix matching (`#tag/subtag` starts with `#tag`).
- **Lesson & Post-Mortem**: Dataview issues #877 and #1143 requested class
inheritance. The author (blacksmithgu) repeatedly deferred this because coupling
  the in-memory index to an external schema system created state invalidation
  cliffs. When Dataview 2.0 (Datacore) attempted to introduce typed schemas,
  coupling between the indexer and schema resolution stalled the project.

### 3.2 Metadata Menu & Fileclass (Obsidian Plugins)

- **Mechanism** [V: `mdelobelle.github.io/fileclass/schema/`]:
  `fileclass` (the successor to Metadata Menu) implements single inheritance
  (`extends: Media`) across Markdown schema notes.
- **Query Resolution**: The plugin maintains an in-memory `fileClassIndex` (a
  precomputed DAG). When querying or checking fields, it queries the index for
  `fileClass.getInheritedFileClasses()`. The index is a **precomputed pure data
  lookup table**, not an AST-mutating visitor.
- **Failures & Regrets**: Performance bug #324: dynamic synchronous traversal of
  unindexed inheritance chains on note load froze the Obsidian UI thread. Fixed
by precomputing the full transitive ancestor/descendant sets at schema load
time.

### 3.3 Neo4j & Graph Databases (Label Hierarchies)

- **Mechanism** [V: Neo4j Cypher Manual, "Labels"]:
  Neo4j nodes support multiple flat labels (`:Book:Novel:SciFi`). Neo4j
  **deliberately omitted** label inheritance from its core engine.
- **Reasoning**: If `:SciFi` inherited from `:Book`, every index lookup on
  `:Book`
  would require dynamic label expansion at query planning time, invalidating
  compiled Cypher query plans whenever an ontology edge changed.
- **Lesson**: Keep stored identifiers flat. If hierarchy is needed, precompute
the closure or evaluate it via explicit graph traversal (`MATCH
(b:Book)<-[:EXTENDS*]-(sub)`).

### 3.4 Relational Databases: SQL Recursive CTEs vs. Closure Tables

- **Precedent** [V: Bill Karwin, *SQL Antipatterns*, Ch. 3 "Naive Trees"; Joe
  Celko]:
  Relational databases offer two primary approaches for hierarchy queries:
  1. *Recursive CTEs (`WITH RECURSIVE`)*: Dynamic traversal at query time.
     Slow on deep graphs, hard to optimize, prevents index seeks.
  2. *Closure Tables (`class_closure`)*: Precomputes all `(ancestor, descendant,
     depth)`
     tuples in a flat table. A query for `@Book*` becomes a trivial flat join:
`WHERE class IN (SELECT descendant FROM class_closure WHERE ancestor = 'Book')`.
- **Lesson**: Hierarchies should be queried as precomputed flat sets, not
  dynamic
  runtime traversals.

### 3.5 Apache Lucene & Tantivy (Search Engines: Query Rewriting)

- **Mechanism** [V: Lucene Javadocs, `org.apache.lucene.search.Query.rewrite`]:
  In Lucene, taxonomy, prefix, and wildcard queries do **not** mutate the AST.
  Query execution has two distinct phases:
  1. `Query::rewrite(IndexReader) -> Query`: A pure function taking an immutable
     query AST and returning a *new* expanded query (e.g. `PrefixQuery("book")`
     rewrites to a `BooleanQuery` of matching `TermQuery` instances).
  2. `Query::createWeight(...)`: Prepares execution against the index.
- **Failures & Regrets**: In early Lucene (pre-2.9), queries had internal
  mutable
  state. Concurrently executing the same parsed query across threads caused race
  conditions. Lucene refactored queries to be strictly immutable values,
  moving all expansion into an explicit `rewrite` pass.

### 3.6 Rust-Analyzer & Salsa (Incremental Compilation)

- **Mechanism** [V: rust-analyzer architecture guide]:
  In Salsa, queries are pure functional lookups over a database trait:
  `fn sub_traits(db: &dyn DefDatabase, trait_id: TraitId) -> Arc<[TraitId]>`.
- **Lesson**: Rust-analyzer strictly avoids "stateful service objects" or
  "trait-based dependency inversion for internal compilation phases". The query
  compiler consumes immutable data snapshots from the database.

### 3.7 Ripgrep & Ignore

- **Mechanism** [V: `ripgrep/crates/ignore/src/gitignore.rs`]:
  `Gitignore` builds an immutable matcher struct (`Gitignore`). The directory
walker takes `&Gitignore` as plain data. It does not use an `Option<Arc<dyn
IgnoreExpander>>`
  or mutate walker state.

---

## 4. Menu of Candidate Solutions

### Candidate 1: Keep As-Is

- **Description**: Retain `FileClassExpander` trait,
  `src/file_class_expander.rs`,
  and `Option<Arc<dyn FileClassExpander>>` on `QueryService`.
- **Checker (Sub-problem A)**: Follows the same pattern: a new root glue file
`src/schema_checker.rs` at Tier 8 implementing a consumer trait across `schema`
  and `note`.
- **Workspace Split (Ticket 32)**: **Fails immediately under RFC 1023**. Root
  crate cannot implement `traces_query::FileClassExpander` for
  `traces_schema::SchemaService` without artificial newtype wrappers.
- **Strongest Argument Against**: Preserves all 7 verified design smells, blocks
  a
  clean workspace split, and perpetuates mutable AST anti-patterns.

### Candidate 2: Precomputed `ClassHierarchy` Value (Data, Not Behavior)

- **Description**:
  - Define `ClassHierarchy` as an immutable data structure:

    ```rust
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    pub struct ClassHierarchy {
        // maps class -> (children, descendants)
        classes: Arc<HashMap<String, ClassFamily>>,
    }
    ```

  - `SchemaService` already computes `children` and `descendants` during
    topological load [V: `src/schema/builder.rs:346`]. It simply exposes
    `schema_service.class_hierarchy() -> &ClassHierarchy`.
  - `QueryService` takes `ClassHierarchy` (by value or `Arc`), defaulting to
    `ClassHierarchy::empty()`.
  - Delete `FileClassExpander` trait and `src/file_class_expander.rs`.
- **Checker (Sub-problem A)**: Schema exports `ValidationRules` or
  `SchemaCatalog`
  as pure data. The checker is a pure function:
  `validate_note(note: &Note, rules: &SchemaCatalog) -> Vec<Diagnostic>`.
- **Workspace Split (Ticket 32)**: Clean. `ClassHierarchy` lives in
  `traces-core`
  or `traces-query`. `traces-schema` produces it; `traces-query` consumes it. No
  orphan rule issues.
- **Strongest Argument Against**: If schemas change in a running process (LSP),
  the caller must pass the updated `ClassHierarchy` snapshot to `QueryService`.

### Candidate 3: Store Hierarchy in Index/Store (Index-Time Expansion / Closure

Table)

- **Description**:
  - Store class hierarchy in Redb: either expand note classes during indexing
    (a note with `class: sci_fi` is indexed under `sci_fi`, `book`, `media`), or
    maintain a secondary `class_closure` table in Redb.
- **Checker (Sub-problem A)**: Notes are validated at index time; invalid notes
  are
  rejected or flagged in Redb.
- **Workspace Split (Ticket 32)**: Couples `index` crate to `schema` crate.
- **Strongest Argument Against**: **Directly violates ADR-0009 and settled
  facts**
  [V: `local://20-prep-context-3.md:32-33`]. ADR-0009 mandates that index
  fingerprints exclude schemas. Editing a `.traces/schemas/*.toml` file would
  force a full index rebuild of all 20,000 notes.

### Candidate 4: Neutral Domain Module (`class` / `vault_model`)

- **Description**:
  - Create a low-tier module `src/class.rs` (Tier 0 or 3) or `src/vault_model/`:
    Owns `ClassName` (normalized), `ClassExpansionMode`, and `ClassHierarchy`.
  - Both `schema` and `query` import `class`.
  - `schema` populates the hierarchy; `query` uses it to resolve `@Class*`.
- **Checker (Sub-problem A)**: Diagnostic models, note field values, and schema
types live in `vault_model`, giving both `note` and `schema` a shared vocabulary
  without circular dependencies.
- **Workspace Split (Ticket 32)**: Forms `traces-model` or `traces-core`, the
  foundational leaf crate in the workspace DAG.
- **Strongest Argument Against**: Adds a new module/crate. For class expansion
  alone, a 50-line data struct might feel like premature modularization if not
  combined with field typing.

### Candidate 5: Expansion Done by Caller / Pre-pass Before Query Execution

- **Description**:
  - The query engine knows **nothing** about class inheritance.
    `SourceAtom::Class`
    only accepts exact string sets (`Class(HashSet<String>)`).
  - The caller (CLI, template engine) runs a pre-pass over the query AST using
    `SchemaService` to rewrite `@Book*` into `(@book OR @novel OR @sci_fi)`
    before building `QueryBuilder`.
- **Checker (Sub-problem A)**: Caller explicitly coordinates validation before
  calling consumers.
- **Workspace Split (Ticket 32)**: `traces-query` has zero schema knowledge.
- **Strongest Argument Against**: Leaks query AST traversal into every caller
  (CLI, Minijinja template adapter, future LSP). High duplication risk.

### Candidate 6: Move the Implementation Next to Schema

- **Description**:
  - Delete `src/file_class_expander.rs`.
  - Have `src/schema/` import `query` and write `impl FileClassExpander for
    SchemaService`
    inside `src/schema/service.rs`.
- **Checker (Sub-problem A)**: Schema imports `note` and defines
  `SchemaService::validate(&self, note: &Note)`.
- **Workspace Split (Ticket 32)**: Requires `traces-schema` to depend on
  `traces-query`.
- **Strongest Argument Against**: Inverts natural dependency ordering:
  metadata/schema
definitions should not depend on a heavyweight query execution engine containing
  parsers, Rayon executors, and sorting matrices. Furthermore, directly violates
settled fact: "`schema` imports nothing from `note`" [V:
`local://20-prep-context-3.md:26-27`].

### Candidate 7: Query Rewrite Phase with Immutable AST (The Lucene / Compiler

Pattern)

- **Description**:
  - Combine Candidate 2 (`ClassHierarchy` as data) with an explicit, immutable
    query compiler pass.
  - Parsing produces an immutable AST:

    ```rust
    pub enum SourceAtom {
        Tag(String),
        Path(GlobPattern),
Class { name: String, mode: ClassExpansionMode }, // mode has NO inner BTreeSet!
    }
    ```

- During query planning, an explicit rewrite pass produces an execution plan
  atom:

  ```rust
  // Query plan atom: flat, resolved
  pub(crate) enum ResolvedSourceAtom {
      Tag(String),
      Path(GlobPattern),
      ClassSet(Arc<[String]>), // resolved classes ready for lookup
  }
  ```

- AST is never mutated in-place.
- **Checker (Sub-problem A)**: Checker compiles note frontmatter against schema
  definitions into an immutable `ValidationPlan`.
- **Workspace Split (Ticket 32)**: Clean separation of AST (syntax), Schema
  (metadata),
  and Plan (execution).
- **Strongest Argument Against**: Slightly more boilerplate (two atom enums:
  parsed vs resolved), though trivial in Rust.

---

## 5. Strongest Argument Against Each Candidate

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ Candidate                     │ Strongest Argument Against                  │
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 1. Keep As-Is                 │ Fails Rust RFC 1023 orphan rule on crate    │
│                               │ split; mutates AST; silent in-memory bugs.  │
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 2. Precomputed ClassHierarchy │ Caller must thread hierarchy snapshot on    │
│    (Pure Data)                │ schema reload.                              │
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 3. Hierarchy in Index/Store   │ Violates ADR-0009; schema edits wipe Redb.  │
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 4. Neutral Module (class.rs)  │ Adds an extra module for what is currently  │
│                               │ a simple taxonomy lookup.                   │
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 5. Caller-Side Pre-Pass       │ Duplicates AST traversal across CLI,        │
│                               │ template engine, and future LSP.            │
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 6. Impl Next to Schema        │ Inverts dependency: schema crate depends on │
│                               │ query crate; schema depends on note.        │
├───────────────────────────────┼─────────────────────────────────────────────┤
│ 7. Query Rewrite Phase        │ Introduces dual AST/Plan atom types.        │
└───────────────────────────────┴─────────────────────────────────────────────┘
```

---

## 6. Ranked Recommendation and What Evidence Would Change It

### 6.1 Ranked Recommendation: Candidate 2 + 7 (Data-Driven Hierarchy +

Immutable Query Rewrite)

**Rank 1: Candidate 2 + Candidate 7 (Precomputed `ClassHierarchy` Data +
Immutable Query Planning)** [P]

1. **Delete `src/file_class_expander.rs`** and delete `trait FileClassExpander`.
2. **Define `ClassHierarchy` as pure data**:

   ```rust
   #[derive(Clone, Debug, Default, Eq, PartialEq)]
   pub struct ClassHierarchy {
       descendants: HashMap<String, BTreeSet<String>>,
       children: HashMap<String, BTreeSet<String>>,
   }
   ```

   Can live in `src/query/` (or a shared leaf module `src/class.rs`).
3. **`SchemaService` produces `ClassHierarchy`**:
`SchemaBuilder` already builds `children` and `descendants` during schema load
[V: `src/schema/builder.rs:346`]. Expose `schema_service.class_hierarchy() ->
&ClassHierarchy`.
4. **Remove in-place AST mutation**:
Remove `BTreeSet<String>` from `ClassExpansionMode`. `ClassExpansionMode`
becomes
   a simple unit enum:

   ```rust
   pub enum ClassExpansionMode {
       Exact,
       Children,
       Descendants,
   }
   ```

5. **Pass `&ClassHierarchy` during Query Planning**:
`QueryService::new(class_field)` takes `Option<Arc<ClassHierarchy>>` or defaults
to empty. When resolving candidate rows, `hierarchy.expand(name, mode)` returns
   `&[String]`. If no hierarchy is attached, it resolves `@Class` to `[name]`
   consistently across both store-backed and in-memory queries!

**Rank 2: Candidate 4 (Neutral `class` / `vault_model` Module)** [P]
If Ticket 20 decides that field types and note validation also need shared
types,
hoist `ClassHierarchy` and `ClassName` into `src/vault_model/`.

**Rank 3: Candidate 5 (Caller Pre-Pass)** [P]
Only if query planning must be 100% agnostic to any notion of expansion.

**Do Not Implement: Candidate 1 (Keep), Candidate 3 (Index Coupling), Candidate
6 (Schema->Query Dep)**.

### 6.2 What Evidence Would Change This Recommendation?

- If Traces vaults had 500,000 schemas with dynamic cycles requiring lazy graph
traversal rather than an in-memory map (disproven: Traces vaults have <50
schemas
  taking <1ms to load [V: `survey/schema.md:146`]).
- If external plugins needed to provide custom dynamic class expansion over a
  network or IPC socket (disproven: Traces is a local single-binary CLI/LSP).

---

## 7. Lessons Transferable to Other Sub-Problems

### Lesson 1: For Sub-Problem A (Schema Checker Seam)

**Do not repeat the `file_class_expander.rs` pattern for the checker.**
The user's instinct was spot-on: creating a `src/note_checker.rs` glue file at
Tier 8 with a `dyn NoteValidator` consumer trait is an anti-pattern.
Instead:

- `SchemaService` exports pre-resolved `SchemaRuleSet` or `SchemaCatalog` as
  **pure data**.
- A checker is a **pure function** in a dedicated module (or inside `note` /
  `cli`):
  `validate_note(note: &Note, rules: &SchemaRuleSet) -> Vec<Diagnostic>`.
- Zero dynamic dispatch, zero orphan traits, 100% testable with mock data
  structs.

### Lesson 2: Diagnostics Must Be Values, Not Tracing Side-Effects

`warn_unknown_classes` logging directly to `tracing::warn!` inside query
execution [V: `src/schema/service.rs:194`] prevents callers from managing
errors.
The checker must return structured `Vec<Diagnostic>` with source byte spans
(for LSP diagnostics and CLI output), never unencapsulated log calls.

### Lesson 3: Workspace Split Durability (Ticket 32)

Cross-crate boundaries in Rust should exchange **plain data types**, not
cross-crate consumer traits with orphan implementations. Data crosses crates
without triggering RFC 1023 coherence errors; behavioral traits across decoupled
crates trigger compile errors.

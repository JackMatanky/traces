# Crate evidence for ticket 29: query, plan and encoding crates

Crate facts come from `rust-docs-mcp` against the cached versions. Each finding
names the concept that it informs.

## A. Pushdown and plan shapes (C2, C3)

1. **datafusion-expr 55.2.0.** [V] `TableProviderFilterPushDown` is a public
   enum (`Exact`, `Inexact`, `Unsupported`) beside `TableSource`;
   `utils::split_conjunction` splits top-level AND terms for the planner.
   Lesson for 29: C3 — the ternary judgment and conjunction splitting are
   library-shaped, small surfaces. A Traces pushdown protocol can mirror this
   without a relational engine.

2. **gluesql-core 0.20.0.** [V] `data::key::Key::to_cmp_be_bytes` produces
   order-preserving big-endian key bytes; the planner asks storage for index
   ranges through `store::index`; `gluesql-redb-storage` adapts it to redb.
   Lesson for 29: C3 — a storage engine already ships order-preserving
   encodings over redb. Ticket 29 can cite it for range-key design; ticket 32
   owns the path bytes.

3. **salsa 0.28.5.** [V] Memoized queries with revision invalidation.
   Lesson for 29: C6 — memoized terminal access has a precedent shape, but
   Traces needs memoized typed errors, not silent reuse.

## B. Grammar and values (C1, C4, C12)

4. **minijinja 3.0.1.** [V] `machinery::parse` sits behind `unstable_machinery`;
   AST nodes are `Spanned`; `Value::from_object` and `Object::call_method` carry
   host types; number comparison has `cmp_f64` with total-order handling.
   Lesson for 29: C10 — spans survive on AST nodes. The VM stamps host errors.
   The template bridge decision (ticket 31) can rely on this.

5. **logos 0.16.1.** [V] The lexer generator already powers both grammars
   (`src/query/grammar/`). Unification would share one token stream and two
   atom parsers.
   Lesson for 29: C1 — grammar unification is a lexer-level change first; the
   boolean precedence module (`src/query/grammar/expr.rs`) is already shared.

6. **noyalib 0.0.57.** [V] YAML value parsing with compat config; already the
   frontmatter boundary.
   Lesson for 29: C12 — literal candidate parsing for quoted values should
   reuse the date and duration parsers of the note boundary (ticket 19 rule).
   Do not add a second parser.

## C. Storage encoding (C3, C13)

7. **redb 4.3.0.** [V] Tables order keys by byte comparison; ranges over
   `&[u8]` keys are native.
   Lesson for 29: C3 — folder path ranges and tag key lookups are byte-range
   scans; correctness depends on the persisted order, which ticket 32 fixes.

8. **postcard 1.1.3.** [V] Schema-dependent compact encoding; facet rows keep
   derived serde (ticket 18 rule).
   Lesson for 29: C13 — a range key encoding must be measured (ticket 24 open
   item) before the resolution names it.

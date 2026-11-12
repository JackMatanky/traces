# Digest evidence for ticket 29: query language and plan precedents

This file records precedents for the grilling of ticket 29. It does not decide
the ticket. Each finding names the concept that it informs. Citations use line
numbers in the digest files under `docs/digests/`.

## A. Grammar shape (C1)

1. **Dataview keeps two stages.** [V] `FROM` resolves candidate paths through
   metadata indices. `WHERE` evaluates row expressions on decoded pages
   (`docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:4600-4649,4715-4733`).
   Lesson for 29: C1 — a two-stage split is a user-facing convention, not a
   storage requirement. Research 06 showed SQLite and DataFusion use one
   predicate language.

2. **Obsidian Tasks uses line regexes.** [V] No AST, no boolean combinations,
   no arithmetic (`docs/digests/obsidian_obsidian-tasks-src-digest.txt`, query
   parser sections).
   Lesson for 29: C1 — the floor grammar loses expressions Traces already ships.
   Do not simplify below the current DSL.

3. **zk builds SQL predicates.** [V] Exact text uses `LIKE`, FTS uses `MATCH`,
   tags use relations, and the builder combines predicates with `AND` plus sort
   and limit (`docs/digests/zk-src-digest.txt:9404-9471,9475-9495,9664-9732`).
   Lesson for 29: C2 — one builder can map each source shape to a different
   store access without two user languages.

## B. Plan and pushdown (C2, C3)

4. **DataFusion ternary pushdown.** [V] `supports_filters_pushdown` returns
   `Exact`, `Inexact` or `Unsupported`. `Inexact` keeps a residual filter above
   the scan (research 06; `datafusion_expr` docs).
   Lesson for 29: C3 — the candidate-superset rule has a named precedent shape.
   Traces needs the same ternary judgment per predicate.

5. **zk column selection.** [V] `NoteDAO` selects minimal or full note columns
   through one `findRows` method
   (`docs/digests/zk-src-digest.txt:9264-9315,9701-9732`).
   Lesson for 29: C7 — one row interface can serve different decode depths.

6. **Dataview inverse maps.** [V] Tag sources read `tags.getInverse`; folder
   sources walk a prefix structure; negation starts from every path
   (`docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:4275-4279,4600-4617,4671-4678`).
   Lesson for 29: C3 — each source shape has a distinct candidate access.
   Traces tickets 24 already assigns them; the plan maps predicates to those
   accesses.

## C. Rows, values and serialization (C5, C7, C11)

7. **IWE serialisable query document.** [V] MongoDB-style YAML or JSON with
   `find`, `count`, `update`, `delete`; a published JSON Schema; no pushdown
   (research 13).
   Lesson for 29: C11 — a serde IR lets hosts build queries as data. Traces
   must weigh this against DSL-string freeze rules from ticket 16.

8. **IWE value semantics converge with ticket 19.** [V] Strict scalar types,
   cross-type `==` false, ICU natural collation, rank-based total order
   (research 13, `crates/liwe/src/query/filter.rs`, `sort.rs`, `collation.rs`).
   Lesson for 29: C4 — an independent engine landed the same comparison rules.
   Traces owns period membership and adaptive literals that IWE lacks.

9. **Dataview GROUP BY restructures rows.** [V] Grouping changes row schema to
   `{ key, rows }` and later stages address `rows.field`
   (`docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt`, query
   engine `group` case).
   Lesson for 29: C4 — real grouping changes row cardinality. The Traces
   `GroupBy` sort alias is neither nesting nor removal; the ticket must pick.

## D. Task and list rows (C8)

10. **Tasks derives the heading above a task.** [V] It walks the cached heading
    list up to the task line and returns none without the cache
    (`docs/digests/obsidian_obsidian-tasks-src-digest.txt:10373-10387`).
    Lesson for 29: C8 — ticket 23 removed the stored heading link; the plan
    derives it from heading ranges. This precedent shows the derived form
    works.

11. **Tasks keeps one task array with state.** [V] `Cold`, `Initializing`,
    `Warm` states gate reads
    (`docs/digests/obsidian_obsidian-tasks-src-digest.txt:9971-10010`).
    Lesson for 29: C6 — laziness contracts need named states so a consumer
    never sees a silent partial result.

# Web precedent for ticket 29: plans, pushdown and encodings

Each URL below was fetched for this prep. `[V]` marks a fact in the fetched
source. `[H]` marks an implication for ticket 29.

## A. Order-preserving key encodings

1. **SQLite4 key encoding.** [V] Keys are built so `memcmp` sorts them into
   proper order; NULL gets the lowest byte, text ends with 0x00 so a prefix
   sorts before its extensions, numbers use a sign-and-exponent scheme
   (`https://sqlite.org/src4/doc/trunk/www/key_encoding.wiki`).
   Lesson for 29: C3 — range keys for scalar fields need exactly this
   discipline. The ticket 24 gate (prove candidate superset) becomes decidable
   when the encoding keeps the filter relation's order.

2. **CockroachDB column encoding.** [V] Each type has one canonical byte
   encoding with the property enc(x) <= enc(y) iff x <= y for ascending
   columns, and prefix-free composable fields
   (`https://github.com/cockroachdb/cockroach/blob/master/docs/tech-notes/encoding.md`).
   Lesson for 29: C3 — composite keys (row mode + field + value) need
   prefix-free encoding so a range over the value does not leak across fields.

3. **gluesql over redb.** [V] `Key::to_cmp_be_bytes` plus the redb storage
   adapter show a working SQL index-range layer over redb
   (cached rust-docs, `gluesql-core` and `gluesql-redb-storage`).
   Lesson for 29: C2 — a small planner over byte-ordered keys is a shipped
   design at Traces scale.

## B. Pushdown protocols

4. **DataFusion filter pushdown.** [V] `Exact` removes the filter from the
   plan; `Inexact` prunes and keeps a residual filter; `Unsupported` scans
   (research 06; `datafusion_expr` docs).
   Lesson for 29: C3 — the ticket's "candidates must contain every match" rule
   is `Exact`-or-scan; `Inexact` prunes never decides. The protocol design can
   be three named judgments per predicate.

5. **Polars predicate pushdown.** [V] Predicates accumulate downward and attach
   at scans that allow pushdown (research 06).
   Lesson for 29: C3 — accumulation order matters for `limit` and `flatten`;
   the equivalence proof per row mode is the Traces-specific part.

## C. Grammar and IR serialization

6. **IWE query document.** [V] A YAML/JSON query document with a published
   schema lets non-CLI hosts build queries as data (research 13).
   Lesson for 29: C11 — a serde IR is the alternative to string synthesis for
   hosts. Ticket 16 makes DSL spelling migratable. An IR can become the stable
   surface with the DSL as one frontend.

7. **Dataview group semantics.** [V] `GROUP BY` restructures rows into
   `{ key, rows }` and changes row identity for later stages
   (research 06; digest).
   Lesson for 29: C4 — if `GroupBy` stays, it must either nest (a new row
   shape) or be renamed to `sort`. The current sort alias is documented as
   deceptive.

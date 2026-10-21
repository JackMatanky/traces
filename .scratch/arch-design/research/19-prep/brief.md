# Pre-grilling brief: ticket 19 (coercion and comparison policy)

Inputs: [a-code-inventory](./a-code-inventory.md), [b-digest-lessons](./b-digest-lessons.md),
[c-external-precedent](./c-external-precedent.md), [d-prior-decisions](./d-prior-decisions.md),
plus tickets 16-18 and `research/03`. Tags: [V] verified in a source I or a scout read, [H]
inference, [P] proposal. Nothing here is a decision; it is the menu for the grilling.

## 1. Ground truth (all [V], from the code probes in `a-code-inventory.md` section 3)

The crate has **three equality evaluators and two orderings**, and they disagree:

| Case | derived `PartialEq` | `compare()` (`note/field.rs:286`) | filter `==` (`is_equal_to_literal`) | `SortKey` (`query/sort.rs`) |
|---|---|---|---|---|
| `Date` vs same-day midnight `DateTime` | false | Equal (ambient zone) | true (ambient zone) | by ambient-zone key |
| same, but zone differs between index and query | false | **Date > DateTime** | false | (same leak) |
| `[[x]]` vs `![[x]]` | false | **Equal** | false | Text(target) |
| `NaN` vs `NaN` | false | **Equal** | false | n/a |
| `Int(1)` vs `Float(1.0)` | false | n/a today (`Number(f64)` only) | n/a | n/a |
| `1mo` vs `30d` | true | Equal | n/a | Equal (nominal seconds, `1y`=365d) |
| `List`/`Object` in sort | n/a | rank 7/8 (last) | n/a | **mapped to Null (first)** |

Filter semantics today are already half type-bracketed and half SQLite-style:

- `==` across kinds is `false` (`42 == "42"` false); `!=` is its complement.
- `<`, `<=`, `>`, `>=` poison `Null` (always false), but across non-null kinds they **leak the rank table**:
  `v < "42"` matches bool/date/dur/num rows, `v > "42"` matches link/list/obj/str rows, `true < 42` is true.
- Three promotion seams, none coordinated: frontmatter classify (`field.rs:843`), filter literal classify
  (`filter.rs:238`), sort-time `TextShape::classify` (`sort.rs:462`). The sort-time one means a **String** field
  holding `"2026-07-29"` sorts as a DateTime today.
- Naive datetimes are shifted to UTC with the ambient zone at parse and **persisted** as `...Z` text; Date to
  DateTime promotion at compare time uses the *current* ambient zone. So the cache bakes in the indexing
  machine's zone while the comparator uses the querying machine's zone.
- No cast functions appear in the inventory [H: the query grammar has arithmetic but I saw no `date()`/`string()`];
  every "explicit cast" option below presupposes new grammar owned by ticket 26. Check before grilling.

## 2. Corrections to scout output and earlier notes (do not carry these forward)

1. CodeInventory says lexical storage (1C) is "rejected by ticket 18". It is not: ticket 18 lists it as
   alternative D and defers it to this ticket (`18:321-323`, `18:463`). It is a live option.
2. DecisionSynthesis "Challenge 1" says Noyalib exposes scalar style so frontmatter could just honour quotes.
   The parser's `ScalarStyle` is `pub(crate)` (`noyalib-0.0.53/src/parser/scanner.rs:21`); the public
   `ser::ScalarStyle` is output-side. Ticket 18 already measured that only the CST route exposes style and that
   `span_at(path)` is unsound for dotted keys, tags and block scalars (`18:283-288`). The *rule* (quoted
   date/duration is a string) is cheap; the *mechanism* is the unsolved part.
3. DecisionSynthesis cites "ADR-0009" for bare date literals. The map says ADR-0009 is index refresh. Treat as unverified.
4. DigestLessons suggests `1yr = 365.25d`; the code uses 365d (`temporal/unit.rs:194-224`, per CodeInventory).
5. ExternalPrecedent's PostgreSQL `1yr == 360d` anomaly does not apply (Traces uses 365d). The real wart is
   `1mo == 30d` for ordering while `+ 1mo` is calendar arithmetic.
6. DecisionSynthesis proposes a custom `PartialEq` equating `Int`/`Float`. ExternalPrecedent shows why not:
   `Eq`/`Hash` coherence (`Int(1)` and `Float(1.0)` would need equal hashes), and ticket 18 already chose
   structural. Semantic numeric equality belongs in the comparator, which already exists as a separate path.
7. `src/temporal/GLOSSARY.md:24-25` enshrines "Date promotes to local midnight". It must change whichever way Q4 goes.

## 3. The cross-cutting idea that simplifies most questions [P]

Today one function (`compare`) is asked to be equality, filter ordering and sort order at once. Separate five jobs:

| Job | Owner | Property |
|---|---|---|
| Classify (text to typed value) | note boundary, one precedence order (ticket 18) | context-free, at parse |
| Cast (explicit, query-time) | query binder | fallible; failure = Null |
| Loose equality (filter `==`) | one value-semantics module | false across brackets, no ambient state |
| Bracketed order (filter `<`..) | same module | `Option<Ordering>`; `None` across brackets means no match |
| Total sort order | `SortKey` built **with** a context | zone/now enter when the key is built, never inside `Ord` |

`NoteFieldValue: PartialEq` stays structural, with no `Ord` and no `Hash`. This is DataFusion's split
(`ScalarValue` partial_cmp is `None` across types; coercion is planner-side) and MongoDB's split (total BSON order
for sort, type bracketing for filters). It deletes the three-evaluator disagreement table above by construction.
Behaviour change to record: relational filters across brackets stop matching (today they match by rank).

## 4. Question by question

Each: options, my lean, what to stress-test. "Keep" = status quo.

### Q5 + Q8: filter relation vs sort order, and sniffing (grill first; everything hangs on it)

- A keep: `==` strict, `<` leaks rank, sort sniffs strings. Confusing; `v < "42"` match sets are meaningless.
- B Mongo split [lean]: brackets Numeric{Int,Float}, Temporal (per Q4), Duration, String, Bool, Link, List, Object,
  Null. Cross-bracket `==` false, `!=` true, relational no-match. Rank table survives for sort only.
- C erroring filters (CEL): rejects `--where` on heterogeneous vaults; open-world notes make this hostile.
- Sort fixes under B: delete `TextShape::classify` from `SortKey`; fix List/Object-to-Null; one rank table.
  Pick which table: code today is `Null<Bool<Number<Duration<Temporal<Text<Link<List<Object`, the retired
  temporal-core register says `...<String<Date<DateTime<Duration...`. [P] keep the implemented one.
- Literal promotion (`filter.rs:238`) is separate from sniffing: under B a string literal against a `Date` field
  must still work. Options: typed literal syntax; operand-adaptive coercion (literal adopts the other operand's
  kind when it parses, SQLite column-affinity style, deterministic given the row's value); keep parse-time
  literal sniffing. Stress: does `title == "2025-01-15"` on a string field still behave?
- Null placement (Dataview nulls first, Tasks nulls last): small, likely belongs to ticket 26; flag it.

### Q4: Date vs DateTime, zone (needs ticket 21's context shape; decide the principle here)

- T1 keep ambient local midnight: rejected by probe (non-deterministic) and by ticket 19's own text.
- T2 Date as UTC midnight: pure, but wrong for a user whose `12:00` means local wall clock.
- T3 civil model: naive datetimes stored as civil `DateTime` (no zone); `Date` compares as civil midnight;
  only offset-bearing values (`Z`, `+02:00`) are instants. Civil vs instant (e.g. against `file.mtime`) needs a
  zone from the execution context, supplied when `SortKey`/filter operands are built. Costs a variant or an inner
  enum; size fits (16 B payload). Cache contains no machine state.
- T4 configured zone at parse: naive datetime resolved with an explicit config zone, stored as an instant;
  `ConfigDigests` already feeds the index fingerprint (`207cc3fb`), so a zone change would rebuild the cache.
  One `DateTime` variant, smallest change; ambient becomes config. DST gap/fold resolved once at parse.
- T5 incomparable: Date and DateTime are separate brackets; cross compare needs explicit cast.
- Consequence to surface: ticket 18 excluded inline `DateTime` *because* of the persisted zone shift. Under T3/T4
  the reason disappears, so `time:: 2026-07-29T14:30` and frontmatter `time:` can both be DateTime
  (temporal-core spec story 15: "where I write a date never changes what it means", `spec.md:43`).
- Stress: `Date` vs instant is "does this day contain that instant", which is zone-dependent under every option
  except T5; make the user say which zone defines a day (config zone vs per-machine).

### Q3: numbers

- N1 [lean]: structural `PartialEq`; one exact comparator for `Int`/`Float` (SQLite `sqlite3IntFloatCompare`
  / CEL: compare in the number line, no `i64 as f64`), `-0.0 == 0.0`, `NaN` equal to nothing in filters.
- N2: also make NaN unrepresentable (reject at construction: YAML `.nan`, inline text) so `Float` is never NaN.
  Check whether any parser path can produce NaN today; if none, the question is moot and free.
- N3: `OrderedFloat` in the enum: changes the variant payload and size budget for no query benefit.
- Sort: NaN (if it exists) last; ties are stable.

### Q1 + Q2: where promotion lives, and the frontmatter string escape

- C1 [lean as baseline]: keep classify-at-parse with ticket 18's single precedence order; remove sort-time and
  query-side sniffing (Q5/Q8). Smallest change, preserves zero-config date queries.
- C2 lexical-first: values stay `String`; Date/Duration appear via schema field type or explicit cast. Matches
  `research/03`, Obsidian Properties (`types.json`), DuckDB. Costs: zero-config `due < 2025-01-01` stops working
  without a schema; the cache would hold strings that are re-parsed per comparison unless memoised. Migratable
  per ticket 16, but it is the largest user-visible change on the table.
- C3 schema-informed override: schema `type = string` suppresses classification, `type = date` forces it. Requires
  the schema at parse time, so schema edits invalidate the index (the cost `research/03` calls out) unless
  classification stays schema-free and the schema only affects interpretation at read time.
- Escape (Q2) options: none, with a `string()`-style cast (Dataview's answer); quote-sensitive for
  Date/DateTime/Duration only, **Link exempt** (links must be quoted in YAML, so this is the exemption ticket 18
  asked for); `!!str` tag; schema override (C3).
  - Mechanism for quote-sensitivity is unsolved (section 2, item 2). Ticket 18 left a "lazy CST consult only for
    date/duration/link/blank-shaped plain-eligible strings" variant unevaluated (`18:221-223`); that is the one
    thing worth costing.
  - `!!str 2025-01-15` is cheap iff Noyalib 0.0.53 surfaces it as `Value::Tagged`; ticket 18 says tags are
    currently dropped to the inner value (`field.rs:793`). I did not verify how `!!str` is represented [H]; a
    two-line probe settles it before the grilling reaches this question.
  - Real-world pressure: no fixture relies on quoted frontmatter dates/durations (`18:289`); advanced-metadata-cache
    and Dataview both document users hitting the unquoted-number/title trap.

### Q6 + Q7: links

- L1 [lean]: equality and order on `ComparisonKey(target)` plus subpath (Dataview `Link.equals`; Obsidian, Marksman:
  display and embed are not identity). `display` and `embedded` stay queryable *attributes*, which also satisfies
  ticket 18's "user wants these distinguishable in queries". Needs one definition of "target" (does it include
  `#heading`/`^block`? `LinkTarget` decomposes it).
- L2 keep structural (eq includes `embedded` and `text`) while `compare` ignores `embedded`: the current disagreement.
- L3 resolved-note identity: needs the index at comparison time; rejects itself (comparisons must be pure).
- Link vs String (Q7): under B the brackets differ so `project == "Alpha"` is false. Options: strict (use
  `link("Alpha")` or write `[[Alpha]]`); or define one explicit reading, "String operand against Link compares as
  a wikilink target", which the schema select check in ticket 20 can reuse (parse `"[[Alpha]]"` with the shared
  parser, then Link vs Link). Stress: should `"Alpha"` also match `[[Alpha|Display]]` by display? [lean: no]
- Case folding: `ComparisonKey` (NFC + lowercase) exists for resolution; the link relation should reuse it rather
  than add a second folding.

### Q9: durations

- D1 [lean for this ticket]: keep nominal seconds (1mo=30d, 1y=365d) for equality/order, documented; calendar
  arithmetic stays Jiff-based. Dataview does the same. Deterministic and already coherent (`Eq`/`Ord`/`Hash`).
- D2 Jiff `SpanRelativeTo` / TC39 `relativeTo`: correct but needs an anchor date per comparison; breaks total sort.
- D3 fieldwise equality, nominal for sort: `1mo != 30d`, order still total; equality/order inconsistent.
- Ticket 27 owns temporal dialects; take D1 provisionally and say so.

### Q10: strings

- S1 [lean]: byte-exact `==` and sort (deterministic, zero-alloc).
- S2: sort key = simple Unicode case fold, then byte tiebreak (case-insensitive name sort without locale).
  Never host-locale collation (Postgres libc warning in `c-external-precedent.md` section 7).
- S3: filter-side `lower()`/case-insensitive operator; depends on function grammar (ticket 26).
- Ask the user how they expect `file.name` sorting to treat `Zebra`/`apple`.

### Q11: error mode

- E1 [lean]: classification failure keeps the string (already so); explicit casts fail to Null (TRY_CAST);
  filters never error; arithmetic on mismatched kinds yields Null (already `filter.rs:371`).
- E2: add advisory diagnostics (taplo pattern) for failed casts and cross-bracket comparisons; needs spans
  (ticket 23) and the diagnostic architecture (ticket 31), so decide only the *principle* here.
- E3 erroring filters: see Q5-C.

## 5. Handoffs this ticket must make explicit

| To ticket | What ticket 19 must pin down first |
|---|---|
| 21 | Principle: no ambient zone anywhere; a zone and `now` arrive as explicit inputs when sort keys/filter operands are built. Shape is theirs. |
| 20 | Is classification schema-free (lean) or schema-overridable; Link-vs-String reading for select matching. |
| 23 | Parse-time eager classification vs raw lexeme retention (C1 vs C2); multi-value elementwise. |
| 24/25 | Temporal cache encoding (text today, re-parsed every load) and whether a zone change invalidates the index (T3 no, T4 yes). |
| 26 | Relation split, typed literals or cast functions, null placement, `GroupBy` key relation. |
| 27 | Nominal duration order; Date vs DateTime kinds. |
| 28 | Template comparisons use the same relation module, not minijinja's own; number rendering (`5` vs `5.0`). |

## 6. Proposed grilling order (resolve a spine first, then branches)

1. Q5/Q8: filter relation vs total sort order, and whether sniffing dies. (Gates 26, 28.)
2. Q4 + Q9: temporal principle (T3 vs T4) with the Date/DateTime bracket. (Gates 21, 24, 25, 27.)
3. Q1/Q2: promotion seam and the frontmatter escape; first run the `!!str` probe and cost the lazy-CST variant.
4. Q3: numbers (short; confirm NaN reachability).
5. Q6/Q7: links.
6. Q10/Q11: strings and error mode (short).

## 7. Not verified here

- `!!str` handling in Noyalib 0.0.53, and whether any path can yield `NaN` (items above).
- Whether any cast/function grammar exists in the query language (ticket 26's territory).
- `git diff 207cc3fb..HEAD` shows drift only in `dirtree`, `template/engine/*`, tests and baselines; the value,
  note, query and temporal code the scouts read is unchanged. The probe test file was deleted; `git status`
  shows only `research/19-prep/` as new.

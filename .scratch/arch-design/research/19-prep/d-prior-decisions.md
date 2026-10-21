# Prior Decisions, Constraints, and Handoffs Bearing on Ticket 19

- **Topic:** Architectural audit of earlier decisions, constraints, compatibility boundaries, and cross-ticket dependencies for Wayfinder Ticket 19.
- **Status:** Complete.
- **Deliverable:** `.scratch/arch-design/research/19-prep/d-prior-decisions.md`.

---

## 1. Settled Constraints List (Tickets 01–18)

Ticket 19 must honour all settled invariant and architectural decisions from tickets 01–18:

1. **Two Owned Value Enums, Zero Borrowed Mirror Enums:** [V: `issues/18-value-model.md:447-450`] Runtime note metadata, queries, and templates share `note::NoteFieldValue`. Schema literals use a private `SchemaLiteral` (strings remain strings; no temporal/link variants) [V: `18:477-483`]. The borrowed enum mirror `NoteFieldValueRef` is eliminated crate-wide (retained temporarily only as a private query-local view pending benchmarking in Ticket 26) [V: `18:518-520,527-530`].
2. **Canonical Variant Topology of `NoteFieldValue`:** [V: `issues/18-value-model.md:471-476`] The runtime enum owns exactly 11 variants: `Null`, `Bool(bool)`, `Int(i64)`, `Float(f64)`, `String(String)`, `Date(DateValue)`, `DateTime(DateTimeValue)`, `Duration(DurationValue)`, `Link(Link)`, `List(Box<[Self]>)`, and `Object(IndexMap<String, Self>)`. `Number(f64)` is removed [V: `18:529`]. Postcard size remains bounded by 80 bytes unboxed [V: `18:474-475`].
3. **Derived Structural `PartialEq` on `NoteFieldValue`:** [V: `issues/18-value-model.md:438-439,474`] Derived Rust `PartialEq` remains purely structural (variant tag + payload equality). Cross-kind equality and numeric equivalence (`Int` vs `Float`) are explicitly decoupled from `PartialEq` and delegated to query evaluation / comparator logic in Ticket 19 [V: `18:438-439,541`].
4. **Unified Numeric Rank & Value Equivalence:** [V: `issues/18-value-model.md:507-511`] `Int(i64)` and `Float(f64)` are preserved across note parsing, schema data, and query computed fields (`file.size`, `list.line`, etc.). `Int` and `Float` share a single numeric rank and compare by mathematical value [V: `18:511`].
5. **Root `Link` Canonical Struct:** [V: `issues/18-value-model.md:484-490`] `Link { target: String, display: Option<String>, embedded: bool }` is the sole shared link primitive. `display` is normalized to `None` if absent, empty, or equal to unescaped target [V: `18:484-486`]. `NoteLink` (carrying AST source span and syntax kind) is deferred to the editor document model [V: `18:489-490; issues/23:33-37`].
6. **Frontmatter Ingestion Engine:** [V: `issues/18-value-model.md:492-498`] Frontmatter retains Noyalib compat parsing, converting `noyalib::Value` directly into `NoteFieldValue` without intermediate `FieldValueRef`. `integer_overflow_errors` is disabled in YAML so integers $> 2^{64}-1$ convert to approximate `Float` without dropping the frontmatter block [V: `18:493-496`]. YAML tags are ignored and merge keys are plain keys [V: `18:497-498`].
7. **Unified Classification Precedence & Quote Doctrine:** [V: `issues/18-value-model.md:500-505`] Frontmatter and inline fields share the same date, duration, link, and blank parsers in one precedence order. Frontmatter classification is quote-insensitive; inline quotes are the literal string escape. Inline fields do NOT gain `DateTime` [V: `18:501-503`].
8. **Schema Boundary Interpretation:** [V: `issues/18-value-model.md:481-482,524-525`] Schema select options storing wikilink syntax remain literal strings in `SchemaLiteral`; conversion/interpretation to `Link` occurs strictly at the consumer boundary via the root `Link` wikilink parser in Ticket 20.
9. **Elimination of Ambient Zone in Ordering:** [V: `issues/01:44-48; research/02:76-80; research/03:77`] `NoteFieldValueRef::compare` promoting `Date` to `DateTime` via ambient `local_zone()` was verified as non-deterministic and inconsistent with value equality. Total ordering must be pure, deterministic, and free of system-clock/timezone syscalls [V: `research/03:77,107-109`].

---

## 2. Compatibility-Envelope Digest (Ticket 16)

Ticket 16 establishes the operational compatibility envelope for the redesign:

- **The Sole-Consumer Rule:** [V: `issues/16-grilling-compatibility-envelope.md:24-25`] *"The user is the only consumer. No published release, no external Rust crate depends on the library. Every surface is therefore at most Migratable unless it would break the user's own vault files."*
- **The Visibility Rule:** [V: `issues/16:47`] Reduce visibility wherever possible and limit `pub` to outward-facing components; widening requires outward-facing justification.
- **Oracle & Change Mechanism:** [V: `issues/16:26-27`] `tests/e2e/*` and vault parsing tests serve as the oracle. Any observable behavior change must be documented in the redesign specification's "behaviour changes" table with an accompanying migration; undocumented observable changes are regressions.
- **Classification of Surfaces Bearing on Ticket 19:**
  - **Vault-Format Parsing (Frozen):** [V: `issues/16:30`] Markdown files must parse to the same semantic meaning (`---` frontmatter, `Key:: Value` inline fields, `[[wikilinks]]`, tasks, tags). Classification of note text cannot silently invert.
  - **Index Equivalence (Frozen):** [V: `issues/16:32`] Index query results must strictly match a fresh parse of raw vault files and configuration.
  - **Template / Query Capabilities (Frozen):** [V: `issues/16:42`] Querying notes, lists, tasks, date arithmetic, and `--from`/`--where` filtering capabilities must remain reachable.
  - **Query DSL Spelling & Coercion Rules (Migratable):** [V: `issues/16:43`] *"Template function/object API and query DSL spelling, incl. heuristic string-to-date promotion: Migratable (Coercion policy is open to redesign)."* Ticket 19 is explicitly authorized to alter or eliminate heuristic coercion.
  - **Dataview / Templater Parity (Free):** [V: `issues/16:31`] Historical influence only; Traces does not owe bug-for-bug or semantic parity to Dataview or Templater. Cross-kind sort rank is free to be redesigned.
  - **Known Defects (Not Preserved):** [V: `issues/16:44`] Integer truncation to `f64` at note boundary (`src/note/field.rs:545`), closed-pipe panic on EPIPE, and `.first()`-only multi-value reads are defects; fixing them is expected.
  - **Rust Library Surface (Free):** [V: `issues/16:45`] Baselines (`tests/baselines/*.txt`) and `traces::testing` exports are change detectors, not an external API contract.

---

## 3. Forward-Consumer Constraints (Ticket 17)

Forward-looking requirements from Ticket 17 constrain coercion and comparison:

1. **LSP Committed, MCP Near-Term:** [V: `issues/17:37-41`] The shared core must support an LSP server as the next major consumer. Coercion and type checking must be callable on transient, in-memory live buffers without disk I/O [V: `17:43-44; issues/34:9-14`].
2. **Source-Aware Diagnostics:** [V: `issues/17:43-44`] Editor features require byte-accurate source spans (`ByteSpan`). Type mismatches, failed casts, and malformed dates/durations must retain source coordinates for LSP diagnostic publishing (`publishDiagnostics`) [V: `research/03:48-57,110-111`].
3. **Synchronous Core Invariant:** [V: `issues/17:57-58,76-77`] Domain analysis, parsing, and query execution remain synchronous (using Rayon for data parallelism). All comparators, coercion functions, and cast operations must be strictly synchronous, thread-safe (`Send + Sync`), and free of async runtimes.
4. **Performance Budgets Across Seven Vault Sizes:** [V: `issues/17:51-55`] The engine must scale across 50, 200, 1k, 5k, 10k, 20k, and 50k notes. In inner query loops over 50k notes, sorting and filtering comparators cannot allocate heap memory, invoke OS timezone syscalls, or parse regexes repeatedly [V: `research/11:163-172`].
5. **No General Write-Back Scope:** [V: `issues/17:44,99`] Note AST write-back is out of scope. Coercion does not need to maintain AST round-trip whitespace or formatting trivia.

---

## 4. Open-Ticket Dependency Map (Tickets 20–30)

Ticket 19 sits at the nexus of the type system, temporal domain, and query engine:

```text
[Ticket 18: Value Model] (resolved)
        │
        ▼
[Ticket 19: Coercion & Comparison Policy]
 ├──► [Ticket 20: Schema Note Typing & Validation] (blocked by 19)
 ├──► [Ticket 21: Ambient Environment & Execution Context] (supplies TimeZone/now)
 ├──► [Ticket 23: Note Document Model] (spans, multi-values)
 ├──► [Ticket 24: Index Read Model & Query Execution] (redb value storage)
 ├──► [Ticket 25: Index Freshness & Invalidation] (cache invalidation on zone shift)
 ├──► [Ticket 26: Query Core & Plan] (blocked by 19; filter ==, sort total order)
 ├──► [Ticket 27: Temporal Boundary & Dialects] (blocked by 19; duration ==)
 ├──► [Ticket 28: Template Engine Boundary] (minijinja value bridge)
 └──► [Ticket 30: Workspace Host & Composition] (session context threading)
```

- **Ticket 20 (`schema-note-typing-and-validation.md`):** [V: `issues/20:5,11-14`]
  - *Needs from 19:* The coercion policy (does schema drive coercion at parse time, index time, or query time? what is the error mode: null, diagnostic, or query failure?); the select string vs note `Link` matching rule [V: `20:11-14,33-37`].
  - *19 needs from 20:* Whether schema types override lexical inference or act purely as an advisory validation pass [V: `19:26-28; 20:11-14`].
  - *Ordering Trap:* 20 is explicitly blocked by 19. If 19 leaves schema-driven coercion undefined, 20 cannot resolve how schemas interact with runtime notes.
- **Ticket 21 (`ambient-environment-and-execution-context.md`):** [V: `issues/21:9-18`]
  - *Needs from 19:* Specification of context needed by comparators (is `Ord` completely zone-free? does query evaluation take `&ExecutionContext` containing `TimeZone` and `now`?) [V: `19:19-21,30-32`].
  - *19 needs from 21:* An explicit injected context mechanism to eliminate `local_zone()` and `ZoneGuard` [V: `21:15-18; issues/04:109-112`].
  - *Ordering Trap:* 19 cannot depend on ambient system state; it must provisionally specify that contextual comparisons take an explicit `&TimeZone`, leaving the struct shape to 21.
- **Ticket 23 (`note-document-model.md`):** [V: `issues/23:9-16`]
  - *Needs from 19:* Whether note parsing eagerly coerces strings or preserves raw lexical tokens; whether multi-valued inline fields coerce element-wise [V: `23:12-14`].
  - *19 needs from 23:* The `NoteLink` struct definition (editor span vs root `Link`) and multi-value storage representation [V: `18:489,548-549`].
- **Ticket 24 (`index-read-model-and-query-execution.md`):** [V: `issues/24:9-16`]
  - *Needs from 19:* Whether values stored in redb secondary tables are pre-coerced typed scalars or lexical strings; ordering of secondary index keys [V: `24:14-16`].
  - *19 needs from 24:* Storage format constraints (postcard serialization overhead of dynamic types) [V: `18:550-552`].
- **Ticket 25 (`index-freshness-and-invalidation.md`):** [V: `issues/25:13-31`]
  - *Needs from 19:* Invalidation rules for zone-shifted datetimes: if naive frontmatter datetimes are shifted to UTC at parse time, does host timezone change invalidate the index? [V: `18:546-547; 25:13-24`].
  - *Ordering Trap:* Storing zone-shifted UTC datetimes in redb bakes machine-local state into the persistent cache, violating cache portability across machines [V: `research/05:114-121`].
- **Ticket 26 (`query-core-and-plan.md`):** [V: `issues/26:5,9-18`]
  - *Needs from 19:* Total order lattice for `sort`; cross-type equality semantics (`==`) for filter evaluation; coercion rules for literals (`ComparisonExpr::classify_literal`); `TRY_CAST` vs `CAST` semantics [V: `19:18-24; 26:9-18`].
  - *19 needs from 26:* Query grammar expression capabilities (e.g., presence of `date()` / `dur()` functions).
  - *Ordering Trap:* 26 is blocked by 19. Query filter execution cannot be designed until 19 decides if mismatched equality returns `false`, `Null`, or emits an error.
- **Ticket 27 (`temporal-boundary-and-dialects.md`):** [V: `issues/27:5,9-17`]
  - *Needs from 19:* Equality and ordering semantics for `DurationValue` (does `1mo == 30d` hold in query comparison?); Date vs DateTime equality rules [V: `19:10-12; 27:9-17`].
  - *19 needs from 27:* Final temporal type definitions from `src/temporal` (Jiff wrappers) [V: `27:9-16`].
- **Ticket 28 (`template-engine-boundary.md`):** [V: `issues/28:5,9-18`]
  - *Needs from 19:* Minijinja dynamic value bridging: how `NoteFieldValue` behaves under template comparison operators; number rendering (`5` vs `5.0`) [V: `18:515-516; 28:9-18`].
  - *Ordering Trap:* Minijinja historically stringified durations and links (`src/template/engine/query.rs:604`); 19's equality model must coordinate with template operator expectations.
- **Ticket 30 (`workspace-host-and-composition.md`):** [V: `issues/30:9-17`]
  - *Needs from 19:* Evaluation session requirements (threading `ExecutionContext` through query services).

---

## 5. De-duplicated Unresolved Items Handed to Ticket 19

1. **Frontmatter Literal-String Escape:** YAML quotes are quote-insensitive (`title: "2025-01-15"` is parsed as `Date`), whereas inline quotes escape (`title:: "2025-01-15"` is `String`). How does an author force a literal string in frontmatter? [Source: `issues/18:37-38,540-541; issues/19:36-37`].
2. **`Int` vs `Float` Semantic Equality & Ordering:** `NoteFieldValue` has derived structural `PartialEq` (`Int(5) != Float(5.0)`). How do `Int` and `Float` compare losslessly in queries, sorts, and filters? Exactness for integers $> 2^{53}$, NaN handling, and $-0.0$ normalization [Source: `issues/18:38-39,511,541; issues/19:38-40; research/02:120-123`].
3. **`Link` Equality and Ordering:** Comparing `target`, `display`, and `embedded`. Does `embedded` (`![[note]]`) affect equality with standard `[[note]]`? Textual identity vs resolved target note identity [Source: `issues/18:40,484-486,541-542; issues/19:40`].
4. **Schema Select String vs Note `Link` Matching:** Schema select options store literal strings (`"[[Project A]]"` or `"Project A"`), while notes store parsed `NoteFieldValue::Link`. How do filters like `project == "Project A"` match? [Source: `issues/18:41-42,481-482,542-543; issues/19:41-42`].
5. **Zone-Shifted Naive Datetimes & Ambient Zone in `Ord`:** Naive frontmatter datetimes (`12:00`) are shifted by ambient local zone to UTC (`10:00Z`) at parse time and stored in the postcard cache; `NoteFieldValueRef::compare` calls `DateTimeValue::try_from(*a)` using ambient local zone, which breaks `Ord` determinism across machines and disagrees with derived `PartialEq` [Source: `issues/18:41-42,542; issues/19:19-21,41; issues/01:16,44-48; survey/temporal.md:60-63`].
6. **Cross-Kind Rank Order and Equality Lattice:** Current sort rank is `Null < Bool < Number < Duration < Temporal < Text < Link < List < Object` (`src/query/sort.rs:425-439`), while divergence register Entry 4 recorded `Null < Bool < Number < String < Date < DateTime < Duration < Link < List < Object`. How do `<` and `==` behave across different types? Is cross-type equality always false/null, or does it coerce? [Source: `issues/19:21-24; .scratch/temporal-core/divergence-register.md:294-299; survey/query.md:76-79`].
7. **Query Literal Promotion and Sort-Time Sniffing:** `src/query/grammar/filter.rs:238` promotes literals heuristically; `src/query/sort.rs:462` runs `TextShape::classify` on strings at sort time. Should lexical sniffing be eliminated in favor of explicit casts (`date(...)`, `dur(...)`) or schema typing? [Source: `issues/19:18-19; survey/query.md:76-79; research/03:102-104`].
8. **Duration Comparison with Calendar Units:** `DurationValue` encapsulates Jiff `Span` + total seconds. `1mo == 30d` for equality/ordering in nominal regime, but `+ 1mo` differs from `+ 30d` on calendar. How to handle duration comparisons when reference dates are absent? [Source: `issues/19:23-24; .scratch/temporal-core/divergence-register.md:287-293; src/GLOSSARY.md:41-51`].
9. **String Collation & Unicode Normalization:** How do strings compare in filters and sorts? Case-sensitive vs case-insensitive; NFC vs NFD; interaction with wikilink Comparison Key (`NFC + to_lowercase`) [Source: `issues/19:Q10; src/GLOSSARY.md:107-116`].
10. **Strictness-by-Seam Spread & Error Mode on Failed Coercion:** Four owners currently apply different strictness (`.scratch/temporal-core/divergence-register.md:329-334`). When coercion fails, does it yield `Null` (DuckDB `TRY_CAST`), boolean `false`, an advisory diagnostic, or a fatal query error (DuckDB `CAST`)? [Source: `issues/19:23-24,Q11; research/03:24-27,93-96`].

---

## 6. Apparent Tensions and Inconsistencies Among Earlier Decisions

1. **Quote-Insensitive Frontmatter vs Value Representation Truthfulness:**
   - *Tension:* Ticket 18 ruled: *"Frontmatter classification is quote-insensitive (a link must be quoted in YAML); inline quotes remain the literal escape"* [V: `issues/18:501-502`]. But Research 03 established: *"Value Representation Truthfulness: Values reflect source syntax (string is string; number is number). No lossy guessing during raw ingest... frontmatter and inline fields should remain truthful strings unless coerced by a known schema field type or explicit query function"* [V: `research/03:76,102-104`].
   - *Impact:* Quoted strings like `"2025-01-15"` in frontmatter lose their string identity and become dates without author consent, leaving frontmatter with zero literal string escape hatch.
2. **`Int`/`Float` Value Comparison vs Derived Structural `PartialEq`:**
   - *Tension:* Ticket 18 mandated: *"Int and Float share one numeric rank and compare by value... derived structural PartialEq"* [V: `issues/18:474,511`].
   - *Impact:* In Rust code, `NoteFieldValue::Int(5) == NoteFieldValue::Float(5.0)` evaluates to `false`. But in query filter expressions, `5 == 5.0` must evaluate to `true`. This establishes two conflicting definitions of equality within the same crate.
3. **Inline Does Not Gain `DateTime` vs Frontmatter `DateTime`:**
   - *Tension:* Ticket 18 ruled: *"Inline does not gain DateTime"* [V: `issues/18:503`]. Frontmatter continues to classify ISO timestamps into `NoteFieldValue::DateTime` [V: `18:471`].
   - *Impact:* `due: 2026-07-29T14:30:00` in frontmatter becomes a `DateTime`, while `due:: 2026-07-29T14:30:00` in the note body becomes a `String` (or truncated `Date`). This directly violates Spec Story 15: *"where I write a date never changes what it means"* [V: `.scratch/temporal-core/spec.md:43`].
4. **Deterministic `Ord` vs Persisted Zone-Shifted Naive Datetimes:**
   - *Tension:* Ticket 19 requires: *"how is determinism preserved (no ambient timezone in Ord)?"* [V: `issues/19:11-12`]. But Ticket 18 recorded: *"Naive frontmatter datetimes are zone-shifted at parse time and persisted in the postcard cache"* [V: `issues/18:546-547`]. Divergence Register Entry 17 documents that `From<DateValue> for DateTimeValue` falls back to UTC midnight because `Ordering::cmp` cannot fail [V: `.scratch/temporal-core/divergence-register.md:385-391`].
   - *Impact:* Storing zone-shifted UTC values in redb means cache content depends on the OS timezone active when `traces index` ran. Comparing `Date` to `DateTime` requires attaching a midnight instant, which cannot be localized without injecting timezone context into `Ord`.
5. **Cross-Kind Rank Inconsistency:**
   - *Tension:* Production query sort implements: `Null < Bool < Number < Duration < Temporal < Text < Link < List < Object` [V: `src/query/sort.rs:425-439`].
   - *Divergence:* Temporal Divergence Register Entry 4 records: `Null < Bool < Number < String < Date < DateTime < Duration < Link < List < Object` [V: `.scratch/temporal-core/divergence-register.md:296`].
   - *Impact:* The codebase contains contradictory specifications for whether `Duration` ranks before `Date`/`DateTime` and whether `Text` ranks before or after `Temporal`.

---

## 7. Domain Vocabulary & Glossary Map Audit

### Canonical Terms to Preserve

- **Note Field Value:** [V: `src/note/GLOSSARY.md:30-35`] The typed metadata value parsed from frontmatter or inline fields. *Avoid:* dynamic value, field payload.
- **Comparison Key:** [V: `src/GLOSSARY.md:107-116`] The derived form of a wikilink path, file stem, or request path for equality comparisons: NFC, then `char::to_lowercase`, then trailing NFC. Byte-exact attempted first. Never applied to sort order or display output. *Avoid:* normalized path, canonical path, fold key.
- **Duration:** [V: `src/GLOSSARY.md:41-52; src/temporal/GLOSSARY.md:28-36`] An elapsed duration carrying nominal seconds magnitude. Compares and hashes canonically by total seconds (`1h 30m == 90m`). *Avoid:* interval, period.
- **Date:** [V: `src/temporal/GLOSSARY.md:11-18`] Zone-free civil calendar date (`DateValue`). Requires four-digit year; `YYYY-MM` normalizes to the first of the month.
- **Date-Time:** [V: `src/temporal/GLOSSARY.md:19-27`] Timeline point stored as UTC and rendered using process-local clock. *Avoid:* timestamp, instant.
- **Filter Expression:** [V: `src/query/GLOSSARY.md:16-21`] Boolean predicate evaluated against note fields. *Avoid:* where clause, filter string.
- **Sort Order:** [V: `src/query/GLOSSARY.md:43-49`] Ordered sequence of field paths and directions.

### Terms Ticket 19 Must Formalize

- **Classification:** Syntactic sniffing of a string's shape (Date, Duration, Link) at parsing boundary without schema context.
- **Coercion:** Runtime conversion of a value from one dynamic variant to another during expression evaluation or comparison (e.g. string to date, integer to float).
- **Promotion:** Automatic widening within a compatible type domain (`Int` to `Float`, `Date` to `DateTime`).
- **Cast vs Try-Cast:** Explicit conversions; `Cast` fails loudly on invalid input, while `Try-Cast` evaluates to `Null` (following DuckDB).
- **Lattice:** The directed graph of allowed implicit promotions.
- **Collation:** Specific rules governing string comparison (Unicode normalization, case sensitivity).

### Glossary Discrepancies & Drift

- **Ambient Leak in Glossary:** [V: `src/temporal/GLOSSARY.md:24-25`] *"Comparisons with Date promote the Date to local midnight."* The glossary currently enshrines the ambient-zone leak as a core definition, directly conflicting with Ticket 19's mission to eliminate ambient state from `Ord`.
- **Duration Source Components Drift:** [V: `src/GLOSSARY.md:43-44`] Claims duration carries *"written source components"*, but Temporal Refinement 2 [V: `.scratch/temporal-core/divergence-register.md:245-253`] and Ticket 18 removed source string retention in favor of canonical `Display`.
- **Incomplete Value List:** [V: `src/note/GLOSSARY.md:32-34`] Defines `NoteFieldValue` as *"scalar, ISO date, link, list, or nested object"*, omitting `Duration`, `DateTime`, `Int`, and `Float`.

---

## 8. Challenges to Earlier Decisions

1. **Challenge to Ticket 18: Frontmatter Classification Quote-Insensitivity [P]**
   - *Evidence:* YAML syntax explicitly provides quotes (`"..."`, `'...'`) to signify string literals. Quoting a date (`title: "2025-01-15"`) is the universal YAML convention for preventing date parsing. Forcing quote-insensitivity removes any mechanism for note authors to write literal date-shaped strings in frontmatter without workarounds.
   - *Finding:* Ticket 18's argument that Noyalib CST is too complex was based on full CST round-tripping. But Noyalib's `ScalarStyle` / `Value` tracks whether a scalar was quoted. Frontmatter parsing SHOULD treat quoted YAML scalars as literal `NoteFieldValue::String`.
2. **Challenge to Ticket 18: Inline Field Exclusion of `DateTime` [P]**
   - *Evidence:* Disallowing `DateTime` in inline fields causes `time:: 2026-07-29T14:30:00` to be treated as a `String` while `time: 2026-07-29T14:30:00` in frontmatter becomes a `DateTime`. This is an arbitrary syntactic trap that confuses users and breaks Spec Story 15.
   - *Finding:* The date parser set is already unified [V: `18:500-501`]. If a string matches the ISO date-time grammar, it should parse to `DateTime` uniformly, regardless of whether it appears in frontmatter or an inline field.
3. **Challenge to Temporal Core: Parse-Time Local-to-UTC Zone Shifting [P]**
   - *Evidence:* Shifting naive datetimes to UTC at parse time and storing that in redb bakes the host machine's timezone into the persistent cache. If a repository is synced across machines in different time zones, timestamps drift by offset differences, and cache invalidation rules become fragile.
   - *Finding:* Naive datetimes should be stored as zone-free civil date-times (Jiff `civil::DateTime`). Resolution to UTC should occur strictly when comparing against UTC file timestamps or when explicitly requested by query evaluation.
4. **Challenge to Ticket 18: Derived Structural `PartialEq` on `NoteFieldValue` [P]**
   - *Evidence:* Making `Int(5) == Float(5.0)` evaluate to `false` under Rust `PartialEq` while query expressions evaluate `5 == 5.0` to `true` violates the principle of least astonishment. Rust standard types (`i64`, `f64`) do not equate directly, but unified dynamic value enums across mature engines (SQLite `Mem`, DuckDB `Value`) equate numbers by value.
   - *Finding:* `NoteFieldValue` should implement a custom `PartialEq` that equates `Int` and `Float` losslessly when mathematically equal.

---

## 9. Implications for Ticket 19 (Q1–Q11 Design Choices)

- **Q1: String-to-Date/Duration Promotion Seam:**
  - *Keep as is:* Parse-time heuristic sniffing via `FieldStringValue::classify` during frontmatter/inline ingestion.
  - *Design Options:* (1) Eager parse-time classification (Ticket 18 baseline); (2) Lexical storage in Note AST, with typing on demand in query engine; (3) Explicit query casts (`date(field)`) with optional schema-driven typing.
  - *Constraint:* Changing promotion from parse-time to query-time is **Migratable** under Ticket 16 [V: `16:43`].
- **Q2: Frontmatter Literal-String Escape:**
  - *Keep as is:* Quote-insensitive; no escape mechanism in frontmatter.
  - *Design Options:* (1) Respect YAML quotes (quoted = `String`, plain = classify); (2) Schema escape (schema declares field as `string`, disabling promotion); (3) Explicit prefix/escape syntax.
- **Q3: Int vs Float Equality & Ordering:**
  - *Keep as is:* Structural `PartialEq` (`Int != Float`); external query comparator coerces.
  - *Design Options:* (1) Implement SQLite's `sqlite3IntFloatCompare` losslessly in custom `PartialEq` and query comparator [V: `research/02:120-123`]; (2) NaN compares unequal to all numbers (`NaN != NaN`) and sorts highest/lowest; (3) Normalize `-0.0` to `0.0` at construction.
- **Q4: Date vs DateTime Equality/Order & Determinism:**
  - *Keep as is:* Ambient zone promotion in comparator with UTC fallback [V: `research/01:44-48`].
  - *Design Options:* (1) Strict partitioning in generic `Ord`: `Date` and `DateTime` never equate; `Date < DateTime` (or vice versa) unconditionally; (2) UTC midnight promotion in `Ord` (pure, zero-syscall, deterministic); (3) Query-contextual promotion: `Ord` is strictly partitioned, but query filter/sort receives `&ExecutionContext` to perform local midnight alignment.
- **Q5: Cross-Kind Rank Order & Equality Lattice:**
  - *Keep as is:* Total rank `Null < Bool < Number < Duration < Temporal < Text < Link < List < Object` [V: `src/query/sort.rs:425-439`].
  - *Design Options:* (1) Two distinct relations: a total order lattice for `sort`, and a strict 2-valued or 3-valued equality relation where cross-kind equality is always `false` (or `Null`), never coercing; (2) Reconcile the rank difference with Divergence Register Entry 4.
- **Q6: Link Equality and Ordering:**
  - *Keep as is:* Structural equality over `target`, `display`, and `embedded`.
  - *Design Options:* (1) Semantic equality compares `target` only via `ComparisonKey` (NFC + lowercase), treating `display` and `embedded` as presentation metadata; (2) Target compares via `ComparisonKey`, `embedded` forms distinct rank; (3) Textual exact equality.
- **Q7: Schema Select String vs Note Link Matching:**
  - *Keep as is:* Unresolved; mismatch fails.
  - *Design Options:* (1) Coerce schema string to `Link` via wikilink parser at schema boundary (Ticket 20), then compare `Link == Link`; (2) If comparing `Link` to `String`, match against `link.target` and/or `link.display`.
- **Q8: Query Literal Promotion & Sort-Time Sniffing:**
  - *Keep as is:* Query lexer promotes bare ISO dates (ADR-0009); `SortKey` runs `TextShape::classify` on strings at sort time.
  - *Design Options:* (1) Retain bare date literals in query grammar (ADR-0009 is confirmed), but eliminate `TextShape::classify` during sorting (strings sort strictly as strings); (2) Require explicit `date()` casts for string fields.
- **Q9: Duration Comparison with Calendar Units:**
  - *Keep as is:* Nominal seconds total order (`1mo == 30d == 720h`) [V: `src/GLOSSARY.md:46-48`].
  - *Design Options:* (1) Keep nominal equality for generic `Ord` and sorting, but reject cross-unit calendar comparison in strict filters without a reference date; (2) Dual regime: exact units compare losslessly, calendar units require date anchors.
- **Q10: String Collation & Unicode Normalization:**
  - *Keep as is:* Byte-exact Rust string comparison (`&str::cmp`); `ComparisonKey` only for wikilinks and paths.
  - *Design Options:* (1) Normalize all strings to Unicode NFC at ingestion; (2) Query filters provide case-insensitive equality operators (e.g. `~=` or `lower(x) == lower(y)`); (3) Case-folding collation in `sort`.
- **Q11: Strictness-by-Seam & Coercion Error Mode:**
  - *Keep as is:* Lenient note parsing; filter silent mismatch; template stringification.
  - *Design Options:* (1) DuckDB dual-cast model: `TRY_CAST` semantics (evaluates to `Null`, non-fatal for filters) for implicit query coercions, explicit `CAST` emitting query diagnostics on failure; (2) Incompatible filter comparisons evaluate strictly to `false`.

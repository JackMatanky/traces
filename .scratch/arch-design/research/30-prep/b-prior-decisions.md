# Prior accepted decisions bearing on the temporal boundary ticket

Ticket: [30-temporal-boundary-and-dialects](../../issues/30-temporal-boundary-and-dialects.md).
This file is research. It is not a decision. It does not amend any Answer.
Baseline: map revision `971bdd5729d9d5f51a1bd6fed46bab6947df936a`. Code reads
carry [V] for the working tree on 2026-11-16. Tags: [V] source inspection,
[H] inference, [P] proposal. The accepted answers outrank ticket summaries and
research files. Where a summary conflicts with an Answer, this file cites the
Answer.

## 1. Relevance screen of completed tickets

All completed tickets (01 to 29, 32) with verdicts. Open tickets 30, 31, 33,
34, 35, 36, 37, 38, 39 are out of scope. This file feeds ticket 30. Ticket 36
records supersessions.

| Ticket | Verdict for this ticket | Reason |
| --- | --- | --- |
| 01 verify claims | Screened out, background | Method ticket. Ticket 18, 19 and 21 Answers supersede its temporal facts (claims 1, 2, 11). One note remains: `QuerySet` chaining does not clone rows. |
| 02 value-model precedent | Screened out, superseded | Its findings entered the [18 Answer](../../issues/18-value-model.md), line 456. Read the Answer, not the research. |
| 03 coercion precedent | Screened out, superseded | Its findings entered the [19 Answer](../../issues/19-coercion-and-comparison-policy.md), line 62. One rule survives verbatim: no comparison reads an ambient zone or clock. |
| 04 execution-context precedent | Screened out, superseded | Superseded by the [21 Answer](../../issues/21-ambient-environment-and-execution-context.md), line 70. Its "sample `now` once" lesson is accepted text there. |
| 05 freshness precedent | Screened out, superseded | Superseded by the [25 Answer](../../issues/25-index-freshness-and-invalidation.md), line 277. Read the Answer for the fingerprint. |
| 06 query-plan precedent | Screened out, superseded | Superseded by the [29 Answer](../../issues/29-query-core-and-plan.md), line 481. |
| 07 template-bridge precedent | Relevant, one claim challenged | [Moment is not a core concept](../07-template-bridge-precedent.md), lines 87-88, is [H]. The code moved the translator into `src/temporal/token.rs` at `6104d0ae` [V]. The owner must settle Q1/Q2 against this tension. |
| 08 errors-topology precedent | Screened out, out of scope | Error topology belongs to ticket 34. Ticket 30 only supplies the temporal error set. |
| 09 capability-path precedent | Screened out, superseded | Superseded by the [32 Answer](../../issues/32-path-and-capability-model.md), line 360. |
| 10 document-model precedent | Screened out, superseded | Superseded by the [23 Answer](../../issues/23-note-document-model.md), line 249. |
| 11 performance evidence | Relevant, two numbers live | The 80-byte unified enum measure [V](../../issues/11-research-performance-evidence.md), line 49, binds Q19. The survey's "temporal is 10k lines" figures are stale; [ticket 30 Current code](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code", replaces them [V]. |
| 12 IWE core and hosts | Relevant, negative evidence | IWE has no date type and hands YAML dates to its validator as ISO strings [V](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-fileclass-research), section "Inputs from the Fileclass research". No temporal boundary input beyond that. |
| 13 IWE query schema | Screened out | No date kind in IWE's query engine [V](../../issues/19-coercion-and-comparison-policy.md), line 349. Tests ticket 19's rules only. |
| 14 Fileclass research | Relevant, corrected | Owner data uses Moment formats `YYYY`, `DDDD`, `Q` [V](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code". Fileclass bypasses pattern validation for a string `dateFormat`. The outer validator checks empty values and `required` [V](../14-fileclass-model-binding-and-validation.md), lines 115-119, narrowed by [K8](k-citation-verification.md). Its duration parser excludes years and months but accepts mixed weeks. It captures a sign without applying it. Do not use it as an RFC 5545 oracle [V, K8]. |
| 15 schematter research | Screened out | Body-section scope. No temporal content. |
| 16 compatibility envelope | Binding | Defines Frozen/Migratable/Free for every surface this ticket touches. See ledger B1 to B8. |
| 17 forward consumers | Binding, scope only | Sets seven measurement points, from 50 to 50,000 notes [V](../../issues/17-grilling-forward-consumers-and-design-pressure.md), lines 51-55. Q9 to Q12 need evidence at those points. Later candidate binary sizes are probe snapshots, not repository measurements (`m-gap-closure.md`). |
| 18 value model | Binding | `NoteFieldValue` variants, the 80-byte test, and the Link model. See B9 to B13. |
| 19 coercion and comparison | Binding | The three relations, adaptive literals, `date(link)`, duration provisional rule, notice set. See B14 to B24. |
| 20 schema typing | Binding, one fragment | Advisory check, `date.format` gap left explicit for this ticket, and one truncated sentence. See B25 to B27 and I1. |
| 21 execution context | Binding | The largest temporal input: frames, civil periods, filename prefixes, removals. See B28 to B41. |
| 22 config ownership | Binding | `[date]` ownership, zone precedence, fingerprint exclusion. See B42 to B46. |
| 23 note document model | Binding | Parser classification, task dates, effective accessor. See B47 to B50. |
| 24 index read model | Binding, as amended | Store keeps temporal values as text; no `day` persisted; range-key proof duty. See B51 to B56. |
| 25 freshness | Binding, as amended | Extractor fingerprint crate list, civil-variant epoch duty, period-policy condition. See B57 to B62 and D7. |
| 26 journal measurement | Screened out, one handoff | No temporal content. Its read-only-open control passed to ticket 25 [V](../../issues/26-measure-index-recovery-journal.md), lines 74-75. |
| 27 replay prototype | Screened out, one amendment | No temporal content. Platform-family supersession noted under ticket 25 (B59). |
| 28 live buffer | Binding, narrow | Buffer rows keep `file.size` as buffer bytes; the pinned generation and frame interact with Q33. See B63, B64. |
| 29 query core | Binding | Group classes, `member_of` reservation, pushdown protocol, keywords at plan time, notices. See B65 to B73 and the Main-confirmed C3. |
| 32 path model | Binding | `RelativePath`, last-segment reads, fold map. See B74 to B78. |

## 2. Accepted-constraint ledger

Classification:

- **B** binding: accepted Answer text, as amended.
- **P** proposal: explicitly left open, or a lean recorded as [P].
- **S** superseded in part: the older sentence no longer binds.
- **I** incomplete: a fragment that no one may read as a rule.

### 2.1 Compatibility envelope (ticket 16)

- **B1. Vault-format parsing is Frozen.** Frontmatter, `Key:: Value` inline
  fields, wikilinks, tasks and tags parse to the same meaning
  ([16](../../issues/16-grilling-compatibility-envelope.md), line 76). This binds
  the shared note parser. Ticket 21 scopes the new filename-period parser
  to filenames only ([21](../../issues/21-ambient-environment-and-execution-context.md),
  lines 175-176). Check the shared parser separately for ticket 30
  ([30](../../issues/30-temporal-boundary-and-dialects.md#checks-before-resolution),
  section "Checks before resolution").
  Preserve explicitly accepted changes, including civil naive datetimes and
  inline `DateTime`, with their migration rows ([19](../../issues/19-coercion-and-comparison-policy.md),
  lines 110-118, 232-246; [L3](l-answer-verification.md)).
  Today the note parser calls `DatePoint::parse_prefix` for task dates and
  inline fields [V](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code".
- **B2. Template and query capabilities are Frozen.** Date arithmetic stays
  reachable ([16](../../issues/16-grilling-compatibility-envelope.md), line 89).
- **B3. Template function API and query DSL spelling are Migratable.** This
  includes heuristic string-to-date promotion
  ([16](../../issues/16-grilling-compatibility-envelope.md), line 90). A
  dialect change needs a change-table row.
- **B4. `config.toml` and schema TOML formats are Migratable.** A `[date]`
  table and a `date.format` check need a change-table row
  ([16](../../issues/16-grilling-compatibility-envelope.md), line 86).
- **B5. `index.redb` layout and refresh strategy are Free.** The store had
  already been wiped once ([16](../../issues/16-grilling-compatibility-envelope.md), line 80).
- **B6. Exit codes and stdout/stderr split are Frozen**
  ([16](../../issues/16-grilling-compatibility-envelope.md), line 81). A
  higher-epoch read miss exits 0 with one stderr notice, per ticket 25.
- **B7. Dataview/Templater semantic parity is Free.** Those tools supply
  historical influence only ([16](../../issues/16-grilling-compatibility-envelope.md), line 78).
  Moment parity is also Free. Ticket 30 decides parity on design grounds,
  not compatibility grounds.
- **B8. Reduce visibility by default. Justify each `pub`**
  ([16](../../issues/16-grilling-compatibility-envelope.md), lines 94-100).
  The divergence register is not an input
  ([16](../../issues/16-grilling-compatibility-envelope.md), lines 60-62).
  Use `.scratch/temporal-core/divergence-register.md` as a pointer only
  ([30](../../issues/30-temporal-boundary-and-dialects.md#query-pushdown-and-notices), section "Query pushdown and notices").

### 2.2 Value model (ticket 18)

- **B9. `NoteFieldValue` is the single runtime value.** Variants: `Null`,
  `Bool`, `Int(i64)`, `Float(f64)`, `String`, `Date`, `DateTime`, `Duration`,
  `Link(Link)`, `List(Box<[Self]>)`, `Object(IndexMap<String, Self>)`
  ([18](../../issues/18-value-model.md), lines 482-489). Ticket 19 revoked the
  exclusion of inline `DateTime` ([18](../../issues/18-value-model.md), line 518,
  [19](../../issues/19-coercion-and-comparison-policy.md), lines 117-118).
- **B10. The 80-byte size test stays.** Do not box more variants
  ([18](../../issues/18-value-model.md), lines 484-485). Sizes: `DurationValue`
  72 B, `DateTimeValue` 16 B, `DateValue` 4 B [V in research 11]
  ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-value-model-decision), section "Inputs from the value model decision"). A
  civil variant must keep the total at 80 bytes
  ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-value-model-decision), section "Inputs from the value model decision").
  Ticket 18 lists no period variant
  ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-value-model-decision), section "Inputs from the value model decision").
  A period variant must meet the same size test. Q20 decides its placement.
- **B11. Schema literals stay a separate type** without `Date`, `DateTime`,
  `Duration` or `Link` ([18](../../issues/18-value-model.md), lines 491-493).
- **B12. Root `Link` carries `target`, normalized `display`, `embedded`, and,
  after ticket 23, the syntax kind**
  ([18](../../issues/18-value-model.md), lines 499-505,
  [23](../../issues/23-note-document-model.md), lines 436-437). Link equality
  uses the folded target plus subpath; `display` and `embedded` do not decide
  equality ([19](../../issues/19-coercion-and-comparison-policy.md), lines 147-149,
  [19](../../issues/19-coercion-and-comparison-policy.md), lines 172-174).
- **B13. Temporal values persist through derived tagged serde. A model
  change invalidates the cache**
  ([18](../../issues/18-value-model.md), lines 529-531).
  Ticket 24 keeps them as text until compact encoding shows a measured
  gain (B52).

### 2.3 Coercion and comparison (ticket 19, as refined)

- **B14. One value-semantics module owns three separate relations**: loose
  equality, bracketed order, total sort order
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 64-102).
  Across brackets `==` is false; relational operators do not match
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 92-95).
- **B15. Temporal brackets hold `Date`, `DateTime` and `CalendarPeriod`**
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 90-92,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-coercion-and-comparison-decision), section "Inputs from the coercion and comparison decision").
- **B16. Naive datetimes are civil, zone-free values. Only input with `Z` or
  an offset is an instant**
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 110-113). An
  ordinary `Date` field compares with a civil `DateTime` field at civil
  midnight ([19](../../issues/19-coercion-and-comparison-policy.md), lines 113-114).
- **B17. Civil-versus-instant comparison uses the pinned zone from the
  execution frame. No ambient zone lookup in ordering**
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 115-117).
- **B18. Period membership is a filter relation, not `PartialEq` or `Ord`.**
  Point-in-period `==` matches when the point's civil date is inside the
  period. Relational operators use period bounds with an exclusive upper
  bound for instants
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 150-160).
  Ticket 21 refines this. Period equality is exact `(start, unit)` identity.
  Relational order uses that key with an explicit unit tie-break. Mixed
  temporal sorting has a distinct total key
  ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 149-156,
  [19](../../issues/19-coercion-and-comparison-policy.md), lines 283-297).
- **B19. A quoted date-shaped literal parses once at build time into
  candidates**: `DatePoint` and its validated `CalendarPeriod`, `DateTime`,
  `Duration`, `Link` target, `Number`, `Bool`, and later a natural-language
  date ([19](../../issues/19-coercion-and-comparison-policy.md), lines 133-137).
  Do not parse or allocate per row
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 145-146).
- **B20. A period literal matches a whole span; the upper bound is
  exclusive** ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-coercion-and-comparison-decision), section "Inputs from the coercion and comparison decision").
  This supersedes ticket 19's inclusive `[s, e]` wording
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 283-297).
- **B21. Advisory notices: a build-time advisory covers an invalid
  date-shaped quoted literal (for example `"2025-02-30"` runs as a string).
  A bare invalid date literal stays a syntax error**
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 207-211).
  Ticket 34 owns the notice mechanism
  ([19](../../issues/19-coercion-and-comparison-policy.md), line 212).
- **B22. `date(link)` reads only the target's last path segment.** Remove
  folders, `#anchor` and `.md`. Never read display text. Accept an optional
  pattern argument for non-ISO titles. Return `Null` on parse failure
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 172-180,
  [19](../../issues/19-coercion-and-comparison-policy.md), lines 307-310).

  Today `date()` rejects `Link` [V]
  ([19-prep/g:20](../19-prep/g-links-holding-dates.md), line 20).
  Two Markdown-link questions remain:

  - Do `[[b]]` and `[b](b.md)` fold to one `ComparisonKey`?
  - How does `date(link)` read a percent-encoded Markdown target
    ([19](../../issues/19-coercion-and-comparison-policy.md), lines 299-313,
    [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-coercion-and-comparison-decision), section "Inputs from the coercion and comparison decision")?

  The later Markdown-link handoff leaves extension normalization and
  percent decoding open ([19](../../issues/19-coercion-and-comparison-policy.md),
  lines 299-313; [L](l-answer-verification.md), remaining human choices).
  Do not extend the earlier `.md` rule to every Markdown target.
- **B23. Duration equality and order keep nominal seconds provisionally**
  (`1mo == 30d`, `1y == 365d`). Calendar arithmetic stays Jiff-based.
  Ticket 30 owns refinement
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 182-184).
  Q23 must refine or reaffirm the rule with a counterexample.
  PostgreSQL keeps months separate. DuckDB and Dataview use nominal
  conversions [V](../19-prep/e-query-engine-conventions.md), lines 84-89.
- **B24. Classification happens once at parse time. Only query string
  literals get late typing. No comparison reads an ambient clock or zone**
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 64-68).

### 2.4 Schema typing (ticket 20)

- **B25. Schemas check note data on demand, advisory, pure and span-free.
  Schemas never enter the index or its fingerprint**
  ([20](../../issues/20-schema-note-typing-and-validation.md), lines 53-70).
- **B26. `date.format` is a write-format hint. The check accepts
  Date/DateTime and any scalar when the format has no date component. This
  gap stays explicit until ticket 30 fixes the dialect. After choosing a
  dialect, add the date-format check**
  ([20](../../issues/20-schema-note-typing-and-validation.md), lines 120-123,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-schema-typing-decision), section "Inputs from the schema typing decision").
  Q4 decides how a format with no full date (`YYYY`, `Q`, `DDDD`) is
  treated. The owner's data uses exactly those formats
  ([30](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code").
- **B27. The cli, lsp and mcp modules are thin hosts of `core`. Domain
  modules expose data and pure functions. Do not add one-implementor traits**
  ([20](../../issues/20-schema-note-typing-and-validation.md), lines 72-77).
  This binds Q28's policy handoff shape.
- **I1. Truncated fragment.** Ticket 20's Answer ends mid-sentence in its
  `Date-shaped text` discussion: "Only
  date, datetime and" ([20](../../issues/20-schema-note-typing-and-validation.md),
  lines 305-315).
  Do not read a rule into that fragment
  ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-schema-typing-decision), section "Inputs from the schema typing decision").
  Keep the fragment as a separate ticket 36 recovery item. The fragment
  does not block the delegated date-format check. B26's complete
  provisional scalar-acceptance rule stands on its own.

### 2.5 Execution context (ticket 21)

- **B28. One frame per query, render or request pins the clock and the week
  rule. The frame acquires the zone only when needed. The frame retains a
  zone-acquisition success or failure**
  ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 106-110,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-execution-context-decision), section "Inputs from the execution-context decision").
- **B29. A host may select UTC explicitly. UTC is never an implicit
  fallback**
  ([21](../../issues/21-ambient-environment-and-execution-context.md), line 127,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-execution-context-decision), section "Inputs from the execution-context decision").
- **B30. A missing required zone, calendar resolution or presentation is a
  typed resolution, materialization or render failure. Never drop the row,
  return `Null`, or blank the cell**
  ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 123-127,
  [19](../../issues/19-coercion-and-comparison-policy.md), lines 203-206).
- **B31. `CalendarPeriod` becomes a validated value: a civil start date plus
  a unit. The boundary-selector role renames to `CalendarUnit` with `Day`,
  `Week`, `Month`, `Quarter`, `Year`. No second period type, no heap
  allocation for this value**
  ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 136-141).
  Today `CalendarPeriod` has `Week`, `Month`, `Year` only, and `Week` is the
  ISO week [V](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code".
- **B32. Period-versus-instant filters use the instant's civil date in the
  pinned zone. A skipped local day matches no instants. A skipped next day
  does not invalidate a real preceding day**
  ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 142-145).
  Civil points use their civil dates without a zone
  ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-execution-context-decision), section "Inputs from the execution-context decision").
- **B33. Do not use Rust `Eq`/`Ord` for membership. Use a separate
  deterministic total sort key for mixed temporal values. The period unit
  tie-break stays open for this ticket**
  ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 148-156,
  [21](../../issues/21-ambient-environment-and-execution-context.md), lines 269-270).
- **B34. Filename-prefix rules**
  ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 170-174):
  - Keep day/month recognition.
  - Add validated ISO week forms with Monday start and four-day week one.
  - Add quarter forms.
  - Enable a bare year prefix only through `DateConfig`.
  - Prefer the longest valid shape.
  - Never reinterpret a malformed longer prefix as a shorter one.
- **B35. `file.date` holds a filename-derived `CalendarPeriod`. Return
  `Null` when no prefix matches. Never fall back to creation time.
  Retire `file.day` without a new meaning**
  ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 157-161).
- **B36. `file.cdate`/`file.ctime` use only creation time. Return `Null` when
  creation time is unavailable. Never substitute mtime. `file.mdate` uses
  mtime. Timestamp fields stay instants**
  ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 162-169).
  During evaluation, `cdate`/`mdate` project to civil dates in the pinned
  zone. An unavailable required zone causes a typed failure. Never persist
  those zone-derived dates.
- **B37. The new prefix forms apply to filenames only. Preserve frozen
  shared parsing except for explicitly accepted changes and migration rows**
  ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 175-176;
  [L3](l-answer-verification.md)).
- **B38. Ambient removals: `local_zone()` leaf reads, per-row zone lookups,
  and the test-only `ZoneGuard` leave after injection lands**
  ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 196-200,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-execution-context-decision), section "Inputs from the execution-context decision").
  The evaluator must not read the clock or ambient zone
  ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 115-119).
  Current code still has `local_zone()` in `src/temporal/zone.rs` and
  test-only `ZoneGuard` in `src/temporal/mod.rs`. `local_zone()` is at
  `zone.rs` line 13; `ZoneGuard` is at `mod.rs` line 109 [V].
- **B39. Temporal owns validated periods, week rules and explicit-zone civil
  conversion. Query owns pinned frames. Template owns the render frame**
  ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 242-245,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-execution-context-decision), section "Inputs from the execution-context decision").
- **B40. Left to later tickets by ticket 21**: the period sort unit
  tie-break, compact encoding, week formats beyond ISO, and the configured
  zone source
  ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 269-274).
- **B41. A lazy `QuerySet` keeps its creator's resolved plan and frame.
  Terminal access must not hide execution errors**
  ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 128-132,
  [29](../../issues/29-query-core-and-plan.md), lines 979-981).

### 2.6 Config ownership (ticket 22)

- **B42. `config` owns and parses the `[date]` table into `DateConfig`.
  Ticket 30 decides the syntax and validation**
  ([22](../../issues/22-config-ownership-and-lifecycle.md), lines 227-228).
  Zone precedence: host override, then config, then system
  ([22](../../issues/22-config-ownership-and-lifecycle.md), lines 229-230).
- **B43. The time zone never enters the index fingerprint**
  ([22](../../issues/22-config-ownership-and-lifecycle.md), line 114).
- **B44. Ticket 22 removed `title`, `aliases` and the date keys from the
  fingerprint because no production code reads them. An index-time read must
  add its key to `IndexPolicy`**
  ([22](../../issues/22-config-ownership-and-lifecycle.md), lines 109-111).
  `IndexPolicy` holds the filename-period policy only if the index stores
  parsed periods
  ([22](../../issues/22-config-ownership-and-lifecycle.md), lines 100-101).
- **B45. Keep `deny_unknown_fields`. Add no `version` key before the first
  breaking change. The `[date]` table needs an exact field set**
  ([22](../../issues/22-config-ownership-and-lifecycle.md), lines 225-226,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-config-decision), section "Inputs from the config decision").
- **B46. Config merge applies global first, then local, field by field.
  This inherited rule also applies to `[date]`**
  ([22](../../issues/22-config-ownership-and-lifecycle.md), lines 223-235;
  [L8](l-answer-verification.md)). A different merge needs an explicit amendment.
  Config values from a local file apply only after the trust gate passes
  ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-config-decision), section "Inputs from the config decision").
- **P1. Edge direction.** `config` reads `DEFAULT_DATETIME_PATTERN` from
  temporal today [V](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-config-decision), section "Inputs from the config decision";
  temporal imports `DelimiterType` from the crate root in
  `src/temporal/token.rs` [V].
  The ticket records a [P] lean: keep the edge from `config` to temporal
  ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-config-decision), section "Inputs from the config decision").
  This is a proposal, not an accepted rule.
- **Current config surface [V]**: no `DateConfig` type exists. Date config
  is `DateFieldConfig { name, format }` under `[frontmatter.date_created]`
  and `[frontmatter.date_modified]`, default format `%Y-%m-%dT%H:%M:%S`
  from `DEFAULT_DATETIME_PATTERN` [V]
  (`src/temporal/datetime.rs`, `DEFAULT_DATETIME_PATTERN`;
  `src/config/model/frontmatter.rs`, date-field defaults).
  The `%Y-%m-%d %H:%M` value in
  [22-prep/a2](../22-prep/a2-inventory-digests-lifecycle.md), lines 253-262,
  is stale and not the current default.
  Zone, week rules and bare-year parsing have no config presence in
  the recorded snapshot [V]
  ([22-prep/a2](../22-prep/a2-inventory-digests-lifecycle.md), line 262).

### 2.7 Note document model (ticket 23)

- **B47. The parser classifies each value once at parse time; frontmatter
  and inline fields share one parser set and one precedence order**
  ([23](../../issues/23-note-document-model.md), lines 334-336,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-note-document-decision), section "Inputs from the note document decision").
- **B48. Inline fields can yield `DateTime`; a naive one is civil**
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 110-118;
  element-wise classification at
  [23](../../issues/23-note-document-model.md), lines 334-335; read through
  the ticket 30 inputs,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-coercion-and-comparison-decision), section "Inputs from the coercion and comparison decision").
  Ticket 23 line 477 concerns the task payload, not inline datetime
  classification.
- **B49. Task dates stay civil `DateValue` values; `TaskDateSet` holds six
  of them** ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-note-document-decision), section "Inputs from the note document decision").
  [V] `TaskDateSet` in `src/task.rs` has six `Option<DateValue>` slots
  and first-wins insert (`TaskDateSet`, fields and `insert`).
  This is current code, not a new rule in the ticket 23 Answer.
- **B50. A fact-changing parser setting enters `IndexPolicy`; a change of
  the stored `Note` changes the extractor fingerprint or bumps the format
  epoch** ([23](../../issues/23-note-document-model.md), lines 368-371).

### 2.8 Index read model (ticket 24, as amended by 28/29/32)

- **B51. The store persists source timestamps and size; it does not persist
  `day`. A query derives the filename period; this ticket owns that parser**
  ([24](../../issues/24-index-read-model-and-query-execution.md), lines 402-405,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-read-model-decision), section "Inputs from the read model decision").
- **B52. The store keeps temporal values as text until a compact encoding
  shows a measured gain; postcard re-parses them on each load**
  ([24](../../issues/24-index-read-model-and-query-execution.md), lines 400-401,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-read-model-decision), section "Inputs from the read model decision").
- **B53. A range key needs a proof for each operator, row kind, temporal
  frame and field precedence. Unsupported mappings scan**
  ([24](../../issues/24-index-read-model-and-query-execution.md), lines 312-327).
- **B54. A civil datetime must deserialize from the cache without a zone
  lookup; the store persists naive datetimes as civil values and holds no
  zone**
  ([24](../../issues/24-index-read-model-and-query-execution.md), lines
  503-510, and [25](../../issues/25-index-freshness-and-invalidation.md),
  lines 438-439).
  The earlier link labelled `24:455` pointed at ticket 30 and was
  mislabeled.
- **B55. Research 25 says a persisted key for `file.mdate` keeps UTC
  instants, not civil dates**
  ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-read-model-decision), section "Inputs from the read model decision").
  This is research input, not an accepted physical encoding. Ticket 21
  fixes source instants with evaluation-time civil projection (B36), and
  ticket 24 keeps compact temporal encoding conditional on measurement
  (B52).
- **B56. Ticket 29 starts range keys for unquoted numeric, temporal and
  bool literals. It adds no text range table at first. Bracket counts live
  in the `meta` facet. Remove old derived facts before adding replacements**
  ([24](../../issues/24-index-read-model-and-query-execution.md), lines 761-765).

### 2.9 Freshness (ticket 25, as amended by 28/29/32)

- **B57. The extractor fingerprint hashes the resolved versions of the
  crates that feed persisted values: `pulldown-cmark`, `logos`, `noyalib`,
  `jiff`, `speedate` and `postcard`, plus encoded probe records. Whoever
  prunes a crate updates the list**
  ([25](../../issues/25-index-freshness-and-invalidation.md), lines 304-311,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-freshness-decision), section "Inputs from the freshness decision").
- **D7. Recorded omission.** Ticket 25's crate list omits `winnow`. Ticket
  30 records the effect. `winnow` locates duration prefixes for inline fields.
  Those values reach the store. Keep `winnow` in the list, or remove it from
  the code ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-freshness-decision), section "Inputs from the freshness decision").
  [V] `winnow` 1.0.4 has one call site, `src/temporal/lexer.rs:7,118`.
  This appears in the ticket 36 supersession list
  ([30](../../issues/30-temporal-boundary-and-dialects.md#checks-before-resolution), section "Checks before resolution").
- **B58. The corpus uses explicit-offset datetimes only until the civil
  variant lands; adding it bumps the format epoch**
  ([25](../../issues/25-index-freshness-and-invalidation.md), lines 332-334,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-freshness-decision), section "Inputs from the freshness decision").
- **B59. Store identity: engine tag, format epoch, extractor fingerprint,
  policy fingerprint, written-by version, canonical root; platform family
  removed by ticket 32. Store-open rules and rebuild/miss policy are
  ticket 25's, as amended**
  ([25](../../issues/25-index-freshness-and-invalidation.md), lines 285-302,
  [32](../../issues/32-path-and-capability-model.md), lines 1150-1159).
- **B60. Ticket 25 does not fingerprint filename-period rules; the store
  does not persist them.** A persisted facet that uses those rules must
  include them in `IndexPolicy` and the corpus. The sources are
  [ticket 30, Inputs from the freshness decision](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-freshness-decision)
  and [ticket 25](../../issues/25-index-freshness-and-invalidation.md),
  lines 180-184.
- **B61. A change of the parsing stack changes the fingerprint and forces
  one rebuild** ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-freshness-decision), section "Inputs from the freshness decision").
- **B62. Journal and replay batches accept validated disk facts only; journal
  fact paths use canonical UTF-8 `/` text**
  ([26](../../issues/26-measure-index-recovery-journal.md), lines 502-506,
  [27](../../issues/27-index-replay-prototype-and-fault-tests.md), lines 629-640).
  No temporal content otherwise.
- **B63. A host overlay keeps the base file's `mtime` and `created`
  metadata; a new buffered file has neither. Buffer rows keep `file.size`
  as buffer bytes**
  ([28](../../issues/28-live-buffer-and-persisted-index-consistency.md),
  lines 1070-1078).
  Do not infer period dates from the buffer clock or invent metadata for
  new buffers.
- **B64. Memo identity must include the frame and the generation. A changed
  plan, frame or generation needs a distinct memo identity**
  ([29](../../issues/29-query-core-and-plan.md), lines 965-978).

### 2.10 Query core (ticket 29)

- **B65/C3. Temporal group class order is accepted text: Civil, Instant,
  Period.** Ticket 29 section 3 sets it:
  "The classes stay apart and order Civil, Instant, Period"
  ([29](../../issues/29-query-core-and-plan.md), lines 721-730). Confirmed
  owner per Main, 2026-11-16. C3 owns group-class order. C4 owns period
  upper-bound wording. Ticket 30's “Nobody has defined that order”
  sentence is a historical discrepancy that was repaired. Its current
  open sort question is the complete mixed temporal value-sort key and
  period unit tie-break ([L6](l-answer-verification.md)).
- **B66. A `Date` and a civil-midnight `DateTime` share a group key when
  their civil values agree. Civil values and instants stay separate.
  A period groups by exact `(start, unit)`. Membership does not define
  group equality. Ticket 29 section 3 does not ban operator-specific
  membership bounds. R5 governs those bounds (B70)**
  ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-query-core-decision), section "Inputs from the query-core decision";
  [29](../../issues/29-query-core-and-plan.md), lines 716-730).
- **B67. Mixed civil/instant comparators are unsound across a DST gap.** The
  cycle is civil 02:30 < civil 03:00 < instant 07:15Z < civil 02:30 in
  `America/New_York` on 2026-03-08. `sort_by` panicked in 15 of 200 runs [V]
  ([29](../../issues/29-query-core-and-plan.md), lines 724-727,
  [29-prep/h3](../29-prep/h3-redteam-round2.md), lines 161-163).

  For grouping, project each temporal value to its canonical class key
  once. Do not compare raw civil and instant values. Civil group keys use
  `(date, time)`. Instant keys use epoch nanoseconds. Period keys use
  `(start, unit)`. These keys need no zone lookup.

  Ticket 21 forbids zone acquisition for civil-only work. Keep the pinned
  frame for operations that need it. The DST comparator failure is probe
  evidence from ticket 29 preparation, not a result of this task.
- **B68. `member_of` is reserved as an IR node name only; no frontend, no
  implementation. The spelling waits on ticket 30 (Q18). Its relation is
  never a range key and never a group key**
  ([29](../../issues/29-query-core-and-plan.md), lines 1186-1191).
- **B69. Relative keywords (`today`, `eom`) bind to the pinned frame at plan
  time. A standalone query acquires a new frame; a render or request
  supplies its shared frame**
  ([29](../../issues/29-query-core-and-plan.md), lines 976-978,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-query-core-decision), section "Inputs from the query-core decision").
  Q17 decides the filter spelling.
- **B70. Temporal probe bounds need operator-specific proofs for `Date`,
  civil `DateTime`, instant and `CalendarPeriod` values under the pinned
  frame. Point-period matching uses membership bounds; period-period
  comparison uses normalized `(start, unit)` identity and order. An unproved
  mapping is Unsupported and scans**
  ([29](../../issues/29-query-core-and-plan.md), lines 538-542). Q35 owns the
  per-operator, per-kind bound table.
- **B71. First pushdown scope: unquoted temporal literals and `And` only;
  quoted literals and `Not`/`Or` scan**
  ([29](../../issues/29-query-core-and-plan.md), lines 571-577,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-query-core-decision), section "Inputs from the query-core decision").
- **B72. Filename-derived `file.date` keys stay Unsupported until the parser
  contract is complete. `file.day` is retired**
  ([29](../../issues/29-query-core-and-plan.md), lines 596-597,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-query-core-decision), section "Inputs from the query-core decision").
- **B73. Notices: a bracket mismatch, an invalid date-shaped quoted literal
  and NaN-to-Null ride one stderr mechanism; ticket 34 owns the envelope.
  Ticket 29 did not adopt the first-round notice for rows with a `Null`
  date. Tickets 30 and 34 decide it (Q36)**
  ([29](../../issues/29-query-core-and-plan.md), lines 1124-1138,
  [29](../../issues/29-query-core-and-plan.md), lines 1198-1199,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-query-core-decision), section "Inputs from the query-core decision").
- **D8. Recorded discrepancy.** Ticket 29 uses `eom` as a relative keyword,
  although the current grammar defines `eom` as a function that takes a date
  argument [V](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code".
  Ticket 36 records it ([30](../../issues/30-temporal-boundary-and-dialects.md#checks-before-resolution), section "Checks before resolution").
  Q17 resolves the spelling.

### 2.11 Paths (ticket 32)

- **B74. `RelativePath` is a normalized, `/`-joined UTF-8 `String`. Its
  operations are pure and need no I/O. It rejects NUL and `..`.
  Its `Ord` maps `/` to NUL**
  ([32](../../issues/32-path-and-capability-model.md), lines 431-461).
- **B75. The filename-period parser reads the last segment of that string.
  Today `derive_day` reads a `Path` through `BaseNameRef::from_path`**
  ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-path-decision), section "Inputs from the path decision").
  [V] `FileMeta::derive_day` at `src/file.rs:87-97` uses a filename prefix
  first, then `created_at`. Ticket 21 removes that fallback (B35).
  Dataview's inspected helper instead checks typed metadata before the
  filename ([K15](k-citation-verification.md)).
- **B76. The scan skips non-UTF-8 names with a notice**
  ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-path-decision), section "Inputs from the path decision").
- **B77. The Comparison Key (NFC, lowercase, NFC) is a lookup aid only. It
  never applies to redb keys, sort order, I/O paths or overlay keys**
  ([32](../../issues/32-path-and-capability-model.md), lines 657-670).
  Fold-based link resolution bumps the extractor fingerprint epoch
  ([32](../../issues/32-path-and-capability-model.md), lines 671-675).
- **B78. Path sort/range key: the path with `/` replaced by `0x00`; folder
  range `[prefix\0, prefix\x01)`**
  ([32](../../issues/32-path-and-capability-model.md), lines 1403-1404, read
  through the ticket 29 handoffs; see also
  [24](../../issues/24-index-read-model-and-query-execution.md), lines 744-745).

### 2.12 Cross-cutting

- **B79. Performance evidence rules.** Use ticket 17's seven measurement
  points. Do not infer optimality from line count, dependency count, or
  unmeasured cost. Mark unmeasured binary size and runtime cost explicitly
  ([brief.md:75-76](./brief.md), lines 75-76,
  [17](../../issues/17-grilling-forward-consumers-and-design-pressure.md), lines 51-55).
- **B80. Pinned `jiff`.** `jiff = "=0.2.38"` with `default-features = false`,
  features `std`, `tz-system`, `tzdb-bundle-always` [V]
  ([30](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code",
  `Cargo.toml:128`).
  `DateTimeValue` is a transparent `Timestamp` wrapper with derived `Ord`
  [V] (`src/temporal/datetime.rs`, `DateTimeValue`).
  `speedate` 0.18.0 has one call site in `src/temporal/date.rs`.
  `winnow` 1.0.4 has one in `src/temporal/lexer.rs`.
  `logos` 0.16.1 serves `lexer.rs`, `datetime.rs`, `point.rs`, the shared
  lexer, the note parser and the query grammar [V]
  ([30](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code",
  `Cargo.toml:129,143,152`).
- **B81. Stale temporal glossary.** `src/temporal/GLOSSARY.md` still says
  unzoned input uses the process zone and a Date promotes to local midnight
  [V](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code". Ticket 19
  supersedes both sentences ([19](../../issues/19-coercion-and-comparison-policy.md), lines 110-118,
  [19](../../issues/19-coercion-and-comparison-policy.md), lines 320-325).
  Treat the glossary as stale code documentation, not as an accepted rule.
- **B82. ADR-0010 is `proposed`, not accepted.** It lexes a bare ISO date as
  one query literal [V](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code".
  It constrains nothing until accepted. Ticket 28 records that ticket 19
  fixes the ADR's quoted/unquoted divergence
  ([28](../../issues/28-live-buffer-and-persisted-index-consistency.md), line 1238).
- **B83. Divergence register exclusion.** Ticket 16 rules the register is
  not an input; use it as a pointer only (B8).

## 3. Proposal and open-item register

Items that look like rules but are not accepted decisions:

- **P2. Natural-language grammar hook.** Ticket 19 proposes a deterministic,
  closed English grammar. It resolves from injected `today`. Filter literals,
  `date()`, `date_add` and template filters share it. It must yield an
  interval precision for phrases like `next week`. The meaning of a bare
  weekday stays open ([30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-coercion-and-comparison-decision), section "Inputs from the coercion and comparison decision",
  [19](../../issues/19-coercion-and-comparison-policy.md), lines 167-170).
  Ticket 30 owns grammar and crate choice (Q13 to Q16).
- **P3. In-repo grammar default.** Ticket 30 records a [P] lean: run the prep
  first and treat an in-repository grammar as the default until evidence
  beats it ([30](../../issues/30-temporal-boundary-and-dialects.md#prep-before-grilling), section "Prep before grilling").
- **P1. Config-to-temporal edge direction** (see above).
- **P4. `.scratch/temporal-core/design-nl-dates.md` is an unreviewed agent
  draft [H].** It conflicts with accepted decisions in four places:
  - It returns month precision for `next month`. Ticket 19 needs an interval.
  - It adds a template-only `date.relative`. Ticket 19 needs one shared
    grammar.
  - It reads `now_instant()` and `ZoneGuard`. Ticket 21 replaces both.
  - It fixes weekday meanings that ticket 30 must still decide
    ([30](../../issues/30-temporal-boundary-and-dialects.md#prep-before-grilling), section "Prep before grilling").

  Treat it as a pointer, not evidence.
- **P5. Ticket 21 handoffs that await this ticket**: period/unit vocabulary
  outcome, ISO/custom weeks, skipped days, `DateConfig` shape and filename
  recognition ([21](../../issues/21-ambient-environment-and-execution-context.md), lines 291-292).
- **P6. Downstream needs (not blockers)**: ticket 31 needs the translator
  owner, calendar helpers, render civil/instant/period rendering, frame and
  zone-failure error type, unsupported-token error shape
  ([30](../../issues/30-temporal-boundary-and-dialects.md#downstream-consumers), section "Downstream consumers").
  Ticket 34 needs the temporal error set, typed zone failure, `Null`-date
  notice condition and any new schema date-format code
  ([30](../../issues/30-temporal-boundary-and-dialects.md#downstream-consumers), section "Downstream consumers").
  Ticket 35 prunes `speedate`/`winnow` after this ticket
  ([30](../../issues/30-temporal-boundary-and-dialects.md#downstream-consumers), section "Downstream consumers").
  Ticket 36 records the `file.day` to `file.date` migration and checks the
  frozen-parser separation
  ([30](../../issues/30-temporal-boundary-and-dialects.md#downstream-consumers), section "Downstream consumers").

## 4. Contradictions and exact human decisions needed

Each item names a conflict, a supersession, or an open choice.
Only the human can authorize an amendment to an earlier Answer.

- **C1. Moment translator home.** Research 07 concluded Moment is a
  Templater-compat feature, not a temporal concept, and proposed a template
  compat module [H](../07-template-bridge-precedent.md), lines 87-88,
  [30](../../issues/30-temporal-boundary-and-dialects.md#inputs-from-the-template-bridge-research), section "Inputs from the template bridge research").
  Commit `6104d0ae` moved the translator into `src/temporal/token.rs` [V]
  ([30](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code").
  Ticket 16 makes parity Free (B7), so the decision is design-only. Human
  decision: ratify the temporal home (Q1/Q2), return it to a template
  adapter, or drop Moment tokens. The owner needs three kinds of evidence:
  - The owner's tokens. `docs/vault/07_templates/12_01_week_periodic.md:84-91`
    uses `ww`, `YY`, `Q`. `14_01_quarter_periodic.md:84-89` uses `Q`, `Qo` [V].
  - The unsupported-token behavior. `MOMENT_TOKENS` has no `ww`, `Q`, `Qo`,
    `DDDD`, so those tokens fail [V by reading]
    ([30](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code").
  - The change-table cost (B3).
- **C2. `date.format` dialect and the no-full-date check.** Four format
  surfaces exist: config `format`, query `date(text, pattern)` (strftime
  only today [V](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code"),
  template `date_format`, and schema `date.format` (unvalidated
  `Option<String>` [V](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code").
  B26 leaves the check gap explicit. I1 forbids reading the truncated
  sentence. Human decision: one dialect or a justified split (Q3), the
  no-full-date rule (Q4), and the fate of `[frontmatter.date_*]` (Q5).
- **C3. Group class order owner.** Ticket 29 §3 lines 721 to 730 fixes
  Civil, Instant, Period. Ticket 30's "nobody has defined that order"
  sentence is stale. Main confirmed on 2026-11-16 these three actions. Cite
  ticket 29 as the accepted owner in the integration synthesis. Record the
  stale sentence in the supersession list. Q22 still needs the complete
  mixed-value sort key and the period unit tie-break (B65).
- **C4. Period upper-bound wording.** Ticket 19's original `[s, e]` wording
  conflicts with ticket 21's exclusive bound. Ticket 19 records the
  refinement as overriding
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 283-297).
  Carry the exclusive rule (B20). Do not reopen the accepted bound.
- **C5. Week-one rule under a custom week start.** Ticket 21 fixes Monday
  start and four-day week one for ISO filename weeks. General week settings
  do not reinterpret those prefixes ([21](../../issues/21-ambient-environment-and-execution-context.md),
  lines 170-179, 266-270; [L10](l-answer-verification.md)).
  A first-weekday setting alone does not define week-year numbering.
  Q8 decides custom numbering and its supported operations.
  ISO `2025-W53` is invalid. Accept no rollover
  [V/H](../21-prep/r5-temporal-adversarial.md), lines 31-34.
- **C6. Bare weekday meaning.** Next, nearest, or current-week day. Ticket
  19 leaves it open
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 170-171);
  the unreviewed draft fixes meanings it must not fix (P4). Human decision:
  Q15.
- **C7. Nominal duration equality.** B23 keeps nominal seconds
  provisionally. Q23 decides refine or keep, with a counterexample. Ticket
  36 must reconcile the glossary wording once settled
  ([19](../../issues/19-coercion-and-comparison-policy.md), lines 324-325).
- **C8. Civil/instant type shape.** A `DatePoint` variant or an inner enum;
  `DatePoint` today stores `Calendar { date, date_precision }` or
  `DateTime(DateTimeValue)` [V] (`src/temporal/point.rs`, `DatePoint`).
  The 80-byte test (B10) constrains it. Q19 decides; B58 ties the corpus
  change to the landing. Correction: `DateTimeValue` is a transparent
  `Timestamp` wrapper with derived `Ord` [V]
  (`src/temporal/datetime.rs`, `DateTimeValue`). Parse-time `LocalClock`
  civil values resolve through the process-local zone [V]
  (`src/temporal/datetime.rs`, `LocalClock`; `src/temporal/point.rs`,
  `DatePoint` conversion).
  Today's runtime keeps no civil datetime: the outer Calendar/DateTime
  split separates civil dates from instants, not civil datetimes from
  instants. Ratifying the current shape does not satisfy the accepted
  civil-datetime target (see [L5](i-leakage-review.md); also
  [owner-evidence.md](./owner-evidence.md), corrected).
- **C9. Period value placement.** Ticket 18 lists no period variant; tickets
  19 and 21 need period values for `file.date`
  ([30](../../issues/30-temporal-boundary-and-dialects.md#value-shapes), section "Value shapes").
  Ticket 36 must reconcile that list
  ([30](../../issues/30-temporal-boundary-and-dialects.md#checks-before-resolution), section "Checks before resolution").
  Q20 decides placement under B9, B10 and B31. A separate runtime carrier
  needs an explicit amendment ([L4](l-answer-verification.md)).
- **C10. Zone acquisition failure reporting.** B29/B30 fix the policy;
  ticket 22 fixes the precedence (B42). Q30 decides the host acquisition
  mechanism and report path with ticket 34. Jiff note [V]
  ([K20](k-citation-verification.md)): `TimeZone::try_system` errors when
  discovery fails or `tz-system` is disabled. The convenience
  `TimeZone::system` can fall back to `Etc/Unknown`, which behaves like UTC
  ([d, TimeZone::try_system](d-crate-evidence.md)).
  B29 forbids that implicit fallback. Keep the exact zone identity
  separate from its UTC-like behavior.
- **C11. `member_of` spelling and Null-date notices.** Ticket 29 reserved
  the IR name only and did not adopt the first-round notice (B68, B73).
  Human decisions: Q18 (spelling) and Q36 (notice condition, with ticket
  34).
- **C12. Probe bounds per operator and kind.** B70 requires proofs; an
  unproved mapping scans. Q35 decides the bound table. Nothing in tickets 24
  or 29 pre-decides the mappings.
- **C13. `winnow` fate interacts with the fingerprint.** Removing `winnow`
  (Q10) requires updating ticket 25's crate list (B57, D7). Keeping it
  requires the list correction anyway. Either way, ticket 36 records the
  correction.
- **C14. `speedate` fate.** One call site [V] (B80). Q9 decides. If pruned,
  update the crate list per B57.
- **C15. Filename parser location and persistence.** B34/B35/B37 fix the
  semantics; B51 fixes derivation at query time; B60 makes persistence a
  conditional fingerprint input. Q7 decides the module home; Q27/Q32 decide
  the `DateConfig` fields that gate bare years and week options.
- **C16. Calendar helpers in the template engine.** `Anchor`, `apply_offset`
  and `weekday` live in `src/template/engine/date.rs` (2,512 lines, about
  880 production [V]
  ([30](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code")).
  `PERIOD_EDGE_LOOKUP` and `resolve_period_edge` already live in
  `src/temporal/calendar.rs` and serve query and template [V]
  ([30](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code").
  Q6 decides which helpers stay. Template `date.weekday(n)` selects ISO
  weekday `n` of the current week today [V]
  ([30](../../issues/30-temporal-boundary-and-dialects.md#current-code), section "Current code").
- **C17. `[date]` table syntax, merge and validation.** B42/B45/B46 fix
  ownership and constraints. Q26 to Q29 decide syntax, fields, the temporal
  policy type, and zone syntax. Q31 decides week-start configurability; B40
  leaves week formats beyond ISO open.
- **C18. Execution-frame exports.** B28/B39/B41 fix ownership. Q33 decides
  what temporal exports to the frame; Q34 names the replacements for
  `local_zone()`, `now_instant()` and `ZoneGuard` reads (their removal is
  already decided, B38). Ticket 33 later wires the frame source and
  `DateConfig` into the host
  ([30](../../issues/30-temporal-boundary-and-dialects.md#query-pushdown-and-notices), section "Query pushdown and notices").
- **C19. Truncated ticket 20 sentence.** Recover I1 through ticket 36.
  Do not infer its missing words. Recovery does not block the complete
  B26 rule or the date-format check delegated to ticket 30 ([L1](l-answer-verification.md)).

## 5. Question-to-constraint map

| Q | Subject | Binding constraints | Open decision |
| --- | --- | --- | --- |
| 1 | Moment as domain concept | B7, B83 | C1 |
| 2 | Translator home | B7, B3, B80, B27 | C1 |
| 3 | Dialect per surface | B3, B4, B80 | C2 |
| 4 | `date.format` no-full-date check | B26 | C2; C19 is a separate recovery item |
| 5 | `[frontmatter.date_*]` fate | B4, B44, current surface [V] | C2 |
| 6 | Calendar helpers in engine | B39, B31 | C16 |
| 7 | Filename-period parser home | B34, B35, B37, B51, B1, B75 | C15 |
| 8 | Week-one rule, custom start | B31, B34, B40 | C5 |
| 9 | `speedate` | B57, B61, B80 | C14 |
| 10 | `winnow` | B57, D7, B80 | C13 |
| 11 | `logos` | B80 (shared lexer base) | open |
| 12 | `jiff` | B80, B57, B61 | open |
| 13 | Grammar shape | P2 | open |
| 14 | Grammar crate vs in-repo | B79, P3, prep step (ticket 30, section "Prep before grilling") | open |
| 15 | Bare weekday meaning | P2 | C6 |
| 16 | Interval precision of phrases | P2, B18 | open |
| 17 | Relative keyword spelling | B69, D8 | C11/D8 |
| 18 | `member_of` spelling | B68 | C11 |
| 19 | Civil/instant type shape | B9, B10, B16, B58 | C8 |
| 20 | `CalendarPeriod` in value model | B9, B10, B31, B52 | C9 |
| 21 | Ambiguous/absent midnight rule | B16, B30, B32 | open, keep separate from membership (ticket 30, Question 21) |
| 22 | Mixed temporal sort key | B33, B65, B66, B67 | Complete value-sort key and unit rank; C3 preserves group order |
| 23 | Duration refinement | B23 | C7 |
| 24 | Calendar arithmetic documentation | B23, B81, 19 TODOs 315-338 | open |
| 25 | `date(link)` target reading | B22, B74, B75 | open Markdown-link sub-questions |
| 26 | `[date]` syntax and validation | B42, B45, B46 | C17 |
| 27 | `DateConfig` fields | B42, B44, current surface [V] | C17 |
| 28 | Temporal-owned policy type | B27, B39, P1 | C17 |
| 29 | Zone syntax and validation | B42 | C17 |
| 30 | System-zone acquisition and failure | B29, B30, B42 | C10 |
| 31 | Week start configurable | B34, B40 | C17 |
| 32 | Bare-year opt-in key | B34, B44, B60 | C15 |
| 33 | Frame exports | B28, B39, B41 | C18 |
| 34 | Ambient read replacements | B38 | C18 |
| 35 | Probe bounds | B53, B70, B71, B72 | C12 |
| 36 | Null-date notice | B21, B73 | C11 |

## 6. Source-screening notes

Claims re-checked before repeating:

- Research 07's template-adapter placement is a proposal [H]. The code has
  moved since. Do not repeat that placement as an accepted rule (C1).
- The owner's vault confirms the Moment tokens in use [V] (B26, C1).
  K8 narrows research 14's Fileclass claims. Its input parser accepts
  mixed weeks and does not apply the captured sign.
- Research 19-prep `g-links-holding-dates.md` reports earlier grammar probes.
  Ticket 19's accepted `date(link)` rule supersedes that report's
  recommendation. Keep the historical code fact that `date()` rejects
  `Link` at that inspection date [V].
- Research 21-prep `r5-temporal-adversarial.md` records the skipped-day
  counterexample (`Pacific/Apia` 2011-12-30), min-days examples, and a
  98-site `ZoneGuard` census
  [V/H](../21-prep/r5-temporal-adversarial.md), lines 22-34,
  [21-prep/e-design:248](../21-prep/e-design-temporal.md), line 248).
  Treat the census as historical. Ticket 21 adopted the skipped-day and
  exclusive-bound rules. Use C10 for the corrected Jiff fallback identity.
- Do not cite `22-prep/n1`, `n2b`, `n5`, `n6` or `23-prep/h1` to `h3`.
  The ticket 22 and 23 Answers mark those agent files unreliable
  ([22](../../issues/22-config-ownership-and-lifecycle.md), lines 72-75,
  [23](../../issues/23-note-document-model.md), lines 255-257).
- Ticket 30's Current code section is fresher than the survey and
  `reverification.md`; the ticket itself notes both predate the translator
  `DEFAULT_DATETIME_PATTERN` is `%Y-%m-%dT%H:%M:%S` [V]
  (`src/temporal/datetime.rs`, `DEFAULT_DATETIME_PATTERN`); config
  defaults in `src/config/model/frontmatter.rs`, date-field defaults.
  The stale `%Y-%m-%d %H:%M` value in research 22-prep a2 is not the
  current default. No formatting execution ran.

## 7. Supersession list for ticket 36 (input from this file)

1. Ticket 20's truncated sentence (I1).
2. Ticket 18's value list versus ticket 21's filename-derived
   `CalendarPeriod` (C9).
3. Ticket 25's `winnow` omission in the extractor crate list (D7).
4. Ticket 23's Question versus its later input on `IndexPolicy` date keys
   ([30](../../issues/30-temporal-boundary-and-dialects.md#checks-before-resolution), section "Checks before resolution").
5. Ticket 29's order is accepted; ticket 30's earlier “nobody defined the
   order” sentence was repaired. Q22 still needs the complete mixed-value
   sort key and period unit tie-break. Also record the `eom`
   keyword-versus-function wording (C3, D8).
6. The temporal survey and `reverification.md` predate the translator move
   (B80 vs the survey figures; see
   [30](../../issues/30-temporal-boundary-and-dialects.md#checks-before-resolution), section "Checks before resolution").
7. Ticket 19's glossary TODOs stay deferred until ticket 30 settles duration
   wording ([19](../../issues/19-coercion-and-comparison-policy.md), lines 320-325).

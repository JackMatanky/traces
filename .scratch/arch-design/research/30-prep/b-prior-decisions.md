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
| 02 value-model precedent | Screened out, superseded | Its findings entered the [18 Answer](../../issues/18-value-model.md:456). Read the Answer, not the research. |
| 03 coercion precedent | Screened out, superseded | Its findings entered the [19 Answer](../../issues/19-coercion-and-comparison-policy.md:62). One rule survives verbatim: no comparison reads an ambient zone or clock. |
| 04 execution-context precedent | Screened out, superseded | Superseded by the [21 Answer](../../issues/21-ambient-environment-and-execution-context.md:70). Its "sample `now` once" lesson is accepted text there. |
| 05 freshness precedent | Screened out, superseded | Superseded by the [25 Answer](../../issues/25-index-freshness-and-invalidation.md:277). Read the Answer for the fingerprint. |
| 06 query-plan precedent | Screened out, superseded | Superseded by the [29 Answer](../../issues/29-query-core-and-plan.md:481). |
| 07 template-bridge precedent | Relevant, one claim challenged | [Moment is not a core concept](../07-template-bridge-precedent.md:87-88) is [H]. The code moved the translator into `src/temporal/token.rs` at `6104d0ae` [V]. The owner must settle Q1/Q2 against this tension. |
| 08 errors-topology precedent | Screened out, out of scope | Error topology belongs to ticket 34. Ticket 30 only supplies the temporal error set. |
| 09 capability-path precedent | Screened out, superseded | Superseded by the [32 Answer](../../issues/32-path-and-capability-model.md:360). |
| 10 document-model precedent | Screened out, superseded | Superseded by the [23 Answer](../../issues/23-note-document-model.md:249). |
| 11 performance evidence | Relevant, two numbers live | The 80-byte unified enum measure [V](../../issues/11-research-performance-evidence.md:49) binds Q19. The survey's "temporal is 10k lines" figures are stale; [ticket 30 Current code](../../issues/30-temporal-boundary-and-dialects.md:147-154) replaces them [V]. |
| 12 IWE core and hosts | Relevant, negative evidence | IWE has no date type and hands YAML dates to its validator as ISO strings [V](../../issues/30-temporal-boundary-and-dialects.md:324-325). No temporal boundary input beyond that. |
| 13 IWE query schema | Screened out | No date kind in IWE's query engine [V](../../issues/19-coercion-and-comparison-policy.md:349). Tests ticket 19's rules only. |
| 14 Fileclass research | Relevant, verified | Owner's Fileclass data uses Moment formats `YYYY`, `DDDD`, `Q` [V](../../issues/30-temporal-boundary-and-dialects.md:207-209). Fileclass checks dates with ISO regexes only; custom formats accept any non-empty string [V](../14-fileclass-model-binding-and-validation.md:115-119). Duration check is an RFC 5545 subset without years or months [V](../../issues/30-temporal-boundary-and-dialects.md:319-320). |
| 15 schematter research | Screened out | Body-section scope. No temporal content. |
| 16 compatibility envelope | Binding | Defines Frozen/Migratable/Free for every surface this ticket touches. See ledger B1 to B8. |
| 17 forward consumers | Binding, scope only | Sets the seven measurement points (50 to 50,000 notes) [V](../../issues/17-grilling-forward-consumers-and-design-pressure.md:51-55). Crate decisions Q9 to Q12 need measurement evidence at those points; no binary-size claim is measured today [H]. |
| 18 value model | Binding | `NoteFieldValue` variants, the 80-byte test, and the Link model. See B9 to B13. |
| 19 coercion and comparison | Binding | The three relations, adaptive literals, `date(link)`, duration provisional rule, notice set. See B14 to B24. |
| 20 schema typing | Binding, one fragment | Advisory check, `date.format` gap left explicit for this ticket, and one truncated sentence. See B25 to B27 and I1. |
| 21 execution context | Binding | The largest temporal input: frames, civil periods, filename prefixes, removals. See B28 to B41. |
| 22 config ownership | Binding | `[date]` ownership, zone precedence, fingerprint exclusion. See B42 to B46. |
| 23 note document model | Binding | Parser classification, task dates, effective accessor. See B47 to B50. |
| 24 index read model | Binding, as amended | Store keeps temporal values as text; no `day` persisted; range-key proof duty. See B51 to B56. |
| 25 freshness | Binding, as amended | Extractor fingerprint crate list, civil-variant epoch duty, period-policy condition. See B57 to B62 and D7. |
| 26 journal measurement | Screened out, one handoff | No temporal content. Its read-only-open control passed to ticket 25 [V](../../issues/26-measure-index-recovery-journal.md:74-75). |
| 27 replay prototype | Screened out, one amendment | No temporal content. Platform-family supersession noted under ticket 25 (B59). |
| 28 live buffer | Binding, narrow | Buffer rows keep `file.size` as buffer bytes; the pinned generation and frame interact with Q33. See B63, B64. |
| 29 query core | Binding | Group classes, `member_of` reservation, pushdown protocol, keywords at plan time, notices. See B65 to B73 and the Main-confirmed C4. |
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
  ([16:76](../../issues/16-grilling-compatibility-envelope.md:76)). This binds
  the shared note parser. Ticket 21 scopes the new filename-period parser to
  filenames only ([21:175-176](../../issues/21-ambient-environment-and-execution-context.md:175-176)),
  and ticket 30 must check the shared parser separately
  ([30:573-574](../../issues/30-temporal-boundary-and-dialects.md:573-574)).
  Today the note parser calls `DatePoint::parse_prefix` for task dates and
  inline fields [V](../../issues/30-temporal-boundary-and-dialects.md:185-187).
- **B2. Template and query capabilities are Frozen.** Date arithmetic stays
  reachable ([16:89](../../issues/16-grilling-compatibility-envelope.md:89)).
- **B3. Template function API and query DSL spelling are Migratable.** This
  includes heuristic string-to-date promotion
  ([16:90](../../issues/16-grilling-compatibility-envelope.md:90)). A
  dialect change needs a change-table row.
- **B4. `config.toml` and schema TOML formats are Migratable.** A `[date]`
  table and a `date.format` check need a change-table row
  ([16:86](../../issues/16-grilling-compatibility-envelope.md:86)).
- **B5. `index.redb` layout and refresh strategy are Free**; the store was
  already wiped once ([16:80](../../issues/16-grilling-compatibility-envelope.md:80)).
- **B6. Exit codes and stdout/stderr split are Frozen**
  ([16:81](../../issues/16-grilling-compatibility-envelope.md:81)). A
  higher-epoch read miss exits 0 with one stderr notice, per ticket 25.
- **B7. Dataview/Templater semantic parity is Free**; historical influence
  only ([16:78](../../issues/16-grilling-compatibility-envelope.md:78)).
  Moment parity is therefore free; ticket 30 decides it on design grounds,
  not compatibility grounds.
- **B8. Visibility rule: reduce visibility by default; justify each `pub`**
  ([16:94-100](../../issues/16-grilling-compatibility-envelope.md:94)).
  The divergence register is not an input
  ([16:60-62](../../issues/16-grilling-compatibility-envelope.md:60)); use
  `.scratch/temporal-core/divergence-register.md` as a pointer only
  ([30:111-114](../../issues/30-temporal-boundary-and-dialects.md:111)).

### 2.2 Value model (ticket 18)

- **B9. `NoteFieldValue` is the single runtime value.** Variants: `Null`,
  `Bool`, `Int(i64)`, `Float(f64)`, `String`, `Date`, `DateTime`, `Duration`,
  `Link(Link)`, `List(Box<[Self]>)`, `Object(IndexMap<String, Self>)`
  ([18:482-489](../../issues/18-value-model.md:482)). Ticket 19 revoked the
  exclusion of inline `DateTime` ([18:518](../../issues/18-value-model.md:518),
  [19:117-118](../../issues/19-coercion-and-comparison-policy.md:117)).
- **B10. The 80-byte size test stays.** Do not box more variants
  ([18:484-485](../../issues/18-value-model.md:484)). Sizes: `DurationValue`
  72 B, `DateTimeValue` 16 B, `DateValue` 4 B [V in research 11]
  ([30:467-468](../../issues/30-temporal-boundary-and-dialects.md:467)). A
  civil variant must keep the total at 80 bytes
  ([30:469](../../issues/30-temporal-boundary-and-dialects.md:469)). A period
  variant, if added, meets the same test (ticket 18 lists no period variant,
  [30:465-466](../../issues/30-temporal-boundary-and-dialects.md:465); Q20
  decides placement).
- **B11. Schema literals stay a separate type** without `Date`, `DateTime`,
  `Duration` or `Link` ([18:491-493](../../issues/18-value-model.md:491)).
- **B12. Root `Link` carries `target`, normalized `display`, `embedded`, and,
  after ticket 23, the syntax kind**
  ([18:499-505](../../issues/18-value-model.md:499),
  [23:436-437](../../issues/23-note-document-model.md:436)). Link equality
  uses the folded target plus subpath; `display` and `embedded` do not decide
  equality ([19:147-149](../../issues/19-coercion-and-comparison-policy.md:147),
  [19:172-174](../../issues/19-coercion-and-comparison-policy.md:172)).
- **B13. Temporal values persist through derived tagged serde; the cache
  invalidates on a model change**
  ([18:529-531](../../issues/18-value-model.md:529)). Ticket 24 keeps them as
  text until a compact encoding shows a measured gain (B54).

### 2.3 Coercion and comparison (ticket 19, as refined)

- **B14. One value-semantics module owns three separate relations**: loose
  equality, bracketed order, total sort order
  ([19:64-102](../../issues/19-coercion-and-comparison-policy.md:64)).
  Across brackets `==` is false; relational operators do not match
  ([19:92-95](../../issues/19-coercion-and-comparison-policy.md:92)).
- **B15. Temporal brackets hold `Date`, `DateTime` and `CalendarPeriod`**
  ([19:90-92](../../issues/19-coercion-and-comparison-policy.md:90),
  [30:291-292](../../issues/30-temporal-boundary-and-dialects.md:291)).
- **B16. Naive datetimes are civil, zone-free values. Only input with `Z` or
  an offset is an instant**
  ([19:110-113](../../issues/19-coercion-and-comparison-policy.md:110)). An
  ordinary `Date` field compares with a civil `DateTime` field at civil
  midnight ([19:113-114](../../issues/19-coercion-and-comparison-policy.md:113)).
- **B17. Civil-versus-instant comparison uses the pinned zone from the
  execution frame. No ambient zone lookup in ordering**
  ([19:115-117](../../issues/19-coercion-and-comparison-policy.md:115)).
- **B18. Period membership is a filter relation, not `PartialEq` or `Ord`.**
  Point-in-period `==` matches when the point's civil date is inside the
  period. Relational operators use period bounds with an exclusive upper
  bound for instants
  ([19:150-160](../../issues/19-coercion-and-comparison-policy.md:150)).
  Ticket 21 refines this. Period equality is exact `(start, unit)` identity.
  Relational order uses that key with an explicit unit tie-break. Mixed
  temporal sorting has a distinct total key
  ([21:149-156](../../issues/21-ambient-environment-and-execution-context.md:149),
  [19:283-297](../../issues/19-coercion-and-comparison-policy.md:283)).
- **B19. A quoted date-shaped literal parses once at build time into
  candidates**: `DatePoint` and its validated `CalendarPeriod`, `DateTime`,
  `Duration`, `Link` target, `Number`, `Bool`, and later a natural-language
  date ([19:133-137](../../issues/19-coercion-and-comparison-policy.md:133)).
  Do not parse or allocate per row
  ([19:145-146](../../issues/19-coercion-and-comparison-policy.md:145)).
- **B20. A period literal matches a whole span; the upper bound is
  exclusive** ([30:295-296](../../issues/30-temporal-boundary-and-dialects.md:295)).
  This supersedes ticket 19's inclusive `[s, e]` wording
  ([19:283-297](../../issues/19-coercion-and-comparison-policy.md:283)).
- **B21. Advisory notices: a build-time advisory covers an invalid
  date-shaped quoted literal (for example `"2025-02-30"` runs as a string).
  A bare invalid date literal stays a syntax error**
  ([19:207-211](../../issues/19-coercion-and-comparison-policy.md:207)).
  Ticket 34 owns the notice mechanism
  ([19:212](../../issues/19-coercion-and-comparison-policy.md:212)).
- **B22. `date(link)` reads only the target's last path segment.** Strip
  folders, `#anchor` and `.md`; never read display text; accept an optional
  pattern argument for non-ISO titles; return `Null` on parse failure
  ([19:172-180](../../issues/19-coercion-and-comparison-policy.md:172),
  [19:307-310](../../issues/19-coercion-and-comparison-policy.md:307)).
  Today `date()` rejects `Link` [V]
  ([19-prep/g:20](../19-prep/g-links-holding-dates.md:20)). Two Markdown-link
  questions remain. Do `[[b]]` and `[b](b.md)` fold to one
  `ComparisonKey`? How does `date(link)` read a percent-encoded Markdown
  target ([19:299-313](../../issues/19-coercion-and-comparison-policy.md:299),
  [30:284-288](../../issues/30-temporal-boundary-and-dialects.md:284))?
- **B23. Duration equality and order keep nominal seconds provisionally**
  (`1mo == 30d`, `1y == 365d`); calendar arithmetic stays Jiff-based;
  refinement belongs to ticket 30
  ([19:182-184](../../issues/19-coercion-and-comparison-policy.md:182)).
  Q23 must refine or re-affirm with a counterexample. Precedent: PostgreSQL
  keeps months separate; DuckDB and Dataview normalize nominally
  [V](../19-prep/e-query-engine-conventions.md:84-89).
- **B24. Classification happens once at parse time; only query string
  literals get late typing. No comparison reads an ambient clock or zone**
  ([19:64-68](../../issues/19-coercion-and-comparison-policy.md:64)).

### 2.4 Schema typing (ticket 20)

- **B25. Schemas check note data on demand, advisory, pure and span-free.
  Schemas never enter the index or its fingerprint**
  ([20:53-70](../../issues/20-schema-note-typing-and-validation.md:53)).
- **B26. `date.format` is a write-format hint. The check accepts
  Date/DateTime and any scalar when the format has no date component. This
  gap stays explicit until ticket 30 fixes the dialect. After choosing a
  dialect, add the date-format check**
  ([20:120-123](../../issues/20-schema-note-typing-and-validation.md:120),
  [30:300-307](../../issues/30-temporal-boundary-and-dialects.md:300)).
  Q4 decides how a format with no full date (`YYYY`, `Q`, `DDDD`) is
  treated. The owner's data uses exactly those formats
  ([30:207-209](../../issues/30-temporal-boundary-and-dialects.md:207)).
- **B27. Layering: cli, lsp, mcp are thin hosts of `core`; domain modules
  expose data and pure functions; no one-implementor traits**
  ([20:72-77](../../issues/20-schema-note-typing-and-validation.md:72)).
  This binds Q28's policy handoff shape.
- **I1. Truncated fragment.** Ticket 20's Answer ends mid-sentence: "Only
  date, datetime and" ([20:311-315](../../issues/20-schema-note-typing-and-validation.md:311)).
  Do not read a rule into that fragment
  ([30:309-310](../../issues/30-temporal-boundary-and-dialects.md:309)).
  Ticket 36 must recover the intended rule before anyone adopts a
  date-format value check beyond B26.

### 2.5 Execution context (ticket 21)

- **B28. One frame per query, render or request pins the clock and the week
  rule. The frame acquires the zone only when needed; a failure is
  memoized**
  ([21:106-110](../../issues/21-ambient-environment-and-execution-context.md:106),
  [30:365-366](../../issues/30-temporal-boundary-and-dialects.md:365)).
- **B29. A host may select UTC explicitly. UTC is never an implicit
  fallback**
  ([21:127](../../issues/21-ambient-environment-and-execution-context.md:127),
  [30:367-368](../../issues/30-temporal-boundary-and-dialects.md:367)).
- **B30. A missing required zone, calendar resolution or presentation is a
  typed resolution, materialization or render failure. Never drop the row,
  return `Null`, or blank the cell**
  ([21:123-127](../../issues/21-ambient-environment-and-execution-context.md:123),
  [19:203-206](../../issues/19-coercion-and-comparison-policy.md:203)).
- **B31. `CalendarPeriod` becomes a validated value: a civil start date plus
  a unit. The boundary-selector role renames to `CalendarUnit` with `Day`,
  `Week`, `Month`, `Quarter`, `Year`. No second period type, no heap
  allocation for this value**
  ([21:136-141](../../issues/21-ambient-environment-and-execution-context.md:136)).
  Today `CalendarPeriod` has `Week`, `Month`, `Year` only, and `Week` is the
  ISO week [V](../../issues/30-temporal-boundary-and-dialects.md:172-174).
- **B32. Period-versus-instant filters use the instant's civil date in the
  pinned zone. A skipped local day matches no instants; a skipped next day
  does not invalidate a real prior day**
  ([21:142-145](../../issues/21-ambient-environment-and-execution-context.md:142)).
  Civil points use their civil dates without a zone
  ([30:344-345](../../issues/30-temporal-boundary-and-dialects.md:344)).
- **B33. Do not use Rust `Eq`/`Ord` for membership. Use a separate
  deterministic total sort key for mixed temporal values; the unit tie-break
  for period sort stays open for this ticket**
  ([21:148-156](../../issues/21-ambient-environment-and-execution-context.md:148),
  [21:269-270](../../issues/21-ambient-environment-and-execution-context.md:269)).
- **B34. Filename prefixes: keep day/month recognition. Add validated ISO
  week forms (Monday start, four-day week one) and quarter forms. Enable a
  bare year prefix only through `DateConfig`. Prefer the longest valid
  shape; never reinterpret a malformed longer prefix as a shorter one**
  ([21:170-174](../../issues/21-ambient-environment-and-execution-context.md:170)).
- **B35. `file.date` holds a filename-derived `CalendarPeriod`; `Null` when
  no recognized prefix. Never fall back to creation time. `file.day` is
  retired and keeps no new meaning**
  ([21:157-161](../../issues/21-ambient-environment-and-execution-context.md:157)).
- **B36. `file.cdate`/`file.ctime` use only creation time; `Null` when
  unavailable; never substitute mtime. `file.mdate` uses mtime. Timestamp
  fields stay instants; `cdate`/`mdate` project to civil dates in the pinned
  zone during evaluation, with a typed failure if the zone is unavailable.
  Never persist those zone-derived dates**
  ([21:162-169](../../issues/21-ambient-environment-and-execution-context.md:162)).
- **B37. The new prefix forms apply to filenames only; the frozen shared
  parser does not change**
  ([21:175-176](../../issues/21-ambient-environment-and-execution-context.md:175)).
- **B38. Ambient removals: `local_zone()` leaf reads, per-row zone lookups,
  and the test-only `ZoneGuard` leave after injection lands**
  ([21:196-200](../../issues/21-ambient-environment-and-execution-context.md:196),
  [30:368-370](../../issues/30-temporal-boundary-and-dialects.md:368)).
  The evaluator must not read the clock or ambient zone
  ([21:115-119](../../issues/21-ambient-environment-and-execution-context.md:115)).
  Today `local_zone` and `ZoneGuard` still exist [V]
  (`src/temporal/mod.rs:64`, `src/temporal/mod.rs:100`).
- **B39. Temporal owns validated periods, week rules and explicit-zone civil
  conversion. Query owns pinned frames. Template owns the render frame**
  ([21:242-245](../../issues/21-ambient-environment-and-execution-context.md:242),
  [30:370-371](../../issues/30-temporal-boundary-and-dialects.md:370)).
- **B40. Left to later tickets by ticket 21**: the period sort unit
  tie-break, compact encoding, week formats beyond ISO, and the configured
  zone source
  ([21:269-274](../../issues/21-ambient-environment-and-execution-context.md:269)).
- **B41. A lazy `QuerySet` keeps its creator's resolved plan and frame;
  terminal access must not hide execution errors**
  ([21:128-132](../../issues/21-ambient-environment-and-execution-context.md:128),
  [29:979-981](../../issues/29-query-core-and-plan.md:979)).

### 2.6 Config ownership (ticket 22)

- **B42. `config` owns and parses the `[date]` table into `DateConfig`.
  Ticket 30 decides the syntax and validation**
  ([22:227-228](../../issues/22-config-ownership-and-lifecycle.md:227)).
  Zone precedence: host override, then config, then system
  ([22:229-230](../../issues/22-config-ownership-and-lifecycle.md:229)).
- **B43. The time zone never enters the index fingerprint**
  ([22:114](../../issues/22-config-ownership-and-lifecycle.md:114)).
- **B44. Ticket 22 removed `title`, `aliases` and the date keys from the
  fingerprint because no production code reads them. An index-time read must
  add its key to `IndexPolicy`**
  ([22:109-111](../../issues/22-config-ownership-and-lifecycle.md:109)).
  `IndexPolicy` holds the filename-period policy only if the index stores
  parsed periods
  ([22:100-101](../../issues/22-config-ownership-and-lifecycle.md:100)).
- **B45. `deny_unknown_fields` stays; no `version` key until the first
  breaking change. The `[date]` table needs an exact field set**
  ([22:225-226](../../issues/22-config-ownership-and-lifecycle.md:225),
  [30:408](../../issues/30-temporal-boundary-and-dialects.md:408)).
- **B46. Config merge applies global first, then local, field by field.
  Ticket 30 must state whether `[date]` follows that rule**
  ([22:231-233](../../issues/22-config-ownership-and-lifecycle.md:231),
  [30:409-410](../../issues/30-temporal-boundary-and-dialects.md:409)).
  Config values from a local file apply only after the trust gate passes
  ([30:412-413](../../issues/30-temporal-boundary-and-dialects.md:412)).
- **P1. Edge direction.** `config` reads `DEFAULT_DATETIME_PATTERN` from
  temporal today [V](../../issues/30-temporal-boundary-and-dialects.md:422-423);
  temporal imports `DelimiterType` from the crate root in
  `src/temporal/token.rs:12` [V]. The ticket records a [P] lean: keep the
  edge from `config` to temporal
  ([30:423-424](../../issues/30-temporal-boundary-and-dialects.md:423)).
  This is a proposal, not an accepted rule.
- **Current config surface [V]**: no `DateConfig` type exists. Date config
  is `DateFieldConfig { name, format }` under `[frontmatter.date_created]`
  and `[frontmatter.date_modified]`, default format `%Y-%m-%d %H:%M` from
  `DEFAULT_DATETIME_PATTERN`
  ([22-prep/a2:253-262](../22-prep/a2-inventory-digests-lifecycle.md:253)).
  The ticket records the same fields and the default
  `%Y-%m-%dT%H:%M:%S` [V](../../issues/30-temporal-boundary-and-dialects.md:191-194);
  the two sources disagree on the default string. Check the constant at
  implementation time. Zone, week rules and bare-year parsing have zero
  config presence today [V](../22-prep/a2-inventory-digests-lifecycle.md:262).

### 2.7 Note document model (ticket 23)

- **B47. The parser classifies each value once at parse time; frontmatter
  and inline fields share one parser set and one precedence order**
  ([23:334-336](../../issues/23-note-document-model.md:334),
  [30:477-479](../../issues/30-temporal-boundary-and-dialects.md:477)).
- **B48. Inline fields can yield `DateTime`; a naive one is civil**
  ([23:477](../../issues/23-note-document-model.md:477), read through the
  ticket 30 inputs,
  [30:298](../../issues/30-temporal-boundary-and-dialects.md:298)).
- **B49. Task dates stay civil `DateValue` values; `TaskDateSet` holds six
  of them** ([30:480-481](../../issues/30-temporal-boundary-and-dialects.md:480)).
  [V] `TaskDateSet` at `src/task.rs:498-505` holds six `Option<DateValue>`
  slots with first-wins insert.
- **B50. A fact-changing parser setting enters `IndexPolicy`; a change of
  the stored `Note` changes the extractor fingerprint or bumps the format
  epoch** ([23:368-371](../../issues/23-note-document-model.md:368)).

### 2.8 Index read model (ticket 24, as amended by 28/29/32)

- **B51. The store persists source timestamps and size; it does not persist
  `day`. A query derives the filename period; this ticket owns that parser**
  ([24:402-405](../../issues/24-index-read-model-and-query-execution.md:402),
  [30:449-450](../../issues/30-temporal-boundary-and-dialects.md:449)).
- **B52. The store keeps temporal values as text until a compact encoding
  shows a measured gain; postcard re-parses them on each load**
  ([24:400-401](../../issues/24-index-read-model-and-query-execution.md:400),
  [30:451-452](../../issues/30-temporal-boundary-and-dialects.md:451)).
- **B53. A range key needs a proof for each operator, row kind, temporal
  frame and field precedence. Unsupported mappings scan**
  ([24:312-327](../../issues/24-index-read-model-and-query-execution.md:312)).
- **B54. A civil datetime must deserialize from the cache without a zone
  lookup; the store persists naive datetimes as civil values and holds no
  zone**
  ([30:437](../../issues/30-temporal-boundary-and-dialects.md:437),
  [24:455](../../issues/30-temporal-boundary-and-dialects.md:455)).
- **B55. Research 25 says a persisted key for `file.mdate` keeps UTC
  instants, not civil dates**
  ([30:456-457](../../issues/30-temporal-boundary-and-dialects.md:456)).
- **B56. Ticket 29 ships range keys first for unquoted numeric, temporal and
  bool literals; no text range table at first; bracket counts live in the
  `meta` facet and delete old derived facts first**
  ([24:761-765](../../issues/24-index-read-model-and-query-execution.md:761)).

### 2.9 Freshness (ticket 25, as amended by 28/29/32)

- **B57. The extractor fingerprint hashes the resolved versions of the
  crates that feed persisted values: `pulldown-cmark`, `logos`, `noyalib`,
  `jiff`, `speedate` and `postcard`, plus encoded probe records. Whoever
  prunes a crate updates the list**
  ([25:304-311](../../issues/25-index-freshness-and-invalidation.md:304),
  [30:430-433](../../issues/30-temporal-boundary-and-dialects.md:430)).
- **D7. Recorded omission.** Ticket 25's crate list omits `winnow`. Ticket
  30 records the effect. `winnow` locates duration prefixes for inline fields.
  Those values reach the store. Keep `winnow` in the list, or remove it from
  the code ([30:434-436](../../issues/30-temporal-boundary-and-dialects.md:434)).
  [V] `winnow` 1.0.4 has one call site, `src/temporal/lexer.rs:7,118`.
  This appears in the ticket 36 supersession list
  ([30:586](../../issues/30-temporal-boundary-and-dialects.md:586)).
- **B58. The corpus uses explicit-offset datetimes only until the civil
  variant lands; adding it bumps the format epoch**
  ([25:332-334](../../issues/25-index-freshness-and-invalidation.md:332),
  [30:438-440](../../issues/30-temporal-boundary-and-dialects.md:438)).
- **B59. Store identity: engine tag, format epoch, extractor fingerprint,
  policy fingerprint, written-by version, canonical root; platform family
  removed by ticket 32. Store-open rules and rebuild/miss policy are
  ticket 25's, as amended**
  ([25:285-302](../../issues/25-index-freshness-and-invalidation.md:285),
  [32:1150-1159](../../issues/32-path-and-capability-model.md:1150)).
- **B60. Ticket 25 does not fingerprint filename-period rules; the store
  does not persist them. If a persisted facet starts to use those rules, the
  rules enter `IndexPolicy` and the corpus**
  ([30:441-443](../../issues/30-temporal-boundary-and-dialects.md:441)).
- **B61. A change of the parsing stack changes the fingerprint and forces
  one rebuild** ([30:430-431](../../issues/30-temporal-boundary-and-dialects.md:430)).
- **B62. Journal and replay batches accept validated disk facts only; journal
  fact paths use canonical UTF-8 `/` text**
  ([26:502-506](../../issues/26-measure-index-recovery-journal.md:502),
  [27:629-640](../../issues/27-index-replay-prototype-and-fault-tests.md:629)).
  No temporal content otherwise.

### 2.10 Query core (ticket 29)

- **B65/C4. Temporal group class order is accepted text: Civil, Instant,
  Period.** Ticket 29 section 3 sets it:
  "The classes stay apart and order Civil, Instant, Period"
  ([29:721-730](../../issues/29-query-core-and-plan.md:721)). Confirmed
  owner per Main, 2026-11-16. Ticket 30's line "Nobody has defined that
  order" ([30:537-538](../../issues/30-temporal-boundary-and-dialects.md:537))
  is stale and belongs in the supersession list, not a reopening. Q22 keeps
  only the mixed temporal value sort key and the period unit tie-break.
- **B66. A `Date` and a civil-midnight `DateTime` share a group key when
  their civil values agree. Civil values and instants stay separate classes.
  A period groups by exact `(start, unit)`. Membership never defines a group
  key or a range key**
  ([30:533-536](../../issues/30-temporal-boundary-and-dialects.md:533),
  [29:727-730](../../issues/29-query-core-and-plan.md:727)).
- **B67. Mixed civil/instant comparators are unsound across a DST gap.** The
  cycle is civil 02:30 < civil 03:00 < instant 07:15Z < civil 02:30 in
  `America/New_York` on 2026-03-08. `sort_by` panicked in 15 of 200 runs [V]
  ([29:724-727](../../issues/29-query-core-and-plan.md:724),
  [29-prep/h3:161-163](../29-prep/h3-redteam-round2.md:161)). The fix:
  project each temporal value to its class key once under the pinned zone;
  never compare raw civil and instant values.
- **B68. `member_of` is reserved as an IR node name only; no frontend, no
  implementation. The spelling waits on ticket 30 (Q18). Its relation is
  never a range key and never a group key**
  ([29:1186-1191](../../issues/29-query-core-and-plan.md:1186)).
- **B69. Relative keywords (`today`, `eom`) bind to the pinned frame at plan
  time. A standalone query acquires a new frame; a render or request
  supplies its shared frame**
  ([29:976-978](../../issues/29-query-core-and-plan.md:976),
  [30:524](../../issues/30-temporal-boundary-and-dialects.md:524)).
  Q17 decides the filter spelling.
- **B70. Temporal probe bounds need operator-specific proofs for `Date`,
  civil `DateTime`, instant and `CalendarPeriod` values under the pinned
  frame. Point-period matching uses membership bounds; period-period
  comparison uses normalized `(start, unit)` identity and order. An unproved
  mapping is Unsupported and scans**
  ([29:538-542](../../issues/29-query-core-and-plan.md:538)). Q35 owns the
  per-operator, per-kind bound table.
- **B71. First pushdown scope: unquoted temporal literals and `And` only;
  quoted literals and `Not`/`Or` scan**
  ([29:571-577](../../issues/29-query-core-and-plan.md:571),
  [30:539](../../issues/30-temporal-boundary-and-dialects.md:539)).
- **B72. Filename-derived `file.date` keys stay Unsupported until the parser
  contract is complete. `file.day` is retired**
  ([29:596-597](../../issues/29-query-core-and-plan.md:596),
  [30:527-528](../../issues/30-temporal-boundary-and-dialects.md:527)).
- **B73. Notices: a bracket mismatch, an invalid date-shaped quoted literal
  and NaN-to-Null ride one stderr mechanism; ticket 34 owns the envelope.
  Ticket 29 did not adopt the first-round notice for rows with a `Null`
  date. Tickets 30 and 34 decide it (Q36)**
  ([29:1124-1138](../../issues/29-query-core-and-plan.md:1124),
  [29:1198-1199](../../issues/29-query-core-and-plan.md:1198),
  [30:531-532](../../issues/30-temporal-boundary-and-dialects.md:531)).
- **D8. Recorded discrepancy.** Ticket 29 uses `eom` as a relative keyword,
  although the current grammar defines `eom` as a function that takes a date
  argument [V](../../issues/30-temporal-boundary-and-dialects.md:167-171).
  Ticket 36 records it ([30:589-590](../../issues/30-temporal-boundary-and-dialects.md:589)).
  Q17 resolves the spelling.

### 2.11 Paths (ticket 32)

- **B74. `RelativePath` is a normalized, `/`-joined UTF-8 `String`; pure, no
  I/O; rejects NUL and `..`; its `Ord` maps `/` to NUL**
  ([32:431-461](../../issues/32-path-and-capability-model.md:431)).
- **B75. The filename-period parser reads the last segment of that string.
  Today `derive_day` reads a `Path` through `BaseNameRef::from_path`**
  ([30:513-518](../../issues/30-temporal-boundary-and-dialects.md:513)).
  [V] `FileMeta::derive_day` at `src/file.rs:87-97`: filename prefix wins,
  else `created_at` (the Dataview rule). Ticket 21 removes that fallback
  (B35).
- **B76. The scan skips non-UTF-8 names with a notice**
  ([30:516](../../issues/30-temporal-boundary-and-dialects.md:516)).
- **B77. The Comparison Key (NFC, lowercase, NFC) is a lookup aid only. It
  never applies to redb keys, sort order, I/O paths or overlay keys**
  ([32:657-670](../../issues/32-path-and-capability-model.md:657)).
  Fold-based link resolution bumps the extractor fingerprint epoch
  ([32:671-675](../../issues/32-path-and-capability-model.md:671)).
- **B78. Path sort/range key: the path with `/` replaced by `0x00`; folder
  range `[prefix\0, prefix\x01)`**
  ([32:1403-1404](../../issues/32-path-and-capability-model.md:1403), read
  through the ticket 29 handoffs; see also
  [24:744-745](../../issues/24-index-read-model-and-query-execution.md:744)).

### 2.12 Cross-cutting

- **B79. Performance evidence rules.** Use the seven measurement points from
  ticket 17 (B above). Do not claim a design is optimal from line count,
  dependency count or unmeasured cost. Mark unmeasured binary size and
  runtime cost explicitly
  ([brief.md:75-76](./brief.md:75),
  [17:51-55](../../issues/17-grilling-forward-consumers-and-design-pressure.md:51)).
- **B80. Pinned `jiff`.** `jiff = "=0.2.38"` with `default-features = false`,
  features `std`, `tz-system`, `tzdb-bundle-always` [V]
  ([30:179-180](../../issues/30-temporal-boundary-and-dialects.md:179),
  `Cargo.toml:128`). `speedate` 0.18.0 has one call site in
  `src/temporal/date.rs`. `winnow` 1.0.4 has one in `src/temporal/lexer.rs`.
  `logos` 0.16.1 serves `lexer.rs`, `datetime.rs`, `point.rs`, the shared
  lexer, the note parser and the query grammar [V]
  ([30:175-178](../../issues/30-temporal-boundary-and-dialects.md:175),
  `Cargo.toml:129,143,152`).
- **B81. Stale temporal glossary.** `src/temporal/GLOSSARY.md` still says
  unzoned input uses the process zone and a Date promotes to local midnight
  [V](../../issues/30-temporal-boundary-and-dialects.md:200-202). Ticket 19
  supersedes both sentences ([19:110-118](../../issues/19-coercion-and-comparison-policy.md:110),
  [19:320-325](../../issues/19-coercion-and-comparison-policy.md:320)).
  Treat the glossary as stale code documentation, not as an accepted rule.
- **B82. ADR-0010 is `proposed`, not accepted.** It lexes a bare ISO date as
  one query literal [V](../../issues/30-temporal-boundary-and-dialects.md:197-198).
  It constrains nothing until accepted. Ticket 28 records that ticket 19
  fixes the ADR's quoted/unquoted divergence
  ([28:1238](../../issues/28-live-buffer-and-persisted-index-consistency.md:1238)).
- **B83. Divergence register exclusion.** Ticket 16 rules the register is
  not an input; use it as a pointer only (B8).

## 3. Proposal and open-item register

Items that look like rules but are not accepted decisions:

- **P2. Natural-language grammar hook.** Ticket 19 proposes a deterministic,
  closed English grammar. It resolves from injected `today`. Filter literals,
  `date()`, `date_add` and template filters share it. It must yield an
  interval precision for phrases like `next week`. The meaning of a bare
  weekday stays open ([30:269-276](../../issues/30-temporal-boundary-and-dialects.md:269),
  [19:167-170](../../issues/19-coercion-and-comparison-policy.md:167)).
  Ticket 30 owns grammar and crate choice (Q13 to Q16).
- **P3. In-repo grammar default.** Ticket 30 records a [P] lean: run the prep
  first and treat an in-repository grammar as the default until evidence
  beats it ([30:254-255](../../issues/30-temporal-boundary-and-dialects.md:254)).
- **P1. Config-to-temporal edge direction** (see above).
- **P4. `.scratch/temporal-core/design-nl-dates.md` is an unreviewed agent
  draft [H].** It conflicts with accepted decisions in four places:
  - It returns month precision for `next month`. Ticket 19 needs an interval.
  - It adds a template-only `date.relative`. Ticket 19 needs one shared
    grammar.
  - It reads `now_instant()` and `ZoneGuard`. Ticket 21 replaces both.
  - It fixes weekday meanings that ticket 30 must still decide
    ([30:227-238](../../issues/30-temporal-boundary-and-dialects.md:227)).

  Treat it as a pointer, not evidence.
- **P5. Ticket 21 handoffs that await this ticket**: period/unit vocabulary
  outcome, ISO/custom weeks, skipped days, `DateConfig` shape and filename
  recognition ([21:291-292](../../issues/21-ambient-environment-and-execution-context.md:291)).
- **P6. Downstream needs (not blockers)**: ticket 31 needs the translator
  owner, calendar helpers, render civil/instant/period rendering, frame and
  zone-failure error type, unsupported-token error shape
  ([30:543-550](../../issues/30-temporal-boundary-and-dialects.md:543)).
  Ticket 34 needs the temporal error set, typed zone failure, `Null`-date
  notice condition and any new schema date-format code
  ([30:551-553](../../issues/30-temporal-boundary-and-dialects.md:551)).
  Ticket 35 prunes `speedate`/`winnow` after this ticket
  ([30:554-555](../../issues/30-temporal-boundary-and-dialects.md:554)).
  Ticket 36 records the `file.day` to `file.date` migration and checks the
  frozen-parser separation
  ([30:556-558](../../issues/30-temporal-boundary-and-dialects.md:556)).

## 4. Contradictions and exact human decisions needed

Each item names the conflict and the decision owner. None of these may be
resolved by amending an earlier Answer.

- **C1. Moment translator home.** Research 07 concluded Moment is a
  Templater-compat feature, not a temporal concept, and proposed a template
  compat module [H](../07-template-bridge-precedent.md:87-88,
  [30:508-510](../../issues/30-temporal-boundary-and-dialects.md:508)).
  Commit `6104d0ae` moved the translator into `src/temporal/token.rs` [V]
  ([30:152-155](../../issues/30-temporal-boundary-and-dialects.md:152)).
  Ticket 16 makes parity Free (B7), so the decision is design-only. Human
  decision: ratify the temporal home (Q1/Q2), return it to a template
  adapter, or drop Moment tokens. The owner needs three kinds of evidence:
  - The owner's tokens. `docs/vault/07_templates/12_01_week_periodic.md:84-91`
    uses `ww`, `YY`, `Q`. `14_01_quarter_periodic.md:84-89` uses `Q`, `Qo` [V].
  - The unsupported-token behavior. `MOMENT_TOKENS` has no `ww`, `Q`, `Qo`,
    `DDDD`, so those tokens fail [V by reading]
    ([30:160-161](../../issues/30-temporal-boundary-and-dialects.md:160)).
  - The change-table cost (B3).
- **C2. `date.format` dialect and the no-full-date check.** Four format
  surfaces exist: config `format`, query `date(text, pattern)` (strftime
  only today [V](../../issues/30-temporal-boundary-and-dialects.md:195-196)),
  template `date_format`, and schema `date.format` (unvalidated
  `Option<String>` [V](../../issues/30-temporal-boundary-and-dialects.md:188-190)).
  B26 leaves the check gap explicit. I1 forbids reading the truncated
  sentence. Human decision: one dialect or a justified split (Q3), the
  no-full-date rule (Q4), and the fate of `[frontmatter.date_*]` (Q5).
- **C3. Group class order owner.** Ticket 29 §3 lines 721 to 730 fixes
  Civil, Instant, Period. Ticket 30's "nobody has defined that order"
  sentence is stale. Main confirmed on 2026-11-16 these three actions. Cite
  ticket 29 as the accepted owner in the integration synthesis. Record the
  stale sentence in the supersession list. Q22 keeps only the mixed-value
  sort key and the period unit tie-break (B65).
- **C4. Period upper-bound wording.** Ticket 19's original `[s, e]` wording
  conflicts with ticket 21's exclusive bound. Ticket 19 itself records the
  refinement as overriding
  ([19:283-297](../../issues/19-coercion-and-comparison-policy.md:283)).
  No new decision; carry the exclusive rule (B20). Do not re-argue it.
- **C5. Week-one rule under a custom week start.** Ticket 21 fixes ISO for
  the default week only ([30:43-44](../../issues/30-temporal-boundary-and-dialects.md:43)).
  A first-weekday setting alone does not define week-year numbering; minimum
  days in week one matter; ISO `2025-W53` is invalid; accept no rollover
  [V/H](../21-prep/r5-temporal-adversarial.md:31-34). Human decision: Q8.
- **C6. Bare weekday meaning.** Next, nearest, or current-week day. Ticket
  19 leaves it open
  ([19:170-171](../../issues/19-coercion-and-comparison-policy.md:170));
  the unreviewed draft fixes meanings it must not fix (P4). Human decision:
  Q15.
- **C7. Nominal duration equality.** B23 keeps nominal seconds
  provisionally. Q23 decides refine or keep, with a counterexample. Ticket
  36 must reconcile the glossary wording once settled
  ([19:324-325](../../issues/19-coercion-and-comparison-policy.md:324)).
- **C8. Civil/instant type shape.** A `DatePoint` variant or an inner enum;
  `DatePoint` today stores `Calendar { date, date_precision }` or
  `DateTime(DateTimeValue)` [V] (`src/temporal/point.rs:41-52`). The 80-byte
  test (B10) constrains it. Q19 decides; B58 ties the corpus change to the
  landing.
- **C9. Period value placement.** Ticket 18 lists no period variant; tickets
  19 and 21 need period values for `file.date`
  ([30:73-75](../../issues/30-temporal-boundary-and-dialects.md:73)).
  Ticket 36 must reconcile that list
  ([30:584-585](../../issues/30-temporal-boundary-and-dialects.md:584)).
  Q20 decides placement under B10 and B31.
- **C10. Zone acquisition failure reporting.** B29/B30 fix the policy;
  ticket 22 fixes the precedence (B42). Q30 decides the host acquisition
  mechanism and report path with ticket 34. Jiff note: `TimeZone::system()`
  yields `Etc/Unknown` on failure, which is not UTC and must not masquerade
  as one [V](../21-prep/e-design-temporal.md:40).
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
  ([30:162-164](../../issues/30-temporal-boundary-and-dialects.md:162))).
  `PERIOD_EDGE_LOOKUP` and `resolve_period_edge` already live in
  `src/temporal/calendar.rs` and serve query and template [V]
  ([30:165-167](../../issues/30-temporal-boundary-and-dialects.md:165)).
  Q6 decides which helpers stay. Template `date.weekday(n)` selects ISO
  weekday `n` of the current week today [V]
  ([30:174](../../issues/30-temporal-boundary-and-dialects.md:174)).
- **C17. `[date]` table syntax, merge and validation.** B42/B45/B46 fix
  ownership and constraints. Q26 to Q29 decide syntax, fields, the temporal
  policy type, and zone syntax. Q31 decides week-start configurability; B40
  leaves week formats beyond ISO open.
- **C18. Execution-frame exports.** B28/B39/B41 fix ownership. Q33 decides
  what temporal exports to the frame; Q34 names the replacements for
  `local_zone()`, `now_instant()` and `ZoneGuard` reads (their removal is
  already decided, B38). Ticket 33 later wires the frame source and
  `DateConfig` into the host
  ([30:137-140](../../issues/30-temporal-boundary-and-dialects.md:137)).
- **C19. Truncated ticket 20 sentence.** I1: get the intended rule through
  ticket 36 before adopting any value check beyond B26.

## 5. Question-to-constraint map

| Q | Subject | Binding constraints | Open decision |
| --- | --- | --- | --- |
| 1 | Moment as domain concept | B7, B83 | C1 |
| 2 | Translator home | B7, B3, B80, B27 | C1 |
| 3 | Dialect per surface | B3, B4, B80 | C2 |
| 4 | `date.format` no-full-date check | B26, I1 | C2, C19 |
| 5 | `[frontmatter.date_*]` fate | B4, B44, current surface [V] | C2 |
| 6 | Calendar helpers in engine | B39, B31 | C16 |
| 7 | Filename-period parser home | B34, B35, B37, B51, B1, B75 | C15 |
| 8 | Week-one rule, custom start | B31, B34, B40 | C5 |
| 9 | `speedate` | B57, B61, B80 | C14 |
| 10 | `winnow` | B57, D7, B80 | C13 |
| 11 | `logos` | B80 (shared lexer base) | open |
| 12 | `jiff` | B80, B57, B61 | open |
| 13 | Grammar shape | P2 | open |
| 14 | Grammar crate vs in-repo | B79, P3, prep step 30:217-253 | open |
| 15 | Bare weekday meaning | P2 | C6 |
| 16 | Interval precision of phrases | P2, B18 | open |
| 17 | Relative keyword spelling | B69, D8 | C11/D8 |
| 18 | `member_of` spelling | B68 | C11 |
| 19 | Civil/instant type shape | B9, B10, B16, B58 | C8 |
| 20 | `CalendarPeriod` in value model | B10, B31, B52 | C9 |
| 21 | Ambiguous/absent midnight rule | B16, B30, B32 | open, keep separate from membership (30:76-77) |
| 22 | Mixed temporal sort key | B33, B65, B66, B67 | C3 (scope reduced) |
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

- Research 07's "Moment belongs in a template compat module" is [H], and the
  code has moved since. Do not repeat it as fact (C1).
- Research 14's Fileclass facts stand; the owner's vault confirms the
  Moment tokens in use [V] (B26, C1).
- Research 19-prep `g-links-holding-dates.md` numbers are probe results
  against the current grammar; ticket 19's accepted `date(link)` rule
  supersedes its recommendation as a decision. Keep only the code facts
  (`date()` rejects `Link` today [V]).
- Research 21-prep `r5-temporal-adversarial.md` facts are sound: the
  skipped-day counterexample (`Pacific/Apia` 2011-12-30), the min-days
  examples, the `Etc/Unknown` hazard, and the 98-site `ZoneGuard` census
  [V/H](../21-prep/r5-temporal-adversarial.md:22-34,
  [21-prep/e-design:248](../21-prep/e-design-temporal.md:248)). Ticket 21's
  Answer adopted the skipped-day and exclusive-bound rules.
- Do not cite `22-prep/n1`, `n2b`, `n5`, `n6` or `23-prep/h1` to `h3`; the
  ticket 22 and 23 Answers mark those agent files unreliable
  ([22:72-75](../../issues/22-config-ownership-and-lifecycle.md:72),
  [23:255-257](../../issues/23-note-document-model.md:255)).
- Ticket 30's Current code section is fresher than the survey and
  `reverification.md`; the ticket itself notes both predate the translator
  move ([30:591](../../issues/30-temporal-boundary-and-dialects.md:591)).
- The default datetime format string differs between ticket 30
  (`%Y-%m-%dT%H:%M:%S`) and research 22-prep a2 (`%Y-%m-%d %H:%M`) [V both
  cited]. Check `DEFAULT_DATETIME_PATTERN` at implementation time; this
  ledger does not pick a winner.

## 7. Supersession list for ticket 36 (input from this file)

1. Ticket 20's truncated sentence (I1).
2. Ticket 18's value list versus ticket 21's filename-derived
   `CalendarPeriod` (C9).
3. Ticket 25's `winnow` omission in the extractor crate list (D7).
4. Ticket 23's Question versus its later input on `IndexPolicy` date keys
   ([30:587-588](../../issues/30-temporal-boundary-and-dialects.md:587)).
5. Ticket 29's stale "nobody defined the order" sentence in ticket 30, and
   `eom` keyword-versus-function wording (C3, D8).
6. The temporal survey and `reverification.md` predate the translator move
   (B80 vs the survey figures; see
   [30:591](../../issues/30-temporal-boundary-and-dialects.md:591)).
7. Ticket 19's glossary TODOs stay deferred until ticket 30 settles duration
   wording ([19:320-325](../../issues/19-coercion-and-comparison-policy.md:320)).

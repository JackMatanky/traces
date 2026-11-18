# Red-team review of the temporal synthesis

Adversarial review of [synthesis.md](./synthesis.md) for
[ticket 30](../../issues/30-temporal-boundary-and-dialects.md).
This file is research. It does not resolve the ticket.
Baseline revision: `971bdd5729d9d5f51a1bd6fed46bab6947df936a`.

## How to read this file

- Tags: [V] source inspection in this report, not execution or proof of every
  attached claim. [H] inference. [P] proposal for the grilling.
- Ledger ids B, C, P and D come from
  [b-prior-decisions.md](./b-prior-decisions.md).
- Verdicts: HOLDS, WEAKENED (with a repair) or FAILS (with a better option).
- The owner edited `synthesis.md` while I worked. I reviewed the version
  with the modification time 19:10 on 2026-11-16. This report does not assess
  later edits.
- I read code with `read` and `grep`. I did not run builds, tests or probes.

This report records an attack on that snapshot. Its verdicts are historical, not
findings about the repaired synthesis. Later source checks correct some attacks:
[K4](./k-citation-verification.md#k4-cldr-first-day-minimum-days-and-week-rollover)
rejects the claim about `(sunday, 4)`.
[L1-L2](./l-answer-verification.md#l1-b26-is-not-the-truncated-schema-fragment)
correct Q4's schema-rule argument.
[L5](./l-answer-verification.md#l5-membership-bounds-do-not-prove-a-conflicting-physical-key-rule)
corrects Q35's asserted contradiction.
[L8-L12](./l-answer-verification.md#l8-config-merge-is-inherited-not-an-undecided-default)
correct the merge, lazy-zone, ISO-prefix, grammar-scope, and tolerance-release
arguments. These corrections also apply to repeated claims below.

[K22](./k-citation-verification.md#k22-markdown-percent-decoding-and-unicode-text)
supersedes Q25's proposed decode-before-segmentation order.
[m-gap-closure.md](./m-gap-closure.md) supplies later bounded probes. The
unmeasured and unexamined items below describe this report only. They do not
deny that later evidence.

## Part 1: question-by-question attack

### Q1 Is Moment a domain concept

- Recommendation: the Q1 to Q3 cluster. Choose P (strftime plus a template
  Moment adapter) or Q (landed state plus a schema check). Reject R and S. [P]
- Attack 1. Both P and Q leave the owner's tokens `Q`, `Qo`, `ww` and `DDDD`
  failing. The table has 21 entries and none of them is `Q`, `ww` or `DDDD` [V:
  `src/temporal/token.rs:15-37`]. The cluster never decides to add them. Jiff
  has `%q` (format only) and `%j`, but no local-week directive [V:
  `.rust-docs/cache/crates/jiff/0.2.38/source/src/fmt/strtime/mod.rs:171-195`].
- Attack 2. The reject reason for S says the owner's templates break. The
  ticket says Traces does not run those Templater scripts [V: ticket 30,
  "Current code"]. Obsidian continues to run those scripts with real Moment.
  The port cost arises only if the owner moves them to `date_format`.
- Attack 3. Dialect choice uses a `%` sniff for the whole pattern
  [V: `token.rs:55-61`]. A pattern cannot mix both dialects. The cluster does
  not consider an explicit dialect marker.
- Rules at risk: B7 (parity is Free, so owner tokens are taste, not a duty).
- Verdict: WEAKENED. Repair: decide the token table scope. Replace the
  "templates break" reason with the real cost of a port. Consider an explicit
  dialect selector instead of the `%` sniff.

### Q2 Where the translator lives

- Recommendation: P moves it to the template engine. Q keeps it in
  `src/temporal/token.rs`. Both call it a pure syntax adapter. [P]
- Attack 1. The translator does more than convert syntax. It takes a civil
  datetime and writes the day ordinal into the output
  [V: `token.rs:186-189`]. A parse-side adapter cannot use that output,
  because the ordinal needs a known day.
- Attack 2. Moment `ww` needs a week rule. Q31 B makes the week rule
  configurable. Then the translator needs the frame, not only a datetime.
- Attack 3. The ticket lists a third home: a sibling `format` unit shared by
  schema and CLI [V: ticket 30, Alternatives]. The cluster never weighs it.
- Rules at risk: B27 (modules are data and pure functions).
- Verdict: WEAKENED. Repair: state that the translator is a render-side
  helper. Add the sibling-unit alternative. Name a separate parse-side owner.

### Q3 Dialect per surface

- Recommendation: same cluster. Reject R (one Moment dialect). [P]
- Attack 1. The cluster rejects R because R "breaks `%Y-%m-%d` configs".
  If Q5's removal proposal is adopted, no config surface reads a format.
  That rejection reason then no longer applies. Q3 would cover three
  surfaces, not four.
- Attack 2. Under Q, the query `date(text, pattern)` speaks strftime and the
  schema `date.format` speaks Moment. One note-name pattern then needs two
  spellings. The ticket asks for a justified split. The cluster gives none.
- Attack 3. A schema value check under Q must parse Moment text. The only
  translator needs a datetime and cannot parse (see Q2). Q has no code path.
- Rules at risk: B3 and B4 (each dialect change needs a change-table row).
- Verdict: WEAKENED. Repair: drop the config reason for rejecting R. Justify
  the query-versus-schema split, or add a parse-side translator to the plan.

### Q4 The `date.format` check for formats with no full date

- Recommendation: B. Classify each token at schema load. The class selects
  the value check. [P]
- Attack 1. The owner's formats `YYYY`, `Q`, `DDDD` and `ww` name values
  such as `2026`, `2`, `152`. The frozen note parser classifies `2026` as an
  Int [H: B47, classification happens once]. A "string shape" check never
  sees a string. For these formats, B reduces to "accept any scalar", which
  is already the B26 rule. B adds no behavior for the owner's data.
- Attack 2. The synthesis says B matches the precision-preserving shape of
  `DatePoint`. Stored fields do not keep precision. `NoteFieldValue::Date`
  holds a `DateValue` [V: `src/note/field.rs:560`], 4 bytes [V: ticket 18
  sizes]. `DatePoint` keeps precision only in `Calendar`
  [V: `src/temporal/point.rs:43-48`].
- Attack 3. The classification needs a scanner that knows `Q` and `DDDD`.
  Today `Q` and `DDDD` are unsupported tokens [V: `token.rs:15-37`].
- Rules at risk: I1 and C19 (do not adopt a value check from the truncated
  ticket 20 sentence). B adds a value check beyond B26.
- Verdict: WEAKENED. Repair: limit this ticket to a load-time token lint.
  Leave the value check to ticket 36, after it recovers the ticket 20 rule.

### Q5 Fate of `format` under `[frontmatter.date_created]` and `date_modified`

- Recommendation: A. Delete the keys and record a change-table row. [P]
- Attack 1. `deny_unknown_fields` stays [V: B45]. A config file that sets
  `format` then fails to load. The synthesis offers no tolerance period.
  The `RawDateFieldConfig` tests already pin the exact key sets
  [V: `src/config/raw.rs:177-195`].
- Attack 2. The config digest still hashes all four keys
  [V: `src/config/model/digest.rs:348-351,386-389`]. Deleting them changes
  that digest input. The synthesis does not name this.
- Attack 3. The synthesis cites B44 as an "ownership sweep" that removes
  unread keys. B44 removes the keys from the fingerprint, not from the file
  format [V: b-file B44].
- Rules at risk: B4 (Migratable, so a row is enough), B45.
- Verdict: WEAKENED. Repair: accept the keys for one release with a notice.
  Ignore their values during that release. Then delete them. Name the digest
  change. Correct the B44 citation.

### Q6 Which calendar helpers stay in the engine

- Recommendation: A. Keep `Anchor`, `apply_offset` and `weekday` as thin
  adapters that read the frame. [P]
- Attack 1. The helpers stay thin only if temporal takes the week rule.
  `snap_date` and `snap_weekday` hard-code the ISO week and take no rule
  [V: `src/temporal/calendar.rs:35-64`]. Q31 B changes both. `weekday`
  moves with the week rule, as the e-file says [V: e-file q6].
- Attack 2. `Anchor::base` reads `DateValue::today()` and
  `DateTimeValue::now()` [V: `src/template/engine/date.rs:245-255`]. That is
  an ambient clock read. Rewrite the `Anchor` body to use the frame.
- Attack 3. `weekday_from_number` rejects `0` [V: `calendar.rs:115-125`].
  The owner's template calls `day(0)` for Sunday [V: ticket 30]. The helper
  cannot carry the owner's numbering.
- Rules at risk: B39 (temporal owns week rules).
- Verdict: WEAKENED. Repair: keep the adapters in the engine. Move every
  week-rule read, including `weekday`, into temporal behind the frame.

### Q7 Where the filename-period parser lives

- Recommendation: A. In temporal, next to the date lexers. [P]
- Attack 1. The synthesis says the parser "shares token shapes" with the
  lexers. The Logos enums `DatePrefixToken` and `TemporalShapeToken` also
  serve the frozen note parser [V: d-crate Q5; `src/temporal/lexer.rs:93-111`].
  A week or quarter token added there changes frozen parsing (B1, B37).
  The parser needs its own token enum.
- Attack 2. Owner data. The owner's week template writes `YYYY-[W]ww`
  [V: `docs/vault/07_templates/12_01_week_periodic.md:85`]. Under Moment's `en`
  locale, the week of Sunday 2025-12-28 is week 1 of week-year 2026 and prints
  `2025-W01` [H: Moment locale `dow` 0, `doy` 6]. The ISO prefix rule reads
  `2025-W01` as 2024-12-30. Two weeks then share one name and one meaning is
  wrong by a year.
- Attack 3. The synthesis defines no prefix grammar. It lacks separators,
  case, `W1` versus `W01` and quarter spelling. See Part 2, check 6.
- Rules at risk: B1, B34, B37.
- Verdict: WEAKENED. Repair: use a separate token enum. Add a grammar
  question. List the owner's `YYYY-[W]ww` labels as a counterexample.

### Q8 Week-one rule for a custom week start

- Recommendation: inherited from Q31. The rule lives in temporal. [P]
- Attack 1. Jiff has no custom week-year. It has `ISOWeekDate` and the
  week-0 directives `%U` and `%W` [V: jiff strtime table lines 193-195;
  `calendar.rs:41-48` uses `iso_week_date`]. The pair needs new arithmetic
  and new tests. Q12 lists a "blocking limitation for an accepted feature"
  as its reject condition. Q31 B triggers that condition.
- Attack 2. The prefix `2026-W01` means ISO in B34. Under a custom pair, it
  changes meaning when config changes [V: ticket 21 input, [H] there].
  A global config can then reinterpret a vault's committed week notes.
- Rules at risk: B34, B46.
- Verdict: WEAKENED. Repair: ship named presets (`iso`, `us`). State that
  the prefix obeys the pinned rule. State which config level may set the rule.

### Q9 `speedate`

- Recommendation: B. Keep speedate until the diagnostic parser lands. End
  state A. [P]
- Attack 1. The synthesis says removal early changes reason strings. The
  call site maps `OutOfRangeMonth` and `OutOfRangeDay` to one reason and all
  else to another [V: d-crate Q3, `date.rs:350-374`]. A digit pre-check can
  keep that mapping [H]. Negative years already use `civil::Date::new`
  [V: d-crate Q3, `date.rs:385-396`].
- Attack 2. The d-crate source read shows that Jiff accepts a strict superset
  of the speedate shape [V: d-crate Q3]. This review did not run the
  nine-scenario probe.
- Rules at risk: B1 (frozen parsing), B61 (one rebuild).
- Verdict: WEAKENED. Repair: base B on the missing parity probe, not on reason
  strings. Schedule the probe.

### Q10 `winnow`

- Recommendation: B. Keep winnow as the boundary scanner. [P]
- Attack 1. If winnow stays, ticket 25's crate list must add it (D7).
  `winnow` also arrives through the `toml` family [V: d-crate Q4]. A `toml`
  bump would then change the extractor fingerprint and force a rebuild [H].
  The synthesis names neither the list change nor this cost.
- Attack 2. `duration_part` is 10 lines [V: `lexer.rs:139-148`]. A hand
  scanner removes the fingerprint coupling.
- Attack 3. The synthesis cites jiff `parser.rs:134-194`. The d-crate file
  gives 183-194 and 311-348 [V: d-crate Q2].
- Rules at risk: B57, D7.
- Verdict: WEAKENED. Repair: add the ticket 25 list contract. Compare the
  rebuild-coupling cost with the cost of the 10-line scanner.

### Q11 `logos`

- Recommendation: keep. [P]
- Attack: a removal rewrites four lexers with no defect reported [V: d-crate
  Q5]. I found no counterexample.
- Verdict: HOLDS.

### Q12 `jiff`

- Recommendation: A. Keep jiff. [P]
- Attack: jiff has no custom week-year (see Q8). That is a gap, not a reason
  to switch. `chrono` has the same gap in its default API [H].
- The synthesis marks the crate claims "pending". The d-crate file now
  confirms `default-features = false` with `std`, `tz-system` and
  `tzdb-bundle-always` [V: d-crate, jiff section]. The owner note guessed
  default features [H there].
- Verdict: HOLDS. Repair: remove the stale "pending" marker.

### Q13 The grammar

- Recommendation: A. Closed English grammar. [P]
- Attack: the closed list itself is not drafted. The Answer needs a table.
  See Part 2, check 6 for the interaction with quoted literals.
- Verdict: HOLDS (closed versus open). The scope gaps go to Part 2.

### Q14 Build or use a crate

- Recommendation: A. Build in the repository. [P]
- Attack: the d-crate source reads support A. No inspected candidate yields
  a period or reports spans. `jiff-english` can panic on non-ASCII identifiers
  [V: d-crate T6]. The ticket asks for the size and test cost of an in-repo
  grammar and for binary size. This review has no measurements for either
  [V: d-crate "Not checked"]. The ticket also lists "defer to fog" as an
  alternative. Q14 omits it.
- Verdict: HOLDS. Repair: add the defer alternative. Mark the costs unmeasured.

### Q15 Bare weekday

- Recommendation: A. The next occurrence strictly after `today`. Add
  `next` and `last` spellings. [P]
- Attack 1. On a Friday, `due == friday` means next Friday. A user who wants
  today gets no rows and no error. `fuzzydate` and `parse_datetime` return
  today [V: d-crate table].
- Attack 2. The template `date.weekday(1)` on Sunday 2026-11-15 gives Monday
  2026-11-09 [V: `calendar.rs:57-64`]. A bare `monday` under A gives
  2026-11-16. The helper and the literal give results seven days apart.
- Attack 3. The c-digest leans to the current-week day [V: c-digest Q15]. The
  synthesis says "no evidence picks one" and does not record that lean.
- Verdict: WEAKENED. Repair: record the c-digest lean. Add `this <weekday>`.
  State the same-day rule.

### Q16 Interval precision

- Recommendation: A. Periods for period phrases, points for point phrases. [P]
- Attack: the rule says which phrases are periods. It does not classify
  `in 2 weeks` or `3 months ago`. From 2027-01-30, `in 1 month` clamps to
  2027-02-28 as a point. `next month` starts on 2027-02-01 as a period.
  Relative-offset phrases and calendar-period phrases need separate rules.
- Verdict: WEAKENED. Repair: classify every offset phrase as point or period.

### Q17 Relative keyword spelling

- Recommendation: A. Nullary calls `today()` and `eom()`. [P]
- Attack: the grammar requires exactly one argument for `sow`, `eom` and the
  other edge functions [V: `src/query/grammar/filter.rs:591-597`]. `eom()`
  adds an arity overload. A smaller option exists: add only `today()` and
  `now()`. Then `eom(today())` composes and no function gains a second
  meaning.
- Attack: a quoted literal `"today"` is also a late-typed candidate (B19).
  Two spellings then name one keyword.
- Verdict: WEAKENED. Repair: consider nullary `today()` and `now()` only.

### Q18 `member_of` spelling

- Recommendation: A or B, the owner picks. Reject `in`. [P]
- Attack: ticket 29 defines `member_of` as point-in-period membership
  [V: ticket 29, section 10]. The synthesis pairs the spelling with a
  "scalar-versus-list mismatch" notice from Fileclass. Ticket 29 names the
  bracket-mismatch advisory instead.
- Attack: under B18 and B32, `point == period` already means membership.
  The Answer must say when `member_of` adds anything.
- Verdict: WEAKENED. Repair: drop the list-mismatch notice. State the use
  case that needs a separate spelling.

### Q19 Type shape for civil and instant values

- Recommendation: A. Keep the outer two-variant `DatePoint`. [P]
- Attack: no civil datetime variant exists today. `DateTimeValue` wraps a
  `Timestamp` [V: `src/temporal/datetime.rs:76-78`]. `DatePoint::parse`
  resolves a naive datetime through the process zone
  [V: `point.rs:34-39,113`]. The owner's frontmatter holds naive values such
  as `date_created: 2023-06-12T08:14` [V: `docs/vault/05_metadata/cal.md:9`].
  The current payloads of the two variants (`Calendar`, `DateTime`) cannot
  retain Date, civil datetime, and instant as distinct kinds. Option A does
  not answer the question without a payload change.
- Attack: the ticket asks about `NoteFieldValue` at 80 bytes. The synthesis
  discusses only `DatePoint`. Its payload discussion does not establish
  compliance with the 80-byte cap. The largest current payload is 72 bytes
  [V: c-digest Q19, ticket 18].
- Attack: a new variant bumps the format epoch (B58). The synthesis omits it.
  It cites a "27-probe" for the DST panic. The probe sits in ticket 29.
- Rules at risk: B16, B58, B24 (parse-time classification, no ambient zone).
- Verdict: FAILS. Better: decide the field-level shape directly. Compare:
  (i) an inner `Civil`/`Instant` enum in `DateTimeValue` without a derived
  `Ord`; (ii) a `NoteFieldValue` civil variant; (iii) a third `DatePoint`
  variant. Measure sizes. Add the epoch bump.

### Q20 `CalendarPeriod` placement

- Recommendation: A. Temporal value, no `NoteFieldValue` variant. [P]
- Attack 1. Query evaluation passes `NoteFieldValue` everywhere. Function
  results are `Option<NoteFieldValue>`. `CompareOp::is_satisfied_by` takes a
  `&NoteFieldValue` literal [V: `filter.rs:359-433,783-809`]. A period
  literal, `file.date` and a group key must travel through that path. A needs
  a second value carrier or a hidden variant.
- Attack 2. The `DatePoint::snap` evidence uses the old kind-only
  `CalendarPeriod` [V: `calendar.rs:23-30`]. Ticket 21 renames that type to
  `CalendarUnit`. The evidence does not support the new type.
- Rules at risk: B14 and B15 (one value module holds the period bracket).
- Verdict: WEAKENED. Repair: choose the carrier. Compare a `QueryValue` wrapper
  with a `NoteFieldValue::Period` variant (about 8 bytes [H]).

### Q21 Date versus instant at an ambiguous or skipped midnight

- Recommendation: A for mapping (civil midnight to instant), C for failure
  (Null plus notice). [P]
- Attack 1. A and C conflict. A always yields an instant. C returns Null when
  the instant is not unique.
- Attack 2. The forward gap mapping is not monotone. In `America/New_York`
  on 2026-03-08, civil 02:30 maps to 03:30 EDT (07:30Z). Civil 03:00 maps to
  07:00Z. Civil order says 02:30 < 03:00. Mapped order says 07:30Z > 07:00Z
  [V: B67 cycle; d-crate `Compatible` row]. A recreates the fabricated order
  that the synthesis rejects.
- Attack 3. The question scope is midnight. The accepted cycle uses 02:30.
  The rule must cover civil datetimes in a gap.
- Attack 4. The code cite `field.rs:310` is wrong. The Less and Greater
  fallbacks are at `field.rs:297-300` [V].
- Better option D: project the instant to a civil value in the pinned zone and
  compare civil values. That projection is total. No gap or fold ambiguity
  exists. Example: `America/Sao_Paulo`, 2015-10-18 has no 00:00. No instant
  has civil 00:00, so `Date 2015-10-18 == instant` never holds, with no
  Null and no notice. Period membership already uses this direction (B32).
- Rules at risk: B17, B30, B67.
- Verdict: FAILS. Better: option D. Keep the zone failure typed (B30).

### Q22 Total sort key for mixed temporal values

- Recommendation: A. Order equal-start periods by unit quantity, then by
  written precision. [P]
- Attack 1. A period key is `(start, unit)` [V: ticket 29 section on group
  keys]. A period has no written precision. The second clause never runs.
- Attack 2. "Unit quantity (year, quarter, month, week, day)" names no
  direction. Week against month has no quantity without an anchor.
- Attack 3. The sort comparison needs no tie-break. Group keys need a fixed
  order. Hash order would make group output change between runs.
- Verdict: WEAKENED. Repair: fix an explicit unit rank
  (Day, Week, Month, Quarter, Year) [V: e-file q22]. Drop the precision clause.

### Q23 Nominal duration equality

- Recommendation: A. Keep nominal equality and order. [P]
- Attack 1. The synthesis writes "1 month = 30 days = 365.25 days / 1 year".
  The code says one year is 365 days [V: `src/temporal/duration.rs:11-13`;
  `unit.rs:210` gives 31,536,000 seconds]. The synthesis misquotes the rule.
- Attack 2. `52w 1d` and `1y` are equal [V: `duration.rs` test case at line
  1846]. A calendar month has 28 to 31 days. A fixed 30-day month matches
  only some months [V: e-file q23].
- Attack 3. "Queries rely on that identity" has no cited evidence. The e-file
  counterexample is not answered.
- Rules at risk: B23 asks this ticket to refine or re-affirm with a
  counterexample.
- Verdict: WEAKENED. Repair: correct the number. Answer the counterexample.
  Choose retained equality or class-aware equality in the Answer.

### Q24 Documents for calendar arithmetic

- Recommendation: describe arithmetic at the behavior level and point to
  ticket 21. [P]
- Attack: a glossary that points at a scratch ticket breaks when the tickets
  move. Ticket 19 says to leave the glossaries alone until the specification
  exists [V: ticket 19, TODOs]. The *Duration* wording waits on Q23.
- Verdict: WEAKENED. Repair: put the rule text in the glossary at cutover.
  Tie the duration wording to the Q23 outcome.

### Q25 `date(link)` and a Markdown target

- Recommendation: A. Read the raw target text through the shared date
  parse. [P]
- Attack 1. Ticket 19 says a Markdown target keeps its extension and may be
  percent-encoded. External URLs are not note targets [V: ticket 19 lines
  299-313]. The synthesis addresses none of the three points. Example:
  `2026-01%20notes.md` and `https://x/2026-01-01`.
- Attack 2. Option B (read the note's date field) is not live. B22 fixes the
  rule: read only the last segment of the target.
- Attack 3. The synthesis cites a "closed pipe of ticket 33". Ticket 33 is
  open. It cannot bind this ticket.
- Rules at risk: B22, ticket 19 Markdown rule.
- Verdict: FAILS. Better: decode once, take the last segment, keep the
  extension, parse a prefix or the given pattern. Non-note targets yield Null
  [P: e-file q25].

### Q26 The `[date]` table syntax

- Recommendation: A. One flat table. [P]
- Attack 1. Q27 and Q32 name `filename.year_prefix`. In TOML a dotted key
  inside `[date]` makes a nested table. That is option B, not A.
- Attack 2. Q26 counts "six keys" with two diagnostic keys. Q27 says there
  are four fields and no diagnostic fields. The seventh-key reject condition
  has no meaning against that count.
- Attack 3. B46 requires this ticket to state the merge rule and the trust
  gate for `[date]`. The synthesis gives neither.
- Verdict: WEAKENED. Repair: use a flat key such as `filename_year_prefix`.
  Fix the key count. State the merge rule.

### Q27 Fields of `DateConfig`

- Recommendation: exactly four fields. [P]
- Attack: the filename parser runs during query evaluation (B35, B51). It
  reads `filename.year_prefix`. The Q33 frame lacks that field. The frame
  must carry the policy. See Q33.
- Verdict: WEAKENED. Repair: list the frame as a consumer of every field.

### Q28 The temporal-owned policy

- Recommendation: B. A new `TemporalPolicy`. [P]
- Attack: B60 does not name any field. The synthesis says "the fields B60
  names". The e-file calls the type `DatePolicy`. The names differ across
  files. A field-for-field copy from `DateConfig` has few fields. The zone
  type differs (text versus loaded zone). This review uses that difference
  to argue for two types. L9 later corrects the loaded-zone premise.
- Verdict: HOLDS. Repair: fix the B60 citation. Pick one type name.

### Q29 Zone syntax and validation

- Recommendation: A. IANA name only, validated at config load. [P]
- Attack 1. A fixed offset has no clean IANA spelling. `Etc/GMT-2` means
  UTC+02:00. The sign is reversed. The repo already uses it in a test
  [V: `src/note/field.rs:1041`, comment "UTC+02:00"]. The e-file recommends
  an explicit `±HH:MM` form [V: e-file q29]. The synthesis says no evidence
  exists. It does not cite the e-file.
- Attack 2. Zone acquisition at config parse makes every command fail on a
  typo, including commands that need no zone. B28 says to acquire a zone
  only when needed [H: tension].
- Attack 3. Legacy names such as `EST` and `EST5EDT` pass a tzdb lookup
  [H]. "Every config value is a named zone" is true but weak.
- Verdict: WEAKENED. Repair: accept `±HH:MM` or document the `Etc/GMT` trap.
  Keep the typed failure. Decide whether a bad name blocks zone-free commands.

### Q30 System zone acquisition

- Recommendation: A. The frame resolves lazily and memoizes. [P]
- Attack: the jiff cache still serves a zone for up to five minutes across
  frames [V: owner-evidence, `mod.rs:69,123-133`]. A fixes drift inside one
  operation, not staleness across operations. The synthesis says the
  concurrency lens "measured" the hazard. The owner note is a source read.
  This review contains no observation of two zones in one operation.
- Verdict: HOLDS. Repair: replace "measured" with "read in source".

### Q31 Is the week start configurable

- Recommendation: B. A pair `(first_day, min_days)`, ISO default. [P]
- Attack 1. The pair allows `(sunday, 4)`. No major locale uses it [V: e-file
  q31]. Named presets avoid nonsense pairs.
- Attack 2. Jiff lacks custom week-year arithmetic (see Q8). The cost is new
  code with boundary tests. The synthesis shows B as cheap.
- Attack 3. The owner lists days from `day(0)`, Sunday [V: ticket 30]. The
  stated reject condition ("owner confirms no use") is already contradicted
  by that evidence. The default `(monday, 4)` may not match the owner's use.
- Attack 4. A global `week_first_day` merges field by field into every vault
  (B46). A vault's ISO week labels then change meaning. That is the nldates
  drift the recommendation claims to avoid.
- Verdict: WEAKENED. Repair: presets, a stated merge level, and a cost note.

### Q32 Opt-in key for bare-year prefixes

- Recommendation: A. One global boolean. [P]
- Attack 1. The synthesis cites B74 to B77 for "no per-directory config
  channel". Those ids cover `RelativePath` and comparison keys, not config
  [V: b-file §2.11]. The citation does not support the claim.
- Attack 2. The opt-in matches `2019 budgets.md` and `1998 census.md` as
  years. A narrower shape exists: match only a stem that equals four digits,
  as in `2026.md`. The synthesis never considers it.
- Attack 3. The key name `filename.year_prefix` conflicts with Q26 A.
- Verdict: WEAKENED. Repair: compare whole-stem-only against prefix matching.
  Fix the citation and the key spelling.

### Q33 What temporal exports to the frame

- Recommendation: B. Clock, week values and a lazy zone cell. [P]
- Attack 1. The filename parser needs `filename.year_prefix` during
  evaluation (B35, B51). The shape has no field for it. The e-file frame
  carries the policy [V: e-file q33].
- Attack 2. `OnceCell` is not `Sync`. A frame shared by an LSP request or a
  lazy `QuerySet` (B41) may cross threads. `OnceLock` fits [H].
- Attack 3. `today()` (Q17) needs the zone when the planner binds that call.
  Queries that use it then acquire the zone during binding. B stays correct.
  The "only when needed" benefit is smaller for those queries.
- Attack 4. "65 crate call sites": I count 133 text hits of `local_zone(`
  in `src/`, tests included. The synthesis count is unverified [H].
- Verdict: WEAKENED. Repair: add the policy. Use `OnceLock`. Recount.

### Q34 Replacements for ambient reads

- Recommendation: pass the frame everywhere. Replace `now_instant()`. [P]
- Attack: the list is partial. It omits `filter.rs:637,643,670,696,758`,
  `datetime.rs:136,202,358` and `point.rs:840` [V: grep]. It omits the clock
  reads in `Anchor::base` [V: `engine/date.rs:245-255`]. The list is an
  implementation gate, so the gap is harmless to the decision.
- Verdict: HOLDS. Repair: regenerate the list at implementation time.

### Q35 Probe bounds per operator and value kind

- Recommendation: A. Prove Exact bounds now for civil dates, instants and
  periods. [P]
- Attack 1. The cited baselines measure numeric `f64` keys and a folder key
  [V: `29-prep/g1-probes.md:46-58`]. No temporal key was measured. Ticket 24
  keeps temporal values as text (B52). No temporal key encoding exists.
- Attack 2. `field.rs:538-546` is an `i64` to `f64` cast for numbers. It is
  not a temporal proof case.
- Attack 3. Civil-to-instant mapping is not monotone in a gap (see Q21).
  A closed bound on that mapping is a superset, not Exact (R5).
- Attack 4. B70 says "point-period matching uses membership bounds". B66 and
  B68 say membership never defines a range key. The synthesis does not settle
  that tension.
- Attack 5. B71 and B72 limit the first scope to unquoted literals. Quoted
  literals and `file.date` stay Unsupported.
- Verdict: WEAKENED. Repair: prove only zone-free kinds now. Keep every
  zone-dependent mapping Unsupported until a probe exists.

### Q36 Notices for `Null` dates

- Recommendation: A. Notice only on outcomes that silently coerce. [P]
- Attack 1. "`Null` compared" is the most common case. Every note without a
  `due` field is compared as `Null`. A therefore equals B for that case.
- Attack 2. The new Null source from this ticket is the filename parser.
  `2026-13-40-notes.md` gives a `Null` `file.date`. Neither alternative
  covers a malformed date-shaped prefix [V: e-file q36].
- Attack 3. The owner's data holds a malformed value,
  `date_modified: 2023-09-2111T051:31`
  [V: `docs/vault/05_metadata/_metadata_values/job_titles.md`]. It is a
  string under the frozen parser [H].
- Verdict: WEAKENED. Repair: adopt the e-file split. No notice for an absent
  value. One notice for a malformed date-shaped value. A typed error for a
  zone failure.

## Part 2: cross-cutting checks

### Check 1: contradictions between recommendations

- Q26 (flat table) against Q27 and Q32 (`filename.year_prefix`).
- Q26 (six keys, two diagnostic) against Q27 (four fields, no diagnostic).
- Q3 (reject R because configs break) against Q5 (delete the config keys).
- Q4 (classify `Q` and `DDDD`) against Q1 and Q2 (the table rejects them).
- Q2 ("pure adapter") against Q31 B (week tokens need a week rule).
- Q21 A (always maps) against Q21 C (Null when ambiguous).
- Q8 ("not in config") against Q27 (the pair is in config). The two agree if
  config stores policy values and temporal owns the algorithm. The text
  does not make that distinction.
- Q33 (frame fields) against Q27 (the filename flag must reach evaluation).

### Check 2: contradictions with accepted Answers

- Q19 A leaves B16 without a place for civil datetimes. It omits B58.
- Q25 ignores the ticket 19 Markdown rule and fixes none of its three points.
- Q4 B adds a value check beyond B26 (I1, C19).
- Q5 A breaks configs under B45 without a tolerance period.
- Q21 C may breach B30 if "calendar resolution" covers a fold. [H]
- Q35 A leaves the B70 against B66 and B68 tension open.

### Check 3: claims that the cited evidence does not support

1. Q4 says B matches the precision-preserving shape of `DatePoint`. Stored
   fields keep no precision [V: `src/note/field.rs:560`].
2. Q5 cites B44 as a sweep of unread config keys. B44 covers the fingerprint
   only [V: b-file B44].
3. Q10 cites jiff `parser.rs:134-194`. The d-crate file gives 183-194 and
   311-348 [V: d-crate Q2].
4. Q21 cites `field.rs:310`. The lines are 297-300 [V].
5. Q23 gives 365.25 days per year. The code uses 365 [V].
6. Q23 says "queries rely" on the identity. No source shows it.
7. Q28 says B60 names fields. B60 names none [V: b-file B60].
8. Q32 cites B74 to B77 for config channels. Those ids are about paths [V].
9. Q35 uses numeric and folder timings as date-key baselines [V: g1-probes].
10. Q35 lists `field.rs:538-546` as a temporal case. It is a number cast [V].
11. Q18 imports a list-mismatch notice. Ticket 29 defines point-period
    membership and the bracket advisory [V: ticket 29 section 10].
12. Q25 cites a closed pipe from ticket 33, which is still open.
13. Q20 uses `DatePoint::snap` as evidence. It consumes the old unit-only
    type [V: `calendar.rs:23-30`].
14. Q19 cites a "27-probe". The 15-of-200 probe is in ticket 29 [V].
15. Q33 and Q30 say the hazard was "measured". The owner note is a source read.
16. Q33 says 65 call sites. I count 133 text hits, tests included [V].
17. Q2 cites P6 for "one entry point". P6 lists what ticket 31 needs.
18. Q6 cites B7 for "Templater parity is a template concern". B7 says
    parity is Free. It supports no placement.
19. Q1 reject S says five owner templates need migration. The templates are
    Templater scripts that Traces does not run [V: ticket 30].

### Check 4: recommendations without measurements in this report

- Q12 and Q14: binary size and in-repo grammar cost. Unmeasured.
- Q19 and Q20: size estimates (32 and 16 bytes) are [H]. Sizes for
  `civil::DateTime` and a period have no measurements in this report.
- Q35: this report contains no temporal key timing.
- Q9: this review did not run the speedate and Jiff differential probe.
- Q10: this report contains no rebuild-cost measurement for a `winnow` bump.
- Q30 and Q33: this report contains no observation of a two-zone operation.
- Q8 and Q31: this report supplies no custom-pair week-year test corpus.

### Check 5: frozen vault-format parsing (ticket 16)

- Q7: new week and quarter tokens must not enter `DatePrefixToken` or
  `TemporalShapeToken`, because the note parser uses them.
- Q19: a naive datetime changes from an instant (host zone) to a civil value.
  B16 authorizes the change. Ticket 36 must record it as a frozen-parse
  semantic change.
- Q9 and Q10: both crates sit on the frozen date and duration paths.
- Q4: the frozen parser classifies `2026`, `2` and `152` as numbers.
- Q13: grammar phrases must not classify note values. Only query and template
  literals use late typing (B47).
- Q25: `parse_with_pattern` is shared. A pattern change must not alter note
  parsing.

### Check 6: questions the synthesis misses or merges wrongly

- The filename-prefix grammar has no question. Q7 covers placement only.
  The ticket checks need invalid weeks, quarter boundaries and malformed
  longer prefixes. A grammar section is missing.
- B46 asks for the `[date]` merge rule and the trust gate. No section does.
- The ticket lists two alternatives that no section weighs: a sibling
  `format` unit, and deferring the natural-language grammar to fog.
- The synthesis promises a downstream contract in every section. None
  appears in the inspected snapshot. Tickets 31, 34, 35, and 36 need a table
  with each invariant's owner, error types, zone-failure type, notice
  condition, and crate-list changes.
- The natural-language grammar has no scope question. It must say which
  phrases are date-shaped for the B21 advisory. A typo such as `"next weke"`
  runs as a string without a notice.
- The grammar table for Q13 is not drafted.
- Q1 to Q3 merge into one cluster. Q2 is nearly absent from it.

### Check 7: designs the synthesis did not consider

1. Q21: project the instant to civil in the pinned zone (option D above).
2. Q17: only nullary `today()` and `now()`. Compose with `eom(today())`.
3. Q20: a query-layer `QueryValue` wrapper that adds a period case.
4. Q32: whole-stem-only year opt-in (`2026.md` but not `2019 budgets.md`).
5. Q31: named week presets instead of a free `(first_day, min_days)` pair.
6. Q1 to Q3: an explicit dialect marker in the pattern, in place of the `%`
   sniff. Example: a `moment:` prefix.
7. Q5: tolerate the old keys for one release, with a notice.
8. Q29: accept `±HH:MM` as a second zone spelling.
9. Q19: a `NoteFieldValue` civil variant next to an instant variant.
10. Q10: the 10-line scanner with the rebuild-coupling price stated.

## Part 3: the 10 weakest points to settle in the grilling

1. Q19. The current `DatePoint` payloads cannot retain Date, civil datetime,
   and instant as distinct kinds. Decide the field-level shape. Measure the
   sizes. Record the epoch bump.
2. Q21. A and C contradict. A maps civil to instant, which is not monotone
   in a gap. Make a test of option D (instant to civil).
3. Q25. The ticket 19 Markdown rule has no answer: percent-encoding,
   extension and external URLs.
4. Q1 to Q3. Decide the token table. Fix the reject reasons. Decide the
   parse-side translator and the dialect marker.
5. Q31 and Q8. Jiff lacks custom week-year code. The owner may use Sunday
   weeks. Decide presets, the merge level and the prefix meaning.
6. Q20. A period has no carrier in the `NoteFieldValue` query pipeline.
   Choose a wrapper or a variant.
7. Q26, Q27, Q32 and Q33. Fix the key shape and count. State the merge rule.
   Put the filename flag into the frame. Use `OnceLock`.
8. Q35. This review supplies no temporal key encoding or timing.
   Civil-to-instant gap mapping is not monotone. Clarify B70 against B66 and
   B68, subject to L5's later correction.
9. Q23. The synthesis misquotes the year length and ignores the
   `1mo == 30d` counterexample. Decide with the real rule in view.
10. Q15 to Q17. The weekday meaning, the offset-phrase classification and the
    keyword spelling interact with adaptive quoted literals. Draft the grammar
    table before any single choice.

## Not checked in this report

- I ran no build, test, probe or benchmark. I used `read` and `grep`, not
  CodeGraph, for code claims.
- I did not read the lens reports (Layout H1 to H3, Deletion D1 to D5,
  Scaling H1 to H5, Upstream H4 and H5, Invariants). They live in agent
  transcripts. I could not check the claims that cite them.
- I took ticket 18, 20, 22, 23, 24, 25 and 32 Answers from the b-file
  ledger. I read tickets 19, 21, 29 and 30 directly.
- Moment `en` locale values (`dow` 0, `doy` 6) and the `ww` output for
  2025-12-28 come from my memory of Moment, not from a source read [H].
- I did not check the Havana and Apia tzdb rules. I used the d-crate and
  e-file text.
- I did not check whether jiff's strptime accepts `%-m` and `%-d` flags in
  parse mode. I only confirmed that `%3f` parses.
- I did not measure the size of `civil::DateTime`, `Timestamp` or a period.
- I did not check the e-file web sources S1 to S23 or the digest line numbers
  in the c-digest.
- I counted `local_zone(` hits by text search. I did not separate test code
  from production code.
- I did not check ticket 22 on whether diagnostic fields belong to the parser,
  as Q27 claims.
- The owner may have edited `synthesis.md` after 19:10. I did not re-read it.

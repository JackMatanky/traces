# Temporal synthesis: evidence, alternatives, recommendations

Integration-owner synthesis for
[Where do temporal, format dialects and calendar helpers belong?](../../issues/30-temporal-boundary-and-dialects.md).
Baseline revision: `971bdd5729d9d5f51a1bd6fed46bab6947df936a`.
This file prepares the grilling. It does not resolve the ticket.
The source descriptions refer to the recorded baseline and later named
checks. They are not a fresh audit of the current working tree.

[h-redteam.md](./h-redteam.md) attacked the first draft. This version repairs
the findings that the owner checked against source.

## Evidence index

| File | Content | Status |
| --- | --- | --- |
| [b-prior-decisions.md](./b-prior-decisions.md) | Ledger B1 to B83 from tickets 01 to 32, proposals P1 to P6, contradictions C1 to C19, question map | Complete |
| [c-digest-lessons.md](./c-digest-lessons.md) | Lessons from `docs/digests/` (Templater, Dataview, nldates, IWE, Tasks, Fileclass, zk) | Complete |
| [d-crate-evidence.md](./d-crate-evidence.md) | jiff, speedate, winnow, logos, parse_datetime, jiff-english, fuzzydate, chrono-english | Complete |
| [e-external-designs.md](./e-external-designs.md) | CLDR, Temporal, java.time, PostgreSQL, Moment, Luxon, Tasks, cron and other designs | Complete |
| [h-redteam.md](./h-redteam.md) | Adversarial review of the first draft of this file | Complete |
| [i-leakage-review.md](./i-leakage-review.md) | Reverse module edges, error leakage at boundaries, carrier conflicts, the public Jiff exposure | Complete |
| [j-test-coverage-review.md](./j-test-coverage-review.md) | Assertion-backed coverage map, five ranked test gaps, six acceptance matrices | Complete |
| [k-citation-verification.md](./k-citation-verification.md) | Claim audit of c and e: corrections K1 to K23, unexamined list | Complete |
| [l-answer-verification.md](./l-answer-verification.md) | Direct read of 12 Answers, 15 corrections, invariants, remaining human choices | Complete |
| [owner-evidence.md](./owner-evidence.md) | Owner checks: jiff system-zone cache, `DatePoint` shape | Complete |
| [m-gap-closure.md](./m-gap-closure.md) | Recorded probes, measurements, reproduction commands and limits | Complete |
| [n-parity-and-scope.md](./n-parity-and-scope.md) | Only the unsigned full-date branch calls speedate; separate carrier/frame acceptance scope | Complete |
| [z-grilling-guide.md](./z-grilling-guide.md) | Session plan and clusters | Complete |
| `lens-receipts.jsonl` | Scout delivery ledger | Complete |

Lens reports live in agent transcripts. Ten lenses were planned. Six
delivered: Upstream, Invariants, Concurrency, Scaling, Deletion and Layout.
Four failed on provider route errors: Allocation, Seams, Leakage and Tests.
The owner covered the Allocation and Seams scope by direct inspection.
The gap-closing round covered Leakage and Tests as i-leakage-review.md and
j-test-coverage-review.md.

The owner also ran two probes. The parity probe checked 4,635,370 inputs
through the speedate path and a guarded jiff path with no difference
(Q9). The Moment probe printed the owner's week tokens under the `en`
locale (Q7). Sizes: `civil::Date` 4 bytes, `civil::DateTime` 12,
`Timestamp` 16, `Span` 64, a two-case payload enum 24 (Q19). The probe
outputs appear in [m-gap-closure.md](./m-gap-closure.md). The parity probe
is preserved at [probes/parity](./probes/parity). Its recorded run
reproduces the earlier counts. The layout test measures a mirror of the
proposed value type, not the production type.

The 2026-11-18 closure executes all four natural-language candidates at
Rust 1.96.0. The M report's
[acceptance closure](./m-gap-closure.md#four-candidate-acceptance-closure)
holds 298 JSON records, first/warmed allocation tables and instrumented
binary sizes. It also records the current broad verification pass.
Earlier abort-profile probes remain bounded historical observations.
The N report separates narrow speedate removal from the carrier/frame
redesign. Neither report accepts a proposal or resolves this ticket.

Evidence tags:

- [V] verified source or observed result.
- [H] hypothesis that needs evidence.
- [P] proposal for human approval.
- [A] the topic is absent from every source read.

## Reading guide

Each section lists inherited rules, the live alternatives, the
recommendation with its reason, and a reject condition. Decision evidence
comes before implementation gates. An implementation gate never decides a
question. Recommendations are [P]. The grilling decides.

Ledger ids B, C, D, I and P come from b-prior-decisions.md.

## Corrections made after the red-team review

The owner checked each red-team finding against source. These claims of the
first draft were wrong and are repaired below:

- Q19 said the outer `DatePoint` enum already holds civil and instant
  values. It does not. `DateTimeValue` wraps a `Timestamp`, and naive input
  resolves through the host zone today (`src/temporal/datetime.rs:76-78`).
- Q21 option A mapped a civil midnight to an instant. That mapping is not
  monotone in a gap (B67). Option D below replaces it.
- Q25 ignored ticket 19's Markdown rules for link targets. It is repaired
  below.
- Q23 gave 365.25 days per year. The code and the glossary use 365 days
  (`src/temporal/duration.rs:11-13`).
- Q4, Q5, Q28 and Q32 cited ledger ids that do not support their claims.
- The 65 call sites of `local_zone()` is a CodeGraph caller count. A text
  search finds more hits when tests count.
- Q1 to Q3 treated the translator as pure in both directions. It is a
  render-side helper. It takes a civil datetime for the `Do` ordinal
  (`src/temporal/token.rs:186-189`).

## Cluster A: dialects and formats (Q1 to Q5)

### Q1 to Q3: Moment, the translator, the four surfaces

Facts.

- Ticket 16 makes Moment parity Free (B7). The owner's tokens are a design
  input, not a duty.
- The owner's templates use `ww`, `YY`, `Q`, `Qo`. The owner's Fileclass data
  uses `YYYY`, `DDDD`, `Q` [V: C1 evidence list]. The table in `token.rs`
  has 21 entries plus `Do`. It has no `Q`, `Qo`, `ww` or `DDDD`
  [V: `src/temporal/token.rs:15-37`].
- Traces does not run the owner's Templater scripts [V: ticket 30 Current
  code]. They run in Obsidian's own Moment. Unsupported Traces tokens
  show a portability limit, not a demonstrated failure of those scripts.
- Today the surfaces use these dialects [V: C2, e-file q3]:
  - Config `format`: strftime. No production code reads it.
  - Query `date(text, pattern)`: strftime only.
  - Template `date_format`: Moment, through the translator.
  - Schema `date.format`: an unvalidated hint. Its tests use Moment tokens.
- The dialect choice is a `%` sniff over the whole pattern
  (`token.rs:55-61`). One pattern cannot mix both dialects.
- The translator renders. It takes a civil datetime and bakes the `Do`
  ordinal into its output (`token.rs:186-189`). It cannot turn Moment text
  into a parse pattern.
- Moment week tokens (`w`, `ww`, `gggg`) take their week rules from the
  locale [V: e-file q1]. Jiff has `%V`, `%G`, `%U`, `%W` and a format-only
  `%q`. It has no local-week-year directive [V: e-file q2, h-redteam Q1].
- All callers of `pattern_to_strftime` sit in `src/template/engine/date.rs`
  [V: CodeGraph]. The other user is the re-export in
  `src/temporal/mod.rs`.
- If the grilling adopts Q5's removal proposal, config loses this format
  surface. The remaining dialect decision then covers three surfaces.

Alternatives.

- P. strftime (jiff `fmt::strtime`) is the one native dialect for query and
  schema. Moment is a render adapter at the template seam. The translator
  moves to the template engine.
- Q. strftime for query. Moment for template and schema. The translator
  stays in temporal. Schema value checks then need a parse-side
  translator, which does not exist.
- T. A sibling `format` unit outside temporal and template, shared by
  schema, query and CLI (ticket 30 alternative).
- R. One Moment dialect for every surface.
- S. Drop Moment tokens. Ticket 16 allows it (B3).

Sub-decisions that every alternative needs.

1. The token table. Which of `Q`, `Qo`, `DDDD` join the table? Never add
   the locale-week tokens (`w`, `ww`, `gggg`). A locale value is not stable
   [H].
2. The dialect marker. Keep the `%` sniff, or add an explicit marker such as
   a `moment:` prefix.
3. The parse side. Q needs a translator from Moment text to a parse
   pattern. P needs none.

Recommendation. P, with the token table decided in sub-decision 1. [P]
P has the smallest change set. It changes the schema surface only, and the
schema surface is a hint today. It keeps query and the native dialect on one
grammar. Reject R: it has no source support and rewrites two surfaces. Reject
S unless the owner stops porting templates. Q is valid only if the owner
keeps Moment tokens in schema hints and accepts a new parse-side translator.

Human questions:

- Do your schema `date.format` hints stay in Moment tokens?
- Do you plan to port your Templater scripts to `date_format`?

Reject condition for P. The owner confirms a query-side need for Moment
tokens.

### Q4 The `date.format` check for formats with no full date

Facts. Fileclass lints format tokens. A string `dateFormat` bypasses its
date-pattern value check; the outer validator still checks empty values
and `required` [V: k-file K8]. A declared `DDDD` format can represent
`152`, which is not a full date [V: e-file q4]. The frozen parser reads
`2026`, `2` and `152` as numbers [H: B47, classification happens once].
Stored fields keep no written precision. `NoteFieldValue::Date` holds a
`DateValue` [V: `src/note/field.rs`].
Ticket 20 already states a complete provisional value rule [V: l-file L1,
ticket 20:119-123]. The rule reads: "`date.format` is a write-format hint".
The provisional check accepts Date and DateTime. If the format has no
date component, it accepts any scalar. A different paragraph ends
mid-sentence ("Only date, datetime and", 20:305-315). That fragment is a
recovery item for ticket 36, not a rule [V: l-file L1].

Alternatives.

- A. A load-time lint only. Classify each format token as full date, partial
  date or no date. Report a format that the table cannot classify.
- B. A lint plus a value check by class.
- C. No check.

Recommendation. A, as a proposed amendment to ticket 20. [P]
Option A removes the provisional value check. It changes the check
contract; it does not inherit it [V: l-file L1]. Record a change-table row
and a ticket 36 supersession note if the grilling adopts A.

Keep three format classes distinct: a full date, a partial date and no
date component [V: l-file L2]. Stored-value acceptance for partial dates
and quarter formats remains a human choice. Ticket 36 owns recovery of
the fragment ("Only date, datetime and", 20:305-315).

Do not infer the missing fragment. Do not make that fragment a prerequisite
for the separately delegated check. Unquoted numeric values parse as
numbers. Quoted numbers stay text (20:311-315). The stored kind determines
what a string-shape check sees.

Reject condition. Ticket 20's intent needs a value check after all.
Then B follows A.

### Q5 The keys under `[frontmatter.date_created]` and `[frontmatter.date_modified]`

Facts. No production code reads the `format` values [V: c-digest Q5,
ticket 22]. The config digest still hashes all four keys
[V: h-redteam Q5, `src/config/model/digest.rs`]. Ticket 22 deletes that
digest type. `deny_unknown_fields` stays (B45), so a config that sets a
removed key then fails to load. Ticket 22's fingerprint sweep (B44) covers
the fingerprint, not the file format.

Alternatives.

- A. Delete the keys. Optionally accept and ignore them for one release
  with a notice; that tolerance needs separate approval.
- B. Move the keys into `[date]`.
- C. Keep the keys where they are.

Recommendation. A. [P]
C keeps two homes for date settings with no reader. B keeps dead keys.
Config formats are Migratable (B4), so A needs one change-table row.
The optional tolerance period avoids a hard load failure for existing
configs.
A tolerance release is new scope: ticket 22 keeps `deny_unknown_fields` and
calls for comment-preserving migration at the first breaking change. Neither
Answer requires an accept-and-ignore path [V: l-file L12]. Treat tolerance
as extra compatibility behavior for human approval. Without it, a config
that sets a removed key fails to load once (B45).

Reject condition. A template reads these keys through a path this review did
not inspect. Then deprecate and do not delete.

## Cluster B: helpers, filenames, weeks (Q6 to Q8)

### Q6 Calendar helpers in the template engine

Facts [V: `src/template/engine/date.rs:183-294`].

- `Anchor` is a template shorthand enum. Its `transform` calls
  `DatePoint::shift`, `snap` and `with_weekday`.
- `apply_offset` parses a template argument. It then calls
  `DatePoint::shift` or `apply`.
- `Anchor::base` reads `DateTimeValue::now()` and `DateValue::today()`. That
  is an ambient clock read.
- `snap_date` and `snap_weekday` hard-code the ISO week
  (`src/temporal/calendar.rs:35-64`).
- `weekday_from_number` rejects `0` (`calendar.rs:115-125`). The owner's
  template calls `day(0)` for Sunday [V: ticket 30].
- The calendar arithmetic already lives in temporal.

Alternatives.

- A. Keep the adapters in the engine. Move every week-rule read into
  temporal behind the frame.
- B. Move the helpers into temporal. Keep filter registration in the engine.

Recommendation. A. [P]
The e-file proposes B. The arithmetic is already in temporal. What remains
in the engine is argument parsing and template naming. The week rule is
temporal's job (B39), so Q31 moves `snap` and `weekday` onto the frame. A
function rename needs a change-table row (B3).

Reject condition. A query-side function needs `Anchor` or `apply_offset`.

### Q7 The filename-period parser

Facts. Ticket 21 scopes the new forms to filenames (B34, B37). The frozen
note parser shares `DatePrefixToken` and `TemporalShapeToken`
(`src/temporal/lexer.rs:17-54`). A new week or quarter token in those enums
changes frozen parsing. The store keeps no `day` (B51).

Alternatives.

- A. A filename parser unit in temporal with its own token enum.
- B. A parser in the file scanner.
- C. Extend the existing lexer enums.

Recommendation. A. [P]
C breaks the frozen boundary. B puts one grammar in two homes. A shares the
week and date validation with temporal and keeps the note parser untouched.

The grammar is a separate decision. The checks list needs a prefix grammar.
It must fix these points:

- The separators after the prefix.
- Letter case of `W` and `Q`.
- `W1` against `W01`.
- The quarter spelling.
- Invalid weeks and quarters, with no rollover (B34).
- A malformed longer prefix never reads as a shorter one (B34).

A counterexample for the grilling, verified at runtime. The owner's week
template writes `YYYY-[W]ww`. A Moment 2.30.1 probe with the `en` locale
(dow 0, doy 6) prints `2025-W01` for Sunday 2025-12-28 and `2026-w01` for
the locale week token `gggg-[w]ww`. The ISO form `GGGG-[W]WW` prints
`2025-W52` [V: owner probe `moment-proof.cjs`, recorded in
[m-gap-closure.md](./m-gap-closure.md), assertions pass]. The ISO prefix
rule reads `2025-W01` as the week of
2024-12-30. Two meanings for one name are possible.

Reject condition. The grilling adds filename-local rules that temporal should
not know. Then use B.

### Q8 The week-one rule for a custom week start

Inherited from Q31. See Q31 for the presets. The rule lives with the
`CalendarPeriod::Week` snap in temporal. Jiff has no custom week-year. It
has `ISOWeekDate` and the week-0 directives `%U` and `%W`
[V: h-redteam Q8, `calendar.rs:41-48`]. A custom pair needs new arithmetic
and a test corpus.

No rollover. A week 53 that does not exist is invalid, for a typed literal
and for a filename prefix (ticket 21). CLDR recommends rollover as
compatibility guidance for lenient readers, not as a parser duty
[V: e-file q8, k-file K4]. This ticket must say that Traces does not.

## Cluster C: parsing stack (Q9 to Q12)

### Q9 speedate

Facts. One call site: `src/temporal/date.rs:350-374`. The code maps
`OutOfRangeMonth` and `OutOfRangeDay` to one reason and every other error
to another. Negative years already use `civil::Date::new`
[V: d-crate Q3]. Jiff accepts a superset of the shape that speedate accepts:
unguarded `Date::from_str` accepts `20260101` and
`2026-01-01T00:00:00` [V: d-crate Q3, owner probe]. The parity probe ran on
4,635,370 generated inputs. It found 0 acceptance, 0 value, and
0 error-category differences between the speedate path and a guarded jiff
path. Source: owner probe [V], recorded in
[m-gap-closure.md](./m-gap-closure.md), reproduced in-repo at
[probes/parity](./probes/parity). Removal drops
`lexical-parse-float` and `strum` from the lockfile
[V: Deletion lens D1].

Alternatives.

- A. Remove speedate with the shape guards and error-category mapping.
- B. Keep speedate until the diagnostic parser exists, then remove it.
- C. Keep permanently.

Recommendation. A, with the guards below. [P]
The probe finds no difference within its generated guarded-date domain.
Retain the length, separator and digit guards and mapped reason strings.
[n-parity-and-scope.md](./n-parity-and-scope.md) confirms that only the
unsigned full-date field branch calls speedate. Datetime, duration and
zone comparisons are not missing removal evidence. The probe does not
execute repository wrappers or establish whole-parser parity. The future
carrier/frame redesign still needs separate acceptance.
The ticket 25 fingerprint list changes when the crate leaves (B61), and the
lockfile diff needs a rebuild-coupling note (h-redteam Q10).

Reject condition for the end state. The grilling wants the diagnostic
parser to land first, because the reason strings feed it. Then B.

### Q10 winnow

Facts. One call site: `duration_part` in `src/temporal/lexer.rs:139-148`.
Jiff's friendly parser cannot report a consumed-prefix length
[V: d-crate Q2, `fmt/friendly/parser.rs:183-194,311-348`]. Winnow stays in
the lockfile through `toml` either way [V: d-crate Q4].

Alternatives.

- A. Remove the direct dependency. Write a hand scanner of about 10 lines.
- B. Keep winnow.

Recommendation. B. [P]
The scanner owns the frozen duration boundary (B1). The doc comment at
`lexer.rs:139-148` is the decision record. Two costs need prices before the
owner chooses:

- Ticket 25's crate list omits `winnow` today (D7). Keep winnow, and the list
  must add it.
- A `toml` bump can change the winnow version. That would change the
  extractor fingerprint and force one rebuild [H].

Reject condition. The rebuild coupling costs more than the 10-line scanner.

### Q11 logos

Facts. Logos drives the temporal lexers, the note lexer and the query
grammars (`src/temporal/lexer.rs`, `src/note/parser/lexer.rs`,
`src/query/grammar/filter.rs`) [V: d-crate Q5]. No defect is on file.
Recommendation. Keep. A removal rewrites four lexers. [P]

### Q12 jiff

Facts. `jiff = { version = "=0.2.38", default-features = false, features =
["std", "tz-system", "tzdb-bundle-always"] }` (`Cargo.toml:128`) [V]. Every
accepted ticket assumes jiff semantics. Jiff has no custom week-year (Q8).

Recommendation. Keep jiff. [P]
The gap in Q8 is a gap, not a reason to switch. `chrono` has the same gap in
its default API [H]. Reject condition: the d-crate file finds a blocking
limitation for an accepted feature. It finds none.

## Cluster D: natural-language dates and keywords (Q13 to Q18)

### Q13 The grammar

Inherited. Ticket 19 proposes a deterministic, closed English grammar,
resolved from the injected `today` (P2).

Alternatives.

- A. A closed list of keywords, weekdays, offsets and period phrases.
- B. An open natural-language parser.
- C. Defer the grammar to fog. Keep only the ticket 19 hook (ticket 30
  alternative).

Recommendation. A. [P]
Dataview uses a closed keyword set. Tasks documents ambiguous ranges as the
failure of flexible parsing [V: e-file q13]. A query engine needs
deterministic plans. C is valid only with an explicit scope decision.
Adoption, the rule-table semantics and the crate choice in Q14 are
separate decisions (l-file L11). The grilling can adopt a closed grammar
and still change every proposed rule.

The Answer needs a rule table with positive and negative examples.
The table below is a proposal, not an implementation. Its completeness
remains unproved. It uses Q17 option A: nullary `today()` and `now()`,
with explicit arguments for period-edge functions. The natural-language
phrase examples use the proposed shared parser. Any different spelling
needs a separate Q17 decision [P].

| Phrase | Kind | Meaning (proposal) | Positive | Negative |
| --- | --- | --- | --- | --- |
| `today()` | point | the frame civil date | `due = today()` | `2 today()` |
| `now()` | point | the frame instant | `t < now()` | `now() < 1d` is a Temporal-to-Duration cross-bracket |
| `tomorrow` / `yesterday` | point | today ± 1 day | `due = date("tomorrow")` | `2 tomorrow` |
| `<weekday>` | point | Q15 decides the same-day rule; default proposal: next occurrence strictly after today | `due = monday` | `mondays` |
| `this/next/last <weekday>` | point | current-week day, next occurrence, previous occurrence | `due = next monday` | `last next monday` |
| `this/next/last week/month/quarter/year` | period | `(start, unit)` of that period | `due in next week` | `file.date + next week` puts a period in arithmetic |
| `in <n> <unit>` | point (Q16 open: point or period) | today + n units, checked | `due < in 2 weeks` | `in 2 fortnights` |
| `<n> <unit>(s) ago` | point (Q16 open: point or period) | today − n units | `done > 3 days ago` | `3 days ago tomorrow` |
| `eom(today())`, `sow(today())`, and the other six edge functions | point | start or end of the current period | `due <= eom(today())` | `eom()` needs the separate Q17-B arity proposal |
| `start of/end of <period>` | point | natural-language alias of the corresponding edge calculation | `date("start of month")` | `middle of month` |

Rules the table must also state:

- Anchor. Every phrase resolves against the injected frame. `now()` reads
  the frame instant and `today()` reads the frame civil date (B16). No
  phrase reads an ambient clock.
- Full consumption. A literal matches a row only when the whole literal
  matches that row. `in 2 weeks ago` matches no row.
- Unicode. Match and slice only at valid `char` boundaries. The proposed
  parser must return an error, not panic, for unsupported input.
- Checked integers. Parse counts and calculate offsets with checked
  arithmetic. `in 999999999999 weeks` exceeds the supported date range.
  Do not describe this as integer overflow without a specified integer
  type. The failure stage and diagnostic remain human choices [P].
- Date-shaped advisory scope (B21). A recognized but invalid phrase and an
  unrecognized string are different cases. The proposed grammar must
  define recognition separately from value validation [P]. The existing
  quoted-literal rule still governs its accepted scope. The synthesis
  does not establish a new notice rule for `"next weke"` [H].
- Weekday rows use the Q15 same-day rule. The bare-weekday row proposes
  strictly after today [P]. The `this/next/last` rows need explicit
  same-day rules from the grilling.
- Phrase validation and arithmetic clamping are different operations.
  Point offsets use the accepted arithmetic policy, which clamps
  month-end dates. For example, a one-month offset from 2027-01-30
  produces 2027-02-28 [V: h-redteam Q16, `point.rs:1726-1771`].
  The proposed period rows instead shift the normalized period start.
  Failure handling for an invalid count or an unsupported result remains
  a grammar decision, not an inferred extension of B21.

Reject condition. The owner uses phrases the closed list cannot cover.

### Q14 Build in the repository or use a crate

Facts [V: d-crate Q6/T1 to T10; M
[acceptance closure](./m-gap-closure.md#four-candidate-acceptance-closure)].

- All four candidates return one point, without a period unit or an error
  byte span. `next week` errors in English and chrono-english; fuzzydate
  and GNU return March 21 noon from the March 14, 2024 noon UTC anchor.
- All four execute the 64 shared case definitions. English also executes
  two lenient cases. Candidate `OK/ERROR/PANIC` totals are 32/27/7 for
  English, 29/34/1 for fuzzydate, 38/26/0 for GNU and 32/30/2 for
  chrono-english. The totals exclude the four baselines and 36 host
  variations. Every record contains identical first/warmed outcomes.
- The English UTF-8 abort was already demonstrated under the historical
  abort profile (1,234 failures from 5,514 inputs, bounded). The new
  unwind harness also catches unchecked Span constructor panics and a
  fuzzydate `NaiveDateTime`/`TimeDelta` overflow panic on
  `3000000000 days ago`. Chrono-english panics on UTF-8 slicing. GNU has
  no sampled panic, not a universal safety proof. A product `Result`
  wrapper cannot catch abort-profile process termination.
- Both English parsers turn `4294967295 years` into a 2023 result from
  the 2024 anchor. Chrono-english accepts `today\u0000junk` by ignoring
  the suffix and emits unsolicited `fkd date` twice for `1000000 years`.
  GNU accepts timestamp and input-zone items beyond the proposed grammar.
  Its `2024-03-14 x` succeeds because `x` is a military zone.
- All seven matching weekday anchors execute. On Thursday March 14 noon,
  bare `thursday` gives March 21 midnight in English/chrono-english,
  March 14 noon in fuzzydate and March 14 midnight in GNU.
  US/UK `next friday` gives March 15/22 midnight in both English parsers.
  Fuzzydate gives March 22 noon in both; GNU gives March 15 midnight.
  The latter two APIs have no dialect argument.
- Explicit `00:00`, not bare `today` or `today at 00:00`, establishes the
  observed midnight results. English/GNU move Sao_Paulo's 2015-10-18 gap
  to 01:00-02 and Havana's 2020-03-08 gap to 01:00-04.
  Fuzzydate/chrono-english error. Havana's 2020-11-01 fold gives
  00:00-04 in English/fuzzydate, 00:00-05 in GNU and an error in
  chrono-english. These do not select the human's conversion policy.
- Selected explicit-anchor/zone paths stay unchanged across 36 host
  `TZ`, `LC_ALL` and `TZDIR` variations. Plain fuzzydate and GNU
  entry points still read the clock. English's Jiff defaults and the
  Chrono stack in fuzzydate/chrono-english remain feature-coupling facts,
  not corrected by explicit-context results.
- All four build at the actual repository floor, Rust 1.96.0.
  GNU declares `rust-version = "1.71.1"`; chrono-english uses edition
  2024 without a declared floor. English/fuzzydate use edition 2021
  without a declared floor. Upstream support for such an undocumented
  minimum is a vendor promise this research cannot test; the successful
  repository-floor builds close the compatibility question here.
- The M allocation and size tables cover all four instrumented candidates.
  For `thursday` at March 14 noon UTC, each pair shows first and warmed
  counts. English allocates 8(73 B) and 8(73 B). Fuzzydate allocates
  12(12,834 B) and 5(78 B). GNU allocates 1(128 B) and 1(128 B), and
  chrono-english allocates 4(41 B) and 4(41 B).
  Fixtures and zone lookup stay outside the call window; native input
  copies and unwind handling stay inside. Construction and baseline
  parsing allocate zero. Formatting and result drop are separate.
- The new sizes use opt-z, LTO, strip, one codegen unit and unwind
  instrumentation. They are not comparable to earlier abort-profile
  sizes or production integration costs. No new throughput figures exist.
  M preserves older indicative timings and dependency-linkage costs.
- The English repository still returns 404. Public repository and commit
  searches find no alternative; package VCS SHA
  `a8b87edbeebafebd06a6c4f5a2b502b79b61e409` does not expose maintenance
  history. The 2026-11-17 snapshot recorded 29 downloads; a 2026-11-18
  read records 30. No count establishes maintenance quality.

Alternatives.

- A. Build the grammar in the repository.
- B. Use `jiff-english`.
- C. Use `fuzzydate`.
- E. Use `parse_datetime`.
- F. Use `chrono-english`.
- D. Defer to fog.

Recommendation. A. [P]
The digest evidence adds an error contract: span-carrying errors, no
zero-date fallback and no process exit [V: c-digest Q14]. No unmodified
candidate meets the complete contract. Three have actual panic evidence.
GNU has no bounded panic but has overbroad grammar and point-only output.
All four now have runtime, allocation, same-day and floor-build evidence.
T1 to T10 distinguish observed outcomes from the human's chosen semantics.
An adapter needs its own restriction, period and diagnostic evidence.
The in-repository grammar has no demonstrated coverage or completeness.
Determinism also needs pinned zone data, week rules and dialect (K17);
bounded host invariance does not prove every transitive path.

Reject condition. A later read or probe shows a candidate that yields
`(start, unit)` periods with an injected anchor, spans and no second
calendar stack.

### Q15 The meaning of a bare weekday

Facts [V: e-file q15, c-digest Q15].

- Tasks resolves the nearest weekday.
- `chrono-english` resolves the coming weekday and splits on US and UK.
- java.time names four adjusters and picks none.
- The template filter `date.weekday(n)` picks weekday `n` of the current ISO
  week. On Sunday 2026-11-15, `weekday(1)` gives Monday 2026-11-09
  [V: h-redteam Q15, `calendar.rs:57-64`].
- The c-digest leans to the current-week day. It matches the template
  filter and is deterministic.
- All four execute seven same-day weekday anchors. Fuzzydate and GNU
  include today; the English parsers move strictly forward in the tested
  bare-day cases. These runtime observations confirm the cited source
  rules; none selects the owner's preference [V: d T3, M].

Alternatives.

- A. The next occurrence strictly after `today`.
- B. The nearest occurrence.
- C. The day in the current week of the frame.

Two counterexamples expose the choice. On Wednesday, the nearest Tuesday
is yesterday, so `due > tuesday` includes today. The current-week Monday
can be six days before Sunday. The strictly-after rule instead selects
the next week. These observations do not establish that a nearest-weekday
function is nonmonotone.

Recommendation. The default proposal is A, but the owner decides [P].
It names a forward-only rule. State separate meanings for `this`, `next`
and `last`, including the same-day case. Also decide whether the existing
template filter and query spelling must agree.

Human questions:

- Which query meaning do you expect: next, nearest or this week?
- Should the query and template meanings agree?

### Q16 Interval precision of `next week`

Inherited. The grammar must yield a period with a unit (B31).

Alternatives.

- A. Period phrases yield `CalendarPeriod`. Point phrases yield `DatePoint`.
- B. Yield points only, with two spellings for the bounds (Dataview).
- C. Yield a start and an end (Tasks).

Recommendation. A. [P]
An end bound requires a boundary rule under option C. No measurement
establishes its per-shift cost here [H]. Option B needs an explicit rule
for `file.date = next week` [V: e-file q16].

Open questions:

- Does `in 1 month` return a point or a period?
- Does `3 months ago` return a point or a period?

If `in 1 month` denotes a point, 2027-01-30 clamps to 2027-02-28 under
the accepted arithmetic rule [V: h-redteam Q16]. A period phrase instead:

- Derive the period that holds `today`.
- Shift it by whole units.

The start is then always a unit start [H].

Reject condition. A period phrase needs a start that is not a unit start.

### Q17 The spelling of relative keywords

Facts. A bare word parses as a field path. The edge functions take exactly
one argument (`src/query/grammar/filter.rs:591-597`). A quoted `"today"` is
also a late-typed candidate (B19). Ticket 29 binds relative keywords to the
pinned frame at plan time (B69).

Alternatives.

- A. Nullary calls `today()` and `now()` only. Compose as `eom(today())`.
- B. Nullary calls for every edge: `eom()`, `sow()`.
- C. Keywords as arguments: `date(today)` (Dataview).
- D. Bare keywords (Tasks).

Recommendation. A. [P]
The collision with field paths excludes C and D. B gives every edge function
a second arity. A adds two names and no new arity.

Disagreement to record. The c-digest leans to C. The e-file leans to B.
Reject condition. A user needs `eom()` without a date argument. Then add B
for that function.

### Q18 The spelling of `member_of`

Inherited. Ticket 29 reserves the IR name only (B68). Membership does not
define canonical group identity or a total range-key order. This does
not prohibit R5's operator-specific candidate bounds. Under B18 and B32,
`point == period` already tests membership.

Alternatives.

- A. A named function: `member_of(...)`.
- B. A prose word such as `within`.
- C. Overload `in`.

Recommendation. Prefer A or B [P].
The word `in` already has range-related precedents, but that alone does
not prove a grammar ambiguity. Do not exclude C without actual parse
examples [V: k-file K23].

The Answer must identify a query that needs a separate operator.
If `point == period` covers every case, the new spelling is a readable
alias [P].

Human questions:

- Which word do you want?
- Which query needs it?

## Cluster E: value shapes (Q19 to Q25)

### Q19 The type shape for civil and instant values

Facts.

- `DateTimeValue` wraps a `Timestamp`. A naive datetime resolves through the
  host zone at parse time [V: `datetime.rs:76-78`, `point.rs:34-39,113`].
- Ticket 19 makes a naive datetime a civil, zone-free value (B16). The store
  must hold it without a zone (B54).
- `DatePoint` has two variants today: a civil date and an instant. Neither
  holds a civil datetime.
- `NoteFieldValue` holds `Date(DateValue)` and `DateTime(DateTimeValue)`.
  The enum is 80 bytes. The largest payload is 72 bytes [V: ticket 18].
- The probe measured the sizes: `civil::Date` 4 bytes, `civil::DateTime` 12,
  `Timestamp` 16, `Span` 64, a two-case `{Civil|Instant}` payload enum
  24 bytes [V: owner probe, recorded in
[m-gap-closure.md](./m-gap-closure.md)].
- The layout probe measures a mirror enum at 80 bytes on the recorded
  host and compiler. Its Duration and Object payloads both measure
  72 bytes. The mirror's 16-to-24-byte temporal payload increase does not
  increase its total size. It is not a production-type size assertion.
  The allocation window covers enum construction, not parsing
  [V: [m-gap-closure.md](./m-gap-closure.md),
  probes/parity/tests/layout.rs].
- A new variant bumps the format epoch (B58).

Alternatives.

- A. An inner enum inside `DateTimeValue`: civil or instant. Remove any
  derived `Ord` across the two classes.
- B. A `NoteFieldValue` variant for a civil datetime beside the instant
  variant.
- C. A third `DatePoint` variant. This does not reach stored values. A
  stored value is a `DateTimeValue`. C needs A or B as well.

Recommendation. A. [P]
A keeps `NoteFieldValue` at its current variant count. It keeps one
temporal bracket for filters (B15). The payload enum measures about 24
bytes for the two-case `{Civil|Instant}` carrier. The mirror enum
measures 80 bytes on the recorded host and compiler. This is evidence for
the proposal, not a guarantee for the production type or another target
[V: [m-gap-closure.md](./m-gap-closure.md),
probes/parity/tests/layout.rs].

The existing `field.rs:579-580` assertion measures the current type.
Keep a size assertion on the implemented carrier. Ticket 19 forbids
`Ord` on `NoteFieldValue`; removing `Ord` from the proposed mixed
Civil/Instant carrier is a recommendation, not a ban on every inner type.
A compile-fail check can guard a chosen type-level prohibition. A runtime
comparison cannot establish that a trait implementation is absent
[V: j-file, l-file L6].

Gates (implementation). Run the production carrier's size assertion.
Bump the format epoch. Add the
civil case to the fingerprint corpus (B58). Record the naive-datetime
semantic change for ticket 36. It is authorized by B16 but it changes a
frozen parse result.

Reject condition. A later layout change pushes `NoteFieldValue` past 80
bytes with the inner enum. Then choose B.

### Q20 Where `CalendarPeriod` sits in the value model

Facts. Ticket 21 renames today's kind-only type to `CalendarUnit` and makes
`CalendarPeriod` a civil start plus a unit (B31). The old type has three
variants today (`src/temporal/calendar.rs:23-30`). Query evaluation passes
`NoteFieldValue` everywhere: function results, `CompareOp::is_satisfied_by`
and group keys (`src/query/grammar/filter.rs:359-433,783-809`). A period
literal, `file.date` and a period group key must travel through that path.

Alternatives.

- A. A `NoteFieldValue::Period` variant. Estimated at about 8 bytes [H].
- B. A query-layer value wrapper that adds a period case. Stored values
  stay unchanged.

Recommendation. B if the owner wants stored fields free of periods. A if
periods appear in stored fields. [P]
`file.date` is derived at evaluation and never stored (B35, B51). Stored
notes hold no period in frontmatter [H]. That points to B. Ticket 18 chose
one runtime value type (18:458-461). B is therefore a proposed explicit
revision of that single-carrier rule, not a silent inheritance
[V: l-file L4, i-file L6]. It needs a change-table row and a ticket 36
supersession note.
Runtime representation stays separate from note-parser acceptance: a
runtime period variant does not force the note parser to produce periods.
The option must carry a computed `file.date` from evaluation through
grouping or a template without storing it (i-file L6).

Reject condition for B. A query needs a period inside a `List` or `Object`
field value, or a computed `file.date` cannot reach grouping or a template
end to end. Then B falls.

### Q21 Date versus instant at an ambiguous or skipped local midnight

Facts. A civil midnight can be absent (a gap) or repeated (a fold). In
`America/Sao_Paulo` on 2015-10-18 no instant has civil time 00:00
[V: d-crate, `Compatible` row]. A gap breaks monotonic mapping. In
`America/New_York` on 2026-03-08, civil 02:30 maps to 07:30Z. Civil 03:00
maps to 07:00Z [V: B67, h-redteam Q21]. Ticket 21 compares a period with an
instant by the instant's civil date in the pinned zone (B32).

Alternatives.

- A. Map the civil value to an instant, then compare instants.
- B. Compare civil fields only and never convert.
- C. Return `Null` with a notice when the mapping is ambiguous.
- D. Project the instant to a civil value in the pinned zone. Compare civil
  values.

Recommendation. D. [P]
The projection has no local-time gap or fold choice once the zone is
available. Zone acquisition can still fail. Under a civil-midnight equality
rule, no instant equals `Date 2015-10-18` on that Sao Paulo transition day.
That does not make the civil day invalid or prevent period membership.
Option A is not monotone across the cited gap. Retain a typed zone
failure; the error must not become a comparison result (B30).

The current fallback is at `src/note/field.rs:297-300`. The code maps a zone
failure to a `Less` or `Greater` outcome there.

Reject condition. A filter needs an instant bound that D cannot express.

### Q22 The total sort key for mixed temporal values

Inherited. Ticket 29 fixes the group-key class order: Civil, Instant,
Period (B65). Ticket 21 separately requires a deterministic total sort key
for mixed temporal values (21:149-156). This question decides that complete
key, not only the unit tie-break [V: l-file L6].

Facts. A period key is `(start, unit)`. A period has no written precision.
Group keys need a fixed order, or hash order changes the output between runs
[V: h-redteam Q22]. Sorting projects each value once to its canonical class
key. That projection needs no zone lookup (l-file L7). Ticket 19 forbids
`Ord` on `NoteFieldValue` itself (19:87-102). Inner temporal types can
implement order.

Recommendation. Class first, then the within-class key. [P]

- Civil: `(date, time)` order.
- Instant: epoch order.
- Period: start date, then the proposed unit rank Day, Week, Month,
  Quarter, Year [P; precedent: e-file q22].

No quantity ranking is possible, because a week and a month have no common
length without an anchor. Whether the sort key reuses the group-class rank
Civil, Instant, Period is part of this decision.

Reject condition. Ticket 29's key encoding already fixes a tie-break.

### Q23 Nominal duration equality and order

Facts. One month equals 30 days. One year equals 365 days
[V: `src/temporal/duration.rs:11-13`, `unit.rs:210`]. `52w 1d` equals
`1y`. A calendar month has 28 to 31 days. A fixed 30-day month matches only
some months [V: e-file q23].

Alternatives.

- A. Keep nominal equality and order (B23).
- B. Class-aware equality: calendar units compare only with calendar units.
- C. Forbid nominal units in stored durations. Keep them in arithmetic.

Recommendation. A, with the counterexample answered in the Answer. [P]
B and C change documented behavior. B23 allows a refinement with a
counterexample. The counterexample is real, but no source shows a query that
relies on the identity or fails because of it [A].

Reject condition. A user-facing defect report shows a wrong result.

### Q24 Documents for calendar arithmetic

Recommendation. Put the rule text in the glossary at cutover. Leave the
glossaries unchanged until then (ticket 19 and ticket 36 rule). Tie the
duration wording to the Q23 outcome. Use three lists in the text: exact
rules, rules with a caveat, undefined cases. [P]

### Q25 `date(link)` and a Markdown target

Inherited. `date(link)` reads only the last path segment. It strips folders,
`#anchor` and `.md`, and it never reads display text (B22). Ticket 19 adds
three Markdown points: a target keeps its extension, it can be
percent-encoded, and external URLs are not note targets [V: ticket 19].

Alternatives.

- A. Take the last raw `/`-separated segment first. Strip `#anchor` and
  the extension. Decode percent-escapes once. Parse a prefix or the given
  pattern. A target with a URL scheme yields `Null`.
- B. Read the linked note's date field. This is not a live option (B22).

Recommendation. A. [P]
Order matters. Component separation precedes decoding (RFC 3986 section
2.4). An encoded separator never creates a new path boundary, and no
pass decodes twice [V: k-file K22]. Example: `2026-01%20notes.md` takes
its last raw segment, drops `.md`, then decodes to `2026-01 notes`. A
target `2026/01%2F02.md` has the last raw segment `01%2F02.md`, which
decodes to `01/02`. Decoding first would change segment boundaries and
read `02` instead. A URL such as `https://x/2026-01-01` yields `Null`.

Open point. Does `[[b]]` fold to the same `ComparisonKey` as `[b](b.md)`
(B22)? Ticket 23 or the value module owns that rule.

Reject condition. Ticket 19's Answer already fixes the decode rule.

## Cluster F: configuration, zone, frame (Q26 to Q34)

### Q26 The `[date]` table syntax

Inherited. Config owns `[date]` (B42). TOML is Migratable (B4).
`deny_unknown_fields` stays (B45).

Alternatives.

- A. One flat table with scalar keys.
- B. Subtables per surface.

Recommendation. A, with flat key names. [P]
Use `filename_year_prefix`. A dotted key such as `filename.year_prefix`
makes a nested table in TOML, which is B. The design has exactly three
keys: `zone`, `week` (a preset or a pair), and `filename_year_prefix`
[P; key inventory: l-file L13]. A diagnostic key would be new scope.
Identify its consumer and owner before adding it. Ticket 22 decides the
config-parser boundary (22:87-94, 22:223-233). It does not decide whether
a diagnostic setting exists elsewhere.

Merge and trust (B46, ticket 22). Ticket 22 already fixes the merge for
`[date]`: global config first, then local, field by field (22:223-235).
Local values apply only after the trust gate passes. A `[date]` exception
would be a proposed amendment, not a missing rule [V: l-file L8].
A global setting can change calendar operations in each vault.
It does not reinterpret ISO filename-week labels. See Q31.

Reject condition. A fourth key joins. Then split the table by consumer.

### Q27 The fields of `DateConfig`

Fields.

- `zone`: the zone spelling (Q29).
- `week`: a preset or a `(first_day, min_days)` pair (Q31).
- `filename_year_prefix`: a bool opt-in (Q32).

Consumers. The frame is a consumer of every field. The filename parser
runs during query evaluation (B35, B51). It needs `filename_year_prefix`
there. The frame carries the policy (Q33).

Recommendation. Exactly these three keys. [P]
Reject condition. A non-temporal consumer needs a field. Then that field
moves to its own policy.

### Q28 The temporal-owned policy

Accepted. Ticket 22 gives `[date]` to config (B42). Ticket 20 keeps modules
as data and pure functions (B27). The edge from config to temporal is a
[P] lean only (P1).

Alternatives.

- A. Extend `IndexPolicy`.
- B. A new temporal-owned policy type.

Recommendation. B. [P]
A couples index policy to rules the index never consumes. Ticket 25 puts a
rule in `IndexPolicy` only when a persisted facet uses it (B60). B60 names
no field. The e-file calls the type `DatePolicy`. Pick one name in the
Answer. The types differ by module ownership: config owns the raw table,
temporal owns the validated policy. The policy carries the zone spelling and
the week rule, not a loaded zone. Acquisition stays lazy in the frame
(Q30) [V: l-file L9].

### Q29 The syntax and validation of a configured zone

Facts. Precedence is host override, then config, then system (B42). A fixed
offset has no clean IANA name. `Etc/GMT-2` means UTC+02:00, with a reversed
sign. The repo uses it in a test (`src/note/field.rs:1041`)
[V: h-redteam Q29]. The e-file recommends an explicit `+HH:MM` form
[V: e-file q29]. Legacy names such as `EST5EDT` are documented exceptions
to the tzdb naming guidelines [V: k-file K14].

Alternatives.

- A. An IANA name only.
- B. An IANA name or an explicit `+HH:MM` offset.

Recommendation. B. [P]
B avoids the `Etc/GMT` sign trap. Three concerns stay separate [V: l-file
L9]. The spelling syntax is an IANA name or an explicit `+HH:MM` offset.
A spelling check validates that text against the bundled tzdb
(`tzdb-bundle-always`) and acquires no zone. Acquisition loads the zone
at first need and stays lazy per frame (B28). A config-load spelling
check is possible without any acquisition. Whether to run it at load is
a human choice. A load-time spelling check must not force a zone for
civil-only work, and it must not fall back to UTC. The system zone stays
lazy (Q30).

Open point. Choose when to check spelling: config load or first
acquisition. B28 already fixes lazy acquisition. A load-time spelling
check changes failure timing, not acquisition timing.

Reject condition. A measured need for POSIX TZ strings. None exists.

### Q30 How the frame gets the system zone

Facts. `TimeZone::try_system` needs feature `tz-system`. The project enables
it by name (`Cargo.toml:128`). On failure it returns an error. A five-minute
cache sits behind one `RwLock` [V: owner-evidence.md].

Alternatives.

- A. The frame resolves the system zone lazily at first need. It memoizes the
  result and any failure for one operation (B28).
- B. The host resolves the zone once at workspace open and holds it.

Recommendation. A. [P]
B holds a stale zone after the host zone changes. A long-lived host exists
because ticket 17 commits the LSP. B also changes ticket 21's
per-operation acquisition rule, so it needs an explicit amendment, not a
silent choice [V: l-file L9].

- The hazard is in source. An operation that calls `local_zone()` more than
  once can read two zones if the cache expires in between
  [V: owner-evidence.md].
- Nobody observed it.
- The cache can still serve a zone for up to five minutes across frames.

Reject condition. A measured need for host-held zone state.

### Q31 Is the week start configurable?

Facts.

- `CalendarPeriod::Week` is ISO, Monday start, with the ISO week-one rule
  [V: c-digest Q8].
- Tasks uses ISO weeks. The inspected Dataview `sow` uses the Luxon ISO
  week [V: k-file K3]. A claim about a separate `useLocaleWeeks` path
  needs its own source and version. nldates reads a private locale field
  and changes labels silently [V: c-digest Q8].
- CLDR, Moment and java.time define a week by a first day and a minimum
  number of days in week one [V: e-file q8].
- The owner's week template lists days from `day(0)`, which is Sunday
  [V: ticket 30 Current code].
- Jiff has no custom week-year. A custom rule needs new arithmetic and tests.
- The `(sunday, 4)` pair is real: CLDR 49 lists Iceland and Portugal in
  both the four-day and the Sunday-first groups [V: k-file K4]. The
  preset design does not depend on locale coverage counts.

Alternatives.

- A. ISO only. No config key.
- B. A free pair `(first_day, min_days)` with the ISO default.
- C. Named presets: `iso`, `us`.

Recommendation. C, with B as the escape if the owner needs another pair. [P]
Presets name common pairs. A free pair is not invalid merely because it
is uncommon: CLDR includes `(sunday, 4)` [V: k-file K4].
The owner's Sunday-first template is evidence for discussing a non-ISO
rule, not proof that the query must use it.

Open points.

- Which config level may set the week rule? A global setting affects
  general calendar operations in each vault. A local setting limits that
  effect to one vault. Neither setting changes committed ISO filename
  labels such as `2026-W01`. A local-only week setting is a proposed
  exception to the inherited merge rule [V: l-file L8].
- Filename prefixes stay ISO. Ticket 21 fixes filename weeks to Monday
  start and the ISO week-one rule (21:170-179). A configured week rule does
  not reinterpret them [V: l-file L10]. A spelling for a non-ISO filename
  week would be new scope, and the grilling must ask for it separately.

Reject condition. The owner confirms no use of a non-ISO week. Then take A.

### Q32 The opt-in key for bare-year filename prefixes

Facts. Ticket 21 enables a bare-year prefix only through `DateConfig` (B34).
A bare `2019` collides with names like `2019 budgets.md` and `1998 census.md`.

Alternatives.

- A. A global bool: a leading four-digit year counts as a year prefix.
- B. A narrower shape: only a stem that is exactly four digits (`2026.md`).
- C. A per-directory opt-in by a glob list.

Recommendation. B if the owner's year notes are whole-stem names. A
otherwise. [P]
B removes the collision with ordinary names. C adds a per-directory config
channel. No source shows such a channel [H]. The key is `filename_year_prefix`
(Q26).

Human question:

- How are your year notes named?

### Q33 What temporal exports to the frame

Inherited. Ticket 21: one frame per query, render or request pins the clock
and the week rule. The frame acquires the zone only when needed. A failure
is memoized (B28).

Alternatives.

- A. Capture the zone, clock and week rule at operation start.
- B. Capture the clock and the policy at operation start. Acquire the zone at
  first need. Memoize the result and any failure.

Recommendation. B [P].
The proposed payload contains a clock sample, the temporal policy (Q27)
and a memoized zone cell. A frame shared by an LSP request or a lazy
result set (B41) can cross threads [H].

Use `OnceLock`, not `OnceCell`, for that proposed cross-thread carrier
[P]. Option A acquires a zone for work that needs none. Ticket 21
forbids that acquisition for stored civil values (B36).

Count. CodeGraph shows 65 callers of `local_zone()`. A text search shows 133
hits when tests count [V: h-redteam Q33]. Regenerate the list at
implementation time.

Reject condition. A measured case where lazy acquisition changes a result in
one operation.

### Q34 Replacements for ambient reads

Inherited. Ticket 21 removes the ambient reads (B38).

The recorded ambient-read list is not exhaustive. Rebuild it during
implementation:

- `local_zone()` in filter arithmetic and boundary functions
  (`src/query/grammar/filter.rs`).
- `local_zone()` in template helpers (`src/template/engine/date.rs`).
- `local_zone()` in `is_midnight_of` and `TryFrom<DateValue>`
  (`src/temporal/datetime.rs`).
- `DateTimeValue::now()` and `DateValue::today()` in `Anchor::base`
  (`engine/date.rs:245-255`).
- `now_instant()` in `is_past` and `is_future` (`engine/date.rs:795-817`).

Recommendation. Pass the frame into each. Replace `now_instant()` sampling
with the frame clock. [P]
The scaling lens prices the per-row lookup at EAV 0.90 [E]. The reason that
matters is correctness: a failed lookup drops rows silently today. That list
is an implementation gate.

## Cluster G: pushdown and notices (Q35, Q36)

### Q35 Temporal probe bounds per operator and value kind

Inherited. Ticket 29 R1 to R7: Exact, Inexact and Unsupported. The residual
predicate always runs. An unproved mapping scans (B70).

Facts. The cited timings measure numeric and folder keys: 171 microseconds
per 5000 numeric rows and 261.5 microseconds per 7143 folder rows at 50k
[V: `29-prep/g1-probes.md:46-58`]. Nobody measured a temporal key. The store
keeps temporal values as text (B52). A civil-to-instant mapping is not
monotone in a gap (Q21). `field.rs:538-546` is an `i64` to `f64` cast. It is
not a temporal case.
R5 already uses membership bounds for point-period matching (29:538-542).
Section 10 reserves `member_of` and bars that relation from range and group
keys (29:1186-1194). The two statements describe different things: canonical
keys versus operator-specific candidate bounds. A membership bound does not
make the relation a total key order [V: l-file L5]. The earlier draft read
this as a contradiction. That reading is withdrawn [P: a one-sentence
clarification in the reconciliation stays optional].

Alternatives.

- A. Prove Exact bounds now for zone-free kinds only: civil dates, civil
  datetimes and periods. Keep every zone-dependent mapping Unsupported.
- B. Prove bounds for instants as well.
- C. Keep every temporal mapping Unsupported until the implementation ticket
  proves it.

Recommendation. A [P].
Keep instant kinds Unsupported in this proposed first scope. This is a
scope choice, not an inherent zone dependency. An instant-to-instant
bound needs no civil projection. A mixed civil/instant predicate can
need one.

Prove bounds per operator and row kind before classifying a mapping as
Exact. Include quoted literals, zone-gap days and period boundaries.
Do not derive predicate bounds from the total sort rank.

The inspected plan has no temporal candidate-bound protocol [V: j-file].
This is not a whole-repository absence proof.

Reject condition. The grilling wants zone-dependent bounds before the
config pins the zone set. Then C for zone-dependent kinds.

### Q36 Notices for `Null` dates

Inherited. Ticket 34 owns the notice mechanism. Ticket 29 did not adopt the
first-round notice for `Null` dates (B73).

Facts. "`Null` compared" is the most common case: every note without a `due`
field compares as `Null` [V: h-redteam Q36]. A malformed date-shaped
filename prefix, such as `2026-13-40-notes.md`, gives a `Null` `file.date`.

Alternatives.

- A. A notice on every silent coercion, including an absent value.
- B. No notice for an absent value. One notice for a malformed date-shaped
  value. A typed error for a zone failure.

Recommendation. B. [P]
Option A produces notices for absent values. Option B separates absence,
malformed input and environmental failure [V: e-file q36]. The frame
memoizes zone failure (B28). That memoization alone does not guarantee
one notice; ticket 34 owns notice delivery and deduplication.

Reject condition. Ticket 34's Answer enumerates a broader notice class.

## Downstream contracts

| Ticket | Needs from this ticket |
| --- | --- |
| 31 template | Owner and behavior of `date_format` and the translator (Q1 to Q3). The engine helpers that stay (Q6). The names that accept natural language (Q13 to Q17). How civil, instant and period values render (Q19 to Q21). The frame type and the zone failure type (Q33). The unsupported-token error shape. |
| 34 diagnostics | Temporal error set. The typed zone failure. The `Null` and malformed-date notice conditions (Q36). Any schema date-format code (Q4). |
| 35 topology | The parsing-stack result (Q9 to Q12). The crate list change for ticket 25. The temporal policy type and its owner (Q28). |
| 36 reconciliation | The supersession list below. The naive-datetime semantic change (Q19). The `file.day` to `file.date` migration. The filename prefix parser stays apart from the frozen note parser (Q7). |

## Supersession items for ticket 36

- An earlier ticket draft called the temporal class order undefined.
  The corrected text cites ticket 29's group-class order (B65).
- Ticket 20's Answer ends mid-sentence (I1).
- Ticket 18 lists no period value. Ticket 21 needs one (Q20).
- Ticket 25's crate list omits `winnow` (D7).
- Ticket 23's Question still adds date keys to `IndexPolicy`.
- Ticket 29 uses `eom` as a relative keyword. The grammar defines a function
  (D8).
- The temporal survey and `reverification.md` predate the translator move.

## Corrections made after the gap-closing round

The four review files i to l repaired these points of the red-team round:

- The membership-bounds contradiction in Q35 was not established. R5 and
  section 10 of ticket 29 describe different things (l-file L5).
- Q22 is the complete mixed-value sort key, not only the unit tie-break
  (l-file L6). Group keys project once to the canonical class key with no
  zone lookup (l-file L7).
- The `[date]` merge order is inherited from ticket 22, not open (l-file
  L8). A local-only week setting is a proposed exception.
- The temporal policy carries no loaded zone (l-file L9). Q28 and Q29 no
  longer assume eager acquisition.
- Filename prefixes stay ISO under a configurable week (l-file L10).
- A tolerance release for deleted config keys is new scope (l-file L12).
- The Q13 grammar table is drafted below as a proposal.
- The citation audit corrected 23 claims in the digest and e-files (k-file).
  The affected spots above cite the k-file corrections.
- The direct Answer read corrected 15 ledger citations (l-file L14).

## Corrections made in the closure round

This repair round fixed these points of the sections above:

- Q4 presents lint-only as a proposed amendment to ticket 20's provisional
  scalar acceptance, not as an inherited behavior (l-file L1).
- Q13 writes Q17's function spelling, fixes the `now()` negative example,
  and aligns the clamp wording with the accepted arithmetic policy.
- Q13 separates adoption, rule-table semantics and crate choice, and marks
  the grammar unimplemented and completeness unproved (l-file L11).
- Q19 separates the payload measurement from the mirror enum's measured
  80-byte size. The implemented production carrier still needs a size
  assertion [V: [m-gap-closure.md](./m-gap-closure.md)].
- Q14 now has a four-candidate matrix, including real panics in three
  candidates. All four return points without periods or error spans.
  First/warmed allocations, same-day cases and Rust 1.96.0 builds are
  recorded in M; the old two-candidate observations remain historical.
- Q9 cites guarded field parity and N's source trace. No missing
  speedate datetime/duration/zone comparison remains.
- Q20 marks option B as an explicit revision of ticket 18's single-carrier
  rule (l-file L4, i-file L6).
- Q25 orders segment selection before decoding (k-file K22).
- Q26 names exactly three keys (l-file L13).
- Q29 separates the spelling check from zone acquisition (l-file L9).
- Q31 corrects the Dataview `sow` and `(sunday, 4)` claims (k-file K3,
  K4).
- Q35 removes a false reject condition about membership bounds.
- Probe references link [m-gap-closure.md](./m-gap-closure.md) instead of
  session-local artifact paths.
- The 2026-11-18 acceptance round adds the four-candidate instrumented
  closure and the N scope trace. Q14, Q15, Q16 and Q21 now cite executed
  runtime results; the S24-era two-candidate and source-only statements
  above stay as historical records of that round.

## Current verification and remaining gates

The current managed `mise verify` passes 3,975 unit, 41 integration,
104 E2E, 155 feature and 87 doctests; two pre-existing E2E tests stay
ignored [V: M]. Thirteen semantic help contracts replace brittle
snapshot-prose tests; eleven obsolete fixtures are removed.
CLI behavior is unchanged. The corrected hk effects produce a plan with
zero unknown effects; scoped hk, clippy, visibility and layer gates pass.
The failed/cancelled earlier broad run remains historical evidence only.
These checks validate existing code, not the proposed temporal redesign.

Only implementation-dependent gates remain for the human-selected design:

- Implement the adopted grammar and prove its rule-table coverage, periods,
  full consumption, checked arithmetic and error spans.
- Implement the carrier/frame and assert the actual size, traits,
  construction costs, format epoch and rebuild behavior.
- Check filename boundaries apart from frozen note parsing, and implement
  the selected dialect and notice contracts.
- Measure integrated startup, size, throughput and temporal-key costs.
  Prove residual equivalence per operator and row kind before `Exact`.

The human forks above remain open. Upstream access and universal limits
are not implementation gates: English maintenance history is inaccessible;
no finite run proves all Unicode, overflow, locale or ambient-state safety.
No new throughput measurement or full production integration exists.
N's narrow speedate scope must not be used as whole-parser redesign parity.

# Grilling guide for ticket 30

Session plan for
[Where do temporal, format dialects and calendar helpers belong?](../../issues/30-temporal-boundary-and-dialects.md).
Use it with [synthesis.md](./synthesis.md). This guide holds the order, the
forks and the questions for you. It does not resolve the ticket.

## How to run the session

1. Take one cluster at a time, in the order below.
2. For each cluster, read the recommendation. Name your disagreements.
3. Record one disposition per question: Decided, Inherited,
   Evidence-blocked or Deferred (ticket 36 rules).
4. Do not reopen an inherited rule without new evidence.
5. Close the ticket only after every numbered question has a disposition.

## Clusters, in order

### Cluster 1: frame, zone and configuration (Q26 to Q34)

Why first. Every other cluster reads the frame and the policy.

| Question | Status | Proposal |
| --- | --- | --- |
| Q33 frame payload | Inherited timing; proposed carrier [P] | Pin the clock and week rule at operation start. Acquire the zone at first need and retain success or failure. `OnceLock` is a proposed implementation choice. |
| Q34 ambient reads | Inherited (B38) | Pass the frame into filter arithmetic, template helpers and value conversions. Replace `now_instant()` sampling. |
| Q30 system zone | Inherited (B28) | One lazy resolution per frame. A host-held zone needs an explicit amendment to ticket 21. |
| Q29 configured zone | Decision | An IANA name or an explicit `+HH:MM` offset. A spelling check does not acquire a zone. You choose its timing: config load or first need. |
| Q26 `[date]` syntax | Decision | One flat table. Dotted keys make nested tables. |
| Q27 `DateConfig` fields | Decision | Three keys: `zone`, `week`, `filename_year_prefix`. |
| Q28 policy carrier | Decision | A new temporal-owned policy type. Name it `DatePolicy` or `TemporalPolicy`. It carries spellings, not a loaded zone. |
| Q31 week key | Human decision | Presets `iso` and `us`, with a free pair as the escape. Your week template lists Sunday first. |
| Q8 week-one rule | Follows Q31 | The `min_days` half of the preset. No rollover from an invalid week. |
| Q32 bare-year key | Decision | Whole-stem shape if your year notes are `2026.md`. A leading-year rule otherwise. |

Questions for you.

- Do your week templates need Sunday-first weeks?
- May a global config set the week rule, or only a local one?
- Do any of your file names start with a bare year that is not a date?
- Do your year notes use whole-stem names such as `2026.md`?

### Cluster 2: value shapes (Q19 to Q25)

| Question | Status | Proposal |
| --- | --- | --- |
| Q19 civil and instant | Decision | An inner Civil/Instant enum inside `DateTimeValue` [P]. The mirror measures 80 bytes, with a 24-byte temporal payload. The production carrier still needs a size assertion and any chosen compile-fail `Ord` check. See [m-gap-closure.md](./m-gap-closure.md). |
| Q20 period placement | Decision | `CalendarPeriod` in temporal, by value, not in `NoteFieldValue`. A query-only wrapper is an explicit revision of ticket 18's single-carrier rule, not an inherited application. |
| Q21 midnight gap or fold | Decision | Option D: project an instant to civil in the pinned zone. Total, and it matches the membership direction. All four candidate APIs error or split on the sampled midnight gap and fold anchors; the observed facts do not pick this policy [V: D T8, M]. |
| Q22 mixed sort key | Decision | Proposed class order: Civil, Instant, Period. Within each class: `(date, time)`, epoch, or `(start, unit)`. Proposed unit rank: Day, Week, Month, Quarter, Year. Ticket 29 already fixes this class order for group keys. |
| Q23 duration identity | Ratify | Keep the nominal rules. One year is 365 days. |
| Q24 arithmetic notes | Decision | Three lists: exact, caveat, undefined. |
| Q25 `date(link)` | Decision | Take the last raw segment. Remove the anchor and extension. Decode once. `%2F` does not become a segment boundary. A URL target yields `Null`. |

Questions for you.

- Which unit order do you want for equal period starts?
- Does instant-to-civil projection meet your ordinary-field comparison needs?
- Does `[[b]]` fold to the same comparison key as `[b](b.md)`?

### Cluster 3: dialects, translator, helpers, filenames (Q1 to Q7)

Why a cluster. Q1, Q2 and Q3 close together.

| Question | Status | Proposal |
| --- | --- | --- |
| Q1 to Q3 dialects | Human fork | Keep strftime for config and query. Settle the schema `date.format` dialect first. The probe verified the Moment week-token ambiguity. |
| Q4 format check | Decision | A load-time lint only [P]. This removes ticket 20's provisional value check (20:119-123). It needs an explicit amendment. Partial-date value acceptance stays open. |
| Q5 date keys in config | Decision | Delete the two unread keys. One change-table row. A tolerance release is new scope. |
| Q6 engine helpers | Decision | Keep thin adapters in the engine. The arithmetic already lives in temporal. |
| Q7 filename parser | Decision | A filename-scoped parser unit in temporal with its own token enum. The prefix grammar stays a listed decision. |

The fork. If your schema `date.format` hints stay in Moment tokens, the
translator stays in temporal (alternative Q). If you move them to strftime,
the translator moves to the template engine (alternative P). Ask for your
choice before any other question in this cluster.

### Cluster 4: parsing stack (Q9 to Q12)

| Question | Status | Proposal |
| --- | --- | --- |
| Q9 speedate | Decision | Remove the unsigned full-date field call [P]. The guarded probe found no difference on 4,635,370 inputs. Preserve guards and reason strings. [N](./n-parity-and-scope.md) confirms no missing datetime/duration/zone replacement comparison; carrier/frame acceptance stays separate. |
| Q10 winnow | Decision | Keep it. Jiff cannot report a prefix length. It stays in the lockfile through toml anyway. |
| Q11 logos | Inherited | Keep it. It serves the frozen grammars. |
| Q12 jiff | Ratify | Keep it. |

Gate. Add the missing `winnow` entry to the ticket 25 crate list when
speedate leaves (B61).

### Cluster 5: natural-language dates and keywords (Q13 to Q18)

| Question | Status | Proposal |
| --- | --- | --- |
| Q13 grammar | Decision | Adoption, rule semantics and crate choice are separate. The rule table is a proposal with no implementation or completeness proof. |
| Q14 build or buy | Decision | Build in the repository [P]. All four unmodified APIs fail the complete contract. English, fuzzydate and chrono-english have observed panics; GNU has none in the sample but exposes overbroad grammar and point-only output. All four return points, without native periods or error spans. [M](./m-gap-closure.md#four-candidate-acceptance-closure) records four-candidate behavior, allocations, sizes and Rust 1.96.0 builds. |
| Q15 bare weekday | Human decision | Default proposal: next occurrence after today. Add `next`, `last` and `this` prefixes. State the same-day rule. Runtime: the English parsers move strictly forward, fuzzydate and GNU include today; phrase times keep or reset the anchor time by crate [V: D T3, M]. |
| Q16 `next week` | Decision | A `CalendarPeriod` [P]. Derive it from the period that holds today. Point or period remains open for offset phrases such as `in 2 weeks`. Runtime: `next week` is a point in fuzzydate and GNU and an error in the English parsers; no candidate yields a period [V: D T1, M]. |
| Q17 keyword spelling | Decision | Add nullary `today()` and `now()` [P]. Keep explicit arguments for edge functions, as in `eom(today())`. A nullary `eom()` needs the separate Q17-B proposal. |
| Q18 `member_of` | Human decision | Pick a word. The reserved node stays Unsupported in this scope. |

Questions for you.

- Which weekday meaning do you expect: next, nearest or this week?
- Which word do you want for membership?
- Which phrase time policy do you want: midnight or retained anchor time?
  The executed month-end, same-day and midnight cases expose different
  native policies. They do not select yours.

### Cluster 6: pushdown and notices (Q35, Q36)

| Question | Status | Proposal |
| --- | --- | --- |
| Q35 probe bounds | Decision | Prove bounds for civil dates, civil datetimes and periods [P]. Keep instants Unsupported in this proposed first scope. Classify a mapping as Exact only after a proof for its operator and row kind. |
| Q36 `Null` date notice | Decision | No notice for an absent value. One notice for a malformed date-shaped value. A typed error for a zone failure. |

Gate. Each bound needs a written residual-equivalence proof before it
ships. The proof covers each operator and row kind separately.

## Disposition map, Q1 to Q36

One row per question. Status names the current state, not the outcome.
Human decisions stay open until you answer them. [P] marks a proposal.

| Question | Status | Human decision or implementation gate |
| --- | --- | --- |
| Q1 to Q3 | Human fork | You pick the dialects. Probe result: week tokens are ambiguous. |
| Q4 | Decision (proposal [P]) | Amendment to ticket 20's value-check contract. Gate: lint table with three format classes. |
| Q5 | Decision (proposal [P]) | Gate: record one change-table row. A tolerance release is new scope. |
| Q6 | Decision (proposal [P]) | Gate: move the week-rule reads behind the frame. |
| Q7 | Decision (proposal [P]) | Gate: filename parser with its own boundary tests, apart from the frozen note parser. |
| Q8 | Follows Q31 | You pick the preset list. Gate: new week arithmetic plus a test corpus. |
| Q9 | Decision (proposal [P]) | Gate: retain the length guard and error categories. Update ticket 25's crate list and lockfile notes. |
| Q10, Q11 | Decision, Inherited | Q10 retains winnow for prefix consumption [P]. Q11 retains logos for the frozen grammars. |
| Q12 | Ratify | Gate: re-check on any blocking d-crate finding. |
| Q13 | Decision (proposal [P]) | Choose whether to adopt the closed grammar. Gate: check rule-table completeness. Nothing is implemented yet. |
| Q14 | Decision (proposal [P]) | Evidence: [M acceptance closure](./m-gap-closure.md#four-candidate-acceptance-closure), D T1 to T10. All four have observed results. The selected product needs grammar, period, span and safety acceptance; no unmodified candidate passes. |
| Q15 | Human decision | You pick the weekday meaning and the same-day rule. |
| Q16 | Decision (proposal [P]) | You confirm period derivation. Open sub-choice: offset phrases as point or period. |
| Q17 | Decision (proposal [P]) | You confirm the spelling. |
| Q18 | Human decision | You pick the membership word. The reserved node stays Unsupported. |
| Q19 | Decision (proposal [P]) | The mirror measures 80 bytes. The implemented type needs its own size assertion, chosen trait checks and a format epoch bump. |
| Q20 | Decision (proposal [P]) | You choose the carrier. A wrapper needs a change-table row against ticket 18. |
| Q21 | Decision (proposal [P]) | Gate: pinned-zone projection in the frame. |
| Q22 | Decision (proposal [P]) | You confirm the sort key. Gate: no zone lookup in group keys. |
| Q23 | Ratify | Gate: keep the 365-day-year documents. |
| Q24 | Decision (proposal [P]) | Gate: glossary text at cutover, three lists. |
| Q25 | Decision (proposal [P]) | Open fold choice: `[[b]]` versus `[b](b.md)` comparison keys. |
| Q26 | Decision (proposal [P]) | Gate: flat `[date]` table with the three keys. |
| Q27 | Decision (proposal [P]) | Gate: `DateConfig` fields match Q26 and Q31. |
| Q28 | Decision (proposal [P]) | You name the policy type. Gate: carry spellings, not a loaded zone. |
| Q29 | Decision (proposal [P]) | You choose spelling-check timing. Gate: retain lazy acquisition without a UTC fallback. |
| Q30 | Inherited (B28) | A host-held zone needs an explicit ticket 21 amendment. |
| Q31 | Human decision | You pick presets or a free pair, and the config level. Gate: filename weeks stay ISO (21:170-179). |
| Q32 | Decision (proposal [P]) | You pick the bare-year shape. |
| Q33 | Decision (proposal [P]) | Gate: frame payload and typed zone-failure type. |
| Q34 | Inherited (B38) | Gate: replace the listed ambient reads. Retain one frame per operation. |
| Q35 | Decision (proposal [P]) | Gate: prove residual equivalence per operator and row kind before classifying a mapping as Exact. |
| Q36 | Decision (proposal [P]) | Gate: match the notice conditions to ticket 34's mechanism. |

Human decision and Human fork mean that you choose the outcome.
Decision (proposal [P]) means that the synthesis proposes an outcome for
your approval. Inherited identifies an accepted rule. Ratify proposes
retaining current behavior; it does not claim prior acceptance.

## Corrections to earlier preparation

These corrections change research claims, not accepted Answers:

- Ticket 29 already fixes the group-class order: Civil, Instant, Period
  (B65). The earlier undefined-order claim is withdrawn.
- The project disables Jiff default features and explicitly enables
  `tz-system` (`Cargo.toml:128`).
- `apply_offset` arithmetic already lives in temporal. Adapter ownership
  remains a separate question.
- The L report corrects 15 ledger citations. Read the accepted source
  text when a research summary and its source disagree.
- M records the four-candidate acceptance closure and broad verification
  pass. N proves the narrow speedate call scope. The layout remains a
  mirror measurement; product gates and human choices remain open.

## Current evidence and remaining limits

The 2026-11-18 closure executes all four candidates at Rust 1.96.0.
The 64 shared cases, two English lenient cases, four baselines and 36
host variations produce 298 JSON records with first/warmed native calls.
All seven same-day weekday anchors execute. Selected explicit-anchor/zone
results stay invariant under `TZ`, `LC_ALL` and `TZDIR` variations.
First-call/warmed allocations and whole-binary sizes are in M.
The instrumented unwind profile is not the product abort profile.
Earlier indicative timings and linkage costs remain historical evidence;
no new throughput measurement or complete integration cost exists.

The current managed `mise verify` passes 3,975 unit, 41 integration,
104 E2E, 155 feature and 87 doctests; two pre-existing E2E tests stay
ignored. The old failed/cancelled run is historical. Corrected hk effects
leave zero unknown plan effects; scoped hk, clippy, visibility and layer
gates pass. These results do not validate the proposed redesign [V: M].

After the human chooses the design, these implementation gates remain:

- Implement the grammar, periods, full consumption, arithmetic and spans.
- Implement the carrier and frame. Assert the production size, chosen
  traits and format epoch; the 80-byte mirror does not pass these gates.
- Check filename boundaries separately from frozen note parsing.
- Measure integrated startup, size, throughput and temporal-key costs.
- Prove residual equivalence per operator and row kind before `Exact`.
- Implement the chosen dialect and notice contracts.

The human forks remain open. English's advertised upstream is inaccessible
(404); public repository/commit searches find no alternative. Its package
VCS SHA is not maintenance history. No finite sample proves arbitrary
Unicode, overflow, locale or transitive ambient-state safety. Fuzzydate's
older zero-failure Unicode sample remains valid, but the new count-overflow
panic disproves general safety. GNU's zero sampled panics proves no universal
claim. N rules out a missing speedate datetime/duration/zone experiment,
not the broader carrier/frame acceptance work. All four build at the
repository floor, so no candidate MSRV test gap remains; upstream promises
about undocumented minima are untestable facts, not checks owed.

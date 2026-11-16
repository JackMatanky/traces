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
| Q33 frame payload | Inherited (B28) | Clock and week values at operation start. Zone cell acquired at first need, memoized with any failure. OnceLock, no ambient reads. |
| Q34 ambient reads | Inherited (B38) | Pass the frame into filter arithmetic, template helpers and value conversions. Replace `now_instant()` sampling. |
| Q30 system zone | Inherited (B28) | One lazy resolution per frame. A host-held zone needs an explicit amendment to ticket 21. |
| Q29 configured zone | Decision | An IANA name or an explicit `+HH:MM` offset. The validation time is your choice: config load or first need. |
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
| Q19 civil and instant | Decision | An inner Civil/Instant enum inside `DateTimeValue`. Measured payload about 24 bytes, inside the 80-byte cap. Size test, no-cross-class-`Ord` test, epoch bump. |
| Q20 period placement | Ratify | `CalendarPeriod` in temporal, by value, not in `NoteFieldValue`. Record any query-only wrapper as an explicit change to ticket 18. |
| Q21 midnight gap or fold | Decision | Option D: project an instant to civil in the pinned zone. Total, and it matches the membership direction. |
| Q22 mixed sort key | Decision | The complete key: class first, then `(date, time)`, epoch, or `(start, unit)` with the unit rank Day, Week, Month, Quarter, Year. No zone lookup. |
| Q23 duration identity | Ratify | Keep the nominal rules. One year is 365 days. |
| Q24 arithmetic notes | Decision | Three lists: exact, caveat, undefined. |
| Q25 `date(link)` | Decision | Decode once, take the last segment, strip anchors and extensions. A URL target yields `Null`. |

Questions for you.

- Which unit order do you want for equal period starts?
- Does a notice for an ambiguous midnight help you?
- Does `[[b]]` fold to the same comparison key as `[b](b.md)`?

### Cluster 3: dialects, translator, helpers, filenames (Q1 to Q7)

Why a cluster. Q1, Q2 and Q3 close together.

| Question | Status | Proposal |
| --- | --- | --- |
| Q1 to Q3 dialects | Human fork | Keep strftime for config and query. Settle the schema `date.format` dialect first. The probe verified the Moment week-token ambiguity. |
| Q4 format check | Decision | A load-time lint only. The accepted hint rule already covers values (20:119-123). |
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
| Q9 speedate | Decision | Remove now. The parity probe found no difference on 4,635,370 inputs. Keep the length guard and the reason strings. |
| Q10 winnow | Decision | Keep it. Jiff cannot report a prefix length. It stays in the lockfile through toml anyway. |
| Q11 logos | Inherited | Keep it. It serves the frozen grammars. |
| Q12 jiff | Ratify | Keep it. |

Gate. Add the missing `winnow` entry to the ticket 25 crate list when
speedate leaves (B61).

### Cluster 5: natural-language dates and keywords (Q13 to Q18)

| Question | Status | Proposal |
| --- | --- | --- |
| Q13 grammar | Decision | The synthesis drafts the rule table. Edit rows; do not design from scratch. |
| Q14 build or buy | Decision | Build in the repository. No candidate yields a period or a span error. |
| Q15 bare weekday | Human decision | Default proposal: next occurrence after today. Add `next`, `last` and `this` prefixes. State the same-day rule. |
| Q16 `next week` | Decision | A `CalendarPeriod`. Derive from the period that holds today. Offset phrases such as `in 2 weeks` are points. |
| Q17 keyword spelling | Decision | Nullary functions: `today()`, `now()`, and composition such as `eom(today())`. |
| Q18 `member_of` | Human decision | Pick a word. The reserved node stays Unsupported in this scope. |

Questions for you.

- Which weekday meaning do you expect: next, nearest or this week?
- Which word do you want for membership?

### Cluster 6: pushdown and notices (Q35, Q36)

| Question | Status | Proposal |
| --- | --- | --- |
| Q35 probe bounds | Decision | Prove Exact bounds for zone-free kinds only. Civil dates, civil datetimes and periods. Instants stay Unsupported. |
| Q36 `Null` date notice | Decision | No notice for an absent value. One notice for a malformed date-shaped value. A typed error for a zone failure. |

Gate. Each bound needs a written residual-equivalence proof before it
ships. The proof covers each operator and row kind separately.

## Corrections to ticket 30 text

Record these as supersession items. Do not rewrite an earlier Answer.

- Ticket 30 says nobody defined the temporal class order. Ticket 29 section
  3 defines it: Civil, Instant, Period (B65).
- The c-digest Q22 section repeats the same stale claim.
- `owner-evidence.md` said jiff used default features. The project sets
  `default-features = false` and enables `tz-system` by name
  (`Cargo.toml:128`).
- The e-file q6 lean says to move `apply_offset` arithmetic into temporal.
  The arithmetic already lives there.
- The l-file corrected 15 ledger citations. Use its cites, not the B ledger
  prose, for accepted text.

## What the preparation did not cover

- Binary size and run time for every crate option.
- Any temporal key timing.
- The owner's answers to the human questions above.

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
| Q33 frame payload | Inherited (B28) | Clock and week values at operation start. Zone cell acquired at first need, memoized with any failure. |
| Q34 ambient reads | Inherited (B38) | Pass the frame into filter arithmetic, template helpers and value conversions. Replace `now_instant()` sampling. |
| Q30 system zone | Inherited (B28) | One lazy resolution per frame. Reject a host-held zone. |
| Q29 configured zone | Decision | IANA name only, loaded once at config parse. |
| Q26 `[date]` syntax | Decision | One flat table. |
| Q27 `DateConfig` fields | Decision | `zone`, `week_first_day`, `week_min_days`, `filename.year_prefix`. |
| Q28 policy carrier | Decision | A new temporal-owned `TemporalPolicy`. The host passes it. |
| Q31 week key | Human decision | Pair `(first_day, min_days)` with ISO default. Your week template lists Sunday first. |
| Q8 week-one rule | Follows Q31 | The `min_days` half of the pair. No rollover from an invalid week. |
| Q32 bare-year key | Decision | One global opt-in key. |

Questions for you.

- Do your week templates need Sunday-first weeks?
- Do any of your file names start with a bare year that is not a date?

### Cluster 2: value shapes (Q19 to Q25)

| Question | Status | Proposal |
| --- | --- | --- |
| Q19 civil and instant | Ratify | Keep the outer `DatePoint` enum. Add a size assert and a no-`Ord` assert. |
| Q20 period placement | Ratify | `CalendarPeriod` in temporal, by value, not in `NoteFieldValue`. |
| Q21 midnight gap or fold | Decision | One documented mapping through the frame zone. A notice when it is ambiguous. |
| Q22 mixed sort key | Decision | Class order is fixed by ticket 29 (B65). Decide the period unit tie-break only. |
| Q23 duration identity | Ratify | Keep the nominal rules. |
| Q24 arithmetic notes | Decision | Three lists: exact, caveat, undefined. |
| Q25 `date(link)` | Decision | Read the raw link target text. |

Questions for you.

- Which unit order do you want for equal period starts?
- Does a notice for an ambiguous midnight help you?

### Cluster 3: dialects, translator, helpers, filenames (Q1 to Q7)

Why a cluster. Q1, Q2 and Q3 close together.

| Question | Status | Proposal |
| --- | --- | --- |
| Q1 to Q3 dialects | Human fork | Keep strftime for config and query. Settle the schema `date.format` dialect first. |
| Q4 format check | Decision | Classify each format token at schema load. The class sets the value check. |
| Q5 date keys in config | Decision | Delete the two unread keys. One change-table row. |
| Q6 engine helpers | Decision | Keep thin adapters in the engine. The arithmetic already lives in temporal. |
| Q7 filename parser | Decision | A filename-scoped parser unit in temporal. |

The fork. If your schema `date.format` hints stay in Moment tokens, the
translator stays in temporal (alternative Q). If you move them to strftime,
the translator moves to the template engine (alternative P). Ask for your
choice before any other question in this cluster.

### Cluster 4: parsing stack (Q9 to Q12)

| Question | Status | Proposal |
| --- | --- | --- |
| Q9 speedate | Decision | Keep it until the diagnostic parser lands. Remove it after. |
| Q10 winnow | Decision | Keep it. Jiff cannot report a prefix length. It stays in the lockfile through toml anyway. |
| Q11 logos | Inherited | Keep it. It serves the frozen grammars. |
| Q12 jiff | Ratify | Keep it. |

Gate. Add the missing `winnow` entry to the ticket 25 crate list. Run the
speedate differential probe before removal.

### Cluster 5: natural-language dates and keywords (Q13 to Q18)

| Question | Status | Proposal |
| --- | --- | --- |
| Q13 grammar | Decision | Closed list. One positive and one negative example per rule. |
| Q14 build or buy | Decision | Build in the repository. No candidate yields a period or a span error. |
| Q15 bare weekday | Human decision | Default proposal: next occurrence after today. Add `next` and `last` prefixes. |
| Q16 `next week` | Decision | A `CalendarPeriod`. Derive from the period that holds today. |
| Q17 keyword spelling | Decision | Function calls: `today()`, `eom()`. |
| Q18 `member_of` | Human decision | Pick a word. Keep `in` for ranges. |

Questions for you.

- Which weekday meaning do you expect: next, nearest or this week?
- Which word do you want for membership?

### Cluster 6: pushdown and notices (Q35, Q36)

| Question | Status | Proposal |
| --- | --- | --- |
| Q35 probe bounds | Decision | Prove Exact bounds for civil, instant and period kinds. Leave the rest Unsupported. |
| Q36 `Null` date notice | Decision | Notice only on silent coercions. |

Gate. Each bound needs a written residual-equivalence proof before it
ships.

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

## What the preparation did not cover

- The Leakage and Tests lenses produced no findings.
- Binary size and run time for every crate option.
- The speedate and jiff differential probe.
- The owner's answers to the human questions above.

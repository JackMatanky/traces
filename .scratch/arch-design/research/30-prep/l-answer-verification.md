# Direct Answer check for temporal preparation

## Scope and evidence

This report checks accepted text against `b-prior-decisions.md` and `synthesis.md`.
It does not supply a new Answer.
It changes no accepted Answer or production file.

[V] means a direct source read in this task.
[H] means an interpretation that the cited text does not settle.
[P] means a correction proposed for the preparation only.
No build, test, benchmark, formatter, linter or diagnostic script ran.
Earlier measurements remain reports from their original authors.

I read the Answers and later amendments in tickets 16, 18, 19, 20, 21, 22, 23, 24, 25, 28, 29 and 32.
Ticket 25 records its accepted decisions under `Comments`, including its later replacement rules.
I used those direct sections, not the B ledger, as authority.

I read these research sections only to trace changed claims:

- `h-redteam.md`, Q4: the proposed removal of the schema value check.
- `h-redteam.md`, Q20: the second runtime value carrier.
- `h-redteam.md`, Q26 to Q29: merge, key counts and the loaded zone.
- `h-redteam.md`, Q35: the claimed conflict about membership bounds.

I also inspected two code facts through CodeGraph.
`TaskDateSet` has six `Option<DateValue>` fields (`src/task.rs:498-505`).
`DEFAULT_DATETIME_PATTERN` equals `%Y-%m-%dT%H:%M:%S` (`src/temporal/datetime.rs:32`).
LSP references connect that constant to config defaults (`src/config/model/frontmatter.rs:261,296`).
These are source reads, not execution results.

## Direct-source coverage

| Answer | Direct section | Temporal rule or gap |
| --- | --- | --- |
| 16 | Answer and amended compatibility table | Vault parsing stays Frozen. Later accepted exceptions need change-table rows. Parity is Free. |
| 18 | Answer and supersessions by 19 and 20 | One runtime value serves notes, queries and templates. Keep the 80-byte test. Schema literals stay separate. |
| 19 | Answer, refinements and comments | Civil datetimes, adaptive literals, period matching, duration policy and Markdown-link questions bind the preparation. |
| 20 | Answer, handoffs and comments | The provisional date-format value rule is complete. A different paragraph contains the truncated fragment. |
| 21 | Answer | Frames, lazy system zone, typed failures, normalized periods and ISO filename weeks bind the preparation. |
| 22 | Answer | Config owns `DateConfig`. Zone precedence and field-by-field merge are already accepted. |
| 23 | Answer, amendments and code delta | Classification occurs once per element. Frontmatter wins in the effective accessor. Schema checks remain frontmatter-only. |
| 24 | Answer, later inputs and amendments | Derive filename periods during queries. Keep source timestamps. Prove candidate soundness independently of physical encoding. |
| 25 | Confirmed rounds, replacements, store identity and amendments | Fingerprints cover persisted producers. Civil values need an epoch change. Zones and unused filename rules stay outside policy fingerprints. |
| 28 | Answer and final review amendments | Host queries read a derived generation. Metadata timestamps remain from the base. Memo identity must include the frame and generation. |
| 29 | Answer and final review amendments | Group classes, R1 to R7, scoped notices and the reserved `member_of` node have distinct contracts. |
| 32 | Answer and final review amendments | Path values preserve listed spelling. NUL-form database keys and Comparison Keys serve different purposes. Percent decoding remains open. |

## Mismatches and minimal corrections

### L1. B26 is not the truncated schema fragment

Affected text: B26, I1, C19 and synthesis Q4.
The synthesis says a value check needs recovery of the truncated sentence first.
It recommends a token lint only.

[V] Ticket 20 gives a complete provisional value rule:

> `date.format` is a write-format hint: the check accepts Date/DateTime and any scalar if the format has no date component.

Source: 20, Answer, lines 119-123.
The same Answer hands the dialect and date-format check to ticket 30.
It also says no declared option stays silently ignored.
Sources: 20:416-417, 20:119.

[V] The truncated words occur in `Date-shaped text`, after the discussion of quote styles and quoted numbers:

> Only date, datetime and

Source: 20:305-315.
That fragment does not state a date-format rule.

[P] Keep the fragment as a separate recovery item.
Remove the claim that fragment recovery blocks the already delegated date-format check.
Present lint-only behavior as a proposed narrowing of ticket 20, not as its inherited requirement.
Do not complete the fragment by inference.

### L2. Partial dates and formats with no date component are different cases

Affected text: B26, synthesis Q4 and the Q4 argument inherited from `h-redteam.md:74-89`.

[V] Ticket 20 says “no date component”.
Ticket 30 asks about “no full date”, with `YYYY`, `Q` and `DDDD` as examples.
Sources: 20:120-123, 30:51-52.
The accepted rule does not equate these categories.

[V] Ticket 20 also says quoted numbers stay text.
Source: 20:311-315.
Thus, “a string-shape check would never see a string” is not established.
The written quoting and parser route matter.

[P] Separate full-date, partial-date and no-date-component formats.
Keep the provisional scalar acceptance rule only where its accepted condition applies.
Describe numeric examples as specific input spellings, not all possible field values.
Leave the partial-date acceptance rule for the human decision.

### L3. Frozen parsing has explicit accepted exceptions

Affected text: B1, B24, B37 and synthesis Q7/Q19.
These statements need one shared qualification.

[V] Ticket 16 freezes the meaning of vault files.
Ticket 19 expressly authorizes civil naive datetimes and inline `DateTime`.
It lists both as deliberate changes.
Sources: 16:76-77, 19:110-118, 19:232-246.
Ticket 18 records that supersession directly.
Source: 18:586-590.

[V] Ticket 21 limits its new week, quarter and year forms to filename extraction.
It does not authorize those forms in note fields.
Source: 21:170-176.

[P] State: preserve frozen classification except for explicitly accepted changes and their migration rows.
Keep the new filename grammar separate.
Do not use Frozen to reject the accepted civil change.
Do not use that civil change to authorize unrelated new note syntax.
The synthesis already recognizes the civil exception in Q19; carry it into the common constraints.

### L4. The period carrier alternative conflicts with the one-runtime-value rule

Affected text: synthesis Q20, B9, C9 and the Q20 question map.

[V] Ticket 18 says:

> `note::NoteFieldValue` serves notes, queries and templates.

Source: 18:458-461.
Ticket 20 places that runtime value below both note and schema.
Source: 20:85-87.

[V] Ticket 21 forbids a second period-value type and heap allocation for the period.
It requires normalized `CalendarPeriod` values.
Source: 21:136-141.

[P] Do not recommend a second query runtime carrier as an unchanged application of ticket 18.
Mark that option as an explicit change to the accepted runtime-value contract.
Adding one runtime period case does not, by itself, permit period syntax in persisted note fields.
Keep runtime representation and note-parser acceptance as separate decisions.
The unresolved variant list does not reopen the one-runtime-value rule automatically.

### L5. Membership bounds do not prove a conflicting physical key rule

Affected text: B66, B68, B70, synthesis Q18/Q35 and `h-redteam.md:508-510`.

[V] Ticket 29 R5 explicitly states:

> Point-period matching uses membership bounds. Period-period comparison uses normalized `(start, unit)` identity and order.

Source: 29:538-542.
R1 also requires the temporal key coverage to include period-valued rows.
Source: 29:519-530.

[V] Section 10 reserves `member_of` without a frontend or implementation.
It says its relation is never a range key or group key.
Source: 29:1186-1194.
Section 3 forbids membership as group equality.
It does not add the B66 sentence that membership “never defines a range key”.
Source: 29:716-730.

[H] The wording needs careful separation, but these reads do not prove a contradiction.
An equivalence/order key and operator-specific candidate bounds are different things.
A membership relation need not become a total key order to select a sound candidate interval.
R5 expressly anticipates such bounds.
It does not supply their proof or adopt a `member_of` probe.

[P] Remove the asserted contradiction and its reject condition from Q35.
Describe it as a terminology clarification if the human wants one.
Keep the reserved node Unsupported in the current scope.
For existing point-period comparisons, each operator and row kind needs the R1/R5 proof.
Never substitute period identity order for membership.

### L6. Temporal group class order does not settle every mixed-value sort key

Affected text: B65/C4, B67 and synthesis Q19/Q22.

[V] Ticket 29 section 3 defines temporal group classes and their order:

> The classes stay apart and order Civil, Instant, Period.

Source: 29:721-730.
The final review confirms that group equality is not every filter matching relation.
Source: 29:1519-1526.

[V] Ticket 21 separately requires a deterministic mixed temporal total sort key.
It leaves the period unit tie-break open.
Source: 21:149-156, 21:266-270.
Ticket 30 Q22 asks for that total key, including its unit tie-break.
Source: 30:97-98.

[P] Keep Civil, Instant, Period as the accepted group-key order.
Do not reduce Q22 to the unit tie-break alone.
State the complete proposed value-sort key and its relation to the group key.

Any change to the group-class order needs an explicit amendment.
Do not present ticket 29 as a universal ban on every inner temporal `Ord` implementation.
Ticket 19 expressly forbids `Ord` on `NoteFieldValue` itself.
Source: 19:87-102.

B65/C4 also names the wrong contradiction identifier.
The ledger's C3 owns group-class order; its C4 owns period upper-bound wording.

### L7. Group-key construction does not need a zone lookup

Affected text: B67's proposed repair “under the pinned zone”.

[V] Ticket 29 group keys use civil `(date, time)`, instant epoch nanoseconds and period `(start, unit)`.
They do not convert one class into another.
Source: 29:721-730.

[V] Ticket 21 forbids acquiring a zone for work solely on stored civil values.
Source: 21:123-127.

[P] Say “project once to the canonical class key”.
Keep the pinned frame for operations that need it.
Do not add civil-to-instant conversion to grouping.
Keep the reported DST comparator failure as earlier probe evidence, not a result from this task.

### L8. Config merge is inherited, not an undecided default

Affected text: B46, C17 and synthesis Q26/Q31.

[V] Ticket 22 says:

> Keep the merge. Apply the global config first, then the local config, field by field.

Source: 22:223-235.
The same section gives config ownership of `[date]` and fixes zone precedence.
It states no `[date]` exception.
Local trust-before-parse remains Frozen.
Sources: 22:225-233, 16:87.

[P] Present this merge as the inherited rule for `[date]`.
A local-only week setting is a proposed exception, not a missing accepted rule.
If the human chooses an exception, record its precise field scope and amendment.
Do not confuse the explicit host zone override with config-file merge order.

### L9. A loaded zone in the policy conflicts with the stated lazy design

Affected text: synthesis Q28/Q29/Q30/Q33.
Q28 says the temporal policy contains a loaded zone.
Q33 says the frame acquires the zone at first need.

[V] Ticket 21 pins one clock sample and week rule per operation.
It acquires the system-local zone only when needed and retains success or failure.
It prohibits a zone acquisition for civil-only work.
Source: 21:106-127.
Ticket 22 fixes the selected source order: host, config, system.
Source: 22:227-230.

[P] Distinguish zone spelling checks, zone-source selection and actual zone acquisition.
Do not justify two policy types by assuming an eagerly loaded zone.
Keep the configured-zone validation time as an explicit human choice.
Do not claim that rejecting malformed config necessarily equals acquiring a required runtime zone.
The chosen design must preserve civil-only work and avoid implicit UTC fallback.

Q30's “keep a loaded zone from workspace open” option needs an explicit change to per-operation acquisition.
It is not an equivalent implementation of the inherited frame lifetime.

### L10. Configurable weeks do not silently redefine accepted ISO filename prefixes

Affected text: C5, synthesis Q8/Q31 and its global-week risk note.

[V] Ticket 21 specifies validated ISO filename weeks with Monday start and four-day week one.
It separately mentions configurable first weekdays and leaves week formats beyond ISO open.
Sources: 21:170-179, 21:266-270.
It does not say that a configured general week rule reinterprets an ISO prefix.

[P] Remove the statement that the filename prefix obeys the configured frame week rule.
Keep ISO prefixes stable under a template or query week setting unless the human expressly changes ticket 21.
Ask separately which operations use a custom week rule and which spelling identifies a non-ISO filename week.
The note about global settings changing committed ISO labels assumes a change that is not yet accepted.

### L11. New grammar and membership syntax remain choices

Affected text: synthesis Q13/Q14/Q16/Q18.

[V] Ticket 19 says:

> Natural-language dates remain a hook, not a decided grammar.

Source: 19:167-170.
Ticket 29 adopts no new DSL syntax and reserves only the `member_of` node name.
Sources: 29:467-478, 29:1186-1199.
Ticket 21 permits new overlap or containment functions only when a query needs them.
Source: 21:149-152.

[P] Make grammar adoption a separate decision from grammar design and crate choice.
Keep the hook-only alternative valid without implying a missing implementation.
Make Q16 conditional on adoption of period phrases.
For Q18, distinguish a spelling reservation, a readable alias and a genuinely new relation.
Do not infer a required new implementation from the reserved name.

### L12. A release-long config tolerance path is new scope

Affected text: synthesis Q5.

[V] Ticket 16 makes config formats Migratable through the specification change table.
Ticket 22 keeps `deny_unknown_fields` and calls for comment-preserving migration at the first breaking change.
Neither Answer requires a one-release accept-and-ignore path.
Sources: 16:60-72, 16:86, 22:223-226.

[P] Remove the implied requirement for a tolerance release.
Compare a direct documented migration with any proposed tolerance path.
Label tolerance as more compatibility behavior for human approval.
Keep the chosen removal scope precise: the format key and the whole frontmatter date table are different changes.

### L13. Config key counts and diagnostic ownership overstate the direct Answer

Affected text: synthesis Q26/Q27.
Q26 says “four keys” but names the three keys listed in Q27.
It also attributes diagnostic ownership to ticket 22 without a direct source.

[V] Ticket 22 owns the config/parser boundary and delegates `[date]` syntax.
It does not decide temporal diagnostic fields.
Sources: 22:87-94, 22:223-233.

[P] Use the explicit field list without the stale count.
Remove the diagnostic-ownership attribution.
Treat any diagnostic setting as new scope with an identified consumer and owner.

### L14. Some B entries still use proxy evidence where a direct Answer exists

Affected text: B48, B49, B54, B55, B60 and the claimed B63/B64 coverage.

[V] B48 points to ticket 23 line 477, which concerns the task payload, not inline datetime classification.
Use 19:110-118 for inline civil datetimes.
Use 23:334-335 for element-wise classification.

[V] B49's six task-date slots are current code, not a new rule in ticket 23's Answer.
`TaskDateSet` confirms six civil slots (`src/task.rs:498-505`).
Keep that code fact separate from the accepted parser-preservation rule.

[V] Use 24:503-510 and 25:438-439 for zone-free cache decoding.
B54 currently includes a mislabeled link whose target is ticket 30.

[V] B55's UTC-key statement is research, not an accepted physical encoding.
Ticket 21 fixes source instants and evaluation-time civil projection.
Ticket 24 keeps compact temporal encoding conditional on measurement.
Sources: 21:162-169, 24:397-405.

[V] B60 can cite 25:180-184 directly.
That text excludes unused filename parsing rules from the fingerprint.

[V] The relevance table promises B63 and B64, but the ledger skips those entries.
Use ticket 28 directly for the missing temporal interaction.
A host overlay keeps base `mtime` and `created`; a new file has neither.
Source: 28:1070-1078.
The frame/generation memo rule is explicit in 29:965-978.

[P] Replace proxy citations with these direct owners.
Do not infer period dates from the buffer clock or invent metadata for new buffers.

### L15. The default-format disagreement is now closed by source inspection

Affected text: B's current-config paragraph and source-screening notes.

[V] `DEFAULT_DATETIME_PATTERN` is `%Y-%m-%dT%H:%M:%S` (`src/temporal/datetime.rs:32`).
Config defaults reference it (`src/config/model/frontmatter.rs:261,296`).
The stale `%Y-%m-%d %H:%M` research value is not the current default.

[P] Replace “check at implementation time” with this source fact.
Do not present the two earlier reports as equally current evidence.
No formatting execution ran.

## Invariants that the preparation must retain

1. **Note parser:** classify each field element once. Keep schemas out of classification.
   Preserve frozen meaning except for accepted changes and migration rows.
   Sources: 19:64-68, 23:334-341.
2. **Runtime value owner:** keep one runtime carrier across notes, queries and templates.
   Keep the 80-byte test. Do not add another period-value type.
   Sources: 18:458-489, 21:136-141.
3. **Value semantics:** keep structural equality, filter matching, group equivalence and total sorting distinct.
   Period membership does not define a partition or period identity order.
   Sources: 19:87-102, 29:1519-1526.
4. **Temporal:** normalize a period to `(start, unit)`.
   Project an instant to its civil date for membership.
   A skipped next day must not invalidate the prior day.
   Source: 21:136-156.
5. **Query and template:** retain one operation frame and its lazy zone result, including failure.
   Keep terminal failures typed. Never replace them with `Null`, blank cells or missing rows.
   Source: 21:106-132.
6. **Config:** own `DateConfig`, preserve trust-before-parse, and apply accepted merge and zone precedence.
   Temporal must not import config merely to get its policy.
   Sources: 22:223-235, 20:72-77.
7. **Index:** persist source timestamps, not zone-derived dates or filename periods.
   Decode civil datetimes without a machine zone.
   Sources: 24:397-405, 24:503-510.
8. **Fingerprint owner:** cover persisted producers and probe outputs.
   Add the civil case and epoch change. Audit `winnow` before declaring the producer list complete.
   Query-only grammar changes do not automatically become extractor changes.
   Sources: 25:304-334, 25:180-184.
9. **Planner and store:** prove matching-set equality for Exact probes.
   Run residual predicates. Scan Unsupported cases and merely Inexact cases in the first scope.
   Sources: 29:504-542, 29:571-583.
10. **Path owner:** read listed path text, not NUL-form database keys or folded Comparison Keys.
    Keep link percent decoding and encoded separators as explicit decisions.
    Sources: 32:463-470, 32:657-675, 32:872-874.
11. **Host and memo owner:** use one pinned base or derived generation with its frame.
    A changed plan, frame or generation needs a distinct memo identity.
    Buffer metadata does not create a new filesystem timestamp.
    Sources: 29:965-978, 28:1070-1078.

## Remaining human choices and evidence blockers

These items do not authorize changes to earlier Answers.

- **Schema semantics:** choose the dialect and define partial-date value acceptance.
  Recover the unrelated truncated classification fragment without guessing its missing words.
- **Civil/instant shape:** choose the field-level representation within the runtime-value and size constraints.
  The accepted civil behavior is not open again.
- **Period carrier:** complete the one-runtime-value model, or explicitly amend that contract before choosing another carrier.
  Do not make stored period syntax an accidental consequence.
- **Ordinary Date versus instant:** choose the gap/fold comparison rule separately from period membership.
  Ticket 21 does not settle that ordinary field comparison.
- **Total sort key:** specify the mixed-value key and period unit rank.
  Preserve the already accepted group classes and transitivity requirements.
- **Configured weeks:** choose the supported operations and week-one rule.
  Do not silently reinterpret ISO filename weeks through a general week setting.
- **Zone validation:** choose allowed spellings and the timing of spelling checks and acquisition.
  Preserve lazy operation-scoped resolution and civil-only work.
- **New language:** decide whether to adopt grammar or frontend syntax at all.
  Then decide phrase precision, weekdays, spelling and crate ownership.
- **Duration semantics:** retain or refine nominal seconds explicitly.
  Ticket 19 keeps `1mo == 30d` and `1y == 365d` provisionally.
- **Markdown links:** decide percent decoding, encoded separators and extension normalization.
  Decide `[[b]]` versus `[b](b.md)` equality separately from `date(link)` extraction.
  Sources: 19:299-313, 32:872-874.
- **Pushdown evidence:** no direct Answer supplies a temporal physical encoding or operator proof table.
  Key eligibility needs the full temporal bracket, including period rows, and the pinned frame.
  Quoted literals remain Unsupported in the accepted first scope.
  A timing for numeric or folder keys supplies no temporal proof.
- **Implementation evidence:** this task does not measure enum size, parser parity, key cost, grammar cost or shipped-profile behavior.
  Those are evidence gates, not permission to choose the human's semantics.

Only this new research report was written.
No existing file changed. No checks ran.

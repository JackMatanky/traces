# Premise audit for ticket 21 (round 2, critical redo)

Audits every upstream claim that [ticket 21](../../issues/21-ambient-environment-and-execution-context.md), its
inputs, and round-1 prep ([brief](./brief.md), a–g, [b-prior-decisions](./b-prior-decisions.md)) rely on. Each
premise is (1) quoted with source, (2) classified **R** (ratified: the user's own words exist, cited), **D**
(derived by an earlier agent; untrusted until verified), or **C** (contradicted by code, a primary source, or a
later user statement), (3) checked against the current tree or primary source, (4) traced to the round-1
conclusion that leaned on it, and (5) given a verdict: **KEEP**, **CHALLENGE** (needs the user's ruling in the
grilling), **WITHDRAW** (false, superseded, or not evidence).

User-quote key (all from `/Users/jack/.omp/agent/sessions/-Documents-41_personal-traces/`; session file, UTC
timestamp, JSONL row):

- **U1** 2026-10-18T22:07 (`...2026-10-18T19-58...`, row 504): "1. I am the only consumer, there is no
  published release, and no external Rust crate depends on the library 2. Yes. Vault-format parsing is Frozen.
  Dataview and Templater semantic parity is Free 3. Yes. Result equivalence with a fresh parse is Frozen.
  Everything about the file's layout, versioning and refresh strategy is Free"
- **U2** 2026-10-18T22:49 (same, row 574): "we must not reject widening visibility as a rule. the primary rule
  about visibility should be that we reduce visibility whenever possible and try to limit pub to components
  that are outward facing"
- **U3** 2026-10-18T22:46 (same, row 549): "I think I agree with all your suggestions, but q10 must not
  redesign everything as pub because this will be very hard to change once we move to a workspace layout"
- **U4** 2026-10-19T08:02 (same, row 586): "2. during this wayfinder, I think we should consider evolving the
  codebase to a workspace layout, starting with separating out CLI"
- **U5** 2026-10-19T09:25 (`...2026-10-19T08-17...`, row 74): "1. The LSP is the next big addition to the
  codebase, but the MCP planning will start shortly. ... 3. performance decisions must target a range of vault
  sizes ... 500 ... 50000 notes"
- **U6** 2026-10-19T10:03 (same, row 77): "1. Agreed, account for both LSP and MCP as likely consumers. The
  idea being that the codebase should be ready and flexible enough to easily enable adding MCP-specific seams
  and multi-workspace machinery"
- **U7** 2026-10-19T13:01 (same, row 89): "3. yes", answering the assistant's direct question
  "**Q3 – Should the core remain synchronous?** ... Do you want that as the default for both LSP and MCP?"
- **U8** 2026-10-20T12:39 (`...2026-10-20T08-53...`, row 114): "1. I am leaning toward agreeing to all
  recommendations, but you must stress test the recommendations..."; the recommendations included Q4's option
  (a) "the civil model" (assistant row 106: "➡️ (a), the civil model. I'm least sure of this one"), whose text
  contains: "A naive datetime stays a zone-free civil value. A `Date` compares as civil midnight. Only values
  written with `Z` or an offset are instants. Comparing a civil value to an instant ... takes an explicit zone
  from the execution context (ticket 21). The cache holds no machine state."
- **U9** 2026-10-20T16:32 (same, row 179; reinforced row 188 "i agree with the recommendations"): "I agree with
  the recommendations, but why not extend to integers and floats that are #.0? Also, natural language dates
  ... will have to be implemented at some point"
- **U10** 2026-10-20T20:33 (`...2026-10-20T17-35...`, row 360): "there has always been something about
  file_class_expander.rs that has never felt right to me and I cannot think of any codebase that designs and
  implements components according to this pattern. It almost seems like a lazy anti-pattern"
- **U11** 2026-10-20T19:53 (same, row 342): "...without this contradicting ticket 13's general decision to
  place note write-back out of scope"
- **U12** 2026-10-20T22:29 (same, row 499): "remember to consider that there is a chance we might move toward
  a workspace structure with crates, the first of which being the cli crate"
- **U13** 2026-10-20T22:30 (same, row 503): "if we move to a workspace structure, a composition/command/app
  layer is a possible design"
- **U14** 2026-10-20T22:39 (same, row 539): "1. in general, yes. however, once we move to workspace crates,
  cli will become one consumer of core for the rest, I am leaning toward agreeing", answering the layering
  recommendation; the assistant's read-back (row 546) added: "Host policy (the `ignore | warn | reject` map per
  code) lives in the consumers".
- **U15** 2026-10-21T23:04 (`...2026-10-21T22-07...`, row 110): "1. This seems like you justified a lot during
  the design phase on failible documentation and decisions, when you should have also challenged those
  decisions when relevant. it is better to identify a bad decision earlier rather than later 2. why would use
  cases only be functions with explicit inputs? this seems like an arbitrary or, much worse, a poor design
  choice I do not recall deciding. in almost every codebase I have ever reviewed, an app layer/composition root
  is primarily designed with objects and never only functions 3. 'Time is one (now, zone) value per query'
  seems like it would lead to extreme coupling between temporal/ and query/ 4. so what if CwdGuard is a bigger
  job? if it is correct, then it should be done"

Provenance note: a bundle "agreed"/"I agree with the recommendations" counts as R only for items named in the
recommendation text the user answered; items an assistant slipped into a later summary are D. Row references
above were read directly from the session JSONL.

## 1. Premise ledger

| # | Premise (quote → source) | Class | Code / primary-source check | Round-1 dependence → consequence if false or unratified | Verdict |
|---|---|---|---|---|---|
| P1 | "`Cwd`/`CwdGuard` (`src/cli/cwd.rs`, judged sound as a single audited boundary)" → [ticket 21](../../issues/21-ambient-environment-and-execution-context.md):16-18 | D: no user statement found; the only upstream is a scout note (P38) and the map's own unverified list (map.md:87) | `[V]` Cwd is the single production `env::current_dir` wrapper (cwd.rs:16-17; clippy.toml:93-94); `CwdGuard` is `#[cfg(test)]` only (cwd.rs:63-126) | b:59-63 and f:8 used "judged sound" to require "a concrete defect" before deleting; brief rev 1 leaned keep-H4 by migration cost. Unratified: soundness must be re-argued on correctness; user U15-4 removes cost as an argument | **CHALLENGE** |
| P2 | Ticket-21 Facts: "test HOME hack `src/dirs.rs:81-83`", "ZoneGuard (`src/temporal/mod.rs:58-255`)" → ticket 21:15-17 | D (facts stated in the ticket) | `[V]` HOME hack exactly at dirs.rs:81-83; ZoneGuard drifted: thread-local `test_support` at mod.rs:57-203 (`ZONE_OVERRIDE` :198-203), not :58-255; the old range matches the survey-era static lock that commit `0dce868d` removed | Round-1 inventory and every design file cite both; no conclusion changes beyond line hygiene | KEEP (facts; fix the range) |
| P3 | Alternatives framing: "explicit `ExecutionContext`/`Environment` passed down; split contexts (time/zone vs filesystem roots vs env policy); keep ambient with narrower guards. Include keep-as-is" → ticket 21:20-22 | D (question framing; predates the three-lifetime split in brief §0.3) | n/a (framing) | e/f/g each answered a different split of this menu; if the user prefers a coarser framing (one app object, U13), the menu reorders but no option is lost | KEEP as menu, not constraint |
| P4 | Value-model input: naive datetimes "zone-shifted at parse time and persisted in the postcard cache, so the zone input also affects index contents" → ticket 21:30-34 (from ticket 18) | C: superseded by [coercion and comparison policy](../../issues/19-coercion-and-comparison-policy.md):94-100; ticket 21:36-43 already corrects it | `[V]` today's code still does this (file.rs:88-98 + temporal/date.rs:178-181 persist a zone-derived `FileMeta.day`; note/metadata.rs:56-74 resolves naive input via `local_zone()`), so the *code* half is true and is the T6 gate | b-prior item 31 ([b:187-191](./b-prior-decisions.md)) calls this "the most consequential reversal"; any design that fingerprints zone into the index dies with it | WITHDRAW (superseded; kept only as the T6 gate fact) |
| P5 | "No comparison may read the clock or system zone" → ticket 19:42 (Answer: "No ambient zone or clock reaches any comparison", :58-59) | R: inside U8's Q4(a), which U8/U9 accepted | `[V]` today `NoteFieldValueRef::compare` promotes `Date` at ambient-zone midnight (note/field.rs:279-312) and `SortKey` sniffs text (e:129); the rule is a destination, not a description | The entire no-ambient-comparison invariant (T1, T5, guardrail tests) rests here; reversing it reopens ambient comparators | KEEP |
| P6 | "The context must supply the zone and `today`/`now` when sort keys and filter operands are built" → ticket 19:40-41 | R: same Q4(a) acceptance (U8) | `[V]` handoff to 21 recorded at ticket 19:200-201 | f/g evaluation-input plumbing depends on it; if reversed, query/template need no evaluation input at all | KEEP |
| P7 | "The cache holds no machine state" → ticket 19:98-99 | R: same acceptance (U8) | `[C]` vs current code (P4's code half); that contradiction is the owned T6 cutover (e T6; brief §0.1) | T6 gating ("cannot claim evaluation-only zone until cutover") is built on exactly this pair | KEEP |
| P8 | "A `Date` compares as civil midnight against civil datetimes" → ticket 19:96-97 | R: same acceptance (U8) | not yet implemented (current: ambient promotion, note/field.rs:297-300) | e T4 lean C "preserves old exact-midnight ordering" conflicts with this row; the midnight rule and P9's interval rule must be reconciled in the grilling (also N6) | KEEP (flag the T4 interaction) |
| P9 | Coarse date literals are intervals: "`"2025-01"` is the whole month and a day literal against a `DateTime` is the whole day. With `[s, e]` ... `==` is `s <= x <= e`" → ticket 19:126-128 | D: present in the Answer, but no user statement names interval semantics; U9's words cover natural-language dates and `#.0` literals only | not implemented | e T4 lean A (half-open day intervals) extends this same intuition; if the user never actually chose intervals over midnight-equality, T4 is a live semantics decision, not a handoff | **CHALLENGE** (cheap confirm) |
| P10 | "Decide the default zone" (and supply zone/now) handed to 21 → ticket 19:43, 200-201 | R: the handoff is part of the accepted Answer (U8/U9) | n/a | e T3's five options exist to answer it; no consequence | KEEP |
| P11 | Audience: "The user is the only consumer. No published release, no external Rust crate depends on the library" → [compatibility envelope](../../issues/16-grilling-compatibility-envelope.md):24 | R: U1 item 1 verbatim | consistent with baselines-as-detectors (`check:visibility`, mise.toml:123-146) | All round-1 files used it to size the change budget; if false, signature migrations (H4, T1) become contract breaks | KEEP |
| P12 | Tier table: Frozen / Migratable / Free surfaces → ticket 16:22,28-45 | R: U1 items 2-3 + U3 ("I think I agree with all your suggestions") | n/a | Trust-frozen (P13), output-frozen, and index-free rows each gate a round-1 lean (H8 path preservation; T6 cache freedom) | KEEP |
| P13 | "Trust gate behaviour (untrusted or stale config refuses to load; hash before parse)" is **Frozen** → ticket 16:40 | R: accepted inside the P12 table (U3) | `[V]` enforced by typestate + service build (config/file.rs per survey config-dialog.md:20; service.rs:118-136) | b:14-16 extended it to "config discovery cannot move"; that extension is **not** ratified (see N2); only the gate ordering is frozen | KEEP (gate); WITHDRAW the "discovery cannot move" extension |
| P14 | Visibility rule: "reduce visibility whenever possible and to limit `pub` to outward-facing components ... each `pub` item needs an outward-facing justification" → ticket 16:47 | R: U2 verbatim ("user, round 3, corrected") | repo mechanism: pub-token baseline (mise.toml:142-143) | g D-lean's "keep host aggregate private, `pub(crate)` inside core" and the 32 handoff both inherit this rule | KEEP |
| P15 | "Rust library surface ... **Free**; Baselines are change detectors, not a contract" → ticket 16:45 | R: U1 + U3 | `[V]` baselines regenerated per commit (survey cross-cutting.md:3,25) | Frees Cwd/env/zone signature migrations from baseline churn; no round-1 conclusion depended on the opposite | KEEP |
| P16 | "the LSP as the next major consumer" (committed) → [forward consumers](../../issues/17-grilling-forward-consumers-and-design-pressure.md):37-38 | R: U5, U6 | n/a | The single flip fact behind D0 (brief §2: "if the committed LSP does not need different roots ... A plus explicit evaluation input" wins); also e T1's rejection of thread-local scope | KEEP |
| P17 | MCP "likely near-term consumer"; build seams, not machinery → ticket 17:38-41 | R: U6 | MCP spec 2026-07-28 statelessness verified in ticket 20's round-6 work | H6 (host lifetime, per-request identity in MCP) is sized by this; overstating it would force speculative session machinery the user excluded | KEEP |
| P18 | "Keep analysis and domain logic synchronous by default. Async belongs at host or transport seams" → ticket 17:57-58 | R: U7 answers the direct question "Should the core remain synchronous?" with "3. yes" | `[V]` core is sync today (rayon inside index parse, sync query) | Used in b:20-21 and g to disqualify async-flavored contexts; if reversed, transport-seam shaping of evaluation inputs changes (minor for 21) | KEEP |
| P19 | "General note write-back is not a requirement of this redesign" → ticket 17:44-45 | R: U11 cites "ticket 13's general decision to place note write-back out of scope" | n/a | Keeps spans/edit-ranges out of the context's field list | KEEP |
| P20 | Performance points 50–50 000 notes → ticket 17:51-55 | R: U5 item 3, U6 item 3 | n/a | Bounds "long-lived host" scale arguments in H6/H7; no direct 21 conclusion | KEEP |
| P21 | Layering: "consumers (cli, lsp, mcp) are thin hosts of `core`. `core` holds an app layer of use cases that composes domain modules ... over foundation types" → [schema note typing](../../issues/20-schema-note-typing-and-validation.md):66-71 | R: U13 ("a composition/command/app layer is a possible design"), U14 ("cli will become one consumer of core") | app layer does not exist in code yet (`cli/` composes directly, cli/mod.rs:224-254) | g's topologies were all shaped to fit this paragraph; the app layer's *shape* (objects vs functions) is exactly P22 | KEEP |
| P22 | "Use cases are functions with explicit inputs, not a held context object" → ticket 20:72 | **D**: introduced by the assistant, not the user: "Fix: organise it by use case, each a function with explicit inputs and no held context" (assistant row 557, 2026-10-20T22:46), then a bundled "agreed" (row 559); **disputed**: U15-2 ("a poor design choice I do not recall deciding ... never only functions") | n/a (no app layer exists) | b:3-4,22-28 tagged it binding `[V]`; g:7 leaned topology D as its "direct fit"; e:5 cited it as a constraint; brief rev 1 used it to rule out topology B. Withdrawn in brief rev 2; U15-2 confirms the withdrawal: the real constraint is the god-object hazard, and object-shaped use cases (B/E/C) re-open pending ticket 30 | **WITHDRAW** as binding |
| P23 | "No one-implementor trait exists to let one module call another" → ticket 20:71 | D: same assistant sentence as P22 ("The rule becomes 'no one-implementor trait between modules'"); user evidence covers only the FileClassExpander instance (U10) | `[V]` the named instance smells: single impl, `Option<Arc<dyn>>`, out-param (ticket 20:152-166); but `DialogProvider` is also effectively one-production-impl (Terminal; Preset is test/MCP) | g used the rule to disqualify topology G's `InputProvider`; as a blanket rule it would also condemn future legitimate seams. Keep the instance, question the generalization | **CHALLENGE** |
| P24 | Host policy: "Host policy maps each code to ignore, warn or reject" (consumers own policy) → ticket 20:248 | D: assistant read-back row 546; covered only by U14's "for the rest, I am leaning toward agreeing" | n/a (nothing to check yet) | b:41-45 treats it as decided when separating check from context; low-contention but unconfirmed; confirm in passing during the grilling | **CHALLENGE** (pro-forma) |
| P25 | "Delete the trait [`FileClassExpander`], the crate-root `src/file_class_expander.rs`, the `&mut ClassExpansionMode` out-parameter and `Option<Arc<dyn FileClassExpander>>`" → ticket 20:152-166 | R: U10's own instinct plus accepted deletion | `[V]` seam still present (survey cross-cutting.md:36; file_class_expander.rs exists) | Kills the survey's "bridge crate" idea (P38) and confirms hierarchy-as-data at the app layer; 26 owns the query side | KEEP |
| P26 | Orphan rule: "the orphan rule forbidding this impl in any third crate once `query` and `schema` are separate crates (reproduced: E0117 ...)" → ticket 20:157-160 | D (in-ticket experiment; E0117 is a real, correctly described Rust rule) | not re-derived (out of scope; plausible) | Feeds 32's crate-topology constraints; no 21 conclusion | KEEP |
| P27 | Workspace steer: "consider evolving the codebase toward a workspace layout, **starting by separating the CLI** ... a direction to evaluate, not a decision" → [map](../../map.md):33-38 | R: U4 verbatim, reiterated U12 | n/a | g D-lean's "after CLI split, publish only outward-facing use-case inputs" and the 30/32 handoffs assume the direction; the map correctly keeps it non-binding | KEEP (direction) |
| P28 | Sequencing: "the CLI split depends on first deciding what the CLI owns today but should not (composition, scaffolding, query-source normalisation, task-filter synthesis)" → map.md:39-42 | D: map synthesis of survey cli.md H1/H2/H5/H6 (verified seams: cli/mod.rs:224-254 composition; normalize_source_input per survey) | partially `[V]` (composition and helpers in cli/) | Justifies leaving host/composition ownership to ticket 30 instead of freezing it in 21 | KEEP |
| P29 | "Provisionally withstand scrutiny (scouts, unverified ...): ... ADR-0004 CLI error isolation, `CommandOutcome`/`UserAbort`, `CwdGuard`, stdout/stderr split; acyclic 10-tier module DAG; layered test isolation" → map.md:80-90 | D: the map itself flags these unverified, and says ticket 33 re-tests them | CwdGuard: see P1; 10-tier DAG: `[V]` as description (survey cross-cutting.md:24) but not a constraint | Round-1 leaned on "layered test isolation" + nextest to argue ambient keep is testable (N11); treating the list as evidence was exactly the fallibility U15-1 condemns | **CHALLENGE** item-by-item |
| P30 | "a 2-3 crate split (core / test-support / cli) is the supported shape, a 10-tier split is not" → map.md:158-159 | D: map note from error-topology research | n/a | Constrains where a host aggregate may live (32); contradicts survey cross-cutting H1's 5-6 crate lean (P39) | KEEP as steer for 32 |
| P31 | Standing preferences: "adversarial and independent; evaluate 'keep as is' for every candidate; ... distinguish [V] verified, [H] hypothesis, [P] proposal" → map.md:29-32 | D (map Notes; consistent with U15-1's demand to challenge) | n/a | Method, not constraint; round-1's failure was applying `[V]` to artifact existence (P49) | KEEP as method |
| P32 | Survey H2: "Explicit `WorkspaceContext` replaces ambient static LazyLocks ... forces the `#[cfg(test)] static HOME` hack ... and blocks concurrent multi-workspace execution" → [config-dialog survey](../../survey/config-dialog.md):52-57 | D (scout hypothesis) | `[V]` defects real (dirs.rs:81-83; statics frozen at first read :62-190), `[H]` the multi-workspace block (true by construction for statics) | research 04 lesson 1 and f H2-lean B descend from it; the *alternative* (keep) was under-argued until brief rev 2 (H2 option C now credible: ConfigService::at + ConfigLocator::new exist, discovery.rs:51-61) | KEEP as hypothesis |
| P33 | Survey H4: "Threading `Arc<dyn DialogProvider>` ... into all 10 subcommands is dead parameter plumbing" → config-dialog.md:64-69 | D | `[V]` provider built in `run()` and forwarded (cli/mod.rs:224-228); of the dispatch arms only Init and Template consume it (cli/mod.rs:186-207) | f H5 lean (separate UI seam scoped to Init/Template) inherits it | KEEP |
| P34 | Survey H5: non-TTY `select` "always yields `0` ... renders `DialogError::NotInteractive` dead code" → config-dialog.md:70-75 | D | `[V]` terminal.rs:83-84 returns `Ok(0)`; NotInteractive never produced there | Headless-policy question (brief Q9, f H5) is grounded in this real behavior; ADR-0001's "fallback" framing hid it | KEEP |
| P35 | Survey temporal H2: per-leaf `local_zone()` "causes redundant OS `TimeZone::try_system` lookups" → [temporal survey](../../survey/temporal.md):54-55 | C: contradicted by Jiff's internal TTL cache ([research 04](../04-execution-context-precedent.md):47-50,124-130) | `[V]` local_zone() → `TimeZone::try_system` (zone.rs:19); cache is Jiff-internal | b item 33 ([b:198-201](./b-prior-decisions.md)) corrected this; any remaining perf argument for de-ambienting is unproven (brief §6); the case that stands is determinism + committed LSP | WITHDRAW the perf rationale; direction survives via P5/P6 |
| P36 | Survey I7: "`NoteFieldValueRef::compare` invokes `DateTimeValue::try_from(*date)` ... Core value comparison relies on process ambient `TZ`" → temporal.md:32,60-64 | D, verified fact | `[V]` note/field.rs:279-312; datetime.rs:341-344 | Ticket 19's relation redesign (P5) is the remedy; no open dependence | KEEP (fact) |
| P37 | Survey cli.md: "Centralized `Cwd` and `CwdGuard` ... effectively prevents process-global working directory race conditions" ("Withstands scrutiny") → [cli survey](../../survey/cli.md):112 | D (scout): and it verifies only *test-race* prevention, not that cwd-as-root is right for production or hosts | `[V]` mechanism (CWD_TEST_LOCK, cwd.rs:50-56), but the canonical runner already isolates tests per process (P56), which most of this rationale predates | This line is the **sole upstream of ticket 21's "judged sound"** (P1); the map itself lists CwdGuard as unverified (map.md:87). Withdraw as evidence; the correctness question returns to the grilling | WITHDRAW as evidence |
| P38 | Survey cross-cutting H6: `FileClassExpander` inversion "is sound and should serve as the blueprint" → [cross-cutting survey](../../survey/cross-cutting.md):80-83 | C: overturned by ticket 20 and U10 | seam deletion decided (P25) | Any topology argument still citing the bridge pattern (e.g. for query/schema separation) is dead | WITHDRAW |
| P39 | Cross-cutting H1: workspace decomposition "into 5-6 workspace crates" along the 10-tier DAG → cross-cutting.md:50-55 | D; as a steer it conflicts with map.md:158-159 (2-3 crates) | `[V]` only as a description of today's tiering | For 21 it matters only as "a context type's home is genuinely open (32)"; do not let tiercount logic pick the carrier type | **CHALLENGE** (belongs to 32) |
| P40 | Visibility freeze as invariant: "Every visibility token ... must match `tests/baselines/pub-tokens.txt`" → cross-cutting.md:25 | `[V]` as a repo mechanism (mise.toml:123-146); **C** as a design constraint: ticket 16 ruled the library surface Free (P15) | mechanism live | Round-1 cost arguments ("baseline churn") must not be used against signature changes | KEEP as mechanism; WITHDRAW as constraint |
| P41 | Research 04 lesson 1: "Replace ambient `LazyLock`s with an explicit `HostEnvironment` ... constructed once in `src/cli/mod.rs`" → [research 04](../04-execution-context-precedent.md):89-91 | D (comparable-derived recommendation) | `[V]` that the defect exists (P32); the *shape* is a menu item | f H2 lean B (env-map adapter) is its descendant; credible alternatives exist (promote `at`/`new`, f H2-C) | KEEP as option |
| P42 | Research 04 lesson 2: three tiers `HostEnvironment` / `WorkspaceContext` / `TemporalContext` / `UiContext` → research 04:92-97 | D | lifetimes independently re-derived in brief §0.3 (host/workspace/evaluation) | The brief's "three lifetimes" structure inherits it; sound as analysis, not ratified as types | KEEP as hypothesis |
| P43 | Research 04 lesson 3: "Inject `TemporalContext` into Query and Template pipelines ... Deletes the 200-line `ZoneGuard`" → research 04:98-101 | D; direction separately R via P5/P6 | `[V]` guard is ~150 lines in `test_support` (P2), "200-line" is drift | e T1's carrier options (value vs `Zoned` vs binder) are the live descendants | KEEP |
| P44 | Research 04 lesson 4: "Capture CWD once at the boundary ... Eliminates `CwdGuard` and `CWD_TEST_LOCK`" → research 04:102-103 | D | `[V]` cost surface: 29 guarded test functions across cli tests + `Cli::run`/`Commands::run`/Init/trust signatures (cwd.rs; grep of `CwdGuard` uses) | f:8 demanded "a concrete defect or test-value gain", a cost-flavored gate U15-4 rejects. Under nextest (P56) the guard's cross-test rationale is already stale; decide on correctness | **CHALLENGE** |
| P45 | Jiff caches system-zone discovery behind an RwLock with 5-minute TTL → research 04:47-50 | D, source-verified by research 04 against Jiff; consistent with jiff 0.2.38 pinned (Cargo.toml:128) | spot-consistent | Kills syscall-per-leaf arguments (P35); supports "explicit time for determinism/LSP, not speed" (brief §1) | KEEP |
| P46 | ADR-0001: lazy interactive functions through `DialogProvider`; `is_terminal()` lives in the provider, not the engine → [ADR-0001](../../../../docs/adr/0001-minijinja-interactive-functions.md):30-42,76-90 | D (accepted ADR from the original build; no user quote found) | `[V]` code matches (dialog/mod.rs trait; terminal.rs:43-108; preset provider) | f H5 and the "dialog is a UI seam" framing inherit it; ADRs are fallible context (map.md:23-24) but this one matches the user's LSP/MCP direction | KEEP |
| P47 | ADR-0002: trust before parse; symlink stores under the state dir; global config auto-trusted → [ADR-0002](../../../../docs/adr/0002-config-trust-and-tracking.md):56-71,105-116 | D as ADR text; the *behavior* is Frozen via P13 (R) | `[V]` service.rs build ordering; store roots from dirs.rs:162-190 | b item 26 ([b:151-156](./b-prior-decisions.md)) and the atomic-discovery argument (grant and load must not read different homes) rest on it; that argument survives regardless of where discovery *inputs* come from | KEEP |
| P48 | ADR-0004: one CLI presentation module; domain modules stay miette-free; `CommandOutcome`/`UserAbort`; exit 130 → [ADR-0004](../../../../docs/adr/0004-cli-error-presentation.md):35-45 | D; exit-code contract separately Frozen (P12) | `[V]` main.rs mapping per survey cross-cutting.md:14 | Grounds D0's "argv/stdout/stderr/exit are process-adapter items" (brief §2); zone/dir failure *mapping* goes to 31 | KEEP |
| P49 | ADR-0009 index refresh (fingerprint/meta/drift) governs context rules → [ADR-0009-index-refresh](../../../../docs/adr/0009-index-refresh.md):31-61 | D: status **proposed**, not accepted; implemented in code; **and the number is duplicated** (0009-filter-date-literals.md exists; map.md:23-24 flags it) | `[V]` both files exist; ticket 20:292 cites "ADR-0009" for "note must parse without schemas", which matches *neither* file's content | b item 34 ([b:202-207](./b-prior-decisions.md)) already warns it is "not an accepted source of new context rules"; T6/F6 correctly lean on tickets 19/24/25 instead. Fix the citation habit before spec-writing | **CHALLENGE** (as authority; behavior itself is real) |
| P50 | `clippy.toml` "bans only `current_dir`/`set_current_dir`" → [brief](./brief.md):81-83 | D (brief claim) | `[V]` clippy.toml:90-97; no disallowance for `env::var*`, `Timestamp::now`, `TimeZone::try_system` | The round-1 guardrail ("eliminated as a checkable invariant") extends exactly this list; if the list were wrong the guardrail would be misdirected (it is not) | KEEP |
| P51 | Index database is project-local `.traces/index.redb`, not under the state dir → brief:68-70 | D (brief claim) | `[V]` index/mod.rs:47-48 | Grounds "index path is workspace-derived input" (f Summary) and T6's per-workspace cache identity; also N3's question | KEEP |
| P52 | Two walkers share the global ignore set: index scan and trust subtree discovery → brief:72-74 | D (brief claim) | `[V]` index/service.rs:424-426 (`is_ignored_dir`); discovery.rs:153-159 | f H3's "one captured set passed to both walkers" and the `TRACES_IGNORED_DIRS` question (brief Q8) rest on it | KEEP |
| P53 | `QuerySet::filter` defers through a `OnceLock`; a borrowed per-call input does not reach that path → brief:74-76 | D (brief claim) | `[V]` results.rs:466-467, 526-528 (`cache: OnceLock<Arc<Vec<QueryRow>>>`) | Disqualifies topology F (scoped override) and forces owned/carried evaluation data (e T2 contract) | KEEP |
| P54 | `ConfigService::at` overrides only the two store roots; `load` and the static `trust_subjects` each build `ConfigLocator::from_env` → brief:70-72 | D (brief claim) | `[V]` service.rs:57-65 (`at`), :97-100 (`load` → `from_env`), :194-199 (static `trust_subjects` → `from_env`); `ConfigLocator::new` is `#[cfg(test)]` (discovery.rs:51-61) | f H2 option C ("promote existing seams") and the split-brain risk both depend on it; note `trust_subjects` being static means an `at()`-built service cannot redirect it, which strengthens the audit's point | KEEP |
| P55 | Ambient input scale: 24 production `local_zone()` sites (3 datetime, 1 point, 9 filter, 11 template date) and 3 `Timestamp::now()` sites, all template date → brief:59-61 | D (brief claim) | `[V]` recounted in this audit: datetime.rs:128,192,342; point.rs:780 (:506 is `#[cfg(test)]`); filter.rs:386,399,411,435,640,646,673,699,761; date.rs:121,132,184,254,346,449,474,697,799,928,987; clock at date.rs:162,1010,1026 | All carrier-cost estimates (T1's "9-11 frames", leaf-disposition table) hang on these counts | KEEP |
| P56 | Canonical unit/e2e test runner is nextest, one process per test (`cargo nextest run`; config comment "applies to every `cargo nextest run` (mise run test ...)") → mise.toml:32, .mise/tasks/test/unit:121, .config/nextest.toml:3-4 | D (never cited in round 1) | `[V]` this audit; plus `set_current_dir` forbidden in tests/ (mise.toml:182-187) | **New fact with design weight**: `CwdGuard`'s cross-test serialization rationale (cwd.rs:50-56, "concurrent changes race") is already moot for tests run under nextest; the guard's remaining value is within-test thread discipline and TempDir lifetime. Feeds H4/H7/T5 | KEEP (new) |
| P57 | e2e `Sandbox` gives child-process isolation: `env_clear` default-deny, sandbox-owned `TRACES_STATE_DIR`/`XDG_CONFIG_HOME`/`HOME`/`TZ=UTC`, child cwd → g:19 (citing support.rs) | D (g claim), now verified | `[V]` tests/e2e/support.rs:9-29, 247-296, 358-361 | Grounds "e2e stays the oracle" (H7 lean) and "keep ambient is coherent for CLI-only" (brief D2) | KEEP |
| P58 | Glossary: "Project Root: ... against which all file operations, template outputs, and queries are strictly confined. *Avoid*: working directory" → src/GLOSSARY.md:10-14 | D (glossary text describing intent) | `[V]` text; but the root is *discovered from* cwd (service.rs:97-100) and cwd is the default everywhere; the "avoid" is aspirational, and `RelativePath` confinement degrades to `PathBuf` in `FileMeta`/`CliError` (survey cross-cutting.md:27,44) | b:40 uses the glossary to justify naming root separately from cwd; the tension itself (confined-in-principle, cwd-derived-in-practice) is grilling material for 29 | KEEP with tension noted |
| P59 | `temporal` module doc ("Clock doctrine") documents ambient resolution: "Naive Date-Time input resolves in the process-local zone and is stored as UTC" → temporal/mod.rs:9-22 | D (current doc) | `[V]` | Shows ticket 19's decision *revokes documented current behavior*, so the change table must include it (ticket 19:180-188 already lists the cache-format change) | KEEP |
| P60 | Round-1 tag practice: b-prior-decisions marks "use cases take explicit inputs rather than a held context object" `[V]` on the strength of the cited artifact → b:3-4,22-28 | C as practice: `[V]` was defined as "you read the source and it says that", not "the artifact is ratified"; brief 0.1 ("Provenance discipline") concedes it | n/a | This is the mechanism behind the round-1 failure U15-1 condemns; the audit's R/D/C scheme replaces it | WITHDRAW (practice) |

## 2. Premises round 1 never examined

Round 1 argued options inside a frame it never stated. For each silent premise: the premise, the **opposite
design**, and whether the evidence rules the opposite out.

**N1. "A single process-lifetime host for the CLI means ambient is fine."**
Opposite: ambient is a defect even for one-shot CLIs, because (a) the unit-test `HOME` resolves differently from
production (`CARGO_MANIFEST_DIR/test` vs `$HOME`, dirs.rs:81-83 vs :62-63), so the resolution order is
under-tested in-process; (b) `LazyLock` freeze-on-first-read makes behavior depend on call order, which is
unobservable in review; (c) guards exist *because* ambient already hurts tests (P56 weakens the guard fix
itself). Evidence: e2e child isolation (P57) makes ambient *safe* for e2e; nothing makes it *equivalent*.
Verdict: not ruled out for production behavior; ruled out as "free of test cost". The grilling should ask
whether under-tested env resolution is a live defect (it is today: the hack masks precedence).

**N2. "The trust gate must stay before parse, so config discovery cannot move."**
Opposite: only the *ordering* is Frozen (P13); where discovery gets its inputs (env globals vs an explicit
locator value) is free. A redesign can hand `ConfigLocator` explicit ceilings/global-dir and keep
discover→hash→trust→parse byte-identical. Evidence: the gate lives in `ConfigService::build`/file typestate,
independent of `ConfigLocator::from_env` (service.rs:118-136 vs discovery.rs:43-48); ticket 16 froze
"behaviour", not inputs. Round-1 (b:14-16) used the premise to damp H2. Verdict: the premise as stated is
false; ordering-only survives.

**N3. "The index must stay project-local."**
Opposite: a state-dir index keyed by root hash (like the trust store) would let one process serve several
workspaces and share a cache. Evidence: ticket 16 marks `index.redb` layout/refresh **Free** (P12); nothing
ratifies the location. Counterevidence: project-local ties cache lifetime to the vault (delete vault, cache
goes), avoids cross-vault identity collisions, and matches the derived-cache doctrine (Durability::None,
store.rs:1259-1263). Ownership is 24/25; 21 only needs "index path is a workspace-derived input, not host
state". Verdict: open; not ruled out either way; do not let 21 silently freeze it.

**N4. "Dialog belongs to the host."**
Opposite: rendering is core and `ui.*` is a Frozen *capability* (ticket 16:42), so the strongest form (core
never prompts, returns an interaction plan the host executes) is a redesign of the template engine's lazy
callbacks (ADR-0001), not a context tweak. Evidence rules the strong form *expensive*, not wrong: minijinja
callbacks invoke the provider mid-render; a plan-returning engine changes the render contract. The weak form
(host chooses the provider; deliberate headless policy instead of silent `Ok(0)`, P34) is cheap and unratified
today. Verdict: weak form KEEP-worthy; strong form CHALLENGE (ticket 28's).

**N5. "Zone is an evaluation input, not an index input": therefore the store holds no zone-derived data.**
Opposite: materialize per-zone derived columns (e.g. `file.day` per zone, or day-bucket projections) in the
store so evaluation stays zone-free even faster. Ticket 19 (R, P7-P8) rules out zone-resolved values as the
*primary* representation ("cache holds no machine state"), but does **not** forbid a derived, keyed,
rebuildable projection, the same way nominal duration seconds are stored for ordering. Evidence: `cday`/`mday`
are per-row reads today (query/results.rs:274-298 per survey temporal.md:39); a civil+raw-instant store makes
them computable per evaluation at row cost. Verdict: the premise is right about identity, unexamined about
derived caches; the strong form ("no zone-derived store data at all") is an over-claim round 1 repeated. Give
the strong form to 24/25 explicitly.

**N6. "Date compares as civil midnight against civil datetimes."**
Opposite: whole-day interval (a day equals every instant in that local day), which ticket 19 *also* adopts for
coarse literals (P9), so the Answer holds two conventions whose interaction (day literal vs `Date`-typed row
vs civil-midnight equality) was never reconciled, and P9's own ratification is bundle-level. e T4 flagged the
behavior-change row but treated midnight as settled. Verdict: CHALLENGE both together in the grilling; cheap
to confirm, expensive to discover after spec.

**N7. "Sync core."**
Opposite: async core for a long-lived LSP with concurrent requests. This one *was* directly asked and answered
(U7), the only frontier-1 premise with clean ratification. Evidence for the opposite is speculative (MCP
concurrency). Verdict: KEEP; restate in the grilling as "sync core, async at seams" so the bundle is explicit.

**N8. "Schema/trust/clock are independent."**
Opposite: a configured default zone (e T3-C) makes clock policy a *config* concern, so trust-before-parse and
reload ordering (22) partly own the zone pipeline; conversely index refresh (25) treats config fingerprint, and
zone would ride config revision. Ticket 19 hands "default zone" to 21 *and* `DateConfig` coordination to 22/27
(ticket 19:196-199), so the premise is already half-false by the decisions' own text. Round 1 asserted
independence (b:41-45) while also planning config-zone handover. Verdict: replace with "schema checks are
clock-free (R); zone sourcing is a config/trust concern once configurable"; CHALLENGE the blanket wording.

**N9. "Cwd is an input at all."**
Opposite: every command takes an explicit project path (git's `-C`), and process cwd is only the *default*
when the flag is absent, or not consulted at all. Evidence: LSP/MCP hosts have no meaningful process cwd; the
trust command already accepts an optional PATH with cwd fallback (`trust_args(None)`, cli/trust.rs per its
tests:307-316); flag spellings are Migratable and new flags are additions (P12). Round 1 framed H4 as
"keep `Cwd` vs capture once", never as "root is an argument". Verdict: not ruled out; arguably the
*smallest* host input set (one `PathBuf` per invocation); grill it as its own row, decoupled from the
`CwdGuard` cleanup question.

**N10. "The CLI is the composition root" (is composition inside `cli/` itself a defect?).**
Opposite: composition lives in a library entrypoint (core's `run`/builder) and `cli` becomes a thin adapter,
which is what U12-U14 point toward ("cli will become one consumer of core"). Today's root is
`src/cli/mod.rs:224-228` plus private orchestration (survey cli.md H1), i.e. composition, scaffolding glue and
presentation in one module. Round 1 treated "composition root = cli::run" as a neutral fact; under the
workspace steer it is a *transitional* fact and the seam (who constructs `ConfigService`, the provider, and
the future host) belongs to 30. Verdict: fact KEEP; ownership CHALLENGE → 30, with the note that 21 should
define *inputs* so that the future composition site is swappable.

**N11. "Tests need process-based isolation."**
Opposite: in-process parallel tests with explicit inputs, no child processes at all, which is what removing
ambient state *buys*. The premise as stated is false in both directions: nextest already gives per-test
processes (P56), so ambient globals are *already* isolated across tests, and the guards serialize only
within-test threads; conversely e2e needs child processes regardless (the binary is the oracle). The real
choice is whether *unit* tests may mutate process state (today: only through `CwdGuard`/`ZoneGuard`/HOME-hack,
each with different semantics). Round 1 argued migration counts (29 tests, 98 sites), a cost framing U15-4
rejects, instead of asking what isolation is *for*. Verdict: WITHDRAW the premise; grill the purpose
(determinism? parallelism? parity with production resolution?) per guard.

## 3. Ranked: the eight premises whose falsity would change the answer most

1. **P22 (functions-only use cases)**: unratified and disputed (U15-2). Its withdrawal re-opens topologies B/E
   (an app object owning services) versus D's narrow arguments, changes every signature sketch in g, and
   determines whether 21 may even propose a `Workspace`-style type. Largest single swing.
2. **P5-P9 (civil values + evaluation-only zone/now cluster)**: if the user reverses to parse-time/config zone
   (ticket 19 Q4 option b, which the assistant itself called "much cheaper"), the entire temporal frontier
   (T1-T6, carrier, guardrail tests, T6 cutover) collapses into a much smaller decision. R, but bundle-level;
   the coupling objection U15-3 targets the carrier, not the rule.
3. **P16-P17 (LSP committed, MCP likely)**: the D0 flip fact. A CLI-only world makes keep-ambient-plus-guard
   the cheap sound answer; the committed LSP is what forces explicit acquisition and lifetimes.
4. **P13 (trust gate Frozen) read narrowly vs broadly**: broad reading ("discovery cannot move", b:14-16)
   cripples H2; narrow reading (ordering only) frees the locator redesign. Getting the scope wrong flips the
   host-input design.
5. **P1/P37/P44 (`CwdGuard` sound; delete-or-keep)**: with cost excluded (U15-4) and nextest isolation
   verified (P56), the keep lean loses its two supports; if correctness also favors explicit search start
   (LSP/MCP need it), H4 flips to capture-and-delete.
6. **N9 (cwd is an input at all)**: if commands take explicit roots, the host input set shrinks to
   "one path per invocation", and `Cwd`, the guard, and most of H4 dissolve. Never examined in round 1.
7. **P27-P28 + U12-U14 (workspace split with CLI as one consumer of core)**: determines where composition and
   the host aggregate may live, and whether a core-facing context type could ever be `pub` (it should not,
   per P14). If the user drops the split, a private CLI-side aggregate becomes permanently acceptable.
8. **P53 (`QuerySet` defers through `OnceLock`)**: quietly disqualifies scoped-override/thread-local designs
   (topology F, e T1-E/F) and forces the owned-or-carried evaluation-value contract. Any codegraph/source
   error here would silently re-admit a broken topology. (Verified, so listed last, but it is the premise
   with the least visible support in round 1's prose.)

Runners-up: P45 (Jiff TTL; already withdrew the perf rationale), P9/N6 (interval vs midnight semantics),
N5 (per-zone store projections).

## 4. Corrections to earlier artifacts found by this audit

- [brief](./brief.md) D3: "`IndexerService` holds its ignore set" is wrong as stated: the scan uses the global
  predicate `crate::env_vars::is_ignored_dir` (index/service.rs:424-426); g:15 says this correctly. No design
  changes; the E-topology framing ("services already own stable policy") must not cite it.
- [brief](./brief.md) §1 and ticket 21 Facts: `ZoneGuard` range `mod.rs:58-255` is stale; the guard is
  thread-local `test_support` at mod.rs:57-203 (`ZONE_OVERRIDE` :198-203). P2.
- [cli survey](../../survey/cli.md):112 ("Cwd/CwdGuard ... withstands scrutiny") is the *only* upstream of
  ticket 21's "judged sound", and the map itself lists `CwdGuard` as unverified scout material (map.md:87).
  The ticket's Facts line contradicted the map's own caveat. P1/P37.
- [cross-cutting survey](../../survey/cross-cutting.md) H6 (FileClassExpander "blueprint") is overturned
  (ticket 20; U10), still cited as a pattern in H1's crate-split logic. P38.
- b-prior-decisions `[V]` tags assert artifact existence, not ratification (P60); every `C`-marked item above
  was tagged `[V]` there or in the surveys it came from.
- Ticket 20:292 cites "ADR-0009" for "note must parse without schemas"; neither ADR-0009 file
  (filter-date-literals, index-refresh; both `proposed`, number duplicated) supports that sentence. P49.
- New verification with design weight: the canonical runner is nextest, one process per test (P56); round 1
  never checked it, and it weakens the cross-test rationale for `CWD_TEST_LOCK`/`CwdGuard` before any
  migration is counted.

Nothing in this file is a decision; it is the challenge layer the grilling should resolve question by question.

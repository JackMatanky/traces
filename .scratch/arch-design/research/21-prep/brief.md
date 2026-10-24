> **Superseded for design leans by [brief-r3.md](./brief-r3.md).** Read r3 first. This file (rev 1 plus 0.1) is kept as
> evidence; its section 2 leans, its section 5 grilling script and the rev-1 [V] tags on ratification are withdrawn.

# Pre-grilling brief: ticket 21 (execution context and ambient state)

Inputs: round 1 [a-code-inventory](./a-code-inventory.md), [b-prior-decisions](./b-prior-decisions.md),
[c-digest-lessons](./c-digest-lessons.md), [d-external-precedent](./d-external-precedent.md); round 2 (designs, written
independently) [e-design-temporal](./e-design-temporal.md), [f-design-host-inputs](./f-design-host-inputs.md),
[g-design-topology-and-redteam](./g-design-topology-and-redteam.md). Tags: [V] verified in source or document, [H]
inference, [P] proposal. Nothing here is a decision; it is the menu for the grilling. Format follows
[20-prep](../20-prep/brief.md). Names in sketches (`EvalTime`, `HostPaths`, `IgnorePolicy`) are placeholders.

## 0. Three things to know before the first question

1. **Ticket 19's destination is not the code.** [V] The index still persists a zone-derived `FileMeta.day`
   (`src/file.rs:66-98`) and resolves naive frontmatter datetimes to UTC instants on Rayon workers
   (`src/note/metadata.rs:56-72`, `src/field.rs:769-782`, `src/temporal/datetime.rs:559-570`,
   `src/index/service.rs:329-356`). The "evaluation-only zone" rule from [coercion and comparison
   policy](../../issues/19-coercion-and-comparison-policy.md) is therefore a target. Whatever 21 decides, it cannot claim
   the index is zone-free until a cutover (owned by tickets 19/24/25/26) lands. All three design agents and the
   inventory agree on this independently.
2. **Ticket 20's layering line is not a ratified constraint (revision 2).** [V] "Use cases are functions with explicit
   inputs, not a held context object" appears in the [schema typing and
   validation](../../issues/20-schema-note-typing-and-validation.md) Answer (:72), but its only upstream is
   [research 12](../12-iwe-core-and-hosts-architecture.md):277-279, which said "explicit functions **or thin structs**
   (`DocumentFinder`), not heavy context objects"; the Answer dropped "or thin structs". [H] I cannot find a place where
   it was decided that app layers are functions only. Earlier drafts of this folder (b, e, g, brief rev 1) treated it as
   binding and used it to rule out topology B and to favour "smallest-view arguments"; those leans are withdrawn.
   Whether the app layer is objects (a `Workspace`/`Session`-style type owning same-lifetime collaborators, methods as use
   cases) or functions belongs to [workspace host and composition](../../issues/30-workspace-host-and-composition.md), and
   21 must be answerable under either. The real hazard is narrower: a mega-context passed as a parameter to every leaf
   (service locator), which is different from an app object that owns services.
3. **There are three lifetimes, not one.** [H] Host (dirs, env policy: stable per process), workspace (project root,
   trusted config, stores: stable per config revision), evaluation (`now` + zone: fresh per query or render). One
   process-start snapshot cannot serve all three; rebuilding all per request entangles trust with time.

## 0.1 Revision 2: leans withdrawn or re-opened after review

- **Migration cost is not a design criterion.** Several "keep" and "smaller" leans below were weighted by churn: H4 (keep
  `Cwd`/`CwdGuard`), T5 (guard-based hybrid), H2 option C, D3 option D. [V] `CwdGuard` and its mutex are `#[cfg(test)]`
  only (`src/cli/cwd.rs:50-126`); production reads cwd once. It exists because commands below the CLI boundary depend on
  process-global cwd, and it protects only cooperating tests. [H] If an explicit search start is the correct design (and
  LSP/MCP need it anyway), it should be adopted and the 29 tests migrated. Re-grill H4 on correctness, with cost
  reported separately.
- **The temporal carrier sketch leaks.** [V] The sketch passes `{now, zone}` and filter code then reads `time.zone`
  (`shift(n, unit, &time.zone)`). Query and template already depend on `temporal` (query: `filter.rs` 13 references,
  `sort.rs` 1; template: `date.rs` 18), so the question is not whether they couple but what they know. If query leaves
  handle a zone, DST and `now`, adding week start, collation or a locale means a new parameter through every frame, which
  is how a context grows by accretion. [P] Alternative to grill: `temporal` owns an opaque evaluation object (zone,
  `now`, week start, interval rules) and exposes operations (`today()`, `shift`, `period_edge`, `day_interval`);
  query and template hold the handle and never see a zone. Cost: a temporal-owned type that must stay narrow.
- **Topology B and E are back on the table** pending ticket 30: an app object owning config, stores, index handle and
  ignore policy, with per-call evaluation input as an argument, is a conventional design and fits a long-lived LSP.
- **Provenance discipline.** [V] tags in b-prior-decisions confirm the text exists in an artifact, not that you ratified
  it. Items drawn from ticket Answers, survey "judged sound" notes and ADRs (including `Cwd` "sound", ticket 20's
  layering line, ticket 19's cutover) are inputs to challenge, not constraints.

## 1. Ground truth (current tree, HEAD `d6b63e0d`)

| Ambient input | Where | Scale |
|---|---|---|
| Env + dirs | `src/dirs.rs:57-198`, `src/env_vars.rs:6-45` | 3 direct `env::var*` expressions (nine keys + two `TRACES_*`); `LazyLock`s frozen at first read |
| Zone | `local_zone()` | 24 direct production sites (3 datetime, 1 point, 9 query filter, 11 template date) plus transitive reads in `NoteFieldValueRef::compare`, `DateTimeValue` `Display`, `DatePoint::instant`, `FileMeta::derive_day` |
| Clock | `Timestamp::now()` | 3 sites, all template date; the zone pin does not pin them |
| cwd | `src/cli/cwd.rs` | one production read; 29 guarded test functions |
| TTY | `TerminalDialogProvider` | probed per prompt; dialog Arc passed to 10 arms, 2 use it |
| Test cost | `ZoneGuard` 98 sites / 87 functions (thread-local since `0dce868d`), `CwdGuard` 29, HOME hack | e2e `Sandbox` already uses `env_clear` + child cwd + `TZ=UTC` |

Facts that change the discussion:

- [V] The index database is project-local (`.traces/index.redb`), not under the state dir. State dir holds only the trust
  and tracker stores.
- [V] `ConfigService::at` overrides only the two store roots. `ConfigService::load` and the static `trust_subjects` each
  still build `ConfigLocator::from_env`, so a "test" service still reads the real global config home and ceilings.
- [V] Two walkers share the global ignore set: index scan (`src/index/service.rs:422-440`, reached from
  `RefreshPlan::collect` as well as `build`) and trust subtree discovery (`src/config/discovery.rs:141-175`).
- [V] `QueryService::run` evaluates its plan eagerly, but `QuerySet::filter` appends a transform that runs on first read
  through a `OnceLock` (`src/query/results.rs:456-490,536-540`). A borrowed per-call input does not reach that path.
- [V] minijinja 3.0.1 `render_captured` builds its own `State`, so a per-render value cannot be seeded by a separately
  built `State`; the temporal sketch uses a reserved render-context object (cost: a reserved key).
- [V] Jiff `TimeZone::system()` returns `Etc/Unknown` on detection failure; that is not explicit UTC. Current
  `local_zone()` uses `try_system` and returns an error. Per-leaf lookups are cached by Jiff (5 min TTL), so the case for
  explicit time is determinism and the committed LSP, not syscall cost [V] (research 04, 11).
- [V] `clippy.toml` bans only `current_dir`/`set_current_dir`. It does not ban `std::env::var*`, `Timestamp::now` or
  `TimeZone::try_system`, so "eliminated" is not currently enforceable by lint.
- [H] Codegraph note: `codegraph_explore` via MCP pointed at a different project (`traces-pkm`) for two agents;
  `codegraph status/sync` from the worktree worked for one. Treat "no index" claims as tooling, and every code claim
  above as direct-read verified.

Corrections to earlier artifacts: [execution-context precedent](../04-execution-context-precedent.md) says delete
`CwdGuard` and `ZoneGuard` and use three tiers. The design agents find: deleting `CwdGuard` means migrating 29 test
functions plus `Cli::run`/`Commands::run`/Init/trust signatures, so it needs a concrete defect or test-value gain, not a
label. The ticket's old "zone affects index contents" input (from ticket 18) is superseded by ticket 19.

## 2. What the grilling must decide (decomposed; each is independent unless noted)

Dependencies: T6 gates claims, not decisions. Order: classification, then time and host-input frontiers together, then
guardrails.

### D0. Classification: what is "context" at all

Options: (A) keep implicit categories; (B) explicit values by lifetime; (C) one omnibus object; (D) explicit seams only.
[V] Clear process-adapter items under every option: argv, stdout/stderr, exit code, TTY probe. [P] Lean B's lifetimes with
D's narrow function inputs; host may keep a private aggregate (zk `Container` -> `ServerOpts`, marksman `State`).
Dissent: for a single-root, one-process CLI, A plus an explicit evaluation input is the cheapest sound answer. Flip: if
the committed LSP does not need different roots or policies in one process.
God-object tripwires if you want a checkable test (from g, C2): >5 input domains or 2+ lifetimes in one core-facing
object; consumers read under 25% of fields; mutation in flight; a filter test needs to mock trust or dialog; the object is
`pub` only because the CLI constructs it.

### D1. Time and zone (e-design-temporal)

| # | Decision | Options (lean first) | Best dissent / flip |
|---|---|---|---|
| T6 | Gate: when does the index become zone-free | A civil values + raw timestamps, stop storing `FileMeta.day` (lean, owned by 19/24/25/26); B zone threaded into index parse (temporary); C keep | C is sound only for one-shot same-zone CLI. Check: filename-date prefixes with offsets still need a zone |
| T3a | Default zone absent override | A system-local; C trusted config zone then system-local (lean); B UTC; D `TZ`-only; E CLI flag | A alone suffices for a single-workspace LSP. 22 owns key name, trust, reload |
| T3b | Zone unavailable or invalid | A error when a date-sensitive evaluation needs it (lean); B warn + explicit UTC; C silent unknown (reject); D require config | B if stripped containers must work. 31 owns error type and rendering |
| T3c | Week start | A fixed existing semantics (lean, verify what `sow` means today in `date.rs:51-69`); B/C weekday input from config; D locale | Real per-vault need flips it |
| T2 | Sampling scope | one snapshot per standalone query, one per template render shared with nested queries (lean); per host request; per process (rejected: stale after midnight); per row (rejected) | Per request if every request is one atomic evaluation |
| T1 | Transport | small `{now, zone}` value (lean) or bare `Zoned`; optional bind step owned by 26; trait clock (reject); scoped override or task-local (reject: Rayon, lazy `QuerySet`) | Narrow scoped guard is honestly fine for today's one-thread CLI; fails the LSP and ticket 19's no-ambient-comparison rule |
| T4 | Day boundary | half-open `[start_of_day(d), start_of_day(d+1))` with a fallible temporal primitive (lean); midnight + 24 h; current midnight point; pure civil | Current rule keeps exact-midnight equality. Interval vs point must be a behavior-change row, not a silent swap |
| T5 | Test migration | boundary-first hybrid with a hard removal gate (lean); full cutover; test-only `ClockGuard`; thread-local scope | Guard + `ClockGuard` is the smallest fix for isolated CLI tests |

Cross-decision [P]: sample `now: Timestamp` once, resolve one `TimeZone`, derive `today` from that pair (never
`Zoned::now()` after choosing a zone). Deferred sets either carry the resolved value or require one at materialisation.
A cached bound relative query in an LSP needs rebinding at midnight; fresh input cannot schedule an event (host duty,
tickets 30/34). Midnight, zone change and tzdb change deserve separate refresh policies.

### D2. Host inputs (f-design-host-inputs)

| # | Decision | Options (lean first) | Best dissent / flip |
|---|---|---|---|
| H2 | Representation of dirs and env | explicit resolved value built from an env-map adapter, private to the host, with narrow projections (lean, B); promote existing seams `ConfigService::at`/`ConfigLocator::new` and delete the globals (C, credible and smaller); gix-style `isolated()` mode; broad aggregate; keep `LazyLock` | Keep is coherent for CLI-only (nextest process isolation, no `set_var`). C if all production paths can share one policy without repeated resolution |
| H3 | Ignored-dir policy | one captured set passed to both walkers (lean); config-file key (circular trust bootstrap: reject unless `trust --all` gets a separate policy); walker-specific sets; ignore-file semantics (out of scope) | Keep the global if per-workspace indexing excludes are not a product need |
| H4 | cwd and project root | keep `Cwd`/`CwdGuard` in the CLI, protocol hosts supply a search start (lean, A+D); capture once and pass explicit start deleting the guard (29 test migrations); add `-C`/`--root` (distinguish "discovery starts here" from "this is the authorised root") | Migrate if same-process CLI test parallelism is worth the dispatch churn |
| H5 | Dialog | separate UI seam, scoped to Init and Template (lean); omnibus (reject); explicit headless policy for LSP/MCP (host rule, since non-TTY currently returns the first choice) | Keep dispatch-wide Arc if cleanup is distraction |
| H6 | Host lifetime | host snapshot plus workspace data replaced by explicit events, no process-global "active root", MCP identity per request (lean); rebuild every request; keyed workspace snapshots (measure first) | Per-request rebuild if requests are rare |
| H7 | Test determinism | inject full host policy in unit tests, keep e2e child-process oracle (lean); also migrate all cwd tests; never unsafe `set_var` (Rust 2024) | Subprocess e2e alone may suffice if in-process multi-workspace variation is out of scope |
| H8 | Path conventions | preserve current paths and variable spellings exactly (lean); directory crate swap (reject: `dirs` has no Mac state dir, `etcetera` defaults XDG on Mac, grants would vanish); explicit host-provided paths for protocol hosts | Only with a behavior-change row plus retrust |

### D3. Topology and guardrails (g-design-topology-and-redteam)

Seven topologies were costed: A keep, B universal context, C three immutable lifetimes, D smallest-view arguments with
host-private aggregate, E constructor injection, F scoped override, G capability closure. Leaning D as the app interface
with E for services that already own stable policy (`IndexerService` holds its ignore set; never `now` or dialog), C as
the strongest dissent. B contradicts ticket 20; F is disqualified by Rayon workers and lazy `QuerySet`; G reintroduces
per-leaf sampling. Costs: query path 9-11 frames, config path two constructor seams (not eight forwarding args), two
ignore walkers. Red team: strongest attack on D is parameter soup through eleven frames and the lazy-set snapshot
evaporating (verified: `QuerySet` defers). Spike either to settle. Strongest attack on keep: frozen env + thread-local
zone in a long-lived host; counter: only valid if the LSP is single-policy.

Guardrail ("eliminated" as a checkable invariant): extend `clippy.toml` `disallowed-methods` to `std::env::var*`,
`Timestamp::now`, `Zoned::now`, `TimeZone::system`/`try_system` outside audited acquisition modules; plus behavior
tests (two evaluations with different `(instant, zone)` in one process; the same unbound plan across local midnight; a
template with `date.today` and two nested queries sharing one time; a `QuerySet` materialised after its creator returns;
Rayon refresh producing zone-free records). Harness-independent. Caveat: alias or import escapes; dependency-owned reads
(Jiff, inquire, miette) are unaudited.

Reversibility [P]: cheap: private aggregate, provisional names, `pub(crate)` visibility, narrower `ConfigLocator`.
Moderate: threading eval value through the eager+lazy query and minijinja graph, both walkers, guard migration.
One-way (user-visible): changing path or trust ordering, treating unknown zone as UTC, a public cross-crate god context.

## 3. Handoffs (what 21 fixes, what it leaves)

| Ticket | 21 supplies | Left open |
|---|---|---|
| [config ownership](../../issues/22-config-ownership-and-lifecycle.md) | host discovery snapshot (homes, ceilings, ignore) as one atomic transaction; zone-config handover (resolved zone at evaluation entry) | loader API, trust revalidation, reload triggers, key names |
| [temporal](../../issues/27-temporal-boundary-and-dialects.md) | sampling unit, default and failure rule, who owns `now` | grammar, dialects, DST edge policy, `DateConfig` |
| [query core](../../issues/26-query-core-and-plan.md) / [template](../../issues/28-template-engine-boundary.md) | one value reaches eager plan, lazy `QuerySet`, nested queries, date callbacks | bind/rebind IR, minijinja carrier |
| [path model](../../issues/29-path-and-capability-model.md) | selected root distinct from cwd, passive not authority | capability or confinement |
| [workspace host](../../issues/30-workspace-host-and-composition.md) | input acquisition and lifetime rules | host type, service lifetimes, scheduling (midnight) |
| [errors](../../issues/31-error-and-diagnostic-architecture.md) | whether zone and dir failures return, warn or default | error types, codes, rendering |
| [crate topology](../../issues/32-crate-topology-and-public-surface.md) | concrete types crossing host to core | placement, `pub` surface |

Recommended handling of the triangle: grill 21 alone (who captures, when it changes, who receives), then a joint
interface check against 22 and 30 before any public type is frozen. Bundling 21+22+30 would ask for type ownership,
trust ordering and host lifecycle in one approval.

## 4. Questions only you can answer (the flip facts)

1. Does the committed LSP need several independently configured workspaces in one process (different roots, ceilings,
   ignore rules, zones)? This is the single assumption that most flips D, C and keep-as-is.
2. If the system zone cannot be found, should date-sensitive commands fail or run on a visibly warned UTC fallback?
3. Is a per-workspace timezone or week start a requirement now, or is system-local plus fixed Monday enough until
   someone needs more? Does a CLI flag deserve to exist at all (no evidence it does)?
4. Is one template render (including nested queries) one atomic time, even across midnight?
5. For a long-lived LSP, must date-dependent diagnostics refresh at local midnight, or only on the next request?
6. Is invoking the CLI from outside a project worth adding `-C`/`--root`? Is same-process test parallelism worth
   removing the 29 cwd guards?
7. Is date-only equality (exact midnight) vs whole-day interval a place where a user-visible behavior-change row is
   acceptable?
8. Should `TRACES_IGNORED_DIRS` stay a process policy shared by index and `trust --all`, or become per-project?
9. In a non-interactive host, should an interactive template reject, require preset answers, or fall back to defaults?

## 5. Proposed grilling script (frontiers; "yes" accepts one default, not the package)

Frontier 1 (ask together; independent):

- **F1. Classification and acquisition (D0, T1 of g):** Limit owned OS reads to one audited host acquisition seam, keep
  `Cwd` and the concrete dialog adapter, function inputs narrow, no held context? Default: yes (D-compatible). Flip: LSP
  is single-policy and CLI-only, then A plus explicit temporal input.
- **F2. Time sampling and carrier (T2, T1):** one `(now, zone)` per query, one per render shared with nested queries,
  value (not trait or scoped override), `today` derived from the pair? Default: yes. Flip: product wants live ticking
  time, then a separate explicit live operation.
- **F3. Zone default and failure (T3a, T3b):** system-local with trusted-config override, error on unavailable zone? Default:
  yes pending Q2 above. Flip: containers must work without zoneinfo.
- **F4. Host path representation and ignore policy (H2, H3):** resolved value from an env-map, private to host, one
  captured ignore set for both walkers, `ConfigService`/`ConfigLocator` constructed from it in one transaction (and
  `trust_subjects` made an instance method)? Default: yes; smaller credible alternative is promoting existing seams.
  Flip: concrete constructors already keep load/trust/scan on one policy.

Frontier 2 (after F1-F4):

- **F5. cwd/root (H4):** keep `CwdGuard`, protocol hosts supply search start? Default A+D.
- **F6. Index gate (T6):** is ticket 19's civil-value cutover a prerequisite for claiming evaluation-only zone, and which
  ticket owns the migration? Default: yes, owned by 19/24/25/26.
- **F7. Day boundary and week start (T4, T3c):** hand to 27 with the stated requirements (fallible half-open interval,
  fixed week until demand), or decide here? Default: hand to 27.
- **F8. Dialog and headless (H5):** UI separate; hosts choose no-prompt behavior deliberately? Default: yes, detail to 28/30.
- **F9. Lifetimes in hosts (H6, T2 rebind):** host snapshot fixed per process; workspace data replaced by explicit events;
  cached bound relative queries rebound at midnight by the host? Default: yes; detail to 30/34.
- **F10. Enforcement and migration (T5, guardrail, H7):** lint plus behavior tests, boundary-first migration with a
  removal gate, `CwdGuard` kept until evidence? Default: yes.
- **F11. Handoff review:** 21 independent, then joint interface check with 22 and 30.

## 6. Evidence gaps

- No compiled spike for D, C or E; signature deltas are counts and reasoned sketches, not measured diffs. A one-slice
  spike (one filter function + sort + a template filter) would settle D's parameter-soup attack.
- Nothing benchmarks startup or zone-lookup cost; Jiff's TTL cache means a performance case is unproven.
- Dependency-owned ambient reads (Jiff, inquire, miette) are unaudited.
- Digest evidence is snapshots; several comparables establish nothing about date queries (tagged "not established").
- Jiff day-boundary edge cases (São Paulo gap, Pacific/Apia skipped date, range limits) are for ticket 27 to validate.

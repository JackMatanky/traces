# Pre-grilling brief r3: ticket 21 (execution context and ambient state)

Supersedes the design leans in [brief.md](./brief.md) (rev 1 and 0.1). Rev 1 stays as evidence. Nothing here is a
decision. Tags: [V] I read the source, ran the code, or read the quote; [H] inference; [P] proposal. Upstream tickets,
ADRs and surveys are treated as input to challenge, never as constraint, unless the user's own words ratify them (see
[r2-premise-audit](./r2-premise-audit.md)).

Evidence base for this revision:
[r2-premise-audit](./r2-premise-audit.md) (60 premises, 11 unexamined),
[r2-defect-hunt](./r2-defect-hunt.md) (12 defects, invariants),
[r2-fact-audit-a-d](./r2-fact-audit-a-d.md) (92 claims re-verified),
[use-case-input-matrix](../../../../.design/2026-10-22-review-ambient-execution-context/scouts/use-case-input-matrix.md),
[invariant-ownership](../../../../.design/2026-10-22-review-ambient-execution-context/scouts/invariant-ownership.md)
(partial value, see section 7), and a compiled lab
(`.design/2026-10-22-review-ambient-execution-context/models/ctx-lab/`, 15 tests, all passing).

## 0. Process correction and provenance

- [V] The delegated agents run on a small model (`zai/glm-5.3-flash`) that also hit a provider usage limit mid-run.
  Ten lens scouts, one fact audit (e-g) and three of four architecture builders produced shallow or fabricated output
  and were discarded (kept in `.design/.../rejected/` for provenance). The 10-lens scouting step of the rust-design skill
  is therefore NOT complete; the lens objectives that mattered (seams, invariants, boundaries, tests, deletion) were
  covered by the premise audit, defect hunt, use-case matrix, and the lab instead. Accepted outputs are those whose claims
  I could spot-check against source.
- [V] A session "ponytail" mode was inherited by subagents and produced skipped checks; later prompts overrode it.
- The architecture comparison was done by me in a single compiled scratch crate, not by four independent builders. That
  trades independence for verified code, and means the lab reflects my design choices (see section 7).

## 1. What was wrong in round 1 (why this exists)

1. Upstream artifacts were treated as binding. [V] The premise audit found: ticket 20's 'functions with explicit inputs'
   was assistant-introduced (session row 557) and bundled into a later 'agreed'; the 'Cwd judged sound' note descends from
   one survey line; ticket 20:292 cites an ADR-0009 that matches neither duplicate file; 'discovery cannot move' stretched
   the Frozen trust ORDER to the locator's inputs.
2. Migration cost selected leans (CwdGuard, parameter spread). [V] `CwdGuard` and its lock are `#[cfg(test)]` only
   (`src/cli/cwd.rs:50-126`) and the canonical runner is nextest, one process per test (`mise.toml:32`,
   `.config/nextest.toml:3-4`), which removes most of the cross-test rationale before any counting.
3. The `(now, zone)` carrier leaked zone knowledge into query code. [V] Measured in the lab (section 4).
4. The real defects were never ranked; designs were argued from tidiness. They are below.

## 2. The real defects the design must answer (all [V], from the defect hunt)

| # | Defect | Live today? |
|---|---|---|
| D1 | A zone change is invisible to the index fingerprint (10 config digests only, `index/fingerprint.rs:37-38,142-143`); `refresh_store` serves persisted `FileMeta.day` and naive-datetime instants computed under the old zone | Yes, CLI |
| D2 | One query runs in two zones: `file.day` is persisted (old zone) while `file.cdate/mdate`, filter literals and sort promotion use the current zone | Yes, CLI |
| D3 | Zone-dependent code runs on rayon workers that the thread-local `ZoneGuard` cannot pin; no index test sets a zone; every e2e pins `TZ=UTC`; the non-UTC production surface is unverified | Yes (tests) |
| D4 | Unit tests are not hermetic: cfg(test) swaps only HOME; `XDG_*`, `TRACES_STATE_DIR`, ceilings, ignored dirs read the real env; a developer's real config can change a unit-test outcome, and in-process trust writes can hit the real store | Yes (tests) |
| D5 | A template render samples the clock 3 times and the zone 11 times; lazy query sets materialise later still | Narrow window |
| D7 | Non-TTY dialog silently picks the first choice (`dialog/terminal.rs:43-108`); a future non-interactive host inherits it | Yes |
| D8 | Unrepresentable local midnight becomes `Ordering::Less` / `SortKey::Null`, and filter functions use `local_zone().ok()?`, so failures vanish as non-matches (`note/field.rs:297-300`, `query/sort.rs:375-376`, `query/grammar/filter.rs:386-761`) | Exotic zones |
| D10 | Index scan and trust discovery agree on the ignore set only because both re-read the same globals | Hygiene |

Corrections to earlier claims (r2): `src/index/entry.rs:360-365` is test-only; the naive-datetime resolution lives in
`field.rs:769-798` + `datetime.rs:120-135`; the brief's 'dialog Arc passed to 10 arms' is wrong (forwarded to 2 of 10
arms plus two `-i` arms); Cargo.toml sets jiff `default-features = false` with `tzdb-bundle-always`, so named zones
already resolve without system zoneinfo (the residual risk is system-zone detection and bundle staleness); jiff's own
`try_system` doc says UTC fallback but the code returns `Etc/Unknown`; `IndexerService` does NOT hold an ignore set
(it reads the global predicate).

Consequence: [P] ticket 19's civil-value cutover is not an optional purity step. It is the fix for D1/D2, a live bug.
The interim alternative (put the derivation zone into the index fingerprint) is weaker but real.

## 3. The use-case matrix: what it says about objects versus functions

[V] (traced in [use-case-input-matrix](../../../../.design/2026-10-22-review-ambient-execution-context/scouts/use-case-input-matrix.md)):

- Every query-shaped use case (`list`, `table`, `task`, `index`, `template`) takes `&Config` and derives its services from it
  on demand. `Config` is already the de facto workspace object; `ConfigService` (state stores, locator) is the host-lifetime
  object. The existing app-shaped types (`ConfigService`, `IndexerService`, `QueryService`, `TemplateService`,
  `SchemaService`) are all objects. Ticket 20's functions-only line is out of keeping with the code it governs.
- [V] The composition `{index refresh, schema service, query service with class expander}` is written twice today:
  `cli/mod.rs:335-366,470-502` and `TemplateEngine::new` (`template/engine.rs:128-135`, `QueryOps::new`
  `template/engine/query.rs:113-124`). An LSP, an MCP host and the planned `check` would be the third to fifth copies. That
  duplication is the concrete argument for one workspace-lifetime object, independent of taste.
- [V] `ConfigService::trust_subjects` is static and builds its own `ConfigLocator::from_env`, while the instance holds
  injected stores (`config/service.rs:194-199`): a mixed-authority shape that an object owning one coherent host snapshot
  removes (lab test `s6_two_host_snapshots_and_trust_before_parse`).
- [H] The lifetimes that group stably: host (state homes, config home, ceilings, ignore policy), workspace (`Config` plus
  derived services, replaced wholesale on config or schema change), operation (project path, dialog policy), evaluation
  (now, zone, calendar policy). No current use case needs host and evaluation together.

## 4. What the compiled lab showed (ctx-lab, `cargo test`: 15/15 pass)

Shapes: A raw `{now, zone, week_start}` value in query code; B opaque `Frame` with operations; C bound plan
(`bind(&Frame)` then a param-free executor); D ambient thread-local scope. Shared temporal core: `compare_in`,
`add_days_in`, `promote_in`, `day_interval_in` over a zone source.

| Finding | Evidence |
|---|---|
| Opaque handle removes zone vocabulary from query code. `query_a.rs` names `TimeZone` 2x, `Timestamp` 2x, `Weekday` 2x, `to_zoned` 1x, `zone` 7x; `query_b.rs` and `query_c.rs` name none of `jiff`, `TimeZone`, `Timestamp`, `Weekday`, `to_zoned`, `zone` (test `coupling_tokens_in_query_modules`) | [V] |
| Adding a new per-evaluation input (week start): A puts a jiff `Weekday` field into the struct query code reads and applies; B adds a builder method to `Frame` and one `Frame` operation, query code calls `f.start_of_week(..)` (test `week_start_input_exercise`). Signatures do not change in either, so 'parameter soup' applies only if inputs are separate parameters; the real difference is what query code must know | [V] |
| Both carriers work through minijinja 3.0.1: a hidden root-context key `$frame` (not spellable as a template identifier; `{{ frame }}` renders empty) reaches callbacks via `State::lookup`, including `include` and isolated `import`; alternatively a lazily seeded `State` extension gives one sample per render but needs the engine to hold a sampler (tests `s5`, `root_key_carrier_vs_include_and_import`, `lazy_state_extension_carrier`) | [V] |
| A lazy `QuerySet` that captures its creator's `Arc<Frame>` is unaffected by whichever evaluation is active when it materialises; the ambient variant silently uses the reader's zone, or errors with no scope (test `s3`) | [V] |
| A thread-local scope is invisible on rayon workers (all 16 false); propagation needs a dedicated pool plus `broadcast` and teardown; an explicit frame needs nothing (test `s4`) | [V] |
| Zone resolution must be lazy and pinned (`OnceLock`) and operations must test nulls before touching the zone, or a zoneless host fails every query: my first `compare_in` forced zone resolution even when one operand was Null (found by test `s7`, fixed) | [V] |
| Zone failure becomes a typed error only for operations that need it; `Date` vs `Date` and null operands never touch the zone (test `s7`) | [V] |
| Held relative queries: with an unbound AST evaluated per frame, midnight is free; a bound plan is stale after local midnight and must be rebound (test `s2`) | [V] |
| `day_interval_in` returns `[03:00Z, +23h)` for Sao_Paulo 2015-10-18 and `SkippedDay` for Pacific/Apia 2011-12-30 (test `s8`) | [V] |
| `HostInputs::from_env_map` snapshots coexist; trust under one is invisible under the other; trust is bound to config content; an untrusted invalid config reports Untrusted and was never parsed; ceilings and ignore names are per-snapshot values (tests `s6`) | [V] |
| A source scan finds `current_dir`, `std::env::var`, `Timestamp::now`, `Zoned::now`, `TimeZone::system`, `try_system` only in `acquire.rs` (test `s9`) | [V] |
| `NonInteractive` dialog policy yields a typed error; `Preset` answers explicitly (test `s10`) | [V] |
| Shape C is B plus constant folding: the bound plan still holds an `Arc<Frame>` for row-dependent operations. It buys no coupling reduction over B, only folding and bind-time errors, and adds staleness and a second AST. It is an optimisation to decide in the query-core ticket, not here | [V]/[H] |

## 5. Revised decision frame (decomposed; each independent unless stated)

Order: R1 first (it changes what 'evaluation-only zone' means for today's code), then R2-R5 (temporal), R6-R8 (host),
R9-R10.

| # | Decision | Options | Lean and best dissent (both evidence-based) | Flip fact |
|---|---|---|---|---|
| R1 | Zone-derived data in the index (D1/D2) | (a) civil values + raw timestamps, stop persisting `FileMeta.day` (ticket 19 destination); (b) keep, add derivation zone to fingerprint (one rebuild per zone change); (c) keep as is | (a), because D1/D2 are live defects and (b) keeps two interpretations alive. Dissent: (b) is a small interim bridge. Ask also: filename-date prefixes with offsets still need a zone (e T6 disproof), and the ratified bundle behind 'civil' was the assistant's 'least sure' option (premise P5-P8); confirm | If the user prefers parse-time zone, the whole temporal frontier shrinks |
| R2 | What query and template code is given | (A) raw `{now, zone}`; (B) opaque handle with operations; (C) bound plans; (D) ambient scope; (E) keep | (B). It is the only compiled shape with zero zone vocabulary in query code and no worker/lazy hazards. Dissent: (A) if the user accepts query code knowing zones. C is a later optimisation | If the operation vocabulary grows beyond cohesive calendar semantics, split the handle |
| R3 | Sampling unit and held queries | per standalone query; per template render shared with nested queries; per host request; per process | One `(now, zone)` per query and one per render; a held query is re-evaluated, not cached bound, across midnight (S2); the host schedules midnight. Dissent: per request if every request is one evaluation | Product need for live-ticking time |
| R4 | Zone default and failure | system-local with error; UTC; configured zone; warn+UTC | System-local, resolved lazily, typed error from the operation that needs it, never `Etc/Unknown` as UTC; remove `.ok()?` row-dropping (D8). Dissent: warn+UTC for headless use. The config key belongs to the config ticket | Requirement that headless hosts run without a system zone |
| R5 | Date semantics and week start | midnight point vs whole-day interval for `Date` vs instant; week start fixed, configured, or locale | Needs your ruling: ticket 19 holds two conventions (civil midnight, and coarse literals as intervals) whose interaction was bundle-ratified only; keep week start out of the zone and note it is pure civil (`start_of_week_civil` needs no zone) | Whether you meant midnight-equality or whole-day |
| R6 | App-layer shape | free functions; workspace-lifetime object owning `Config` + derived services (use cases as methods, per-call evaluation); one context struct into every function | Workspace object plus a smaller host-inputs object plus per-call evaluation handle, because composition is duplicated 2x today and an LSP/MCP would add more. Dissent: functions for pure domain checks (schema `check`). Decision owner is workspace-host ticket, but 21 needs a working assumption | A use case that needs host stores and an evaluation input together |
| R7 | Host inputs and hermetic tests (D4, D10) | keep `LazyLock`; env-map snapshot object; promote existing seams (`ConfigService::at`, `ConfigLocator::new`) and delete globals | Env-map snapshot, one object for stores + locator + ignore policy, `trust_subjects` becomes an instance method, both walkers use the one ignore set; e2e stays the real-binary oracle. Dissent: promote existing seams if the snapshot adds nothing; neither dissent is cost-based | Whether one process ever needs two policies |
| R8 | Project selection and cwd | cwd default only; explicit project path everywhere with cwd as default (git `-C`); LSP/MCP supply paths | Explicit start path as the operation input, `Cwd` read only in the CLI adapter, and `CwdGuard` removed with its 29 test sites because under nextest it protects a race that no longer exists in tests; decide on correctness (cost reported separately: 29 tests) | A caller that needs process cwd below the CLI boundary |
| R9 | Dialog policy for hosts (D7) | keep silent first choice; explicit non-interactive policy | Explicit `NonInteractive`/`Preset` policy chosen by the host; terminal fallback visible. Ownership of the template contract is the template ticket | MCP template use that wants defaults |
| R10 | Enforcement of 'eliminated' | clippy `disallowed-methods` for `env::var*`, `Timestamp::now`, `Zoned::now`, `TimeZone::system/try_system`; source-scan test; both | Both; the scan test passed in the lab (S9) and does not depend on any harness. Caveat: dependency-owned reads (jiff, inquire, miette) are unaudited | A lint alias escape |

## 6. Questions only you can answer

1. R1: is a zone change allowed to leave stale persisted `day`/naive instants until the cutover lands, or should the interim
   fingerprint bridge go in first?
2. R5: should a `Date` equal exactly civil midnight, or the whole local day, against an instant?
3. R6: do you want the app layer (workspace object, host-inputs object, evaluation handle) as the working assumption for
   21, leaving types and crate placement to the workspace-host and crate-topology tickets?
4. R4: if no system zone is found, error or visible warned UTC?
5. R8: is an explicit project path (with cwd as default) the intended model for the CLI, given LSP/MCP supply paths?
6. Does one LSP process ever need several independently configured workspaces (different ignore sets, zones)? Only R7's
   scope depends on it now.

## 7. What is not verified (read before trusting section 4)

- [H] The lab models the shapes in miniature (about 1,400 lines): five value kinds, a six-variant `Expr`, no sort keys, no
  schema, no real index. It does not prove the real code migrates cleanly; `Frame` would need about 14 operations to cover the
  24 sites (today, now, civil-of-instant, date-of-instant, promote, compare, add duration/unit, diff, component, period edge,
  weekday move, format, parse-local, is_past/is_future). The real temporal module already has explicit-zone siblings
  (`_in(&zone)`; `src/temporal/datetime.rs:134-146`, `point.rs:183-243`), so the handle would be a thin binding of existing
  functions, but I did not count them against real signatures.
- [H] The lab was built by one author (me); a hostile reader should attack `Frame` as a hidden calendar service and the
  lazy `OnceLock` as shared mutable state. The deliberately failing alternatives (A, D) were also written by me.
- Not modeled: persisted index records and fingerprint (R1 is an analysis, not a compiled result), performance, real
  `QuerySet` generics, async hosts, LSP buffer overlay.
- The invariant scout is only partly trusted: its trust-gate section is verified; several rows say 'convention' where code
  or types enforce more, its 'trust and tracker roots must be distinct' item is a misreading (both are children of one
  state root), and row 20 admits it never read the `QuerySet` code. Use it as a lead, not evidence.
- Dependency-owned ambient reads (jiff, inquire, miette) and the md-pkm-lsp tickets (b items 27-29) remain unaudited.

## 8. Revision 4 (2026-10-22, after the user's reply to the first grilling round)

The user leaned toward agreeing with Q1 (several workspaces), Q2 (host-inputs snapshot), Q3 (explicit start path, delete
`CwdGuard`) and, cautiously, Q8 (dialog policy). These are recorded as leaning, to be confirmed at the handoff review.
Two corrections replace parts of this brief.

1. **Section 4's finding on shape C is withdrawn.** It said a bound plan buys no coupling reduction over the opaque
   handle. That tested a plan that still held a full `Frame`. The engine pattern (Spark `ComputeCurrentTime` plus
   `TimeZoneAwareExpression`, DataFusion `now()` simplification, Postgres volatility classes, DuckDB bind data) is a
   resolve pass plus a zone-only capability in the few nodes that need it. It is implemented as shape R in the lab and
   documented in [r3-engine-coupling-precedent](./r3-engine-coupling-precedent.md). R2's lean moves from (B) to (R), with
   (B) as the best dissent. Questions on sampling (R3) and zone failure (R4) are re-asked after the carrier is settled,
   because under R the failure point moves from the row to the resolve step.
2. **R1 was mis-stated.** `FileMeta.day` is not derived from `created_at` in the main case. The filename prefix wins
   (`file.rs:88-98`). `scan_date_prefix` accepts only `YYYY-MM-DD` and `YYYY-MM` (`temporal/lexer.rs:18-26`), producing a
   zone-free `DatePoint::Calendar` that already carries `DatePrecision::{Month, Day}`; `derive_day` then collapses it with
   `.date()`, so month precision is lost. The only zone-derived branch is the `created_at` fallback. Obsidian Dataview's own
   documentation says `file.day` is "only available if the file has a date inside its file name (of form yyyy-mm-dd or
   yyyymmdd), or has a Date field/inline field": it has no creation-time fallback. The code comment that calls the
   filename-first rule "the Dataview rule" is therefore half right. The round-1 'disprove A' worry about an offset
   datetime in a filename prefix does not apply: the prefix lexer accepts only date shapes.
   Consequence: D1/D2 trace to the fallback alone. The user also wants the field renamed `file.date` and extended to week
   (`YYYY-[W]WW`), month (`YYYY-MM`), quarter (`YYYY-[Q]Q`) and year (`YYYY`) prefixes.

### 8.1 Later additions (same session)

- Query-engine options (DataFusion, GlueSQL, Polars, CEL) are assessed in
  [r3-query-engine-options](./r3-query-engine-options.md). DataFusion's built-in Date versus Timestamp comparison promotes
  at UTC midnight regardless of session zone (probe `models/df-probe`), which differs from the local-midnight rule.
- User caveat on Q7b: a later `DateConfig` ticket will likely carry (a) the first day of the week (Sunday, Monday or any
  day) and (b) user-defined date formats per period, so that `file.date` can recognise other prefixes. Consequences for
  21: the week start is a civil, zone-free input supplied from config to the `WeekRule` capability, and the ISO Monday
  default in Q7b becomes only the default; filename-date parsing becomes config-dependent, so either the index fingerprint
  must cover the `DateConfig` digest or the date is parsed from the stored path at query time. Both go to the config
  ownership ticket and the `DateConfig` ticket.

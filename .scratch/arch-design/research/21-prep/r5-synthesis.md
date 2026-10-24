# Ticket 21: adversarial review before locking decisions

Status: historical stress test of tentative L1-L18; the subsequent
[ticket 21 Answer](../../issues/21-ambient-environment-and-execution-context.md#answer)
records the resolved decisions. [V] denotes observed source, executed
prototype or measured result; [H] denotes inference. Migration effort is
reported separately, never used as a design criterion.

## Independently verified counterexamples

1. **The compiled lab does not implement L13.** [V] `models/ctx-lab/src/temporal.rs:90-120` converts a `Date` to one instant (`promote_in`) then compares instants. It never checks membership of the proposed half-open day interval. Its passing tests and the shape benchmark therefore do not validate interval comparison. The scratch period prototype `/tmp/period-proto/src/lib.rs:142-163` models intervals, but has not been integrated with the query plan.
2. **The compiled lab does not implement L4's per-node capability wording.** [V] `models/ctx-lab/src/plan.rs:23-40` places `Arc<Calendar>` on the entire `Resolved` plan, not only on temporal nodes. This is likely a *better* baseline if one plan shares a capability: holding one `Arc` per plan does not require one clone per row. Distinguish a plan that owns calendar policy from an evaluator that may ask for the clock; the latter must remain impossible. A separately compiled temporal node is a potential later performance specialization, not a prerequisite.
3. **A lazy `QuerySet` cannot simply propagate L6's typed per-row errors.** [V] `src/query/results.rs:483-540` has an infallible `rows() -> &Arc<Vec<QueryRow>>`, called by `len`, `get`, `iter`, equality, debug and formatting. `QuerySet::filter()` merely appends a deferred transform. A fallible resolver plus `Result` row evaluator must either materialize before exposing these methods, change the materialization API to return `Result` and handle infallible trait implementations, or prove zone-dependent row operations error-free after resolution. Do not replace `.ok()?` with another silent false or `SortKey::Null`.
4. **Presentation is an overlooked ambient seam.** [V] `src/temporal/datetime.rs:297-306` implements `Display` by re-reading local zone and can return `fmt::Error`. `src/note/field.rs:317-359` ignores `write!` failures (line 337-339), so a DateTime cell can disappear silently; `src/query/value.rs:54-75` forwards it into list/table text. Formatting must receive the same resolved calendar or adopt a deliberately changed output contract; preserve existing stdout shapes. `Display` cannot express a typed zone error, so use a fallible query-owned formatter.
5. **A predecessor of a skipped local date can have a valid day interval.** [H, based on code] `/tmp/period-proto/src/lib.rs:142-163` and `models/ctx-lab/src/temporal.rs:51-69` call `first_instant(next_date)` and reject it if `next_date` is skipped. In Pacific/Apia, 2011-12-29 exists and 2011-12-30 does not: the interval for the 29th must end at the first instant of the 31st rather than fail because the nominal next date is missing. Separately decide whether a period that is itself skipped denotes an empty interval or a typed error. Boundary calculations must handle zone discontinuities and supported timestamp limits.
6. **Ticket 19's interval and field rules conflict with L13, not merely its prose.** [V] [the comparison Answer](../../issues/19-coercion-and-comparison-policy.md) lines 94-99 and 125-130 says Date-vs-civil is midnight, date *literals* are `[s,e]` inclusive, and field-vs-field remains midnight. L13 says a `Date` vs instant is a half-open interval. The old inclusive rule overlaps adjacent days at midnight. Preserve the independent *literal-vs-field* distinction until the user decides whether L13 also changes field-vs-field semantics; amend the resolved ticket only after an explicit ruling. Interval membership is not an equivalence relation over points: a Date can match two distinct instants which do not equal each other, so never implement it as `PartialEq`/`Eq` or use it as a grouping key.
7. **A configurable week start needs a week-one numbering rule.** [V] `/tmp/period-proto/src/lib.rs:60-88` takes both `first_weekday` and `min_days_in_week_one`; prototype tests compared ISO (Monday, 4 days) to jiff over 3710 year/week pairs. Sunday-first alone does not specify whether Jan 1 starts week one or the first full Sunday week does. Store a resolved period's civil start and precision rather than only `(week year, number)` when the rule may change, or include `DateConfig` identity in the index fingerprint. Bare `YYYY` and `YYYY-Www` prefixes in arbitrary filenames need explicit delimiter/ambiguity policy for other users' vaults.
8. **Benchmark interpretation must be bounded.** [V] The compiled Criterion lab measures 100k simple hash-map rows; A/B run generic filters while R+ is a bespoke closure for one `< Date` case (see `/tmp/ctx-lab-bench2/benches/eval.rs:79-148`). Therefore R+'s 1.89 ms vs R's 8.98 ms (America/New_York) shows *potential* constant-bound folding, not a validated 4.75x speed-up of a feature-equivalent production query. The R baseline also uses midnight rather than L13 interval. The 1M-row Rayon run measured 77.75 ms sequential versus 177.60 ms parallel in this fixture; attributing this solely to shared time-zone refcount contention is [H], not measured causation. Borrowed-zone conversion and representative production rows need an isolated follow-up comparison before claiming speedups. No benchmark changed production code.

### Complete lab benchmark results

[V] Criterion release means, with 95% confidence intervals, from `/tmp/ctx-lab-bench2/target/criterion/**/new/estimates.json`. These are prototype alternatives, **not production improvements**; R+ is not feature-equivalent to R for all temporal operators.

| Case | Mean | 95% CI |
|---|---:|---:|
| `compare/borrowed-zone` | 52.67 ns | 51.84–53.57 ns |
| `compare/folded-integer` | 1.51 ns | 1.50–1.53 ns |
| `compare/pinned-frame` | 53.78 ns | 53.23–54.37 ns |
| `civil/NY/A-raw` | 7.499 ms | 7.487–7.518 ms |
| `civil/NY/B-frame` | 7.793 ms | 7.786–7.801 ms |
| `civil/NY/C-bound` | 2.995 ms | 2.990–3.001 ms |
| `civil/NY/D-ambient` | 8.052 ms | 8.040–8.064 ms |
| `civil/NY/R-resolve` | 3.018 ms | 3.015–3.021 ms |
| `civil/UTC/A-raw` | 6.000 ms | 5.906–6.098 ms |
| `civil/UTC/B-frame` | 6.150 ms | 6.024–6.292 ms |
| `civil/UTC/C-bound` | 3.666 ms | 3.596–3.740 ms |
| `civil/UTC/D-ambient` | 5.844 ms | 5.615–6.112 ms |
| `civil/UTC/R-resolve` | 3.805 ms | 3.697–3.933 ms |
| `instant/NY/A-raw` | 11.776 ms | 11.634–11.938 ms |
| `instant/NY/B-frame` | 15.516 ms | 14.909–16.188 ms |
| `instant/NY/C-bound` | 9.034 ms | 8.833–9.246 ms |
| `instant/NY/D-ambient` | 13.873 ms | 13.613–14.165 ms |
| `instant/NY/R+-folded` | 1.887 ms | 1.842–1.934 ms |
| `instant/NY/R-resolve` | 8.980 ms | 8.755–9.221 ms |
| `instant/UTC/A-raw` | 4.488 ms | 4.482–4.496 ms |
| `instant/UTC/B-frame` | 4.601 ms | 4.598–4.605 ms |
| `instant/UTC/C-bound` | 4.218 ms | 4.214–4.223 ms |
| `instant/UTC/D-ambient` | 5.631 ms | 5.517–5.753 ms |
| `instant/UTC/R+-folded` | 1.612 ms | 1.588–1.637 ms |
| `instant/UTC/R-resolve` | 4.500 ms | 4.410–4.597 ms |
| `jiff/midnight-zoned/NY` | 55.41 ns | 53.72–57.15 ns |
| `jiff/midnight-zoned/Apia` | 14.69 ns | 14.66–14.72 ns |
| `jiff/midnight-zoned/UTC` | 7.82 ns | 7.65–8.01 ns |
| `jiff/start-of-day/NY` | 94.37 ns | 94.19–94.55 ns |
| `jiff/start-of-day/Apia` | 28.21 ns | 28.16–28.26 ns |
| `jiff/start-of-day/UTC` | 12.65 ns | 12.24–13.10 ns |
| `jiff/to-civil/NY` | 35.10 ns | 35.04–35.16 ns |
| `jiff/to-civil/Apia` | 7.29 ns | 7.26–7.34 ns |
| `jiff/to-civil/UTC` | 5.32 ns | 5.27–5.38 ns |
| `jiff/to-zoned/NY` | 36.16 ns | 36.14–36.19 ns |
| `jiff/to-zoned/Apia` | 8.89 ns | 8.88–8.89 ns |
| `jiff/to-zoned/UTC` | 5.54 ns | 5.52–5.56 ns |
| `jiff/zone-clone/NY` | 5.53 ns | 5.52–5.53 ns |
| `jiff/zone-clone/Apia` | 5.52 ns | 5.51–5.52 ns |
| `jiff/zone-clone/UTC` | 0.78 ns | 0.77–0.79 ns |
| `rayon/1m/parallel` | 177.604 ms | 172.613–181.982 ms |
| `rayon/1m/sequential` | 77.748 ms | 77.540–77.948 ms |
| `resolve/clone-plan` | 57.65 ns | 56.45–58.87 ns |
| `resolve/new-frame+cold-zone` | 280.23 ns | 268.82–293.37 ns |
| `resolve/civil-expr` | 137.98 ns | 134.63–141.61 ns |

## Review of the independent temporal challenge

[TemporalDesignChallenge](./r5-temporal-adversarial.md) independently confirmed the L6/L13 blockers against production source and ticket 19. Its stronger counterexample is that L13 as written says `Date(D) == D@noon` **and** `Date(D) < D@noon` when ordering uses the day's start. This cannot be the same relation; interval filtering and deterministic sorting must be distinct. For instant `x` and `[s,e)`, the candidate filter predicates are `==: s <= x < e`, `<: x < s`, `<=: x < e`, `>: x >= e`, `>=: x >= s`; reverse operands by swapping, not copying formulas. A date-valued field versus a date literal and two date-valued fields also need distinct, explicit rules. No field-vs-field semantic override may be silently inferred from L13.

The challenge also found [V] `file.cdate` falls back to *modification time* if creation time is unavailable (`src/query/results.rs:271-289`), so `file.date ?? file.cdate` is an explicit alternative but not literally equivalent to the old `created_at` fallback on all platforms. [V] Template `field_value` calls `DateTimeValue::to_string()` (`src/template/engine/query.rs:594-603`), adding a second fallible presentation route. [V] The existing `cached<T,E>` render extension (`src/template/engine/cache.rs:42-55`) **does not cache failures**: if it seeds the frame, a failed zone lookup might retry later in the same render. Pin a frame containing a memoized failure or have the host resolve the frame once. A deferred `QuerySet` must carry the pinned frame/plan itself; a minijinja State-only carrier cannot serve its later `len`, `get`, or iteration once the State is gone. These findings strengthen L18 without pre-selecting an engine mechanism.

The temporal subagent did not run tests or benchmarks, and its skipped-next-day proposal remains untested. Its `/tmp` prototype citations support reasoning, not production proof.

## Review of the independent host challenge

[HostDesignChallenge](./r5-host-adversarial.md) verified production paths, trust and index code and corrected several tempting but unsound conclusions in the unfinished [r4 host report](./r4-host-workspace-alternatives.md):

- [V] Path-valued env uses `var_os` and OS-native `split_paths` (`src/dirs.rs:56-77,90-116,125-162`; `src/env_vars.rs:6-16`), so L2's test constructor must preserve `OsString`/`PathBuf` rather than forcing `HashMap<String,String>`. [H] Relative overrides captured without an absolute base can still depend on process cwd at later I/O; normalize against an explicit base or reject them with an explicit compatibility ruling. Preserve platform-specific HOME/XDG precedence.
- [V] `ArcSwap<T>` already contains `Arc<T>`; r4's `ArcSwap<Arc<Inner>>` double-wraps the allocation. Neither that dependency nor a global registry is justified merely by multi-workspace readiness. A held trusted config is checked only on load; replacing an `Arc` does not detect an edit, revocation or changed global config. Decide next-request versus immediate revocation in the config lifecycle ticket.
- [V] Index persistence is root-local (`.traces/index.redb`), its fingerprint omits root/schema/ignore policy, and query execution reads paths then records in separate operations. [H] Two different policy generations simultaneously reading/writing one root-local DB cannot be made independent just by keying in-memory workspaces with `(root, policy)`; choose one policy owner per root and serialize refresh/read materialization, or define separate persistence namespaces. The multi-root correctness constraint **does not require implementing a registry now** (forward-consumer ticket 17).
- [V] `PresetDialogProvider` falls back to first choice when answers run out, and existing CLI non-TTY/`--no-input` paths also use defaults; a strict headless policy must not silently rewrite frozen CLI behavior. Use a strict provider for new headless hosts, retain an explicit legacy CLI fallback until the CLI-surface decision changes it.
- [V] `Uuid::new_v4()` in `src/template/engine.rs:219-224` is an unlisted entropy read. L17's “everything else explicit” claim is false: either explicitly exempt random UUID generation as a template capability or inject a generator for reproducible renders. Do not stuff entropy into host policy.
- [V] Clippy `disallowed-methods` takes individual fully qualified paths, not the proposed `std::env::var*` wildcard. Keep enforcement per production crate and list concrete forbidden functions. A lexical scan is an incomplete inventory, not a second proof that no ambient path exists; alias/function-pointer/dependency behavior needs appropriate semantic checks if a real escape is shown.

The host scout did not compile its design suggestions or exercise same-root concurrent access and trust revocation; those remain risks, not observed failures. It also found that [ticket 20](../../issues/20-schema-note-typing-and-validation.md) leaves schema definitions out of the index: swapping a schema service does not itself imply rebuilding the persisted index. This belongs in the handoff to [config lifecycle](../../issues/22-config-ownership-and-lifecycle.md), [index freshness](../../issues/25-index-freshness-and-invalidation.md), [path identity](../../issues/29-path-and-capability-model.md), [workspace composition](../../issues/30-workspace-host-and-composition.md) and [crate topology](../../issues/32-crate-topology-and-public-surface.md).

## Revised design direction (provisional, not approval)

The **narrowest complete architecture** is not a mega-context or a raw `(now, zone)` argument at every leaf. Keep four lifetimes distinct: adapter-acquired host policy, workspace config/services, operation path/dialog, and one evaluation snapshot. A reusable unresolved expression is resolved against the snapshot; the resulting plan owns its immutable calendar capability once, while civil-only operations avoid zone acquisition. A lazy result must pin that plan at creation and expose a fallible materialization boundary wherever row evaluation can fail. Query rendering participates in the same snapshot. This architecture is a proposal, not established by the lab.

| Ledger decision | Strengthen / best dissent | One fact that would flip the recommendation |
|---|---|---|
| L1 multiple workspaces | Preserve independent *policy* without building a registry. Dissent: one immutable adapter policy shared across roots is simpler when the host intentionally shares roots/ignore rules, while workspace configs and zones remain independent. | One process must open the **same root** under different policies concurrently; then persistence and ownership need an explicit decision. |
| L2 host snapshot | Capture OS-string env once and derive typed, absolute paths and ignore policy under a documented base; do not retain stringly typed maps in core. Dissent: production constructors for existing locator/stores plus injected ignore set might already align every authority. | A concrete operation needs one indivisible policy snapshot across locator, trust and index. |
| L3 explicit path | Require explicit start path at discovery/open seams; avoid redundantly passing it to operations already bound to an identified workspace. Dissent: pass it on every operation if workspace identity is not stable. | Operations can retarget a different workspace without reopening. |
| L4 resolve + calendar | Keep the clock only at resolve, a shared zone capability on the resolved plan; specialize constant temporal predicates where semantics permit, not as an architectural mandate. Dissent: opaque temporal handle at every evaluator leaf if resolution cannot accommodate adaptive operands. | A real query construct requires per-row evaluation of `now` rather than a request snapshot. |
| L5 sampling | One frame per independent query/render/request; nested lazy sets pin the creator's frame, held unresolved expressions re-resolve next request. Dissent: per-operation sampling in a live-updating UI. | Product requirement for a query to change *while the same request is executing*. |
| L6 zone errors | Lazy acquisition only when needed; typed resolution/materialization errors; redesign infallible `QuerySet` reads rather than suppressing failures. Dissent: a visible host-chosen UTC fallback for headless deployments. | Users demand successful zoned calculations when OS zone lookup fails and no zone is configured. |
| L7 precise file.date | Preserve precision in a zone-free period type; a persisted derived value must be invalidated when its parser or week rule changes. Dissent: compute from stored filename on demand to avoid index coupling. | Representative query profiling shows reparsing is negligible and persistence schema complexity dominates. |
| L8 week default | ISO means Monday **plus** minimum four days in week one; a future configurable start needs its own week-one rule and encoding/versioning. Dissent: use fixed ISO parsing for filename weeks, independent of configurable display weeks. | Product need to interpret filename week numbers under custom calendars. |
| L9 no creation fallback | Remove fallback to restore zone-free index semantics; `file.cdate` remains explicit. Dissent: zone-aware fallback plus derivation-zone fingerprint. | Users depend on implicit creation dates with no migration route. |
| L10 dialog | Strict noninteractive/preset providers for new headless hosts; retain existing CLI `--no-input`/non-TTY fallback until its user-visible contract is separately decided. Dissent: a documented safe default for a specific host operation. | A host's documented noninteractive workflow must select a known safe choice rather than error. |
| L11 evaluator dependency | Keep own evaluator for this language, borrow resolve/fold design rather than wholesale SQL engine; dissent: adopt an engine when joins/grouping/optimizer requirements demonstrably exceed its maintenance cost. | Workload requires engine-scale joins/grouping/columnar plans. |
| L12 frame source | Query service may own a clock/zone source for standalone calls; `run_with` receives a render/request snapshot. Dissent: make the frame an explicit argument on *all* calls so caller controls every sample. | Service instances can outlive host-policy changes that must take effect before reconstructing them. |
| L13 date interval | Define separate interval-predicate and total-sort relations; specify field-vs-field independently, and fix exclusive upper bound. Dissent: retain civil-midnight comparison for fields to honour ticket 19. | User chooses field membership over the settled midnight rule and accepts non-transitive filter matching. |
| L14 workspace object | Compose config and derived services by workspace generation, but keep index persistence and trust liveness separately owned; schema-only changes need not rebuild the index. Dissent: thin pure functions over explicit services for stateless checks. | No same-lifetime shared services remain after composition, or a held config cannot be revalidated across requests. |
| L15 enforcement | List exact fully qualified banned calls per production core crate; allow audited acquisition at adapter callsites. A lexical scan is an inventory, not a proof. Dissent: a semantic AST/HIR audit if a concrete lint escape occurs. | A forbidden ambient call can pass Clippy and behavior checks in a core target. |
| L16 test cutover | Remove guards and HOME override once injected tests and subprocess adapter tests cover their behavior. Dissent: retain a CLI-only CwdGuard for in-process `cargo test` adapter tests even though nextest isolates test processes. | All cwd-mutating CLI tests use subprocesses or no longer mutate cwd. |
| L17 remaining reads | Inventory render entropy (`Uuid::new_v4`) and filesystem liveness as capabilities too; either explicitly exempt entropy at the template seam or inject a generator. Dissent: a random UUID is intentionally part of the template API, not a reproducible workspace policy. | Product requires reproducible renders or same-request deterministic UUID sequencing. |
| L18 template frame | Reuse render-scoped extension where viable, but capture the frame/plan in any escaping lazy set and memoize failures; dissent: hidden root key when callbacks lack extension access. | An include/import, count, iteration or post-render access loses the creator's frame under the extension route. |

The table deliberately separates mechanism decisions from open **product semantics**: field-vs-field date equality, treatment of a skipped local day, zone fallback, bare year filename matching, configured week numbering, and midnight refresh policy. These cannot be inferred from a benchmark or user-vault example.

## Decisions that still need the user's ruling

These are independent first-frontier decisions, **not** one bundled approval:

1. **Date/instant semantics.** Recommend retaining ticket 19's civil-midnight *field-vs-field* relation while using `[s,e)` for period *literal-vs-instant* predicates, with a separate total sort key. Should a `file.date` period-valued field itself also behave as an interval against an instant? If yes, spell out `==`, `<`, `<=`, `>`, `>=` and the overlap/containment rule for two periods; do not retain L13's contradictory “equality is membership, ordering compares the start” wording.
2. **Execution errors.** Recommend a typed `Result` at lazy materialization and rendering rather than a silent nonmatch/null; this **revises ticket 19's “filters never error”** and infallible `QuerySet` interfaces. If that ticket is immutable, choose an explicit host-level error/diagnostic policy and abandon L6's typed per-row failure claim. A failed `DateTimeValue::Display` must never silently erase output.
3. **Same-root policy.** Recommend independent policies across distinct roots but only one index/policy owner at a time for any one canonical root until a persistence namespace is designed. Do you require the *same* root under differing policies concurrently? Distinct roots in one process remain supported either way.
4. **Held trust.** Recommend re-verifying local config identity before publishing each new workspace generation, and specifying whether an edit/revocation blocks the *next request* or aborts already-running ones. Choose the latter only if revocation must take immediate effect; an `Arc` swap alone cannot provide it.
5. **Filename periods.** Recommend ISO week numbering as default (Monday, minimum four days), reserve custom week numbering for `DateConfig`; decide whether bare `YYYY` and non-delimited `YYYY-Www` at the start of *any* title count as dates or need a delimiter/configured format. An author's filenames are not a safe universal default.
6. **Noninteractive CLI.** Recommend keeping current explicit CLI `--no-input`/non-TTY defaults for compatibility, while new LSP/MCP hosts use a strict `NotInteractive` error or explicitly configured preset. Do you intend to change existing CLI defaults as part of this ticket?

Downstream decisions: config precedence and relative override semantics belong to [config lifecycle](../../issues/22-config-ownership-and-lifecycle.md) and [path identity](../../issues/29-path-and-capability-model.md); period construction and skipped-day mapping to [temporal boundary](../../issues/27-temporal-boundary-and-dialects.md); schema service refresh versus persistent index identity to [index freshness](../../issues/25-index-freshness-and-invalidation.md); the frame carrier and UUID capability to [template boundary](../../issues/28-template-engine-boundary.md); host registry, trust publication and crate placement to [workspace host](../../issues/30-workspace-host-and-composition.md) and [crate topology](../../issues/32-crate-topology-and-public-surface.md). Ticket 21 must establish invariants without prematurely selecting those implementations.

## Verification and limits

- The two r5 subagent reports are **read-only investigations**, not compiled implementations. Their key claims were spot-checked against the source cited above; unsupported items remain tagged [H]. `r4-host-workspace-alternatives.md` declares itself incomplete and is not a decision source.
- The prior compiled lab and nine passing `/tmp/period-proto` tests were already run in this conversation; no production source, tests, or executable behavior was changed here. The Apia **predecessor** and the redesigned fallible result contract were not tested. Criterion measurements are prototype observations; they cannot establish a production speedup, and concurrent host policy/trust revocation were not exercised.
- Implementation cost: replacing `QuerySet`'s infallible accessors/trait implementations and formatter is a substantial API cutover; replacing `CwdGuard` changes test fixtures. These are neutral migration facts, not arguments against correct ownership.

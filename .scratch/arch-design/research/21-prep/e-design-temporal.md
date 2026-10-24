# Temporal execution inputs: design menu for the execution-context grilling

## Summary and evidence boundary

[V] [What is the explicit execution context, and what ambient state is eliminated?](../../issues/21-ambient-environment-and-execution-context.md):36-43 adopts the destination of [What are the coercion and comparison rules?](../../issues/19-coercion-and-comparison-policy.md): naive datetimes are civil; explicit-offset datetimes are instants; zone, `now` and `today` enter evaluation, not the persisted index. [V] [What is the schema note typing and validation policy?](../../issues/20-schema-note-typing-and-validation.md):66-72 requires use cases to be functions with explicit inputs, not held context objects. [P] This menu concerns only temporal values and their handoff to query, sort and template; names in sketches are placeholders, not type decisions.

[V] The current tree is **not yet** at that destination: `FileMeta::derive_day` persists a creation-time local date (`src/file.rs:66-98`); frontmatter classification resolves naive dates while Rayon parses notes (`src/note/metadata.rs:56-74`, `src/field.rs:769-782`, `src/temporal/datetime.rs:559-570`, `src/index/service.rs:329-356`). [H] A query-only temporal input today cannot make an already-built index independent of the zone. See T6 before adopting any T1 option.

[V] Direct source inventory (starting point, cross-checked at representative leaves): 24 direct production `local_zone()` expressions (3 temporal datetime, 1 temporal point, 9 filter, 11 template date) plus 3 template `Timestamp::now()` expressions (`a-code-inventory.md:19-62`; `src/query/grammar/filter.rs:375-435,624-765`; `src/template/engine/date.rs:112-184,1007-1026`). [V] Other ambient calls are **transitive**: `NoteFieldValueRef::compare` promotes a date through ambient `TryFrom` (`src/note/field.rs:273-310`), `DateTimeValue::fmt` obtains a zone (`src/temporal/datetime.rs:293-305`), and `DatePoint::instant` promotes a date via the same ambient conversion (`src/temporal/point.rs:246-261`). [V] `file.cday`/`file.mday` also convert timestamps to local dates during row field resolution, independently of `FileMeta.day` (`src/query/results.rs:268-311`). Counting only 24 sites underestimates the cutover.

[V] An attempted current-tree CodeGraph exploration returned “not initialized” with both implicit and explicit project paths; code claims here instead cite direct current-tree source reads, not the stale index. [H] Signature costs are bounded estimates, not compiled patches or measured performance. No build/test was run. Notation, as in the sibling prep files: [V] verified against the cited file or lines, [H] inference, [P] proposal.

## Decisions and dependency order

1. **T6 (index cutover gate):** is a persisted zone-derived date still legal, and which migration precedes an evaluation-only promise?
2. **T3 (zone source/failure and week policy):** how does the host resolve a usable `TimeZone` and a configured start weekday?
3. **T2 (sampling/pairing):** what unit owns a single instant plus resolved zone; what is reused by nested template queries and deferred sets?
4. **T1 (representation/transport):** values, `Zoned`, aggregate, trait, guard, TLS, or binding; how do the leaves receive input?
5. **T4 (day boundaries):** how does a civil date become an instant interval without inventing midnight or 24-hour days?
6. **T5 (tests/cutover):** where do fixed instant and fixed zone enter tests, and how far do 98 guard calls migrate?

[P] A decision on one item does not select all others: a `Zoned` snapshot can coexist with a bind step; a CLI can retain its current guard while a future host passes values. [V] [What are the query core and plan boundaries?](../../issues/26-query-core-and-plan.md), [What is the temporal boundary and dialect policy?](../../issues/27-temporal-boundary-and-dialects.md), [What is the template engine boundary?](../../issues/28-template-engine-boundary.md) each own downstream implementation details (`b-prior-decisions.md:90-103`).

## T6. Prerequisite: current index violates the destination

[V] `FileMeta::from_metadata` stores `day` and chooses filename date or creation time converted through the local zone (`src/file.rs:66-98`). `FileField::Day` reads this stored value; `CreatedDate`/`ModifiedDate` instead derive local dates on field access (`src/query/results.rs:271-311`). [V] YAML classification calls `DateTimeValue::parse_iso` through `FieldStringValue`, and `DateTimeInput::resolve` maps a `LocalClock` through `from_local` to a UTC instant (`src/field.rs:769-782,811-817`; `src/temporal/datetime.rs:115-146,559-570`). [V] Notes are parsed on Rayon threads (`src/index/service.rs:329-356`); the test zone override is TLS and does not cross threads (`src/temporal/mod.rs:57-68,193-203`).

| Route | Signature/site cost [H] | Determinism; long-lived fit [H] | Risk and deletion [H] |
| --- | --- | --- | --- |
| A. First implement civil naive values and stop storing zone-derived `FileMeta.day` [P] | Parser/value/index format plus `FileField::Day`, `cday`/`mday` lookup; nontrivial cache migration, not a 24-site edit. | Enables per-evaluation zone, different zones on same index. | Must preserve filename-date precedence and invalid/missing creation-time handling; deletes parse-time `LocalClock` -> instant and creation-date cache dependency. |
| B. Temporarily pass a chosen zone into index scan and Rayon parsing [P] | `scan`, `scan_file_metadata`, `parse_notes`/parser path plus cache fingerprint. | Deterministic *per index build*, but two zones cannot reuse one index; rebuild on zone change. | Conflicts with ticket 19's zone-free cache destination; retains expensive compatibility migration later. |
| C. Keep persisted behavior as current [P] | Zero changes. | Sound for short-lived CLI with same zone at index and evaluation only when zone remains fixed [H]. | Silently stale `file.day` and naive instants on zone change; no evaluation-only guarantee; deletes nothing. |

[P] **Lean A as a dependency, not an immediate ticket-21 implementation**: [What are the coercion and comparison rules?](../../issues/19-coercion-and-comparison-policy.md) owns the normative civil/instant rule; [What is the index read model and query execution contract?](../../issues/24-index-read-model-and-query-execution.md) owns cache encoding, [What are the index freshness and invalidation rules?](../../issues/25-index-freshness-and-invalidation.md) owns migration/invalidation, and [What are the query core and plan boundaries?](../../issues/26-query-core-and-plan.md) owns demand-time `file.day`/`cday`/`mday` resolution. [H] A changed index record format or full index rebuild must make old zone-derived records inaccessible; merely changing writers is insufficient. [P] Ticket 21 can settle future inputs now but cannot assert current zone-independent indexed behavior until these owners land the cutover.

**Disprove A:** [H] A filename prefix with an offset datetime can itself be an instant; `DatePoint::parse_prefix(name)` then calls `.date()` in `src/file.rs:92-95`, requiring a chosen zone. [P] Retain an unconverted filename source or a civil filename date when truly zone-free, and evaluate instant-shaped filename dates with the evaluation zone; establish exact precedence and error rules before removing stored `day`. [H] Also confirm "demand-time" is deliverable: after T6 the index would store raw timestamps and filename-date text, but converting a *stored* naive value later still needs a convention for missing original information (e.g. whether a naive frontmatter datetime written on a machine in `Etc/GMT-14` recovers its intended instant in UTC: it cannot; civil values must be stored as civil, and only zone-dependent *derived* fields like `file.day` move to evaluation time). **Disprove B:** [H] index parsed in one zone and queried in another produces incompatible mixed values even if the query snapshot is correct. **Disprove C:** [H] a fixed one-shot CLI is plausible; a committed LSP with a later zone change or midnight does not stay one-shot (`b-prior-decisions.md:17-39`).

## T3. Zone selection, discovery failures, and week-start

[V] Current `local_zone()` delegates to `TimeZone::try_system` and returns a date error on discovery failure (`src/temporal/zone.rs:7-20`). [V] Jiff 0.2.38 `TimeZone::system()` instead yields `Etc/Unknown` on failure; unknown has zero offset but is **not** an explicitly selected UTC zone; on Unix discovery may consult `TZ` and zoneinfo ([Jiff documentation](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZone.html#the-system-time-zone); `d-external-precedent.md:29-43`). [V] E2E child setup pins `TZ=UTC`, clears inherited env and isolates cwd/state (`tests/e2e/support.rs:174-193,228-300,352-402`); this is a fixture, not an explicit product default. [H] A minimal container may lack `/etc/localtime`/zoneinfo; packaged tzdb support and system detection are different questions (`d-external-precedent.md:35-53`).

### T3a. Absence-of-override default and configuration precedence

| Candidate | Sites/signatures [H] | Determinism; host fit [H] | Risk; deletion [H] |
| --- | --- | --- | --- |
| A. System-local via `try_system` each evaluation [P] | One host resolver; 24 leaves no longer detect zone once T1 explicit; no new user config. | Tests pass fixed `TimeZone`; LSP observes Jiff's cached local detection (~5 min), not necessarily immediate OS changes. | Matches current behavior if available; zone-less containers fail unless handled. |
| B. Fixed UTC absent an override [P] | One constant; leaf cutover as A. | Highest cross-machine reproducibility; trivial LSP/MCP. | Changes existing local-calendar behavior for non-UTC users; deletes discovery, but not need for zone in civil/instant conversion. |
| C. Trusted workspace config zone, system-local fallback [P] | A parsed setting and config revision handoff in addition to A; not a whole `DateConfig` design. | Explicit per-workspace LSP and stateless MCP select consistent zone; tests construct it. | Trust-before-parse and reload semantics matter; configured IANA availability can vary by deployment. |
| D. `TZ`-only user-selected zone [P] | Boundary env resolution; e2e already sets TZ. | Deterministic child CLI with explicit env; weak per-workspace same-process host. | `TZ` supports filenames/POSIX rules as well as names on Unix; first-use Jiff caches, env portability; no dedicated config. |
| E. CLI `--timezone` override over config/system [P] | Clap option and forwarding through relevant CLI commands; extension to LSP/MCP is not implied. | Easy ad hoc user override; not by itself a stable workspace policy. | New public UI surface/precedence tests and config drift; no known requested use case for a flag. |

[P] **Lean C as an override + A as the absence-of-override rule**, with explicit trusted-config precedence; do not add E solely to enable tests. [H] It preserves current CLI local dates and permits different workspaces, while default system discovery still needs a clear failure choice. **Best dissent:** [H] A-only is less product surface and may be enough for initial one-workspace LSP; per-workspace zone config is needless until a real user needs it. [P] [What is the config ownership and lifecycle?](../../issues/22-config-ownership-and-lifecycle.md) owns key name, trusted loading, scope, precedence and reload; 21 must hand over *resolved selected `TimeZone` at evaluation entry*, visibility after config changes, and the default/failure contract, not write a setting here.

**Disproofs:** [H] A promises “fresh each call” incorrectly if it ignores Jiff's internal TTL (`04-execution-context-precedent.md:47-50`). B breaks compatibility of `file.cday`/`today` except in UTC (`src/query/results.rs:281-298`). C can resolve config after bypassing trust unless 22 preserves the accepted trust-before-parse ordering (`b-prior-decisions.md:52-58`). D alone reproduces process-global env semantics and cannot give two workspaces distinct zones. E does not define how non-CLI hosts choose zones and adds user-visible precedence before a demand exists.

### T3b. Unavailable or invalid zone policy (independent of T3a)

| Policy | Cost/sites [H] | Determinism; long-lived fit [H] | Risk; deletion [H] |
| --- | --- | --- | --- |
| A. Error before evaluation [P] | One boundary fallible resolver, host mapping; remove 24 repeated leaf lookup errors. | Fails consistently for CLI/LSP/MCP; no silently wrong day. | Breaks CLI in stripped container, possibly even operations without dates unless acquisition is lazy. |
| B. Explicit UTC fallback **with warning** on failed system detection [P] | Resolver + advisory rendering/record; no leaf fallbacks. | Stable once selected; LSP/MCP can report degraded semantics. | Changes user's local dates; warning may be easy to lose/duplicate. |
| C. `TimeZone::unknown()` silently [P] | One conversion to unknown. | Zero-offset results; works in stripped images. | Masquerades as local/UTC; silently wrong civil/instant promotions; removes explicit discovery errors. |
| D. Require a configured/host-supplied zone, no system detection [P] | Config/CLI/host obligations; one resolver path. | Fully explicit and predictable in server. | Large breaking CLI default; burden on every first run. |

[P] **Lean A for invalid configured zones and for system-detection failures when a date-sensitive evaluation needs a zone; do not falsely label unknown as UTC.** [H] A non-date command need not fail solely because no zone was discoverable, if no temporal evaluation occurs. [P] Error *placement* and warning rendering go to [What is the error and diagnostic architecture?](../../issues/31-error-and-diagnostic-architecture.md). **Best dissent:** B makes container deployments usable without setup if a clearly surfaced warning is acceptable; decide this as user-facing semantic policy, not a Jiff API accident.

**Disproofs:** [H] A may unexpectedly fail index-only/list calls that previously never touched a zone; resolve on demand or establish all query execution requires it. B could emit a warning even when date logic is unused; demand-scoped resolution fixes noise but complicates host. C contradicts an error contract and loses provenance. D offers a crisp API but is disproportionate for a one-user CLI.

### T3c. Week start (do not smuggle this into `TimeZone`)

[V] The template date namespace exposes `start_of_week`/`sow` and `end_of_week`/`eow` (`src/template/engine/date.rs:51-69`), while Jiff's zone only determines calendar mapping, not product policy. [V] Taskwarrior's `sow` uses Monday and Org defaults Monday but permits another weekday (`d-external-precedent.md:278-289`).

| Option | Sites/signatures [H] | Determinism; host fit [H] | Risk; deletion [H] |
| --- | --- | --- | --- |
| A. Fixed current weekday semantics [P] | No added input; keep existing calendar default. | Same everywhere; server stable. | Cannot honor local preferences; must verify existing `CalendarPeriod::Week` implementation before claiming it means Monday. |
| B. Explicit `Weekday` in each evaluation input [P] | One small field forwarded to query/template period ops. | Per-workspace tests and requests deterministic. | Couples otherwise date-only arithmetic to policy; changes when settings reload. |
| C. `DateConfig` owned by trusted workspace config and projected to evaluation [P] | Config setting + projected weekday; otherwise B. | Long-lived workspace revisions explicit. | Duplication if both context and config retain weekday; don't embed full config in comparator. |
| D. Locale/system week convention read by host [P] | New discovery API/dependency and fallback rule. | Potentially friendly CLI, difficult portable testing and cache freshness. | Hidden/variable convention; no owned-code locale supplier found (`a-code-inventory.md:64-72`). |

[P] **Lean A until a real configurable week-start requirement is accepted; if accepted, C supplies B's small weekday value, never a `DateConfig` held by a value comparator.** [H] Fixed Monday is not automatically the existing Traces rule: verify in [What is the temporal boundary and dialect policy?](../../issues/27-temporal-boundary-and-dialects.md) before writing compatibility claims. **Best dissent:** users may already interpret `sow` by their locale; then B/C is essential, but locale-based D makes host behavior implicit again. [P] [What is the config ownership and lifecycle?](../../issues/22-config-ownership-and-lifecycle.md) owns the setting/reload, 27 owns calendar semantics; 21 passes chosen weekday if configurable.

**Disproofs:** [H] A cannot express requested different week starts. B makes callers send redundant weekday for nonweekly operations. C can unnecessarily reload a whole config to answer `date.today`; project only the field. D is neither specified nor trivially portable.

## T2. Sampling scope, pairing and deferred execution

[V] `QueryService::run` and `run_from_store` immediately call `plan.run(rows)` (`src/query/service.rs:101-113,129-169`); **after that**, `QuerySet` may append new transforms that run only on first read via `OnceLock` (`src/query/results.rs:456-490,520-528,539-590`). [V] Template query globals cache an index in Minijinja 3 render-local typed `State` extensions (`src/template/engine/query.rs:195-233`, `src/template/engine/cache.rs:14-54`), and `TemplateService` creates an engine then renders through it (`src/template/service.rs:62-65,263-265`; `src/template/engine.rs:185-203`). [H] “At `QueryService::run`” is not the only evaluation boundary, and engine construction is not render construction.

| Sampling unit | Cost/sites [H] | Determinism; host fit [H] | Risk; deletion [H] |
| --- | --- | --- | --- |
| A. Per process/startup [P] | One host snapshot, trivial forwarding. | Stable CLI; LSP frozen after midnight/config change. | Deletes three live reads but guarantees stale `today` for server. |
| B. Per host request [P] | Snapshot at CLI invocation/LSP request/MCP call, shared by nested operations. | Coherent request; subsequent requests fresh. | One request may contain multiple independent renders at different times; lazy returned set can outlive request. |
| C. Per query evaluation incl. lazy materialization [P] | Carry snapshot into plan/QuerySet or sample at first materialization; propagate to sort/fields. | One query coherent; LSP reruns receive fresh input. | Separate queries in one template could disagree across midnight. |
| D. Per template render; share with its nested queries [P] | One value at `TemplateEngine::render`; callback/access path to query and date functions. | Coherent output and includes, fresh next render. | Deferred QuerySet escaping render must carry value; Minijinja typed state is not auto-seeded from `render_captured`. |
| E. Per row/filter/callback [P] | Little plumbing, retain leaf reads. | Weak: row order and midnight affect result, TLS worker hazards. | Does not delete repeated reads; worst consistency. |

[P] **Lean C for standalone query, D for template render (nested query inherits the render's snapshot), B only when request itself is one evaluation.** [P] Sample `now: Timestamp` exactly once, resolve/clone one immutable `TimeZone`, derive `today = DatePoint::from(now).date_in(&zone)`; derive relative week from that civil date and chosen weekday. Never separately call `Date::today()` or `Zoned::now()` after selecting zone. [V] Jiff's zone is immutable; zone *name* re-resolved per row would not pin tzdb rules (`d-external-precedent.md:40-47`). **Best dissent:** B is simpler for LSP and every request currently might represent one analysis; spell that assumption out, and bind lazy sets before releasing request ownership.

**Disproofs:** [H] A fails midnight. B is underspecified for a long streamed response and materialization after request return. C leaves template date and its inner queries with distinct clocks unless D overrides its default. D samples even a template without date operations and must pass through static callbacks. E cannot guarantee a single temporal interpretation inside a query.

[P] Choose one of two deferred-set contracts: capture a *resolved value* when building the set and carry it into `QuerySet::push`, or keep an unbound plan and require a value explicitly when materializing; never let `OnceLock` bind to whichever thread happens to call `len()` first. [V] The existing `QuerySet` clones `base` and pending `plan`, resets the cache on `push`, and materializes on `len/get/iter` (`src/query/results.rs:463-528`). [H] A set already materialized has cached results; a new evaluation needs a fresh set or a new bound plan, not a second call to the same `OnceLock`. [P] An LSP host retaining bound relative queries must rebind/re-evaluate on next request or at local midnight and on zone/config changes; re-sampling `now` without invalidating cached results has no effect. [V] Tasks explicitly rebuilds its relative query at midnight; Dataview's observed index-revision refresh does not establish a midnight trigger (`c-digest-lessons.md:3-10,56-64`). [P] Scheduling diagnostics refresh belongs to [What is the workspace host and composition model?](../../issues/30-workspace-host-and-composition.md) and [How do live buffers and the persisted index stay consistent?](../../issues/34-live-buffer-and-persisted-index-consistency.md), not an ambient read in comparison. MCP can sample per stateless call with explicit workspace/config identity (`d-external-precedent.md:185-208`).

## T1. Evaluation input shape and transport

[P] `Temporal`/`EvalTime` are sketch labels. Counts refer to current *direct* occurrences plus transitive consumers; all explicit choices still require the T6 cutover and ticket-19 comparison cutover. [V] The deepest present query path is `List::run -> render -> refresh_page_query -> refresh_query -> run_query_builder_from_store -> QueryService::run_from_store -> plan.run -> QueryTransform::apply -> FilterExpr::is_matching -> ValueExpr/QueryFunction -> local_zone()` (`src/cli/list.rs:61-96`, `src/cli/mod.rs:335-390,470-487`, `src/query/service.rs:129-169`, `src/query/plan.rs:37-55,238-248`, `src/query/grammar/filter.rs:130-149,272-297,624-675`). [V] `QueryService::run` for in-memory/template index is a second entry (`src/query/service.rs:101-113`). [H] Passing one `&Context` blindly through 9-11 frames is avoidable by plan or operation ownership, but the number of affected methods remains nontrivial.

| Shape / transport | Signatures and sites [H] | Determinism; long-lived fit [H] | Risk; what it deletes [H] |
| --- | --- | --- | --- |
| A. Concrete `(Timestamp, TimeZone)` parameters [P] | Two values at host/service and ~9-11 query frames if passed naïvely; sort/row and template callbacks also touched. | Excellent if kept together by call convention; easy fixed-value tests, per-request host. | Accidental different samples at separate calls; deletes all 24 leaves, 3 clocks, ambient compare/format only after full migration. |
| B. A single `jiff::Zoned` [P] | One parameter through same path; zone recovered from zoned value. | Atomic instant+zone, cheap test construction. | No explicit week-start; may obscure distinction between instant and civil `today`; same leaf deletion as A. |
| C. Tiny `{now, zone, week_start?}` data value [P] | One parameter, derive `today` once or on demand; avoid adding computed fields to stored index. | Coherent invariant and easy tests/host reuse. | Can become broad `ExecutionContext`; limiting it to temporal data prevents god-object. Same leaf deletion as A. |
| D. `dyn Clock` supplier [P] | Trait at host and query chain, still separately resolve zone and decide sampling. | Fake clock possible; live host can resample. | If called per leaf gives *less* determinism, dispatch/trait complexity; deletes no ambient zone until paired with explicit zone. |
| E. Scoped `ZoneGuard`-like override plus separate scoped clock [P] | Minimal call signature changes and test changes; production guard must expand beyond current cfg gate. | CLI single evaluation works if thread stays put; LSP concurrent zones/Rayon fail without propagation. | Existing override is test-only TLS; retaining lookup in 24 leaves and clock override adds hidden execution dependency. |
| F. Thread/task-local `with_eval(..)` [P] | Host wrapper and implicit leaf read; few signatures. | Deterministic on one synchronous thread *inside scope*. | Rayon/migration across async awaits and lazy sets outside scope break it; deletes direct `local_zone()` tokens, not hidden dependency. |
| G. Bind Unbound -> Bound plan, freeze relative operands and normalized sort keys [P] | New binder plus AST/sort/QuerySet shape; not just method args. | Executor can be clock/zone-free; bind per evaluation from shared snapshot. | Dynamic row-dependent date functions cannot all fold before rows; cached bound plan stale at midnight; still needs format/render zone. |

[P] **Lean C (or B if no weekday is carried), with an optional G binding phase owned by query core**, not a compulsory `Clock` trait or TLS. [H] A two-argument pair is even smaller if every entry passes both, so choose by actual callsite clarity, not “more types.” [V] Jiff `Timestamp::now().to_zoned(zone)` preserves the chosen zone; `Zoned::now()` chooses system zone again (`d-external-precedent.md:12-17`). **Best dissent:** [H] E genuinely works for today's single-thread CLI and existing zone tests; not all ambient state is automatically a bug. The committed LSP and ticket-19 no-ambient-comparison invariant are the deciding pressures, not line count.

### Exact leaf dispositions (apply regardless of A/B/C/G transport)

| Leaf family [V] | Explicit A/B/C [P] | E/F retained ambient [P] | G binder-only [P] |
| --- | --- | --- | --- |
| 3 `datetime.rs` + 1 `point.rs` zone calls (`src/temporal/datetime.rs:125-146,191-200,329-345`; `src/temporal/point.rs:774-782`) | Replace parse/promotion/render wrappers with `_in(&zone)` at evaluation; **T6 removes parse-time naive coercion**, `Display` cannot take a zone. | Leave wrappers; host guard/TLS must be active for *every* conversion and display; index Rayon misses host TLS. | Binder handles promotion, but arbitrary `Display`/parse outside binder remain ambient unless redesigned. |
| 9 `filter.rs` zone calls (`src/query/grammar/filter.rs:375-435,624-765`) | `QueryFunction`/`ArithOp` use evaluation zone or bound operands; per-row dynamic arithmetic still takes it. | Lookup per row, override must reach row's thread. | Fold row-independent relative operands; for row-dependent `date_add(field, ...)`, binder must build a bound operation containing zone or normalized field. |
| 11 `date.rs` zone calls (`src/template/engine/date.rs:112-184,254-258,974-989`) | Filters, tests and namespace methods accept `&mut State` or a per-render captured value; all conversions share resolved zone. | Scoped zone around whole render; future nested query/parallel render scope may escape. | Query binder alone touches **none** of these; template must still get a render value. |
| 3 template `Timestamp::now()` (`src/template/engine/date.rs:161-163,1007-1026`) | Replace with same render's `now`; `Today` derives from paired zone. | New scoped clock is needed: old `ZoneGuard` freezes only zone. | Query binder does not affect date namespace or `is_past`/`is_future`. |
| `DateTimeValue::Display`; `NoteFieldValueRef::compare` (`src/temporal/datetime.rs:293-305`; `src/note/field.rs:273-310`) | No ambient conversion in generic traits; explicit formatter and normalized comparison keys at evaluation; context-free same-kind comparator remains pure. | Retain ambient wrappers; violates ticket 19 no-ambient-comparison rule and permits mixed-zone sort under concurrency. | Binder normalizes operands/sort keys, but generic `Display` still needs explicit API or a context-free canonical format. |
| `FileMeta::derive_day` (`src/file.rs:66-98`) | T6 move zone-sensitive day computation to evaluation, retain raw timestamp/name source in index. | Host TLS on scanner threads cannot propagate; old cache is zone-bound. | Cannot fix already-persisted derived day; index migration first. |

[V] Sort keys are made once per row/term before comparator calls, but `SortKey::from_note_ref` promotes `Date` to an instant using ambient `TryFrom`, and text classification can parse dates (`src/query/sort.rs:63-94,135-145,351-399,457-475`). [P] Keep a *context-free comparator over normalized keys*, make key construction zone-aware; equality and filter comparisons similarly normalize operands before calling value comparison. [V] `CompareOp` currently calls `NoteFieldValueRef::compare` at filter evaluation (`src/query/grammar/filter.rs:785-836`), so merely changing `SortOrder` is not a complete fix.

**Adversarial tests of the seven shapes:** [H] A has no type-enforced pairing; B loses policy unless separately supplied; C can silently grow host roots; D doesn't establish an evaluation instant; E cannot cross Rayon or satisfy pure comparisons; F leaks scope under deferred sets and async tasks; G cannot precalculate row-dependent date arithmetic or template formatting. [P] For E to be a genuine keep-as-is alternative, explicitly narrow the promise to single-thread/single-zone CLI operation and leave ticket-19 destination unimplemented, rather than describe a hidden TLS as “explicit input.” [V] Jiff already caches system detection behind a TTL, so an OS syscall-per-row performance justification for A/B/C is inaccurate (`04-execution-context-precedent.md:47-50,124-130`).

### Worked Rust sketches (illustrative destination, not compiled code)

[P] First sketch is the minimal time/zone invariant; `from_host` is only a boundary example. [V] Existing `DatePoint::date_in` accepts `&TimeZone` (`src/temporal/point.rs:183-191`). [H] Exact visibility/error integration belongs to 26/27/31.

```rust
use jiff::{Timestamp, tz::TimeZone};

#[derive(Clone, Debug)]
struct EvalTime { now: Timestamp, zone: TimeZone }
impl EvalTime {
    fn today(&self) -> DateValue {
        DatePoint::from(DateTimeValue::from_timestamp(self.now))
            .date_in(&self.zone)
    }
}
fn from_host(zone: TimeZone) -> EvalTime {
    EvalTime { now: Timestamp::now(), zone }
}
```

[P] **Deep CLI path** sketches the affected signatures, not an invented current `today` query syntax. The actual `date_add` method is row-dependent and returns `Option<NoteFieldValue>` (`src/query/grammar/filter.rs:624-679`); a binder cannot replace the zone for a field supplied by a row.

```rust
// List::render -> refresh_page_query -> refresh_query ->
// run_query_builder_from_store -> QueryService::run_from_store(..., &time)
// -> ExecutionPlan::run(rows, &time) -> QueryTransform::apply(rows, &time)
// -> FilterExpr::is_matching(row, &time)
fn date_add(args: &[NoteFieldValue], time: &EvalTime)
    -> Option<NoteFieldValue>
{
    let [date, NoteFieldValue::Number(n), NoteFieldValue::String(unit)] = args
        else { return None };
    if matches!(date, NoteFieldValue::Null) { return Some(NoteFieldValue::Null); }
    let unit = TimeUnit::from_spelling(unit)?;
    let n = (*n).is_finite().then_some(*n)?;
    let n = (n.fract() == 0.0).then_some(n)?.to_i64()?;
    let shifted = date.as_date_point()?.shift(n, unit, &time.zone).ok()?;
    Some(NoteFieldValue::from(shifted))
}
```

[H] `.to_i64()` requires the existing `num_traits::ToPrimitive` import in this file (`src/query/grammar/filter.rs:664-675`); the sketch preserves existing invalid/null semantics, but elides the other `date_add` match arms and query error types. [P] Capture or pass time only once at the query entry, then keep it with any deferred `QuerySet` until first materialization; don't sample again at each `filter` or `sort_field`. [V] Existing `QuerySet::push` resets `cache` when it takes ownership (`src/query/results.rs:520-528`).

[P] **Template path**: current registration puts static free `date_format`/`is_past`/`is_future` callbacks on `Environment<'static>`, plus a `DateNamespace` object (`src/template/engine/date.rs:75-109`); `QueryOps` creates callbacks with `&mut State`, and typed extensions use `get_or_insert_extension` (`src/template/engine/query.rs:237-248`; `src/template/engine/cache.rs:42-54`). [V] MiniJinja 3.0.1 permits `&mut State` on filters/tests and `State::get_extension`/`get_or_insert_extension` ([State API](https://docs.rs/minijinja/3.0.1/minijinja/struct.State.html)); `Template::render_captured` creates its own state from the context ([Template API](https://docs.rs/minijinja/3.0.1/minijinja/struct.Template.html)). [H] **Crucial constraint:** `TemplateEngine::render` cannot simply seed a typed extension in a separately created `State` and then call `render_captured`; that call creates another state. One workable but imperfect adapter is to put an opaque typed object in the render context and copy it into a typed extension on the first callback; template authors can still inspect the reserved context key, so reserve it deliberately and evaluate whether that surface is acceptable. Alternative: register per-render closures in a newly constructed render environment, at the cost of engine setup and sharing.

```rust
// TemplateEngine::render(..., time: EvalTime): render context adapter.
// Implement Object for RenderContext so get_value("__traces_eval") returns
// Value::from_object(RenderTime(time.clone())); other keys remain unchanged.
// `RenderTime` implements minijinja::value::Object, but exposes no attributes.
#[derive(Debug)]
struct RenderTime(EvalTime);
impl minijinja::value::Object for RenderTime {}
fn render_time(state: &mut minijinja::State) -> Result<EvalTime, minijinja::Error> {
    if let Some(time) = state.get_extension::<EvalTime>() {
        return Ok(time.clone());
    }
    let time = state.lookup("__traces_eval")
        .and_then(|v| v.downcast_object::<RenderTime>())
        .ok_or_else(|| minijinja::Error::new(
            minijinja::ErrorKind::InvalidOperation, "missing render time"))?;
    Ok(state.get_or_insert_extension(time.0.clone()).clone())
}
// DateNamespace::register: env.add_test("is_past", is_past);
fn is_past(state: &mut minijinja::State, value: &str)
    -> TemplateEngineResult<bool>
{
    let time = render_time(state)?;
    let instant = parse_date_point(value)?
        .instant_in(&time.zone).map_err(render_calendar_error)?;
    Ok(instant < time.now)
}
// QueryOps::run(..., state: &mut State):
// self.service.run(&index, builder, &render_time(state)?)
```

[H] `instant_in` is a **proposed** explicit-zone sibling of existing ambient `DatePoint::instant` (`src/temporal/point.rs:246-261`), not a current method. [P] If render context exposes a reserved key, document/test collision and security semantics; if that is unacceptable, choose an adapter using per-render registered callbacks, not a shared mutable `DateNamespace` field whose value can race across renders. [H] In this sketch `RenderTime`'s typed object must have no template-facing mutator; implementing `Object` with default empty keys supports that but does not hide existence of the key.

## T4. Civil day intervals and DST/error behavior

[V] Jiff `Zoned::start_of_day()` returns the first instant on a date; it can be 01:00 when midnight does not exist (São Paulo 2015-10-18) and is fallible near representable limits ([Jiff start_of_day](https://docs.rs/jiff/0.2.38/jiff/struct.Zoned.html#method.start_of_day)). [V] Current `DatePoint::zoned_datetime_in` for a calendar date uses `.at(0,0,0,0).to_zoned(zone.clone())` (`src/temporal/point.rs:218-243`); [H] that does not *state* the required interval-boundary rule and must be reconciled with ticket 19's day-literal semantics. [P] Distinguish presentation of a date from promotion in a filter; not every `DatePoint` needs to become an instant.

| Day-boundary contract | Sites/signatures [H] | Determinism; host fit [H] | Risk; deletion [H] |
| --- | --- | --- | --- |
| A. Half-open `[start_of_day(d), start_of_day(d+1))` [P] | One temporal helper taking civil `Date` + resolved `&TimeZone`, bind into query operands. | Handles 23/25-hour days and date-only comparison consistently for any host. | Both starts and successor date can fail near range ends; propagate typed error. |
| B. Midnight plus exactly 24 hours [P] | Short/simple helper. | Stable away from DST. | Wrong over transitions or skipped midnight; reject if ticket 19 requires civil days. |
| C. Keep current compatible local midnight promotion [P] | Reuse `_in(zone)` conversion; minimal migration. | Handles ambiguous/gap via Jiff compatible policy but isn't guaranteed to express next day's *first* instant. | One-point promotion differs from full-day equality/interval; upper bound still needed. |
| D. Pure civil date comparison, never promote [P] | Few zone sites for civil-civil. | Great for same-kind comparisons; cannot compare civil date with instant or evaluate `file.cday`. | Violates ticket-19 cross-kind semantics unless paired with A when needed. |

[P] **Lean A for date-vs-instant comparisons, D for truly civil-civil operations.** [H] A single `TimeZone` value must be the very same resolved zone used for both boundaries. **Best dissent:** C preserves old exact-midnight `Date` vs `DateTime` ordering for existing CLI users; adopting A may alter equality semantics and needs a behavior-change table. **Disproofs:** [H] A still needs an explicit convention for dates whose successor cannot be represented and for entirely skipped local dates; don't silently saturate. B is observably wrong on DST. C produces one instant, not an interval for the whole date. D leaves mixed civil/instant comparisons unspecified.
[P] Temporal module should own a fallible interval primitive (`day_interval(date, &zone) -> Result<(Timestamp, Timestamp), CalendarError>` sketch below); query binder chooses how each operator uses it. A query **day-interval literal** compared with an instant can mean `lower <= instant && instant < upper`; that must not silently replace ticket 19's separate rule that `Date` compares as civil midnight against civil datetimes ([What are the coercion and comparison rules?](../../issues/19-coercion-and-comparison-policy.md):94-100,180-188). [P] Specify which syntax constructs demand interval matching versus point promotion in ticket 26; `instant < day interval` uses `< lower` only under an explicit interval interpretation. [P] 21 provides an immutable zone; [What is the temporal boundary and dialect policy?](../../issues/27-temporal-boundary-and-dialects.md) owns date arithmetic/edge errors, [What are the query core and plan boundaries?](../../issues/26-query-core-and-plan.md) owns operand binding and operator application.

```rust
// Date::tomorrow and Date::to_zoned are Jiff 0.2.38 APIs.
fn day_interval(date: jiff::civil::Date, zone: &TimeZone)
    -> Result<(Timestamp, Timestamp), CalendarError>
{
    let next = date.tomorrow().map_err(|_| CalendarError::OutOfRange)?;
    let first = date.to_zoned(zone.clone())
        .map_err(|_| CalendarError::OutOfRange)?
        .start_of_day().map_err(|_| CalendarError::OutOfRange)?;
    let after = next.to_zoned(zone.clone())
        .map_err(|_| CalendarError::OutOfRange)?
        .start_of_day().map_err(|_| CalendarError::OutOfRange)?;
    if first.date() != date || after.date() != next {
        return Err(CalendarError::OutOfRange);
    }
    Ok((first.timestamp(), after.timestamp()))
}
```

[V] `Date::tomorrow`, `Date::to_zoned` and `Zoned::start_of_day` are provided by Jiff 0.2.38 ([Date API](https://docs.rs/jiff/0.2.38/jiff/civil/struct.Date.html), [Zoned API](https://docs.rs/jiff/0.2.38/jiff/struct.Zoned.html#method.start_of_day)). [H] A skipped entire date may be shifted to another date by compatible resolution: the equality check above chooses **error**, not a silent empty or shifted interval. [P] The choice between error and an empty interval is still a semantic decision for ticket 27; validate São Paulo's midnight gap, fall folds, Pacific/Apia's skipped date and range endpoints there.

## T5. Test determinism and migration cost

[V] Current `ZoneGuard` is `#[cfg(any(test, feature = "test-utils"))]`, scoped to a thread, supports nested `with` and parked `set/reset` (`src/temporal/mod.rs:57-203`); the source inventory counted **98 method sites in 87 test functions**, concentrated in datetime (33), point (26) and template date (14), with query filter (2) and note field (1) (`a-code-inventory.md:117-139`). [V] It controls zone, not any of the three template clock reads (`src/template/engine/date.rs:161-163,1007-1026`). [V] Nextest's process-per-test isolation does not let the same process reset `LazyLock`, and Rust 2024 `set_var` is unsafe for concurrent readers (`d-external-precedent.md:104-116`).

| Migration strategy | Sites/signatures [H] | Fixed instant+zone; long-lived fit [H] | Risk; what it deletes [H] |
| --- | --- | --- | --- |
| A. Immediate full value cutover [P] | 24 direct zone sites + 3 clocks + transitives, ~9-11 deep query frames if naïve; rewrite up to 98 guard usages plus callsites/tests for sorts/template/metadata. | Complete fixed-value tests and LSP isolation after T6. | High churn; test failures may hide legacy formatting behavior; deletes guard only after all wrappers are gone. |
| B. Boundary-first hybrid: value input for new query/template evaluation, guards for old direct formatting tests [P] | Query/template paths and new fixed-clock fixtures first; convert guard tests as each old wrapper disappears. | Deterministic new entry points; guard remains for isolated legacy tests. | Two conventions temporarily; MUST NOT claim ticket-19 no-ambient comparison while old comparator persists. |
| C. Keep test `ZoneGuard`, add test-only `ClockGuard` [P] | Three clock leaves get override; few signatures/tests touched. | Deterministic single-thread tests when both guards installed; LSP needs other design. | Hidden temporal coupling and no propagation into Rayon; deletes nothing. |
| D. Thread-local production scope with test `with_eval` [P] | Convert 24 leaves to scope getter plus 3 clock reads; test guards could shrink after conversion. | Test injection without process mutation, single-thread CLI possible. | Lazy set outside scope, async task migration, Rayon: scope disappears; query values no longer self-describing. |

[P] **Lean B only as a staged migration toward A, with a hard no-ambient-comparison end condition; never claim B alone meets ticket 19.** [H] Up to 98 sites is an upper bound on test edits, not necessarily 98 new API arguments: many tests can share a fixed input fixture; deprecated guard uses for context-free historical formatting may survive only if the behavior is explicitly retained. **Best dissent:** C is honestly the smallest fix for isolated CLI tests, and those guards currently allow parallel test threads without a global zone lock; it is insufficient for a committed long-lived host and cannot pair `today` with `now` across several calls.

**Disproofs:** [H] A assumes every guard test should be rewritten; pure parser tests may still need legacy ambient helpers temporarily. B risks indefinite dual paths without a removal gate. C leaves production zone selection per leaf and timestamp races. D silently binds an evaluation to a worker-thread schedule. [P] Minimum behavior checks after migration: (1) two same-process evaluations with distinct fixed `(instant, zone)` without `set_var`; (2) two evaluations of same unbound plan either side of local midnight; (3) a template `date.today` plus two nested `query.from` calls and `is_past` all share one render time; (4) defer a `QuerySet` past its creator's stack/scope and materialize/clone it consistently; (5) Rayon index refresh produces zone-free records; (6) zone discovery failure is an observable error or explicit warned UTC fallback, never unlabelled unknown.

## Cross-decision interactions and ownership

1. [P] T6 precedes **claims of** evaluation-only zone, but T3/T2/T1 can be specified before implementing T6. [H] Until the index format is migrated, a host cannot recover civil naive input or original creation-date instant from only persisted converted values; 21 must not promise cross-zone reuse of current records (`src/file.rs:66-98`, `src/temporal/datetime.rs:559-570`).
2. [P] T1 binder G requires T2 rebinding: per-request fresh snapshot alone cannot fix a bound relative literal held in an LSP cache; a render-local `State` extension solves the snapshot *within* a render, not cached plan freshness after midnight (`src/template/engine/cache.rs:14-54`; `c-digest-lessons.md:56-64`).
3. [P] T3 configured zone plus T2 per-render sampling implies separate lifetimes: trusted config/zone choice per workspace revision, actual instant per evaluation, optional host env/system-zone discovery per host policy. [V] This separation follows binding input direction in ticket 19 and ticket 20 (`b-prior-decisions.md:22-39,133-150`).
4. [P] T4 interval errors should enter typed temporal/query errors; silently turning all out-of-range inputs into `None` may currently be normal for a filter (`src/query/grammar/filter.rs:375-435,655-703`), but deciding user-visible invalid-date policy is for [What is the error and diagnostic architecture?](../../issues/31-error-and-diagnostic-architecture.md), not an unchecked `unwrap` in temporal.
5. [P] No clock, zone, `DateConfig`, cwd, config roots, dialog provider, or trust state should be stored inside a primitive field comparator. [V] Ticket 20's explicit-input function rule and ticket 19's no-ambient-comparison requirement constrain that cutover (`b-prior-decisions.md:22-34`). [H] Retaining a generic `Display` that consults system zone violates pure formatting unless explicitly designated a CLI-only convenience wrapper; plan explicit-zone presentation instead.

## Open questions only the user can settle

- [P] Does a missing local system zone fail date-sensitive CLI evaluation, or may a **visible** warned UTC fallback change local-day interpretation in stripped containers?
- [P] Is a trusted per-workspace timezone override a product requirement now, or should the smallest default-only resolver suffice until a second timezone is genuinely needed? If accepted, is a CLI flag independently warranted?
- [P] Does `today` in a template mean one value for the entire render and all nested queries, even when a render crosses midnight? Is one CLI command containing two independent queries one evaluation or two?
- [P] Should week start remain fixed to existing semantics or become a trusted setting? If configurable, is it uniformly shared by query and template weekly periods?
- [P] When a long-lived LSP has date-dependent diagnostics, should the host refresh at local midnight, only on next request/edit, or document stale-until-trigger behavior? A fresh input cannot schedule an event.
- [P] Is compatibility of existing date-only equality and `file.day` across the T6/T4 migration strict enough to require a user-visible behavior-change table, or is the ticket-19 civil destination authoritative even where current results differ?

## Proposed grilling script (ordered by actual dependencies)

[P] Each question can be asked independently at its frontier; “yes” accepts the listed default, not the whole package.

1. **Q1 (Index prerequisite):** Shall ticket 19's civil-value destination be the gate before claiming a temporal evaluation-only zone? **Options:** A zone-free cache and demand-time day (recommended); B index-time zone plus reindex/fingerprint as a temporary exception; C keep current index semantics. **Flip fact:** user requires same-zone historical `file.day` to persist without index migration. [V] Present index behavior: `src/file.rs:66-98`, `src/temporal/datetime.rs:559-570`.
2. **Q2 (Default zone):** Without a trusted override, shall it remain system-local? **Options:** A system-local (recommended); B UTC; C configured zone required; D `TZ` only; E CLI flag as primary override. **Flip fact:** cross-machine identical results matter more than current non-UTC CLI output. [V] Current lookup is `try_system` (`src/temporal/zone.rs:7-20`).
3. **Q3 (Discovery failure):** Shall an unusable zone fail date-sensitive evaluation? **Options:** A error (recommended); B warning + explicit UTC; C silent unknown; D configured zone required. **Flip fact:** container deployment must function with no zoneinfo and warnings accepted as semantic degradation. [V] Jiff unknown is not explicit UTC (`d-external-precedent.md:29-38`).
4. **Q4 (Week policy):** Keep current fixed week-start unless demand for trusted `DateConfig` appears? **Options:** A fixed current (recommended); B evaluation weekday input; C trusted config-projected weekday; D host locale. **Flip fact:** distinct vaults need distinct start weekdays now. [V] Namespace has `sow`/`eow` (`src/template/engine/date.rs:51-69`).
5. **Q5 (Sampling boundary):** Shall a standalone query get one evaluation snapshot and a template render one shared snapshot for all nested queries? **Options:** A process; B request; C query plus shared render (recommended); D row. **Flip fact:** host contract treats every request and all lazy results as one atomic, non-escaping evaluation. [V] Lazy `QuerySet` + render index cache: `src/query/results.rs:456-528`, `src/template/engine/query.rs:205-233`.
6. **Q6 (Transport):** Prefer concrete sampled `(now, zone)`/small temporal value over ambient scopes, with optional query bind phase? **Options:** A pair; B `Zoned`; C small value (recommended, B viable); D clock trait; E narrow guard; F thread/task scope; G binder combined with A/B/C. **Flip fact:** CLI remains the sole consumer and ticket-19 no-ambient-comparison rule is withdrawn. [V] Current leaf totals: `a-code-inventory.md:19-62`.
7. **Q7 (Day semantics):** Use temporal-owned fallible successive-day half-open boundaries for civil-day vs instant? **Options:** A half-open (recommended); B midnight+24h; C current compatible midnight point; D pure civil only. **Flip fact:** date-only equality must remain equality with precisely one midnight instant rather than a whole-day relation. [V] Jiff day start can be non-midnight and fallible (`d-external-precedent.md:24-28`).
8. **Q8 (Migration and rollover):** Accept boundary-first hybrid only with an explicit removal gate, and require LSP rebind/refresh policy to be chosen by host tickets? **Options:** A immediate full cutover; B staged with removal gate (recommended); C guard+clock fixture forever; D TLS. **Flip fact:** the staged path can be cleanly isolated from the index cutover, and no ticket-19 comparisons have already been rewritten. [V] 98 guard usages and 3 clock leaves (`a-code-inventory.md:117-139`; `src/template/engine/date.rs:161-163,1007-1026`).

[P] Smaller alternative: retain existing CLI zone guard for isolated historical tests during migration, rather than invent a universal clock service. [H] Unchecked risk: code sketches are design illustrations, not compiled implementations; Jiff day-boundary API and rare skipped-date policy require validation in ticket 27 before adopting any exact helper.

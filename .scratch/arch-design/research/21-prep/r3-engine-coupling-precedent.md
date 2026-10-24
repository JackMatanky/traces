# How query engines keep clock and zone out of their operators

Question (user, 2026-10-22): a previous design polluted most of the query module with temporal concerns, including
code where time is irrelevant. How do SQL and other engines get coupling and cohesion right? Tags: [V] I read the
primary source; [H] inference; [P] proposal. Everything below was fetched this session unless marked.

## 1. What the engines actually do

| Engine | Where "now" is decided | Where the zone lives | What operators see | Source |
|---|---|---|---|---|
| Spark SQL | Analyzer/optimizer rule `ComputeCurrentTime` takes one `Instant.now()` snapshot per query and rewrites `CurrentDate`, `Now`, `CurrentTimestamp`, `LocalTimestamp` into literals; `CurrentDate` is `Unevaluable` (no codegen: "should get constant folded") | `TimeZoneAwareExpression` marker trait: a node carries `timeZoneId`, and `resolved` stays false until a rule sets it; `zoneIdForType` returns UTC for the NTZ (zone-free) family | A comparison of two timestamps has no zone field. Only the zone-aware nodes (`Cast`, `Hour`, `TruncTimestamp`, format and parse) carry one. Date-vs-timestamp comparison is made by type coercion inserting an explicit `Cast` | [finishAnalysis.scala](https://raw.githubusercontent.com/apache/spark/master/sql/catalyst/src/main/scala/org/apache/spark/sql/catalyst/optimizer/finishAnalysis.scala), [datetimeExpressions.scala](https://github.com/apache/spark/blob/master/sql/catalyst/src/main/scala/org/apache/spark/sql/catalyst/expressions/datetimeExpressions.scala) [V] |
| DataFusion | `now()` has volatility `Stable` and a `simplify` hook that returns a literal from `SimplifyContext::query_execution_start_time`; its `invoke_with_args` is `internal_err!("invoke should not be called on a simplified now() function")` | The zone is a field of the function object (`NowFunc.timezone`, built from `ConfigOptions` by `with_updated_config`) and part of the Arrow type: `Timestamp(unit, Option<tz>)` | Operators see literals and typed columns | [now.rs](https://raw.githubusercontent.com/apache/datafusion/main/datafusion/functions/src/datetime/now.rs) [V] |
| PostgreSQL | `now()` and the `current_timestamp` family are `STABLE`: same value for the whole statement/transaction, so the planner may evaluate them once (index scan bound) | The `TimeZone` setting is session state. A function whose result depends on it must be `STABLE`, never `IMMUTABLE` (docs: "a common error") | `timestamptz` is an instant (zone-free compare). The zone enters only at I/O, `AT TIME ZONE`, `date_trunc`, `extract`, and timestamp-vs-timestamptz promotion | [xfunc-volatility](https://www.postgresql.org/docs/current/xfunc-volatility.html) [V] |
| DuckDB | `now()`/`today()` are transaction-start stable (documented in earlier brief d:257-259) | ICU functions capture the zone and calendar into their `BindData` at bind time (`tz_setting`, `cal_setting`, `calendar`); a per-thread `CalendarCacheState` is the function's local state | Operators hold a bound function object, not a session handle | [icu-datefunc.hpp](https://raw.githubusercontent.com/duckdb/duckdb/main/extension/icu/include/icu-datefunc.hpp) [V] |
| CockroachDB | `eval.Context` holds `StmtTimestamp`, `TxnTimestamp`, `Location`, `SessionData` and every builtin reads from it | in the context | Every builtin can see everything | [context.go](https://raw.githubusercontent.com/cockroachdb/cockroach/master/pkg/sql/sem/eval/context.go) [V]. Its own authors write: "Context is dusty; it started as a collection of fields needed by expression evaluation, but it has grown quite large; some of the things in it don't seem to belong in this low-level package" |
| Trino, ClickHouse, Taskwarrior | [H] not re-fetched this round. Earlier digest (`d-external-precedent.md` section 6) covers `now` stability for DuckDB, PostgreSQL, SQLite, Taskwarrior, Org, Logseq; none of those shows a different mechanism | | | |

## 2. The pattern, in four moves [V from the rows above, H as stated]

1. **Resolve time-dependent leaves once, before evaluation.** `now`, `today`, `current_date` are nullary leaf nodes
   that cannot be evaluated. A single rewrite pass with one snapshot replaces them with literals (Spark, DataFusion).
   Downstream operators, filters and index bounds never see a clock. [H] Postgres reaches the same effect with a
   volatility class and evaluate-once planner behaviour.
2. **Embed the zone in the few nodes that need it, at resolution time.** Spark's `TimeZoneAwareExpression` is a
   marker; the analyzer fills `timeZoneId`. DataFusion puts it in the function and the type. DuckDB captures it in
   bind data. The zone is data inside a node, not a parameter of the evaluator.
3. **Put zone-sensitivity in the type system.** Instants compare without a zone. Zone-free civil types compare without
   a zone. Mixed comparison is made explicit by a conversion node inserted during analysis, so the comparison node
   stays zone-free. [H] Spark's `Date < Timestamp` is `Cast(date) < ts`; the cast alone is zone-aware.
4. **Name a volatility class for functions that read the environment.** Postgres' IMMUTABLE/STABLE/VOLATILE is the
   contract that tells the planner what may be folded and when a saved plan goes stale. The docs say a prepared plan
   with a prematurely folded `STABLE` value goes stale; that is exactly the midnight problem for a held query.

The counter-example is CockroachDB: a context passed to every evaluator. It works, and its authors call it a
grown-too-large dumping ground. [H] That is the failure mode the user describes, so the engines that stay clean are the
ones that avoid passing a context to leaves.

## 3. Caveats from the same sources

- Postgres hackers (July 2026) discuss that "promote the timestamp side to timestamptz, then compare" makes
  timestamp-vs-timestamptz comparison inconsistently ordered around DST gaps, so an index ordered by timestamp rules looks
  out of order against a timestamptz probe
  ([thread](https://www.postgresql.org/message-id/49f40122adb68f17257b314c2f817313b05684ac.camel%40cybertec.at)) [V].
  [H] Traces' rule "a `Date` compares as local midnight against an instant" has the same property. A mixed
  civil/instant comparison is not a total order, so it must never be a sort key comparator. This bears on the
  midnight-versus-interval decision (ticket 19 holds both conventions).
- [H] Engines are statically typed. Traces rows hold dynamically typed frontmatter values, so whether a comparison is
  civil-vs-civil or civil-vs-instant is known only per row. A resolve pass cannot insert the `Cast` ahead of time. Two
  consequences: row-dependent comparison nodes must carry a zone-only capability, or the schema must supply static types
  for known fields (an optimisation, not required).
- [H] Postgres, Spark and DuckDB hold the session in a process-per-session or task-per-query model. Their ambient or
  session-global parts are acceptable there and are not evidence that ambient is acceptable under rayon workers and
  several workspaces per process.

## 4. What this changes in the lab (`models/ctx-lab`, now 24 tests, all pass)

The lab's earlier shape C (bound plan) kept an `Arc<Frame>` inside the plan, so it was not the engine pattern and I
rated it "no better than B". That finding was wrong for the engine pattern. A new shape R implements it:

- `resolve.rs` is the only query-side module that sees `Frame` (clock, week start, zone). It folds `today`,
  `today + n days`, `start_of_week(constant)` into literals.
- `plan.rs` is the evaluator. It sees a `Calendar` (zone only: compare, shift, day interval) and an opaque `WeekRule`.
  It cannot read the clock because the capability does not exist in its types. A source scan asserts it names none of
  `jiff`, `TimeZone`, `Timestamp`, `Weekday`, `to_zoned`, `zone`, `today`, `now`, `Frame`
  (`tests/shape_r.rs::modules`).
- Evaluators take no evaluation parameter: `Resolved::eval(&row)`. Rayon, lazy sets and queries held in a long-lived
  host need no setup and no scope (`tests/shape_r.rs::hosting`).
- A held query is the unresolved `Expr`; resolving again after midnight gives the new day, and the old `Resolved` is
  stale by design (the Postgres prepared-plan caveat) (`resolution::stale_after_midnight...`).
- An unavailable zone fails at resolve time when `today` is used, even with zero rows, and civil-only queries resolve
  and run with no zone at all. Instant-vs-civil comparison fails per row with a typed error.

Costs and limits of R, stated against myself [H]:

- The plan still carries a `Calendar` for its comparison nodes. Query code therefore names two small opaque types
  (`Calendar`, `WeekRule`), not zero. Adding a new evaluation-time input costs one more opaque capability type threaded
  into the node variants that use it; the lab's B shape needed only a `Frame` method. R trades that for the evaluator
  being unable to read the clock.
- A non-temporal query still goes through `resolve`, taking a `Frame` parameter at the entry point. Whether the entry
  takes a `Frame` or the service owns a sampler is a separate decision (Q below).
- The lab has five node kinds and four value kinds. It does not show that the real filter grammar (about 24 zone sites,
  natural-language dates, functions like `format`, `diff`, `startofmonth`) decomposes as cleanly. Row-dependent
  functions such as `format(due)` need a zone-aware node each, as Spark's `Cast` and `Hour` do.

## 5. Where this leaves the options for Q4-Q7

| Option | Coupling of non-temporal query code | Coupling of temporal leaves | Hazards |
|---|---|---|---|
| A raw pair everywhere | high (names zone types) | high | what the user saw before |
| B opaque handle in every evaluator signature | low vocabulary, but every evaluator takes the handle | medium | the handle can read the clock anywhere; closer to CockroachDB's context |
| R resolve pass plus zone-only capability in the few nodes that need it | none for nodes with no temporal operand; comparison and conversion nodes carry `Calendar` | confined to `resolve` plus the temporal-aware nodes | stale resolved plan after midnight (re-resolve); more node variants |
| D ambient scope | none | none | fails on rayon workers and in lazy sets (lab S3/S4) |

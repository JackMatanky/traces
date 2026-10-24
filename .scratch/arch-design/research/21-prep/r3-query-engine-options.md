# Can an existing engine replace the query module?

Question (user, 2026-10-22): are there crates like DataFusion that can work with redb, so that the query module's design
complexity is offloaded to an established crate? Tags: [V] I read the source, ran the code, or read the primary page;
[S] web search snippet only; [H] inference. Migration cost is not a criterion here (user rule); fit, semantics,
maintainability and runtime model are.

## 1. What the query module is today, and where redb sits

- [V] `src/query` is about 15.9k lines including inline tests. `plan.rs` (571 lines) holds the relational mechanics
  (filter fusion, sort merge, top-k). The rest is semantic: filter grammar 2123, row and field resolution `results.rs`
  1968, service 1302, source selectors 1189, sort keys 1116, field paths 600, values 567, expressions 539, formatting 446.
- [V] redb holds `files` and `notes` as postcard blobs keyed by path, plus multimaps `links`, `paths_by_tag`,
  `paths_by_file_class` (`src/index/tables.rs:16-79`). Queries use redb for source selection only (`paths_with_tag`,
  class lookup, `query/service.rs:396`) and for batch row loads by path. Field predicates (`due < today`) run on decoded
  `Note` values in memory, because the stored blobs are opaque: redb has ordered keys and ranges (`ReadableTable::range`,
  `lower_bound`, `Key::compare` must be a total order, rust-docs-mcp redb 4.3.0) but no secondary index on a frontmatter
  field.
- [H] So the relational machinery an engine would replace is small. The part that causes the temporal coupling (value
  semantics, civil versus instant, field resolution) is not relational and would have to be rebuilt as engine extensions.

## 2. Candidates

| Candidate | What it is | Storage | Time model | Runtime | Verdict on fit |
|---|---|---|---|---|---|
| Apache DataFusion 55.2.0 | Arrow columnar SQL and DataFrame engine, extensible ([docs.rs](https://docs.rs/crate/datafusion/latest)) | Any: implement `TableProvider::scan` and `ExecutionPlan` ([guide](https://datafusion.apache.org/_sources/library-user-guide/custom-table-providers.md.txt)). No redb provider exists in my searches [S] | Zone lives in the Arrow type and `ConfigOptions`; `now()` simplifies to a literal from `query_execution_start_time`, which is an injectable `Option` (`datafusion-expr-55.2.0/src/execution_props.rs:55`, `simplify.rs:132`) [V] | async, tokio | Closest to the pattern we derived; but see the probe |
| GlueSQL 0.20 with `gluesql-redb-storage` | Embeddable SQL engine; schemaless tables with MAP and LIST ([README](https://github.com/gluesql/gluesql)) | Its own redb file: bincode `Vec<Value>` per table, schema in `__GLUESQL_META__`, built on redb 2.6.4, not 4.3 (`gluesql-redb-storage-0.20.0/src/core.rs:6-68`, docs.rs dependencies) [V] | No zone concept. `TIMESTAMP` is stored as UTC and offsets are discarded ([doc](https://gluesql.org/docs/0.20.0/sql-syntax/data-types/timestamp/)); `CURRENT_DATE` is `Utc::now().date_naive()` evaluated per call (`gluesql-core-0.20.0/src/executor/evaluate.rs:603-613`); `parse_timestamp` collapses to `NaiveDateTime` (`data/value/date.rs:52-67`) [V] | looks sync from the docs example [S] | Poor for our temporal semantics. Prior art: `turbovault-sql` builds in-memory GlueSQL tables from vault frontmatter ([docs.rs](https://docs.rs/turbovault-sql/latest/turbovault_sql/index.html)) [V] |
| Polars | DataFrame and lazy expression engine | none (in-memory frames) | Datetime has an optional zone in the dtype; `replace_time_zone` takes explicit `ambiguous` and `non_existent` policies defaulting to `raise`; comparing Date with a zoned Datetime is an error until you cast ([docs](https://docs.pola.rs/docs/python/dev/reference/series/api/polars.Series.dt.replace_time_zone.html)) [S] | sync collect [S] | Not examined in code. Strict typing means the same heterogeneous-column problem as Arrow |
| CEL (`cel` crate) | Expression language with chrono timestamps | none | `getFullYear(tz)` style overloads with IANA zone arguments; durations are hours-and-smaller, no days or months ([PR](https://github.com/cel-rust/cel-rust/pull/382)) [S] | sync | An expression layer only; no civil date, no precision |
| CozoDB | Datalog engine | RocksDB, SQLite, mem | n/a | n/a | Upstream stopped at a December 2024 commit [S] |
| sqlparser-rs | Parser only | none | none | none | Gives a SQL surface, no evaluation |

Not examined: Turso/limbo, SurrealDB embedded, DuckDB via `duckdb-rs`. Each owns its storage, so none works "with redb".

## 3. What I ran: DataFusion's Date versus Timestamp comparison

`models/df-probe` (DataFusion 55.2.0, features `sql` and `datetime_expressions`, release build 7m10s from a cold cache).
Results under session `time_zone` of `UTC` and `America/New_York`, identical in both:

- `arrow_typeof(now())` is `Timestamp(ns, "<session zone>")`: the zone follows the session config [V].
- `ts(2026-07-29T03:00Z, tz=UTC) < DATE '2026-07-29'` is false; the same with tz `America/New_York` and with tz `Asia/Tokyo`
  is also false, and `DATE '2026-07-29' = <New York midnight timestamp>` is false [V]. DataFusion therefore promotes the
  `Date` at UTC midnight regardless of the timestamp's zone or the session zone. Local-midnight promotion would give true
  for the New York case (03:00Z is 23:00 the previous local day).
- An explicit `arrow_cast(DATE ..., Timestamp(ns, "America/New_York"))` gives local midnight (`2026-07-29T00:00:00-04:00`)
  [V], so a zone-aware promotion exists but only when written explicitly as a cast.
- `arrow_cast(DATE '2015-10-18', Timestamp(ns, "America/Sao_Paulo"))` fails in `simplify_expressions` with "Cannot cast
  timezone to different timezone": a nonexistent local midnight is an error, not a chosen instant [V].
- `date_trunc('day'|'week', <New York timestamp>)` truncates in the timestamp's zone, week starts Monday
  (`2026-07-27T00:00:00-04:00`); no week-start option was found in this probe [V].

## 4. Fit against the constraints we hold

| Constraint | DataFusion | GlueSQL |
|---|---|---|
| Sync core (user: "Q3: yes") | Async plan and execute; needs a tokio runtime and a `block_on` bridge, or an async core [V from API] | Appears sync [S] |
| Dynamic frontmatter values | One Arrow type per column; mixed types need Union or Variant. The Variant epic is open (#16116), with a separate `datafusion-variant` crate [V] | MAP and LIST values, schemaless tables [V] |
| Local-midnight civil promotion | Not the default; would need an analyzer rule inserting explicit casts or UDFs [V probe] | No zones at all, UTC only [V] |
| Precision-carrying dates (day, week, month, quarter, year) | No such type; custom extension type | No such type |
| `now`/`today` stable per query, injectable | Yes: `query_execution_start_time` is an injectable `Option`; `None` leaves `now()` unsimplified [V] | No: per-call `Utc::now()` [V] |
| Existing redb tables stay readable | Yes with a custom `TableProvider` that decodes postcard rows; no field pushdown possible | No: own file format on redb 2.6.4 |
| One time library | Brings `chrono` and `chrono-tz` (a second tz database) beside jiff [V, `Cargo.lock`] | Brings `chrono` |
| Release stability | Major releases 52 (Jan), 53 (Mar), 54 (Jun), 55 (Aug 2026), each with an upgrade guide of breaking API changes; 56 guide already exists [S] | 0.17 to 0.20 within about 15 months |
| Build and size | 7m10s cold release for a minimal feature set; the project has an open binary-size epic ([#24727](https://github.com/apache/datafusion/issues/24727), 10MB+ WASM, `sqlparser` alone about 6 MiB in datafusion-cli) [V] | not measured |

## 5. Reading of the evidence

[H] No engine offloads the part that is hard here. The relational operators are about 570 lines of Traces code. What an
engine offers that we lack is SQL syntax, grouping, joins and a mature optimizer; the query language is Dataview-like and
has no joins. What we need that no engine provides is the civil/instant/precision value model and the local-midnight rule:
DataFusion's built-in comparison disagrees with it (probe), GlueSQL has no zones, Polars and Arrow require explicit casts.
The engines that do this cleanly all use the same device our lab's shape R uses: a resolve pass with an injected start time
and the zone carried in the type or node (see [r3-engine-coupling-precedent](./r3-engine-coupling-precedent.md)). DataFusion
is the best precedent for that device and exposes the injectable start time we want. Adopting it would still require writing
the Traces-specific analyzer rule and UDFs, a Variant or JSON mapping for frontmatter, and a sync bridge.

## 6. Options for the grilling record (not decisions)

| Option | What it means for ticket 21 | Best dissent |
|---|---|---|
| A. Own evaluator, adopt the resolve pattern (shape R) | The temporal design question is ours either way | A mature optimizer and SQL for free |
| B. DataFusion as execution backend | Ticket 21 still decides who sets `query_execution_start_time` and `time_zone`; the date semantics move into an analyzer rule and UDFs | Async runtime and Variant typing are open questions; second tz database |
| C. Revisit only if the query language grows aggregation or joins | Same as A for now | Deferred cost if it grows later |

## 7. Tool notes

- rust-docs-mcp cached `redb 4.3.0` only. `gluesql-core`, `gluesql-redb-storage`, `datafusion-expr` and `datafusion-catalog`
  failed with "manifest path `.rust-docs/cache/crates/<crate>/<version>/source/Cargo.toml` does not exist", also when pointed
  at local registry sources. I substituted direct reads of the cargo registry sources and docs.rs rustdoc JSON.
- The Polars, CEL and CozoDB rows come from search snippets only. Treat them as leads.

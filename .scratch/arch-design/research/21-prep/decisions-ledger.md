# Ticket 21 decision ledger (historical pre-decision hypotheses)

The [resolved Answer](../../issues/21-ambient-environment-and-execution-context.md#answer)
supersedes these provisional leans; do not treat L1-L18 below as the specification.

Source: the grilling of
[the ticket](../../issues/21-ambient-environment-and-execution-context.md) on 2026-10-22. The user said they are
"leaning toward agreeing with every recommendation" and asked for a stress test plus alternative-design research before
locking in. Treat every item below as a hypothesis to attack, not a conclusion. The goal of the codebase: a lean,
performant, idiomatic, deep, robust Rust core that is a stable, maintainable, expandable foundation for all other
features and crates (CLI, LSP, MCP, templates). Caution is not a goal; correctness and design quality are.

## The decisions (L1-L18)

| # | Decision (as agreed in the grilling) |
|---|---|
| L1 | Design as if one long-lived process can hold several independently configured workspaces (different config, ignore names, zone). Nothing below the adapters depends on process-global policy. |
| L2 | `HostInputs`: state dir, config home, ancestor ceilings and ignored-dir names are read once at the adapter into one snapshot value, constructible from a plain env map for tests, passed in, replacing the `LazyLock`/`from_env()` reads below the adapter. Index scan, trust discovery and the config locator share it. |
| L3 | Every library operation takes an explicit start path. The process cwd is read only in the CLI adapter as the default. `CwdGuard` is deleted. Whether to add a user-visible `-C <path>` flag is parked for the CLI-surface ticket. |
| L4 | Query evaluation is split in two. A **resolve step** is the only code that sees clock, zone and week start; it folds `today`, `now` and relative literals into constants. A **plan evaluator** has no evaluation parameter; only the few temporal nodes (compare, shift, day interval, component, start/end-of) carry a zone-only `Calendar` capability (and an opaque zone-free `WeekRule`). The evaluator cannot read the clock. (Lab shape R, `models/ctx-lab/src/{plan,resolve}.rs`.) |
| L5 | One frame (now, zone, week rule) is sampled per standalone query, per template render (shared by nested queries and lazy sets), and per LSP request. A held relative query keeps its unresolved expression and re-resolves after midnight; the host schedules any diagnostics refresh. |
| L6 | Default zone is system-local, resolved lazily and pinned. A missing zone is a typed error at resolve time when a temporal leaf (`today`) needs it, or per row for an instant comparison. No `Etc/Unknown`-as-UTC, no `local_zone().ok()?` row dropping. Civil-only queries need no zone. A configured zone is the config ticket's decision. |
| L7 | `FileMeta.day` becomes `file.date`: a zone-free civil value carrying its precision (day, week, month, quarter, year), parsed from the filename only; it may stay persisted in the index. Prefix forms in the author's vault: `YYYY-MM-DD`, `YYYY-[W]WW`, `YYYY-MM`, `YYYY-[Q]Q`, `YYYY`. |
| L8 | `YYYY-[W]WW` is read as ISO 8601 week-year plus week, Monday start, as the **default only**. A later `DateConfig` ticket will likely carry the first day of the week (any weekday) and user-defined date formats per period, so `file.date` can identify other prefixes. |
| L9 | The `created_at` fallback for `file.date` is dropped (Dataview has none; `file.cdate` exists, so `file.date ?? file.cdate` is explicit). Visible behaviour change for files without a date prefix. |
| L10 | Dialog behaviour is an explicit host-chosen policy: interactive, non-interactive (typed error), or preset answers. No TTY sniffing below the adapter. (User: "cautiously".) |
| L11 | No query-engine dependency (DataFusion, GlueSQL, Polars). Keep the own evaluator and the resolve-pass pattern until the language needs joins, grouping or SQL. See `r3-query-engine-options.md`. |
| L12 | The query service owns a frame source (clock, zone choice, week rule) supplied at construction, so `run` takes no frame. An explicit `run_with(&Frame)` lets a template render share one frame across nested queries. |
| L13 | A `Date` compared with an instant is defined by the local day's half-open interval from its first instant. Equality means the instant lies inside the day; ordering compares against the day's start; sort keys use the day's start. Replaces "promote to midnight and compare". |
| L14 | Working assumption for the app layer: a workspace-lifetime object owns `Config` and the services derived from it (index refresh, schema, query, template), exposes use cases as methods, and is replaced wholesale when config or schema changes. A separate host-inputs object (L2) is consumed once at open. Per-operation inputs (start path, optional frame) are method arguments. Crate placement is left to the workspace-host and crate-topology tickets. |
| L15 | "Eliminated" is enforced twice: clippy `disallowed-methods` for `std::env::var*`, `env::current_dir`, `Timestamp::now`, `Zoned::now`, `TimeZone::system`/`try_system`, allowed only in adapter modules; plus a source-scan test of the core crates. Dependency-owned reads (jiff, inquire, miette) are known residue. |
| L16 | `ZoneGuard`, `CwdGuard` and the test-only `HOME` override in `src/dirs.rs` are deleted outright once injection lands (clean cutover, no shim). Unit tests build `HostInputs` from an env map and inject a fixed frame source; a few end-to-end tests launch the real binary with a real environment. |
| L17 | Ambient reads that remain are confined to the binary and adapter code: terminal TTY/width/colour (`NO_COLOR`), logging-subscriber setup, thread-pool size, panic hook. Everything else is passed in explicitly. |
| L18 | Template contract handed to the template ticket: templates never sample the clock or zone themselves; one frame per render serves `date.*` and nested `query` calls; the carrier mechanism (hidden root key vs lazily seeded state extension) is the template ticket's choice. |

## Premises the user ratified in their own words (do not re-litigate without new evidence)

Sole consumer, library surface is free; frozen: vault-format parsing, fresh-index equivalence, trust-before-parse ORDER,
exit codes and stdout shapes; reduce visibility, `pub` only for outward items; LSP committed, MCP likely and stateless;
sync core ("Q3: yes"); naive datetimes are civil, `Date` is civil, cache holds no zone, zone and now are supplied at
evaluation (accepted inside a bundle, so still open to a hostile reading); workspace-of-crates direction with the CLI as
one consumer of core; `FileClassExpander` removal; write-back out of scope. Provenance for each: `r2-premise-audit.md`
(key U1-U15).

## Premises NOT ratified (attack freely)

Functions-only layering (ticket 20 line 72 is assistant-introduced); "no one-implementor trait" as a blanket rule;
"CwdGuard sound"; ADR-0009 as authority (duplicate numbers, both `proposed`); host-policy `ignore|warn|reject`.

## Evidence base (read, do not redo)

- `.scratch/arch-design/research/21-prep/`: `brief-r3.md` (the synthesis), `r2-premise-audit.md`, `r2-defect-hunt.md`
  (defects D1-D12), `r2-fact-audit-a-d.md`, `r3-engine-coupling-precedent.md` (Spark, DataFusion, PostgreSQL, DuckDB,
  CockroachDB patterns), `r3-query-engine-options.md` (DataFusion probe: Date vs Timestamp promotes at UTC midnight),
  `a-code-inventory.md` .. `g-design-topology-and-redteam.md` (round-1 material, some leans withdrawn; see brief-r3 s.0-1).
- Lab (compiled, 24 tests pass): `.design/2026-10-22-review-ambient-execution-context/models/ctx-lab/`
  (`cargo test --offline`; shapes A raw pair, B opaque `Frame`, C bound plan, D ambient thread-local, R resolve plus
  `Calendar`; host snapshot, trust-gated open, dialog policy, minijinja carriers). DataFusion probe:
  `.design/.../models/df-probe/`.
- The wayfinder map and tickets: `.scratch/arch-design/map.md` and `.scratch/arch-design/issues/*.md` (tickets 1-20
  resolved, 21-36 open). Refer to tickets by title with a relative link, never by bare number.
- Verified code facts (HEAD around `d6b63e0d`): 24 production `local_zone()` calls (`temporal/datetime.rs`,
  `temporal/point.rs:780`, `query/grammar/filter.rs`, `template/engine/date.rs`); 3 production `Timestamp::now()` calls (all
  in `template/engine/date.rs`); `ZoneGuard` and `CwdGuard` are `#[cfg(test)]`; nextest runs one process per test;
  `FileMeta.day` is `src/file.rs:88-98`; `scan_date_prefix` accepts only `YYYY-MM-DD` and `YYYY-MM`
  (`src/temporal/lexer.rs:18-26`); `NoteFieldValueRef::compare` promotes Date at ambient-zone midnight
  (`src/note/field.rs:273-310`); sort keys `src/query/sort.rs`; composition written twice (`src/cli/mod.rs:335-366`,
  `src/template/engine.rs:128-135`); `clippy.toml` bans only `current_dir`/`set_current_dir`.

## Rules for every agent

- This is research and design review. Lazy, minimal or "ponytail" modes do NOT apply: be thorough, be adversarial, read
  real code, run things when a claim can be checked by running it.
- Create your output file first, then append sections as you go (agents have stopped early before). Verify the file
  exists and is complete before you finish.
- Do not edit repository source, tickets, the map, or the ledger. Write only your named output file under
  `.scratch/arch-design/research/21-prep/` and scratch work under `/tmp`.
- Tag every claim: [V] you read the source, ran the code, or fetched the primary page; [H] inference; [S] search snippet
  only. Cite file:line (repo) or URL. Never fabricate a citation. State what you could not verify.
- Migration cost and churn are never a design criterion; report them separately and neutrally if relevant. Judge by
  correctness, coupling and cohesion, performance, idiomatic Rust, robustness, and the ability to extend.
- Challenge upstream artifacts (tickets, ADRs, surveys, earlier briefs, the lab). They are fallible.
- Keep dissents honest: for each recommendation give the strongest opposing design and the one fact that would flip it.
- Repo conventions you must respect when you sketch code: Rust edition 2024; `thiserror` for errors; no `unwrap` in
  library code; borrow over clone; `Cow` where ownership is conditional; doc comments on public items; tests use
  `pretty_assertions`; mise tasks for build/test (`mise run verify` is the repo gate, but do not run it).
- Benchmarks: NEVER run benchmarks in parallel; only the performance agent runs benchmarks; release mode only; no
  RUSTFLAGS; use criterion.
- Your final chat message is a short summary (under 300 words) plus the path of your file. The file is the deliverable.

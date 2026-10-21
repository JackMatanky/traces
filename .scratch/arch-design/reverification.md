# Re-verification of all tickets against `main` @ `207cc3fb`

Charting used `2dc3250d`. Nine commits then landed on `main` (config-drift detection, ADR-0009). This file records what that did to every ticket, how each verdict was reached, and what was *not* re-checked.

## Method

| Step | What | Strength |
| --- | --- | --- |
| 1 | `git diff 2dc3250d 207cc3fb -- . ':!.scratch'`: 21 files changed (config model + digest, `hash.rs`, index fingerprint/refresh/service/store/tables/mod, `lib.rs`, `task.rs`, two baselines, one e2e test, ADR-0009, index glossary) | [V] exact |
| 2 | Every `path:line` citation in all 53 map artefacts extracted and intersected with the changed set; 19 artefacts cite changed files | [V] mechanical |
| 3 | Changed production code read in full: `src/index/fingerprint.rs`, the `refresh.rs`/`service.rs`/`store.rs`/`tables.rs`/`digest.rs`/`hash.rs` diffs | [V] read |
| 4 | Drifted claims re-pinned to current lines by symbol | [V] read |
| 5 | Performance claims touching the changed refresh path re-measured with the committed `index_refresh` bench (release, serial, no RUSTFLAGS) | [V] measured, see below |
| 6 | Citations to files **not** in the changed set | [V] unchanged by construction: a claim about untouched code cannot have been invalidated by the diff |

Limits: step 6 proves *unchanged since charting*, not *true*. Scout claims outside the 14 groups of `research/01` keep the status they had: hypotheses, partially verified by the research tickets that touched them. The cold-query RSS harness from `research/11` was throwaway and not retained, so the 20k cold-query numbers were not re-measured ([H] unaffected: the merge adds one `META` read inside the existing read transaction).

## What changed on `main` that matters

- [V] **ADR-0009 is implemented.** `IndexFingerprint` (10 settings) is built by exhaustively destructuring `ConfigDigests` (`src/index/fingerprint.rs:158-201`), stored raw in the redb `meta` table (was `cache_format`) in the same write transaction as the rows (`src/index/store.rs` `write_meta`), read in the same read transaction as the file list (`read_files_and_fingerprint`), and compared in `RefreshPlan::collect`. Drift wins over staleness (`RefreshState::Drifted`), heals by a full in-memory rebuild that reuses the plan's scan and store handle (`service.rs:203-215`), and an unreadable fingerprint fails closed (`DriftCause::Unrecorded`). Root, template directories and `[schemas].directory` are excluded on purpose.
- [V] `RefreshReport` changed from a `Copy` struct to an enum (`Unchanged`/`FilesChanged`/`ConfigDrift`), which changed the `traces::testing` surface: 46 identifiers (was 45), `pub-tokens.txt` 1,550 lines (was 1,528), `public-api-test-utils.txt` 4,444 (was 4,306).
- [V] `ConfigDigests` and its bundles lost their accessor methods for `pub(crate)` fields; `digest.rs` is 380 lines (was 412). `Config::hash`/`compose` remain dead in production; the `syn` source scan remains.
- [V] `src/hash.rs` gained `Blake3Digest::{LEN, to_bytes, from_bytes}`; `TaskStatusMap::iter` is now live.
- [V] A new e2e canary (`tests/e2e/index.rs` `config_edit_heals_the_index_on_the_next_query`) pins an observable behaviour change: a config edit now heals the index on the next query. Legacy stores are wiped once.
- [V] **Unchanged by the merge** and still open: path-set-change cliff (`refresh.rs:410,475,555,560`), no racy-timestamp guard (`delta.rs` untouched; no `racy` in `src/index`), hand-bumped `META_VERSION = b"1"` with no parser identity in the fingerprint, `QueryService::run_from_store` partial `WorkspaceIndex`, everything under `src/{query,note,schema,template,temporal,cli,dialog}`.

## Per-ticket verdict

| Ticket | Verdict | Why |
| --- | --- | --- |
| [verify claims](./issues/01-research-verify-load-bearing-claims.md) | **Amended** | Claim 3, the ADR-0009 row, the staleness invariant and the dead-hashing fact superseded; claims 5 and 8 re-pinned; 12 other claim groups unchanged |
| [value model precedent](./issues/02-research-value-model-precedent.md) | Stands | External comparables; `src/field.rs` and `src/note/field.rs` untouched |
| [coercion precedent](./issues/03-research-coercion-validation-precedent.md) | Stands | External; `src/note`, `src/schema`, `src/temporal` untouched |
| [execution context precedent](./issues/04-research-execution-context-precedent.md) | Stands | External; `dirs`, `env_vars`, `temporal` untouched |
| [freshness precedent](./issues/05-research-freshness-fingerprint-precedent.md) | **Amended** | Traces-side premise outdated; its lessons 1 and 3 were realized as recommended (convergent confirmation); lesson 2 stays superseded by `research/11`; lesson 4 still open |
| [query plan precedent](./issues/06-research-query-plan-precedent.md) | Stands | External; `src/query` untouched |
| [template bridge precedent](./issues/07-research-template-bridge-precedent.md) | Stands | External; `src/template` untouched |
| [errors and topology precedent](./issues/08-research-errors-topology-precedent.md) | **Amended** | Counts now 46 / 1,550 / 4,444; `lib.rs` lines +1; conclusions stand and are reinforced (the merge itself edited baselines and a public `testing` type) |
| [capability and path precedent](./issues/09-research-capability-path-precedent.md) | Stands | `src/path.rs` untouched; `hash.rs` additions unrelated |
| [document model precedent](./issues/10-research-document-model-spans-precedent.md) | Stands | `src/note` untouched; `task.rs:51-55` shape unchanged |
| [performance evidence](./issues/11-research-performance-evidence.md) | **Amended** | Mechanism claims hold (reconcile logic untouched); line citations shifted; refresh numbers re-measured (below) |
| [compatibility envelope](./issues/16-grilling-compatibility-envelope.md) | **Amended** | New evidence: baselines are rewritten per commit; observable heal behaviour and legacy-store wipe landed. Still unanswered, still the frontier |
| [forward consumers](./issues/17-grilling-forward-consumers-and-design-pressure.md) | Stands | Unaffected |
| [value model](./issues/18-value-model.md) | Stands | `field.rs`/`note/field.rs` untouched |
| [coercion policy](./issues/19-coercion-and-comparison-policy.md) | Stands | Unaffected |
| [schema typing](./issues/20-schema-note-typing-and-validation.md) | **Amended (lines)** | fileClass tests now `service.rs:1732-1923` |
| [ambient environment](./issues/21-ambient-environment-and-execution-context.md) | Stands | Unaffected |
| [config ownership](./issues/22-config-ownership-and-lifecycle.md) | **Amended** | Sub-question (c) premise ("no production consumer") is false for 10 of 15 digests; re-scoped to the residual surface |
| [note document model](./issues/23-note-document-model.md) | Stands | `src/note` untouched |
| [index read model](./issues/24-index-read-model-and-query-execution.md) | **Amended (note)** | Query path untouched; cold path now includes the drift check and full-rebuild-on-drift |
| [index freshness](./issues/25-index-freshness-and-invalidation.md) | **Rewritten** | Premise inverted: drift detection exists. Ticket now ratifies/revises the landed model and settles the four still-open items |
| [query core](./issues/26-query-core-and-plan.md) | Stands | `src/query` untouched; `QuerySet` cloning correction already applied |
| [temporal boundary](./issues/27-temporal-boundary-and-dialects.md) | Stands | `src/temporal` untouched |
| [template boundary](./issues/28-template-engine-boundary.md) | Stands | `src/template` untouched |
| [path and capability](./issues/29-path-and-capability-model.md) | Stands | `src/path.rs` untouched |
| [workspace host](./issues/30-workspace-host-and-composition.md) | **Amended (note)** | `IndexerService::from(&Config)` derives a fingerprint; heal is log-only |
| [error architecture](./issues/31-error-and-diagnostic-architecture.md) | Stands | `src/cli/error.rs` untouched; heal observability noted in the ledger |
| [crate topology](./issues/32-crate-topology-and-public-surface.md) | **Amended (numbers)** | 46 identifiers, 1,550-line baseline |
| [reconcile](./issues/33-reconcile-decisions-and-audit-coverage.md) | **Amended** | Standing re-check of drift and ADR-0009 supersession added |

Totals: 29 tickets: 17 stand unchanged by construction, 12 amended (of which 1, the freshness ticket, was rewritten), 0 invalidated outright; no ticket lost its reason to exist. The frontier is unchanged: [compatibility envelope](./issues/16-grilling-compatibility-envelope.md) and [forward consumers](./issues/17-grilling-forward-consumers-and-design-pressure.md).

## Findings promoted to the ledger

ADR-0009 text vs code (format version), residual dead digest surface, two scout path errors, and log-only heal reports: see [findings-ledger.md](./findings-ledger.md).

## Re-measurement of the refresh path

[V] **A-B-A control, serial, release, no RUSTFLAGS.** `benches/index_refresh.rs` (44 benchmark IDs: `WorkspaceIndex::refresh` no-op 50-20k, single-upsert/delete/linked at 1k and 5k, rich profiles, and `QueryService::sync_and_run` cold-cache zero-row/no-op/single-edit/full-scan 50-20k) run as `cargo bench --features test-utils --bench index_refresh -- --quick --noplot` on pre-merge `2dc3250d` (twice, in a detached worktree) and on `207cc3fb`, with isolated `CRITERION_HOME` per run. Apple M4 Pro, same machine, back to back.

| Statistic over 44 IDs | Value |
| --- | --- |
| Median change, `207cc3fb` vs mean of the two pre-merge runs | **+0.7%** (mean +0.9%) |
| Median pre-vs-pre spread (the noise floor) | 1.7% |
| IDs beyond max(10%, 2x their own spread) | 1: `refresh/single-upsert/1000`, **-10.9%** (faster) |

Reference points at 5,000 notes (`207cc3fb`, ms): `refresh` no-op 27.3, single-upsert 28.0, single-delete 27.6, linked-single-upsert 30.5; cold `sync_and_run` zero-row 24.9. At 20,000: `refresh` no-op 69.7, `sync_and_run` zero-row 58.3, full-vault scan 67.2. The no-op floor is 16.0 ms at 50, 200 and 1,000 notes on both commits.

Verdicts:
- [V] The merge added no detectable cost to the refresh or cold-query path: the extra `META` read inside the existing read transaction and `create_tables` on incremental persist are below the noise floor.
- [V] The first, uncontrolled run (rolling-baseline `change:` column) flagged 13 of 44 IDs as regressed, including +12-18% at 5,000 notes across all four `sync_and_run` scenarios. The control shows those flags were machine state against a stale rolling baseline, not the merge. Do not read the rolling `change:` column as a pre/post signal.
- [V] This bench times the in-memory refresh and cold query paths; it does not time a drift heal. A heal is a full build plus persist by construction (`service.rs:203-215`); its cost was not measured and is not claimed.
- [H] `research/11`'s component breakdown (97% note deserialization on a path-set change, 18.5 ms at 5k) and the 20k cold-query RSS figures were not re-measured (the throwaway harnesses were not retained). The code they measure is untouched by the diff (`reconcile`, `merge_refreshed_notes`, `read_all_notes`, `QueryService::run_from_store`), so they carry over as [V at `2dc3250d`].

Cleanup state: the control used a detached worktree at `.worktrees/ab-pre-merge` (gitignored; no uncommitted changes expected). It is left in place pending your approval to remove it.


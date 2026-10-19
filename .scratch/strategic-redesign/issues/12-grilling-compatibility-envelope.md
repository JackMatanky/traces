# What counts as the externally observable surface the redesign must preserve?

Type: grilling
Status: resolved
Blocked by: none

## Question

The destination says "preserve externally observable capabilities and correctness". Which surfaces are frozen, and which may change shape?

Candidates to classify (frozen / may change with migration / free): CLI subcommands, flags, exit codes and stdout/stderr contract (`tests/e2e/`); template function API and minijinja object protocol users write against (`query`, `lists`, `tasks`, `schema`, `ui`, `file`, `date`, string helpers); query DSL syntax for `--from`/`--where` including heuristic string-to-date promotion; schema TOML format incl. external select-values files; config TOML shape and defaults; on-disk `.traces/index.redb` (derived cache, currently disposable); trust/tracking symlink layout under XDG dirs; diagnostic codes (~35 dotted codes) and message text; the Rust library surface (`pub` items in `src/lib.rs`, `traces::testing`, `tests/baselines/public-api-*.txt`).

Evidence already in hand (main @ `207cc3fb`): the merge that landed config-drift healing rewrote `RefreshReport` from a `Copy` struct into an enum, which changed `traces::testing`, added 2 re-exports (45 to 46 identifiers), and regenerated `tests/baselines/pub-tokens.txt` (1,528 to 1,550 lines) and `public-api-test-utils.txt` (4,306 to 4,444 lines): the Rust-library baselines behave as change detectors refreshed per commit, not as a consumer compatibility contract. The same merge also wiped legacy stores once (`cache_format` to `meta`) and made a config edit heal the index on the next query (new e2e canary), which is an observable behaviour change inside an envelope the user may call 'disposable cache'.
Evidence to bring: `survey/cli.md`, `survey/cross-cutting.md` (visibility freeze), existing ADRs, `tests/e2e/*`. Include whether Obsidian/Dataview/Templater compatibility is a requirement or a historical influence (it explains much of the temporal and inline-field behavior). NOT evidence: `docs/divergence-register.md` (user ruling 2026-10-18: it belongs to the temporal-core design only, is not a project-wide change mechanism, and was meant to live under `.scratch/temporal-core/`; a shorter earlier copy already exists there).

Why it is a decision: it bounds every redesign option (e.g. removing heuristic coercion, simplifying select values, changing diagnostic codes, splitting the crate).

Type: HITL. Grill the user; do not answer for them.

## Answer

Resolved with the user (rounds 1-3, 2026-10-18). Tiers: **Frozen** = a change is a regression; **Migratable** = may change if recorded in the redesign spec's own change table with its migration (no standing register; the divergence register is temporal-core-only and is not an input); **Free** = may change without ceremony.

**Audience.** The user is the only consumer. No published release, no external Rust crate depends on the library. Every surface is therefore at most Migratable unless it would break the user's own vault files.

**Oracle and change record.** `tests/e2e/*` (the compiled binary) plus the tests that pin vault-format parsing are the oracle. The spec carries its own "behaviour changes" table (surface, tier, migration); any observable change not in it is a regression.

| Surface | Tier | Note |
| --- | --- | --- |
| Vault-format parsing (frontmatter, `Key:: Value` inline fields, wikilinks, tasks, tags) | **Frozen** | Same files parse to the same meaning |
| Dataview / Templater semantic parity | **Free** | Historical influence only |
| Index result equivalence with a fresh parse of files and config | **Frozen** | |
| `index.redb` layout, versioning, refresh and drift strategy, heal-on-next-query | **Free** | `207cc3fb` already wiped stores once |
| Exit codes 0/1/2/130; stdout = payload only, stderr = diagnostics, mutating commands print nothing to stdout | **Frozen** | |
| Stdout payload shapes: `list` as `- <path>` lines, `table` as Markdown tables, `task` as checkbox lines with the original status char | **Frozen** | Header naming inside them is Migratable; a new `--format` option is an addition |
| Subcommand and flag spellings | **Migratable** | |
| Diagnostic codes (~35, at least 7 asserted by e2e) and e2e-asserted wording | **Migratable** | Via the change table, with the e2e assert changed alongside |
| Unasserted message, help and summary wording | **Free** | |
| `config.toml` and schema TOML formats (incl. external select-values files) | **Migratable** | |
| Trust gate behaviour (untrusted or stale config refuses to load; hash before parse) | **Frozen** | |
| Trust/tracking symlink layout under XDG | **Free** | Re-running `traces trust` is acceptable |
| Template/query **capabilities** (query notes, lists, tasks, schema lookup, dialogs, file ops, date arithmetic, `--from`/`--where`) | **Frozen** | Must stay reachable |
| Template function/object API and query DSL **spelling**, incl. heuristic string-to-date promotion | **Migratable** | Coercion policy is open to redesign |
| Known defects: closed-pipe panic, `i64` to `f64` loss at the note boundary, multi-valued inline fields read via `.first()` only | **Not preserved** | Fixing each is a change-table row, not a regression |
| Rust library surface (`pub` items, `traces::testing`, `public-api-*.txt`, `pub-tokens.txt`) | **Free** | Baselines are change detectors, not a contract |

**Visibility rule (user, round 3, corrected).** The primary rule is to reduce visibility whenever possible and to limit `pub` to outward-facing components. Widening is not banned as a rule: an item may be widened when it is genuinely outward-facing (for example a crate boundary after a workspace split), but the default direction is narrowing, and each `pub` item needs an outward-facing justification. "Library surface is Free" means nothing needs preserving; it does not mean visibility is unconstrained. This rule governs the crate-topology ticket and every boundary decision.

**Consequences for the map.** Unblocks [path and capability model](./25-path-and-capability-model.md) and [value model](./14-value-model.md). The heuristic-coercion removal option in the coercion ticket no longer needs a further compat decision (Migratable). The crate-topology ticket evaluates every alternative (including the single-crate `#[doc(hidden)] pub` backdoor and the workspace split) against the visibility rule above.

**Verified facts (V, main @ `b521a009`).** Cargo `0.1.0`, no release tags, `README.md` declared but absent. e2e asserts at least 7 diagnostic codes and a set of message strings; closed-pipe, `i64` to `f64` (`src/note/field.rs:545`) and `.first()`-only multi-valued reads are all still present.

**Open, not decided here.** Whether to move `docs/divergence-register.md` (417 lines) under `.scratch/temporal-core/` (which holds a different 128-line earlier copy). Unit-test wording pins were not scanned; they only affect the size of the change table.


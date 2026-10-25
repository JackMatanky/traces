# What counts as the externally observable surface the redesign must preserve?

Type: grilling
Status: resolved
Blocked by: none

## Question

The destination says "preserve externally observable capabilities and correctness".
Which surfaces are frozen? Which surfaces may change with a migration?

Classify these surfaces as frozen, migratable or free:

- CLI subcommands, flags, exit codes and the stdout/stderr contract (`tests/e2e/`).
- Template functions and the minijinja object protocol (`query`, `lists`,
  `tasks`, `schema`, `ui`, `file`, `date`, string helpers).
- Query DSL syntax for `--from`/`--where`, including heuristic string-to-date promotion.
- Schema TOML, including external select-values files; config TOML and defaults.
- The on-disk `.traces/index.redb` derived cache, currently disposable.
- The trust/tracking symlink layout under XDG directories.
- Diagnostic codes (~35 dotted codes) and message text.
- The Rust library surface (`pub` items in `src/lib.rs`, `traces::testing`,
  `tests/baselines/public-api-*.txt`).

Evidence already in hand (main @ `207cc3fb`): the config-drift healing merge
changed `RefreshReport` from a `Copy` struct to an enum. It changed
`traces::testing` and added 2 re-exports (45 to 46 identifiers). It also
regenerated `tests/baselines/pub-tokens.txt` (1,528 to 1,550 lines) and
`public-api-test-utils.txt` (4,306 to 4,444 lines). These Rust-library
baselines detect changes per commit; they are not a consumer compatibility
contract. The merge also wiped legacy stores once (`cache_format` to `meta`).
A config edit now heals the index on the next query (new e2e canary).
This is an observable behavior change within an envelope the user may call a
"disposable cache".

Use `survey/cli.md`, `survey/cross-cutting.md` (visibility freeze), existing
ADRs and `tests/e2e/*` as evidence. Decide whether
Obsidian/Dataview/Templater compatibility is required or only a historical
influence on temporal and inline-field behavior. Do not use the divergence
register as evidence. The register is now
`.scratch/temporal-core/divergence-register.md`; it was formerly
`docs/divergence-register.md`. The user ruled on 2026-10-18 that it belongs
only to temporal-core. It is not a project-wide change mechanism. The user
intended it to live under `.scratch/temporal-core/`, where a shorter earlier
copy already exists.

Why it is a decision: it bounds every redesign option (e.g. removing heuristic coercion, simplifying select values, changing diagnostic codes, splitting the crate).

Type: HITL. Grill the user; do not answer for them.

## Answer

The user resolved these tiers in rounds 1-3 on 2026-10-18:

- **Frozen:** A change is a regression.
- **Migratable:** Record the change and its migration in the redesign spec's
  own change table. Do not use a standing register. The divergence register
  belongs only to temporal-core and is not an input.
- **Free:** May change without ceremony.

**Audience.** The user is the only consumer. There is no published release.
No external Rust crate depends on the library. A surface is at most
Migratable unless changing it breaks the user's own vault files.

**Oracle and change record.** Use `tests/e2e/*` with the compiled binary.
Also use the tests that pin vault-format parsing. The spec has its own
"behaviour changes" table with surface, tier and migration. An observable
change absent from that table is a regression.

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

**Visibility rule (user, round 3, corrected).** Reduce visibility whenever
possible. Limit `pub` to outward-facing components. Widen an item only when
it is genuinely outward-facing, such as at a crate boundary after a workspace
split. Justify each `pub` item. The default direction is narrower visibility.
"Library surface is Free" removes the preservation requirement; it does not
remove the visibility rule. Apply this rule to crate topology and every
boundary decision.

**Consequences for the map.** Unblocks [path and capability model](./29-path-and-capability-model.md) and [value model](./18-value-model.md). The heuristic-coercion removal option in the coercion ticket no longer needs a further compat decision (Migratable). The crate-topology ticket evaluates every alternative (including the single-crate `#[doc(hidden)] pub` backdoor and the workspace split) against the visibility rule above.

**Verified facts (V, main @ `b521a009`).** Cargo `0.1.0`, no release tags, `README.md` declared but absent. e2e asserts at least 7 diagnostic codes and a set of message strings; closed-pipe, `i64` to `f64` (`src/note/field.rs:545`) and `.first()`-only multi-valued reads are all still present.

**Open, not decided here.** Unit-test wording pins were not scanned; they only affect the size of the change table. (The divergence register has since been moved to `.scratch/temporal-core/divergence-register.md`, replacing the older 128-line copy.)

# How do mature CLIs make ambient environment (clock, zone, dirs, env) explicit?

Type: research
Status: resolved
Blocked by: none

## Question

Traces reads process-ambient state in many places:

- `LazyLock` XDG/state dirs and ignored-dir env (`src/dirs.rs:85-190`,
  `src/env_vars.rs:8-37`), with a `#[cfg(test)] static HOME` hack
- `local_zone()`/`TimeZone::try_system` per AST leaf, with a 200-line
  thread-local `ZoneGuard` (`src/temporal/mod.rs:58-255`,
  `src/query/grammar/filter.rs:386-761`, `src/template/engine/date.rs:121`)
- `now()`
- `Cwd`/`CwdGuard` (`src/cli/cwd.rs`)

See `survey/config-dialog.md` H2, `survey/temporal.md` H2, `survey/cli.md`.

Study these comparables:

- Cargo `GlobalContext` (`src/cargo/util/context/mod.rs`) and how ops receive it
- jj `UserSettings`/`Workspace`/`Ui` and `jj-lib` config layering
- gitoxide `gix::open::Options`, `gix-sec` trust, and `gix-config` source levels
- Ruff `ruff_workspace` Settings resolution and the `ruff_db::system::System`
  abstraction
- ripgrep `Config`/`flags` parse-to-HiArgs pipeline
- uv `Preview`/`Settings`
- Jiff `tz::TimeZone::system()` caching, `Zoned`/`Timestamp::now`, and the
  `jiff` guidance on injecting the clock/zone
- Templater/Obsidian locale-zone handling for date shortcuts

Focus:

- What is the unit of "execution context": one struct or several?
- Where is it constructed vs consumed?
- How are time (`now`) and zone made deterministic for tests and for long-lived
  hosts?
- How are trust/discovery results carried?
- How does the context avoid becoming a god object?
- What does it cost call sites?

Informs: ambient environment and execution context, config ownership and
lifecycle, temporal boundary, workspace host and composition.

Deliverable: `.scratch/arch-design/research/04-execution-context-precedent.md`
(<= 300 lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp
`get_item_source`, or GitHub raw files) and cite repo path + symbol/line for
every claim. Use this structure:

1. What each comparable actually does (checked from source).
2. The invariants and who owns them.
3. The tradeoffs and costs they pay.
4. Transferable lessons for Traces.
5. Lessons that do NOT transfer, and why.
6. What this changes or challenges in the Traces hypotheses named above.

Mark facts [V], interpretation [H]. Do not propose the final decision; that
belongs to the decision tickets that list this ticket in Blocked by.

## Resolution

Research complete. Deliverable produced at `.scratch/arch-design/research/04-execution-context-precedent.md`.

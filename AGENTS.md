<!-- agent-skills:start -->
# Agent skills

## Issue tracker
Issues: local markdown under `.scratch/`. See `docs/agents/issue-tracker.md`.

## Triage labels
Five roles mapped to local state strings in issue files. See `docs/agents/triage-labels.md`.

## Domain docs
Multi-context — `GLOSSARY-MAP.md` + per-module `GLOSSARY.md` under `src/`. See `docs/agents/domain.md`.
<!-- agent-skills:end -->

## Agent workflow

### Worktree hygiene
- Temporary files from the current task **MAY** stay in the worktree while needed.
- Temporary files **MUST** be removed when no longer needed and before merging. This cleanup includes temporary files in `.scratch/`.
- User files and `.scratch/` issue records **MUST** remain intact.

### Clarification
- Clarification **MUST** start with a check of project context. Use an index or the relevant file. For code navigation, see the CodeGraph section.
- Each round **MUST** limit clarification to five focused questions. Grilling, Wayfinder, and explicitly open-ended discovery have no such limit.
- Independent questions **SHOULD** go in one batch.
- Dependent questions **MUST** wait for prerequisite answers.
- Low-risk, reversible decisions **MAY** use a stated assumption.

### Subagents
- Tasks **SHOULD** use subagents when delegation saves time or improves quality.
- Delegation **MUST** use the harness subagent tool and obey subagent tool rules.
- Independent tasks **SHOULD** run in parallel when supported.
- Dependent tasks **MUST** run in order.
- Subagent results **MUST** be reviewed before use.
- Only per-call timeouts **MAY** be set, and only when needed and allowed by the harness.

<!-- mise:start -->
## Mise — Environment & Task Orchestration

Prerequisite: Set `MISE_EXPERIMENTAL=1` for tool/task discovery.

### Execution Policy
- Builds, tests, lint checks, formatting, and audits **MUST** use `run_task`.
    - **MAY** use raw shell commands only for operations that have no task.
- **MUST** inspect `mise://tasks` before builds or tests.
- **MUST** inspect `mise://tools`, `mise://env`, and `mise://config` for environment failures.
- Check effects on downstream dependencies and tools before changing `.tool-versions` or `mise.toml`.

### Tasks & Completion Gates
| Task | Alias | Scope / Arguments | Gate & Purpose |
| :--- | :--- | :--- | :--- |
| `verify` | `v` | Runs `fmt`, then executes `check`, `lint`, and `test` concurrently | **Completion Gate**: Mandatory before yielding or committing non-trivial work |
| `test` | `t` | `-- --lib <module>`, `-- --test <file>`, or substring filter | Proves correctness |
| `bench` | — | `-m <module>`, `-f <pattern>`, `--mode quick\|test\|normal`, `--compare <baseline>` | Criterion benchmarks; auto-tags git baselines and diffs via `critcmp` |
| `lint` | `l` | Workspace, all targets, all features; `--fix` applies known lints | Strict clippy check; depends on `fmt` |
| `fmt` | `f` | Workspace scope | Apply formatting prior to diffing or staging |
| `modules:tree` | — | `-d <n>`, `-m <module>`, `--cfg-test`; extra args | Print module tree via `cargo modules`; standalone, not part of `verify` |
| `modules:orphans` | — | `--cfg-test`; extra args (`--lib`/`--deny` automatic) | Standalone check for source files outside the module tree; exits non-zero on orphans |
| `modules:deps` | — | `-d <n>`, `-m <module>`, `--acyclic`; extra args (`--lib` automatic) | Standalone DOT graph; capture before previewing (early close panics); `--acyclic` informational |
<!-- mise:end -->

<!-- hk:start -->
## hk — Static Analysis & Safe Fixes

### Inspection & Execution Workflow
1. **Pre-edit Check**: Inspect project status using `hk mcp` or `hk check --safe --format json`.
2. Limit checks to touched files.
   - For exact paths, pass a NUL-delimited list with `--files0-from <path>`.
   - Use `--cd <dir>` to target a subdirectory without changing directories.
3. Checks and fixes **MUST** use `--safe`. Destructive or unknown commands **MUST NOT** run without explicit user approval.
4. Read the JSON or JSONL diagnostics. Keep the raw output for triage. Check the diff after fixes.

### Commands
| Command | Action |
| :--- | :--- |
| `hk check --safe --format jsonl` | Stream lifecycle events post-edit; provides a final summary on both success and step failure |
| `hk fix --safe --no-stage --unstaged` | Apply non-destructive automated fixes directly to unstaged files without staging |
<!-- hk:end -->

<!-- codegraph:start -->
## CodeGraph — Semantic Code Navigation

Code navigation **MUST** start with CodeGraph. Use file or shell tools for literal or regex searches, Markdown docs, non-code assets, and edits.

- **Index:** Use only the active worktree’s `.codegraph/` index. In a new worktree without an index, run `codegraph init -i`. In a primary repository without an index, use standard file search tools.
- **Query:** Use `codegraph_explore` for broad questions. For targeted queries, use `codegraph_search`, `codegraph_node`, or `codegraph_callers`. Query `deferred` symbols by name.
- **Fallback:** If MCP is unavailable, run `codegraph explore "<symbols or question>"`. Use standard file tools if CodeGraph is unavailable or the results are insufficient.
<!-- codegraph:end -->

<!-- rust-docs:start -->
## rust-docs-mcp — Rust Documentation Engine

Crate API, source, and dependency queries **MUST** use `rust-docs_*` tools and only the local `.rust-docs/` index.

### Caching & Storage Scope
- Load indexes only from `.rust-docs/`. Use `cache_crate` with `source_type: "local"` and the crate directory in `.rust-docs/`.
- Workspace crates: pass `member: "<path>"` (e.g., `member: "crates/rmcp"`).
- Local external crates: pass `source_type: "local"` pointing to `.rust-docs/<crate_name>`.

### Navigation Workflow & Tools
1. **Hierarchy**: `structure` to inspect module layout.
2. **Discovery**:
   - Use `search_items_preview` to find item IDs, names, and kinds.
   - Use `search_items_fuzzy({query})` for approximate names.
3. **Specification**: `get_item_details` to inspect signatures, types, and docstrings.
4. **Implementation**: `get_item_source` to read function and type implementations with context lines.
5. **Dependencies**: `get_dependencies({include_tree: true})` for direct and transitive dependency trees.
6. **Filtering**: `list_crate_items({kind_filter})` to browse items filtered by kind.
<!-- rust-docs:end -->

<!-- adrs:start -->
## ADRs — Architecture Decision Records

Use the ADR tools or the [adrs CLI](https://joshrotenberg.com/adrs/).

### Lifecycle & Practices
- AI-generated ADRs **MUST** start with the `proposed` status.
- An ADR **MUST NOT** move to `accepted` without explicit human review.
- Maintain decision history and dependency graphs using `link_adrs`.

### Tool Matrix
| Domain | MCP Tools | CLI Equivalent |
| :--- | :--- | :--- |
| **Read** | `list_adrs`, `get_adr`, `search_adrs`, `run_doctor`, `export_adrs` | `adrs list`, `adrs get <id>` |
| **Write** | `create_adr`, `update_status`, `link_adrs`, `update_content` | `adrs init`, `adrs new "<Title>"` |
| **Analyze** | `validate_adr`, `compare_adrs`, `suggest_tags` | — |
<!-- adrs:end -->

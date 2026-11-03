# Research 01: Surface workflows (candidate guarantee inventory)

Question (from [issue 01](../issues/01-surface-workflows.md)): which observable
behaviors of the surface layer are *intentional guarantees* rather than
incidental behavior? Scope: `src/cli/`, `src/dialog/`, `src/config/`, the six
named e2e files, the three GLOSSARY.md files, ADR 0002/0003/0004, and
`//! Proves` / `Defect class:` headers in scope.

Method: breadth-first reading of source, in-crate tests, and the named e2e
files; docs/ADRs treated as evidence to interrogate, not authority. Every claim
below carries `file:line` in this repo (ADR references as `docs/adr/NNNN:<line>`).

## 0. Survey figures re-verified

The issue carried no numeric claims (only module/scope descriptions), so these
are fresh counts for later tickets, not confirmations:

| Unit | `#[test]` fns | extra `#[case]` attrs |
| --- | --- | --- |
| `src/cli/*.rs` | 224 | 25 |
| `src/config/**` | 116 | 79 |
| `src/dialog/*.rs` | 39 | 0 |
| e2e in scope (`config` 10, `env_policy` 7, `process_contract` 5, `workflow` 2, `init` 1, `completions` 1) | 26 | n/a |

Headers in scope: 1 `Defect class:` in `src/` (`src/cli/mod.rs:1492`);
in the six named e2e files, 22 `Defect class:` + 3 `//! Proves`-style headers
(`tests/e2e/config.rs:20,50,81,107,132,174,206,241,282`,
`env_policy.rs:15,49,98,131,188`, `process_contract.rs:12,86`,
`workflow.rs:92`, `init.rs:20`, `template.rs:42,106,142,234`,
`trust.rs:1,37`, `tracked.rs:1`). Convention lives in `tests/`, not `src/`.

## 1. Candidate guarantees

Legend: **class**: invariant (I) / state transition (ST) / workflow (W) /
error semantics (E) / edge case (X). **Pinned** = an existing test enforces it
(e2e = spawned process, unit = in-process). **Gherkin**: good / unit / poor.

### Process & routing

**G1: Exit-code table.** *class E · verdict intentional · pinned partial · Gherkin: good*
Contract: spawned `traces` exits `0` on completion, on `--help`/`--version`,
and on Escape abort; `1` on any domain failure with a miette diagnostic on
stderr and empty stdout; `2` on argv/usage failure with `Usage:` on stderr and
empty stdout; `130` on Ctrl-C abort; never `101` (no panic).
Evidence: `src/main.rs:19-35`; ADR-0004:41, ADR-0004:63; clap path
`src/cli/mod.rs:227`; pinned for 0/1/2 by `tests/e2e/process_contract.rs:14,36,48,64,88`
plus the house assertion `tests/e2e/support.rs:107`.
Gap: `130` only asserted in-process (`src/main.rs:58`): no spawned SIGINT
test; bare `traces` (no args) → `traces::cli::no_command` exit 1 is unit-only
(`src/cli/mod.rs:1441`).

**G2: stdout/stderr stream split.** *class I · verdict intentional · pinned e2e · Gherkin: good*
Contract: mutating commands (`trust`, `untrust`, `trust clean`, `index`,
`template -i … --no-input`) print a status line to stderr with stdout exactly
`""`; inspecting commands (`trust list`, `trust --show`, `template --list`,
`completions --list-templates`, query commands) print payload to stdout with
empty stderr; all exit 0.
Evidence: `tests/e2e/workflow.rs:95` (exact strings + canonicalized paths);
defect-class header `tests/e2e/workflow.rs:92`.

**G3: Count summaries live on stderr.** *class I · verdict intentional · pinned e2e+unit · Gherkin: good*
Contract: `list`/`table` print `<n> page(s) from <root>` and `task` prints
`<n> task(s) from <root>` to stderr, never to stdout; `task --count` suppresses
it. Evidence: `src/cli/list.rs:65`, `src/cli/table.rs:64`, `src/cli/task.rs:196`;
suppressed under `--count` at `src/cli/task.rs:851`; process-level sightings in
`tests/e2e/query.rs:157,376` (outside the six named files but the only e2e pin).

**G4: Stable diagnostic codes with per-stage help.** *class E · verdict intentional · pinned e2e+unit · Gherkin: good*
Contract: every domain failure surfaces as one `traces::cli::…` code on its own
stderr line regardless of which command triggered it, plus stage-specific help
(`discovery` → "run \`traces init\`…", `untrusted` → "run \`traces trust\`…",
parse/value → "valid TOML…" etc.).
Evidence: `src/cli/error.rs:260-339` (code table), `src/cli/error.rs:573-588`
(config-load code selection), `src/cli/error.rs:475-524` (help per stage);
unit pins throughout `src/cli/error.rs:824+`; e2e pins
`tests/e2e/config.rs:117,147,189,222,300`, `tests/e2e/template.rs:253`.
Wrapping caveat is part of the contract's test surface: see H3.

**G5: User Abort is control flow, not failure.** *class ST · verdict intentional · pinned partial · Gherkin: poor (H2)*
Contract: Escape (`DialogError::UserCancelled`) → `Ok(Aborted(Cancelled))`,
exit 0, no diagnostic, no side effect; Ctrl-C (`UserInterrupted`) → exit 130.
Detection walks the whole source chain for a `DialogError`.
Evidence: ADR-0004:41, ADR-0004:61; `src/cli/mod.rs:138-143`,
`src/cli/error.rs:241-255`, `src/main.rs:23-29`; no-side-effect pins
`src/cli/init.rs:310` (nothing written) and `src/cli/template.rs:608,623`
(no file written); abort mapping pins `src/cli/mod.rs:1461,1495`.
Gap: "no diagnostic printed on Escape" and the 130 process code have no
spawned assertion; all abort tests inject fake providers.

**G6: Commands do not panic.** *class I · verdict intentional-by-convention · pinned e2e · Gherkin: good*
Contract: no surface command path panics (exit 101): every e2e assertion
pairs an expected outcome with `assert_never_exit_101`
(`tests/e2e/support.rs:107-114`), used across `config`, `workflow`,
`process_contract`, `template`, `trust`, `env_policy`.

### Dialog seam (`src/dialog/`)

**G7: Non-TTY fallback table.** *class X · verdict intentional · pinned unit+e2e · Gherkin: good (the non-TTY half)*
Contract: with stdin not a terminal, `TerminalDialogProvider` returns
`text` → default or `""`, `confirm` → default or `false`, `select` → `0`,
`multi_select` → `[]`, and `is_interactive()` → `false`; it never blocks.
Evidence: `src/dialog/terminal.rs:50-108`, `src/dialog/terminal.rs:113-116`;
ADR-0003:84-88; process-level pin for `text` (`tests/e2e/template.rs:108`
under a 15 s watchdog) and for `init` defaults (`tests/e2e/init.rs:23`).
Caveat: unit tests self-skip when run under a real TTY (H1).

**G8: Empty-selection contract runs before the TTY guard.** *class X · verdict intentional · pinned unit · Gherkin: unit material*
Contract: `select(&[])` → `Err(EmptySelectionInput)` in every mode;
`multi_select(&[])` → `Ok([])` (never an error).
Evidence: `src/dialog/terminal.rs:79-99`; ADR-0003:84-87; pins
`src/dialog/terminal.rs:215,229`.

**G9: Selection returns positions, not labels.** *class I · verdict intentional (ADR-0003) · pinned unit · Gherkin: unit material*
Contract: `select`/`multi_select` return zero-based indices; duplicate labels
stay distinguishable; the caller recovers the value by position; preset queues
that hold an out-of-bounds index fail with `InvalidConfiguration` rather than
silently choosing.
Evidence: ADR-0003:55-88; `src/dialog/mod.rs:22-27,140,105`;
pins `src/dialog/preset.rs:373,385,350`; object-safety compile guard
`src/dialog/mod.rs:173` (ADR-0003:129-131).

**G10: Preset replay is deterministic with a defined fallback.** *class I · verdict intentional · pinned unit · Gherkin: unit material*
Contract: queued answers replay FIFO; an exhausted queue falls back to the
prompt default, else `""`/`false`/`0`/`[]`; `is_interactive()` is
`true` only while any queue is non-empty.
Evidence: `src/dialog/preset.rs:173-175`; pins `src/dialog/preset.rs:332,340,407`.

**G11: The template picker refuses to run non-interactively.** *class E · verdict intentional · pinned unit · Gherkin: good (unpinned at process level)*
Contract: `template`/`-i` with no name and no templates → `traces::cli::template::no_templates`;
with templates but a non-interactive provider → `TemplatePicker { NotInteractive }`;
a cancel propagates as User Abort. Evidence: `src/cli/template.rs:236-261`
(guard at `:244`); pins `src/cli/template.rs:426,665`.
Gap: no spawned test runs `traces template` (no `-i`) or bare `traces -i`,
so the exit-1 + code at the process boundary is unpinned.

**G12: `--no-input` swaps the provider wholesale.** *class I · verdict intentional · pinned unit · Gherkin: good*
Contract: `--no-input` discards the injected/Terminal provider and installs an
empty `PresetDialogProvider`, so every `ui.*` call takes its default;
independent of `--dry-run`. Evidence: `src/cli/template.rs:168-176`, flag doc
`src/cli/template.rs:70-74`; pins `src/cli/template.rs:577`; used implicitly by
nearly every e2e template run.

**G13: Output collision: prompt only when interactive, refuse non-destructively otherwise.** *class E · verdict intentional · pinned e2e (non-interactive) · Gherkin: good*
Contract: without `-f`, if `-o` was not passed, the target exists and the
provider is interactive → prompt for an alternative path (re-prompt when input
escapes the root); if not interactive → `traces::cli::template::output_exists`,
exit 1, target byte-identical; `-f` overwrites; `-o` writes the override path.
Evidence: `src/template/writer.rs:238-288`; pins:
`tests/e2e/template.rs:44` (refusal/force/-o at process level),
`src/template/writer.rs:681,706` (interactive branch, unit only).

**G14: `--dry-run` never touches disk.** *class I · verdict intentional · pinned e2e+unit · Gherkin: good*
Contract: renders to stdout, skips the existence check entirely, writes
nothing, and conflicts with `-o` at parse time.
Evidence: pins `tests/e2e/template.rs:210,227-228` (negative filesystem
assertion), `src/cli/template.rs:532,759`.

### `init` (`src/cli/init.rs`)

**G15: Non-TTY `init` scaffolds with defaults.** *class W · verdict intentional · pinned e2e+unit · Gherkin: good*
Contract: on null stdin, prompts short-circuit to defaults, `.traces/` +
`.traces/config.toml` are created with `templates.directory = ".traces/templates"`
and `output_dir = "."`, `initialised traces in <root>` goes to stderr, exit 0.
Evidence: `src/cli/init.rs:90-110`, `src/cli/init.rs:55-87`;
pins `tests/e2e/init.rs:23`, `src/cli/init.rs:351`.

**G16: `init` is transactional.** *class ST · verdict intentional · pinned unit · Gherkin: good*
Contract: absolute / `..` / symlink-escaping template directories →
`init::invalid_template_dir` with nothing written; existing `.traces` →
`init::already_initialized`; a mid-run failure undoes every directory it
created; an existing config file is never overwritten; cancel → nothing
written + User Abort. Evidence: `src/cli/init.rs:55-87` (undo bookkeeping),
docs at `src/cli/init.rs:36-47`; pins `src/cli/init.rs:310,378,395,418,455,486`.

**G17: `init` does not grant trust.** *class ST · verdict intentional · pinned NO · Gherkin: good (high-value gap)*
Contract: after a successful `init`, the root is still untrusted; the next
config-loading command fails `config_build_untrusted` until `traces trust` runs.
Evidence for intent: `src/cli/init.rs` never references trust (whole file);
`tests/e2e/workflow.rs:29-32` documents the requirement as a comment only.
No test fails today if `init` started granting trust: the workflow chain calls
`trust` explicitly and would pass either way.

### Config discovery, scope, trust, tracking (`src/config/`)

**G18: Nearest-local discovery, global alongside.** *class I · verdict intentional · pinned unit+e2e · Gherkin: good*
Contract: upward walk finds the nearest `.traces/config.toml` (inner wins over
outer); no local config anywhere → `NotFound` → `config_discovery_failed`,
exit 1, empty stdout; `~/.config/traces/config.toml` (via `XDG_CONFIG_HOME`) is
discovered alongside when present. Evidence: `src/config/discovery.rs:73-114`,
`src/config/mod.rs:1-11`; pins `src/config/discovery.rs:260,273,301`;
e2e `tests/e2e/config.rs:109` (missing local), `tests/e2e/config.rs:243`
(global leg visible to a spawned child).

**G19: Ceiling directories bound the upward walk.** *class I · verdict intentional · pinned unit only · Gherkin: unit material*
Contract: `TRACES_CEILING_DIRS` stops the walk; the ceiling directory itself is
never inspected (even when it is the start dir) → `NotFound`.
Evidence: `src/config/discovery.rs:88-114`, `src/env_vars.rs:8-16`;
pins `src/config/discovery.rs:318,336`. The env-var plumbing itself is
never exercised end-to-end (declared opt-in at `tests/e2e/support.rs:16`).

**G20: Subtree scan prunes ignored dirs below the root, exempting the root.** *class I · verdict intentional · pinned unit · Gherkin: unit material*
Contract: descendant-config collection skips `.git`, `target`, `node_modules`,
`.traces` + `TRACES_IGNORED_DIRS` beneath the walk root, but an ignored name on
the root itself is scanned; empty subtree falls back to the enclosing config,
excluding sibling configs.
Evidence: `src/config/discovery.rs:141-176` (prune via inherent
`DirTree::filter`, root exempt at `src/dirtree.rs:149-177`);
pins `src/config/discovery.rs:374,387,400,434`.

**G21: Two-level Configuration Scope, merged field-by-field.** *class I · verdict intentional · pinned unit · Gherkin: good*
Contract: local overrides global per setting; for `tag_filters`, `tasks`
statuses and `aliases` a non-empty local list *replaces* the global list while
an absent/empty local list *falls back* to global; template/schema directories
resolve against the root of the config file that declared them (global dir
never resolves against the local project root).
Evidence: `src/config/GLOSSARY.md:17-21,107-113`, `src/config/builder.rs:1-7`;
pins `src/config/builder.rs:322,356,424` and siblings.

**G22: Config load has four ordered failure stages with distinct codes.** *class E · verdict intentional · pinned e2e · Gherkin: good (best scenario in the set)*
Contract: discovery → trust → parse → build. `traces trust` hashes bytes
*without parsing*, so a malformed config can be trusted first and the next load
fails at parse; an invalid empty field key fails at build after trust; editing
config after trust fails at trust with the `untrusted: stale` flavor; a never
trusted root fails at trust with the never-trusted flavor.
Evidence: `src/config/mod.rs:1-11`, `src/config/file.rs:214-236`,
`src/config/service.rs:90-113`; e2e pins the whole table:
`tests/e2e/config.rs:109,134,176,208,319` (defect headers
`unwitnessed-contract (failure-stage table)` at `:107,132`,
`trust-before-parse confusion` at `:174,206`).

**G23: Four Trust Statuses with distinct observable meaning.** *class ST · verdict intentional · pinned e2e+unit · Gherkin: good*
Contract: `Untrusted` (root not in the store) / `MissingBaseline` (root
trusted, no companion hash) / `Stale` (stored BLAKE3 ≠ current file bytes) /
`Trusted` (bytes verified once and reused for parsing); the CLI message names
root, config path and status (`… is untrusted: missing baseline`).
Evidence: `src/config/trust.rs:84-107,240-259`, `src/config/GLOSSARY.md:31-37`;
pins `src/config/trust.rs:388,479,502`, `src/config/file.rs:448,471,485`,
e2e `tests/e2e/config.rs:134` (stale flavor), `:319` (untrusted).

**G24: No config loading without trust, across processes.** *class ST · verdict intentional · pinned e2e+unit · Gherkin: good*
Contract: `index`, `list`, `table`, `task`, `template`, `completions
--list-templates` all fail exit 1 with `config_build_untrusted` on an untrusted
root, and a `untrust` in one process is honored by the next process.
Evidence: unit pins per command
(`src/cli/index.rs:136`, `src/cli/table.rs:554`, `src/cli/task.rs:834`,
`src/cli/template.rs:371`, `src/cli/completions.rs` `fails_with_config_build_…`);
e2e `tests/e2e/trust.rs:14`, `tests/e2e/config.rs:319`,
`tests/e2e/process_contract.rs:88`; ADR-0002:107-112 (gating load gates
templates).

**G25: The global config layer is never trust-gated.** *class I · verdict intentional · pinned e2e · Gherkin: good*
Contract: global config is read and parsed without any trust check; a malformed
global config fails the whole load for a trusted local, with the planted
unknown-field message proving provenance.
Evidence: `src/config/service.rs:104-108`, `src/config/file.rs:290-299`;
pins `tests/e2e/config.rs:285`, `tests/e2e/env_policy.rs:51`;
ADR-0002:137-138 (global auto-trusted carve-out).

**G26: Grant tracks before it trusts; load tracks best-effort; untrust keeps the track.** *class I · verdict intentional (ADR-0002 update) · pinned unit · Gherkin: good*
Contract: `ConfigService::trust` records the config path first and aborts the
grant if recording fails (root stays untrusted); a root-only grant records
nothing; `load` records best-effort *before* verification so refused configs
are still tracked and a tracker failure never fails the load; `untrust` leaves
the path tracked.
Evidence: `src/config/service.rs:128-135`, `src/config/file.rs:214-219`,
`src/config/GLOSSARY.md:45-63`, ADR-0002:67-73;
pins `src/config/service.rs:446,476,515,533,589`, `src/config/file.rs:497`.

**G27: Stores live under env-resolvable state/config homes; cleans report counts.** *class W · verdict intentional · pinned e2e · Gherkin: good*
Contract: trust/tracked stores resolve under `$TRACES_STATE_DIR` else
`$XDG_STATE_HOME/traces` (`src/dirs.rs:159-162`); global config under
`$XDG_CONFIG_HOME/traces`; `trust clean` / `tracked clean` prune dangling
entries and print exact singular/plural messages (`removed 1 stale trust entry`,
`removed 1 stale tracked entry`).
Evidence: `tests/e2e/env_policy.rs:18,100`; exact strings
`tests/e2e/workflow.rs:180`, `tests/e2e/tracked.rs:51`; ADR-0002:58-65.

**G28: Child environment is an explicit default-deny policy.** *class I · verdict intentional (harness-level) · pinned e2e · Gherkin: good but step-heavy*
Contract: children get only sandbox-owned `TRACES_STATE_DIR`,
`XDG_CONFIG_HOME`, `HOME`, `TZ=UTC` plus a passthrough allowlist (`PATH`, …);
everything else is dropped; `TRACES_CEILING_DIRS`/`TRACES_IGNORED_DIRS` are
opt-in only. Ambient `TZ` must not shift naive datetime rendering;
`TRACES_IGNORED_DIRS` applies only when explicitly declared.
Evidence: `tests/e2e/support.rs:4-46,247-300`; pins (incl. negative controls)
`tests/e2e/env_policy.rs:18,51,100,134,190`.

**G29: Queries re-sync the index; `index` rebuilds rather than appends.** *class W · verdict intentional · pinned unit+e2e · Gherkin: good*
Contract: every `list`/`table`/`task` refreshes the persisted index before
querying, so an edit made between two invocations is visible without an
explicit `index` command; `traces index` rebuilds the store; rendered template
output re-enters the index for the next spawned query.
Evidence: `src/cli/mod.rs:325-366`; pins `src/cli/mod.rs:1178`,
`src/cli/index.rs:117`, process-level `tests/e2e/template.rs:144`.

**G30: Query output shapes are stable Markdown.** *class I · verdict intentional · pinned unit (weakly for default order) · Gherkin: good*
Contract: `list` → one `- <file.path>` bullet per matched page;
`table` → one `--column` per flag, header row + separator, `|` in a header
escaped as `\|`, cells aligned; `task` → checkbox lines (or `--table` with
default columns). Evidence: pins `src/cli/list.rs:121`,
`src/cli/table.rs:229,252`, `src/cli/task.rs` (`renders_a_checkbox_line_per_task_in_document_order`),
process-level `tests/e2e/workflow.rs:44-63`.
Caveat: the default (no `--sort`) ordering test creates `a.md` then `b.md`, so
it cannot distinguish sorted order from index insertion order
(`src/cli/list.rs:121-137`): the default-order guarantee is weakly pinned.

## 2. Contradictions: docs vs code

1. **Exit code of `CommandOutcome` (stale doc).** `src/cli/mod.rs:54-55` says
   "The process exit code is `0` for both variants" of `CommandOutcome`, but
   `src/main.rs:27-29` maps `Aborted(Interrupted)` → `130`, matching
   ADR-0004:41. The doc sentence is false for the `Aborted(Interrupted)`
   variant (likely predates ADR-0004). BDD specs should follow ADR/main.rs.
2. **Directory move ⇒ "stale"?** ADR-0002:82-83 claims a moved directory leaves
   the trust entry "stale", but trust is keyed by a hash of the *canonical*
   path (`src/config/trust.rs:245-252`), so a moved/renamed root resolves as
   `Untrusted` (unknown root), never `Stale`; `Stale` is reserved for content
   mismatch (`src/config/trust.rs:93-94`, `src/config/GLOSSARY.md:33-36`).
   ADR-0002:98-99 itself says renaming "invalidates trust": the ADR is
   internally inconsistent, code sides with `Untrusted`.
3. **ADR-0002 "existence check only" hot path.** ADR-0002:84-85 promises "no
   config file parsing … just a file existence check" and ADR-0002:107-111
   describes the check as happening "before a local config candidate is read".
   Current verification reads and hashes the config during the check
   (`src/config/trust.rs:248-259`); only *TOML parsing* is deferred. The
   "cheap hot path" consequence is stale after the companion-hash addition.
4. **ADR-0003 test names/locations do not exist.** ADR-0003:119-131 cites
   `select_index_recovers_the_object_by_position`,
   `select_index_disambiguates_duplicate_labels`,
   `select_index_on_empty_items_errors` in `src/dialog/mod.rs`; the real tests
   are `recovers_the_object_by_position` (`src/dialog/preset.rs:373`),
   `disambiguates_duplicate_labels` (`src/dialog/preset.rs:385`) and
   `returns_error_when_items_are_empty` (`src/dialog/terminal.rs:215`,
   `src/dialog/preset.rs:340`), and `src/dialog/mod.rs` holds only
   `provider_is_send_and_sync` + `is_interactive_default_returns_true`.
   Behavior matches the ADR; the confirmation citations do not.
5. **Misleading sort test names.** `sort_orders_by_the_named_field_ascending_by_default`
   (`src/cli/list.rs:139`, `src/cli/table.rs:148`) actually passes `asc: true`;
   the real default for unprefixed `--sort` keys is **descending**
   (`src/cli/mod.rs:843`, `src/query/sort.rs:264-272`, flag doc
   `src/cli/mod.rs:289`). Behavior and flag docs agree; two test names contradict
   the behavior they exercise: rename before mining them for scenarios.

## 3. Behaviors hostile or untestable in Gherkin

- **H1: TTY-conditional tests pass vacuously.** Every `TerminalDialogProvider`
  fallback unit test self-skips when stdin is a real terminal
  (`src/dialog/terminal.rs:123-132`), so a developer's interactive run silently
  loses that coverage; the true interactive branch (inquire rendering,
  `raw_prompt()` index reads at `src/dialog/terminal.rs:86-107`) has **no test
  at all**. Real-TTY scenarios need a pty driver: poor fit for Gherkin steps.
- **H2: Abort/130 needs a live signal.** Escape and Ctrl-C outcomes are only
  reachable by injecting fake providers (`src/cli/mod.rs:559-596,1403`,
  `src/cli/init.rs:266`, `src/cli/template.rs:301`); no spawned test can send
  Escape, and proving exit 130 needs SIGINT delivery. Scenario wording like
  "When the user presses Escape" has no step implementation without a pty/PTY
  harness or an injected-provider seam.
- **H3: Miette wraps stderr, so exact text is unreliable.** Long paths and
  causal chains wrap at fixed width with `│`/`├─▶` glyphs possibly mid-path
  (`tests/e2e/support.rs:32-46`); assert diagnostic codes and short bullets
  only (`plain()` at `tests/e2e/support.rs:162`).
- **H4: Environment is process-global and cached.** `TRACES_CEILING_DIRS`,
  `TRACES_IGNORED_DIRS` and all `dirs` paths are `LazyLock` statics
  (`src/env_vars.rs:8,25`, `src/dirs.rs:91,161`) read once per process, so an
  env-var scenario requires one process per value; the harness does this by
  re-spawning the *test binary itself* with `--ignored --exact`
  (`tests/e2e/support.rs:63-73`): expressible in Gherkin, but only as a
  two-process step ("run this test in a child with `TZ=…`"), which reads badly
  as a product-level Given.
- **H5: cwd is process-global.** In-process tests need `CwdGuard`
  (`src/cli/cwd.rs`, `src/cli/mod.rs:36-37`); Gherkin steps that change
  directory must spawn a process (as `Sandbox::command` does,
  `tests/e2e/support.rs:357-362`) rather than mutate shared state.
- **H6: "Does not hang" is timing-based.** The non-TTY no-hang guarantee is
  enforced by a 15 s kill watchdog (`tests/e2e/support.rs:413-454`,
  `tests/e2e/template.rs:108-120`): a flaky-by-design threshold that Gherkin
  scenarios should treat as a single explicit regression case, not a general
  step.
- **H7: Printed paths need canonicalization.** macOS `/tmp` → `/private/tmp`
  and Windows 8.3 aliases make string equality wrong; compare canonicalized
  identity (`tests/e2e/support.rs:126-153`). Any "Then the output names the
  project" step must bake this in.
- **H8: Interactive collision prompt is a multi-turn conversation.** The
  prompt/re-prompt-on-escaping-input loop (`src/template/writer.rs:238-288`,
  unit pins `src/template/writer.rs:681,706`) is dialog-stateful and only
  unit-testable today; likewise the interactive half of `init`.
- **H9: Store contents are opaque.** Trust/tracked entries are hash-named
  symlinks (ADR-0002:100-101); scenarios must assert via `trust list` /
  `trust --show` output (`tests/e2e/workflow.rs:114-135`), never raw store
  directory listings.

## 4. Notes for later tickets

- Interactive surface is narrow: only `init` and `template` take dialog input
  (`src/cli/mod.rs:186-207`); `trust`, `untrust`, `index`, `list`, `table`,
  `task`, `completions`, `tracked` never construct a provider: their entire
  behavior is spawn-friendly and Gherkin-shaped.
- Highest-value unpinned gaps found: **G17** (init does not trust),
  **G11** (picker at process boundary), **G5**'s 130/no-diagnostic halves,
  **G19**'s env plumbing, **G30**'s default ordering.
- The `//! Proves` / `Defect class:` header convention already names the
  defects each e2e file guards; those strings are a ready-made checklist for
  scenario coverage mapping (see map.md "Not yet specified" on their fate).

# Design: opencode guard plugins (code-content enforcement)

Date: 2026-10-12
Status: approved (design), pending implementation

## Context

Running `ctxlint` against this environment flagged ~14 ALWAYS/NEVER rules in
`~/.config/opencode/AGENTS.md` as "inviolable framing without a hook" — prose
rules with no mechanical enforcement. The user wants to convert the
mechanically-checkable rules into real enforcement, using opencode plugins for
code-content rules and opencode `permission` config for bash-command rules
(globs cannot check command context, so only exact-shape commands qualify).

Decision record (from brainstorming):

- Plugin scope: **global** (`~/.config/opencode/plugins/`), matching where the
  flagged AGENTS.md rules live.
- Languages: **Rust + Python**.
- Enforcement: **mixed severity** — dangerous rules block, advisory rules warn.
- Structure: **one plugin per language**, self-contained, zero dependencies.
- Bash-command rules go in **config permissions**, not plugins.

## Architecture

```
~/.config/opencode/plugins/
  rust-guard.js    # Rust rules (auto-discovered by opencode)
  py-guard.js      # Python rules (auto-discovered by opencode)
```

Each plugin is a self-contained JS module, `default` export returning an async
plugin function that registers:

- `tool.execute.before`: inspect `edit` (`filePath`, `newString`) and `write`
  (`filePath`, `content`) args. Only **new** content is scanned — never the
  file on disk — so pre-existing violations never block unrelated edits.
  - BLOCK rules throw on violation with a message naming the rule id, the
    offending line, and the fix, so the agent can self-correct.
  - WARN rules pass through untouched.
- `tool.execute.after`: for WARN-rule hits recorded during `before`, append
  `[rule-id] message` to the tool output string so the model sees the advisory.

The two plugins duplicate ~25 lines of hook boilerplate instead of sharing a
helper module: opencode auto-loads every `*.js` in the plugins dir, so a shared
helper file would itself be loaded as a (broken) plugin. Duplication is
trivial; each file stays readable alone.

## Rules

### rust-guard.js (`.rs` files)

| id               | severity | detection                                              |
| ---------------- | -------- | ------------------------------------------------------ |
| `no-unwrap`        | BLOCK    | `.unwrap(` in new lines                                  |
| `safety-comment`   | BLOCK    | `unsafe {` / `unsafe fn` / `unsafe impl` without a `// Safety:` comment on the nearest preceding non-empty line(s) — scan back at most 5 lines |
| `no-println`       | BLOCK    | `println!` or `dbg!(` in new lines                       |
| `expect-text`      | WARN     | `.expect()` with empty args                             |

### py-guard.js (`.py` files)

| id                | severity | detection                                        |
| ----------------- | -------- | ------------------------------------------------ |
| `no-any`            | WARN     | `Any` used as a type annotation                    |
| `no-sys-python`     | WARN     | `python` / `python3` invoked without `uv` prefix   |

### Exemptions (both plugins)

Skip scanning when the file path contains any of:

- `tests/`, `examples/`, `benches/`
- `test_` (test modules), `_test.py`, `conftest.py`

### Accepted v1 limitations

- No comment/string stripping: `.unwrap(` inside a comment or doc-test can
  false-positive a BLOCK. Accepted noise for v1; revisit if it bites.
- WARN rules may fire on prose matches; harmless since they only append
  advisory text.

## Config updates

`~/.config/opencode/opencode.jsonc`, inside the existing `permission.bash`
object (insertion order preserved; existing entries untouched):

```jsonc
"cargo build --features python*": "deny",
"*target-cpu=native*": "deny",
"cargo bench *": "ask",
```

Rationale:

- `cargo build --features python` is banned by AGENTS.md (must use maturin).
- `target-cpu=native` in any RUSTFLAGS invocation is banned for benchmarks.
- `cargo bench` is forced through `ask` because release-vs-debug bench mode
  cannot be expressed as a glob; the confirmation moment covers it.

The remaining flagged bash-ish rules (uv/maturin discipline, release-mode
benches) stay advisory — they need command-context checks that neither globs
nor content scanning can do.

## Block/warn message format

Block: `[rust-guard] no-unwrap: AGENTS.md bans .unwrap() in library code —
use .expect("reason") for invariants or handle the Result. Offending line: ...`

Warn: appended to tool output as
`[rust-guard] expect-text: .expect() should carry a descriptive message.`

## Verification

1. `node --check` both plugin files for syntax.
2. Restart opencode (config/plugins load once at startup).
3. Scratch session checks:
   - Edit a temp `.rs` file adding `.unwrap()` → BLOCK with rule id in message.
   - Edit adding `.expect()` → WARN appended to tool output.
   - Edit under `tests/` with `.unwrap()` → allowed (exemption).
   - `cargo build --features python` → denied by permission config.

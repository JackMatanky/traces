# Design: dash-guard plugin (prose dash asides)

Date: 2026-10-12
Status: implemented (inline, per user's speed preference — same architecture as the reviewed rust-guard/py-guard)

## Context

Follow-up to the guard-plugins work: the user wants agent prose to avoid
em-dash asides when commas/parens would do. Mechanical detection of
"grammatical necessity" is impossible, so the plugin warns on dash usage and
the agent rephrases.

## Decisions (from brainstorming)

- Surface: **file writes only** (edit/write hooks), not chat replies.
- Severity: **warn only** (prose rule; matches py-guard convention).
- Scope: prose files (`.md`, `.txt`) — all lines; code files (`.rs`, `.py`,
  `.js`, `.ts`, `.mjs`, `.go`) — comment lines only (trimmed line starts with
  `//`, `/*`, `*`, or `#` minus shebangs).
- Violations: `—` (U+2014), `–` (U+2013), and spaced `--` (`(^|\s)--(\s|$)`,
  so CLI flags, markdown `---` rules, `-->` arrows, and `|---|` tables stay
  clean).

## Files

- `~/.config/opencode/guard-lib/dash-scan.js` — pure `scanDash()`, named
  exports only, zero deps.
- `~/.config/opencode/plugins/dash-guard.js` — single default export,
  callID-keyed warn queue (128 cap), notes appended to tool output as
  `[dash-guard] em-dash (line N): <forms>. <message>`.

## Accepted limitations (line-based design, same class as sibling guards)

- Mid-line trailing comments (`let x = 1; // note — aside`) are not scanned;
  only lines starting with a comment marker.
- Em-dashes inside code strings and markdown fenced code blocks in `.md` files
  can false-positive (warn-only, tolerable).
- Docstrings and block-comment interiors between `/*` and `*` lines are only
  caught when each line starts with a comment marker.

## Verification

- `node --test guard-tests/` — 13 dash-guard tests plus the 39 rust/py tests:
  52/52 pass. Covers both dash chars, spaced `--`, false-positive shapes
  (hr, arrows, flags, pipes, shebangs, strings), comment-style filtering per
  language, hook append/drain/interleaving/write-branch/bad-args.

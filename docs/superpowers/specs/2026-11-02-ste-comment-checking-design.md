# STE checker for code comments design

**Status:** approved design, pre-plan
**Date:** 2026-11-02
**Prototype:** branch `throwaway/ste-comment-mask-prototype`, commit `7090bee9`, `.scratch/prototypes/ste-comment-mask-prototype.py`

## Problem

The `ste` skill and its checker (`scripts/check_ste.py`) are Markdown-only. Fed a
source file, they produce garbage (77 errors on `src/note/parser.rs`, sentence spans
over code lines, `fix` would rewrite identifiers like `fn utilize()`), and comment
text pasted into the checker only partially works: four grammar warnings (R6, R8,
R10, R20) silently never fire because the `//` prefix defeats list detection.
Comment support must become a designed capability.

## Decisions (validated with the user and by prototype stress test)

1. **Mode selection by file type.** `check file.rs` auto-selects comment mode for
   code extensions and Markdown mode otherwise. No `--comments` flag for file
   inputs. Stdin (`check -`) stays Markdown mode unless `--language` is passed;
   `--language` is only valid together with stdin.
2. **Languages first class:** Rust (`.rs`), Python (`.py`), TS/JS (`.ts .tsx .js
   .mjs .cjs`), Go (`.go`), C (`.c .h .cc .cpp .hpp`). Unknown file extensions fall
   back to Markdown mode per automatic mode selection; explicit stdin `--language`
   values are argparse-restricted and reject unknown language names (exit 2).
3. **Mechanism: mask to whitespace.** A string-aware scanner converts every byte
   outside comment prose to spaces (newlines preserved, length identical to the
   original). The masked text is then checked by the unchanged Markdown pipeline:
   real file/line/column fall out by construction, and `fix` can only ever mutate
   comment spans.

### Stress-test verdicts folded in (all prototype-verified)

- **Doc/continuation prefixes must be blanked per line inside comment spans:**
  `///`, `//!` residue (`/ Extracts:`) and ` * ` continuation residue (`* STILL?`)
  otherwise corrupt block classification and produce phantom R16/R2 findings.
  Language spec carries a per-line prefix policy; the phantom findings disappeared
  once implemented.
- **Everything outside comments is blanked, not just strings.** Strings, char
  literals, raw strings, and templates are scanning context (they stop `//`, `/*`,
  `*/`, `#` inside string literals from opening comments), and they get fully
  blanked. Bare identifiers (`fn`, `utilize`) are blanked as plain code bytes.
- **TS regex literals need a start-guard heuristic** (`/` preceded by
  `=(,:[!&|?+;{~^` and closing before line end, char classes respected).
  Counterexample-verified: `a / b` division must not enter regex mode; an
  unterminated regex candidate must fall through to plain code, never swallow the
  rest of the file.
- **Python: all triple-quoted strings are documentation prose** (interior kept,
  delimiters blanked). Deterministic; also lints long string constants. Documented
  in SKILL.md.
- **Rust block comments nest; C block comments do not** (first `*/` closes).
- **C line comments splice across lines** with a trailing backslash.
- **R16/R2 phantom findings were prefix-residue artifacts**, not rule-scoping
  problems: after per-line prefix blanking, real doc comments produce only genuine
  findings (verified: "via" is an R13 hit; a legit 30-word doc sentence is an R2
  hit; `; ` is an R3 hit). No rule-scope exclusion list is needed.
- **R11 (synonym pairs) is pedantic at document level on comments** but warn-level;
  accepted as-is.
- **Limitation documented:** comment lines containing `|` or starting with `<` are
  "raw" blocks to the Markdown parser and go unchecked (survivor bias), consistent
  with existing Markdown policy.

## Component: `mask_to_prose(text, language) -> str`

Pure function, kept inside the single zero-dependency uv script (per repository
convention). Per language:

- line comments (`//`, `#`), block comments (nesting flag), python docstrings
  (prose), line-splice flag (C), regex guard (ts/js), per-line prefix policy.
- Output invariants asserted in tests: identical length, identical newline
  positions, and every non-kept byte is a literal space.

Precision: no new kinds of findings, no new checkers; the existing
R1-F1 rule set, config, `dict`, `render-dict`, `--only/--ignore`, JSON shape, and
exit codes are untouched.

## `fix` semantics

Run `replace_auto` on the masked text; splice replacements back into the original
at identical coordinates (match start/end in masked text equal coordinates in the
original). Positive-control test required in the plan: a comment containing an
auto entry ("prior to") must change only inside the comment; the negative control
from the review (`let utilize_buffer = 1;` next to a rewritten comment) must stay
byte-identical.

## CLI surface

- `check file.rs` → comment mode (by extension); findings show the real filename,
  line, column. Empty comment prose → clean result.
- `fix file.rs` → comment mode fix, same auto-substitution policy.
- `check -` → Markdown mode (unchanged). `check - --language rs` → comment mode
  from stdin; `--language` without stdin → exit 2.
- `--compare`, `dict`, `render-dict` unchanged (`--compare` stays raw-text and is
  valid for comment-only rewrites).

## Docs

SKILL.md gains a short "Code comments" note in Verify: mode selection, supported
languages, and the two documented limitations (regex-guard heuristic; `<`/`|`
lines unchecked). RULES.md and DICTIONARY.md unchanged; parity tests still hold.

## Testing

- Per language: layout invariants, string-literal safety (comment markers inside
  strings), doc continuation prefixes, nested Rust blocks, C splice, TS regex
  guard with the division counterexample, Python docstrings.
- End-to-end RED/GREEN: a real `src/` file renders comment-only findings with
  correct coordinates; garbage findings from direct-code checking disappear.
- Fix positive and negative controls as above.
- Unknown file extensions fall back to Markdown mode; stdin-without-language returns 2.

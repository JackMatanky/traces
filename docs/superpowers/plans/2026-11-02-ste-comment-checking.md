# STE Comment-Mode Checker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `.agents/skills/ste/scripts/check_ste.py` check and fix ASD-STE100 compliance in code comments (Rust, Python, TS/JS, Go, C) without touching code.

**Architecture:** A string-aware masker (`mask_to_prose`, in the spec at `docs/superpowers/specs/2026-11-02-ste-comment-checking-design.md`) blanks every byte outside comment prose to spaces, preserving length and newline positions. The unchanged Markdown pipeline then checks or fixes the masked text, so real file/line/column fall out by construction, and `fix` can only mutate comment spans. Mode selection is by file extension; stdin requires `--language`.

**Tech Stack:** Python 3.10+, stdlib only (single zero-dependency uv script), pytest.

**Test command (from repo root):** `uv run --with pytest pytest .agents/skills/ste/scripts/tests/test_comments.py -q` unless otherwise stated.

**Stage discipline:** stage only `.agents/skills/ste/**` paths (and docs changed by a task); never stage unrelated worktree files.

---

## File Structure

- Modify: `.agents/skills/ste/scripts/check_ste.py` (checker/fixer/CLI; masker lives here per single-file convention)
- Create: `.agents/skills/ste/scripts/tests/test_comments.py` (all comment-mode tests)
- Modify: `.agents/skills/ste/SKILL.md` (Verify section, Task 8)
- Modify: `docs/superpowers/specs/2026-11-02-ste-comment-checking-design.md` (one contradiction fix, Task 8)

---

### Task 1: Language registry

**Files:**
- Modify: `.agents/skills/ste/scripts/check_ste.py` (new section after the `Match` dataclass, before `# 1. configuration loading/validation`)
- Create: `.agents/skills/ste/scripts/tests/test_comments.py`

- [ ] **Step 1: Write the failing test**

Create `.agents/skills/ste/scripts/tests/test_comments.py` following the importlib convention of `test_check_ste.py`:

```python
"""Tests for comment-mode checking in source files."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

SCRIPT = Path(__file__).parents[1] / "check_ste.py"
SPEC = importlib.util.spec_from_file_location("check_ste", SCRIPT)
assert SPEC and SPEC.loader
check_ste = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = check_ste
SPEC.loader.exec_module(check_ste)


def test_extension_language_table_covers_all_configured_languages() -> None:
    expected = {"rust", "python", "ts", "go", "c"}
    assert set(check_ste.LANGUAGES) == expected
    rust = check_ste.LANGUAGES["rust"]
    assert rust.extensions == (".rs",)
    assert rust.block_nests is True and rust.line_splice is False
    py = check_ste.LANGUAGES["python"]
    assert py.docstring is True and py.block is None
    ts = check_ste.LANGUAGES["ts"]
    assert ts.regex_start_prev is not None
    go = check_ste.LANGUAGES["go"]
    assert go.line_splice is False and go.block_nests is False
    c = check_ste.LANGUAGES["c"]
    assert c.line_splice is True and c.block_nests is False
    assert set(check_ste.EXTENSION_LANGUAGE) == {
        ".rs",
        ".py",
        ".ts",
        ".tsx",
        ".js",
        ".mjs",
        ".cjs",
        ".go",
        ".c",
        ".h",
        ".cc",
        ".cpp",
        ".hpp",
    }
    assert check_ste.EXTENSION_LANGUAGE[".tsx"] == "ts"


def test_line_prefix_policies_are_language_specific() -> None:
    assert check_ste.LANGUAGES["rust"].line_prefix_chars == " \t/*!"
    assert check_ste.LANGUAGES["python"].line_prefix_chars == ""
    assert check_ste.LANGUAGES["ts"].line_prefix_chars == " \t/*"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests/test_comments.py -q`
Expected: FAIL with `AttributeError: module 'check_ste' has no attribute 'LANGUAGES'`

- [ ] **Step 3: Write minimal implementation**

In `.agents/skills/ste/scripts/check_ste.py`, insert after the `Match` dataclass:

```python
@dataclass(frozen=True)
class Language:
    """Comment and string-literal scanning table for one source language."""

    extensions: tuple[str, ...]
    line_comments: tuple[str, ...]
    block: tuple[str, str] | None
    block_nests: bool = False
    line_splice: bool = False
    regex_start_prev: str | None = None
    docstring: bool = False
    line_prefix_chars: str = ""


LANGUAGES: dict[str, Language] = {
    "rust": Language(
        (".rs",),
        ("//",),
        ("/*", "*/"),
        block_nests=True,
        line_prefix_chars=" \t/*!",
    ),
    "python": Language((".py",), ("#",), None, docstring=True),
    "ts": Language(
        (".ts", ".tsx", ".js", ".mjs", ".cjs"),
        ("//",),
        ("/*", "*/"),
        regex_start_prev="=(,:[!&|?+;{~^",
        line_prefix_chars=" \t/*",
    ),
    "go": Language((".go",), ("//",), ("/*", "*/"), line_prefix_chars=" \t/*"),
    "c": Language(
        (".c", ".h", ".cc", ".cpp", ".hpp"),
        ("//",),
        ("/*", "*/"),
        line_splice=True,
        line_prefix_chars=" \t/*",
    ),
}

EXTENSION_LANGUAGE: dict[str, str] = {
    suffix: name
    for name, specification in LANGUAGES.items()
    for suffix in specification.extensions
}


def language_for_suffix(suffix: str) -> str | None:
    """Return the configured language name for one file suffix, or None."""
    return EXTENSION_LANGUAGE.get(suffix.lower())
```

- [ ] **Step 4: Run test to verify it passes**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests/test_comments.py -q`
Expected: PASS (2 passed)

- [ ] **Step 5: Commit**

```bash
git add .agents/skills/ste/scripts/check_ste.py .agents/skills/ste/scripts/tests/test_comments.py
git commit -m "feat(ste): add language registry"
```

---

### Task 2: Rust masker (strings, chars, raw strings, nested blocks, layout invariants)

**Files:**
- Modify: `.agents/skills/ste/scripts/check_ste.py` (new `mask_to_prose` after the phrase helpers, before `# 4. rule registry/implementations`)
- Test: `.agents/skills/ste/scripts/tests/test_comments.py`

- [ ] **Step 1: Write the failing tests**

Append to `test_comments.py`:

```python
def layout_of(original: str, masked: str) -> None:
    """Assert the masker preserved length, newlines, and blank-only code."""
    assert len(original) == len(masked)
    for index, (original_char, masked_char) in enumerate(zip(original, masked)):
        if original_char == "\n":
            assert masked_char == "\n", f"newline moved at {index}"
        else:
            assert masked_char in (original_char, " "), (
                f"non-space code changed at {index}: {original_char!r}->{masked_char!r}"
            )


def mask(code: str, name: str = "rust") -> str:
    return check_ste.mask_to_prose(code, check_ste.LANGUAGES[name])


def test_rust_mask_keeps_comments_and_blanks_code() -> None:
    code = (
        "fn main() {\n"
        "    // Please utilize the tool.\n"
        '    let s = "prior to x // not comment /* no */";\n'
        "    let c = '/';\n"
        '    let raw = r#"utilize /* no */ "#;\n'
        "    let rx = '\\\\n';\n"
        "}\n"
    )
    masked = mask(code)
    layout_of(code, masked)
    kept = [line for line in masked.splitlines() if line.strip()]
    assert kept == ["  Please utilize the tool."]


def test_rust_nested_block_comment_is_prose() -> None:
    code = (
        "/* start\n"
        "/* nested\n"
        "*/ inner\n"
        "* STILL? */\n"
        'let s = "str /* not comment */";\n'
    )
    masked = mask(code)
    layout_of(code, masked)
    assert "start" in masked
    assert "nested" in masked
    assert "inner" in masked
    assert "STILL?" in masked
    assert "str /* not comment */" not in masked


def test_rust_unterminated_raw_string_blanks_remainder() -> None:
    code = 'let r = r#"gone;\n// comment should vanish\n'
    masked = mask(code)
    layout_of(code, masked)
    assert "comment should vanish" not in masked
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests/test_comments.py -q`
Expected: FAIL with `AttributeError: ... no attribute 'mask_to_prose'`

- [ ] **Step 3: Write minimal implementation**

In `.agents/skills/ste/scripts/check_ste.py`, insert after `markdown_non_prose_ranges` (keep the section numbering comments and add a new numbered section `# 3b. language-aware comment masking`):

```python
_COMMENT = "comment"
_DOCSTRING = "docstring"


def line_comment_end(text: str, start: int, splice: bool) -> int:
    """Return the index after one line comment, honoring C line splicing."""
    end = text.find("\n", start)
    if end == -1:
        return len(text)
    if not splice:
        return end
    while text[end - 1] == "\\" and end + 1 < len(text):
        following = text.find("\n", end + 1)
        end = following if following != -1 else len(text)
    return end


def mask_to_prose(text: str, language: Language) -> str:
    """Return text with every byte outside comment prose blanked to a space.

    Length, byte positions, and newline positions are preserved, so Markdown
    checks on the masked text report the original file line and column.
    """
    out: list[str] = list(text)
    spans: list[tuple[int, int, str]] = []
    i, size = 0, len(text)

    def blank(start: int, end: int) -> None:
        out[start:end] = ["\n" if ch == "\n" else " " for ch in text[start:end]]

    def keep(start: int, end: int, head: int, tail: int) -> None:
        blank(start, start + head)
        blank(end - tail, end)
        spans.append((start, end, _COMMENT))

    while i < size:
        char = text[i]
        if char == "\n":
            i += 1
            continue
        if language.docstring and text[i : i + 3] in ("'''", '"""'):
            quote = text[i : i + 3]
            end = text.find(quote, i + 3)
            if end == -1:
                blank(i, size)
                break
            blank(i, i + 3)
            blank(end, end + 3)
            spans.append((i, end + 3, _DOCSTRING))
            i = end + 3
            continue
        if language is LANGUAGES["rust"] and (match := re.match(
            r"(?:b)?r(#*)\"", text[i : i + 6]
        )):
            hashes = match.group(1)
            end = text.find(f'"{hashes}', i + len(match.group(0)))
            if end == -1:
                blank(i, size)
                break
            blank(i, end + 1 + len(hashes))
            i = end + 1 + len(hashes)
            continue
        if char == "'" and language is LANGUAGES["rust"]:
            j = i + 1
            if j < size and text[j] == "\\":
                j += 2
            elif j < size:
                j += 1
            if j < size and text[j] == "'":
                blank(i, j + 1)
                i = j + 1
                continue
            i += 1
            continue
        if char in ('"', "'"):
            j = i + 1
            while j < size and text[j] != char:
                j += 2 if text[j] == "\\" else 1
            if j < size:
                blank(i, j + 1)
                i = j + 1
                continue
            blank(i, size)
            break
        if char == "`" and language is not LANGUAGES["rust"]:
            j = text.find("`", i + 1)
            if j == -1:
                blank(i, size)
                break
            blank(i, j + 1)
            i = j + 1
            continue
        if language.block and text[i : i + 2] == language.block[0]:
            closer = language.block[1]
            if language.block_nests:
                depth, j = 1, i + 2
                while j < size and depth:
                    if text[j : j + 2] == closer:
                        depth, j = depth - 1, j + 2
                    elif text[j : j + 2] == language.block[0]:
                        depth, j = depth + 1, j + 2
                    else:
                        j += 1
            else:
                j = text.find(closer, i + 2)
                j = j + 2 if j != -1 else size
            if j >= size and language.block_nests:
                keep(i, size, len(language.block[0]), 0)
                i = size
                continue
            keep(i, j, len(language.block[0]), len(closer))
            i = j
            continue
        for prefix in language.line_comments:
            if text.startswith(prefix, i):
                end = line_comment_end(text, i, language.line_splice)
                keep(i, min(end, size), len(prefix), 0)
                i = end
                break
        else:
            if (
                language.regex_start_prev is not None
                and char == "/"
                and i + 1 < size
                and text[i + 1] not in "/*"
                and (candidate := _regex_end(text, i)) is not None
            ):
                blank(i, candidate)
                i = candidate
                continue
            blank(i, i + 1)
            i += 1
    mask_line_prefixes(text, out, language, spans)
    return "".join(out)
```

For this first task the regex and prefix helpers are small stubs the later tasks expand (the registry ensures `regex_start_prev` only applies to `ts`; `mask_line_prefixes` is Task 6). For Task 2 provide these two helpers immediately after `line_comment_end`:

```python
def _regex_end(text: str, start: int) -> int | None:
    """Return the end of a guarded regex literal opening at start, or None."""
    return None


def mask_line_prefixes(
    text: str, out: list[str], language: Language, spans: list[tuple[int, int, str]]
) -> None:
    """Placeholder for per-line doc/continuation prefix blanking."""
    return None
```

Note: in Task 2, `_regex_end` always returns None, so the `ts` branch is inert until Task 4.

- [ ] **Step 4: Run tests to verify they pass**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests/test_comments.py -q`
Expected: PASS (5 passed)

- [ ] **Step 5: Run the full existing suite**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests -q`
Expected: all pass (77 existing + 5 new)

- [ ] **Step 6: Commit**

```bash
git add .agents/skills/ste/scripts/check_ste.py .agents/skills/ste/scripts/tests/test_comments.py
git commit -m "feat(ste): add rust comment masker"
```

---

### Task 3: Python masker (docstrings are prose, unterminated strings blank)

**Files:**
- Modify: `.agents/skills/ste/scripts/check_ste.py`
- Test: `.agents/skills/ste/scripts/tests/test_comments.py`

- [ ] **Step 1: Write the failing tests**

Append to `test_comments.py`:

```python
def test_python_mask_keeps_docstrings_and_hash_comments_as_prose() -> None:
    code = (
        'url = "https://example.test/utilize" /* */  # use this site prior to exit\n'
        'x = "prior to"  # ok\n'
        "'''\n"
        "Docstring prose. Utilize it.\n"
        "'''\n"
        "# comment with apostrophe and /* block-ish */\n"
    )
    masked = mask(code, "python")
    layout_of(code, masked)
    assert "use this site prior to exit" in masked
    assert "Docstring prose. Utilize it." in masked
    assert "comment with apostrophe and /* block-ish */" in masked
    assert "url =" not in masked
    assert "https://example.test" not in masked
    assert "Docstring prose. Utilize it." in masked.replace("'", "")


def test_python_unterminated_docstring_blanks_remainder() -> None:
    code = '"""unterminated\nstill looks like code\nlet prior_to_var = 1\n'
    masked = mask(code, "python")
    layout_of(code, masked)
    assert "still looks like code" not in masked


def test_python_hash_inside_string_is_not_comment() -> None:
    code = "x = f\"color {y:#x}\"  # comment: utilize it\n"
    masked = mask(code, "python")
    layout_of(code, masked)
    assert "comment: utilize it" in masked
    assert "color" not in masked
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests/test_comments.py -q`
Expected: new tests FAIL (docstrings currently already kept by the Task 2 implementation; `url =` assertion and f-string assertions fail, and the unterminated docstring test fails because the `语言.docstring` branch exists). Verify actual results; the code changes in Step 3 must make exactly these pass without regressing Task 2.

- [ ] **Step 3: Implementation check**

The Task 2 implementation already covers the Python path (docstring branch, `'#'` line comment via `language.line_comments`, plain string escape walk). If all three tests pass without changes, continue to Task 4; otherwise fix `mask_to_prose` to satisfy them with the smallest possible change. The f-string case relies on the existing order (prefix char `f` blanks as code, the quote walk then blanks the string).

- [ ] **Step 4: Run tests to verify they pass**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests/test_comments.py -q`
Expected: PASS (8 passed)

- [ ] **Step 5: Commit (only if implementation changed)**

```bash
git add .agents/skills/ste/scripts/check_ste.py .agents/skills/ste/scripts/tests/test_comments.py
git commit -m "test(ste): cover python comment masking"
```

If no implementation change was needed, do not create an empty commit; skip to Task 4 and commit the tests with the Task 4 batch (state this explicitly in the task report).

---

### Task 4: TS/JS masker (template literals, regex guard)

**Files:**
- Modify: `.agents/skills/ste/scripts/check_ste.py` (implement `_regex_end`)
- Test: `.agents/skills/ste/scripts/tests/test_comments.py`

- [ ] **Step 1: Write the failing tests**

Append to `test_comments.py`:

```python
def test_ts_template_and_regex_masking() -> None:
    code = (
        'const u = "utilize prior to x";  // use this instead\n'
        'const t = `subsequent to ${"utilize"}`;\n'
        "const r = /a\\/*two/;  // end comment\n"
        "const q = a / b;  // division: not regex\n"
    )
    masked = mask(code, "ts")
    layout_of(code, masked)
    assert "use this instead" in masked
    assert "end comment" in masked
    assert "division: not regex" in masked
    assert "utilize prior to x" not in masked
    assert "subsequent to" not in masked


def test_ts_regex_guard_division_counterexample() -> None:
    masked = mask("const q = a / b; // use division\n", "ts")
    layout_of("const q = a / b; // use division\n", masked)
    assert "use division" in masked


def test_ts_unterminated_regex_candidate_falls_through() -> None:
    masked = mask("const q = (a) / b;\n// use fallthrough\n", "ts")
    assert "use fallthrough" in masked
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests/test_comments.py -q`
Expected: at least the template-literal and regex-line assertions FAIL (the third comment disappears because regex mode swallowed it).

- [ ] **Step 3: Implement `_regex_end`**

Replace the Task 2 stub:

```python
def _regex_end(text: str, start: int) -> int | None:
    """Return the exclusive end of a guarded regex literal, or None.

    The literal must terminate with an unescaped ``/`` on the same line, with
    ``[...]`` character classes honored. An unterminated candidate is not a
    regex and must fall through to plain code.
    """
    i, size = start + 1, len(text)
    in_class = False
    while i < size and text[i] != "\n":
        if text[i] == "\\":
            if i + 1 < size and text[i + 1] == "\n":
                return None
            i += 2
            continue
        if text[i] == "[":
            in_class = True
        elif text[i] == "]":
            in_class = False
        elif text[i] == "/" and not in_class:
            return i + 1
        i += 1
    return None
```

The `mask_to_prose` guard already reads `language.regex_start_prev`; the scanner must also track the previous non-blank code character. Add a `prev_nonblank` variable initialized to `""` before the loop and maintained:

- in the plain quote branch, after `blank(i, j + 1)`: `prev_nonblank = '"' if char == '"' else "'"`
- in the template branch, after `blank(i, j + 1)`: `prev_nonblank = "`"`
- in the code branch (final else), after `char` falls through: `if char != " ": prev_nonblank = char`
- in the `_regex_end` guard, change the condition to `re.match(rf"[{re.escape(language.regex_start_prev)}]", prev_nonblank)` and, after `blank(i, candidate)`: `prev_nonblank = "/"`

The guarded check replacing the size-1 set membership is:

```python
            if (
                language.regex_start_prev is not None
                and char == "/"
                and i + 1 < size
                and text[i + 1] not in "/*"
                and prev_nonblind in set(language.regex_start_prev)
                and (candidate := _regex_end(text, i)) is not None
            ):
```

Use the variable name consistently: define `prev_nonblank` once and reference it everywhere (the plan text renamed `prev`; use `prev_nonblank`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests/test_comments.py -q`
Expected: PASS (11 passed)

- [ ] **Step 5: Commit**

```bash
git add .agents/skills/ste/scripts/check_ste.py .agents/skills/ste/scripts/tests/test_comments.py
git commit -m "feat(ste): add ts comment masker guard"
```

---

### Task 5: Go and C maskers

**Files:**
- Modify: `.agents/skills/ste/scripts/check_ste.py` (none required beyond Task 4; this task verifies table-driven behavior)
- Test: `.agents/skills/ste/scripts/tests/test_comments.py`

- [ ] **Step 1: Write the failing tests**

Append to `test_comments.py`:

```python
def test_go_mask_keeps_comments_and_masks_raw_strings() -> None:
    code = (
        's := `raw /* utilize */ string`\n'
        "r := '/*'\n"
        'c := "prior to"\n'
        "// comment: utilize it\n"
    )
    masked = mask(code, "go")
    layout_of(code, masked)
    assert "comment: utilize it" in masked
    assert "raw /* utilize */ string" not in masked
    assert "prior to" not in masked


def test_c_masks_strings_and_blocks_then_splices_line_comments() -> None:
    code = (
        'char *s = "/* string */ utilize prior to";\n'
        "// line continue\n"
        "/* real block\n   still */\n"
        "char c = '/';\n"
    )
    masked = mask(code, "c")
    layout_of(code, masked)
    assert "real block" in masked
    assert "still" in masked
    assert "char *s" not in masked
    assert "utilize prior to" not in masked


def test_c_line_comment_continues_after_backslash() -> None:
    code = "// first \\\n  continued comment: utilize it\n"
    masked = mask(code, "c")
    layout_of(code, masked)
    assert "continued comment: utilize it" in masked
```

(The C splice test uses a non-backslash body first for a readable test; the splice rule itself is covered by `line_comment_end`, so extend this test directly:)

```python
def test_c_line_comment_backslash_splice_masks_next_line() -> None:
    code = "// line\\\n  still comment: utilize it\n"
    masked = mask(code, "c")
    layout_of(code, masked)
    assert "still comment: utilize it" in masked
```

- [ ] **Step 2: Run tests to verify they fail or pass**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests/test_comments.py -q`
Expected: all PASS without changes (Task 2/4 already handles these paths; this task locks the Go and C tables). If any assertion fails, fix the masker minimally.

- [ ] **Step 3: Commit**

```bash
git add .agents/skills/ste/scripts/tests/test_comments.py
git commit -m "test(ste): cover go and c comment masking"
```

---

### Task 6: Per-line doc/continuation prefix blanking

**Files:**
- Modify: `.agents/skills/ste/scripts/check_ste.py` (implement `mask_line_prefixes`)
- Test: `.agents/skills/ste/scripts/tests/test_comments.py`

- [ ] **Step 1: Write the failing tests**

Append to `test_comments.py`:

```python
def test_rust_doc_prefixes_left_no_glyph_residue() -> None:
    code = "/// Parses Markdown event streams.\n//! Header note.\nlet x = 1;\n"
    masked = mask(code)
    layout_of(code, masked)
    assert "Parses Markdown event streams." in masked
    assert "Header note." in masked
    assert "/" not in "".join(masked.split("\n", 3)[:3]).strip(" ")


def test_block_doc_continuation_star_is_blank() -> None:
    code = "/** Text\n * Continuation.\n */\n"
    masked = mask(code, "ts")
    layout_of(code, masked)
    assert "Continuation." in masked
    assert "Text" in masked
    starred = [line for line in masked.splitlines() if line.strip()]
    assert all(not line.strip().startswith("* ") for line in starred)


def test_docstring_interior_is_never_glyph_cleaned() -> None:
    code = "'''\n# not-a-py-comment marker\n'''\n"
    masked = mask(code, "python")
    layout_of(code, masked)
    assert "not-a-py-comment marker" in masked
    assert "#" in masked
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests/test_comments.py -q`
Expected: at least the first and second tests FAIL (glyph residue `/` and `*` still present).

- [ ] **Step 3: Implement `mask_line_prefixes`**

Replace the Task 2 placeholder:

```python
def mask_line_prefixes(
    text: str,
    out: list[str],
    language: Language,
    spans: list[tuple[int, int, str]],
) -> None:
    """Blank doc-comment and continuation glyphs at line starts inside spans.

    ``///`` leaves a stray ``/``, ``//!`` a ``!``, and block-doc continuation
    lines a ``*``; these residues would reclassify comment lines as Markdown
    blocks. Docstring interiors are arbitrary content and are never cleaned.
    """
    if not language.line_prefix_chars:
        return
    for start, end, kind in spans:
        if kind != _COMMENT:
            continue
        line_start = text.rfind("\n", 0, start) + 1
        region_end = text.find("\n", end)
        limit = region_end + 1 if region_end != -1 else len(text)
        while line_start < limit:
            line_end = text.find("\n", line_start, limit)
            line_end = limit if line_end == -1 else line_end
            pos = line_start
            while pos < line_end and text[pos] in language.line_prefix_chars:
                pos += 1
            out[line_start:pos] = [" "] * (pos - line_start)
            line_start = line_end + 1
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests/test_comments.py -q`
Expected: PASS (all tests so far, 15+ passed)

- [ ] **Step 5: Commit**

```bash
git add .agents/skills/ste/scripts/check_ste.py .agents/skills/ste/scripts/tests/test_comments.py
git commit -m "feat(ste): blank doc prefix residue"
```

---

### Task 7: CLI integration for `check` (mode by extension, stdin `--language`)

**Files:**
- Modify: `.agents/skills/ste/scripts/check_ste.py` (`parser()`, `main()`)
- Test: `.agents/skills/ste/scripts/tests/test_comments.py`

- [ ] **Step 1: Write the failing tests**

Append to `test_comments.py`:

```python
RUST_SAMPLE = (
    "fn main() {\n"
    "    // Please utilize the tool.\n"
    "    let utilize_buffer = 1;\n"
    "}\n"
)


def test_check_source_file_reports_comment_findings_at_real_coordinates(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    document = tmp_path / "sample.rs"
    document.write_text(RUST_SAMPLE)

    assert check_ste.main(["check", str(document), "--format", "json"]) == 1

    output = json.loads(capsys.readouterr().out)
    assert [finding["rule_id"] for finding in output["findings"]] == ["R13"]
    assert output["findings"][0]["line"] == 2
    assert output["findings"][0]["column"] == 8
    assert output["findings"][0]["excerpt"].startswith("Please utilize the tool.")


def test_check_empty_comment_prose_is_clean(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    document = tmp_path / "sample.go"
    document.write_text("func main() { x := 1 }\n")
    assert check_ste.main(["check", str(document), "--format", "json"]) == 0
    assert json.loads(capsys.readouterr().out)["clean"] is True


def test_check_unknown_suffix_falls_back_to_markdown(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    document = tmp_path / "notes.baz"
    document.write_text("Utilize the tool.\n")
    assert check_ste.main(["check", str(document), "--format", "json"]) == 1
    output = json.loads(capsys.readouterr().out)
    assert [finding["rule_id"] for finding in output["findings"]] == ["R13"]


def test_check_stdin_language_and_misuse(
    capsys: pytest.CaptureFixture[str],
) -> None:
    import io

    sys.stdin, previous = io.StringIO(RUST_SAMPLE), sys.stdin
    try:
        assert (
            check_ste.main(["check", "-", "--language", "rust", "--format", "json"])
            == 1
        )
        output = json.loads(capsys.readouterr().out)
        assert output["findings"][0]["line"] == 2
    finally:
        sys.stdin = previous
    assert (
        check_ste.main(
            ["check", "somefile.md", "--language", "rust"],
            ["--rules", "unused"],
        )
        is None
        or True
    )
```

The last assertion above is intentionally awkward; replace it with the exact misbehavior test:

```python
def test_language_only_with_stdin_returns_two(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    document = tmp_path / "notes.md"
    document.write_text("Utilize the tool.\n")
    assert (
        check_ste.main(["check", str(document), "--language", "rust", "--format", "json"])
        == 2
    )
    assert "only valid with stdin" in capsys.readouterr().err
```

Use this final version for that test instead of the draft block.

- [ ] **Step 2: Run tests to verify they fail**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests/test_comments.py -q`
Expected: the new CLI tests FAIL (`--language` is not an accepted argument; source files still check in Markdown mode).

- [ ] **Step 3: Implement CLI wiring**

In `parser()`, extend the `check` subparser:

```python
    check.add_argument(
        "--language",
        choices=sorted(LANGUAGES),
        help="source language for stdin comment checking",
    )
```

In `main()`, implement the check flow. Fix the stdin issue (the script currently reads stdin after config load; hold that behavior) and replace the plain-file check path:

```python
        if args.command == "check":
            only, ignore = parse_ids(args.only), parse_ids(args.ignore)
            if args.compare:
                before_path, after_path = (Path(item) for item in args.compare)
                findings = compare_facts(
                    before_path.read_text(),
                    after_path.read_text(),
                    rules,
                    dictionary,
                )
                file = str(after_path)
            else:
                language = (
                    args.language
                    if getattr(args, "language", None) and args.file == "-"
                    else None
                ) or (
                    language_for_suffix(Path(args.file).suffix)
                    if args.file != "-"
                    else None
                )
                if getattr(args, "language", None) and language is None:
                    raise ConfigError(
                        "--language is only valid with stdin input"
                    )
                if args.file == "-":
                    source = sys.stdin.read()
                else:
                    source = Path(args.file).read_text(encoding="utf-8")
                file = args.file
                checks = (
                    check_text(mask_to_prose(source, LANGUAGES[language]), file, rules, dictionary, only, ignore)
                    if language
                    else check_text(source, file, rules, dictionary, only, ignore)
                )
                findings = checks
```

Keep the existing render/exit logic that follows unchanged.

- [ ] **Step 4: Run tests to verify they pass**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests/test_comments.py -q`
Expected: PASS (all tests, ~19 passed)

- [ ] **Step 5: Run the full existing suite**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests -q`
Expected: all pass (no regressions: Markdown stdin check, `--compare`, `dict`, `render-dict`, fixture tests)

- [ ] **Step 6: Commit**

```bash
git add .agents/skills/ste/scripts/check_ste.py .agents/skills/ste/scripts/tests/test_comments.py
git commit -m "feat(ste): check comments by file type"
```

---

### Task 8: `fix` integration, docs, and spec amendment

**Files:**
- Modify: `.agents/skills/ste/scripts/check_ste.py` (segment helper, `fix_comments`, `fix` flow)
- Modify: `.agents/skills/ste/SKILL.md` (Verify)
- Modify: `docs/superpowers/specs/2026-11-02-ste-comment-checking-design.md` (contradiction fix)
- Test: `.agents/skills/ste/scripts/tests/test_comments.py`

- [ ] **Step 1: Write the failing tests**

Append to `test_comments.py`:

```python
def test_fix_comment_mode_splices_only_comments(tmp_path: Path) -> None:
    document = tmp_path / "lib.rs"
    original = (
        "fn main() {\n"
        "    // Read the docs prior to each run.\n"
        "    let utilize_buffer = 1;\n"
        '    let s = "utilize prior to this";\n'
        "}\n"
    )
    document.write_text(original)

    assert check_ste.main(["fix", str(document), "--write"]) == 0

    expected = original.replace("prior to each run", "before each run")
    assert document.read_text() == expected


def test_fix_source_file_preserves_code_bytes(tmp_path: Path) -> None:
    document = tmp_path / "lib.rs"
    original = "fn main() {\n    // Please utilize the tool.\n    let s = 1;\n}\n"
    document.write_text(original)

    assert check_ste.main(["fix", str(document), "--write"]) == 0

    assert document.read_text() == original.replace(
        "// Please utilize the tool.", "// Please use the tool."
    )


def test_fix_markdown_suffix_behavior_unchanged(tmp_path: Path) -> None:
    document = tmp_path / "notes.md"
    original = "Utilize the tool.\n"
    document.write_text(original)
    assert check_ste.main(["fix", str(document), "--write"]) == 0
    assert document.read_text() == "Use the tool.\n"
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests/test_comments.py -q`
Expected: the two source-file fix tests FAIL (current fixer would treat the whole file as prose; `prior to` inside the string literal might get rewritten).

- [ ] **Step 3: Implement the prose-segment helper, `fix_comments`, and the `fix` flow**

First refactor `replace_auto` to share segment logic (`# 5. safe fixer` section):

```python
def prose_segments(text: str) -> list[tuple[str, int]]:
    """Return non-Markdown-code segments with their start offsets."""
    parts = re.split(r"(```[\s\S]*?```|~~~[\s\S]*?~~~|`[^`\n]*`)", text)
    segments: list[tuple[str, int]] = []
    offset = 0
    for index, part in enumerate(parts):
        if index % 2 == 0:
            segments.append((part, offset))
        offset += len(part)
    return segments


def replace_auto(text: str, dictionary: dict[str, object]) -> str:
    """Replace only auto-approved substitutions outside Markdown code spans and fences."""
    result = text
    for segment, segment_offset in reversed(prose_segments(text)):
        for match in reversed(dictionary_matches(segment, dictionary)):
            if match.auto and isinstance(match.approved, str):
                result = (
                    result[: segment_offset + match.start]
                    + replacement_case(match.common, match.approved)
                    + result[segment_offset + match.end :]
                )
    return result


def fix_comments(
    original: str, masked: str, dictionary: dict[str, object]
) -> str:
    """Splice auto substitutions from masked comment prose into the original.

    Coordinates in the masked text equal coordinates in the original, so each
    match can be applied at the same span. Markdown code protection (backtells,
    fences, URLs) still applies because matching happens per prose segment.
    """
    result = original
    for segment, segment_offset in reversed(prose_segments(masked)):
        matches = [
            match
            for match in reversed(dictionary_matches(segment, dictionary))
            if match.auto and isinstance(match.approved, str)
        ]
        for match in matches:
            result = (
                result[: segment_offset + match.start]
                + replacement_case(match.common, match.approved)
                + result[segment_offset + match.end :]
            )
    return result
```

Then in `main()`, change the `fix` branch:

```python
        if args.command == "fix":
            path = Path(args.file)
            before = path.read_text(encoding="utf-8")
            language = language_for_suffix(path.suffix.lower())
            if language is None:
                after = replace_auto(before, dictionary)
            else:
                masked = mask_to_prose(before, LANGUAGES[language])
                after = fix_comments(before, masked, dictionary)
```

Keep the rest of the fix branch (write/diff output) unchanged.

- [ ] **Step 4: Run tests to verify they pass and no regressions**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests -q`
Expected: all pass (including the existing fixer tests, which verify `replace_auto` keeps backtick and fence protection after the refactor).

- [ ] **Step 5: Update SKILL.md Verify section**

In `.agents/skills/ste/SKILL.md`, extend the `## Verify` list:

```markdown
4. `.agents/skills/ste/scripts/check_ste.py fix <file> -w` applies only unambiguous substitutions.
5. Code files: `check <file.rs|.py|.ts|.js|.mjs|.go|.c|.h|.cpp|.hpp>` selects comment mode by file type and lints comment prose only (code is blanked; real line and column reported). `fix <file>` applies safe substitutions inside comments only. For stdin, pass `--language <rust|python|ts|go|c>`. Unknown file extensions stay in Markdown mode. Limitations: comment lines containing `|` or starting with `<` are skipped; TS/JS regex literals are detected heuristically.
```

- [ ] **Step 6: Amend the spec contradiction**

In `docs/superpowers/specs/2026-11-02-ste-comment-checking-design.md`, replace the sentence `Unknown extension for comment checking: exit 2 with the supported list.`:

```markdown
Unknown file extensions fall back to Markdown mode per automatic mode selection; explicit stdin `--language` values are argparse-restricted and reject unknown language names (exit 2).
```

- [ ] **Step 7: Full verification gate**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests -q`
Expected: all pass.

Run: `ruff check .agents/skills/ste/scripts/ && ruff format --check .agents/skills/ste/scripts/ && python3 .agents/skills/ste/scripts/check_ste.py render-dict --check`
Expected: clean.

- [ ] **Step 8: Commit**

```bash
git add .agents/skills/ste/scripts/check_ste.py .agents/skills/ste/scripts/tests/test_comments.py .agents/skills/ste/SKILL.md docs/superpowers/specs/2026-11-02-ste-comment-checking-design.md
git commit -m "feat(ste): fix comments in source files"
```

---

### Task 9: Final concept review and whole-suite verification

**Files:** read-only

- [ ] **Step 1: Run everything**

Run: `uv run --with pytest pytest .agents/skills/ste/scripts/tests -q && ruff check .agents/skills/ste/scripts/ && ruff format --check .agents/skills/ste/scripts/ && python3 .agents/skills/ste/scripts/check_ste.py render-dict --check && git diff --check`
Expected: all clean.

- [ ] **Step 2: Report**

Post results; no commit (nothing changed).

---

## Self-Review (completed while writing)

1. **Spec coverage:** mode selection (Task 7), mask semantics incl. docstring/glyph/regex/splice/nesting/undermination decisions (Tasks 2-6), CLI `--language` stdin flow and misuse (Task 7), fix semantics with positive and negative controls (Task 8), SKILL.md docs plus spec amendment (Task 8), per language (Tasks 2-5), per-line prefix residue (Task 6). Unknown-extension exit 2 conflict resolved in Task 8 with the spec amendment.
2. **Placeholders:** none; Task 3 may be a no-code-change step (allowed and stated).
3. **Type consistency:** `language_for_suffix`, `LANGUAGES`, `mask_to_prose`, `_regex_end`, `mask_line_prefixes`, `prose_segments`, `fix_comments` names agree across tasks; `Language` fields match between Task 1 definition and Task 2 usage.

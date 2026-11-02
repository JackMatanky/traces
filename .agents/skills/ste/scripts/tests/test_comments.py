"""Tests for comment-mode checking in source files."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest  # noqa: F401  (fixtures arrive with later comment-mode tasks)

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
    assert kept == ["       Please utilize the tool."]


def test_rust_nested_block_comment_is_prose() -> None:
    code = (
        '/* start\n/* nested\n*/ inner\n* STILL? */\nlet s = "str /* not comment */";\n'
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


def test_line_prefix_policies_are_language_specific() -> None:
    assert check_ste.LANGUAGES["rust"].line_prefix_chars == " \t/*!"
    assert check_ste.LANGUAGES["python"].line_prefix_chars == ""
    assert check_ste.LANGUAGES["ts"].line_prefix_chars == " \t/*"

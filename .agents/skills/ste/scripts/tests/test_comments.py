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
    assert rust.raw_strings is not None and rust.char_literals is True
    py = check_ste.LANGUAGES["python"]
    assert py.docstring is True and py.block is None
    assert py.backtick_delim is False and py.raw_strings is None
    ts = check_ste.LANGUAGES["ts"]
    assert ts.regex_start_prev is not None
    assert ts.backtick_delim is True
    go = check_ste.LANGUAGES["go"]
    assert go.line_splice is False and go.block_nests is False
    assert go.backtick_delim is True
    c = check_ste.LANGUAGES["c"]
    assert c.line_splice is True and c.block_nests is False
    assert c.backtick_delim is False and c.raw_strings is None
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


def test_block_comment_terminating_at_eof_blanks_closer() -> None:
    code = "/* x */"
    masked = mask(code)
    layout_of(code, masked)
    assert "x" in masked
    assert "/" not in masked


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
    assert "Docstring prose. Utilize it." in masked.replace("'", "")
    assert "comment with apostrophe and /* block-ish */" in masked
    assert "url =" not in masked
    assert "https://example.test" not in masked


def test_python_unterminated_docstring_blanks_remainder() -> None:
    code = '"""unterminated\nstill looks like code\nlet prior_to_var = 1\n'
    masked = mask(code, "python")
    layout_of(code, masked)
    assert "still looks like code" not in masked


def test_python_hash_inside_string_is_not_comment() -> None:
    code = 'x = f"color {y:#x}"  # comment: utilize it\n'
    masked = mask(code, "python")
    layout_of(code, masked)
    assert "comment: utilize it" in masked
    assert "color" not in masked


def test_python_docstring_escape_sequence_not_false_terminator() -> None:
    code = '""" she said \\""" then # note\n"""\n'
    masked = mask(code, "python")
    layout_of(code, masked)
    assert " she said " in masked
    assert "then # note" in masked
    assert "note" in masked


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


# Even inner-quote count made the old 6-char-window implementation coincide by
# quote parity, so this pins one direction only: a mis-parsed >=5-hash raw
# string must not hide the trailing comment line.
def test_rust_raw_string_with_five_hashes_is_delimiter_extensible() -> None:
    code = 'let r = r#####"a"b"c"#####;\n// comment: utilize it\n'
    masked = mask(code)
    layout_of(code, masked)
    assert "comment: utilize it" in masked
    assert 'b"c' not in masked


def test_rust_raw_string_odd_quotes_blanks_only_raw_string() -> None:
    code = 'let s = r#####"a"b"#####;\n// comment: utilize it\n'
    masked = mask(code)
    layout_of(code, masked)
    assert "comment: utilize it" in masked
    assert '"a"b"' not in masked


def test_go_mask_keeps_comments_and_masks_backtick_raw_string() -> None:
    code = (
        "s := `raw /* gone */ string`\n"
        "r := '/*'\n"
        'c := "prior to"\n'
        "// comment: utilize it\n"
    )
    masked = mask(code, "go")
    layout_of(code, masked)
    assert "comment: utilize it" in masked
    assert "gone" not in masked
    assert "prior to" not in masked


def test_c_masks_strings_and_blocks_then_keeps_line_comment() -> None:
    code = (
        'char *s = "/* bogus */ utilize prior to";\n'
        "// line continue\n"
        "/* outer /* inner */ still code */\n"
        "char c = '/';\n"
    )
    masked = mask(code, "c")
    layout_of(code, masked)
    assert "outer" in masked
    assert "inner" in masked
    assert "still code" not in masked
    assert "char *s" not in masked
    assert "bogus" not in masked
    assert "utilize prior to" not in masked


def test_c_line_comment_frontslash_splice_marks_next_line() -> None:
    code = "// first \\\n  continued comment: utilize it\n"
    masked = mask(code, "c")
    layout_of(code, masked)
    assert "continued comment: utilize it" in masked


def test_c_line_comment_backslash_splice_masks_next_line() -> None:
    code = "// line\\\n  still comment: utilize it\n"
    masked = mask(code, "c")
    layout_of(code, masked)
    assert "still comment: utilize it" in masked


def test_c_unterminated_block_comment_keeps_to_eof() -> None:
    code = "/* unterminated\nstill prose.\n"
    masked = mask(code, "c")
    layout_of(code, masked)
    assert "still prose." in masked


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
    assert "'''" not in masked

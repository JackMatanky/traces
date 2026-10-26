"""Parity and golden tests for the STE skill's checked artifacts."""

from __future__ import annotations

import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

TEST_ROOT = Path(__file__).parent
STE_ROOT = TEST_ROOT.parents[1]
SCRIPTS = STE_ROOT / "scripts"
CHECKER = SCRIPTS / "check_ste.py"
RULES = STE_ROOT / "RULES.md"
DICTIONARY = STE_ROOT / "DICTIONARY.md"
SKILL = STE_ROOT / "SKILL.md"
EXAMPLES = STE_ROOT / "EXAMPLES.md"

SPEC = importlib.util.spec_from_file_location("check_ste_parity", CHECKER)
assert SPEC and SPEC.loader
check_ste = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = check_ste
SPEC.loader.exec_module(check_ste)


def load_dictionary() -> dict[str, object]:
    """Load the checker dictionary from its real configuration source."""
    return check_ste.load_config(None, None)[1]


def generated_dictionary_block() -> str:
    """Return the generated dictionary table, failing if its markers drift."""
    document = DICTIONARY.read_text(encoding="utf-8")
    start = "<!-- BEGIN GENERATED: substitutions (from scripts/dictionary.json) -->"
    end = "<!-- END GENERATED -->"
    assert document.count(start) == 1, (
        "DICTIONARY.md must contain one generated-block start marker"
    )
    assert document.count(end) == 1, (
        "DICTIONARY.md must contain one generated-block end marker"
    )
    before, block_and_after = document.split(start, maxsplit=1)
    assert before, "generated dictionary marker must not be the first content"
    block, after = block_and_after.split(end, maxsplit=1)
    assert after, "generated dictionary marker must not be the last content"
    assert block.startswith("\n") and block.endswith("\n"), (
        "generated table must be marker-bounded"
    )
    return block[1:-1]


def ste_examples() -> list[tuple[str, str]]:
    """Return labeled After (STE) quote blocks, failing on malformed examples."""
    lines = EXAMPLES.read_text(encoding="utf-8").splitlines()
    examples: list[tuple[str, str]] = []
    for index, line in enumerate(lines):
        if line != "**After (STE)**":
            continue
        label = next(
            (
                candidate[3:]
                for candidate in reversed(lines[:index])
                if candidate.startswith("## ")
            ),
            None,
        )
        assert label, f"After (STE) section at line {index + 1} has no example heading"
        quote_lines: list[str] = []
        for quote_line in lines[index + 1 :]:
            if not quote_line.startswith(">"):
                break
            content = quote_line[1:]
            quote_lines.append(content.removeprefix(" "))
        assert quote_lines, f"{label}: After (STE) must contain a quoted block"
        examples.append((label, "\n".join(quote_lines)))
    assert examples, "EXAMPLES.md must contain at least one After (STE) quoted block"
    return examples


def test_rules_config_matches_documented_rule_ids() -> None:
    documented = re.findall(
        r"^- \*\*(R\d+)\.\*\*", RULES.read_text(encoding="utf-8"), re.MULTILINE
    )
    assert documented, "RULES.md must document R IDs"
    assert documented == [f"R{number}" for number in range(1, 21)]

    configured = json.loads((SCRIPTS / "rules.json").read_text(encoding="utf-8"))[
        "rules"
    ]
    configured_ids = [rule["id"] for rule in configured]
    assert configured_ids == documented + ["F1"]
    assert len(configured_ids) == len(set(configured_ids))

    rules_by_id = {rule["id"]: rule for rule in configured}
    for rule_id in ("R4", "R9", "R14", "R15"):
        assert rules_by_id[rule_id]["check"] == "none"
        assert rules_by_id[rule_id]["severity"] == "none"


def test_generated_dictionary_matches_checker_and_cli() -> None:
    assert generated_dictionary_block() == check_ste.dictionary_table(load_dictionary())

    result = subprocess.run(
        [sys.executable, str(CHECKER), "render-dict", "--check"],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr


def test_skill_short_dictionary_terms_are_configured() -> None:
    forbidden: list[str] = []
    for line in SKILL.read_text(encoding="utf-8").splitlines():
        match = re.fullmatch(r"- .+ \(not ([^)]+)\)", line)
        if not match:
            continue
        forbidden.extend(
            alternative.split(",", maxsplit=1)[0].strip()
            for alternative in match.group(1).split("/")
        )
    assert forbidden, "SKILL.md must contain short-dictionary '(not X)' bullets"

    common_terms = {
        common
        for substitution in check_ste.object_list(
            load_dictionary()["substitutions"], "substitutions"
        )
        for common in check_ste.string_list(
            check_ste.object_dict(substitution, "substitution")["common"], "common"
        )
    }
    for term in forbidden:
        assert term in common_terms, (
            f"SKILL.md forbidden term is not configured: {term!r}"
        )


def test_after_ste_examples_have_no_error_findings() -> None:
    rules, dictionary = check_ste.load_config(None, None)
    failures: list[str] = []
    for label, text in ste_examples():
        errors = [
            finding
            for finding in check_ste.check_text(
                text, f"EXAMPLES.md: {label}", rules, dictionary
            )
            if finding.severity == "error"
        ]
        if errors:
            details = ", ".join(
                f"{finding.rule_id} at {finding.line}:{finding.column}"
                for finding in errors
            )
            failures.append(f"{label}: {details}")
    assert not failures, "After (STE) examples contain errors:\n" + "\n".join(failures)

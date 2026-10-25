"""Tests for the standalone STE checker."""

from __future__ import annotations

import importlib.util
import json
import sys
from copy import deepcopy
from pathlib import Path

import pytest

SCRIPT = Path(__file__).parents[1] / "check_ste.py"
SPEC = importlib.util.spec_from_file_location("check_ste", SCRIPT)
assert SPEC and SPEC.loader
check_ste = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = check_ste
SPEC.loader.exec_module(check_ste)


def config() -> tuple[dict[str, object], dict[str, object]]:
    return check_ste.load_config(None, None)


def ids(text: str) -> set[str]:
    rules, dictionary = config()
    return {
        finding.rule_id
        for finding in check_ste.check_text(text, "test.md", rules, dictionary)
    }


def findings_for(text: str, rule_id: str) -> set[str]:
    rules, dictionary = config()
    return {
        finding.rule_id
        for finding in check_ste.check_text(
            text, "test.md", rules, dictionary, only={rule_id}
        )
    }


@pytest.mark.parametrize(
    ("rule_id", "violating", "compliant"),
    [
        ("R1", "- " + "word " * 21 + ".", "- " + "word " * 20 + "."),
        ("R2", " ".join(["word"] * 26) + ".", " ".join(["word"] * 25) + "."),
        ("R13", "Utilize the tool.", "Use the tool."),
        ("R16", "Use the tool. " * 7, "Use the tool. " * 6),
        (
            "R17",
            "First do this, then do that, next do this, after do that.",
            "First do this, then do that.",
        ),
        ("R18", "This WARNING is inline.", "WARNING\n\nUse the tool."),
    ],
)
def test_error_rules_detect_and_allow_minimal_cases(
    rule_id: str, violating: str, compliant: str
) -> None:
    assert rule_id in findings_for(violating, rule_id)
    assert rule_id not in findings_for(compliant, rule_id)


def test_r1_applies_to_command_sentences_outside_lists() -> None:
    violating = "Use " + "word " * 20 + "."
    compliant = "Use " + "word " * 19 + "."
    assert "R1" in findings_for(violating, "R1")
    assert "R1" not in findings_for(compliant, "R1")


def test_r16_applies_only_to_procedural_paragraphs() -> None:
    assert "R16" not in findings_for("This is descriptive. " * 7, "R16")
    assert "R16" in findings_for("Use the tool. " * 7, "R16")


def test_findings_preserve_multiline_and_repeated_sentence_locations() -> None:
    rules, dictionary = config()
    text = "First sentence.\nUtilize the tool.\nUtilize the tool."
    findings = [
        finding
        for finding in check_ste.check_text(
            text, "test.md", rules, dictionary, only={"R13"}
        )
    ]
    assert [(finding.line, finding.column) for finding in findings] == [(2, 1), (3, 1)]


@pytest.mark.parametrize(
    ("rule_id", "text"),
    [
        ("R3", "- Use the tool, and then close the file."),
        ("R5", "The file must be removed."),
        ("R6", "- The user was ready."),
        ("R8", "- Use tool."),
        ("R10", "- Running the tool updates the file."),
        ("R11", "Begin the task. Start the task."),
        ("R12", "Work in the fall."),
        ("R19", "This might fail."),
        ("R20", "- The user must open the file."),
    ],
)
def test_warning_rules_detect_configured_patterns(rule_id: str, text: str) -> None:
    assert rule_id in findings_for(text, rule_id)


def test_r7_detects_only_when_a_stricter_config_makes_it_defensible() -> None:
    _, dictionary = config()
    block = check_ste.markdown_blocks("- The user opens the file.")[0]
    sentence = check_ste.split_sentences(block.text, block.line, dictionary, block)[0]
    rule = {
        "id": "R7",
        "severity": "warn",
        "message": "Start with a command verb.",
        "params": {
            "verbs": ["open"],
            "allow_caller_domain_verbs": False,
            "flag_unlisted_leading_verb": True,
        },
    }
    assert check_ste.check_command(rule, sentence, "test.md", dictionary)
    assert "R7" not in findings_for("- Deploy the package.", "R7")


def test_sentence_protection_and_word_count() -> None:
    sentences = check_ste.split_sentences(
        "Use `v1.2.3` at https://example.test/1.2. It is e.g. 1.5 seconds.", 1
    )
    assert len(sentences) == 2
    assert check_ste.word_count("Use `v1.2.3` and [a link](https://example.test).") == 5
    assert check_ste.word_count("It's a well-known tool.") == 4
    blocks = check_ste.markdown_blocks(
        "[link text](https://example.test)\n\n```text\nutilize\n```"
    )
    assert check_ste.word_count(blocks[0].text) == 2
    assert blocks[1].kind == "fence" and not blocks[1].prose


def test_dictionary_matching_boundaries_conditions_and_case() -> None:
    _, dictionary = config()
    matches = check_ste.dictionary_matches("UTILIZE prior to utilization.", dictionary)
    assert [match.common for match in matches] == ["UTILIZE", "prior to"]
    assert (
        check_ste.replace_auto("UTILIZE Prior to `utilize`.", dictionary)
        == "USE Before `utilize`."
    )
    assert (
        check_ste.dictionary_matches("Follow rules.", dictionary)[0].approved == "obey"
    )
    assert not check_ste.dictionary_matches("Follow trees.", dictionary)
    assert not check_ste.dictionary_matches("utilize-utilize's", dictionary)
    precedence_dictionary = {
        "matching": dictionary["matching"],
        "substitutions": [
            {
                "common": ["prior"],
                "approved": "earlier",
                "auto": False,
                "condition": None,
            },
            {
                "common": ["prior to"],
                "approved": "before",
                "auto": True,
                "condition": None,
            },
        ],
    }
    assert [
        match.common
        for match in check_ste.dictionary_matches(
            "Prior to work.", precedence_dictionary
        )
    ] == ["Prior to"]


def test_fixer_changes_only_auto_entries_and_preserves_code(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    document = tmp_path / "input.md"
    original = "Utilize prior to subsequent to ensure. `utilize prior to`\n\n```text\nutilize prior to\n```\n"
    document.write_text(original)
    assert check_ste.main(["fix", str(document)]) == 0
    assert document.read_text() == original
    assert "-Utilize" in capsys.readouterr().out
    assert check_ste.main(["fix", str(document), "--write"]) == 0
    updated = document.read_text()
    assert "Use before after ensure." in updated
    assert "`utilize prior to`" in updated
    assert "```text\nutilize prior to\n```" in updated


def test_core_rules_and_unknown_domain_verb() -> None:
    assert {"R13", "R19", "R12"} <= ids(
        "Utilize this. Basically test it. Work in the fall."
    )
    assert "R7" not in ids("- Deploy the package.")
    assert "R10" in ids("- Running the tool updates the file.")
    assert "R20" in ids("- The user must open the file.")


def test_compare_is_order_independent_and_detects_number_change() -> None:
    rules, _dictionary = config()
    assert not check_ste.compare_facts(
        "Use 2 flags: --fast and --safe.", "Use --safe and --fast: 2 flags.", rules
    )
    assert check_ste.compare_facts("Use 2 flags.", "Use 3 flags.", rules)


@pytest.mark.parametrize(
    "before,after",
    [
        ("Use `one`.", "Use `two`."),
        ("```text\nONE\n```", "```text\nTWO\n```"),
        ("Use --fast.", "Use --safe."),
        ("Use config.toml and src/main.py.", "Use config.json and src/lib.py."),
        ("Set TOKEN.", "Set SECRET."),
        ("Use --fast and --fast.", "Use --fast once."),
    ],
)
def test_f1_detects_every_configured_fact_class(before: str, after: str) -> None:
    rules, dictionary = config()
    assert check_ste.compare_facts(before, after, rules, dictionary)


def test_f1_obeys_protected_categories_in_temporary_config() -> None:
    rules, dictionary = config()
    configured = deepcopy(rules)
    for rule in configured["rules"]:
        if rule["id"] == "F1":
            rule["params"]["protected"] = ["numbers"]
    assert not check_ste.compare_facts(
        "Use --fast.", "Use --safe.", configured, dictionary
    )
    assert check_ste.compare_facts("Use 2.", "Use 3.", configured, dictionary)


def test_render_dictionary(tmp_path: Path) -> None:
    document = tmp_path / "DICTIONARY.md"
    document.write_text(
        "before\n<!-- BEGIN GENERATED: substitutions (from scripts/dictionary.json) -->\nold\n<!-- END GENERATED -->\nafter\n"
    )
    _, dictionary = config()
    assert check_ste.render_dictionary(document, dictionary, check=True) == 1
    assert check_ste.render_dictionary(document, dictionary, check=False) == 0
    assert "utilize" in document.read_text()
    assert check_ste.render_dictionary(document, dictionary, check=True) == 0


@pytest.mark.parametrize("failure", ["severity", "duplicate", "regex"])
def test_config_failures_return_exit_two(tmp_path: Path, failure: str) -> None:
    base_rules = json.loads((Path(__file__).parents[1] / "rules.json").read_text())
    base_dictionary = json.loads(
        (Path(__file__).parents[1] / "dictionary.json").read_text()
    )
    if failure == "severity":
        base_rules["rules"][0]["severity"] = "invalid"
    elif failure == "duplicate":
        base_rules["rules"].append(deepcopy(base_rules["rules"][0]))
    else:
        base_dictionary["substitutions"][0]["condition"] = "["
    rules = tmp_path / "rules.json"
    dictionary = tmp_path / "dictionary.json"
    rules.write_text(json.dumps(base_rules))
    dictionary.write_text(json.dumps(base_dictionary))
    assert (
        check_ste.main(
            ["check", "-", "--rules", str(rules), "--dictionary", str(dictionary)]
        )
        == 2
    )


@pytest.mark.parametrize(
    "failure", ["required", "check", "boundary_regex", "protected_regex"]
)
def test_additional_config_failures_return_exit_two(
    tmp_path: Path, failure: str
) -> None:
    base_rules = json.loads((Path(__file__).parents[1] / "rules.json").read_text())
    base_dictionary = json.loads(
        (Path(__file__).parents[1] / "dictionary.json").read_text()
    )
    if failure == "required":
        del base_rules["rules"][0]["message"]
    elif failure == "check":
        base_rules["rules"][0]["check"] = "unknown"
    elif failure == "boundary_regex":
        base_dictionary["matching"]["phrase_boundary_regex_template"] = "["
    else:
        base_dictionary["matching"]["sentence_segmentation"]["protected_token_regexes"][
            "url"
        ] = "["
    rules = tmp_path / "rules.json"
    dictionary = tmp_path / "dictionary.json"
    rules.write_text(json.dumps(base_rules))
    dictionary.write_text(json.dumps(base_dictionary))
    assert (
        check_ste.main(
            ["check", "-", "--rules", str(rules), "--dictionary", str(dictionary)]
        )
        == 2
    )


def test_cli_json_text_strict_only_and_ignore(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    document = tmp_path / "input.md"
    document.write_text("This might fail. Utilize the tool.")
    assert check_ste.main(["check", str(document), "--format", "json"]) == 1
    output = json.loads(capsys.readouterr().out)
    assert output["summary"] == {"errors": 1, "warnings": 1}
    assert check_ste.main(["check", str(document), "--only", "R19", "--strict"]) == 1
    assert "[R19 warn]" in capsys.readouterr().out
    assert check_ste.main(["check", str(document), "--only", "R19"]) == 0
    capsys.readouterr()
    assert check_ste.main(["check", str(document), "--ignore", "R13"]) == 0
    assert "[R19 warn]" in capsys.readouterr().out

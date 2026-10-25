"""Tests for the standalone STE checker."""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

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


def test_sentence_protection_and_word_count() -> None:
    sentences = check_ste.split_sentences(
        "Use `v1.2.3` at https://example.test/1.2. It is e.g. 1.5 seconds.", 1
    )
    assert len(sentences) == 2
    assert check_ste.word_count("Use `v1.2.3` and [a link](https://example.test).") == 5
    assert check_ste.word_count("It's a well-known tool.") == 4


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


def test_render_dictionary(tmp_path: Path) -> None:
    document = tmp_path / "DICTIONARY.md"
    document.write_text(
        "before\n<!-- BEGIN GENERATED: substitutions (from scripts/dictionary.json) -->\nold\n<!-- END GENERATED -->\nafter\n"
    )
    _, dictionary = config()
    assert check_ste.render_dictionary(document, dictionary, check=False) == 0
    assert "utilize" in document.read_text()
    assert check_ste.render_dictionary(document, dictionary, check=True) == 0


def test_config_errors(tmp_path: Path) -> None:
    rules = tmp_path / "rules.json"
    dictionary = Path(__file__).parents[1] / "dictionary.json"
    rules.write_text(
        json.dumps({"rules": [{"id": "R1", "severity": "bad", "check": "none"}]})
    )
    try:
        check_ste.load_config(rules, dictionary)
    except check_ste.ConfigError:
        pass
    else:
        raise AssertionError("invalid severity must fail")

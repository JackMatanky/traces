#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Check Markdown prose against the starter ASD-STE100 rules."""

from __future__ import annotations

import argparse
import difflib
import json
import re
import sys
from collections import Counter
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).parent
VALID_SEVERITIES = {"error", "warn", "none"}
CHECK_NAMES = {
    "sentence_words",
    "compound_instruction",
    "passive_voice",
    "tense",
    "command_verb_start",
    "missing_article",
    "gerund_start",
    "synonym_pair",
    "one_meaning",
    "banned_word",
    "paragraph_length",
    "list_required",
    "warning_block",
    "hedge",
    "second_person",
    "facts_preserved",
    "none",
}


class ConfigError(Exception):
    """Raised when the JSON configuration is absent or malformed."""


@dataclass(frozen=True)
class Block:
    """A Markdown block retaining its source line and prose eligibility."""

    text: str
    line: int
    kind: str
    prose: bool = True
    column: int = 1


@dataclass(frozen=True)
class Sentence:
    """A sentence and its one-based source location."""

    text: str
    line: int
    column: int
    block: Block


@dataclass(frozen=True)
class Finding:
    """A checker result suitable for text and JSON rendering."""

    rule_id: str
    severity: str
    file: str
    line: int
    column: int
    message: str
    excerpt: str
    suggestion: str | None = None


@dataclass(frozen=True)
class Match:
    """A dictionary substitution occurrence."""

    common: str
    approved: str | list[str]
    auto: bool
    note: str
    start: int
    end: int


# 1. configuration loading/validation
def object_dict(value: object, label: str) -> dict[str, object]:
    """Return a JSON object or raise a useful configuration error."""
    if not isinstance(value, dict):
        raise ConfigError(f"{label} must be a JSON object")
    return {str(key): item for key, item in value.items()}


def object_list(value: object, label: str) -> list[object]:
    """Return a JSON array or raise a useful configuration error."""
    if not isinstance(value, list):
        raise ConfigError(f"{label} must be a JSON array")
    return value


def read_json(path: Path) -> dict[str, object]:
    """Read one JSON object from ``path``."""
    try:
        return object_dict(json.loads(path.read_text(encoding="utf-8")), str(path))
    except OSError as error:
        raise ConfigError(f"cannot read {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise ConfigError(f"invalid JSON in {path}: {error}") from error


def validate_config(rules: dict[str, object], dictionary: dict[str, object]) -> None:
    """Validate the deliberately small, versioned configuration contract."""
    entries = object_list(rules.get("rules"), "rules.rules")
    seen: set[str] = set()
    for raw in entries:
        rule = object_dict(raw, "rule")
        for key in ("id", "severity", "check", "params", "message"):
            if key not in rule:
                raise ConfigError(f"rule missing {key}")
        identifier = rule["id"]
        severity = rule["severity"]
        checker = rule["check"]
        if not isinstance(identifier, str) or identifier in seen:
            raise ConfigError(f"duplicate or invalid rule id: {identifier!r}")
        if not isinstance(severity, str):
            raise ConfigError(f"invalid severity for {identifier}: {severity!r}")
        if not isinstance(checker, str):
            raise ConfigError(f"invalid checker for {identifier}: {checker!r}")
        if severity not in VALID_SEVERITIES:
            raise ConfigError(f"invalid severity for {identifier}: {severity!r}")
        if checker not in CHECK_NAMES:
            raise ConfigError(f"unknown checker for {identifier}: {checker!r}")
        seen.add(identifier)
    matching = object_dict(dictionary.get("matching"), "dictionary.matching")
    template = matching.get("phrase_boundary_regex_template")
    if not isinstance(template, str) or "{phrase}" not in template:
        raise ConfigError("dictionary matching boundary template is invalid")
    substitutions = object_list(
        dictionary.get("substitutions"), "dictionary.substitutions"
    )
    for raw in substitutions:
        item = object_dict(raw, "substitution")
        if not isinstance(item.get("common"), list) or "approved" not in item:
            raise ConfigError("substitution requires common and approved")
        condition = item.get("condition")
        if condition is not None:
            if not isinstance(condition, str):
                raise ConfigError("substitution condition must be a string or null")
            try:
                re.compile(condition, re.IGNORECASE)
            except re.error as error:
                raise ConfigError(f"invalid condition regex: {error}") from error
    try:
        re.compile(template.format(phrase="example"), re.IGNORECASE)
        protected = object_dict(
            matching.get("sentence_segmentation"), "sentence segmentation"
        )
        for pattern in object_dict(
            protected.get("protected_token_regexes"), "protected regexes"
        ).values():
            if not isinstance(pattern, str):
                raise ConfigError("protected regex must be a string")
            re.compile(pattern)
    except re.error as error:
        raise ConfigError(f"invalid configured regex: {error}") from error


def load_config(
    rules_path: Path | None, dictionary_path: Path | None
) -> tuple[dict[str, object], dict[str, object]]:
    """Load and validate rules and dictionary relative to this script by default."""
    rules = read_json(rules_path or ROOT / "rules.json")
    dictionary = read_json(dictionary_path or ROOT / "dictionary.json")
    validate_config(rules, dictionary)
    return rules, dictionary


# 2. markdown blocks with line spans
def markdown_blocks(text: str) -> list[Block]:
    """Parse the Markdown subset used by this checker without changing line spans."""
    blocks: list[Block] = []
    lines = text.splitlines()
    index = 0
    paragraph: list[str] = []
    paragraph_line = 1

    def flush() -> None:
        nonlocal paragraph
        if paragraph:
            joined = "\n".join(paragraph)
            blocks.append(Block(joined, paragraph_line, "paragraph"))
            paragraph = []

    while index < len(lines):
        line = lines[index]
        line_number = index + 1
        if re.match(r"^\s*(```|~~~)", line):
            flush()
            fence = line.lstrip()[:3]
            body = [line]
            index += 1
            while index < len(lines):
                body.append(lines[index])
                if lines[index].lstrip().startswith(fence):
                    index += 1
                    break
                index += 1
            blocks.append(Block("\n".join(body), line_number, "fence", False))
            continue
        if not line.strip():
            flush()
        elif line.lstrip().startswith("<") or "|" in line:
            flush()
            blocks.append(Block(line, line_number, "raw", False))
        elif re.match(r"^\s{0,3}#{1,6}\s+", line):
            flush()
            blocks.append(
                Block(
                    re.sub(r"^\s{0,3}#{1,6}\s+", "", line),
                    line_number,
                    "heading",
                )
            )
        elif match := re.match(r"^\s*(?:[-*+] |\d+[.)] )(.+)$", line):
            flush()
            blocks.append(
                Block(
                    match.group(1),
                    line_number,
                    "list",
                    column=match.start(1) + 1,
                )
            )
        else:
            if not paragraph:
                paragraph_line = line_number
            paragraph.append(line)
        index += 1
    flush()
    return blocks


# 3. sentence and word utilities
def protected_pattern(dictionary: dict[str, object]) -> re.Pattern[str]:
    matching = object_dict(dictionary["matching"], "matching")
    segmentation = object_dict(matching["sentence_segmentation"], "segmentation")
    patterns = object_dict(segmentation["protected_token_regexes"], "protected regexes")
    return re.compile(
        "|".join(f"({value})" for value in patterns.values() if isinstance(value, str))
    )


def split_sentences(
    text: str,
    line: int,
    dictionary: dict[str, object] | None = None,
    block: Block | None = None,
) -> list[Sentence]:
    """Split terminal-punctuation sentences while preserving configured protected tokens."""
    dictionary = dictionary or load_config(None, None)[1]
    token = protected_pattern(dictionary)

    def hide(match: re.Match[str]) -> str:
        value = match.group(0)
        trailing = ""
        # URL regexes intentionally include punctuation; prose punctuation still ends a sentence.
        if value.startswith(("http://", "https://")) and value[-1:] in ".!?":
            value, trailing = value[:-1], value[-1]
        marker = "\x00\x00"
        return marker + "\x00" * (len(value) - len(marker)) + trailing

    hidden = token.sub(hide, text)
    result: list[Sentence] = []
    start = 0
    for match in re.finditer(r"[.!?]+(?=\s|$)", hidden):
        raw = text[start : match.end()]
        leading = len(raw) - len(raw.lstrip())
        value = raw.strip()
        if value:
            offset = start + leading
            preceding = text[:offset]
            line_delta = preceding.count("\n")
            result.append(
                Sentence(
                    value,
                    line + line_delta,
                    offset
                    - preceding.rfind("\n")
                    + (block.column - 1 if block and line_delta == 0 else 0),
                    block or Block(text, line, "paragraph"),
                )
            )
        start = match.end()
    raw = text[start:]
    leading = len(raw) - len(raw.lstrip())
    trailing = raw.strip()
    if trailing:
        offset = start + leading
        preceding = text[:offset]
        line_delta = preceding.count("\n")
        result.append(
            Sentence(
                trailing,
                line + line_delta,
                offset
                - preceding.rfind("\n")
                + (block.column - 1 if block and line_delta == 0 else 0),
                block or Block(text, line, "paragraph"),
            )
        )
    return result


def prose_words(text: str) -> list[str]:
    text = re.sub(
        r"!?\[([^]]*)\]\([^)]*\)",
        lambda match: match.group(1),
        text,
    )
    text = re.sub(r"`[^`\n]*`", " CODETOKEN ", text)
    text = re.sub(r"https?://[^\s`<>]+", " URLTOKEN ", text)
    return re.findall(r"[A-Za-z0-9]+(?:[.'’-][A-Za-z0-9]+)*", text)


def word_count(text: str) -> int:
    """Count prose words under the checker’s Markdown policy."""
    return len(prose_words(text))


def phrase_regex(phrase: str, dictionary: dict[str, object]) -> re.Pattern[str]:
    matching = object_dict(dictionary["matching"], "matching")
    template = str(matching["phrase_boundary_regex_template"])
    return re.compile(template.format(phrase=re.escape(phrase)), re.IGNORECASE)


def dictionary_matches(sentence: str, dictionary: dict[str, object]) -> list[Match]:
    """Return longest-first canonical dictionary substitutions in one sentence."""
    entries = object_list(dictionary["substitutions"], "substitutions")
    candidates: list[tuple[str, dict[str, object]]] = []
    for raw in entries:
        entry = object_dict(raw, "substitution")
        for common in object_list(entry["common"], "common"):
            if isinstance(common, str):
                candidates.append((common, entry))
    candidates.sort(key=lambda item: len(item[0]), reverse=True)
    found: list[Match] = []
    occupied: list[tuple[int, int]] = []
    for common, entry in candidates:
        condition = entry.get("condition")
        if isinstance(condition, str) and not re.search(
            condition, sentence, re.IGNORECASE
        ):
            continue
        for match in phrase_regex(common, dictionary).finditer(sentence):
            if any(
                match.start() < end and start < match.end() for start, end in occupied
            ):
                continue
            approved = entry["approved"]
            if isinstance(approved, str) or (
                isinstance(approved, list)
                and all(isinstance(item, str) for item in approved)
            ):
                found.append(
                    Match(
                        match.group(0),
                        approved,
                        bool(entry.get("auto")),
                        str(entry.get("note", "")),
                        match.start(),
                        match.end(),
                    )
                )
                occupied.append((match.start(), match.end()))
    return sorted(found, key=lambda item: item.start)


def replacement_case(matched: str, replacement: str) -> str:
    """Apply the exact configured capitalization policy to a replacement."""
    if matched.isupper():
        return replacement.upper()
    if matched[:1].isupper() and not matched.isupper():
        return replacement[:1].upper() + replacement[1:]
    return replacement


# 4. rule registry/implementations
def rule_entries(
    rules: dict[str, object], only: set[str] | None, ignore: set[str] | None
) -> list[dict[str, object]]:
    items: list[dict[str, object]] = []
    for raw in object_list(rules["rules"], "rules"):
        item = object_dict(raw, "rule")
        identifier = str(item["id"])
        if (
            item["check"] != "none"
            and (only is None or identifier in only)
            and (ignore is None or identifier not in ignore)
        ):
            items.append(item)
    return items


def finding(
    rule: dict[str, object],
    file: str,
    sentence: Sentence,
    message: str | None = None,
    suggestion: str | None = None,
    severity: str | None = None,
) -> Finding:
    return Finding(
        str(rule["id"]),
        severity or str(rule["severity"]),
        file,
        sentence.line,
        sentence.column,
        message or str(rule["message"]),
        sentence.text,
        suggestion,
    )


def procedural(sentence: Sentence, verbs: set[str]) -> bool:
    words = prose_words(sentence.text.lower())
    return sentence.block.kind == "list" or bool(words and words[0] in verbs)


def check_sentence_words(
    rule: dict[str, object], sentence: Sentence, file: str, _: dict[str, object]
) -> list[Finding]:
    params = object_dict(rule["params"], "params")
    max_words = int(str(params["max"]))
    verbs = {
        str(item) for item in object_list(params.get("command_verbs", []), "verbs")
    }
    is_procedural = procedural(sentence, verbs)
    scope = str(params["scope"])
    applies = (scope == "procedural" and is_procedural) or (
        scope == "descriptive" and not is_procedural
    )
    return (
        [finding(rule, file, sentence, str(rule["message"]).format(max=max_words))]
        if applies and word_count(sentence.text) > max_words
        else []
    )


def check_pattern(
    rule: dict[str, object], sentence: Sentence, file: str, _: dict[str, object]
) -> list[Finding]:
    params = object_dict(rule["params"], "params")
    patterns = [
        str(value) for value in object_list(params.get("patterns", []), "patterns")
    ]
    return (
        [finding(rule, file, sentence)]
        if any(item.lower() in sentence.text.lower() for item in patterns)
        else []
    )


def check_passive(
    rule: dict[str, object], sentence: Sentence, file: str, _: dict[str, object]
) -> list[Finding]:
    auxiliaries = [
        re.escape(str(item))
        for item in object_list(
            object_dict(rule["params"], "params").get("auxiliaries", []),
            "auxiliaries",
        )
    ]
    pattern = r"\b(?:" + "|".join(auxiliaries) + r")\s+\w+(?:ed|en)\b"
    return (
        [finding(rule, file, sentence)]
        if re.search(pattern, sentence.text, re.IGNORECASE)
        else []
    )


def check_tense(
    rule: dict[str, object], sentence: Sentence, file: str, _: dict[str, object]
) -> list[Finding]:
    if sentence.block.kind != "list":
        return []
    words = object_list(
        object_dict(rule["params"], "params").get("past_auxiliaries", []),
        "past auxiliaries",
    )
    return (
        [finding(rule, file, sentence)]
        if any(
            re.search(rf"\b{re.escape(str(word))}\b", sentence.text, re.IGNORECASE)
            for word in words
        )
        else []
    )


def check_command(
    rule: dict[str, object], sentence: Sentence, file: str, _: dict[str, object]
) -> list[Finding]:
    if sentence.block.kind != "list":
        return []
    params = object_dict(rule["params"], "params")
    words = prose_words(sentence.text.lower())
    verbs = {str(item) for item in object_list(params["verbs"], "verbs")}
    if not words or words[0] in verbs or bool(params.get("allow_caller_domain_verbs")):
        return []
    return (
        [finding(rule, file, sentence)]
        if bool(params.get("flag_unlisted_leading_verb"))
        else []
    )


def check_missing_article(
    rule: dict[str, object], sentence: Sentence, file: str, _: dict[str, object]
) -> list[Finding]:
    words = prose_words(sentence.text.lower())
    params = object_dict(rule["params"], "params")
    verbs = {str(item) for item in object_list(params["command_verbs"], "verbs")}
    determiners = {
        str(item) for item in object_list(params["allowed_determiners"], "determiners")
    }
    if (
        sentence.block.kind == "list"
        and len(words) > 1
        and words[0] in verbs
        and words[1] not in determiners
        and not words[1].isdigit()
    ):
        return [finding(rule, file, sentence)]
    return []


def check_gerund(
    rule: dict[str, object], sentence: Sentence, file: str, _: dict[str, object]
) -> list[Finding]:
    words = prose_words(sentence.text)
    return (
        [finding(rule, file, sentence)]
        if sentence.block.kind == "list" and words and words[0].lower().endswith("ing")
        else []
    )


def check_synonyms(
    rule: dict[str, object], sentence: Sentence, file: str, _: dict[str, object]
) -> list[Finding]:
    # Document-level state is handled in check_text to avoid repeated findings.
    return []


def check_one_meaning(
    rule: dict[str, object],
    sentence: Sentence,
    file: str,
    dictionary: dict[str, object],
) -> list[Finding]:
    results: list[Finding] = []
    for raw in object_list(dictionary["one_meaning"], "one meaning"):
        entry = object_dict(raw, "one meaning entry")
        for phrase in object_list(entry["forbidden_phrases"], "forbidden phrases"):
            if isinstance(phrase, str) and phrase_regex(phrase, dictionary).search(
                sentence.text
            ):
                results.append(
                    finding(
                        rule,
                        file,
                        sentence,
                        suggestion=str(entry.get("note", "")),
                    )
                )
    return results


def check_banned(
    rule: dict[str, object],
    sentence: Sentence,
    file: str,
    dictionary: dict[str, object],
) -> list[Finding]:
    results: list[Finding] = []
    for match in dictionary_matches(sentence.text, dictionary):
        approved = (
            match.approved
            if isinstance(match.approved, str)
            else " or ".join(match.approved)
        )
        results.append(finding(rule, file, sentence, suggestion=approved))
    return results


def check_hedge(
    rule: dict[str, object],
    sentence: Sentence,
    file: str,
    dictionary: dict[str, object],
) -> list[Finding]:
    results: list[Finding] = []
    params = object_dict(rule["params"], "params")
    for raw in object_list(params["patterns"], "hedges"):
        item = object_dict(raw, "hedge")
        phrase = str(item["text"])
        if phrase_regex(phrase, dictionary).search(sentence.text):
            results.append(
                finding(
                    rule,
                    file,
                    sentence,
                    severity=str(item.get("severity", params["default_severity"])),
                )
            )
    return results


def check_second_person(
    rule: dict[str, object], sentence: Sentence, file: str, _: dict[str, object]
) -> list[Finding]:
    if sentence.block.kind != "list":
        return []
    terms = object_list(
        object_dict(rule["params"], "params")["third_person_actor_terms"],
        "actor terms",
    )
    return (
        [finding(rule, file, sentence)]
        if any(
            re.search(rf"\b{re.escape(str(term))}\b", sentence.text, re.IGNORECASE)
            for term in terms
        )
        else []
    )


CHECKS: dict[
    str,
    Callable[[dict[str, object], Sentence, str, dict[str, object]], list[Finding]],
] = {
    "sentence_words": check_sentence_words,
    "compound_instruction": check_pattern,
    "passive_voice": check_passive,
    "tense": check_tense,
    "command_verb_start": check_command,
    "missing_article": check_missing_article,
    "gerund_start": check_gerund,
    "synonym_pair": check_synonyms,
    "one_meaning": check_one_meaning,
    "banned_word": check_banned,
    "hedge": check_hedge,
    "second_person": check_second_person,
}


def check_text(
    text: str,
    file: str,
    rules: dict[str, object],
    dictionary: dict[str, object],
    only: set[str] | None = None,
    ignore: set[str] | None = None,
) -> list[Finding]:
    """Check prose blocks and return stable, source-located findings."""
    selected = rule_entries(rules, only, ignore)
    findings: list[Finding] = []
    blocks = markdown_blocks(text)
    sentences = [
        sentence
        for block in blocks
        if block.prose
        for sentence in split_sentences(block.text, block.line, dictionary, block)
    ]
    command_verbs: set[str] = set()
    for candidate in rule_entries(rules, None, None):
        if candidate["check"] == "command_verb_start":
            command_verbs = {
                str(item)
                for item in object_list(
                    object_dict(candidate["params"], "params")["verbs"], "verbs"
                )
            }
            break
    for rule in selected:
        checker = CHECKS.get(str(rule["check"]))
        if checker:
            if rule["check"] == "sentence_words":
                params = object_dict(rule["params"], "params") | {
                    "command_verbs": list(command_verbs)
                }
                rule = rule | {"params": params}
            findings.extend(
                item
                for sentence in sentences
                for item in checker(rule, sentence, file, dictionary)
            )
        if rule["check"] == "paragraph_length":
            max_sentences = int(str(object_dict(rule["params"], "params")["max"]))
            for block in blocks:
                block_sentences = split_sentences(
                    block.text, block.line, dictionary, block
                )
                if (
                    block.prose
                    and any(
                        procedural(sentence, command_verbs)
                        for sentence in block_sentences
                    )
                    and len(block_sentences) > max_sentences
                ):
                    findings.append(
                        finding(
                            rule,
                            file,
                            Sentence(block.text, block.line, 1, block),
                            str(rule["message"]).format(max=max_sentences),
                        )
                    )
        if rule["check"] == "list_required":
            min_steps = int(str(object_dict(rule["params"], "params")["min_steps"]))
            for block in blocks:
                words = prose_words(block.text.lower())
                if (
                    block.kind == "paragraph"
                    and sum(word in {"then", "next", "after"} for word in words) + 1
                    >= min_steps
                ):
                    findings.append(
                        finding(
                            rule,
                            file,
                            Sentence(block.text, block.line, 1, block),
                            str(rule["message"]).format(min_steps=min_steps),
                        )
                    )
        if rule["check"] == "warning_block":
            terms = {
                str(term).lower()
                for term in object_list(
                    object_dict(rule["params"], "params")["terms"], "terms"
                )
            }
            for block in blocks:
                first = prose_words(block.text.lower())[:1]
                if any(
                    re.search(rf"\b{re.escape(term)}\b", block.text, re.IGNORECASE)
                    for term in terms
                ) and (not first or first[0] not in terms):
                    findings.append(
                        finding(
                            rule,
                            file,
                            Sentence(block.text, block.line, 1, block),
                        )
                    )
        if rule["check"] == "synonym_pair":
            lowered = "\n".join(block.text.lower() for block in blocks if block.prose)
            for pair in object_list(
                object_dict(rule["params"], "params")["pairs"], "pairs"
            ):
                terms = object_list(pair, "synonym pair")
                if len(terms) == 2 and all(
                    re.search(rf"\b{re.escape(str(term))}\b", lowered) for term in terms
                ):
                    findings.append(
                        finding(
                            rule,
                            file,
                            Sentence(str(terms[0]), 1, 1, Block("", 1, "document")),
                        )
                    )
    return sorted(findings, key=lambda item: (item.line, item.column, item.rule_id))


# 5. safe fixer
def replace_auto(text: str, dictionary: dict[str, object]) -> str:
    """Replace only auto-approved substitutions outside Markdown code spans and fences."""
    parts = re.split(r"(```[\s\S]*?```|~~~[\s\S]*?~~~|`[^`\n]*`)", text)
    for index in range(0, len(parts), 2):
        segment = parts[index]
        matches = dictionary_matches(segment, dictionary)
        for match in reversed(matches):
            if match.auto and isinstance(match.approved, str):
                segment = (
                    segment[: match.start]
                    + replacement_case(match.common, match.approved)
                    + segment[match.end :]
                )
        parts[index] = segment
    return "".join(parts)


# 6. F1 fact extraction/compare
def facts(text: str, protected: Iterable[str]) -> Counter[str]:
    patterns = {
        "inline_code": r"`[^`\n]+`",
        "fenced_code": r"```[\s\S]*?```",
        "numbers": r"\b\d+(?:\.\d+)*\b",
        "flags": r"--[A-Za-z][\w-]*",
        "paths": r"(?:[\w.-]+/)+[\w.-]+|\b[\w.-]+\.[A-Za-z]{1,8}\b",
        "all_caps_identifiers": r"\b[A-Z][A-Z0-9_]{1,}\b",
    }
    return Counter(
        match.group(0)
        for category in protected
        for match in re.finditer(patterns.get(category, r"(?!)"), text)
    )


def compare_facts(
    before: str,
    after: str,
    rules: dict[str, object],
    _: dict[str, object] | None = None,
) -> list[Finding]:
    """Return F1 findings for multiset protected-fact differences."""
    rule = next(
        object_dict(item, "rule")
        for item in object_list(rules["rules"], "rules")
        if object_dict(item, "rule")["id"] == "F1"
    )
    protected = object_list(
        object_dict(rule["params"], "params")["protected"], "protected"
    )
    changed = facts(before, (str(item) for item in protected)) != facts(
        after, (str(item) for item in protected)
    )
    return (
        [
            Finding(
                "F1",
                "error",
                "compare",
                1,
                1,
                str(rule["message"]),
                "protected facts differ",
            )
        ]
        if changed
        else []
    )


# 7. renderers
def render_text(findings: Iterable[Finding]) -> str:
    """Render concise human-readable diagnostics."""
    return "\n".join(
        f"{item.file}:{item.line}:{item.column}: [{item.rule_id} {item.severity}] {item.message}\n  {item.excerpt}"
        + (f"\n  suggestion: {item.suggestion}" if item.suggestion else "")
        for item in findings
    )


def result_json(file: str, findings: Sequence[Finding]) -> str:
    """Render the documented machine-readable check result."""
    errors = sum(item.severity == "error" for item in findings)
    warnings = sum(item.severity == "warn" for item in findings)
    data = {
        "file": file,
        "clean": not findings,
        "findings": [item.__dict__ for item in findings],
        "summary": {"errors": errors, "warnings": warnings},
    }
    return json.dumps(data, indent=2)


def dictionary_table(dictionary: dict[str, object]) -> str:
    lines = [
        "| Common word or phrase | Approved replacement | Automatic |",
        "| --- | --- | --- |",
    ]
    for raw in object_list(dictionary["substitutions"], "substitutions"):
        entry = object_dict(raw, "substitution")
        approved = entry["approved"]
        lines.append(
            f"| {', '.join(str(item) for item in object_list(entry['common'], 'common'))} | {' / '.join(approved) if isinstance(approved, list) else approved} | {'yes' if entry.get('auto') else 'no'} |"
        )
    return "\n".join(lines)


def render_dictionary(path: Path, dictionary: dict[str, object], check: bool) -> int:
    """Render the marked generated substitutions table, or check for drift."""
    begin = "<!-- BEGIN GENERATED: substitutions (from scripts/dictionary.json) -->"
    end = "<!-- END GENERATED -->"
    try:
        content = path.read_text(encoding="utf-8")
    except OSError as error:
        raise ConfigError(f"cannot read {path}: {error}") from error
    if (
        begin not in content
        or end not in content
        or content.index(begin) > content.index(end)
    ):
        raise ConfigError(
            f"{path} does not contain the generated substitutions markers"
        )
    replacement = f"{begin}\n{dictionary_table(dictionary)}\n{end}"
    updated = re.sub(
        re.escape(begin) + r"[\s\S]*?" + re.escape(end),
        replacement,
        content,
        count=1,
    )
    if check:
        return 0 if updated == content else 1
    path.write_text(updated, encoding="utf-8")
    return 0


# 8. CLI
def parse_ids(value: str | None) -> set[str] | None:
    return (
        {item.strip() for item in value.split(",") if item.strip()} if value else None
    )


def parser() -> argparse.ArgumentParser:
    """Build the command-line parser."""
    result = argparse.ArgumentParser(description=__doc__)
    sub = result.add_subparsers(dest="command", required=True)
    check = sub.add_parser("check")
    check.add_argument("file", nargs="?", default="-")
    check.add_argument("--compare", nargs=2, metavar=("BEFORE", "AFTER"))
    check.add_argument("--format", choices=("text", "json"), default="text")
    check.add_argument("--strict", action="store_true")
    check.add_argument("--only")
    check.add_argument("--ignore")
    fix = sub.add_parser("fix")
    fix.add_argument("file")
    fix.add_argument("-w", "--write", action="store_true")
    word = sub.add_parser("dict")
    word.add_argument("word")
    render = sub.add_parser("render-dict")
    render.add_argument("--check", action="store_true")
    for command in (check, fix, word, render):
        command.add_argument("--rules", type=Path, help=argparse.SUPPRESS)
        command.add_argument("--dictionary", type=Path, help=argparse.SUPPRESS)
    return result


def main(argv: Sequence[str] | None = None) -> int:
    """Run the STE command-line interface."""
    args = parser().parse_args(argv)
    try:
        rules, dictionary = load_config(args.rules, args.dictionary)
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
                text = (
                    sys.stdin.read()
                    if args.file == "-"
                    else Path(args.file).read_text(encoding="utf-8")
                )
                file = args.file
                findings = check_text(text, file, rules, dictionary, only, ignore)
            print(
                result_json(file, findings)
                if args.format == "json"
                else render_text(findings)
            )
            return (
                1
                if any(
                    item.severity == "error"
                    or (args.strict and item.severity == "warn")
                    for item in findings
                )
                else 0
            )
        if args.command == "fix":
            path = Path(args.file)
            before = path.read_text(encoding="utf-8")
            after = replace_auto(before, dictionary)
            if args.write:
                path.write_text(after, encoding="utf-8")
                print(f"applied {before != after}; deferred manual substitutions")
            else:
                print(
                    "".join(
                        difflib.unified_diff(
                            before.splitlines(True),
                            after.splitlines(True),
                            fromfile=str(path),
                            tofile=str(path),
                        )
                    )
                )
            return 0
        if args.command == "dict":
            matches = dictionary_matches(args.word, dictionary)
            for match in matches:
                print(f"{match.common}: {match.approved}")
            for raw in object_list(dictionary["one_meaning"], "one meaning"):
                entry = object_dict(raw, "one meaning entry")
                if str(entry["word"]).lower() == args.word.lower():
                    print(f"{entry['word']}: {entry['note']}")
                    return 0
            if not matches:
                print("not in the starter substitution list")
            return 0
        return render_dictionary(ROOT.parent / "DICTIONARY.md", dictionary, args.check)
    except (ConfigError, OSError, UnicodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

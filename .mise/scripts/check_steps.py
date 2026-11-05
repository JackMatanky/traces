#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.13"
# dependencies = []
# ///
"""Check that feature steps and step definitions match one to one.

Fails on an undefined step, an unused step definition, a duplicate definition,
and an ambiguous line (two patterns tie on specificity). `scenarios!` reports
the first two only at run time, and never reports the others.
"""

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from re import Match, Pattern
from typing import Final

DEFINITION: Final[Pattern[str]] = re.compile(
    r'#\[(given|when|then)\(\s*(?:r#"(.*?)"#|"((?:[^"\\]|\\.)*)")\s*\)\]',
    re.DOTALL,
)
STEP: Final[Pattern[str]] = re.compile(
    r"^\s*(Given|When|Then|And|But)\s+(.*?)\s*$"
)
PLACEHOLDER: Final[Pattern[str]] = re.compile(r"(\{[^{}]*\})")
COMMENT_LINE: Final[Pattern[str]] = re.compile(r"(?m)^\s*//.*$")
DOCSTRING_FENCES: Final[frozenset[str]] = frozenset({'"""', "```"})
CONTINUATION: Final[Pattern[str]] = re.compile(r"\\\n\s*")
ESCAPE: Final[Pattern[str]] = re.compile(r"\\(.)")


@dataclass(frozen=True)
class Definition:
    """A step definition parsed from a Rust attribute."""

    keyword: str
    pattern: str
    source: str
    matcher: Pattern[str]
    specificity: tuple[int, int]


@dataclass(frozen=True)
class FeatureStep:
    """One step line of a feature file, with its effective keyword."""

    keyword: str
    text: str
    where: str


def is_placeholder(part: str) -> bool:
    return part.startswith("{") and part.endswith("}")


def unescape(literal: str) -> str:
    joined: str = CONTINUATION.sub("", literal)
    return ESCAPE.sub(r"\1", joined)


def regex_of(pattern: str) -> Pattern[str]:
    parts: list[str] = PLACEHOLDER.split(pattern)
    body: str = "".join(
        "(.+?)" if is_placeholder(part) else re.escape(part) for part in parts
    )
    return re.compile(f"^{body}$")


def specificity(pattern: str) -> tuple[int, int]:
    """Mirror the `rstest-bdd` tie-break: more literal text, then fewer holes."""
    parts: list[str] = PLACEHOLDER.split(pattern)
    literal: int = sum(len(p) for p in parts if not is_placeholder(p))
    holes: int = sum(1 for p in parts if is_placeholder(p))
    return literal, -holes


def definitions(steps: Path) -> list[Definition]:
    found: list[Definition] = []
    for source in sorted(steps.rglob("*.rs")):
        text: str = COMMENT_LINE.sub("", source.read_text())
        for match in DEFINITION.finditer(text):
            raw: str | None = match.group(2)
            escaped: str | None = match.group(3)
            pattern: str = raw if raw is not None else unescape(escaped or "")
            found.append(
                Definition(
                    keyword=match.group(1),
                    pattern=pattern,
                    source=source.name,
                    matcher=regex_of(pattern),
                    specificity=specificity(pattern),
                )
            )
    return found


def feature_steps(features: Path) -> list[FeatureStep]:
    found: list[FeatureStep] = []
    for feature in sorted(features.rglob("*.feature")):
        keyword: str = "given"
        fence: str | None = None
        for number, line in enumerate(feature.read_text().splitlines(), 1):
            marker: str = line.strip()
            if fence is not None:
                if marker == fence:
                    fence = None
                continue
            if marker in DOCSTRING_FENCES:
                fence = marker
                continue
            match: Match[str] | None = STEP.match(line)
            if match is None:
                continue
            if match.group(1) not in ("And", "But"):
                keyword = match.group(1).lower()
            found.append(
                FeatureStep(keyword, match.group(2), f"{feature.name}:{number}")
            )
    return found


def check(root: Path) -> list[str]:
    steps: list[Definition] = definitions(root / "tests/features/steps")
    errors: list[str] = []
    seen: set[tuple[str, str]] = set()
    for step in steps:
        key: tuple[str, str] = (step.keyword, step.pattern)
        if key in seen:
            errors.append(
                "ERROR: Duplicate step definition: "
                f"{step.keyword} '{step.pattern}' ({step.source})"
            )
        seen.add(key)
    used: set[tuple[str, str]] = set()
    for line in feature_steps(root / "tests/features"):
        hits: list[Definition] = [
            d
            for d in steps
            if d.keyword == line.keyword and d.matcher.match(line.text)
        ]
        if not hits:
            errors.append(
                f"ERROR: Undefined step at {line.where}: "
                f"{line.keyword} '{line.text}'"
            )
            continue
        ranked: list[Definition] = sorted(
            hits, key=lambda d: d.specificity, reverse=True
        )
        used.add((ranked[0].keyword, ranked[0].pattern))
        if len(ranked) > 1 and ranked[0].specificity == ranked[1].specificity:
            errors.append(
                f"ERROR: Ambiguous step at {line.where}: '{line.text}' matches "
                f"'{ranked[0].pattern}' and '{ranked[1].pattern}'"
            )
    errors.extend(
        f"ERROR: Unused step definition: {d.keyword} '{d.pattern}' ({d.source})"
        for d in steps
        if (d.keyword, d.pattern) not in used
    )
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", default=".", type=Path)
    root: Path = parser.parse_args().root
    errors: list[str] = check(root)
    for error in errors:
        print(error, file=sys.stderr)
    return int(bool(errors))


if __name__ == "__main__":
    sys.exit(main())

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
from collections import Counter
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
PLACEHOLDER: Final[Pattern[str]] = re.compile(r"(\{[A-Za-z_][A-Za-z0-9_]*(?::[A-Za-z0-9_]+)?\})")
HINT: Final[Pattern[str]] = re.compile(r"\{[A-Za-z0-9_]+(?::(?P<hint>[A-Za-z0-9_]+))?\}")
UNSUPPORTED_LITERAL: Final[Pattern[str]] = re.compile(r"[{}\\]")
# Mirrors `rstest_bdd_patterns::get_type_pattern`; any other hint is `.+?`.
UNSIGNED: Final[str] = r"\d+"
SIGNED: Final[str] = r"[+-]?\d+"
FLOAT: Final[str] = r"(?i:(?:[+-]?(?:\d+\.\d*|\.\d+|\d+)(?:[eE][+-]?\d+)?|nan|inf|infinity))"
QUOTED: Final[str] = r""""(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'"""
HINT_REGEX: Final[dict[str, str]] = {
    **dict.fromkeys(("u8", "u16", "u32", "u64", "u128", "usize"), UNSIGNED),
    **dict.fromkeys(("i8", "i16", "i32", "i64", "i128", "isize"), SIGNED),
    **dict.fromkeys(("f32", "f64"), FLOAT),
    "string": QUOTED,
}
# Mirrors `regex::escape`: space and quotes stay literal.
RUST_ESCAPED: Final[frozenset[str]] = frozenset("\\.+*?()|[]{}^$#&-~")
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
    specificity: tuple[int, int, int]


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


class UnsupportedPattern(ValueError):
    """A step pattern uses syntax the mirror does not model; it fails closed."""


def hole_regex(part: str) -> str:
    """Mirror the `rstest-bdd` regex of one placeholder, including its type hint."""
    hint: Match[str] | None = HINT.fullmatch(part)
    if hint is None:
        raise UnsupportedPattern(part)
    return f"({HINT_REGEX.get(hint['hint'] or '', '.+?')})"


def literal_regex(part: str) -> str:
    """Mirror `regex::escape`. Braces and backslashes have framework escape rules."""
    if UNSUPPORTED_LITERAL.search(part):
        raise UnsupportedPattern(part)
    return "".join(f"\\{c}" if c in RUST_ESCAPED else c for c in part)


def regex_source(pattern: str) -> str:
    """Return the anchored regex text `rstest-bdd` builds for `pattern`."""
    parts: list[str] = PLACEHOLDER.split(pattern)
    body: str = "".join(
        hole_regex(part) if is_placeholder(part) else literal_regex(part)
        for part in parts
    )
    return f"^{body}$"


def regex_of(pattern: str) -> Pattern[str]:
    return re.compile(regex_source(pattern))


def specificity(pattern: str) -> tuple[int, int, int]:
    """Mirror `SpecificityScore`: more literal text, then fewer holes, then more typed holes."""
    parts: list[str] = PLACEHOLDER.split(pattern)
    holes: list[str] = [p for p in parts if is_placeholder(p)]
    literal: int = sum(len(p) for p in parts if not is_placeholder(p))
    typed: int = sum(1 for p in holes if ":" in p)
    return literal, -len(holes), typed


def mirror_columns(pattern: str) -> tuple[str, str]:
    """Return the regex and the `literal/holes/typed` score, or `ERROR` twice."""
    try:
        regex: str = regex_source(pattern)
    except UnsupportedPattern:
        return "ERROR", "ERROR"
    literal, holes, typed = specificity(pattern)
    return regex, f"{literal}/{-holes}/{typed}"


def definitions(steps: Path) -> tuple[list[Definition], list[str]]:
    found: list[Definition] = []
    errors: list[str] = []
    for source in sorted(steps.rglob("*.rs")):
        text: str = COMMENT_LINE.sub("", source.read_text())
        for match in DEFINITION.finditer(text):
            raw: str | None = match.group(2)
            escaped: str | None = match.group(3)
            pattern: str = raw if raw is not None else unescape(escaped or "")
            try:
                matcher: Pattern[str] = regex_of(pattern)
            except UnsupportedPattern:
                errors.append(
                    "ERROR: Step pattern outside the checked syntax: "
                    f"{match.group(1)} '{pattern}' ({source.name})"
                )
                continue
            found.append(
                Definition(
                    keyword=match.group(1),
                    pattern=pattern,
                    source=source.name,
                    matcher=matcher,
                    specificity=specificity(pattern),
                )
            )
    return found, errors


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


def check_baseline(baseline: Path) -> list[str]:
    """Compare the mirror with the framework values the Rust test pins."""
    errors: list[str] = []
    for number, line in enumerate(baseline.read_text().splitlines(), 1):
        pattern, *framework = line.split("\t")
        mirror: list[str] = list(mirror_columns(pattern))
        if mirror != framework:
            errors.append(
                f"ERROR: Mirror differs from the framework at {baseline.name}:"
                f"{number}: '{pattern}' mirror {mirror} framework {framework}"
            )
    return errors


def check_registry(registry: Path, steps: list[Definition]) -> list[str]:
    """Compare every registered step with the mirror and with the parsed definitions."""
    errors: list[str] = []
    registered: Counter[tuple[str, str]] = Counter()
    for line in registry.read_text().splitlines():
        keyword, pattern, *framework = line.split("\t")
        registered[(keyword, pattern)] += 1
        mirror: list[str] = list(mirror_columns(pattern))
        if mirror != framework:
            errors.append(
                f"ERROR: Mirror differs from the framework registry for {keyword} "
                f"'{pattern}': mirror {mirror} framework {framework}"
            )
    parsed: Counter[tuple[str, str]] = Counter(
        (d.keyword, d.pattern) for d in steps
    )
    errors.extend(
        f"ERROR: Step registered but not found in the step files: {k} '{p}'"
        for k, p in sorted((registered - parsed).elements())
    )
    errors.extend(
        f"ERROR: Step found in the step files but not registered: {k} '{p}'"
        for k, p in sorted((parsed - registered).elements())
    )
    return errors


def check(root: Path) -> list[str]:
    steps, errors = definitions(root / "tests/features/steps")
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
    parser.add_argument(
        "--baseline",
        type=Path,
        help="TSV of probe patterns with framework regex and score, compared with the mirror",
    )
    parser.add_argument(
        "--registry",
        type=Path,
        help="TSV of registered steps with framework regex and score, written by the features binary",
    )
    args: argparse.Namespace = parser.parse_args()
    errors: list[str] = check(args.root)
    if args.baseline is not None:
        errors.extend(check_baseline(args.baseline))
    if args.registry is not None:
        steps, _ = definitions(args.root / "tests/features/steps")
        errors.extend(check_registry(args.registry, steps))
    for error in errors:
        print(error, file=sys.stderr)
    return int(bool(errors))


if __name__ == "__main__":
    sys.exit(main())

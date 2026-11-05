#!/usr/bin/env python3
"""Check that feature steps and step definitions match one to one.

Fails on an undefined step, an unused step definition, a duplicate definition,
and an ambiguous line (two patterns tie on specificity). `scenarios!` reports
the first two only at run time, and never reports the others.
"""

import argparse
from pathlib import Path
import re
import sys

DEFINITION = re.compile(
    r'#\[(given|when|then)\(\s*(?:r#"(.*?)"#|"((?:[^"\\]|\\.)*)")\s*\)\]', re.DOTALL
)
STEP = re.compile(r"^\s*(Given|When|Then|And|But)\s+(.*?)\s*$")
PLACEHOLDER = re.compile(r"(\{[^{}]*\})")


def unescape(literal: str) -> str:
    literal = re.sub(r"\\\n\s*", "", literal)
    return re.sub(r"\\(.)", r"\1", literal)


def definitions(steps: Path) -> list[tuple[str, str, str]]:
    found = []
    for source in sorted(steps.rglob("*.rs")):
        for match in DEFINITION.finditer(source.read_text()):
            raw, escaped = match.group(2), match.group(3)
            pattern = raw if raw is not None else unescape(escaped)
            found.append((match.group(1), pattern, source.name))
    return found


def regex_of(pattern: str) -> re.Pattern[str]:
    parts = PLACEHOLDER.split(pattern)
    body = "".join(
        "(.+?)" if part.startswith("{") and part.endswith("}") else re.escape(part)
        for part in parts
    )
    return re.compile(f"^{body}$")


def specificity(pattern: str) -> tuple[int, int]:
    parts = PLACEHOLDER.split(pattern)
    literal = sum(len(p) for p in parts if not (p.startswith("{") and p.endswith("}")))
    return literal, -sum(1 for p in parts if p.startswith("{") and p.endswith("}"))


def feature_steps(features: Path) -> list[tuple[str, str, str]]:
    lines = []
    for feature in sorted(features.rglob("*.feature")):
        keyword, in_docstring = "given", False
        for number, line in enumerate(feature.read_text().splitlines(), 1):
            if line.strip() == '"""':
                in_docstring = not in_docstring
                continue
            if in_docstring or not (match := STEP.match(line)):
                continue
            if match.group(1) not in ("And", "But"):
                keyword = match.group(1).lower()
            lines.append((keyword, match.group(2), f"{feature.name}:{number}"))
    return lines


def check(root: Path) -> list[str]:
    steps = definitions(root / "tests/features/steps")
    compiled = [(kw, pat, regex_of(pat), specificity(pat)) for kw, pat, _ in steps]
    errors = []
    seen = set()
    for keyword, pattern, source in steps:
        if (keyword, pattern) in seen:
            errors.append(f"ERROR: Duplicate step definition: {keyword} '{pattern}' ({source})")
        seen.add((keyword, pattern))
    used = set()
    for keyword, text, where in feature_steps(root / "tests/features"):
        hits = [c for c in compiled if c[0] == keyword and c[2].match(text)]
        if not hits:
            errors.append(f"ERROR: Undefined step at {where}: {keyword} '{text}'")
            continue
        ranked = sorted(hits, key=lambda c: c[3], reverse=True)
        used.add((ranked[0][0], ranked[0][1]))
        if len(ranked) > 1 and ranked[0][3] == ranked[1][3]:
            errors.append(
                f"ERROR: Ambiguous step at {where}: '{text}' matches "
                f"'{ranked[0][1]}' and '{ranked[1][1]}'"
            )
    errors += [
        f"ERROR: Unused step definition: {keyword} '{pattern}' ({source})"
        for keyword, pattern, source in steps
        if (keyword, pattern) not in used
    ]
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", default=".", type=Path)
    errors = check(parser.parse_args().root)
    for error in errors:
        print(error, file=sys.stderr)
    return int(bool(errors))


if __name__ == "__main__":
    sys.exit(main())

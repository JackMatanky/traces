#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.13"
# dependencies = []
# ///
"""Check both directions of the research/10 scenario ownership contract."""

import argparse
import re
import sys
from pathlib import Path
from re import Match, Pattern
from typing import Final

ROW: Final[Pattern[str]] = re.compile(
    r"^\|\s*(\d{2}-[\w-]+)\s*\|.*\|\s*covered:\s*(.*?)\s+"
    r"\(reviewed\s+\d{4}-\d{2}-\d{2}\)\s*\|$"
)
SCENARIO: Final[Pattern[str]] = re.compile(
    r"^\s*Scenario(?: Outline)?:\s*(.*?)\s*$"
)


def declared_scenarios(features: Path) -> set[str]:
    titles: set[str] = set()
    for feature in features.rglob("*.feature"):
        for line in feature.read_text().splitlines():
            match: Match[str] | None = SCENARIO.match(line)
            if match is not None:
                titles.add(match.group(1))
    return titles


def check(root: Path) -> list[str]:
    scenarios: set[str] = declared_scenarios(root / "tests/features")
    if not scenarios:
        return ["ERROR: No scenarios found in tests/features/"]

    research: Path = root / ".scratch/bdd-strategy/research/10-invariance-map.md"
    rows: list[tuple[str, str]] = []
    for line in research.read_text().splitlines():
        row: Match[str] | None = ROW.match(line)
        if row is not None:
            rows.append((row.group(1), row.group(2).strip()))
    errors: list[str] = [
        f"ERROR: Research 10 row {row_id} points to missing scenario: '{title}'"
        for row_id, titles in rows
        for title in titles.split(";")
        if title.strip() not in scenarios
    ]

    owners: Path = root / ".scratch/bdd-testing/scenario-owners.md"
    owned: set[str] = set()
    if owners.is_file():
        for line in owners.read_text().splitlines():
            if not line.startswith("|") or line.startswith("|---"):
                continue
            columns: list[str] = [c.strip() for c in line.strip("|").split("|")]
            if len(columns) != 2 or columns[0] == "Scenario title":
                continue
            title, reason = columns
            if not reason:
                errors.append(f"ERROR: Scenario owner has no reason: '{title}'")
            if title not in scenarios:
                errors.append(
                    f"ERROR: Scenario owner points to missing scenario: '{title}'"
                )
            if title in owned:
                errors.append(f"ERROR: Duplicate scenario owner: '{title}'")
            owned.add(title)
    cited: set[str] = {t.strip() for _, titles in rows for t in titles.split(";")}
    errors.extend(
        f"ERROR: Uncited scenario: '{title}'"
        for title in sorted(scenarios - cited - owned)
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

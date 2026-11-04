#!/usr/bin/env python3
"""Check both directions of the research/10 scenario ownership contract."""

import argparse
from pathlib import Path
import re
import sys


ROW = re.compile(
    r"^\|\s*(\d{2}-[\w-]+)\s*\|.*\|\s*covered:\s*(.*?)\s+"
    r"\(reviewed\s+\d{4}-\d{2}-\d{2}\)\s*\|$"
)
SCENARIO = re.compile(r"^\s*Scenario(?: Outline)?:\s*(.*?)\s*$")


def check(root: Path) -> list[str]:
    features = root / "tests/features"
    scenarios = {
        match.group(1)
        for feature in features.rglob("*.feature")
        for line in feature.read_text().splitlines()
        if (match := SCENARIO.match(line))
    }
    if not scenarios:
        return ["ERROR: No scenarios found in tests/features/"]

    research = root / ".scratch/bdd-strategy/research/10-invariance-map.md"
    rows = [
        (match.group(1), match.group(2).strip())
        for line in research.read_text().splitlines()
        if (match := ROW.match(line))
    ]
    errors = [
        f"ERROR: Research 10 row {row_id} points to missing scenario: '{title}'"
        for row_id, titles in rows
        for title in titles.split(";")
        if title.strip() not in scenarios
    ]

    owners = root / ".scratch/bdd-testing/scenario-owners.md"
    owned: set[str] = set()
    if owners.is_file():
        for line in owners.read_text().splitlines():
            if not line.startswith("|") or line.startswith("|---"):
                continue
            columns = [column.strip() for column in line.strip("|").split("|")]
            if len(columns) != 2 or columns[0] == "Scenario title":
                continue
            title, reason = columns
            if not reason:
                errors.append(f"ERROR: Scenario owner has no reason: '{title}'")
            if title not in scenarios:
                errors.append(f"ERROR: Scenario owner points to missing scenario: '{title}'")
            if title in owned:
                errors.append(f"ERROR: Duplicate scenario owner: '{title}'")
            owned.add(title)
    for title in sorted(
        scenarios - {t.strip() for _, titles in rows for t in titles.split(";")} - owned
    ):
        errors.append(f"ERROR: Uncited scenario: '{title}'")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", default=".", type=Path)
    args = parser.parse_args()
    errors = check(args.root)
    for error in errors:
        print(error, file=sys.stderr)
    return int(bool(errors))


if __name__ == "__main__":
    sys.exit(main())

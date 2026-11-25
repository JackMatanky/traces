#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.13"
# dependencies = []
# ///
"""Generate or check `.pi/pi-warden.json` from the agent rule files.

The pi-warden rules engine does not read folders, and a project `rules.files`
list replaces the global list. This script rebuilds the project list from two
folders: the global rules in `~/.agents/rules` and the project rules in
`.agents/rules`.
"""

import argparse
import json
import os
import sys
from pathlib import Path

PROJECT_RULES_DIR = Path(".agents/rules")
PROJECT_RULE_GLOB = "traces-*.md"
OUTPUT = Path(".pi/pi-warden.json")

# `README.md` documents a rules folder. It is never a rule file.
NON_RULE_FILES = frozenset({"README.md"})

EXCLUDE = [
    ".scratch/**",
    ".worktrees/**",
    ".rust-docs/**",
    ".criterion/**",
    "fuzz/target/**",
]

# A project `rules.sensitivePaths` map replaces the user map. These six entries
# repeat the global map from the chezmoi `pi-warden/config.json.tmpl`. Update
# them here when that template changes.
GLOBAL_SENSITIVE_PATHS = {
    "**/migrations/**": "Verify database migration compatibility and rollback safety.",
    "**/permissions/**": "Changes to permission enforcement require careful security review.",
    "**/auth/**": "Changes to authentication code require explicit verification.",
    "**/.github/workflows/**": "Validate GitHub Actions permissions, secrets handling, and CI behavior.",
    "**/Cargo.lock": "Review dependency changes and affected versions.",
    "**/uv.lock": "Review dependency changes and affected versions.",
}

PROJECT_SENSITIVE_PATHS = {
    "**/Cargo.toml": "Review dependency, feature, and lint changes.",
    "**/clippy.toml": "Review lint threshold changes against CODING_STANDARDS.",
    "**/deny.toml": "Review supply-chain policy changes.",
    "mise.toml": "Review task and tool changes. The verify task is the completion gate.",
    "docs/adr/**": "An agent must not mark an ADR accepted. A human reviews every ADR.",
}


def global_rules_dir() -> Path:
    """Return the global rules folder.

    `TRACES_GLOBAL_RULES` overrides the default `~/.agents/rules`.
    """
    override = os.environ.get("TRACES_GLOBAL_RULES")
    return Path(override).expanduser() if override else Path.home() / ".agents/rules"


def build_config(root: Path) -> dict[str, object]:
    """Build the project pi-warden config from the rule folders.

    Args:
        root: The repository root.

    Returns:
        The config object that `.pi/pi-warden.json` holds.

    Raises:
        SystemExit: When the global rules folder has no rule file.
    """
    global_dir = global_rules_dir()
    global_files = sorted(
        str(path.resolve())
        for path in global_dir.glob("*.md")
        if path.name not in NON_RULE_FILES
    )
    if not global_files:
        sys.exit(f"error: no global rule files in {global_dir}. Run chezmoi apply.")
    project_files = sorted(
        path.relative_to(root).as_posix()
        for path in (root / PROJECT_RULES_DIR).glob(PROJECT_RULE_GLOB)
    )
    return {
        "rules": {
            "files": [*global_files, *project_files],
            "exclude": EXCLUDE,
            "sensitivePaths": {**GLOBAL_SENSITIVE_PATHS, **PROJECT_SENSITIVE_PATHS},
        }
    }


def render(config: dict[str, object]) -> str:
    """Render the config as stable, indented JSON with a final newline."""
    return json.dumps(config, indent=2) + "\n"


def main() -> int:
    """Run the `write` or `check` command and return the exit code."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["write", "check"])
    args = parser.parse_args()
    root = Path.cwd()
    expected = render(build_config(root))
    target = root / OUTPUT
    if args.command == "write":
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(expected, encoding="utf-8")
        count = len(json.loads(expected)["rules"]["files"])
        print(f"wrote {OUTPUT} with {count} rule files")
        return 0
    if not target.exists():
        print(f"{OUTPUT} is missing. Run: mise run rules:sync", file=sys.stderr)
        return 1
    if target.read_text(encoding="utf-8") != expected:
        print(f"{OUTPUT} is stale. Run: mise run rules:sync", file=sys.stderr)
        return 1
    print(f"{OUTPUT} is current")
    return 0


if __name__ == "__main__":
    sys.exit(main())

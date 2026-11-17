#!/usr/bin/env python3
"""Sweep short Unicode inputs through one built candidate probe binary.

Usage: unicode-sweep.py <path-to-temporal-cost-probe>

Each input runs in its own process. A non-zero exit status counts as a crash
(the probe builds with `panic = "abort"`). The sweep is a bounded sample, not
an exhaustive proof. It covers Latin, Greek, Hebrew, CJK, emoji and format
characters in six shapes: alone, before `x`, after `ab`, tripled, after `mon`
and before `day`.
"""
import collections
import subprocess
import sys

CODE_POINTS = (
    list(range(0x20, 0x250))
    + list(range(0x370, 0x400))
    + list(range(0x590, 0x600))
    + list(range(0x4E00, 0x4E40))
    + list(range(0x1F600, 0x1F620))
    + [0x200B, 0x202E, 0xFEFF, 0x3000, 0xFF21, 0xFF41, 0x130, 0x131, 0x17F, 0x212A]
)


def inputs() -> list[str]:
    found = set()
    for cp in CODE_POINTS:
        c = chr(cp)
        found.update([c, c + "x", "ab" + c, c * 3, "mon" + c, c + "day"])
    return sorted(found)


def main() -> None:
    binary = sys.argv[1]
    samples = inputs()
    failures = []
    for text in samples:
        run = subprocess.run([binary, text], capture_output=True, text=True)
        if run.returncode != 0:
            lines = run.stderr.strip().splitlines()
            failures.append((text, lines[1] if len(lines) > 1 else run.stderr[:100]))
    print(f"inputs={len(samples)} crashes={len(failures)}")
    for message, count in collections.Counter(m for _, m in failures).most_common(3):
        print(f"{count:5d} {message}")
    print("examples:", [t for t, _ in failures[:8]])


if __name__ == "__main__":
    main()

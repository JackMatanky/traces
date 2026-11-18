#!/usr/bin/env python3
"""Run the bounded candidate matrix and compare explicit-context results.

Pass the directory of separately built probe binaries. Each case uses a fresh
process. JSON lines go to stdout. Diagnostics and totals go to stderr.
These checks describe candidate behavior, not adopted Traces semantics.
"""
from dataclasses import dataclass, replace
from datetime import datetime, timedelta, timezone
from pathlib import Path
from collections import Counter
import json
import os
import subprocess
import sys


@dataclass(frozen=True)
class Case:
    name: str
    text: str
    anchor: str = "2024-03-14T12:00:00Z"
    zone: str = "UTC"
    dialect: str = "us"
    lenient: bool = False


def cases() -> list[Case]:
    result = [
        Case("T1-period", "next week"),
        Case("T2-weekday-us", "next friday"),
        Case("T2-weekday-uk", "next friday", dialect="uk"),
        Case("T3-same-day", "thursday"),
        Case("T6-unicode", "éé"),
        Case("T6-hebrew", "מחר"),
        Case("T7-trailing-time", "10pm meeting"),
        Case("T7-trailing-date", "2024-03-14 x"),
        Case("T8-epoch", "@1758724019"),
        Case("T8-zone-override", 'TZ="UTC" 2 days'),
        Case("T9-month-end", "1 month ago", "2024-03-31T12:00:00Z"),
        Case("T9-leap-year", "1 year ago", "2024-02-29T12:00:00Z"),
        Case("T10-error", "not-a-date"),
        Case("nul", "today\u0000junk"),
        Case("nul-suffix", "tomorrow\u0000"),
    ]
    for name, anchor, zone in [
        ("sao-gap", "2015-10-18T15:00:00Z", "America/Sao_Paulo"),
        ("havana-gap", "2020-03-08T16:00:00Z", "America/Havana"),
        ("havana-fold", "2020-11-01T17:00:00Z", "America/Havana"),
        ("apia-skip", "2011-12-29T22:00:00Z", "Pacific/Apia"),
    ]:
        for text in ("today", "today at 00:00", "00:00", "tomorrow"):
            result.append(Case(f"T4-{name}-{text}", text, anchor, zone))
    monday = datetime(2024, 3, 11, 12, tzinfo=timezone.utc)
    for day in range(7):
        anchor = monday + timedelta(days=day)
        result.append(Case(f"same-day-{day}", anchor.strftime("%A").lower(),
                           anchor.strftime("%Y-%m-%dT%H:%M:%SZ")))
    for text in [
        "1000000 years", "4294967295 years", "4294967296 years",
        "3000000000 days ago", "2147483647 days", "2147483648 days",
        "18446744073709551616 days", "9999999999999999999999 days",
        "-2147483648 days", "0 days", "1 month", "1 month later",
        "next month", "this week", "last week", "eom", "in 2 weeks",
        "TOMORROW", "next\u00a0week", "tomorrow\u200b", "明日",
        "", "   ", "x" * 100000,
    ]:
        label = text if len(text) < 60 else "long-input"
        result.append(Case(f"edge-{label!r}", text))
    result.append(Case("negative-fractional-anchor", "now", "1969-12-31T23:59:59.5Z"))
    result.append(Case("year-boundary", "next week", "2024-12-31T12:00:00Z"))
    return result


def invoke(binary: Path, case: Case, environment: dict[str, str], baseline: bool = False) -> dict:
    args = [str(binary), json.dumps(case.text), case.anchor, case.zone, case.dialect]
    if baseline:
        args.append("--baseline")
    if case.lenient:
        args.append("--lenient")
    run = subprocess.run(args, capture_output=True, text=True, timeout=15, env=environment)
    if run.returncode:
        raise RuntimeError(f"{case.name}: probe exit {run.returncode}: {run.stdout} {run.stderr}")
    record = json.loads(run.stdout)
    record["case"] = case.name
    record["candidate_stderr"] = run.stderr
    first, warmed = record["samples"]
    if first["result"] != warmed["result"]:
        raise AssertionError(f"{case.name}: parser result changed with identical context")
    for sample in record["samples"]:
        if sample["result"]["status"] not in {"OK", "ERROR", "PANIC", "BASELINE"}:
            raise AssertionError(f"{case.name}: unexpected result state")
    if baseline:
        for sample in record["samples"]:
            if sample["result"]["status"] != "BASELINE" or any(sample["call"].values()):
                raise AssertionError(f"{case.name}: baseline measurement contains parser work")
    return record


def main() -> None:
    directory = Path(sys.argv[1]).resolve()
    base_environment = dict(os.environ, TZ="UTC", LC_ALL="C")
    totals: Counter = Counter()
    sample_cases = cases()
    for feature in ("english", "fuzzy", "gnu", "chronoenglish"):
        binary = directory / feature
        selected = sample_cases.copy()
        if feature == "english":
            selected.extend(replace(case, name=case.name + "-lenient", lenient=True)
                            for case in sample_cases if case.name.startswith("T7-"))
        for case in selected:
            record = invoke(binary, case, base_environment)
            print(json.dumps(record, ensure_ascii=True))
            totals[feature, record["samples"][0]["result"]["status"]] += 1
        baseline = invoke(binary, Case("allocation-baseline", "tomorrow"), base_environment, True)
        print(json.dumps(baseline, ensure_ascii=True))
        for zone in ("UTC", "America/Havana", "Europe/Prague"):
            case = Case(f"T5-{zone}", "tomorrow", "2024-03-31T00:30:00Z", zone)
            expected = invoke(binary, case, base_environment)["samples"][0]["result"]
            for host_zone, locale in [("Pacific/Honolulu", "C"), ("Europe/Berlin", "tr_TR.UTF-8"),
                                      ("invalid-zone", "he_IL.UTF-8")]:
                environment = dict(base_environment, TZ=host_zone, LC_ALL=locale,
                                   TZDIR="/nonexistent-traces-probe-tzdb")
                record = invoke(binary, case, environment)
                if record["samples"][0]["result"] != expected:
                    raise AssertionError(f"{feature}: explicit result depends on host {host_zone}/{locale}")
                record["host_variant"] = {"TZ": host_zone, "LC_ALL": locale, "TZDIR": environment["TZDIR"]}
                print(json.dumps(record, ensure_ascii=True))
    for (feature, outcome), count in sorted(totals.items()):
        print(f"{feature} {outcome}={count}", file=sys.stderr)
    print(f"case_definitions={len(sample_cases)} explicit_context_variants=36", file=sys.stderr)


if __name__ == "__main__":
    main()

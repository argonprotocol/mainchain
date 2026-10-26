#!/usr/bin/env python3
"""Emit conservative stateful-review routing signals for a Git diff."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from collections import defaultdict


SIGNALS = {
    "persistence": re.compile(r"sqlite|database|\bdb\b|table|persist|storage|save|insert|update|delete", re.I),
    "recovery": re.compile(r"recover|replay|restore|resume|reconcile|backfill|history", re.I),
    "events": re.compile(r"event|subscription|subscribe|emit|watch|indexer|blockwatch", re.I),
    "retry_ordering": re.compile(r"retry|timeout|poll|queue|race|concurr|pending|inflight|in-flight|generation", re.I),
    "migration": re.compile(r"migrat|schema|versioned|upgrade", re.I),
    "observers": re.compile(r"cache|alert|badge|timer|observer|canonical|reactive|loaded consumer", re.I),
}

PATH_SIGNALS = {
    "persistence": re.compile(r"(^|/)(db|database|tables?|stores?)(/|\.)|migrations?", re.I),
    "recovery": re.compile(r"recover|replay|history|import", re.I),
    "events": re.compile(r"event|watch|indexer|subscription", re.I),
}


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    ).stdout


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", help="Base commit or ref. Defaults to HEAD for working-tree review.")
    parser.add_argument("--head", help="Head commit or ref. Omit to include the working tree.")
    args = parser.parse_args()

    base = args.base or "HEAD"
    diff_args = [base]
    if args.head:
        diff_args.append(args.head)

    files = [path for path in git("diff", "--name-only", *diff_args).splitlines() if path]
    diff = git("diff", "--unified=0", "--no-color", *diff_args)
    changed_lines = "\n".join(
        line[1:]
        for line in diff.splitlines()
        if (line.startswith("+") or line.startswith("-")) and not line.startswith(("+++", "---"))
    )

    evidence: dict[str, set[str]] = defaultdict(set)
    for name, pattern in PATH_SIGNALS.items():
        for path in files:
            if pattern.search(path):
                evidence[name].add(f"path:{path}")

    for name, pattern in SIGNALS.items():
        if pattern.search(changed_lines):
            evidence[name].add("changed-code-match")

    signals = [
        {"category": name, "evidence": sorted(items)}
        for name, items in sorted(evidence.items())
        if items
    ]
    result = {
        "base": base,
        "head": args.head or "working-tree",
        "files_changed": len(files),
        "stateful_review_recommended": bool(signals),
        "signals": signals,
        "note": "Signals route review; they are not findings or proof of completeness.",
    }
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()

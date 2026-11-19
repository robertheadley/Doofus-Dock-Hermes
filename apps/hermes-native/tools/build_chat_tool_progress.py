#!/usr/bin/env python3
"""Build the user's scoped chat/tool progress view from the fixed source overlay."""

from __future__ import annotations

import csv
import json
import os
import re
import tempfile
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
STATUS = ROOT / "apps" / "hermes-native" / "docs" / "status"
SOURCE = STATUS / "IMPLEMENTATION_PROGRESS.csv"
SCOPE = STATUS / "CHAT_TOOL_SCOPE.csv"
OUTPUT = STATUS / "CHAT_TOOL_PROGRESS.csv"
TOKEN = re.compile(r"^(PAR-\d{4})(?:-(PAR-\d{4}))?$")


def read_csv(path: Path) -> tuple[list[str], list[dict[str, str]]]:
    with path.open("r", encoding="utf-8-sig", newline="") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames is None:
            raise ValueError(f"{path} has no CSV header")
        return list(reader.fieldnames), list(reader)


def expand_selector(selector: str) -> list[str]:
    expanded: list[str] = []
    for token in selector.split("|"):
        match = TOKEN.fullmatch(token.strip())
        if not match:
            raise ValueError(f"invalid source test selector: {token!r}")
        first = int(match.group(1).split("-")[1])
        last = int(match.group(2).split("-")[1]) if match.group(2) else first
        if last < first:
            raise ValueError(f"descending source test selector: {token!r}")
        expanded.extend(f"PAR-{number:04d}" for number in range(first, last + 1))
    return expanded


def write_atomic(path: Path, fieldnames: list[str], rows: list[dict[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary_name: str | None = None
    try:
        with tempfile.NamedTemporaryFile(
            "w", encoding="utf-8", newline="", dir=path.parent, delete=False
        ) as handle:
            temporary_name = handle.name
            writer = csv.DictWriter(handle, fieldnames=fieldnames, lineterminator="\n")
            writer.writeheader()
            writer.writerows(rows)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary_name, path)
    finally:
        if temporary_name and os.path.exists(temporary_name):
            os.unlink(temporary_name)


def main() -> int:
    source_fields, source_rows = read_csv(SOURCE)
    _, scope_rows = read_csv(SCOPE)
    source_by_id = {row["test_id"]: row for row in source_rows}
    if len(source_by_id) != len(source_rows):
        raise ValueError("source progress CSV contains duplicate test IDs")

    scoped_ids: list[str] = []
    for row in scope_rows:
        scoped_ids.extend(expand_selector(row["source_test_ids"]))
    duplicates = sorted({test_id for test_id in scoped_ids if scoped_ids.count(test_id) > 1})
    if duplicates:
        raise ValueError(f"chat/tool scope contains duplicate IDs: {duplicates}")
    missing = sorted(set(scoped_ids) - set(source_by_id))
    if missing:
        raise ValueError(f"chat/tool scope references IDs missing from source matrix: {missing}")

    scoped_rows = [source_by_id[test_id] for test_id in scoped_ids]
    scoped_rows.sort(key=lambda row: row["test_id"])
    write_atomic(OUTPUT, source_fields, scoped_rows)

    states = Counter(row["implementation_state"] for row in scoped_rows)
    evidence = Counter(row["evidence_level"] for row in scoped_rows)
    live = Counter(row["live_runtime_state"] for row in scoped_rows)
    visual = Counter(row["visual_pair_state"] for row in scoped_rows)
    paired = Counter(row["source_pair_state"] for row in scoped_rows)
    print(
        json.dumps(
            {
                "status": "ok",
                "scope": "chat_and_tool_functionality",
                "requirements": len(scoped_rows),
                "implementation_states": dict(sorted(states.items())),
                "requirements_with_mapped_evidence": sum(
                    count for level, count in evidence.items() if level != "not_reviewed"
                ),
                "evidence_levels": dict(sorted(evidence.items())),
                "live_runtime_states": dict(sorted(live.items())),
                "visual_pair_states": dict(sorted(visual.items(), reverse=True)),
                "source_pair_states": dict(sorted(paired.items())),
                "output": str(OUTPUT),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, KeyError, ValueError) as error:
        raise SystemExit(f"chat/tool progress generation failed: {error}") from error

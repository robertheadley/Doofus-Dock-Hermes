#!/usr/bin/env python3
"""Build a structural crosswalk without transferring status or evidence."""

from __future__ import annotations

import argparse
import csv
import json
import os
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
FIXED = ROOT / "docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12"
SUPPLEMENTAL = ROOT / "apps/hermes-native/docs/reference/Hermes_Upstream_UI_DoofusDock_Rust_Parity_2026-11-13"
DEFAULT_OUTPUT = ROOT / "apps/hermes-native/docs/status/REQUIREMENT_CROSSWALK.csv"
FIXED_HERMES_SOURCE = "8783054ef6cbafd86f10892241436862c1171a66"
FIXED_DOOFUS_SOURCE = "0278571393b0ae98fd016b67cfd32f66cee986ee"
SUPPLEMENTAL_SOURCE = "ffdc7eeb25343df1a7ee84503ba552538db9af72"

FIELDS = [
    "relationship_id",
    "baseline_package",
    "baseline_source_snapshot",
    "baseline_requirement_id",
    "baseline_capability_id",
    "baseline_test_id",
    "baseline_surface",
    "baseline_scope",
    "comparison_package",
    "comparison_source_snapshot",
    "comparison_requirement_id",
    "comparison_capability_id",
    "comparison_test_id",
    "match_basis",
    "relationship",
    "acceptance_text_equal",
    "semantic_review",
    "evidence_transfer",
    "disposition_note",
]


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open("r", encoding="utf-8-sig", newline="") as handle:
        return list(csv.DictReader(handle))


def index_unique(rows: list[dict[str, str]], key: str, label: str) -> dict[str, dict[str, str]]:
    result: dict[str, dict[str, str]] = {}
    for row in rows:
        value = row.get(key, "").strip()
        if not value:
            raise ValueError(f"{label} has an empty {key}")
        if value in result:
            raise ValueError(f"{label} has duplicate {key}: {value}")
        result[value] = row
    return result


def write_atomic(output: Path, rows: list[dict[str, str]]) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    fd, temp_name = tempfile.mkstemp(prefix=f".{output.name}.", suffix=".tmp", dir=output.parent)
    temp = Path(temp_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=FIELDS, lineterminator="\n")
            writer.writeheader()
            writer.writerows(rows)
            handle.flush()
            os.fsync(handle.fileno())

        roundtrip = read_csv(temp)
        if len(roundtrip) != len(rows) or any(set(row) != set(FIELDS) for row in roundtrip):
            raise ValueError("temporary crosswalk failed schema/count validation")
        ids = [row["relationship_id"] for row in roundtrip]
        if len(ids) != len(set(ids)):
            raise ValueError("temporary crosswalk has duplicate relationship IDs")
        os.replace(temp, output)
    except BaseException:
        temp.unlink(missing_ok=True)
        raise


def build(output: Path) -> dict[str, int]:
    fixed_dir = FIXED / "planning"
    supplemental_dir = SUPPLEMENTAL / "planning"
    fixed_req = read_csv(fixed_dir / "REQUIREMENTS_LEDGER.csv")
    fixed_acc = read_csv(fixed_dir / "ACCEPTANCE_MATRIX.csv")
    supplemental_req = read_csv(supplemental_dir / "REQUIREMENTS_LEDGER.csv")
    supplemental_acc = read_csv(supplemental_dir / "ACCEPTANCE_MATRIX.csv")

    fixed_by_cap = index_unique(fixed_req, "capability_id", "fixed requirements")
    supplemental_by_cap = index_unique(supplemental_req, "capability_id", "supplemental requirements")
    fixed_acc_by_cap = index_unique(fixed_acc, "capability_id", "fixed acceptance matrix")
    supplemental_acc_by_cap = index_unique(supplemental_acc, "capability_id", "supplemental acceptance matrix")
    if set(fixed_by_cap) != set(fixed_acc_by_cap):
        raise ValueError("fixed requirements and acceptance matrix do not join one-to-one")
    if set(supplemental_by_cap) != set(supplemental_acc_by_cap):
        raise ValueError("supplemental requirements and acceptance matrix do not join one-to-one")
    if {row.get("source_snapshot") for row in supplemental_acc} != {SUPPLEMENTAL_SOURCE}:
        raise ValueError("supplemental acceptance matrix source pin changed")

    deferred_doc: dict[str, Any] = json.loads(
        (SUPPLEMENTAL / "maps/DEFERRED_OUT_OF_SCOPE.json").read_text(encoding="utf-8")
    )
    deferred = index_unique(deferred_doc.get("deferred", []), "id", "supplemental deferred inventory")
    fixed_ids = set(fixed_by_cap)
    supplemental_ids = set(supplemental_by_cap)
    if fixed_ids - supplemental_ids != set(deferred):
        raise ValueError("deferred inventory must exactly match fixed-only capabilities")
    extras = supplemental_ids - fixed_ids
    if len(fixed_req) != 565 or len(supplemental_req) != 491 or len(deferred) != 108 or len(extras) != 34:
        raise ValueError(
            "inventory counts changed; review the fixed target and scope before regenerating "
            f"(fixed={len(fixed_req)}, supplemental={len(supplemental_req)}, "
            f"deferred={len(deferred)}, supplemental_only={len(extras)})"
        )

    rows: list[dict[str, str]] = []
    fixed_in_scope = 0
    fixed_excluded = 0
    fixed_shared = 0
    fixed_in_scope_deferred = 0
    for cap_id, baseline in fixed_by_cap.items():
        surface = baseline.get("surface", "")
        candidate = supplemental_by_cap.get(cap_id)
        baseline_case = fixed_acc_by_cap[cap_id]
        source_snapshot = baseline_case.get("source_snapshot", "")
        if source_snapshot == FIXED_HERMES_SOURCE:
            scope = "in_scope"
        elif source_snapshot == FIXED_DOOFUS_SOURCE:
            scope = "excluded"
        else:
            raise ValueError(f"unknown fixed-source pin for {cap_id}: {source_snapshot!r}")
        fixed_in_scope += scope == "in_scope"
        fixed_excluded += scope == "excluded"
        if candidate is not None:
            comparison_case = supplemental_acc_by_cap[cap_id]
            same_text = baseline_case.get("acceptance", "") == comparison_case.get("acceptance", "")
            relation = "exact_capability_overlap" if same_text else "exact_id_text_differs"
            match_basis = "exact_capability_id"
            comparison_values = {
                "comparison_requirement_id": candidate.get("requirement_id", ""),
                "comparison_capability_id": cap_id,
                "comparison_test_id": candidate.get("test_id", ""),
            }
            note = "Exact ID correspondence only; review semantics before any status/evidence transfer."
            fixed_shared += 1
        else:
            deferred_row = deferred[cap_id]
            comparison_values = {
                "comparison_requirement_id": "",
                "comparison_capability_id": "",
                "comparison_test_id": "",
            }
            relation = (
                "fixed_scope_omitted_from_comparison"
                if scope == "in_scope"
                else "fixed_out_of_scope"
            )
            match_basis = "none"
            same_text = False
            note = f"Deferred from comparison inventory: {deferred_row.get('defer_reason', 'reason not recorded')}"
            if scope == "in_scope":
                fixed_in_scope_deferred += 1

        rows.append(
            {
                "relationship_id": f"baseline:{cap_id}",
                "baseline_package": "Hermes_DoofusDock_Rust_Parity_2026-11-12",
                "baseline_source_snapshot": source_snapshot,
                "baseline_requirement_id": baseline.get("requirement_id", ""),
                "baseline_capability_id": cap_id,
                "baseline_test_id": baseline.get("test_id", ""),
                "baseline_surface": surface,
                "baseline_scope": scope,
                "comparison_package": "Hermes_Upstream_UI_DoofusDock_Rust_Parity_2026-11-13",
                "comparison_source_snapshot": SUPPLEMENTAL_SOURCE,
                **comparison_values,
                "match_basis": match_basis,
                "relationship": relation,
                "acceptance_text_equal": str(same_text).lower(),
                "semantic_review": "pending",
                "evidence_transfer": "prohibited",
                "disposition_note": note,
            }
        )

    for cap_id, comparison in supplemental_by_cap.items():
        if cap_id in fixed_by_cap:
            continue
        rows.append(
            {
                "relationship_id": f"comparison:{cap_id}",
                "baseline_package": "Hermes_DoofusDock_Rust_Parity_2026-11-12",
                "baseline_source_snapshot": "",
                "baseline_requirement_id": "",
                "baseline_capability_id": "",
                "baseline_test_id": "",
                "baseline_surface": "",
                "baseline_scope": "not_in_fixed_inventory",
                "comparison_package": "Hermes_Upstream_UI_DoofusDock_Rust_Parity_2026-11-13",
                "comparison_source_snapshot": SUPPLEMENTAL_SOURCE,
                "comparison_requirement_id": comparison.get("requirement_id", ""),
                "comparison_capability_id": cap_id,
                "comparison_test_id": comparison.get("test_id", ""),
                "match_basis": "none",
                "relationship": "comparison_only_addition",
                "acceptance_text_equal": "false",
                "semantic_review": "pending",
                "evidence_transfer": "prohibited",
                "disposition_note": "Supplemental-only requirement; not part of the fixed baseline denominator.",
            }
        )

    if fixed_in_scope != 499 or fixed_excluded != 66 or fixed_shared != 457 or fixed_in_scope_deferred != 42:
        raise ValueError(
            "scope/crosswalk counts differ from the recorded baseline; review before publication "
            f"(in_scope={fixed_in_scope}, excluded={fixed_excluded}, shared={fixed_shared}, "
            f"in_scope_deferred={fixed_in_scope_deferred})"
        )
    write_atomic(output, rows)
    return {
        "rows": len(rows),
        "fixed_in_scope": fixed_in_scope,
        "fixed_excluded": fixed_excluded,
        "exact_capability_overlaps": fixed_shared,
        "fixed_in_scope_deferred": fixed_in_scope_deferred,
        "supplemental_only_additions": len(extras),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    counts = build(args.output)
    print(json.dumps({"status": "ok", "output": str(args.output), **counts}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

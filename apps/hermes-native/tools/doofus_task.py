#!/usr/bin/env python3
"""Portable, repository-backed task handoff and evidence progress CLI."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import time
import uuid
from contextlib import contextmanager
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any


SCHEMA_VERSION = 1
STATES = {"planned", "in_progress", "implemented", "tested", "verified", "accepted", "blocked", "failed", "unknown"}
PROVIDERS = {"codex", "antigravity", "hermes"}
EVENT_LIMIT = 16 * 1024 * 1024
LEASE_MINUTES = 60
ETA_ALPHA = 0.2
ETA_MIN_SAMPLES_PER_PHASE = 3


class TaskError(RuntimeError):
    pass


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def git(root: Path, *args: str, check: bool = True, preserve_whitespace: bool = False) -> str:
    result = subprocess.run(["git", *args], cwd=root, text=True, encoding="utf-8", errors="replace", capture_output=True)
    if check and result.returncode:
        raise TaskError(f"git {' '.join(args)} failed ({result.returncode}): {result.stderr.strip()}")
    return result.stdout.rstrip("\r\n") if preserve_whitespace else result.stdout.strip()


def repo_root(start: Path | None = None) -> Path:
    base = (start or Path.cwd()).resolve()
    value = git(base, "rev-parse", "--show-toplevel")
    return Path(value).resolve()


def safe_remote(value: str) -> str:
    value = value.strip()
    if value.startswith(("https://", "http://")):
        from urllib.parse import urlsplit, urlunsplit

        parsed = urlsplit(value)
        if parsed.username or parsed.password:
            raise TaskError("origin URL contains embedded credentials; refusing to persist it")
        return urlunsplit((parsed.scheme, parsed.netloc, parsed.path, "", ""))
    if re.match(r"^[^/@:]+@[^/:]+:.+$", value):
        return value
    raise TaskError("origin URL format is unsupported or unsafe to persist")


def _status_rows(root: Path) -> list[dict[str, str]]:
    raw = git(
        root,
        "-c",
        "core.quotepath=false",
        "status",
        "--porcelain=v1",
        "--untracked-files=all",
        preserve_whitespace=True,
    )
    rows: list[dict[str, str]] = []
    for line in raw.splitlines():
        if len(line) < 4:
            continue
        status, path = line[:2], line[3:]
        if " -> " in path:
            path = path.rsplit(" -> ", 1)[1]
        path = path.replace("\\", "/")
        if path.startswith("/") or ".." in Path(path).parts:
            raise TaskError("git returned an unsafe non-relative changed path")
        rows.append({"status": status, "path": path})
    return rows


def git_snapshot(root: Path) -> dict[str, Any]:
    remote_value = git(root, "remote", "get-url", "origin", check=False)
    remote = safe_remote(remote_value) if remote_value else None
    branch = git(root, "branch", "--show-current") or "detached"
    head = git(root, "rev-parse", "HEAD")
    git_dir = Path(git(root, "rev-parse", "--git-dir"))
    if not git_dir.is_absolute():
        git_dir = (root / git_dir).resolve()
    worktree_id = hashlib.sha256(f"{remote or 'no-origin'}\0{git_dir.as_posix()}".encode()).hexdigest()[:24]
    changes = _status_rows(root)

    digest = hashlib.sha256()
    digest.update(f"{branch}\0{head}\0".encode())
    diff = git(root, "diff", "--binary", "HEAD", "--", ".", ":!.doofus/**")
    digest.update(diff.encode("utf-8", errors="replace"))
    for item in changes:
        if item["path"].startswith(".doofus/"):
            continue
        digest.update(f"{item['status']}\0{item['path']}\0".encode())
        if item["status"] == "??" and not item["path"].startswith(".doofus/"):
            path = root / Path(item["path"])
            if path.is_file():
                size = path.stat().st_size
                digest.update(f"{size}\0".encode())
                if size <= 128 * 1024 * 1024:
                    with path.open("rb") as stream:
                        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                            digest.update(chunk)
                else:
                    digest.update(b"content-hash-skipped-over-128MiB")
    return {
        "repository_identity": remote,
        "branch": branch,
        "head_commit": head,
        "worktree_id": worktree_id,
        "changed_files": changes,
        "change_fingerprint": digest.hexdigest(),
    }


def validate_json_schema(value: Any, schema: dict[str, Any], path: str = "$" ) -> None:
    expected = schema.get("type")
    valid = {
        "object": lambda x: isinstance(x, dict),
        "array": lambda x: isinstance(x, list),
        "string": lambda x: isinstance(x, str),
        "integer": lambda x: isinstance(x, int) and not isinstance(x, bool),
        "number": lambda x: isinstance(x, (int, float)) and not isinstance(x, bool),
        "boolean": lambda x: isinstance(x, bool),
        "null": lambda x: x is None,
    }
    if isinstance(expected, list):
        matched = next((option for option in expected if option in valid and valid[option](value)), None)
        if matched is None:
            raise TaskError(f"schema violation at {path}: unexpected value type")
        expected = matched
    elif expected in valid and not valid[expected](value):
        raise TaskError(f"schema violation at {path}: expected {expected}")
    if "enum" in schema and value not in schema["enum"]:
        raise TaskError(f"schema violation at {path}: value is outside enum")
    if "const" in schema and value != schema["const"]:
        raise TaskError(f"schema violation at {path}: value does not match const")
    if isinstance(expected, str) and expected in {"integer", "number"} and "minimum" in schema and value < schema["minimum"]:
        raise TaskError(f"schema violation at {path}: number is below minimum")
    if expected == "object":
        required = schema.get("required", [])
        missing = [key for key in required if key not in value]
        if missing:
            raise TaskError(f"schema violation at {path}: missing {', '.join(missing)}")
        properties = schema.get("properties", {})
        if schema.get("additionalProperties") is False:
            extras = set(value) - set(properties)
            if extras:
                raise TaskError(f"schema violation at {path}: unexpected {', '.join(sorted(extras))}")
        for key, child in properties.items():
            if key in value:
                validate_json_schema(value[key], child, f"{path}.{key}")
    if expected == "array":
        if len(value) < schema.get("minItems", 0) or len(value) > schema.get("maxItems", 2**31):
            raise TaskError(f"schema violation at {path}: array length outside bounds")
        for index, item in enumerate(value):
            validate_json_schema(item, schema.get("items", {}), f"{path}[{index}]")
    if expected == "string":
        if len(value) < schema.get("minLength", 0) or len(value) > schema.get("maxLength", 2**31):
            raise TaskError(f"schema violation at {path}: string length outside bounds")


def validate_document(root: Path, kind: str, value: Any) -> None:
    schema_path = root / ".doofus" / "schemas" / f"{kind}.schema.json"
    try:
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise TaskError(f"cannot load {kind} schema at .doofus/schemas: {exc}") from exc
    validate_json_schema(value, schema)
    if value.get("schema_version") != SCHEMA_VERSION:
        raise TaskError(f"unsupported {kind} schema version: {value.get('schema_version')}")
    if kind == "handoff":
        process_ids: set[str] = set()
        process_fields = {"process_id", "kind", "provider", "model", "pid", "state", "observed_at", "note"}
        for item in value.get("active_processes", []):
            if set(item) != process_fields:
                raise TaskError("active process record has missing or unexpected fields")
            if not re.fullmatch(r"[A-Za-z0-9_.:-]{1,128}", item["process_id"]) or not re.fullmatch(r"[A-Za-z0-9_.:-]{1,128}", item["kind"]):
                raise TaskError("active process record has an invalid stable ID or kind")
            if item["provider"] not in PROVIDERS or item["state"] not in {"running", "stopped", "unknown"}:
                raise TaskError("active process record has an invalid provider or state")
            if any(ord(ch) < 32 for ch in item["note"]):
                raise TaskError("active process note contains a control character")
            if item["process_id"] in process_ids:
                raise TaskError(f"duplicate active process ID: {item['process_id']}")
            process_ids.add(item["process_id"])
        operation_ids: set[str] = set()
        operation_fields = {"operation_id", "kind", "owner", "state", "observed_at", "note", "evidence"}
        for item in value.get("external_operations", []):
            if set(item) == {"operation", "state", "verified_at"} and item["state"] == "not_running_at_bootstrap":
                continue
            if set(item) != operation_fields:
                raise TaskError("external operation record has missing or unexpected fields")
            if not re.fullmatch(r"[A-Za-z0-9_.:-]{1,128}", item["operation_id"]) or not re.fullmatch(r"[A-Za-z0-9_.:-]{1,128}", item["kind"]):
                raise TaskError("external operation has an invalid stable ID or kind")
            if item["owner"]["provider"] not in PROVIDERS or item["state"] not in {"running", "succeeded", "failed", "unknown"}:
                raise TaskError("external operation has an invalid owner or state")
            if any(ord(ch) < 32 for ch in item["note"]):
                raise TaskError("external operation note contains a control character")
            if item["operation_id"] in operation_ids:
                raise TaskError(f"duplicate external operation ID: {item['operation_id']}")
            operation_ids.add(item["operation_id"])
    if kind == "task-ledger":
        seen: set[str] = set()
        flattened: list[dict[str, Any]] = []

        def collect(task: dict[str, Any], parent_id: str | None) -> None:
            task_id = task.get("task_id")
            if not isinstance(task_id, str) or not task_id or task_id in seen:
                raise TaskError("task ledger has an empty or duplicate task ID")
            if task.get("parent_task_id") != parent_id:
                raise TaskError(f"task {task_id} has an invalid parent reference")
            if task.get("state") not in STATES:
                raise TaskError(f"task {task_id} has an invalid state")
            seen.add(task_id)
            flattened.append(task)
            for child in task.get("children", []):
                collect(child, task_id)

        for task in value.get("tasks", []):
            collect(task, None)
        for task in flattened:
            missing = set(task.get("dependencies", [])) - seen
            if missing:
                raise TaskError(f"task {task['task_id']} references unknown dependencies: {sorted(missing)}")
        parity = next((task for task in flattened if task["task_id"] == "hermes-native-ui-parity"), None)
        csv_path = root / "apps" / "hermes-native" / "docs" / "status" / "IMPLEMENTATION_PROGRESS.csv"
        if parity is None or not csv_path.is_file():
            raise TaskError("task ledger lacks the pinned Hermes parity root or implementation CSV")
        with csv_path.open(encoding="utf-8-sig", newline="") as stream:
            expected_ids = {f"hermes-{row['test_id'].lower()}" for row in csv.DictReader(stream)}
        actual_ids = {task["task_id"] for task in parity.get("children", [])}
        if len(expected_ids) != 499 or actual_ids != expected_ids:
            raise TaskError("task ledger requirement leaves do not match the fixed 499-row PAR inventory")


def atomic_json(root: Path, path: Path, value: Any, kind: str) -> None:
    validate_document(root, kind, value)
    path.parent.mkdir(parents=True, exist_ok=True)
    temp_name: str | None = None
    try:
        with tempfile.NamedTemporaryFile("w", encoding="utf-8", newline="\n", dir=path.parent, prefix=f".{path.name}.", suffix=".tmp", delete=False) as stream:
            temp_name = stream.name
            json.dump(value, stream, ensure_ascii=False, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        parsed = json.loads(Path(temp_name).read_text(encoding="utf-8"))
        validate_document(root, kind, parsed)
        os.replace(temp_name, path)
        temp_name = None
    finally:
        if temp_name and Path(temp_name).exists():
            Path(temp_name).unlink()


def read_json(root: Path, path: Path, kind: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise TaskError(f"cannot read {path.relative_to(root)}: {exc}") from exc
    validate_document(root, kind, value)
    return value


def validate_evidence_refs(root: Path, refs: list[str]) -> None:
    for ref in refs:
        if "#" not in ref:
            raise TaskError(f"evidence reference must use relative-path#locator syntax: {ref}")
        relative, locator = ref.split("#", 1)
        path = Path(relative)
        if path.is_absolute() or ".." in path.parts or not locator:
            raise TaskError(f"evidence reference is unsafe or incomplete: {ref}")
        target = root / path
        if not target.is_file():
            raise TaskError(f"evidence file is missing: {relative}")
        try:
            content = target.read_text(encoding="utf-8", errors="replace")
        except OSError as exc:
            raise TaskError(f"cannot read evidence file {relative}: {exc}") from exc
        if locator not in content:
            raise TaskError(f"evidence locator not found in {relative}: {locator}")


def event_path(root: Path) -> Path:
    return root / ".doofus" / "events.jsonl"


def read_events(root: Path) -> list[dict[str, Any]]:
    path = event_path(root)
    if not path.exists():
        return []
    raw = path.read_bytes()
    if len(raw) > EVENT_LIMIT:
        raise TaskError("events.jsonl exceeds 16 MiB; archive it through an audited rotation before writing")
    result = []
    for number, line in enumerate(raw.splitlines(), 1):
        try:
            item = json.loads(line)
        except json.JSONDecodeError as exc:
            raise TaskError(f"events.jsonl line {number} is corrupt: {exc}") from exc
        validate_document(root, "event", item)
        result.append(item)
    return result


def write_events(root: Path, events: list[dict[str, Any]]) -> None:
    path = event_path(root)
    payload = "".join(json.dumps(item, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n" for item in events)
    if len(payload.encode("utf-8")) > EVENT_LIMIT:
        raise TaskError("events.jsonl would exceed 16 MiB; checkpoint refused")
    temp_name: str | None = None
    try:
        with tempfile.NamedTemporaryFile("w", encoding="utf-8", newline="\n", dir=path.parent, prefix=".events.", suffix=".tmp", delete=False) as stream:
            temp_name = stream.name
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
        for item in read_events_from_temp(Path(temp_name), root):
            validate_document(root, "event", item)
        os.replace(temp_name, path)
        temp_name = None
    finally:
        if temp_name and Path(temp_name).exists():
            Path(temp_name).unlink()


def read_events_from_temp(path: Path, root: Path) -> list[dict[str, Any]]:
    parsed = []
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        try:
            parsed.append(json.loads(line))
        except json.JSONDecodeError as exc:
            raise TaskError(f"temporary event log line {number} is invalid: {exc}") from exc
    return parsed


def make_event(event_id: str, kind: str, phase: str, revision: int, details: dict[str, Any]) -> dict[str, Any]:
    return {"schema_version": SCHEMA_VERSION, "event_id": event_id, "event_type": kind, "phase": phase, "revision": revision, "timestamp": utc_now(), "provider": details.get("provider"), "model": details.get("model"), "task_id": details.get("task_id"), "details": details}


def append_event_idempotent(root: Path, event: dict[str, Any]) -> None:
    events = read_events(root)
    prior = next((item for item in events if item["event_id"] == event["event_id"] and item["phase"] == event["phase"]), None)
    if prior is not None:
        compare = {key: value for key, value in event.items() if key != "timestamp"}
        existing = {key: value for key, value in prior.items() if key != "timestamp"}
        if compare != existing:
            raise TaskError("request ID was already used with different event contents")
        return
    write_events(root, events + [event])


@contextmanager
def mutation_lock(root: Path, request_id: str):
    lock = root / ".doofus" / ".mutation.lock"
    lock.parent.mkdir(parents=True, exist_ok=True)
    try:
        lock.mkdir()
    except FileExistsError as exc:
        owner_file = lock / "owner.json"
        try:
            owner = json.loads(owner_file.read_text(encoding="utf-8"))
        except Exception:
            owner = {"state": "unknown"}
        raise TaskError(f"checkpoint lock exists ({owner}); inspect or recover it explicitly before retrying") from exc
    try:
        owner = {"request_id": request_id, "pid": os.getpid(), "created_at": utc_now(), "expires_at": (datetime.now(timezone.utc) + timedelta(minutes=5)).replace(microsecond=0).isoformat().replace("+00:00", "Z")}
        (lock / "owner.json").write_text(json.dumps(owner, sort_keys=True), encoding="utf-8")
        yield
    finally:
        try:
            (lock / "owner.json").unlink(missing_ok=True)
            lock.rmdir()
        except OSError:
            pass


def load_pair(root: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    base = root / ".doofus"
    handoff = read_json(root, base / "handoff.json", "handoff")
    ledger = read_json(root, base / "task-ledger.json", "task-ledger")
    if handoff["task_ledger_revision"] != ledger["revision"]:
        raise TaskError(f"task ledger revision mismatch: handoff expects {handoff['task_ledger_revision']}, ledger is {ledger['revision']}; reconcile before writing")
    return handoff, ledger


def requirement_tasks(root: Path) -> list[dict[str, Any]]:
    csv_path = root / "apps" / "hermes-native" / "docs" / "status" / "IMPLEMENTATION_PROGRESS.csv"
    if not csv_path.is_file():
        raise TaskError("the generated fixed-scope implementation progress CSV is missing")
    with csv_path.open(encoding="utf-8-sig", newline="") as stream:
        rows = list(csv.DictReader(stream))
    if len(rows) != 499 or len({row.get("test_id") for row in rows}) != 499:
        raise TaskError(f"expected exactly 499 unique pinned PAR rows; found {len(rows)}")
    result = []
    for row in rows:
        implementation = row["implementation_state"]
        state = {"partial": "in_progress", "implemented": "implemented", "not_implemented": "planned", "unknown": "unknown"}.get(implementation)
        if state is None:
            raise TaskError(f"unknown implementation state for {row['test_id']}: {implementation}")
        if row["source_pair_state"] == "accepted":
            state = "accepted"
        result.append({
            "task_id": f"hermes-{row['test_id'].lower()}", "parent_task_id": "hermes-native-ui-parity",
            "agent_identity": {"provider": None, "model": None}, "title": f"{row['test_id']} {row['scenario']}",
            "objective": row["capability_id"], "state": state, "owner": None, "lease_id": None,
            "dependencies": [], "inputs": {"source_snapshot": row["source_snapshot"], "phase": row["phase"], "priority": row["priority"]},
            "outputs": [], "verification_evidence": json.loads(row["evidence_refs_json"]),
            "progress": {"implementation_state": implementation, "evidence_level": row["evidence_level"], "source_pair_state": row["source_pair_state"], "visual_pair_state": row["visual_pair_state"], "live_runtime_state": row["live_runtime_state"]},
            "estimated_remaining_effort": None, "size_units": None, "started_at": None, "last_updated_at": row["reviewed_on"] + "T00:00:00Z" if row["reviewed_on"] else None,
        })
    return result


def build_task_ledger(root: Path, revision: int = 1) -> dict[str, Any]:
    parity_children = requirement_tasks(root)
    parity = {"task_id": "hermes-native-ui-parity", "parent_task_id": "hermes-native-project", "agent_identity": {"provider": "codex", "model": None}, "title": "Hermes Native 1:1 UI parity", "objective": "Complete the fixed 499-row UI replacement scope for pinned source 8783054ef6cbafd86f10892241436862c1171a66.", "state": "in_progress", "owner": "codex", "lease_id": None, "dependencies": [], "inputs": {"acceptance_matrix": "docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/planning/ACCEPTANCE_MATRIX.csv"}, "outputs": [], "verification_evidence": [], "progress": {"children": len(parity_children)}, "estimated_remaining_effort": None, "size_units": None, "started_at": None, "last_updated_at": utc_now(), "children": parity_children}
    handoff_task = {"task_id": "hermes-cross-provider-handoff", "parent_task_id": "hermes-native-project", "agent_identity": {"provider": "codex", "model": None}, "title": "Provider-independent checkpoint and handoff", "objective": "Build durable, auditable task continuity and evidence-based progress reporting.", "state": "in_progress", "owner": "codex", "lease_id": None, "dependencies": [], "inputs": {}, "outputs": [], "verification_evidence": [], "progress": {"phase": "phase-a"}, "estimated_remaining_effort": None, "size_units": None, "started_at": None, "last_updated_at": utc_now()}
    project = {"task_id": "hermes-native-project", "parent_task_id": None, "agent_identity": {"provider": "codex", "model": None}, "title": "Doofus Dock Hermes", "objective": "Deliver a verified Rust/egui/DoofusDock replacement for the pinned Hermes Desktop source.", "state": "in_progress", "owner": "codex", "lease_id": None, "dependencies": [], "inputs": {"repository": "https://github.com/robertheadley/Doofus-Dock-Hermes.git", "source_snapshot": "8783054ef6cbafd86f10892241436862c1171a66", "in_scope_requirements": 499, "excluded_research_requirements": 66}, "outputs": [], "verification_evidence": [], "progress": {"parity_scope": 499, "source_acceptance": "pending"}, "estimated_remaining_effort": None, "size_units": None, "started_at": None, "last_updated_at": utc_now(), "children": [parity, handoff_task]}
    return {"schema_version": SCHEMA_VERSION, "revision": revision, "updated_at": utc_now(), "tasks": [project]}


def new_request(args: argparse.Namespace) -> str:
    return args.request_id or str(uuid.uuid4())


def initialize(root: Path, args: argparse.Namespace) -> dict[str, Any]:
    base = root / ".doofus"
    if (base / "handoff.json").exists():
        existing = load_handoff(root)
        if args.request_id and existing.get("last_operation", {}).get("request_id") == args.request_id and existing.get("last_operation", {}).get("type") == "bootstrap":
            ledger_path = base / "task-ledger.json"
            if not ledger_path.exists():
                atomic_json(root, ledger_path, build_task_ledger(root, existing["task_ledger_revision"]), "task-ledger")
            ledger = read_json(root, ledger_path, "task-ledger")
            if ledger["revision"] != existing["task_ledger_revision"]:
                raise TaskError("bootstrap task ledger differs from checkpoint; manual reconciliation is required")
            if not event_path(root).exists():
                write_events(root, [make_event(args.request_id, "task.initialized", "committed", existing["revision"], {"provider": existing["previous_agent"]["provider"], "model": existing["previous_agent"]["model"], "task_id": existing["task_id"], "source_snapshot": "8783054ef6cbafd86f10892241436862c1171a66", "in_scope_requirements": 499})])
            validate_document(root, "task-ledger", ledger)
            read_events(root)
            return existing
        raise TaskError("handoff state already exists; refusing to overwrite it")
    if (base / "task-ledger.json").exists() or (base / "events.jsonl").exists():
        raise TaskError("partial handoff state exists without a canonical manifest; inspect and reconcile it before retrying init")
    if args.provider not in PROVIDERS:
        raise TaskError("provider must be codex, antigravity, or hermes")
    snapshot = git_snapshot(root)
    request_id = new_request(args)
    lease = str(uuid.uuid4())
    now = utc_now()
    ledger = build_task_ledger(root)
    for task in flatten(ledger["tasks"][0]):
        if task["task_id"] in {"hermes-native-project", "hermes-native-ui-parity", "hermes-cross-provider-handoff"}:
            task["agent_identity"]["provider"] = args.provider
            task["owner"] = args.provider
            task["lease_id"] = lease
    handoff = {
        "schema_version": SCHEMA_VERSION, "revision": 1, "state_revision": 1, "created_at": now, "updated_at": now,
        "project_id": "doofus-dock-hermes", "repository": snapshot,
        "task_id": "hermes-cross-provider-handoff", "parent_task_id": "hermes-native-project", "scope_task_id": "hermes-native-project",
        "task_description": "Continue the pinned Hermes Desktop 1:1 Rust/DoofusDock UI migration; finish and validate provider-independent task continuity, then resume parity work.",
        "approved_plan_revision": 173, "authorization": {"source": "direct user request in this Codex task", "scope": "pinned Hermes Native parity plus portable checkpoint/handoff foundation", "status": "authorized"},
        "phase": "phase-a-handoff-foundation", "checkpoint": {"state": "in_progress", "verified_at": now, "summary": "Repository and existing progress ledger inspected; no prior .doofus handoff system found.", "command": None, "evidence": []},
        "completed_subtasks": [], "remaining_subtasks": [task["task_id"] for task in flatten(ledger["tasks"][0]) if task["state"] not in {"tested", "verified", "accepted"}],
        "changed_files": [{"path": item["path"], "status": item["status"], "owner": "unattributed_preexisting", "change_type": "preexisting_at_handoff_bootstrap"} for item in snapshot["changed_files"]],
        "tests_executed": [], "known_failures": [], "blockers": [], "unresolved_questions": [],
        "pending_user_approvals": [], "active_processes": [], "external_operations": [{"operation": "headless_rust_validation", "state": "not_running_at_bootstrap", "verified_at": now}],
        "last_operation": {"request_id": request_id, "state": "committed", "type": "bootstrap"}, "uncertain_operations": [],
        "recommended_next_action": "Finish the planned portable handoff CLI and run its provider-simulation and crash-safety tests.",
        "previous_agent": {"provider": "codex", "model": None},
        "current_owner": {"provider": args.provider, "model": args.model, "lease_id": lease, "scope_task_id": "hermes-native-project", "lease_expires_at": (datetime.now(timezone.utc) + timedelta(minutes=LEASE_MINUTES)).replace(microsecond=0).isoformat().replace("+00:00", "Z"), "ownership_state": "active"},
        "intended_recipient": None, "task_ledger_revision": ledger["revision"], "last_event_id": request_id,
        "phase_effort_samples": [], "legacy_checklist_measurement": {"completed": 33, "total": 59, "source": "user-supplied supplement; original measurement file not found in the repository", "comparable_to_parity": False},
    }
    validate_document(root, "task-ledger", ledger)
    validate_document(root, "handoff", handoff)
    base.mkdir(parents=True, exist_ok=True)
    atomic_json(root, base / "handoff.json", handoff, "handoff")
    atomic_json(root, base / "task-ledger.json", ledger, "task-ledger")
    event = make_event(request_id, "task.initialized", "committed", 1, {"provider": args.provider, "model": args.model, "task_id": handoff["task_id"], "source_snapshot": "8783054ef6cbafd86f10892241436862c1171a66", "in_scope_requirements": 499})
    write_events(root, [event])
    return handoff


def load_handoff(root: Path) -> dict[str, Any]:
    return read_json(root, root / ".doofus" / "handoff.json", "handoff")


def record_operation(root: Path, handoff: dict[str, Any], request_id: str, kind: str, provider: str, model: str | None, details: dict[str, Any], mutate) -> dict[str, Any]:
    with mutation_lock(root, request_id):
        current = load_handoff(root)
        prior = [event for event in read_events(root) if event["event_id"] == request_id and event["event_type"] == kind]
        committed = next((event for event in reversed(prior) if event["phase"] == "committed"), None)
        if committed:
            expected = {"provider": provider, "model": model, "task_id": current["task_id"], **details}
            if committed["provider"] != provider or committed["details"] != expected:
                raise TaskError("request ID was already committed with different operation contents")
            if current.get("last_operation", {}).get("request_id") != request_id:
                raise TaskError("request was already applied and state has advanced; inspect status instead of replaying it")
            return current
        if prior:
            raise TaskError(f"request {request_id} has an uncertain prepared operation; reconcile it before retrying")
        if current["revision"] != handoff["revision"]:
            raise TaskError(f"stale state revision {handoff['revision']}; current revision is {current['revision']}")
        target_revision = current["revision"] + 1
        prepared = make_event(request_id, kind, "prepared", target_revision, {"provider": provider, "model": model, "task_id": current["task_id"], **details})
        append_event_idempotent(root, prepared)
        mutate(current, target_revision)
        current["revision"] = target_revision
        current["state_revision"] = target_revision
        current["updated_at"] = utc_now()
        current["last_event_id"] = request_id
        current["last_operation"] = {"request_id": request_id, "state": "committed", "type": kind}
        atomic_json(root, root / ".doofus" / "handoff.json", current, "handoff")
        done = make_event(request_id, kind, "committed", target_revision, {"provider": provider, "model": model, "task_id": current["task_id"], **details})
        append_event_idempotent(root, done)
        return current


def require_owner(handoff: dict[str, Any], provider: str, lease_id: str) -> None:
    owner = handoff["current_owner"]
    if not owner or owner["ownership_state"] != "active":
        raise TaskError("there is no active owner; resume the task before writing")
    if owner["provider"] != provider or owner["lease_id"] != lease_id:
        raise TaskError("owner/provider lease mismatch; refusing a concurrent or stale write")


def unresolved_operations(root: Path) -> list[str]:
    events = read_events(root)
    prepared = {(event["event_id"], event["event_type"]) for event in events if event["phase"] == "prepared"}
    terminal = {(event["event_id"], event["event_type"]) for event in events if event["phase"] in {"committed", "reconciled"}}
    return [event_id for event_id, kind in sorted(prepared) if (event_id, kind) not in terminal]


def status_report(root: Path) -> dict[str, Any]:
    handoff = load_handoff(root)
    ledger = read_json(root, root / ".doofus" / "task-ledger.json", "task-ledger")
    actual = git_snapshot(root)
    drift = actual["change_fingerprint"] != handoff["repository"]["change_fingerprint"]
    mismatch = handoff["task_ledger_revision"] != ledger["revision"]
    owners = {item["path"]: item["owner"] for item in handoff["changed_files"]}
    changed = [{**item, "owner": owners.get(item["path"], "unattributed_since_checkpoint")} for item in actual["changed_files"]]
    process_records = handoff.get("active_processes", [])
    external_records = handoff.get("external_operations", [])
    return {"project_id": handoff["project_id"], "task_id": handoff["task_id"], "parent_task_id": handoff["parent_task_id"], "scope_task_id": handoff["scope_task_id"], "revision": handoff["revision"], "phase": handoff["phase"], "owner": handoff["current_owner"], "intended_recipient": handoff["intended_recipient"], "git_drift": drift, "expected_head": handoff["repository"]["head_commit"], "actual_head": actual["head_commit"], "changed_files": changed, "unresolved_operations": unresolved_operations(root), "active_processes": process_records, "external_operations": external_records, "processes_needing_reconciliation": [item["process_id"] for item in process_records if item["state"] in {"running", "unknown"}], "external_operations_needing_reconciliation": [item["operation_id"] for item in external_records if item.get("operation_id") and item["state"] in {"running", "unknown"}], "task_ledger_revision": ledger["revision"], "state_consistency": "reconcile_required" if mismatch else "consistent", "recommended_next_action": handoff["recommended_next_action"], "last_checkpoint": handoff["checkpoint"]}


def progress_report(root: Path) -> dict[str, Any]:
    handoff, ledger = load_pair(root)
    csv_path = root / "apps" / "hermes-native" / "docs" / "status" / "CHAT_TOOL_PROGRESS.csv"
    with csv_path.open(encoding="utf-8-sig", newline="") as stream:
        rows = list(csv.DictReader(stream))
    if len(rows) != 176 or len({row.get("test_id") for row in rows}) != 176:
        raise TaskError(f"expected 176 unique agent-workflow progress rows, found {len(rows)}")
    implementation: dict[str, int] = {}
    acceptance: dict[str, int] = {}
    live: dict[str, int] = {}
    headless_evidence = 0
    for row in rows:
        implementation[row["implementation_state"]] = implementation.get(row["implementation_state"], 0) + 1
        acceptance[row["source_pair_state"]] = acceptance.get(row["source_pair_state"], 0) + 1
        live[row["live_runtime_state"]] = live.get(row["live_runtime_state"], 0) + 1
        if row["evidence_level"] not in {"", "not_reviewed", "none"}:
            headless_evidence += 1

    scoped_task_ids = {f"hermes-{row['test_id'].lower()}" for row in rows}
    all_tasks = [task for root_task in ledger["tasks"] for task in flatten(root_task)]
    tasks_by_id = {task["task_id"]: task for task in all_tasks}
    missing_tasks = sorted(scoped_task_ids - tasks_by_id.keys())
    if missing_tasks:
        raise TaskError(f"task ledger is missing agent-workflow rows: {missing_tasks[:8]}")
    tasks = [tasks_by_id[task_id] for task_id in sorted(scoped_task_ids)]
    task_states: dict[str, int] = {}
    for task in tasks:
        task_states[task["state"]] = task_states.get(task["state"], 0) + 1
    events = read_events(root)
    tests = handoff.get("tests_executed", [])
    test_outcomes = {"passed": 0, "failed": 0, "other": 0}
    for event in tests:
        result = str(event.get("result", "")).lower()
        key = "passed" if result.startswith("pass") else "failed" if result.startswith("fail") else "other"
        test_outcomes[key] += 1
    phase_samples = handoff.get("phase_effort_samples", [])
    eta_by_phase: dict[str, Any] = {}
    for sample_phase in sorted({sample["phase"] for sample in phase_samples}):
        samples = [sample for sample in phase_samples if sample["phase"] == sample_phase and sample.get("size_units") and sample.get("duration_minutes")]
        remaining_tasks = [
            item for item in tasks
            if (item.get("phase") or item.get("inputs", {}).get("phase")) == sample_phase
            and not (
                item.get("progress", {}).get("implementation_state") in {"implemented", "verified"}
                and item.get("progress", {}).get("live_runtime_state") == "verified"
                and item.get("progress", {}).get("source_pair_state") == "accepted"
            )
        ]
        base = {
            "eta_minutes": None,
            "samples": len(samples),
            "ewma_alpha": ETA_ALPHA,
            "remaining_tasks": len(remaining_tasks),
            "confidence": "unavailable",
            "uncertainty": "throughput samples or remaining task size estimates are insufficient",
        }
        if len(samples) < ETA_MIN_SAMPLES_PER_PHASE:
            eta_by_phase[sample_phase] = {**base, "status": "unavailable_insufficient_evidence"}
            continue
        if not remaining_tasks:
            eta_by_phase[sample_phase] = {**base, "eta_minutes": 0.0, "status": "no_remaining_tasks", "confidence": "high", "uncertainty": "no remaining tasks are recorded for this phase"}
            continue
        missing_size_ids = [item["task_id"] for item in remaining_tasks if not item.get("size_units") or item["size_units"] <= 0]
        if missing_size_ids:
            eta_by_phase[sample_phase] = {**base, "status": "unavailable_remaining_size_unknown", "tasks_missing_size": missing_size_ids}
            continue
        rate = samples[0]["duration_minutes"] / samples[0]["size_units"]
        for sample in samples[1:]:
            rate = ETA_ALPHA * (sample["duration_minutes"] / sample["size_units"]) + (1 - ETA_ALPHA) * rate
        remaining_size = sum(item["size_units"] for item in remaining_tasks)
        eta_by_phase[sample_phase] = {
            **base,
            "eta_minutes": round(rate * remaining_size, 1),
            "status": "estimated",
            "confidence": "limited",
            "uncertainty": "EWMA is based on observed phase throughput; duration variance and external acceptance work are not calibrated",
        }
    meaningful = [event for event in events if event["event_type"] in {"task.checkpointed", "task.state_updated", "test.run"}]
    meaningful_at = meaningful[-1]["timestamp"] if meaningful else handoff["created_at"]
    return {
        "schema_version": 1, "generated_at": utc_now(), "project_id": handoff["project_id"], "state_revision": handoff["revision"],
        "pinned_source_snapshot": "8783054ef6cbafd86f10892241436862c1171a66", "agent_workflow_scope": {"name": "chat_and_tool_functionality", "total": 176},
        "tasks": {"total": len(tasks), "states": task_states, "tested_or_accepted": sum(task_states.get(state, 0) for state in ("tested", "verified", "accepted")), "active": task_states.get("in_progress", 0), "blocked": task_states.get("blocked", 0)},
        "implementation": {"states": implementation, "reviewed": 176 - implementation.get("unknown", 0), "review_coverage_percent": round((176 - implementation.get("unknown", 0)) * 100 / 176, 1)},
        "automated_evidence": {"requirements_with_mapped_test_or_headless_evidence": headless_evidence, "test_runs_recorded": len(tests), "test_run_outcomes": test_outcomes, "latest_test_run": tests[-1] if tests else None},
        "live_runtime": live, "source_pair_acceptance": acceptance,
        "critical_path": {"status": "not_mapped_from_authoritative_dependency_data", "tasks": []},
        "elapsed_execution_seconds": None, "protocol_elapsed_seconds": max(0, int((datetime.now(timezone.utc) - datetime.fromisoformat(handoff["created_at"].replace("Z", "+00:00"))).total_seconds())), "elapsed_status": "project-wide task start time was not recorded; protocol age is reported separately",
        "estimated_remaining_effort": eta_by_phase or {
            "status": "unavailable_insufficient_phase_samples",
            "eta_minutes": None,
            "ewma_alpha": ETA_ALPHA,
            "minimum_samples_per_phase": ETA_MIN_SAMPLES_PER_PHASE,
            "confidence": "unavailable",
            "uncertainty": "no phase has enough measured, size-normalized throughput samples",
        },
        "owner": handoff["current_owner"], "last_meaningful_progress_at": meaningful_at, "recommended_next_action": handoff["recommended_next_action"],
    }


def flatten(task: dict[str, Any]):
    yield task
    for child in task.get("children", []):
        yield from flatten(child)


def resume(root: Path, args: argparse.Namespace) -> dict[str, Any]:
    if args.provider not in PROVIDERS:
        raise TaskError("provider must be codex, antigravity, or hermes")
    handoff, _ledger = load_pair(root)
    if handoff["authorization"]["status"] != "authorized":
        raise TaskError("task scope is not authorized; handoff cannot grant missing permission")
    actual = git_snapshot(root)
    drift = actual["change_fingerprint"] != handoff["repository"]["change_fingerprint"]
    if drift and not args.reconcile:
        raise TaskError("worktree drift detected; inspect doofus-task status, then resume with --reconcile and a truthful --reconcile-note")
    if args.reconcile and not args.reconcile_note:
        raise TaskError("--reconcile requires a note recording the inspected filesystem/process outcome")
    owner = handoff["current_owner"]
    if owner and owner["ownership_state"] == "active":
        expiry = datetime.fromisoformat(owner["lease_expires_at"].replace("Z", "+00:00"))
        if expiry > datetime.now(timezone.utc):
            raise TaskError("active owner lease has not expired; require explicit handoff or wait for expiry")
        if not args.takeover:
            raise TaskError("owner lease expired but prior process state is uncertain; use --takeover after inspecting active operations")
    recipient = handoff["intended_recipient"]
    if recipient and recipient != args.provider:
        raise TaskError(f"handoff is addressed to {recipient}; {args.provider} cannot take ownership")
    if args.takeover and not args.reconcile_note:
        raise TaskError("--takeover requires --reconcile-note documenting process and operation reconciliation")
    request_id = new_request(args)
    lease = str(uuid.uuid4())
    previous = handoff["current_owner"]
    def mutate(current: dict[str, Any], revision: int) -> None:
        current["previous_agent"] = {"provider": previous["provider"] if previous else None, "model": previous["model"] if previous else None}
        current["current_owner"] = {"provider": args.provider, "model": args.model, "lease_id": lease, "scope_task_id": current["scope_task_id"], "lease_expires_at": (datetime.now(timezone.utc) + timedelta(minutes=LEASE_MINUTES)).replace(microsecond=0).isoformat().replace("+00:00", "Z"), "ownership_state": "active"}
        current["intended_recipient"] = None
        current["repository"] = actual
        current["checkpoint"] = {"state": "in_progress", "verified_at": utc_now(), "summary": args.reconcile_note or "Resumed from validated repository checkpoint.", "command": "resume", "evidence": []}
        current["recommended_next_action"] = current.get("recommended_next_action") or "Inspect remaining tasks and continue the next dependency-ready item."
        current["uncertain_operations"] = []
    return record_operation(root, handoff, request_id, "task.resumed", args.provider, args.model, {"git_drift_reconciled": drift, "takeover": bool(args.takeover)}, mutate)


def handoff(root: Path, args: argparse.Namespace) -> tuple[dict[str, Any], str]:
    if args.to not in PROVIDERS or args.provider not in PROVIDERS:
        raise TaskError("provider names must be codex, antigravity, or hermes")
    handoff_state, ledger = load_pair(root)
    require_owner(handoff_state, args.provider, args.lease_id)
    requested_scope = args.scope_task_id or handoff_state["scope_task_id"]
    if not task_in_subtree(ledger["tasks"], handoff_state["scope_task_id"], requested_scope):
        raise TaskError("handoff scope cannot exceed the current owner's task subtree")
    unresolved = unresolved_operations(root)
    if unresolved:
        raise TaskError(f"cannot hand off with unresolved operations: {', '.join(unresolved)}")
    if args.preview:
        preview = dict(handoff_state)
        preview["scope_task_id"] = requested_scope
        return preview, make_handoff_prompt(preview, args.to, handoff_state["revision"] + 1)
    request_id = new_request(args)
    def mutate(current: dict[str, Any], revision: int) -> None:
        current["previous_agent"] = {"provider": args.provider, "model": current["current_owner"]["model"]}
        current["current_owner"] = None
        current["intended_recipient"] = args.to
        current["scope_task_id"] = requested_scope
        current["phase"] = current.get("phase", "")
        current["checkpoint"] = {"state": "in_progress", "verified_at": utc_now(), "summary": args.note, "command": "handoff", "evidence": []}
        current["repository"] = git_snapshot(root)
    updated = record_operation(root, handoff_state, request_id, "task.handed_off", args.provider, handoff_state["current_owner"]["model"], {"to_provider": args.to, "note": args.note}, mutate)
    return updated, make_handoff_prompt(updated, args.to, updated["revision"])


def make_handoff_prompt(handoff_state: dict[str, Any], recipient: str, revision: int) -> str:
    repo = handoff_state["repository"]["repository_identity"] or "the repository checkout"
    branch = handoff_state["repository"]["branch"]
    return (f"Continue Doofus Dock Hermes task {handoff_state['task_id']} within authorized subtree {handoff_state['scope_task_id']} from checkpoint revision {revision}. "
            f"Open {repo} on branch {branch}. First read docs/AGENT_HANDOFF.md, validate .doofus schemas/state, inspect actual Git/worktree status, reconcile active processes, external operations, uncertain operations and approvals, and resume with `python apps/hermes-native/tools/doofus_task.py resume --as {recipient}`. "
            "Do not rely on prior conversation or repeat completed work. Check `progress --json` and continue the recommended next action.")


def checkpoint(root: Path, args: argparse.Namespace) -> dict[str, Any]:
    handoff_state, _ledger = load_pair(root)
    require_owner(handoff_state, args.provider, args.lease_id)
    if args.state == "accepted":
        raise TaskError("formal acceptance can only come from the verified source-pair acceptance ledger")
    if args.state in {"tested", "verified"} and not args.evidence:
        raise TaskError(f"checkpoint state {args.state} requires at least one evidence reference")
    validate_evidence_refs(root, args.evidence)
    if unresolved_operations(root):
        raise TaskError("uncertain operation exists; reconcile it before checkpointing")
    request_id = new_request(args)
    actual = git_snapshot(root)
    def mutate(current: dict[str, Any], revision: int) -> None:
        current["repository"] = actual
        current["checkpoint"] = {"state": args.state, "verified_at": utc_now(), "summary": args.summary, "command": args.command, "evidence": args.evidence}
        current["recommended_next_action"] = args.next_action
        current["tests_executed"].extend({"command": command, "result": args.test_result, "timestamp": utc_now()} for command in args.test)
        for failure in args.failure:
            current["known_failures"].append({"id": str(uuid.uuid4()), "description": failure, "state": "failed", "recorded_at": utc_now()})
        for blocker in args.blocker:
            current["blockers"].append({"id": str(uuid.uuid4()), "description": blocker, "state": "blocked", "recorded_at": utc_now()})
        for approval in args.pending_approval:
            if approval not in current["pending_user_approvals"]:
                current["pending_user_approvals"].append(approval)
        current["changed_files"] = [{"path": item["path"], "status": item["status"], "owner": args.provider if item["path"] in args.owned_path else "unattributed_preexisting", "change_type": "modified" if item["status"] != "??" else "untracked"} for item in actual["changed_files"]]
        current["current_owner"]["lease_expires_at"] = (datetime.now(timezone.utc) + timedelta(minutes=LEASE_MINUTES)).replace(microsecond=0).isoformat().replace("+00:00", "Z")
        current["uncertain_operations"] = []
    operation_details = {"checkpoint_state": args.state, "summary": args.summary, "next_action": args.next_action, "command": args.command, "tests": args.test, "test_result": args.test_result, "evidence": args.evidence, "failures": args.failure, "blockers": args.blocker, "pending_approvals": args.pending_approval, "owned_paths": args.owned_path}
    updated = record_operation(root, handoff_state, request_id, "task.checkpointed", args.provider, handoff_state["current_owner"]["model"], operation_details, mutate)
    if args.test:
        event_id = f"{request_id}:tests"
        event = make_event(event_id, "test.run", "committed", updated["revision"], {"provider": args.provider, "model": updated["current_owner"]["model"], "task_id": updated["task_id"], "commands": args.test, "result": args.test_result})
        append_event_idempotent(root, event)
    return updated


def _bounded_record_text(value: str, label: str, limit: int) -> str:
    if not isinstance(value, str) or not value.strip() or len(value) > limit or any(ord(ch) < 32 for ch in value):
        raise TaskError(f"{label} must be non-empty, at most {limit} characters, and contain no control characters")
    return value.strip()


def _stable_record_id(value: str, label: str) -> str:
    value = _bounded_record_text(value, label, 128)
    if not re.fullmatch(r"[A-Za-z0-9_.:-]+", value):
        raise TaskError(f"{label} must be a stable ID containing only letters, digits, '.', '_', ':', and '-'")
    return value


def record_process(root: Path, args: argparse.Namespace) -> dict[str, Any]:
    handoff_state, _ledger = load_pair(root)
    require_owner(handoff_state, args.provider, args.lease_id)
    process_id = _stable_record_id(args.process_id, "process ID")
    kind = _stable_record_id(args.kind, "process kind")
    note = _bounded_record_text(args.note, "process note", 2048)
    if args.state not in {"running", "stopped", "unknown"}:
        raise TaskError("process state must be running, stopped, or unknown")
    if args.pid is not None and args.pid < 1:
        raise TaskError("PID must be a positive integer")
    details = {"process_id": process_id, "kind": kind, "state": args.state, "pid": args.pid, "note": note}

    def mutate(current: dict[str, Any], _revision: int) -> None:
        record = {**details, "provider": args.provider, "model": args.model, "observed_at": utc_now()}
        records = [item for item in current["active_processes"] if item.get("process_id") != process_id]
        records.append(record)
        current["active_processes"] = sorted(records, key=lambda item: item["process_id"])

    return record_operation(root, handoff_state, args.request_id or str(uuid.uuid4()), "process.state_changed", args.provider, args.model, details, mutate)


def record_external_operation(root: Path, args: argparse.Namespace) -> dict[str, Any]:
    handoff_state, _ledger = load_pair(root)
    require_owner(handoff_state, args.provider, args.lease_id)
    operation_id = _stable_record_id(args.operation_id, "external operation ID")
    kind = _stable_record_id(args.kind, "external operation kind")
    note = _bounded_record_text(args.note, "external operation note", 2048)
    if args.state not in {"running", "succeeded", "failed", "unknown"}:
        raise TaskError("external operation state must be running, succeeded, failed, or unknown")
    validate_evidence_refs(root, args.evidence)
    details = {"operation_id": operation_id, "kind": kind, "state": args.state, "note": note, "evidence": args.evidence}

    def mutate(current: dict[str, Any], _revision: int) -> None:
        record = {**details, "owner": {"provider": args.provider, "model": args.model}, "observed_at": utc_now()}
        records = [item for item in current["external_operations"] if item.get("operation_id") != operation_id]
        records.append(record)
        current["external_operations"] = sorted(records, key=lambda item: item.get("operation_id", item.get("operation", "")))

    return record_operation(root, handoff_state, args.request_id or str(uuid.uuid4()), "external_operation.state_changed", args.provider, args.model, details, mutate)


def find_task(tasks: list[dict[str, Any]], task_id: str) -> dict[str, Any] | None:
    for task in tasks:
        if task["task_id"] == task_id:
            return task
        found = find_task(task.get("children", []), task_id)
        if found:
            return found
    return None


def task_in_subtree(tasks: list[dict[str, Any]], scope_task_id: str, target_task_id: str) -> bool:
    by_id = {task["task_id"]: task for top in tasks for task in flatten(top)}
    current = by_id.get(target_task_id)
    while current is not None:
        if current["task_id"] == scope_task_id:
            return True
        parent_id = current.get("parent_task_id")
        current = by_id.get(parent_id) if parent_id else None
    return False


def update_task(root: Path, args: argparse.Namespace) -> dict[str, Any]:
    handoff_state, ledger = load_pair(root)
    require_owner(handoff_state, args.provider, args.lease_id)
    if handoff_state["authorization"]["status"] != "authorized" or handoff_state["pending_user_approvals"]:
        raise TaskError("task state mutation is not authorized or has pending user approvals")
    target = find_task(ledger["tasks"], args.task_id)
    if target is None:
        raise TaskError(f"task ID not found: {args.task_id}")
    if not task_in_subtree(ledger["tasks"], handoff_state["scope_task_id"], args.task_id):
        raise TaskError("task update is outside the current owner's authorized task subtree")
    if args.state in {"tested", "verified", "accepted"} and not args.evidence:
        raise TaskError(f"state {args.state} requires at least one evidence reference")
    validate_evidence_refs(root, args.evidence)
    if args.task_id.startswith("hermes-par-"):
        par_id = args.task_id.removeprefix("hermes-").upper()
        csv_path = root / "apps" / "hermes-native" / "docs" / "status" / "IMPLEMENTATION_PROGRESS.csv"
        with csv_path.open(encoding="utf-8-sig", newline="") as stream:
            row = next((row for row in csv.DictReader(stream) if row["test_id"] == par_id), None)
        if row is None:
            raise TaskError(f"fixed parity row {par_id} is missing")
        if args.state == "accepted" and row["source_pair_state"] != "accepted":
            raise TaskError("cannot mark a parity task accepted until its source-pair ledger says accepted")
        if row["source_pair_state"] == "accepted" and args.state != "accepted":
            raise TaskError("task state cannot weaken a source-pair acceptance receipt")
    task_index = {item["task_id"]: item for top in ledger["tasks"] for item in flatten(top)}
    incomplete = [dependency for dependency in target.get("dependencies", []) if task_index[dependency]["state"] not in {"tested", "verified", "accepted"}]
    if args.state in {"tested", "verified", "accepted"} and incomplete:
        raise TaskError(f"task dependencies are incomplete: {', '.join(incomplete)}")
    if args.duration_minutes is not None and (args.duration_minutes <= 0 or not args.size_units or args.size_units <= 0 or not args.started_at):
        raise TaskError("measured effort requires positive duration, positive size units, and a recorded start timestamp")
    if args.started_at:
        try:
            datetime.fromisoformat(args.started_at.replace("Z", "+00:00"))
        except ValueError as exc:
            raise TaskError("task start timestamp must be an ISO-8601 datetime") from exc
    request_id = new_request(args)
    prior = [event for event in read_events(root) if event["event_id"] == request_id and event["event_type"] == "task.state_updated"]
    if prior:
        committed = next((event for event in prior if event["phase"] == "committed"), None)
        expected = {"task_id": args.task_id, "new_state": args.state, "evidence": args.evidence}
        if committed and all(committed["details"].get(key) == value for key, value in expected.items()) and handoff_state.get("last_operation", {}).get("request_id") == request_id:
            return handoff_state
        raise TaskError("task update request ID is already prepared, failed, or has conflicting contents; reconcile it before retrying")
    old_ledger_revision = ledger["revision"]
    target_ledger_revision = old_ledger_revision + 1
    with mutation_lock(root, request_id):
        current_handoff, current_ledger = load_pair(root)
        if current_handoff["revision"] != handoff_state["revision"] or current_ledger["revision"] != old_ledger_revision:
            raise TaskError("task state changed concurrently; reload status before retrying")
        details = {"task_id": args.task_id, "new_state": args.state, "old_ledger_revision": old_ledger_revision, "target_ledger_revision": target_ledger_revision, "evidence": args.evidence}
        append_event_idempotent(root, make_event(request_id, "task.state_updated", "prepared", current_handoff["revision"] + 1, {"provider": args.provider, "model": args.model, **details}))
        task_to_update = find_task(current_ledger["tasks"], args.task_id)
        assert task_to_update is not None
        task_to_update["state"] = args.state
        task_to_update["owner"] = args.provider
        task_to_update["lease_id"] = args.lease_id
        task_to_update["agent_identity"] = {"provider": args.provider, "model": args.model}
        task_to_update["verification_evidence"] = list(dict.fromkeys(task_to_update["verification_evidence"] + args.evidence))
        task_to_update["last_updated_at"] = utc_now()
        if args.phase:
            task_to_update["phase"] = args.phase
        if args.size_units is not None:
            task_to_update["size_units"] = args.size_units
        if args.duration_minutes is not None:
            task_to_update["measured_effort_minutes"] = args.duration_minutes
            task_to_update["started_at"] = args.started_at
        current_ledger["revision"] = target_ledger_revision
        current_ledger["updated_at"] = utc_now()
        atomic_json(root, root / ".doofus" / "task-ledger.json", current_ledger, "task-ledger")
        current_handoff["revision"] += 1
        current_handoff["state_revision"] = current_handoff["revision"]
        current_handoff["updated_at"] = utc_now()
        current_handoff["task_ledger_revision"] = target_ledger_revision
        current_handoff["last_event_id"] = request_id
        current_handoff["last_operation"] = {"request_id": request_id, "state": "committed", "type": "task.state_updated"}
        all_tasks = [task for top in current_ledger["tasks"] for task in flatten(top)]
        completed_states = {"tested", "verified", "accepted"}
        current_handoff["completed_subtasks"] = [task["task_id"] for task in all_tasks if task["state"] in completed_states]
        current_handoff["remaining_subtasks"] = [task["task_id"] for task in all_tasks if task["state"] not in completed_states]
        if args.duration_minutes and args.size_units and args.phase and args.started_at:
            current_handoff["phase_effort_samples"].append({"phase": args.phase, "duration_minutes": args.duration_minutes, "size_units": args.size_units, "started_at": args.started_at, "observed_at": utc_now(), "task_id": args.task_id})
        atomic_json(root, root / ".doofus" / "handoff.json", current_handoff, "handoff")
        append_event_idempotent(root, make_event(request_id, "task.state_updated", "committed", current_handoff["revision"], {"provider": args.provider, "model": args.model, **details}))
        return current_handoff


def select_task(root: Path, args: argparse.Namespace) -> dict[str, Any]:
    handoff_state, ledger = load_pair(root)
    require_owner(handoff_state, args.provider, args.lease_id)
    if handoff_state["authorization"]["status"] != "authorized" or handoff_state["pending_user_approvals"]:
        raise TaskError("task selection is not authorized or has pending user approvals")
    if not task_in_subtree(ledger["tasks"], handoff_state["scope_task_id"], args.task_id):
        raise TaskError("selected task is outside the current owner's authorized task subtree")
    target = find_task(ledger["tasks"], args.task_id)
    assert target is not None
    if target["state"] in {"tested", "verified", "accepted"}:
        raise TaskError(f"task {args.task_id} is already complete at state {target['state']}")
    by_id = {task["task_id"]: task for top in ledger["tasks"] for task in flatten(top)}
    incomplete = [dependency for dependency in target.get("dependencies", []) if by_id[dependency]["state"] not in {"tested", "verified", "accepted"}]
    if incomplete:
        raise TaskError(f"selected task dependencies are incomplete: {', '.join(incomplete)}")
    request_id = new_request(args)
    old_id = handoff_state["task_id"]
    def mutate(current: dict[str, Any], revision: int) -> None:
        current["parent_task_id"] = target.get("parent_task_id")
        current["task_id"] = target["task_id"]
        phase = target.get("phase") or target.get("inputs", {}).get("phase")
        if phase:
            current["phase"] = phase
        current["recommended_next_action"] = args.next_action or target["objective"]
        current["checkpoint"] = {"state": target["state"], "verified_at": utc_now(), "summary": args.note, "command": "task select", "evidence": target.get("verification_evidence", [])}
    return record_operation(root, handoff_state, request_id, "task.selected", args.provider, args.model, {"from_task_id": old_id, "to_task_id": args.task_id, "note": args.note}, mutate)


def reconcile(root: Path, args: argparse.Namespace) -> dict[str, Any]:
    events = read_events(root)
    prepared = next((event for event in events if event["event_id"] == args.operation_id and event["phase"] == "prepared"), None)
    if not prepared:
        raise TaskError("prepared operation ID not found")
    terminal = any(event["event_id"] == args.operation_id and event["phase"] in {"committed", "reconciled"} for event in events)
    if terminal:
        return {"status": "already_terminal", "operation_id": args.operation_id}
    handoff_state = load_handoff(root)
    if prepared["event_type"] == "task.state_updated" and args.outcome == "committed" and handoff_state.get("last_event_id") != args.operation_id:
        ledger = read_json(root, root / ".doofus" / "task-ledger.json", "task-ledger")
        detail = prepared["details"]
        if ledger["revision"] != detail["target_ledger_revision"] or handoff_state["revision"] + 1 != prepared["revision"] or handoff_state["task_ledger_revision"] != detail["old_ledger_revision"]:
            raise TaskError("disk state does not prove a recoverable committed task update; manual repair is required")
        handoff_state["revision"] = prepared["revision"]
        handoff_state["state_revision"] = prepared["revision"]
        handoff_state["task_ledger_revision"] = ledger["revision"]
        handoff_state["last_event_id"] = args.operation_id
        handoff_state["last_operation"] = {"request_id": args.operation_id, "state": "committed", "type": "task.state_updated"}
        handoff_state["updated_at"] = utc_now()
        all_tasks = [task for top in ledger["tasks"] for task in flatten(top)]
        complete = {"tested", "verified", "accepted"}
        handoff_state["completed_subtasks"] = [task["task_id"] for task in all_tasks if task["state"] in complete]
        handoff_state["remaining_subtasks"] = [task["task_id"] for task in all_tasks if task["state"] not in complete]
        atomic_json(root, root / ".doofus" / "handoff.json", handoff_state, "handoff")
    if args.outcome == "committed" and handoff_state.get("last_event_id") != args.operation_id:
        raise TaskError("manifest does not prove this operation committed; cannot mark it committed")
    if args.outcome == "failed" and handoff_state.get("last_event_id") == args.operation_id:
        raise TaskError("manifest records this operation as current; cannot mark it failed")
    if prepared["event_type"] == "task.state_updated":
        ledger = read_json(root, root / ".doofus" / "task-ledger.json", "task-ledger")
        expected = prepared["details"]["target_ledger_revision"] if args.outcome == "committed" else prepared["details"]["old_ledger_revision"]
        if ledger["revision"] != expected:
            raise TaskError(f"task ledger revision {ledger['revision']} does not prove {args.outcome}; manual repair is required")
        if args.outcome == "committed" and handoff_state["task_ledger_revision"] != ledger["revision"]:
            raise TaskError("handoff manifest and task ledger disagree; cannot mark this operation committed")
    event = make_event(args.operation_id, prepared["event_type"], "reconciled", prepared["revision"], {"provider": args.provider, "model": args.model, "task_id": prepared["task_id"], "outcome": args.outcome, "note": args.note})
    append_event_idempotent(root, event)
    return {"status": "reconciled", "operation_id": args.operation_id, "outcome": args.outcome}


def recover_lock(root: Path, args: argparse.Namespace) -> dict[str, Any]:
    lock = root / ".doofus" / ".mutation.lock"
    if not lock.exists():
        return {"status": "no_lock", "recovered": False}
    if not args.confirm or not args.note:
        raise TaskError("lock recovery requires --confirm and an evidence-based --note")
    try:
        owner = json.loads((lock / "owner.json").read_text(encoding="utf-8"))
        pid = int(owner["pid"])
        expiry = datetime.fromisoformat(owner["expires_at"].replace("Z", "+00:00"))
    except Exception as exc:
        raise TaskError(f"lock metadata is corrupt ({exc}); preserve it and resolve manually") from exc
    try:
        os.kill(pid, 0)
        process_alive = True
    except ProcessLookupError:
        process_alive = False
    except PermissionError:
        process_alive = True
    if process_alive:
        raise TaskError("lock owner process still exists; refusing recovery even if the lease expired")
    expired = expiry <= datetime.now(timezone.utc)
    recovery = lock.with_name(f".mutation.lock.recovered-{uuid.uuid4().hex}")
    os.replace(lock, recovery)
    created_lock = False
    try:
        lock.mkdir()
        created_lock = True
        (lock / "owner.json").write_text(json.dumps({"request_id": args.request_id or str(uuid.uuid4()), "pid": os.getpid(), "created_at": utc_now(), "expires_at": (datetime.now(timezone.utc) + timedelta(minutes=5)).replace(microsecond=0).isoformat().replace("+00:00", "Z")}, sort_keys=True), encoding="utf-8")
        event_id = new_request(args)
        handoff_state = load_handoff(root)
        event = make_event(event_id, "task.lock_recovered", "committed", handoff_state["revision"], {"provider": args.provider, "model": args.model, "task_id": handoff_state["task_id"], "note": args.note, "expired_owner_process": "not_running", "lease_expired": expired})
        append_event_idempotent(root, event)
    finally:
        if created_lock:
            (lock / "owner.json").unlink(missing_ok=True)
            try:
                lock.rmdir()
            except OSError:
                pass
        try:
            (recovery / "owner.json").unlink(missing_ok=True)
            recovery.rmdir()
        except OSError:
            pass
    return {"status": "recovered", "recovered": True, "lease_expired": expired, "note": args.note}


def render(report: dict[str, Any], json_output: bool) -> str:
    if json_output:
        return json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True)
    if "implementation" not in report:
        return "\n".join([
            f"Task: {report['task_id']} · revision {report['revision']}",
            f"Phase: {report['phase']}",
            f"Owner: {report['owner']}",
            f"Worktree drift: {'yes' if report['git_drift'] else 'no'}",
            f"Unresolved operations: {report['unresolved_operations']}",
            f"Process records: {report.get('active_processes', [])}",
            f"External operations: {report.get('external_operations', [])}",
            f"Needs reconciliation: processes={report.get('processes_needing_reconciliation', [])} operations={report.get('external_operations_needing_reconciliation', [])}",
            f"Next: {report['recommended_next_action']}",
        ])
    counts = report["implementation"]["states"]
    return "\n".join([
        f"Hermes Native agent-workflow progress · pinned {report['pinned_source_snapshot']}",
        f"Agent-workflow scope: {report['agent_workflow_scope']['total']} requirements",
        f"Implementation: {counts.get('implemented', 0)} implemented · {counts.get('partial', 0)} partial · {counts.get('not_implemented', 0)} absent by review · {counts.get('unknown', 0)} not reviewed",
        f"Task lifecycle (separate from feature state): {report['tasks']['tested_or_accepted']} tested/accepted · {report['tasks']['active']} active · {report['tasks']['blocked']} blocked",
        f"Evidence mapping: {report['implementation']['reviewed']}/{report['agent_workflow_scope']['total']} reviewed rows ({report['implementation']['review_coverage_percent']}% evidence coverage, not completion)",
        f"Headless/test evidence mapped: {report['automated_evidence']['requirements_with_mapped_test_or_headless_evidence']}/{report['agent_workflow_scope']['total']} · test runs recorded: {report['automated_evidence']['test_runs_recorded']}",
        f"Live runtime states: {report['live_runtime']}", f"Source-pair acceptance states: {report['source_pair_acceptance']}",
        f"Critical path: {report['critical_path']['status']}", f"ETA: {report['estimated_remaining_effort']}", f"Elapsed time: {report['elapsed_status']}",
        f"Owner: {report['owner']}", f"Last meaningful progress: {report['last_meaningful_progress_at']}", f"Next: {report['recommended_next_action']}",
    ])


def parser() -> argparse.ArgumentParser:
    root_parser = argparse.ArgumentParser(description="Provider-neutral durable task handoff and evidence progress")
    root_parser.add_argument("--repo", type=Path, help="repository path; defaults to current Git worktree")
    commands = root_parser.add_subparsers(dest="subcommand", required=True)
    init = commands.add_parser("init"); init.add_argument("--provider", required=True); init.add_argument("--model"); init.add_argument("--request-id");
    status = commands.add_parser("status"); status.add_argument("--json", action="store_true")
    progress = commands.add_parser("progress"); progress.add_argument("--json", action="store_true")
    validate = commands.add_parser("validate")
    checkpoint_parser = commands.add_parser("checkpoint"); checkpoint_parser.add_argument("--provider", required=True); checkpoint_parser.add_argument("--lease-id", required=True); checkpoint_parser.add_argument("--request-id"); checkpoint_parser.add_argument("--state", choices=STATES, default="in_progress"); checkpoint_parser.add_argument("--summary", required=True); checkpoint_parser.add_argument("--next-action", required=True); checkpoint_parser.add_argument("--command"); checkpoint_parser.add_argument("--test", action="append", default=[]); checkpoint_parser.add_argument("--test-result", default="recorded"); checkpoint_parser.add_argument("--evidence", action="append", default=[]); checkpoint_parser.add_argument("--failure", action="append", default=[]); checkpoint_parser.add_argument("--blocker", action="append", default=[]); checkpoint_parser.add_argument("--pending-approval", action="append", default=[]); checkpoint_parser.add_argument("--owned-path", action="append", default=[])
    process = commands.add_parser("process"); process_commands = process.add_subparsers(dest="process_command", required=True); process_record = process_commands.add_parser("record"); process_record.add_argument("--provider", required=True); process_record.add_argument("--model"); process_record.add_argument("--lease-id", required=True); process_record.add_argument("--request-id"); process_record.add_argument("--process-id", required=True); process_record.add_argument("--kind", required=True); process_record.add_argument("--state", choices=["running", "stopped", "unknown"], required=True); process_record.add_argument("--pid", type=int); process_record.add_argument("--note", required=True)
    operation = commands.add_parser("operation"); operation_commands = operation.add_subparsers(dest="operation_command", required=True); operation_record = operation_commands.add_parser("record"); operation_record.add_argument("--provider", required=True); operation_record.add_argument("--model"); operation_record.add_argument("--lease-id", required=True); operation_record.add_argument("--request-id"); operation_record.add_argument("--operation-id", required=True); operation_record.add_argument("--kind", required=True); operation_record.add_argument("--state", choices=["running", "succeeded", "failed", "unknown"], required=True); operation_record.add_argument("--note", required=True); operation_record.add_argument("--evidence", action="append", default=[])
    task_parser = commands.add_parser("task"); task_commands = task_parser.add_subparsers(dest="task_command", required=True); task_update = task_commands.add_parser("update"); task_update.add_argument("--provider", required=True); task_update.add_argument("--model"); task_update.add_argument("--lease-id", required=True); task_update.add_argument("--request-id"); task_update.add_argument("--task-id", required=True); task_update.add_argument("--state", choices=STATES, required=True); task_update.add_argument("--evidence", action="append", default=[]); task_update.add_argument("--phase"); task_update.add_argument("--duration-minutes", type=float); task_update.add_argument("--size-units", type=float); task_update.add_argument("--started-at")
    task_select = task_commands.add_parser("select"); task_select.add_argument("--provider", required=True); task_select.add_argument("--model"); task_select.add_argument("--lease-id", required=True); task_select.add_argument("--request-id"); task_select.add_argument("--task-id", required=True); task_select.add_argument("--note", required=True); task_select.add_argument("--next-action")
    hand = commands.add_parser("handoff"); hand.add_argument("--provider", required=True); hand.add_argument("--lease-id", required=True); hand.add_argument("--to", required=True); hand.add_argument("--note", required=True); hand.add_argument("--request-id"); hand.add_argument("--scope-task-id"); hand.add_argument("--preview", action="store_true")
    resume_parser = commands.add_parser("resume"); resume_parser.add_argument("--as", dest="provider", required=True); resume_parser.add_argument("--model"); resume_parser.add_argument("--request-id"); resume_parser.add_argument("--reconcile", action="store_true"); resume_parser.add_argument("--reconcile-note"); resume_parser.add_argument("--takeover", action="store_true")
    rec = commands.add_parser("reconcile"); rec.add_argument("--operation-id", required=True); rec.add_argument("--outcome", choices=["committed", "failed"], required=True); rec.add_argument("--note", required=True); rec.add_argument("--provider", required=True); rec.add_argument("--model")
    recover = commands.add_parser("recover-lock"); recover.add_argument("--provider", required=True); recover.add_argument("--model"); recover.add_argument("--note", required=True); recover.add_argument("--confirm", action="store_true"); recover.add_argument("--request-id")
    return root_parser


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        root = repo_root(args.repo)
        if args.subcommand == "init":
            print(json.dumps({"status": "initialized", "revision": initialize(root, args)["revision"], "state": ".doofus/handoff.json"}, sort_keys=True)); return 0
        if args.subcommand == "validate":
            handoff_state, ledger = load_pair(root); read_events(root)
            print(json.dumps({"status": "valid", "revision": handoff_state["revision"], "task_ledger_revision": ledger["revision"], "event_count": len(read_events(root))}, sort_keys=True)); return 0
        if args.subcommand == "status":
            report = status_report(root); print(render(report, args.json)); return 0
        if args.subcommand == "progress":
            report = progress_report(root); print(render(report, args.json)); return 0
        if args.subcommand == "resume":
            updated = resume(root, args); print(json.dumps({"status": "resumed", "revision": updated["revision"], "owner": updated["current_owner"], "lease_id": updated["current_owner"]["lease_id"]}, sort_keys=True)); return 0
        if args.subcommand == "handoff":
            updated, prompt = handoff(root, args)
            if args.preview:
                result = {"status": "preview", "revision": updated["revision"], "proposed_revision": updated["revision"] + 1, "to": args.to, "ownership_transferred": False, "prompt": prompt}
            else:
                result = {"status": "handed_off", "revision": updated["revision"], "to": updated["intended_recipient"], "ownership_transferred": True, "prompt": prompt}
            print(json.dumps(result, ensure_ascii=False, sort_keys=True)); return 0
        if args.subcommand == "checkpoint":
            updated = checkpoint(root, args); print(json.dumps({"status": "checkpointed", "revision": updated["revision"], "owner": updated["current_owner"]}, sort_keys=True)); return 0
        if args.subcommand == "process" and args.process_command == "record":
            updated = record_process(root, args); print(json.dumps({"status": "process_recorded", "revision": updated["revision"], "process_id": args.process_id, "state": args.state}, sort_keys=True)); return 0
        if args.subcommand == "operation" and args.operation_command == "record":
            updated = record_external_operation(root, args); print(json.dumps({"status": "external_operation_recorded", "revision": updated["revision"], "operation_id": args.operation_id, "state": args.state}, sort_keys=True)); return 0
        if args.subcommand == "task" and args.task_command == "update":
            updated = update_task(root, args); print(json.dumps({"status": "task_updated", "revision": updated["revision"], "task_id": args.task_id, "state": args.state}, sort_keys=True)); return 0
        if args.subcommand == "task" and args.task_command == "select":
            updated = select_task(root, args); print(json.dumps({"status": "task_selected", "revision": updated["revision"], "task_id": args.task_id}, sort_keys=True)); return 0
        if args.subcommand == "reconcile":
            print(json.dumps(reconcile(root, args), sort_keys=True)); return 0
        if args.subcommand == "recover-lock":
            print(json.dumps(recover_lock(root, args), ensure_ascii=False, sort_keys=True)); return 0
        raise TaskError(f"unsupported command: {args.subcommand}")
    except TaskError as exc:
        print(json.dumps({"status": "error", "error": str(exc)}, ensure_ascii=False), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

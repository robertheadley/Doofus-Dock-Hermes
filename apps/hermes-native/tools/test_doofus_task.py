"""Temporary-repository simulations for durable handoff and recovery."""

from __future__ import annotations

import json
import csv
import io
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from contextlib import redirect_stdout
from types import SimpleNamespace
from unittest import mock

import doofus_task as task


REPO = Path(__file__).resolve().parents[3]
SIMULATED_PASS_EVIDENCE = "SIMULATED_PASS_EVIDENCE"


class DoofusTaskTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="doofus-task-test-")
        self.root = Path(self.temp.name)
        (self.root / ".doofus" / "schemas").mkdir(parents=True)
        for schema in (REPO / ".doofus" / "schemas").glob("*.schema.json"):
            shutil.copy2(schema, self.root / ".doofus" / "schemas" / schema.name)
        csv_dir = self.root / "apps" / "hermes-native" / "docs" / "status"
        csv_dir.mkdir(parents=True)
        shutil.copy2(REPO / "apps" / "hermes-native" / "docs" / "status" / "IMPLEMENTATION_PROGRESS.csv", csv_dir / "IMPLEMENTATION_PROGRESS.csv")
        shutil.copy2(REPO / "apps" / "hermes-native" / "docs" / "status" / "CHAT_TOOL_PROGRESS.csv", csv_dir / "CHAT_TOOL_PROGRESS.csv")
        (self.root / "tests").mkdir()
        (self.root / "tests" / "evidence.txt").write_text(SIMULATED_PASS_EVIDENCE + "\n", encoding="utf-8")
        (self.root / "README.md").write_text("isolated simulated repository\n", encoding="utf-8")
        self.git("init", "--quiet")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "user.name", "Simulation")
        self.git("remote", "add", "origin", "https://github.com/example/hermes.git")
        self.git("add", ".")
        self.git("commit", "--quiet", "-m", "fixture")
        self.initial = task.initialize(self.root, SimpleNamespace(provider="codex", model=None, request_id="init-1"))

    def tearDown(self) -> None:
        self.temp.cleanup()

    def git(self, *args: str) -> str:
        return subprocess.run(["git", *args], cwd=self.root, check=True, text=True, capture_output=True).stdout.strip()

    def handoff(self, provider: str, lease_id: str, to: str, request_id: str):
        return task.handoff(self.root, SimpleNamespace(provider=provider, lease_id=lease_id, to=to, note=f"simulation to {to}", request_id=request_id, scope_task_id=None, preview=False))

    def resume(self, provider: str, request_id: str):
        return task.resume(self.root, SimpleNamespace(provider=provider, model=None, request_id=request_id, reconcile=False, reconcile_note=None, takeover=False))

    def test_codex_antigravity_hermes_handoff_keeps_task_state_without_transcripts(self) -> None:
        codex_lease = self.initial["current_owner"]["lease_id"]
        handed, prompt = self.handoff("codex", codex_lease, "antigravity", "hand-1")
        self.assertIn("revision 2", prompt)
        self.assertIn("docs/AGENT_HANDOFF.md", prompt)
        self.assertNotIn("chat transcript", prompt.lower())
        with self.assertRaisesRegex(task.TaskError, "active owner"):
            task.checkpoint(self.root, SimpleNamespace(provider="codex", lease_id=codex_lease, state="in_progress", evidence=[], summary="stale", next_action="none", command=None, test=[], test_result="recorded", failure=[], blocker=[], pending_approval=[], owned_path=[], request_id="stale"))

        ag = self.resume("antigravity", "resume-2")
        ag_lease = ag["current_owner"]["lease_id"]
        updated = task.update_task(self.root, SimpleNamespace(provider="antigravity", model=None, lease_id=ag_lease, request_id="task-tested", task_id="hermes-cross-provider-handoff", state="tested", evidence=["tests/evidence.txt#SIMULATED_PASS_EVIDENCE"], phase="phase-a", duration_minutes=None, size_units=None, started_at=None))
        self.assertIn("hermes-cross-provider-handoff", updated["completed_subtasks"])
        self.handoff("antigravity", ag_lease, "hermes", "hand-2")
        hermes = self.resume("hermes", "resume-3")
        ledger = task.read_json(self.root, self.root / ".doofus" / "task-ledger.json", "task-ledger")
        leaf = task.find_task(ledger["tasks"], "hermes-cross-provider-handoff")
        self.assertEqual(leaf["state"], "tested")
        self.assertEqual(hermes["intended_recipient"], None)
        self.assertEqual(task.unresolved_operations(self.root), [])
        packed = json.dumps([hermes, ledger, task.read_events(self.root)])
        for forbidden_key in ("conversation_history", "prior_messages", "chat_history", "transcript_content"):
            self.assertNotIn(f'"{forbidden_key}"', packed)
        self.assertNotIn("chat transcript", prompt.lower())
        self.assertEqual(ledger["tasks"][0]["children"][0]["progress"]["children"], 499)

    def test_process_and_external_operation_records_survive_handoff_and_require_reconciliation(self) -> None:
        lease = self.initial["current_owner"]["lease_id"]
        process_args = SimpleNamespace(provider="codex", model=None, lease_id=lease, request_id="process-running", process_id="subagent:dock-group-minimize", kind="delegated_agent", state="unknown", pid=None, note="Agent may still be running; verify before modifying its worktree.")
        first = task.record_process(self.root, process_args)
        retry = task.record_process(self.root, process_args)
        self.assertEqual(first["revision"], retry["revision"])
        process_args.state = "stopped"
        with self.assertRaisesRegex(task.TaskError, "different operation contents"):
            task.record_process(self.root, process_args)
        operation_args = SimpleNamespace(provider="codex", model=None, lease_id=lease, request_id="operation-running", operation_id="isolated-doofusdock-build", kind="cargo_workspace_check", state="running", note="Build runs in a separate DoofusDock worktree.", evidence=[])
        recorded = task.record_external_operation(self.root, operation_args)
        operation_retry = task.record_external_operation(self.root, operation_args)
        self.assertEqual(recorded["revision"], operation_retry["revision"])
        status = task.status_report(self.root)
        self.assertIn("subagent:dock-group-minimize", status["processes_needing_reconciliation"])
        self.assertIn("isolated-doofusdock-build", status["external_operations_needing_reconciliation"])
        handed, _prompt = self.handoff("codex", lease, "antigravity", "handoff-with-running-work")
        self.assertEqual(handed["active_processes"][0]["state"], "unknown")
        self.assertEqual(handed["external_operations"][-1]["state"], "running")
        resumed = self.resume("antigravity", "resume-running-work")
        self.assertEqual(resumed["current_owner"]["provider"], "antigravity")
        self.assertEqual(resumed["external_operations"][-1]["operation_id"], "isolated-doofusdock-build")
        self.assertEqual(recorded["revision"], 3)

    def test_process_and_external_operation_records_reject_stale_leases_and_duplicate_ids(self) -> None:
        lease = self.initial["current_owner"]["lease_id"]
        args = SimpleNamespace(provider="codex", model=None, lease_id="stale-lease", request_id="process-stale", process_id="test", kind="test", state="running", pid=4, note="Known test process.")
        with self.assertRaisesRegex(task.TaskError, "lease mismatch"):
            task.record_process(self.root, args)
        args.lease_id = lease
        args.process_id = "unsafe/path"
        with self.assertRaisesRegex(task.TaskError, "stable ID"):
            task.record_process(self.root, args)
        args.process_id = "test"
        task.record_process(self.root, args)
        handoff = task.load_handoff(self.root)
        handoff["active_processes"].append(dict(handoff["active_processes"][0]))
        with self.assertRaisesRegex(task.TaskError, "duplicate active process ID"):
            task.validate_document(self.root, "handoff", handoff)
        handoff["active_processes"] = []
        valid_operation = {"operation_id": "op-1", "kind": "cargo_test", "owner": {"provider": "codex", "model": None}, "state": "unknown", "observed_at": task.utc_now(), "note": "Outcome requires reconciliation.", "evidence": []}
        handoff["external_operations"].extend([valid_operation, dict(valid_operation)])
        with self.assertRaisesRegex(task.TaskError, "duplicate external operation ID"):
            task.validate_document(self.root, "handoff", handoff)

    def test_all_provider_pairings_require_explicit_resume_and_preserve_task_state(self) -> None:
        sequence = [
            ("codex", "antigravity"),
            ("antigravity", "codex"),
            ("codex", "hermes"),
            ("hermes", "codex"),
            ("codex", "antigravity"),
            ("antigravity", "hermes"),
            ("hermes", "antigravity"),
        ]
        owner = "codex"
        lease = self.initial["current_owner"]["lease_id"]
        observed = set()
        for index, (provider, recipient) in enumerate(sequence):
            self.assertEqual(owner, provider)
            handed, prompt = self.handoff(provider, lease, recipient, f"pair-handoff-{index}")
            self.assertEqual(handed["intended_recipient"], recipient)
            self.assertIn("resume with `python apps/hermes-native/tools/doofus_task.py resume", prompt.lower())
            with self.assertRaises(task.TaskError):
                task.checkpoint(
                    self.root,
                    SimpleNamespace(
                        provider=provider,
                        lease_id=lease,
                        state="in_progress",
                        evidence=[],
                        summary="stale owner",
                        next_action="resume from recipient",
                        command=None,
                        test=[],
                        test_result="recorded",
                        failure=[],
                        blocker=[],
                        pending_approval=[],
                        owned_path=[],
                        request_id=f"stale-pair-{index}",
                    ),
                )
            resumed = self.resume(recipient, f"pair-resume-{index}")
            self.assertEqual(resumed["current_owner"]["provider"], recipient)
            owner = recipient
            lease = resumed["current_owner"]["lease_id"]
            observed.add((provider, recipient))
        self.assertEqual(
            observed,
            {
                ("codex", "antigravity"),
                ("codex", "hermes"),
                ("antigravity", "codex"),
                ("antigravity", "hermes"),
                ("hermes", "codex"),
                ("hermes", "antigravity"),
            },
        )
        self.assertEqual(task.unresolved_operations(self.root), [])
        handoff = task.load_handoff(self.root)
        self.assertIsNone(handoff["intended_recipient"])
        self.assertEqual(handoff["current_owner"]["provider"], "antigravity")

    def test_worktree_drift_requires_explicit_reconciliation(self) -> None:
        lease = self.initial["current_owner"]["lease_id"]
        self.handoff("codex", lease, "antigravity", "hand-drift")
        (self.root / "unexpected.txt").write_text("new local change\n", encoding="utf-8")
        self.assertTrue(task.status_report(self.root)["git_drift"])
        with self.assertRaisesRegex(task.TaskError, "worktree drift"):
            self.resume("antigravity", "resume-drift-no")
        result = task.resume(self.root, SimpleNamespace(provider="antigravity", model=None, request_id="resume-drift-yes", reconcile=True, reconcile_note="Inspected unexpected.txt; preserving it as an uncommitted change.", takeover=False))
        self.assertEqual(result["current_owner"]["provider"], "antigravity")
        self.assertFalse(task.status_report(self.root)["git_drift"])

    def test_dotdoofus_events_do_not_create_false_git_drift(self) -> None:
        lease = self.initial["current_owner"]["lease_id"]
        self.handoff("codex", lease, "antigravity", "hand-dotdoofus-drift")
        report = task.status_report(self.root)
        self.assertFalse(report["git_drift"])
        self.assertTrue(
            any(item["path"] == ".doofus/events.jsonl" for item in report["changed_files"])
        )

    def test_stale_and_wrong_provider_leases_cannot_update_tasks(self) -> None:
        lease = self.initial["current_owner"]["lease_id"]
        self.handoff("codex", lease, "antigravity", "hand-owner")
        with self.assertRaisesRegex(task.TaskError, "addressed to antigravity"):
            self.resume("hermes", "resume-wrong")
        ag = self.resume("antigravity", "resume-owner")
        with self.assertRaisesRegex(task.TaskError, "lease mismatch"):
            task.update_task(self.root, SimpleNamespace(provider="codex", model=None, lease_id=lease, request_id="wrong-writer", task_id="hermes-cross-provider-handoff", state="implemented", evidence=[], phase=None, duration_minutes=None, size_units=None, started_at=None))

    def test_handoff_scope_only_narrows_and_task_selection_respects_dependencies(self) -> None:
        lease = self.initial["current_owner"]["lease_id"]
        preview_state, prompt = task.handoff(self.root, SimpleNamespace(provider="codex", lease_id=lease, to="antigravity", note="bounded UI task", request_id=None, scope_task_id="hermes-par-0002", preview=True))
        self.assertIn("authorized subtree hermes-par-0002", prompt)
        self.assertEqual(preview_state["scope_task_id"], "hermes-par-0002")
        self.assertEqual(task.load_handoff(self.root)["scope_task_id"], "hermes-native-project")
        narrow = task.handoff(self.root, SimpleNamespace(provider="codex", lease_id=lease, to="antigravity", note="bounded UI task", request_id="narrow-handoff", scope_task_id="hermes-par-0002", preview=False))[0]
        ag = task.resume(self.root, SimpleNamespace(provider="antigravity", model=None, request_id="narrow-resume", reconcile=False, reconcile_note=None, takeover=False))
        with self.assertRaisesRegex(task.TaskError, "outside the current owner's authorized task subtree"):
            task.select_task(self.root, SimpleNamespace(provider="antigravity", model=None, lease_id=ag["current_owner"]["lease_id"], request_id="escape-scope", task_id="hermes-par-0003", note="out of scope", next_action=None))
        self.assertEqual(narrow["scope_task_id"], "hermes-par-0002")

    def test_handoff_preview_reports_no_transfer_and_preserves_owner(self) -> None:
        lease = self.initial["current_owner"]["lease_id"]
        before = task.load_handoff(self.root)
        output = io.StringIO()
        with redirect_stdout(output):
            result = task.main(["--repo", str(self.root), "handoff", "--provider", "codex", "--lease-id", lease, "--to", "antigravity", "--note", "preview only", "--preview"])
        response = json.loads(output.getvalue())
        after = task.load_handoff(self.root)
        self.assertEqual(result, 0)
        self.assertEqual(response["status"], "preview")
        self.assertEqual(response["to"], "antigravity")
        self.assertFalse(response["ownership_transferred"])
        self.assertEqual(response["proposed_revision"], before["revision"] + 1)
        self.assertEqual(after, before)

    def test_selected_task_cannot_skip_an_unfinished_dependency(self) -> None:
        ledger_path = self.root / ".doofus" / "task-ledger.json"
        ledger = task.read_json(self.root, ledger_path, "task-ledger")
        parity = task.find_task(ledger["tasks"], "hermes-par-0002")
        parity["dependencies"] = ["hermes-par-0001"]
        task.atomic_json(self.root, ledger_path, ledger, "task-ledger")
        handoff = task.load_handoff(self.root)
        handoff["task_ledger_revision"] = ledger["revision"]
        task.atomic_json(self.root, self.root / ".doofus" / "handoff.json", handoff, "handoff")
        with self.assertRaisesRegex(task.TaskError, "dependencies are incomplete"):
            task.select_task(self.root, SimpleNamespace(provider="codex", model=None, lease_id=handoff["current_owner"]["lease_id"], request_id="skip-dependency", task_id="hermes-par-0002", note="skip dependency", next_action=None))

    def test_handoff_does_not_grant_pending_user_approval(self) -> None:
        manifest_path = self.root / ".doofus" / "handoff.json"
        handoff = task.load_handoff(self.root)
        handoff["pending_user_approvals"] = ["approve the next protected task"]
        task.atomic_json(self.root, manifest_path, handoff, "handoff")
        lease = handoff["current_owner"]["lease_id"]
        self.handoff("codex", lease, "antigravity", "approval-handoff")
        antigravity = self.resume("antigravity", "approval-resume")
        with self.assertRaisesRegex(task.TaskError, "not authorized or has pending user approvals"):
            task.update_task(self.root, SimpleNamespace(provider="antigravity", model=None, lease_id=antigravity["current_owner"]["lease_id"], request_id="approval-bypass", task_id="hermes-cross-provider-handoff", state="implemented", evidence=[], phase=None, duration_minutes=None, size_units=None, started_at=None))
        self.assertEqual(task.load_handoff(self.root)["pending_user_approvals"], ["approve the next protected task"])

    def test_atomic_checkpoint_crash_keeps_old_manifest_and_requires_resolution(self) -> None:
        lease = self.initial["current_owner"]["lease_id"]
        args = SimpleNamespace(provider="codex", lease_id=lease, request_id="checkpoint-crash", state="in_progress", evidence=[], summary="simulated crash", next_action="reconcile", command="fake", test=[], test_result="recorded", failure=[], blocker=[], pending_approval=[], owned_path=[])
        manifest_path = self.root / ".doofus" / "handoff.json"
        original = manifest_path.read_bytes()
        atomic = task.atomic_json
        def fail_manifest(root: Path, path: Path, value, kind: str) -> None:
            if path == manifest_path:
                raise OSError("injected crash before atomic replace")
            atomic(root, path, value, kind)
        with mock.patch.object(task, "atomic_json", side_effect=fail_manifest):
            with self.assertRaisesRegex(OSError, "injected crash"):
                task.checkpoint(self.root, args)
        self.assertEqual(manifest_path.read_bytes(), original)
        self.assertEqual(task.unresolved_operations(self.root), ["checkpoint-crash"])
        result = task.reconcile(self.root, SimpleNamespace(operation_id="checkpoint-crash", outcome="failed", note="Manifest stayed at the prior revision after injected pre-replace failure.", provider="codex", model=None))
        self.assertEqual(result["outcome"], "failed")
        self.assertEqual(task.unresolved_operations(self.root), [])

    def test_partial_task_update_crash_is_explicitly_reconciled_from_disk(self) -> None:
        lease = self.initial["current_owner"]["lease_id"]
        args = SimpleNamespace(provider="codex", model=None, lease_id=lease, request_id="task-crash", task_id="hermes-cross-provider-handoff", state="tested", evidence=["tests/evidence.txt#SIMULATED_PASS_EVIDENCE"], phase="phase-a", duration_minutes=None, size_units=None, started_at=None)
        manifest_path = self.root / ".doofus" / "handoff.json"
        atomic = task.atomic_json
        def fail_manifest(root: Path, path: Path, value, kind: str) -> None:
            if path == manifest_path:
                raise OSError("injected crash between task ledger and manifest writes")
            atomic(root, path, value, kind)
        with mock.patch.object(task, "atomic_json", side_effect=fail_manifest):
            with self.assertRaisesRegex(OSError, "between task ledger"):
                task.update_task(self.root, args)
        report = task.status_report(self.root)
        self.assertEqual(report["state_consistency"], "reconcile_required")
        result = task.reconcile(self.root, SimpleNamespace(operation_id="task-crash", outcome="committed", note="Task ledger revision and tested evidence are present; applying matching checkpoint revision.", provider="codex", model=None))
        self.assertEqual(result["outcome"], "committed")
        self.assertEqual(task.load_pair(self.root)[0]["task_ledger_revision"], 2)

    def test_progress_reports_only_the_agent_workflow_scope_and_keeps_partial_states(self) -> None:
        report = task.progress_report(self.root)
        self.assertEqual(report["agent_workflow_scope"]["total"], 176)
        progress_path = self.root / "apps" / "hermes-native" / "docs" / "status" / "CHAT_TOOL_PROGRESS.csv"
        with progress_path.open(newline="", encoding="utf-8") as handle:
            rows = list(csv.DictReader(handle))
        expected_states = {}
        for row in rows:
            state = row["implementation_state"]
            expected_states[state] = expected_states.get(state, 0) + 1
        self.assertEqual(report["implementation"]["states"], expected_states)
        self.assertEqual(report["implementation"]["reviewed"], sum(row["implementation_state"] != "unknown" for row in rows))
        self.assertEqual(report["source_pair_acceptance"], {"pending": 176})
        self.assertNotIn("completed", report["tasks"])
        self.assertEqual(
            report["tasks"]["tested_or_accepted"],
            sum(report["tasks"]["states"].get(state, 0) for state in ("tested", "verified", "accepted")),
        )
        plain = task.render(report, json_output=False)
        self.assertIn("Agent-workflow scope: 176 requirements", plain)
        self.assertIn("Task lifecycle (separate from feature state)", plain)
        self.assertNotIn("499", plain)
        self.assertNotIn("Parity inventory", plain)
        self.assertIsNone(report["elapsed_execution_seconds"])
        self.assertEqual(report["estimated_remaining_effort"]["status"], "unavailable_insufficient_phase_samples")
        self.assertEqual(report["estimated_remaining_effort"]["confidence"], "unavailable")
        self.assertIn("measured", report["estimated_remaining_effort"]["uncertainty"])

    def test_progress_eta_requires_measured_samples_and_sizes_for_remaining_work(self) -> None:
        handoff = dict(self.initial)
        handoff["phase_effort_samples"] = [
            {"phase": "phase-a", "duration_minutes": 10.0, "size_units": 1.0, "started_at": "2026-11-01T00:00:00Z"},
            {"phase": "phase-a", "duration_minutes": 8.0, "size_units": 1.0, "started_at": "2026-11-02T00:00:00Z"},
            {"phase": "phase-a", "duration_minutes": 6.0, "size_units": 1.0, "started_at": "2026-11-03T00:00:00Z"},
        ]
        ledger = task.read_json(self.root, self.root / ".doofus" / "task-ledger.json", "task-ledger")
        matching = next(
            item for root_task in ledger["tasks"] for item in task.flatten(root_task)
            if item["task_id"] == "hermes-par-0001"
        )
        matching["state"] = "in_progress"
        matching["phase"] = "phase-a"
        matching["size_units"] = None
        with mock.patch.object(task, "load_pair", return_value=(handoff, ledger)):
            missing = task.progress_report(self.root)["estimated_remaining_effort"]["phase-a"]
            self.assertEqual(missing["status"], "unavailable_remaining_size_unknown")
            self.assertIsNone(missing["eta_minutes"])
            self.assertEqual(missing["tasks_missing_size"], ["hermes-par-0001"])
            self.assertEqual(missing["confidence"], "unavailable")
            matching["size_units"] = 2.0
            estimated = task.progress_report(self.root)["estimated_remaining_effort"]["phase-a"]
        self.assertEqual(estimated["status"], "estimated")
        self.assertEqual(estimated["eta_minutes"], 17.8)
        self.assertEqual(estimated["ewma_alpha"], 0.2)
        self.assertEqual(estimated["confidence"], "limited")
        self.assertIn("not calibrated", estimated["uncertainty"])

    def test_checkpoint_retry_is_idempotent_and_conflicting_retry_is_rejected(self) -> None:
        lease = self.initial["current_owner"]["lease_id"]
        args = SimpleNamespace(provider="codex", lease_id=lease, request_id="checkpoint-idempotent", state="in_progress", evidence=[], summary="same request", next_action="continue", command="unit", test=["python -m unittest"], test_result="PASS 1", failure=[], blocker=[], pending_approval=[], owned_path=[])
        first = task.checkpoint(self.root, args)
        second = task.checkpoint(self.root, args)
        self.assertEqual(first["revision"], second["revision"])
        self.assertEqual(len(second["tests_executed"]), 1)
        args.summary = "different payload"
        with self.assertRaisesRegex(task.TaskError, "different operation contents"):
            task.checkpoint(self.root, args)

    def test_bootstrap_retry_repairs_only_missing_projection(self) -> None:
        for path in (self.root / ".doofus" / "handoff.json", self.root / ".doofus" / "task-ledger.json", self.root / ".doofus" / "events.jsonl"):
            path.unlink()
        original = task.atomic_json
        ledger_path = self.root / ".doofus" / "task-ledger.json"
        def fail_ledger(root: Path, path: Path, value, kind: str) -> None:
            if path == ledger_path:
                raise OSError("injected bootstrap crash")
            original(root, path, value, kind)
        with mock.patch.object(task, "atomic_json", side_effect=fail_ledger):
            with self.assertRaisesRegex(OSError, "bootstrap crash"):
                task.initialize(self.root, SimpleNamespace(provider="codex", model=None, request_id="init-repair"))
        repaired = task.initialize(self.root, SimpleNamespace(provider="codex", model=None, request_id="init-repair"))
        self.assertEqual(repaired["last_operation"]["type"], "bootstrap")
        self.assertEqual(task.load_pair(self.root)[1]["revision"], 1)
        self.assertEqual(task.read_events(self.root)[0]["event_id"], "init-repair")

    def test_stale_lock_requires_explicit_recovery_and_audit_event(self) -> None:
        lock = self.root / ".doofus" / ".mutation.lock"
        lock.mkdir()
        (lock / "owner.json").write_text(json.dumps({"pid": os.getpid() + 1000000, "expires_at": "2020-01-01T00:00:00Z"}), encoding="utf-8")
        args = SimpleNamespace(provider="codex", model=None, confirm=False, note="", request_id="lock-recovery")
        with self.assertRaisesRegex(task.TaskError, "requires --confirm"):
            task.recover_lock(self.root, args)
        args.confirm = True
        args.note = "Verified the recorded process is absent before removing its stale local lock."
        with mock.patch.object(task.os, "kill", side_effect=ProcessLookupError):
            result = task.recover_lock(self.root, args)
        self.assertTrue(result["recovered"])
        self.assertFalse(lock.exists())
        self.assertTrue(any(event["event_type"] == "task.lock_recovered" for event in task.read_events(self.root)))

    def test_schema_and_task_graph_reject_corruption_and_unknown_dependencies(self) -> None:
        ledger = task.read_json(self.root, self.root / ".doofus" / "task-ledger.json", "task-ledger")
        ledger["tasks"][0]["dependencies"] = ["does-not-exist"]
        with self.assertRaisesRegex(task.TaskError, "unknown dependencies"):
            task.validate_document(self.root, "task-ledger", ledger)


if __name__ == "__main__":
    unittest.main()

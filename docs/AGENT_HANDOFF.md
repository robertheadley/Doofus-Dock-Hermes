# Agent handoff and task recovery

This repository uses portable, versioned records under `.doofus/`. They contain task state and evidence references, never conversation transcripts, credentials, machine-specific absolute paths, or a local SQLite database. The active milestone is a usable primary AI coding agent in native Rust/DoofusDock, proven by a complete real coding assignment through Hermes Native. The 24–72 hour window is an aggressive target, not an ETA. Preserve the broader parity and architecture objectives as later milestones; do not discard working features. Release scope, current blockers, and evidence dimensions are in `apps/hermes-native/docs/DELIVERY_OBJECTIVE.md`.

## Startup procedure for every provider

1. Read this file, `apps/hermes-native/docs/PLAN.md`, the relevant backlog, and the pinned requirement row(s).
2. Run `python apps/hermes-native/tools/doofus_task.py validate` and `status --json`.
3. Inspect `git status`, HEAD/branch, the listed changed files and their recorded owners. A reported worktree drift requires inspection and an explicit `resume --reconcile --reconcile-note ...`; never overwrite changes just because a handoff names a recipient.
4. Inspect unresolved prepared operations and active/external process records. Reconcile each operation using disk evidence and `doofus_task.py reconcile`; a timeout, missing response, or conversation ending is not success or proof that a process stopped.
5. Confirm the intended recipient, authorization scope, pending approvals, plan revision, task dependencies and current lease. Handoff never grants new permissions.
6. Take ownership using `python apps/hermes-native/tools/doofus_task.py resume --as codex|antigravity|hermes`. Supply `--model` only when known. Record a replacement-worktree inspection with `--reconcile` and a truthful note when Git state differs.
7. Continue the first uncompleted dependency-ready task. Before a meaningful boundary, checkpoint exact commands, evidence references, failures, blockers and the next action. Evidence references use repository-relative `path#locator` form and are validated against disk.
8. Run `progress` to report implementation, automated evidence, live behavior, visual pairing and formal acceptance separately. Do not convert coverage or tests into a parity completion percentage.
9. Before stopping or switching provider, checkpoint, then run `handoff --provider <current> --lease-id <lease> --to <codex|antigravity|hermes> --note <reason>`. Copy the generated prompt. That operation relinquishes the lease; a recipient must explicitly resume. Do not claim the recipient actually received or resumed it without a receipt.

The isolated-repository test suite exercises all six directed Codex/Antigravity/Hermes provider pairings, requiring an explicit resume after each relinquishment. It also simulates Codex → Antigravity → Hermes without conversation history. This validates the portable protocol, not delivery or takeover by a live external provider.

## CLI

```text
python apps/hermes-native/tools/doofus_task.py status [--json]
python apps/hermes-native/tools/doofus_task.py validate
python apps/hermes-native/tools/doofus_task.py checkpoint --provider codex --lease-id LEASE --summary TEXT --next-action TEXT --test COMMAND --test-result "PASS ..."
python apps/hermes-native/tools/doofus_task.py process record --provider codex --lease-id LEASE --process-id subagent:build --kind delegated_agent --state unknown --note "Verify whether the delegated agent is still active."
python apps/hermes-native/tools/doofus_task.py operation record --provider codex --lease-id LEASE --operation-id build-check-1 --kind cargo_check --state running --note "Build is running in the isolated worktree."
python apps/hermes-native/tools/doofus_task.py task update --provider codex --lease-id LEASE --task-id hermes-par-0002 --state tested --evidence apps/hermes-native/src/app.rs#test_name
python apps/hermes-native/tools/doofus_task.py task select --provider codex --lease-id LEASE --task-id hermes-par-0002 --note TEXT
python apps/hermes-native/tools/doofus_task.py handoff --provider codex --lease-id LEASE --to antigravity --scope-task-id hermes-par-0002 --note TEXT
python apps/hermes-native/tools/doofus_task.py handoff --provider codex --lease-id LEASE --to antigravity --scope-task-id hermes-par-0002 --note TEXT --preview
python apps/hermes-native/tools/doofus_task.py resume --as antigravity
python apps/hermes-native/tools/doofus_task.py progress [--json]
python apps/hermes-native/tools/doofus_task.py reconcile --operation-id ID --outcome committed|failed --note TEXT --provider codex
python apps/hermes-native/tools/doofus_task.py recover-lock --provider codex --confirm --note TEXT
```

`init` is a one-time bootstrap operation and refuses to replace existing state; retry with the same bootstrap request ID repairs only a missing initial projection. `checkpoint`, `task select` and `task update` require the active provider and exact lease. Task selection and mutation must stay inside the owner's recorded subtree; delegated handoffs may narrow that subtree but cannot widen it. Selection also respects dependency completion. Tested/verified task states require a real file locator. Formal acceptance can only follow the pinned source-pair CSV receipt. A stale or mismatched owner cannot write. If a multi-file task update is interrupted, revision mismatch blocks further writes until `reconcile` proves and records the outcome. Lock recovery is explicit, requires a dead process and a note, and refuses to remove a lock whose process still exists. Handoff preview prints a copyable prompt with `status: preview`, the intended recipient, and `ownership_transferred: false`; it does not change the lease or claim delivery. Only a non-preview handoff relinquishes the lease, and only recipient resume records a receipt.

## Record ownership

- `.doofus/handoff.json` is the latest atomic checkpoint and provider lease.
- `.doofus/task-ledger.json` is the versioned hierarchical task tree, including every fixed PAR requirement.
- `.doofus/events.jsonl` is a bounded, append-only JSONL audit of prepared, committed and explicitly reconciled operations.
- `.doofus/schemas/*.schema.json` are the versioned data contracts.
- `apps/hermes-native/docs/status/IMPLEMENTATION_PROGRESS.csv` remains the evidence-backed reference inventory overlay; handoff tooling reads it but does not change source acceptance. The `progress --json` `parity_scope.total` field is a legacy count of 499 source rows, not the agent-workflow completion denominator. `tasks.total` counts all hierarchy nodes, including project and source-requirement nodes; it is not a feature-completion measure. Reviewed-row coverage is evidence coverage, not product completion.

The current project has no measured phase-specific task-size/effort sample set. Therefore remaining-effort ETA is **unavailable / insufficient evidence**. A phase-aware EWMA with alpha 0.2 becomes eligible only after at least three completed, measured, size-normalized samples for that phase. The report includes a qualitative confidence and uncertainty explanation; an eligible EWMA is still labeled `limited` because duration variance and external acceptance work are not calibrated. Do not use the 33/59 checklist or 499-row reference inventory as elapsed-time or effort data.

## Known protocol limits

The portable records and CLI implement explicit, manual checkpoints and handoffs; application-boundary and periodic checkpoint triggers are not automatic yet. Process and external-operation states can now be recorded by stable ID, shown in status, and preserved across handoff, but the CLI does not independently discover provider-owned processes. Unknown/running entries are highlighted for reconciliation; never infer they stopped. Critical-path mapping and effort estimates remain unavailable until authoritative dependency/size data and at least three measured phase samples exist. The simulated provider chain validates repository recovery only; it does not deliver a handoff to a live Antigravity or Hermes agent.

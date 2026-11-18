# Agent task and execution contracts (v1)

These interfaces prepare Hermes Native for a future Doofus host without implementing recursive orchestration. The provider-neutral task and event records live under `.doofus/`; SQLite/URNE may later project these records, but the tracked JSON and JSONL remain sufficient to rebuild that projection.

## Ownership boundaries

| Component | Future responsibility | Current contract boundary |
|---|---|---|
| Doofus | Canonical requirements/tasks, planning, delegation, permission policy, verification and acceptance | `task-ledger.json`, checkpoints, typed events and acceptance evidence are portable inputs; no Doofus service dependency is assumed |
| DoofusDock | Native Rust UI, canonical layout, docking, resizing, morphing, rendering | Existing Hermes Native UI and layout work remains on the pinned DoofusDock revision |
| URNE / SQLite | Hierarchical task projection, events, ownership/provenance/evidence queries | Stable task IDs, parent IDs, dependency IDs, event IDs/revisions and artifact references are projection-friendly; no database file is committed |
| Hermes | Session/tool execution and provider-specific runtime adapter | Existing Python gateway remains authoritative and is not replaced |
| Codex, Antigravity, other providers | Task execution under a provider lease | Provider/model identity is explicit and nullable when unknown; a handoff is a relinquished lease, not proof of delivery or permission |
| MATC and capability registry | Compile provider-neutral tasks to verified provider-specific prompts/settings | Versioned capability negotiation, constraints and outputs are defined below; model-specific prompt text stays outside canonical task state |
| Provenance engine | Attribute incorporated source, license, version/commit, modifications and evidence | Existing source pins, `Cargo.lock`, vendored license files and `assets/fonts/README.md` remain inputs; this phase does not implement a full engine |

## Provider-neutral task envelope

Schema version 1 task nodes carry `task_id`, `parent_task_id`, provider/model identity, title/objective, state, owner/lease, dependencies, typed input/output references, verification evidence, progress, remaining-effort/size fields, and timestamps. States are `planned`, `in_progress`, `implemented`, `tested`, `verified`, `accepted`, `blocked`, `failed`, or `unknown`. The active manifest lease binds to a task subtree; a handoff may narrow that scope, never widen it, and task selection checks dependency completion. Capability declarations and negotiation are records with an explicit provider/model identity, declared capabilities, verification source/time, and unsupported/unknown outcomes; a provider name alone proves no capability.

Execution constraints are carried separately from task content: authorized scope, permission/approval references, resource bounds, allowed operations and expiry. Inputs and outputs use repository-relative artifact references with locators or content hashes; large transcripts are never copied into task state. Expected outputs specify artifact type, schema version, required fields and validation command. Verification criteria name the evidence and its scope. Handoff semantics include source owner/lease/revision, recipient, relinquishment, checkpoint revision, reconciliation status and failure/uncertainty state.

## Event and operation contract

Each event has a stable event ID, event type, prepared/committed/reconciled phase, monotonically increasing state revision, timestamp, provider/model/task identity and structured details. Mutations use request IDs for idempotency, an exclusive short-lived local lock, expected-revision checks, atomic replacement and explicit recovery. A prepared event without a terminal receipt is uncertain. Recovery records a human/provider evidence-based resolution; it never assumes the attempted action succeeded. Isolated tests cover all six directed Codex/Antigravity/Hermes handoff pairs plus a no-transcript three-provider continuation chain; these are protocol simulations, not proof of live provider receipt.

The CLI progress report exposes task totals and states, implementation review, automated evidence, live-runtime, visual-pair and formal-acceptance counts separately. Effort estimates carry a qualitative confidence and uncertainty statement. Missing throughput samples or task-size estimates produce an unavailable ETA and identify missing task sizes; a calculable EWMA is marked limited until duration variance and external acceptance effort are measured.

Provider-owned processes and external operations are explicit stable-ID records in the handoff manifest. Lease-checked record commands update their state through the atomic event protocol; status flags `running` and `unknown` records for reconciliation, and handoff preserves them. The CLI records observations supplied by an agent; it does not claim automatic OS or provider process discovery.

## MATC extension point

The compiler input is `{schema_version, task_id, objective, authorized_scope, capabilities_required, execution_constraints, input_artifacts, expected_output_contract, verification_criteria, handoff_policy, failure_policy}`. A provider adapter returns `{provider, model, capability_match, unsupported_capabilities, compiled_instruction_ref, execution_settings_ref}`. Canonical task state retains provider-neutral fields and artifact references, not provider-specific prompt formatting. Capability matching is `verified`, `unverified`, `unsupported`, or `unknown`, each with evidence; no model capability is inferred from a label.

## Provenance contract

Any future incorporated component is attributable by source repository, license identifier and evidence, dependency/package version, source commit or immutable release, local modification reference, and attribution artifact. Current pins and notices remain authoritative; no dependency may be relabeled as verified without inspecting its actual source/license evidence.

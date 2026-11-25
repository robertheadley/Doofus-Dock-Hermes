# Hermes Native immediate delivery objective

Updated: 2026-11-25

## Active milestone

Deliver a functional, reliable native Rust/DoofusDock Hermes application that the operator can use as their primary AI coding agent. The immediate acceptance target is one complete real coding assignment from the Hermes Native UI, without requiring the original Hermes Desktop application. The stated 24–72 hour window is an aggressive target only; it is not a verified ETA and does not waive correctness, safety, or validation.

## Minimum usable release scope

- Select a local Hermes profile and connect to an already-running local or supported remote Gateway.
- Select providers/models and configure session model choices.
- Create, open, resume, rename, and manage conversations; stream answers and render Markdown, code, reasoning, and tool results.
- Run tools through Hermes, show progress/results, answer approvals and clarification requests, stop work, and recover from common errors.
- Select a project/working directory and attach files or images.
- Discover/select/manage Skills; use Hermes backend plugins and MCP integrations.
- Keep concurrent session state isolated and preserve DoofusDock layouts.
- Reach relevant terminal output, files, and artifacts; restart and resume sessions reliably.
- Produce a usable packaged Windows executable.

Keep existing functional behavior. Treat the detailed feature ledger as supporting evidence and implementation tracking, not as a product-completion percentage. Prefer a small number of end-to-end workflows that exercise several capabilities together.

## Release acceptance and evidence

The core acceptance scenario launches the native app, attaches to an isolated Gateway profile, selects a working model/provider, creates a project conversation, sends a coding task, observes a streamed response and tool execution, handles any approval/clarification, verifies changed files and tests, and resumes the conversation after a restart. Keep each result separated into implementation, automated evidence, live runtime, packaged Windows, and release acceptance.

For every active blocker record description, dependency, acceptance criteria, effort basis, status, and critical-path status. ETA stays unavailable until there are measured, comparable work samples; do not infer it from feature counts, commits, tests, or prose volume.

## Current critical path and effort basis (2026-11-25)

| Gate | Status and dependency | Acceptance evidence | Effort basis | Critical path |
|---|---|---|---|---|
| Usable model/provider | **Blocked**: isolated profile's configured DeepSeek `deepseek-flash` returned HTTP 402, insufficient balance. Depends on provider account entitlement or an already-configured usable provider. | A plain prompt completes in Hermes Native against the isolated Gateway without an error. | **Unestimated**: external account state; no comparable engineering throughput sample. | Yes; blocks any real tool workflow. |
| Real coding assignment | **Not accepted**: prompt reached the provider, but the turn failed before a tool call; the controlled project files did not change. Depends on usable model access. | Stream a response, observe at least one Hermes tool call/result, change the isolated project's `calc.py`, and pass `python -m unittest -q`. | **Medium, high uncertainty**: UI and Gateway attach/session plumbing work, but live model/tool execution has not succeeded. | Yes; primary release acceptance. |
| Skills, backend plugins, and MCP | Native paths and headless evidence exist; live profile workflows are unverified. Depends on a usable model for chat invocation; inventory/config checks can proceed separately. | Discover and invoke one no-secret Skill; exercise one safe Hermes Agent Plugin operation; add/test/invoke/remove a local read-only MCP fixture. | **Small–medium** for Skills/plugins and **medium** for MCP, based on existing code/tests; not elapsed-time estimates. | Required release scope; after the coding gate. |
| Restart, recovery, and concurrent sessions | Headless evidence exists; cold restart/resume and concurrent live-session acceptance remain open. Depends on a completed live session. | Restart the app, resume without duplicate submit, verify transcript/artifacts, then prove two isolated session owners. | **Small–medium**, with live behavior uncertainty. | Required release scope; after the coding gate. |
| Portable Windows package | No release executable or packaging workflow is present yet. Depends on a release build and adjacent PDFium asset; WebView2 must be available on the launch target. | Build a portable folder, cold-launch it with the original Hermes Desktop closed, and connect/resume through Hermes Native. | **Medium** for portable folder and cold-launch; installer/signing work is not specified. | Yes, final release gate. |

The 24–72 hour window remains an aggressive target, not a verified ETA. The external provider gate is currently unestimated and therefore prevents a defensible total ETA.

## Deferred until after the first usable release

Unless a concrete dependency blocks the acceptance scenario: Electron Desktop plugin compatibility; a comprehensive Rust-native Desktop plugin host; advanced delegated-worker UI; voice capture/transcription; pixel parity; exhaustive interaction edge cases; unrelated Desktop administration/cosmetic refinement; and formal source-paired acceptance for every historical requirement. Backend plugins and MCP remain in the release scope.

## Preserved longer-term objectives

Continue to preserve the prior broad Hermes functional-parity objective against the downloaded pinned source, complete DoofusDock integration, later invert Hermes into a Doofus-hosted execution backend, support alternative workspace layouts/layout morphing, build an extensible native plugin architecture, provide hierarchical URNE task visualization and agent orchestration/delegation, maintain software provenance/attribution, and retain existing Doofus architecture requirements. These are later milestones and do not delay the first usable agent release.

## Current verified state (2026-11-25)

- The current Hermes Native automation executable compiled and launched headlessly against an isolated local Hermes Gateway. The profile-scope fix is present; the live prompt reached the configured model.
- The controlled coding assignment was attempted at 2026-11-25T17:25:29Z. DeepSeek returned HTTP 402 (`Insufficient Balance`) for `deepseek-flash` before tool execution. The assistant error appeared in the native transcript; no file changed and the unit test was not run. This is a provider blocker, not a UI approval or tool result.
- The live agent runner also surfaced a separate repository-discovery diagnostic because Hermes did not report its installed Python runtime. This is recorded for follow-up, not assumed to have caused the model failure.
- The Terminal Dock panel now offers an explicitly started local PTY, configurable working folder, command input, output, resize, interrupt, and stop. Windows PTY tests prove command output and clean exit; the terminal view is line-oriented, and full-screen character input/emulation plus packaged-app acceptance remain open.
- The source workspace has pre-existing operator-owned continuity state, screenshots, and work artifacts. Preserve them and stage only files explicitly owned for this task.


## Latest verified state (2026-11-25 20:28 UTC)

- The isolated live coding attempt used `deepseek-flash` and failed with HTTP 402 Insufficient Balance before any tool call. The persisted session has two messages, zero tool calls, and an unchanged fixture; the prompt was not resubmitted.
- Native attach now merges canonical live and stored session scopes, preserving legacy TUI behavior. Headless runtime readback shows five sessions and successfully opens the previously hidden stored session with its two-message transcript.
- The new-chat model picker exposes provider-qualified model names. Headless state readback verifies `deepseek` / `deepseek-chat` with `source=user_selected`; this proves selection only, not account availability.
- The full workspace suite passed with 777 Hermes Native library tests; all-target check, automation build, runner test, formatting, diff check, and canonical task validation passed.
- Session-recency HTTP 422 is resolved: the pinned `/api/sessions` contract caps `limit` at 100, so native startup/refresh now requests 100 instead of 200. Focused regression, full workspace tests, all-target feature check, automation build, format/diff checks, and a headless live Gateway readback pass; the app reports five sessions with no load error. Workspace repository-discovery settings now load from the pinned Gateway after correcting config profile scope; the saved profile has discovery disabled and no roots, so scan execution remains unverified. Provider-backed coding acceptance (prior HTTP 402), restart/resume after a successful turn, packaged Windows behavior, concurrent sessions, and remaining feature workflows remain open.

## Visual direction for the post-feature-parity pass

Use the operator-provided Doofus Design Guide as the visual target: warm charcoal and restrained earthy accents; compact, information-dense desktop geometry; cohesive docked workspaces; semantic user/agent/status colors; IRC-style conversation identity; keyboard access, reduced motion, and theme/function separation. Treat the guide's exact color and size tokens as proposed values to validate against DoofusDock APIs and contrast requirements. Preserve the operator's prior explicit JetBrains Mono and Fira Code selection as the typography override. Visual polish remains after the agent-workflow release.
## Latest implementation update (2026-11-25 20:53 UTC)

- PAR-0083 now has a Sessions menu action that sends the pinned profile-scoped `session.workspace.move` RPC. It filters out the current project, the no-project bucket, rootless entries, and other profiles; only a matching confirmed receipt updates cwd and triggers project-tree refresh. Busy, denied, stale, malformed, and unknown outcomes leave rows unchanged with no automatic retry.
- Validation: five focused project-move tests passed; the previously timed-out image-paste test passed in isolation; the complete Hermes Native library suite passed 788/788 on rerun. The first full run's one egui Harness max-step timeout is recorded as a transient flake. Doofus task records validate at revision 1070.
- Live project move, packaged Windows interaction, formal source-paired acceptance, and the provider-backed coding workflow remain unverified; the earlier provider HTTP 402 still blocks real tool execution.

# Hermes Native immediate delivery objective

Updated: 2026-11-27

## Active milestone

Deliver a functional, reliable native Rust/DoofusDock Hermes application that the operator can use as their primary AI coding agent. The first real coding-assignment gate passed once through Hermes Native on 2026-11-26. The active delivery target remains a reliable primary agent workflow across provider/model selection, chat, tools, Skills, plugins/MCP, recovery, and a usable Windows package; one successful run does not prove those broader capabilities. The stated 24–72 hour window is an aggressive target only; it is not a verified ETA and does not waive correctness, safety, or validation.

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
| Usable model/provider | **Verified once**: the dedicated OpenRouter key drove a complete live coding assignment with `deepseek/deepseek-v4.1-flash-1017` on 2026-11-26. Existing-session model switching and broad provider reliability remain open. | `artifacts/agent-ui/live-coding-result-2026-11-26T20-49-37-265Z.json`; persisted Gateway session; `docs/EXECUTION_LOGS.md#hermes-native-openrouter-profile-scope-all-targets-20261126T2340Z`. | **Unestimated**: one run is not a throughput sample. | No longer blocks the first coding workflow; model switching remains a core feature gap. |
| Real coding assignment | **Passed once**: Hermes streamed a live coding turn, ran tools, changed only the isolated `calc.py`, and the four fixture tests passed. Cold restart/resume and repeated reliability are still open. | `artifacts/agent-ui/live-coding-result-2026-11-26T20-49-37-265Z.json` and paired live coding capture; `python -m unittest -q` result is recorded there. | **Unestimated**: only one end-to-end sample. | First-run gate passed; recovery and packaging remain release gates. |
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
- The Terminal Dock panel offers an explicitly started local PTY, configurable working folder, command input, output, resize, interrupt, and stop, plus a collapsed-by-default Hermes Agent output section. Expanding Agent output shows its process cards and read-only streaming output immediately; the section loads bounded `process.list` snapshots and merges them with live session-scoped process events. Windows PTY tests prove command output and clean exit; the local terminal remains line-oriented, and full-screen character input/emulation plus packaged-app acceptance remain open.
- The source workspace has pre-existing operator-owned continuity state, screenshots, and work artifacts. Preserve them and stage only files explicitly owned for this task.


## Latest verified state (2026-11-25 20:28 UTC)

- The isolated live coding attempt used `deepseek-flash` and failed with HTTP 402 Insufficient Balance before any tool call. The persisted session has two messages, zero tool calls, and an unchanged fixture; the prompt was not resubmitted.
- Native attach now merges canonical live and stored session scopes, preserving legacy TUI behavior. Headless runtime readback shows five sessions and successfully opens the previously hidden stored session with its two-message transcript.
- The new-chat model picker exposes provider-qualified model names. Headless state readback verifies `deepseek` / `deepseek-chat` with `source=user_selected`; this proves selection only, not account availability.
- The full workspace suite passed with 777 Hermes Native library tests; all-target check, automation build, runner test, formatting, diff check, and canonical task validation passed.
- Session-recency HTTP 422 is resolved: the pinned `/api/sessions` contract caps `limit` at 100, so native startup/refresh now requests 100 instead of 200. Focused regression, full workspace tests, all-target feature check, automation build, format/diff checks, and a headless live Gateway readback pass; the app reports five sessions with no load error. Workspace repository-discovery settings now load from the pinned Gateway after correcting config profile scope; the saved profile has discovery disabled and no roots, so scan execution remains unverified. Provider-backed coding acceptance (prior HTTP 402), restart/resume after a successful turn, packaged Windows behavior, concurrent sessions, and remaining feature workflows remain open.

## Visual direction for the post-feature-parity pass

Use the operator-provided Doofus Design Guide as the visual target: warm charcoal and restrained earthy accents; compact, information-dense desktop geometry; cohesive docked workspaces; semantic user/agent/status colors; IRC-style conversation identity; keyboard access, reduced motion, and theme/function separation. Treat the guide's exact color and size tokens as proposed values to validate against DoofusDock APIs and contrast requirements. Preserve the operator's prior explicit JetBrains Mono and Fira Code selection as the typography override. Visual polish remains after the agent-workflow release.
## Latest implementation update (2026-11-25T21:16:13Z)

- PAR-0254 adds profile-wide Skill enable/disable and a disable-unused action. Updates use the full loaded inventory, independent of search, and serialize profile-scoped writes with confirmed receipts, rollback, stop-on-denial and stale-owner fencing.
- Validation: Skills-focused suite passed 34/34; full Hermes Native library suite passed 792/792; all-target Cargo check and rustfmt check passed. The check reports one existing dead-code warning in `gateway_runtime.rs`. No live Gateway mutation or packaged Windows interaction was performed.
- DoofusDock remains the reusable Rust/egui windowing layer: its workspace/docking owns window geometry and placement, while Hermes owns agent-specific panels and actions. This feature changes the Hermes Skills panel only.

## Previous implementation update (2026-11-25 20:53 UTC)

- PAR-0083 now has a Sessions menu action that sends the pinned profile-scoped `session.workspace.move` RPC. It filters out the current project, the no-project bucket, rootless entries, and other profiles; only a matching confirmed receipt updates cwd and triggers project-tree refresh. Busy, denied, stale, malformed, and unknown outcomes leave rows unchanged with no automatic retry.
- Validation: five focused project-move tests passed; the previously timed-out image-paste test passed in isolation; the complete Hermes Native library suite passed 788/788 on rerun. The first full run's one egui Harness max-step timeout is recorded as a transient flake. Doofus task records validate at revision 1070.
- Live project move, packaged Windows interaction, formal source-paired acceptance, and the provider-backed coding workflow remain unverified; the earlier provider HTTP 402 still blocks real tool execution.


## Latest runtime verification (2026-11-25T22:57Z)

- The freshly built headless Hermes Native process attached to the isolated local Gateway, opened the saved test session (two transcript messages), opened the Terminal Dock, and expanded the collapsed-by-default Agent output section. The resolved UI displayed “No background process output is available for this session yet.” with no process-list/snapshot error and no loading indicator. No model prompt was sent and no screenshot was taken.
- This verifies the live Gateway snapshot request and empty-result presentation only. A populated process, live output tail, snapshot-to-stream merge under an active process, packaged Windows workflow, and pinned-source acceptance remain open. The broader coding acceptance remains blocked by the configured provider’s HTTP 402 credit response.
- DoofusDock remains the reusable UI/windowing library; the Agent output section and its process/session semantics belong to Hermes.


## Latest dependency and process contract check (2026-11-25T23:02Z)

- Hermes Native pins all four DoofusDock crates (`core`, `host-egui`, `presentation`, `render-egui`) to `f23ed02a99c7f8c57f83164d98feadb17cc4ed87`; a read-only remote ref check found DoofusDock `main` at the same commit. No dependency refresh is currently needed.
- The pinned Hermes Gateway provides background-process list/stop/kill operations, but no process-spawn RPC. Spawning comes from Hermes' `terminal(background=true)` agent tool, so producing a populated live process pane without a model/tool turn is unsupported. No synthetic process was injected.


## Portable package candidate (2026-11-25T23:24Z)

- A local source-qualified Windows x64 bundle was built at `artifacts/releases/hermes-native-windows-x64-1765d78f01a9`. It contains `hermes-native.exe`, `hermes-automation.exe`, the pinned PDFium DLL, `HermesPage`, provenance/hash metadata, and the license files present in this checkout. The app itself declares MIT in Cargo metadata, but this checkout has no standalone Hermes Native license text; the bundle flags that gap.
- Headless validation confirms the packaged native helper exposes `hermes-native-window/1`, renders the New session route with Settings visible, and closes cleanly. The main binary rejects an invalid child-window identity before opening a window. No screenshot, profile, credentials, Gateway prompt, or model call was used.
- A cold visible launch, packaged live-Gateway connection, and provider-backed coding assignment remain unverified. The package is local and tied to the source commit in its `PACKAGE.json`; it is not a signed installer.


## Portable package refresh (2026-11-25T23:43Z)

- Rebuilt the Windows x64 package from published source commit `e6b2653c351f01fee133dcaf7b6ca5a2bbd019ae` at `artifacts/releases/hermes-native-windows-x64-e6b2653c351f`. The package manifest pins Hermes `8783054ef6cbafd86f10892241436862c1171a66` and DoofusDock `f23ed02a99c7f8c57f83164d98feadb17cc4ed87`; all four file lengths and hashes validate, and no user profile or credentials are included.
- The packaged off-screen `HermesPage` smoke reached route `/`, showed Settings and Terminal controls, and closed cleanly. No screenshot, Gateway prompt, or model call was used. This does not prove a visible cold launch, live Gateway attachment, or provider-backed agent coding.


## Latest live agent-workflow evidence (2026-11-27)

A disposable profile check verified MCP stdio credential persistence, the authenticated probe, runtime startup/reload and tool discovery. The profile-scoped synthetic secret was read back from `.env`, while `config.yaml` retained only an environment reference; `tools.list` exposed the registered local tool and runtime status reported it connected. The test Gateway and local MCP child were stopped and verified absent. This check did not invoke the tool through a model turn or drive the app's credential UI. The dedicated OpenRouter key is available for a later controlled agent-turn test, but it was deliberately not forwarded to the MCP-only Gateway check. See `artifacts/agent-ui/mcp-live-profile-acceptance-20261127.json`.


## Latest MCP implementation update (2026-11-27)

The stdio Add form now separates profile environment passthrough names from explicit environment values. Exact imported `${NAME}` self-references become passthrough rows and serialize back to `${NAME}`; names are validated, duplicate passthrough names are rejected, and the original source precedence is retained if an explicit assignment uses the same name. Add/Remove controls are accessible and covered by rendered tests. The feature remains partial until the native UI save path and model-driven tool invocation are verified; live profile backend evidence is recorded separately in `artifacts/agent-ui/mcp-live-profile-acceptance-20261127.json`.


## Live MCP agent-turn evidence (2026-11-27)

One fresh Hermes Native chat used OpenRouter `deepseek/deepseek-v4.1-flash-1017`, called the local synthetic MCP tool once, and displayed the actual `MCP_PROBE_OK` result. The app reported a connected isolated Gateway, selected model, completed transcript, and no pending approval. This verifies one working model-to-MCP loop. It does not verify the native credential-save flow, OAuth, broad provider reliability, packaged Windows behavior, recovery, or source-paired acceptance. See `artifacts/agent-ui/mcp-agent-turn-20261127.json`.

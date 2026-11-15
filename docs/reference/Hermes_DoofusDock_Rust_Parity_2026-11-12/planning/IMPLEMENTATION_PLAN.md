# Implementation plan: execution gates rather than time estimates

**Read `PROMPT_FOR_HERMES.md` before acting. No time estimates.** The implementation is multi-phase and evidence-gated, not a single giant UI rewrite.

## Gate 0 — version and scope freeze (read-only)
- Obtain user acceptance of plan through normal Hermes plan gate; do not treat this package as permission to change repositories.
- Refresh `NousResearch/hermes-agent`, `robertheadley/Hermes-Doofus`, `robertheadley/doofusdock`, `robertheadley/skills`; create read-only tree and hash snapshots. Record changes since SHAs.
- Enumerate **every actual Desktop route, panes/components, Desktop SDK registration area, gateway RPC/event/server-request schema, Doofus plugin route, bridge endpoint, and relevant DoofusDock public API**. Map to IDs in `maps/full_program_capability_map.json`.
- Update this baseline inventory: split overly broad rows, add unlisted observable behaviors, identify unsupported/obsolete components; preserve IDs and evidence history. Distinguish source feature absence from uninspected source.
- Produce threat model, acceptance rubric and integration option decision (native protocol client vs isolated intermediate process); all critical decisions recorded before implementation.

**Gate:** scope coverage table reviewed; unknowns recorded, no missing UX support functions; no repo mutations until plan acceptance.

## Phase 1 — native app shell and DoofusDock adapter
- New isolated Rust application crate; link existing DoofusDock crates by compatible workspace/path or pinned Git revision after inspection.
- Build canonical workspaces with `DockingService`, stable pane IDs, `doofusdock-render-egui` tab strip, themed panel chrome, group intentions and `doofusdock-host-egui` viewports.
- Implement responsive template and custom layout profiles; persist layout once through Dock's atomic API, not a secondary tree.
- All existing DoofusDock physical host uncertainties (multi-monitor, mixed DPI, drag interruption, focus/accessibility) get repro tests or marked blockers.

**Gate:** native launch, drag/drop, float/re-dock, resize, save/restore, undo/redo; no fake content or conditional silent swallow.

## Phase 2 — gateway and bridge transport
- Implement official Hermes attach, one-use tickets, `/api/ws` transport, JSON-RPC ID map, `runtime.describe`, capability negotiation, typed event reducer and bounded retry/backoff.
- Support session list/create/activate/transcript, streaming, tool progress, session interrupt, config and model routes. Reconstruct in-flight state from authoritative history; no duplicate prompts.
- Implement full server-initiated request matrix: approvals, clarify, sudo, secret, vault, MCP/connection prompts and desktop-bridge requests. Unimplemented methods return valid fail-closed errors.
- Implement authenticated Doofus bridge adapter; bootstrap through a **supported per-profile capability**, never copy `bridge.token` to arbitrary renderer-visible storage. SSE cursor/gap/resnapshot and status freshness model.

**Gate:** recorded sanitized protocol fixtures; attach/reconnect, out-of-order/duplicate/stale event tests, profile switch isolation and malicious/malformed RPC cases pass. Live test with two clients attached to the same session.

## Phase 3 — chat and session core
- Virtualized transcript, Markdown/code/diff/tool cards, timeline rail, message editing/regen with destructive `prompt.submit` confirmations, composer, attachments and durable session history.
- Session creation, rename, pin/reorder, archive, fork, search and scoped multi-profile/gateway navigation.
- Explicit empty, loading, failed, stale, retry, interrupted and busy-input states; strong keyboard path and accessibility.

**Gate:** chat parity with ongoing tool calls, complete replay, model swap, image upload, cancellation and rewind guarded by gateway semantics; no dropped or duplicated output.

## Phase 4 — files, previews, artifacts and terminals
- Lazy filesystem tree, preview types, artifact gallery, scoped downloads, editor read-only view, MIME/file-size handling and explicit unsupported file presentations.
- Terminal tabs with real PTY/remote process ownership, ANSI/Unicode, selection, scrollback, resize, copy, copy-to-chat and persistence through hide/restore and gateway changes.
- Browser/HTML previews: explicitly choose bounded sandbox and native implementation, preserve state while hidden.

**Gate:** close vs hide tests, remote file origin tests, large terminal streams, alternate gateway tests and no shell recreation on Hide.

## Phase 5 — Git, worktrees, background tasks and subagents
- Git review (Uncommitted/Branch/Last turn), stage/unstage, diff, commit/push, branches, create PR, worktree lanes. Every destructive action needs explicit permission and backend proof.
- Active subagents, steer/stop, task graph, activity and completion; same client event stream, not a parallel tracker.
- Background work must remain visible in a detached window or task view while execution proceeds server-side.

**Gate:** scoped Git operations and worktree migration receipts; agent steer means queued until confirmed; no fabricated progress or ETA.

## Phase 6 — models, profiles, skills and management
- Model quota/cost/capability browsing, favorites, sorting, role-specific routing, planner model; profile and gateway registry.
- Skill/toolset/MCP, messaging/Bot Mode, cron/webhooks, settings, logs, usage analytics, admin dashboard, onboarding and secure connection forms.
- Native plugin contribution registry; do **not** advertise arbitrary existing JS plugins as supported unless compatibility is implemented and tested.

**Gate:** mutating config/skills/plugins/channel controls only after proper authorization and configuration roundtrip. Profile-switch stale reads must not mutate previous scope.

## Phase 7 — Hermes-Doofus parity
- Rust-native Overview, Hierarchy, Testing, Tasks, Activity, Vault, Settings, Models & Quota; compact pane + full routes, data in one authoritative Doofus bridge client.
- Current task/requirements/audit semantics, canonical event seq/id, authoritative `needs attention`, bridge availability and stale markers, prompt lifecycle and acceptance evidence.
- Vault: redacted metadata, authorized issue/rotation/revocation flows, no secret value in logs, state caches or screenshots.
- Four-line verification status: separator / concise status content / evidence+blockers / separator; exact display may be user-configured.

**Gate:** event sequence replay/cursor-gap, demand closure only on positive evidence, no historical toast storm, status never exceeds auditor verdict, secure vault penetration checks.

## Phase 8 — hardening and UI support infrastructure
- Cross-platform architecture may be portable, but **Windows 11 is first acceptance target**. Validate native focus, DPI, monitor unplug, keyboard shortcuts, accessibility, IME, high-contrast and reduced-motion in real Windows environment.
- UI virtualization, bounded memory, renderer idle behavior, layout migration and crash recovery. Require report of actual measured results, no unsupported benchmark promises.
- OS notifications, voice permissions, Quick Entry/HUD, tray, localization, RTL and settings synchronization.

**Gate:** Windows app live smoke and accessibility/interaction receipts with actual host evidence, not just unit tests.

## Phase 9 — side-by-side parity signoff and optional cutover
- Run every `planning/ACCEPTANCE_MATRIX.csv` test against pinned old UI and new native UI, including negative paths and relevant selected settings.
- Validate complete feature matrix and **all** support functions. Exceptions require explicit owner acceptance, severity and follow-up ID; unknowns cannot be labeled verified.
- Produce signed source/version evidence, screenshots with nonsecret examples, resource/latency reports, security and license attribution, downgrade/rollback test.
- Only after approval provide a migration switch; preserve ability to return to old Hermes Desktop without data transformation or loss.

**Gate:** no unresolved P0/P1 parity gaps, no high-severity security regression, no incorrect claim status. Update UCM with real evidence files and run checker.

## Reusable acceptance protocol for every feature
```
Record: REQ-ID, capability-id, source UI path, target Rust UI path, source commit, target commit,
preconditions, user action, expected successful behavior, denied/offline/invalid path,
expected network call (if any), expected durable state owner, accessibility check,
result (pass/fail/skipped + reason), test evidence file, reviewer, date.
```

## Doofus Rules during implementation
Orthogonality, idempotency, typed contracts, atomic operations, fail fast, disk source of truth, auditability, zero-trust, pure core/impure shell. Use versioned event schemas, no silent fallback, truthful verification status, a plan-before-action gate, bounded test runs, and Software Provenance / Open Source Attribution Engine credit tracking.

## Non-goals unless explicitly approved
- Replacing the Hermes model runtime, tool invocation engine, MCP server, memory DB or Doofus truth layer with Rust.
- Importing JS React code into DoofusDock core or enabling arbitrary untrusted `eval`.
- Rewriting every web dashboard route as a network server; parity is the native UI surface.
- Changing live user configuration, vault data, sessions or project files during mock UI testing.
- Making a shiny animated skin while functional parity remains incomplete.

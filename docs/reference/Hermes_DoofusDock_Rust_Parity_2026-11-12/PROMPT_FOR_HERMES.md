# Copy/paste implementation prompt for Hermes / Codex

You are implementing a **native Rust replacement UI** for Hermes Desktop with the Doofus Hermes plugin, using `robertheadley/doofusdock` as the **canonical docking/layout foundation** and native egui application widgets. The existing Hermes agent, gateway, memory, skills, tools, and Doofus truth layer remain in charge; this is a **one-to-one functional replacement for the graphical UI**, not a mockup, re-skin, or new agent framework.

## Required inputs and repositories
- The complete ZIP contents, especially `skills/doofusdock-hermes-parity/SKILL.md`, `planning/IMPLEMENTATION_PLAN.md`, `planning/ARCHITECTURE.md`, and `maps/capabilities.yaml`.
- `https://github.com/robertheadley/doofusdock` (Rust docking foundation).
- `https://github.com/robertheadley/Hermes-Doofus` (Python truth layer and current `desktop-plugin/plugin.js`).
- `https://github.com/NousResearch/hermes-agent` (canonical gateway, desktop UI, SDK and management surfaces).
- `https://github.com/robertheadley/skills/tree/main/Universal%20Capability%20Map` (UCM mapping methodology/checker).
- Source snapshots and feature IDs are pinned in `maps/SOURCE_SNAPSHOTS.json`; refresh before implementing.

## PLAN-FIRST SAFETY GATE
Before making **any mutations**, inspect current repositories and produce a detailed plan using the available Hermes/Doofus plan mechanism. Record plan ID/revision, scope, requirement IDs, dependencies, risks, files to edit, relevant API validation, test strategy and rollback. **Do not create/edit/delete/commit/install/execute mutable operations until the user explicitly accepts this plan.** Read-only source reviews and pre-existing tests that cannot mutate user state may be run. Preserve already-running tasks and do not contend for their locks. No fabricated success or unverifiable completion claims. No time estimates.

## Assignment
1. Reconcile the 565 feature requirements from `maps/full_program_capability_map.json` with **current** upstream Desktop, Dashboard, TUI gateway and Hermes-Doofus code. Enumerate new/changed routes and widgets, the actual `apps/desktop/src` component/registration trees, gateway RPC schemas, server-initiated request types, plugin SDK contributions and Doofus bridge endpoints. Update capability map and requirement ledger without losing IDs/history. Treat unknown as unknown, not absent. Add any omitted behaviors rather than shrinking scope for convenience.
2. Build a composable **native Rust + egui + DoofusDock** application with a four-zone default layout: left file tree and task status, center chat with fixed composer, right previews/artifacts/Doofus Control/Models & Quota, bottom terminal/tests/logs. The exact arrangement is customizable, savable and detachable with canonical Dock operations. No Electron dependency for the native host.
3. Keep `LayoutDocument` authoritative for geometry and panel identity. Use public DoofusDock crate APIs, do not duplicate the layout graph. Implement egui domain widgets where DoofusDock only supplies layout. Use typed adapters for transport, reducers, UI, native OS and Doofus bridge. Egui remains rendering/input; backend decisions never live in widget callbacks.
4. Build the Hermes JSON-RPC gateway adapter using documented `hermes gateway attach --json`, single-use ticket, `hermes-gateway-v1`, `runtime.describe`, `client.capabilities`, request/reply IDs, `session.transcript` opaque cursors and event replay. Handle all server-initiated requests (approval, clarify, sudo, secret, vault, connection, terminal, preview, window) or fail closed with JSON-RPC error. Never treat a timeout as a successful action or automatically repeat a non-idempotent submission.
5. Recreate all Hermes Desktop user workflows: session panes/history/search, transcript/Markdown/streaming/tool cards/diffs, composer attachments, message edit/rewind, models, profiles/gateways, file viewer/tree, artifacts, terminal, Git review/worktrees, background tasks/subagents, skills/MCP/plugins, messaging/Bot Mode, cron, settings, analytics/diagnostics, voice, Quick Entry/HUD, keyboard shortcuts, theme, localization, accessibility and native Windows affordances.
6. Recreate the current **Hermes-Doofus** plugin UI in Rust: Overview, Hierarchy, Testing, Tasks, Activity, Vault, Settings, Doofus Settings, Models & Quota pane, right-side compact Control, canonical event/requirement inspector, route decisions, model capability evidence, notifications and recovery. Use the existing token-gated bridge and its current versioned schemas. Do not bypass session/profile security or expose vault secrets. Verification summary must fit max **four lines including two separator lines**; textual Observing/Intervening/Verified labels must reflect canonical evidence. Causal event seq/replay and positive evidence demand closure are mandatory.
7. Inventory **third-party Desktop Plugin SDK** contributions. JS/React plugins are not magically executable in Rust: define a robust native extension/compatibility policy; track unresolved contributions, do not claim full 1:1 plugin parity until they can run safely or the user has accepted exclusions. Preserve backend plugin RPC independently of rendered UI contributions.
8. Implement every UI-support function: keyboard/focus/selection, validation, errors, empty/loading/stale/retry, context menus, search/sort/filter, accessibility, high contrast, reduced motion, pagination, virtualization, clipboard, localization, responsive layouts, hide-versus-close lifecycle and correct window-ownership semantics. No fake buttons or inert controls.
9. Include a complete **source→target capability crosswalk**, a live result matrix with one test ID per behavior, source snapshot and target commit, positive and negative path, proof artifacts, real Windows smoke checks and security probes. Use the Universal Capability Map skill, append actual evidence under `maps/capabilities/evidence/`, and promote to `verified` only with real verification-level evidence. Never mark an untested behavior verified.
10. Maintain the **Doofus Rules** (orthogonality, idempotency, type safety, atomic ops, fail fast, disk truth, auditability, zero trust, pure core/impure shell). Maintain open source provenance/license credit. Build with source-compliant dependency licenses. Keep old Hermes Desktop installed as rollback path until full parity is accepted.

## Decision policy
- Prefer adapting stable upstream gateway protocols to duplicating backend state.
- Prefer DoofusDock canonical dock/profile/host APIs to a second layout engine.
- Prefer egui native controls to embedding a whole web app; document any justified sandboxed WebView exception before implementing it.
- Do not silently remove behavior to simplify implementation; every unsupported item remains visible in the map as `partial`, `unknown` or `not_implemented` with a gap explanation.
- Keep Rust shell, bridge, rendering, OS integration and pure state reducers in separate modules with typed schemas and bounded messages.
- Choose a small, coherent, independently testable slice per accepted work increment, but preserve the comprehensive parity plan.

## Required first response
A read-only readiness report: updated source SHAs, current vs pinned differences, list of actual gateway protocol seams, existing DoofusDock APIs and host gaps, plugin compatibility decision points, size of capability inventory and any missing feature IDs, complete phased execution plan, evidence/acceptance/rollback plan, and **explicit request for plan acceptance**. Do not implement or mutate until approval.

## Required implementation outputs AFTER plan approval
- Rust workspace with actual code and tests, reproducible Windows build/run instructions, no Electron dependency in native process.
- UCM map, source↔target crosswalk, versioned requirement ledger and proof/evidence index.
- Continuous documentation of changes, tests, execution logs and truthful completion status.
- Security/provenance reports and a side-by-side acceptance matrix with every missing behavior named.
- No time estimates. No silent omissions. No unverified claims.

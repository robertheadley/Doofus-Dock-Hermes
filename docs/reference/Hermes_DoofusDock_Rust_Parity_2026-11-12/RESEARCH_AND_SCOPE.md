# Scope, distinctions, and research findings

## The three Hermes user interfaces are not the same
1. **Hermes Desktop** is the reference full-workspace application (Electron/React in upstream, to be replaced with Rust/egui). It supports chat, tabs, windows, previews, file tree, terminal, worktrees, Git review, model/provider switching, profiles, skills, messaging, quick entry, voice, session and artifact management, etc.
2. **Hermes Web Dashboard** is an administrative web UI whose Chat tab embeds the actual Hermes TUI via PTY. It also has status, sessions, config, API keys, logs, analytics, cron, profiles, skills, MCP, channels, pairing, webhooks and system views. The dashboard's routes are included as management parity requirements, but replacing its web server is not a prerequisite to the native desktop being usable.
3. **Hermes TUI** is an independent terminal frontend. Preserve its RPC command semantics through the same gateway; do not try to reproduce ANSI TUI presentation merely because the web dashboard embeds it. An **optional** raw TUI terminal remains a compatibility surface.

## What is being replaced
- Replace the graphical desktop shell, React-based domain widgets, and the custom Hermes-Doofus desktop plugin's rendered views with native Rust/egui/DoofusDock versions.
- Preserve Hermes Python agent runtime, gateway, message delivery, sessions, memory, skills, MCP, cron, and installed backend plugins.
- Preserve Hermes-Doofus Python truth layer, bridge, evidence, vault and audit rules. UI is a client, not a second authority.
- Maintain an optional compatibility opening of old Electron UI while Rust parity is incomplete. **Do not disable the old UI based solely on smoke tests.**

## Source-observed Doofus Control specifics
`robertheadley/Hermes-Doofus/desktop-plugin/plugin.js` registers the following visible contributions:
- Doofus Settings route, Doofus Control (Overview, Hierarchy, Testing, Tasks, Activity, Vault, Settings) routes, and sidebar nav entries.
- A right `Models & Quota` pane and right `Doofus Control` pane.
- Models & Quota: provider catalog hierarchy, capacity/capabilities, benchmarking, pricing, per-provider quota, favorites, sorting and filters, mid-session model switching.
- Control: task/requirements ledger, canonical event/activity stream, hierarchy with persisted expansion, tests and background work, intervention inspector, vault management, routing decisions, diagnostics and notifications.
- Bridge current code includes token-gated routes for status, requirements, task control, graphs, events, activity, routing, provider, vault, settings, project and diagnostics. Some GET endpoints are intentionally unguarded for availability/bootstrap; validate the current exact gate in code.

## What DoofusDock actually supplies
`doofusdock-core`: canonical layout, operations, typed validated transactions, persistence, undo, profiles, extensions, URNE. `doofusdock-render-egui`: reusable panel shell/chrome, tabs, transients, vector/nine-slice skin, software renderer. `doofusdock-host-egui`: native viewport lifecycle, host geometry, focus signals. `doofusdock-presentation`: layout projection and effects. `doofusdock-audio`: optional audio clock/decoding; not required for a normal Hermes UI.

**Explicit DoofusDock gaps observed:** shader chrome/effects are `NotImplemented` in renderer capability reporting; broader physical accessibility, arbitrary child-widget focus, cross-platform cross-viewport dragging, mixed-DPI/multi-monitor/hot-plug behavior retain open acceptance items in `docs/BACKLOG.md`. Native Windows on one display has some evidence, but that cannot certify all host configurations. The core does not supply a Hermes gateway client, terminal, chat renderer, or React/JavaScript plugin compatibility.

## Why 'use Doofus Dock skill' is represented as a generated draft
The available installed ChatGPT skills and the scanned DoofusDock repo tree do **not** expose a distributable, versioned `Doofus Dock Skill` named as such. This package includes a **new project-specific skill draft** tied to the verified crate APIs. The coding agent must check for any newer official/project-maintained skill before installing or superseding it. The user-provided `Universal Capability Map` skill at `robertheadley/skills` *does* exist; its UCM status and evidence conventions are adopted.

## Completion definition
One-to-one parity means every in-scope feature has an acceptance case and a tested negative path, live source/target side-by-side result, permission-failure behavior, correct data owner, correct profile/session scope, accessibility behavior and native Windows outcome. A visual match alone does not pass. A working Rust shell with empty mock panes does not pass. Unsupported third-party plugin behaviors remain a tracked gap, not a silent omission.

# Approved implementation plan

- **Plan ID:** `HUR-001`
- **Approved scope:** revision 1; continuation authorized by the user on 2026-11-13.
- **Working revision:** 57 (implementation in progress).
- **Status:** in progress; do not describe this crate as a 1:1 replacement until all acceptance gates pass.
- **Fixed Hermes parity snapshot:** `NousResearch/hermes-agent@8783054ef6cbafd86f10892241436862c1171a66` (downloaded 2026-11-14; target remains fixed unless explicitly changed).
- **Current DoofusDock:** `robertheadley/doofusdock@f6eb49b9e488f4e851d8d3abe18e95d296201bba` (current `main` on 2026-11-13).
- **Scope:** stock upstream Hermes Desktop UI, native Rust/egui with DoofusDock; upstream Python Hermes gateway remains authoritative.
- **Excluded:** Hermes-Doofus plugin UI/backend, Doofus Vault/Control and the separate browser Dashboard.

## Downloaded parity source snapshot

The selected and downloaded target is commit `8783054ef6cbafd86f10892241436862c1171a66`. The source checkout remains isolated at `../../.staging/hermes-upstream-8358fc`; immutable local ref `refs/codex/parity/hermes-8783054` preserves the exact target. The source review that selected this snapshot found an `a7904dc..8783054` delta across 1,070 repository files, including 46 Desktop files and seven `tui_gateway` files. This historical selection audit is recorded for provenance; later upstream commits do not change the parity target. The preserved 565-row `docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12` package contains 499 in-scope stock Hermes Desktop requirements and 66 excluded Hermes-Doofus requirements. The 499 stock Desktop requirements define the fixed-snapshot acceptance target; the excluded rows remain pending in the research package for provenance only. The separate 491-capability package at historical pin `ffdc7eeb` is supplemental reference material and does not redefine this scope.

## Dependency-ordered stages and gates

### 0. Source and inventory control

- Preserve and validate the 565-row research package under `docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12`; reconcile the 499 in-scope Hermes Desktop rows to the fixed snapshot and retain the 66 excluded Hermes-Doofus rows as pending provenance. Keep the older 491-item package supplemental.
- Compare every source index/crosswalk route to the selected source SHA. Append revisions/evidence rather than silently dropping or relabeling requirements.
- Obtain stable source baseline captures for first-run, every route/overlay, layout and error/loading/offline states. The earlier upstream preview timed out; no paired visual signoff exists yet.

**Gate:** current pins and route/component evidence are recorded; unknowns remain explicitly unknown.

### 1. Native source shell

- Preserve the source layout trees and visibility semantics. Implement route classification, workspace pages, overlays and contribution-route reservation before building feature workflows.
- Match titlebar, sidebar navigation, session tabs, status, overlays, focused/hover states, window behavior, keyboard/accessibility and responsive breakpoints.
- Use DoofusDock's single canonical `LayoutDocument`, public profiles/transactions and renderer primitives. Real Hermes widgets own application state; the host does not duplicate the layout graph.
- Verify resize, drag/reorder/reparent, closed-pane reveal, save/restore and mode preference shadowing.

**Gate:** source and target route/layout state transitions match with paired Windows evidence. Placeholder pages do not satisfy this gate.

### 2. Gateway transport and session spine

- Read the current pinned gateway CLI, ticket, WebSocket, JSON-RPC, capability, replay and recovery contracts. Implement typed DTOs, credential-safe gateway discovery/attach, one-use tickets, version/capability negotiation, request correlation, bounded replay/reconnect and server-request replies.
- Preserve upstream request/admission IDs, profile/session scope, idempotency and source-of-truth. Implement new-session and session restore, sidebar history, transcript streaming, composer/drafts, stop, approvals, tool cards and model selection.
- [x] Render active-session `approval.request` events as accessible permission cards and answer only offered choices through generation-scoped `approval.respond`; reject stale, cross-session, duplicate and unavailable decisions. A local WebSocket fixture validates request scope and rejects a mismatched receipt. True server→client JSON-RPC request handling and live gateway acceptance remain open.
- [x] Render active-session single-question `clarify.request` events with source choices, free text, multi-select serialization, skip, and replayed-answer staging; send generation-scoped `clarify.respond` and validate its prompt receipt. Local WebSocket and AccessKit fixtures cover valid and mismatched receipts, stale/oversized inputs and offline controls.
- [x] Route active-session batch `clarify` server requests into an accessible editable card. Confirm sends per-question `clarify.lock` calls sequentially in upstream order, replayed values are staged for editing, and Skip cancels the request with an empty result. Actor and AccessKit fixtures cover request routing, lock receipts and visible controls. Capability advertisement remains false while other request handlers and request ownership are incomplete.
- [x] Render current-session `tool.start`/`tool.complete` events as generation-scoped transcript groups; match reused tool IDs within a generation, preserve start arguments, show completion output, and expose the full bounded row text to AccessKit. A headless fixture checks reused IDs across two generations and the rendered accessible tree. This is a basic event projection, not full upstream tool/reasoning card, out-of-order reconciliation, retry, or live-gateway parity.
- [ ] Handle upstream server→client JSON-RPC requests. Rust now recognizes a request frame during another RPC and returns `-32601` for an unsupported method, then continues the original call. The integration fixture covers the request/reply correlation and continued RPC; capability advertisement remains false until the supported request set, in-flight response routing and request ownership are complete.
- [x] Route the upstream `approval` request into the existing accessible permission card for the active runtime session and return `{choice}` on the original request ID. Unsupported choices and unshown sessions fail closed. Local actor and AccessKit fixtures cover same-socket response, disabled offline controls, and `allow_permanent=false`; capability advertisement remains disabled until remaining request handling is implemented.
- [x] Add first existing-session submit/event support and partial fresh-session create → initial submit flow. Local protocol fixture verifies create params and distinct runtime/stored session ID roles. This slice does not provide the full upstream create option/retry behavior, submission-ID server idempotency, server requests, reconnect, or full event reconciliation; keep the full bullet/gate open.
- [x] Add profile-scoped, bounded older-transcript page reads using `/api/sessions/{id}/messages?limit=120&offset=…&order=latest&include_compacted=true`. The accessible “Show earlier messages” control prepends verified pages, handles legacy complete-transcript responses, rejects wrong-order receipts, and ignores stale session/profile responses. Headless AccessKit and HTTP fixtures cover the control, profile query, offset/order contract and legacy fallback. Full scrolling/virtualization parity, scroll-anchor preservation, transcript retention, and live-gateway acceptance remain open.
- Never launch a duplicate Python owner, double-submit an uncertain prompt, echo credentials, or project stale events as live.

**Gate:** protocol fixture tests cover correct and adversarial/negative receipts, replay, unsupported capabilities, disconnect, restored delivery and profile scope; a live isolated gateway session passes before UI capability claims are updated.

### 3. Remaining stock Desktop workflows

- Implement, in dependency order: files/previews/artifacts; terminal and PTY lifetime; Git/worktrees/review; tasks/subagents; profiles/models/settings; skills/MCP/plugins; messaging/bots/scheduling/webhooks; memory/analytics/voice/HUD; command palette, notifications, diagnostics, accessibility, themes, native platform and security release behavior.
- Keep same panel identity and active resource through hide, layout changes, resize, detach and window changes. Mutating actions require explicit confirmation/failure semantics from upstream.
- Exclude arbitrary third-party React compatibility and the deferred plugin/dashboard rows unless later scope explicitly changes.


**Gate:** every in-scope row (499 Hermes Desktop requirements) is traced to the fixed source and implementation; functional and failure-path receipts exist for the applicable controls.

### 4. Agent-operated UI and background evidence

- [ ] Provide Playwright-like semantic locators for the native app: accessible role/name, stable test ID, visible/enabled state, bounds, and parent/child relationships.
- [x] Support click, text entry, selection, keyboard input, bounded pointer drag, wait-for-state, screenshot, and structured action receipts through the shared egui UI input path; drag targets use exact accessible locators and generation checks.
- [ ] Run a separate isolated app window/profile in the background for routine navigation and screenshots. Do not steal foreground focus or mutate the installed Hermes profile; live gateway acceptance must use an isolated profile or explicitly read-only scenarios.
- [ ] Add a separate authenticated/local diagnostics surface for route/layout/session/gateway state, logs, errors, and backend receipts. Diagnostics can inspect more than a person can see, but must not bypass UI confirmation, authorization, or normal action paths.
- [ ] Save screenshots and interaction traces with source SHA, build revision, route, viewport/DPI, fixture/profile identity, and timestamp so results are reviewable without operator participation.

**Gate:** an agent completes the same visible workflows as a person using semantic controls, records screenshots and receipts from a non-foreground run, and can inspect extra diagnostic state without changing the meaning or safety of the user-facing workflow.

### 5. Full parity acceptance

- Run the refreshed acceptance matrix against pinned upstream and Rust builds with matched fixture, theme, fonts, viewport and scale. Capture actual source and target windows and record exclusions/masks, route, state and backend receipt.
- Cover 100%, 125% and 150% DPI where the Windows host permits; include focus, hover, IME, keyboard, screen reader, narrow window, error/offline/recovery and multiple monitor/window lifecycle.
- Use the supplied image-diff script as a metric aid, never as the only acceptance decision.

**Gate:** all applicable in-scope requirements are verified with visual and functional evidence, or an explicit unresolved-difference review is accepted. No compilation-only, map-only or screenshot-only parity claim.

### 6. Alternate profiles and release

- Only after source parity, add alternate layouts through the same panel registry and DoofusDock public `WorkspaceProfile`/`Morph` APIs. Keep animation frames transient; commit canonical target once.
- Verify interruption, reverse, retarget, reduced motion, session/PTY/state preservation and exact return to Hermes.
- Harden licensing, accessibility, DPI, crash recovery, rollback and installer. Keep the current app and profile untouched; migration requires a separate explicit user decision.

## Current implementation slice

The native shell has seven source-defined layout trees, a separate Simple/Advanced selector, bounded preferences/layout persistence, Sessions and New session panels, route classification/navigation and still-incomplete source pages. Center-docked panels render accessible, switchable tab strips; the headless agent protocol can drag accessible panel headers, reorder tabs by semantic source/destination locators, and resize split dividers through actual egui pointer events. It attaches to the Hermes gateway, lists and resumes sessions, restores transcripts, and has partial existing- and fresh-session send paths with basic message events. The Sessions pane searches full stored history with a 200 ms debounce through the profile-scoped native HTTP route and preserves backend rank/snippets; its archived view pages `archived=only`, locally filters loaded rows, and exposes fenced Unarchive actions. Capabilities preserves source `?tab=` route state, reads Skills and Toolsets through separate one-use profile-scoped HTTP tickets, and provides local search and status states. Toolsets can be enabled/disabled through a typed shared UI/agent action and serialized bounded HTTP PUT; server receipts are checked and failed or mismatched updates roll the optimistic state back. All Toolsets network behavior is fixture-tested only; live-profile acceptance remains open. Plugins and Connectors are explicit incomplete states. The chat flows lack upstream submission identity/idempotent retry, server-request handling, richer event reconciliation and reconnect. An accessible Stop control now interrupts the active runtime session and reports a correlated result. An opt-in headless AccessKit runner inspects the semantic tree, supports role/name plus stable IDs for fixed first-party controls, clicks/types/selects/keys/drags controls, waits for visible targets, and captures screenshots in unique temporary profiles; it starts no Hermes gateway and opens no native window. A typed shared app-action registry covers navigation, pane toggles, interface mode, profile selection, Capabilities tabs, refresh, Toolsets enable/disable and `stop_generation` through shared UI handlers. Read-only diagnostics now include active runtime session ID, pending submission/interrupt flags and `can_stop` without exposing prompt or transcript contents. The default build excludes the runner and renderer dependency. Session rows and archived-view controls have stable IDs; dynamic toolset rows still need them. Modifier combinations, remaining typed actions, full chat workflows, source-complete routes, feature-specific overlays, floating/cross-window docking, external bridge/supervisor, physical Windows automation, and paired source/target screenshot receipts remain incomplete. The backlog and preserved 565-row research inventory under docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12 are the acceptance source of truth: 499 stock Hermes Desktop requirements are in scope and 66 Hermes-Doofus rows are excluded.

### Revision 23 — Capabilities Skills and Toolsets read workflows

Replaced the Capabilities offline placeholder with source-style Skills, Plugins, Connectors and Toolsets tabs, query-preserving route state, searchable Skills/Toolsets inventories, refresh/loading/error states, and explicit unfinished messages for Plugins/Connectors. Skills and Toolsets load from the pinned upstream `/api/skills` and `/api/tools/toolsets` endpoints using independent, one-use profile-scoped HTTP tickets on the bounded HTTP worker. Added typed `select_capabilities_tab` and `refresh_capabilities` app actions and stable semantic IDs; UI actions call the same app handlers as the stdio agent interface. Local mock HTTP tests validate ticket scope, endpoints, DTO parsing and bounded behavior. This is one route slice only; dynamic-row actions, mutation workflows, live-profile acceptance, source-paired visual evidence and complete 1:1 parity remain open.

### Revision 26 — stop active chat response and refresh source pin

Added an accessible Stop button while a chat response is pending and a typed `stop_generation` action. Both call the same app handler, which requires an active runtime session and pending submission, sends `session.interrupt`, correlates the response and distinguishes `interrupted` from `not_interrupted`. Successful interruption marks the partial assistant message terminal while preserving its text; errors keep the pending submission available for retry. The local WebSocket fixture checks runtime session scoping and both receipts; app tests cover stale, success, idle and error handling, while the agent protocol rejects disconnected calls. This remains fixture evidence only. Upstream `main` was refreshed to `8783054`; its broad delta is recorded above, while the source checkout and installed Hermes remain untouched.

### Revision 25 — Toolsets enable/disable

Added the upstream-compatible `PUT /api/tools/toolsets/{name}` mutation with a one-use profile-scoped ticket, bounded loopback-only HTTP transport, percent-encoded toolset names, and strict response validation. The Toolsets row presents a stateful toggle; human input and `app.action.execute` use the same typed handler. UI state updates optimistically and rolls back on transport or mismatched-receipt errors. The HTTP worker preserves FIFO order so accepted mutations each receive a result. Local fixture coverage checks method, path, ticket, JSON body, valid receipt and mismatched-state rejection; protocol coverage verifies the action is registered and rejects disconnected mutations. Live installed-profile acceptance and source-paired visuals remain open.

### Revision 24 — selected typefaces and current-source audit

Bundled the user's selected JetBrains Mono and Fira Code static TTF releases with upstream OFL 1.1 text, source versions and archive hashes. JetBrains Mono is the app UI family; its Medium face styles headings/buttons. Fira Code is the egui monospace family and the message composer uses it explicitly. Default egui fonts remain fallbacks. A headless 1440×900 screenshot and isolated-run metadata capture the chosen typography. The active source target remains the downloaded Hermes snapshot 8783054ef6cbafd86f10892241436862c1171a66. Later commits were inspected only for historical provenance and do not alter implementation scope or acceptance. No recurring upstream update checks are part of this work. The user-selected Fira Code and JetBrains Mono typefaces remain a visual override and do not change the fixed 565-item parity denominator. Fonts are a user-directed style override and do not satisfy source-paired visual parity.

### Revision 4 — gateway discovery implementation

The native app invokes the upstream-supported `hermes gateway ensure --json --timeout 60` command on a named worker thread. It requires a ready protocol-1 endpoint, loopback origin, nonempty profile/instance identity, authority epoch, and `session-authority-v1` capability. Failures remain visible in the shell. This stage does not mint a ticket, dial WebSocket, negotiate `client.capabilities`, or load sessions. The installed Hermes CLI is available on PATH, but the command was not run during implementation because `ensure` may start the persistent gateway owner; runtime attach remains an acceptance step.

### Revision 5 — authenticated gateway and session read workflow

The installed Hermes CLI is `0.22.5+2213.g4ca3db7`; it implements `gateway ensure` and does not implement the newer `gateway attach` command. Rust now follows upstream Desktop's Windows Python ticket bridge: it resolves the launcher's own Python bootstrap, calls `_session_ticket` for `interactive`, zeroizes ticket-bearing buffers, and keeps the one-use ticket in the canonical WebSocket subprotocol. It rejects redirects and protocol/identity mismatches, waits for `gateway.ready`, declares only the capabilities it actually handles, validates `runtime.describe`, and reads `session.list`. The socket remains owned by a worker for `session.resume`; a live Windows run loaded two sessions and rendered a real 295-message transcript. Existing-session submit and a partial fresh-session `session.create` → `prompt.submit` path are fixture-tested; both use basic message start/delta/complete projection. Full upstream create options/retry, retained admission identity, stop, server-request handlers, heartbeat/reconnect recovery, profile switching and full transcript/event projection remain open.


### Revision 6 — unattended agent-operated UI

User requirement: the coding agent should be able to do anything a person can do in the UI, plus inspect/debug more, while working without disturbing the operator. The implementation target is a Playwright-like semantic control surface backed by the native accessibility tree and stable test IDs, with actions routed through the same application handlers as normal input. A separate local diagnostics interface may expose richer state, logs, and receipts but cannot bypass authorization or UI confirmation. Routine runs should use an isolated background window and profile, avoid foreground focus, and preserve screenshots/traces with build and viewport metadata. The headless semantic runner is now implemented; stable test IDs, external control, diagnostics, background supervisor, and physical host acceptance remain open.

### Revision 7 — supplemental Doofus Apps agent-control architecture

The user supplied an additional architecture for application core, UI host, semantic control service, development-only Debug Bridge, and development supervisor. Apply only its Hermes-specific requirements; do not expand implementation into Doofus Image, Doofus Code, or unrelated apps, and do not embed an AI model. Keep the Hermes Python gateway authoritative for session execution and backend events. Keep app control separate from Hermes session control.

Current implementation adds a headless egui_kittest/AccessKit runner over the shared `HermesNativeApp::draw` path. It can inspect controls, click/type by accessible name, and render a screenshot without a native window, gateway startup, or installed profile. This is semantic UI evidence, not an external bridge or physical Windows evidence.

The remaining supplemental gates are: (1) typed capability registry and shared semantic app operations, (2) authenticated, versioned, loopback-only development bridge with read/write scopes, schema and size bounds, idempotency, revision checks, audit/redaction/revocation, and no production exposure, (3) structured state/events/diagnostics, (4) safe declarative reload and native Rust rebuild/restart supervisor with generation/readiness/reconnect and session preservation, and (5) separate core-contract, semantic UI, and physical Windows evidence. All agent mutations must pass through the same application/DoofusDock/Hermes authorization paths as human actions. See `docs/ARCHITECTURE.md` and `docs/AGENT_CONTROL_CROSSWALK.md`.

### Revision 8 — inherited-stdio semantic UI protocol

Added the opt-in `hermes-automation --stdio` child-process interface. The parent-owned inherited pipe is its control capability; it does not open a listener or native window. The v1 NDJSON contract supports health/capability/state/tree/event/diagnostic reads, exact accessible role/name click and type actions, screenshot capture, and close. UI actions require an expected state generation and return structured before/after receipts. The response cache replays matching request IDs, detects payload conflicts, and rejects expired IDs to prevent stale retries from repeating a mutation. Request/text/response/event/unique-ID bounds are specified in `docs/AGENT_AUTOMATION_PROTOCOL.md`.

This is a background headless control surface, not the originally planned authenticated network Debug Bridge, supervisor, stable test-ID interface, native window controller, or physical-host evidence. Network authentication is unnecessary for the inherited-pipe transport because only the trusted parent launches and owns it; exposing the pipe to an untrusted peer is out of scope. The full supplemental bridge and supervisor requirements remain in the backlog.

### Revision 9 — Playwright-like semantic inputs

The child protocol now opens accessible selectors and chooses one unique visible option, sends a bounded set of egui key events, and waits up to five seconds for a visible accessible role/name. These use egui_kittest input and the shared UI widgets; each UI mutation requires the current state generation and produces an action receipt. Accessibility tree bounds are nullable when AccessKit has no rectangle, avoiding a node-geometry panic. These operations make navigation more complete but do not yet cover modifiers, stable test IDs, native host input, or upstream UI parity.

### Revision 10 — accessible target safety

Named the message composer in the AccessKit tree and made click/type/selector actions reject disabled targets before dispatch. This allows agents to identify the actual composer while ensuring that its offline/disabled state is respected. Full composer/backend behavior remains part of the incomplete chat workflow stage.

### Revision 11 — existing-session prompt and event slice

The composer is enabled only for a resumed existing session with a live gateway. It submits `prompt.submit` with the runtime session ID (not the stored sidebar route ID), displays an optimistic user row, restores that text on an RPC refusal, and applies basic `message.start`, `message.delta`, `message.complete`, and `error` events only to the active runtime session. Prompt acknowledgements and stream events are routed through the single gateway worker. A local WebSocket fixture verifies exact request scoping and event ordering; app projection tests cover deltas, completion and cross-session isolation. New-session creation, upstream retained submission/admission IDs, stop, server prompts, reasoning/tools, rich transcript semantics, reconnect and live gateway write acceptance remain outstanding.

### Revision 12 — partial fresh-session creation

The new-chat composer can now call `session.create` with the desktop source, 96-column width, and attached profile identity. It requires both runtime and stored session IDs, navigates using the stored ID, and submits the original draft using the runtime ID. A local WebSocket fixture verifies the ordered create/submit exchange and the distinct ID roles. The installed Hermes gateway was not sent a new-session write. The create options are not yet the complete upstream `desktopSessionCreateParams` contract: working-directory resolution, model/provider/reasoning/fast/service-tier settings and unsupported-field retry are open. Prompt admission IDs are local correlation only; server-side idempotency, stop, server prompts, rich event handling and recovery are also open.

### Revision 13 — local session list filtering

The Sessions pane exposes an accessible `Search sessions` field and locally filters loaded gateway rows by title, preview and ID. Revision 20 adds upstream 200 ms debounce and `/api/sessions/search` over full stored history through a profile-scoped native HTTP ticket on a separate worker. Requests are loopback-only, proxy and redirects are disabled, response time and size are bounded, and stale responses are rejected. Backend rank is preserved and loaded-only local matches are appended. HTTP fixture and feature merge tests pass; installed-profile search and paired screenshots remain open.

### Revision 14 — session recency metadata

The pinned snapshot's `SessionListRow.started_at` is a JSON number and may contain fractional epoch seconds. The Desktop sidebar uses its HTTP `/api/sessions` list and displays `last_active || started_at`; Rust now reads the same fields with a fresh native HTTP ticket and keeps the authenticated Gateway list as fallback. A fixture verifies request scope, fractional timestamps, nullable fields and the compact age formatter's precedence behavior.

### Revision 15 — canonical titlebar drag/drop slice

Panel titlebar drag now tracks a revision-bound DoofusDock `DragSession`, previews center and 22%-edge drop zones, and commits center drops as tabs or edge drops as 40/60 splits through `DockingService::transact`. Successful commits persist through the existing `LayoutStore`. Automation-feature tests cover all five drop zones plus a center transaction and persisted layout. The child-process UI protocol still has no pointer-drag command, so an agent cannot yet reproduce this gesture through its Playwright-like action interface. Native-window gesture acceptance, tab reorder, resize, float/cross-window docking and paired upstream visual evidence remain open.
### Revision 16 — agent pointer drag and reachable dock tabs

The v1 UI protocol now implements `actions.drag` using the real egui pointer event path. It resolves a unique AccessKit source by role and name, begins at the accessible bounds center, validates an in-viewport destination, limits interpolation to 1–32 steps, checks expected state generation, and returns before/after app state. Panel title areas and tab controls have semantic labels. Center-docked DoofusDock groups render switchable tabs; selected tabs update via `Operation::SelectTab`, and the projection renderer omits zero-opacity inactive leaves so hidden panels do not paint over the active panel. Feature tests exercise a center drag, tab selection, a left-edge split, and invalid step bounds. A 1440×900 isolated headless screenshot visually confirms the Sessions tab is reachable after docking. Native OS gesture acceptance and paired upstream visuals remain open.
### Revision 17 — canonical split resizing

The native host renders the pinned DoofusDock `resize_handle` at each projected split divider. Pointer movement previews a clone of the canonical document with adjacent weights changed using the same 5%/95% pair clamp as the DoofusDock demo; release commits one revision-checked `Operation::ResizeSplit` and persists via `LayoutStore`. The accessible action name includes the stable split node ID and divider index, so the v1 `actions.drag` command can resize it by role/name. Feature tests verify geometric clamping, real pointer input, changed weights, revision advance and clean persistence status. A 1440×900 headless screenshot shows the Sessions/Workspace boundary moved. Physical Windows resizing, nested/all-profile host acceptance, keyboard splitter controls, tab reorder and cross-window interactions remain open.

### Revision 18 — canonical dock tab reorder

Dragging a tab over another tab in the same DoofusDock group now records a transient reorder candidate and commits through the revision-checked `Operation::ReorderPanel`. The operation preserves panel identity, selects the moved tab and saves through `LayoutStore`; dragging outside the source tab strip still uses the existing panel-drop behavior. Agent `actions.drag` now accepts semantic destination role/name locators, and the headless protocol test performs tab reorder without destination coordinates. The isolated child-process run verified canonical order, active tab, revision advance and persistence, then captured and visually inspected `artifacts/agent-ui/tab-reorder-v18.png`. This completes only the semantic/headless tab-reorder slice; physical Windows drag behavior, pinned-tab policy, overflow/narrow-width and cross-window interactions remain open. The supplemental agent-control architecture remains only partially implemented: typed shared capability registry, authenticated external bridge, structured app-wide diagnostics and native rebuild/restart supervisor are separate gates.

### Revision 19 — preserve docking when a tab drag leaves its strip

If a tab drag is released outside its own tab strip, the reorder handler now leaves the existing panel `DragSession` intact so center/edge docking can handle the release. If the pointer is still over a tab in the source strip, canonical reorder takes priority. An app-level regression test verifies that the no-tab-target path preserves the generic panel drag session. A fresh isolated run repeated semantic tab reorder and captured `artifacts/agent-ui/tab-reorder-v19.png` against the final binary. This fixes the handoff boundary between tab reorder and panel docking; cross-group drops and physical Windows host behavior still need acceptance evidence.

### Revision 20 — upstream full-history session search

The accessible Sessions search now uses the upstream 200 ms debounce and profile-scoped `/api/sessions/search` route. A dedicated worker mints Hermes' `native-http` ticket and performs a loopback-only request with proxy discovery disabled, redirects rejected, a 5-second timeout and a 512 KiB body limit. Request IDs and query values fence stale responses; returned backend rank is retained and loaded-only local matches are appended. Results map titles, highlighted snippets, source and `last_active` metadata into the existing session list. A local HTTP fixture verifies percent encoding, the ticket header, DTO decoding, and a ranked response; feature tests verify ranked merge and local pending behavior. Live installed-profile search and matched source/target UI screenshots remain open acceptance work.

### Revision 21 — stable semantic IDs for fixed first-party controls

`ui.tree` now includes a nullable `test_id`, and click/type/select/drag/wait methods accept either that ID or role/name. Explicit IDs cover fixed controls such as `sessions.search`, `chat.composer`, `app.interface-mode`, `route.capabilities`, and dock targets including `panel.sessions` and `dock.tab.workspace`. Session rows expose `session.row.<stored-id>` and include the stored ID in their accessible name so locators are unique when titles repeat. IDs are resolved against the current visible AccessKit tree and ambiguity fails closed. Headless tests exercise click, type, select, wait, and semantic drag using IDs. Arbitrary contributed controls and user content remain open. The protocol remains an inherited-pipe local process capability, not the supplemental authenticated network Debug Bridge.

### Revision 22 — shared typed app-action registry

Added a versioned, serde-tagged app-action registry for route navigation, pane toggles, interface mode and layout profile selection. Human toolbar, route, and pane controls now call the same app-core handlers exposed as `app.action.execute`; `app.actions.list` publishes their schema and permission class. Dispatch validates tagged action objects, rejects unknown fields and pane IDs, requires the current UI state generation, and records structured receipts. A headless protocol integration test proves a typed pane toggle and a human-equivalent click drive the same resulting state, and checks rejection of an unregistered pane. The registry does not yet cover all UI, DoofusDock, or Hermes operations, nor does it replace the planned authenticated Debug Bridge.

### Revision 29 — generation-scoped tool-call transcript cards

The active-session event projection now upserts `tool.start` and `tool.complete` into transcript groups, matching a reused tool ID only within the supplied execution generation. Start arguments remain available when completion arrives; result text, summary, inline diff, and other bounded completion fields are rendered. Each transcript group has a combined AccessKit label so the background agent can inspect what a person sees. A 1440×900 isolated egui_kittest capture shows two completed tool calls, and the fixture checks their generation scoping and accessible labels. Event ordering/replay and full upstream tool/reasoning visuals remain open; this does not establish live gateway or source-paired parity.

### Revision 30 — inbound server-request fail-fast fallback

The gateway reader now recognizes an inbound JSON-RPC request frame while another RPC is in flight and sends a correlated `-32601` response when the native app has no handler, then continues waiting for the original RPC. The idle event loop uses the same fallback. A local WebSocket fixture proves the response ID/error and that `runtime.describe` still completes. This is only the protocol fallback: the client still advertises `server_requests: false`, and approval, batch clarify and other server-request handlers are not implemented.

### Revision 31 — active-session approval server-request route

The idle gateway actor now forwards upstream `approval` requests to the app and sends the UI's `{choice}` result back on the request's original JSON-RPC ID. The app reuses the event-driven permission card, respects backend choices and `allow_permanent`, and rejects requests for a session other than the active runtime session. An AccessKit fixture verifies the visible card and restricted choices; a local WebSocket fixture verifies request routing and the same-socket result. This is fixture-backed transport/UI coverage only: `server_requests` remains false until batch clarify and the rest of request ownership/dispatch are implemented.

### Revision 32 — active-session batch clarify card and lock transport

The idle actor now forwards upstream `clarify` server requests to the active-session chat. The card stages all questions, restores editable replay answers, exposes upstream question progress and selection hints, and sends `clarify.lock` calls sequentially in question order when confirmed. Skip answers the server request with an empty result, matching the current Desktop path. Local actor, RPC receipt, helper and AccessKit fixtures cover routing, recommendation-label stripping, multi-select encoding and replay staging. `server_requests` remains false because remaining request methods, in-flight request multiplexing and multiwindow ownership are incomplete; no live gateway or source-paired visual acceptance is claimed.

### Revision 33 — fixed downloaded Hermes parity snapshot

The parity target is the downloaded Hermes source commit `8783054ef6cbafd86f10892241436862c1171a66`, preserved by local ref `refs/codex/parity/hermes-8783054`. Later upstream `main` commits do not change the target; re-pin only when a replacement source is explicitly selected. The Sessions sidebar reads `last_active` from the pinned Desktop session-list API and falls back to the live Gateway rows if the HTTP request fails.

### Revision 34 — dynamic session-row locators

Session rows now expose descriptive accessible names containing their stored IDs. The automation tree derives stable `session.row.<stored-id>` locators for these dynamic controls, allowing a coding agent to address an exact session even when multiple rows share a title. App and protocol tests verify the accessible row label and locator mapping. Other contributed/user content and live process-level navigation remain open.

### Revision 35 — source-fenced session pin action

Session row menus now expose Pin/Unpin. The async Gateway HTTP worker reads the pinned Desktop mutation snapshot with one `native-http` ticket and sends the PATCH with a second one-use ticket, the action request ID, and the snapshot's expected revision and generation. The app updates the row only after `{ok:true}` and correlates the receipt to the pending session action. The action is disabled while offline. A local two-request HTTP fixture verifies profile scope, ticket headers, fence fields and the receipt; no installed session was modified.

### Revision 36 — persisted session unread action

Session list rows now decode the backend `unread` state. The row menu exposes Mark unread/Mark read, uses the same mutation snapshot and two-ticket revision/generation fence as Pin/Unpin, and updates local state only after a correlated `{ok:true}` receipt. The action is unavailable offline and has `session.unread.<stored-id>` semantic locators. Fixtures verify the unread PATCH body and shared fence fields. Transient finished-response unread indicators and the remaining row actions stay open.

### Revision 37 — fenced session archive action

Session rows now decode the Desktop `archived` state and expose Archive/Unarchive menu labels with `session.archive.<stored-id>` agent locators. The mutation uses the same one-use snapshot and PATCH tickets, request ID, named-profile scope, and expected revision/generation fence as Pin and unread. Archiving also sends `pinned:false`, matching the source behavior. A confirmed archive removes the row from active sessions and current search results. The active list excludes archived rows, so a dedicated archived-session query/view and reachable Unarchive flow remain open. HTTP and AccessKit/locator fixtures pass; installed-profile writes remain unverified.

### Revision 38 — paged archived-session view and reachable Unarchive

The Sessions pane now switches between active and archived views. The archived view reads the pinned source's `archived=only` endpoint through a fresh native HTTP ticket per 200-row page, follows fixed page offsets, deduplicates pinned rows that the backend may backfill across pages, and fences UI publication by request ID. Loaded archived rows can be filtered locally and expose the existing revision/generation-fenced Unarchive action; a confirmed restore moves the row back to active sessions. Agent locators identify both view toggles, and diagnostics report archived-view/loading state. HTTP pagination, deduplication, AccessKit rendering and locator mapping are tested. The source Desktop can aggregate across profiles; this implementation currently queries the attached profile and remains open to profile-switch parity.

### Revision 39 — cross-profile archived-session ownership

The archived view now follows the fixed Desktop's `/api/profiles/sessions?profile=all&archived=only` query and 500-row page size. Session rows retain profile ownership; deduplication keys include profile and ID, scoped agent locators distinguish duplicate IDs, and Pin/Unread/Archive requests carry the row profile through the existing mutation snapshot fence. Archived rows visibly identify their profile. A row from a different profile is disabled for opening because the native chat UI has not yet propagated profile scope through resume, prompt, interrupt and prompt-response lifecycles. Local fixtures verify the all-profile request, same-ID separation, profile-specific mutation body and safe visible state. Remote connection aggregation and cross-profile chat routing remain open.


### Revision 42 — acceptance target and scope reconciliation

The acceptance package preserves all 565 research rows. Scope audit identified 499 stock Hermes Desktop rows and 66 Hermes-Doofus rows. Only the 499 stock Desktop rows define the approved 1:1 target; the other rows remain excluded and pending. The 499 matrix source targets now name the fixed downloaded Hermes snapshot `8783054ef6cbafd86f10892241436862c1171a66`, while historical evidence locators retain their original SHA. This is metadata/scope accounting only: all acceptance results remain pending and no row-level source verification is claimed.


### Revision 43 — bounded transcript history

Added an accessible earlier-history action for long sessions. The native HTTP actor requests at most 120 stored messages with the session profile, `offset`, `order=latest`, and compacted history included. It checks session identity, optional returned profile, and echoed pagination before accepting a page; orderless legacy responses are treated as complete transcripts. The UI fences results to the exact pending request and currently active session/profile, merges overlapping page boundaries, and retains a page error for retry. A headless `--long-session-fixture` emits an AccessKit tree and screenshot. Tests cover the request query, ticket, wrong-order rejection, legacy full transcript, merge overlap, and accessible control. This slice does not provide upstream transcript virtualization, scroll-boundary-triggered expansion, retention, or scroll-anchor restoration; no live gateway has been exercised.


### Revision 44 — legacy transcript page compatibility

Corrected the bounded-read response guard: current paginated responses remain limited to 120 and must echo `latest`, offset and returned count; a legacy response with no pagination metadata may contain the complete transcript (subject to the existing 512 KiB transport limit and an additional 10,000-row cap) and replaces the loaded transcript. A regression fixture returns 121 legacy rows to prove it is not mistaken for an oversized current page. The fixed-source target and live-gateway acceptance remain unchanged.

### Revision 45 — source-faithful session-tab architecture plan

Source inspection confirmed that the pinned Desktop does not implement session tabs as a separate list: existing sessions are dynamic pane-tree entries (`session-tile:<stored-id>`) stacked into DoofusDock's workspace group or docked into other zones. The workspace tab is a special primary chat pane. Opening a new session creates an unlisted tile, with one untouched default-owner draft and a per-window single-flight create latch. Closing a main tab promotes the next tile or resets an occupied main pane to a fresh draft; closing a busy tile requires confirmation and does not terminate the Hermes session. Bot-mode new tabs carry exact owner/profile scope when available.

Implementation sequence (ordered):

1. [x] Extend the canonical `LayoutDocument`/panel registry and `DockingService` transaction path to support dynamic, stable, profile-scoped session-pane IDs without a parallel app-owned layout tree. Persist validated stored-session/profile metadata, add/front/close panes through revision-bound Dock operations, and preserve pane state through profile changes and layout restore. Runtime linkage, title synchronization and UI exposure remain open.
2. Make each dynamic pane own a session view model (draft, transcript window/cursor, runtime identity, pending/uncertain send, approval/clarify state and scroll position). Route gateway events and action receipts by `(profile, stored_session_id, runtime_session_id)` even when the pane is hidden. Re-selecting a pane must restore its visible state without duplicate `session.resume` or prompt submission.
3. Implement source-faithful open-existing behavior, unlisted draft creation, per-window single-flight creation, Bot Mode exact-owner scope and first-message listing. The native now reuses a clean primary draft or known idle empty unlisted same-profile tile; Bot Mode owner routes, attachment-aware emptiness and live draft debounce flushing remain open. Keep failures attached to the originating pane.
4. [in progress] Implement active pane selection/reorder and close semantics through DoofusDock's canonical tab/docking transactions. `Ctrl+W`/agent `close_active_tab`, busy tile confirmation, workspace promotion and reset are implemented. Include remaining close-other/right/all commands, focus precedence, accessibility names, narrow overflow and no-window-close guarantees.
5. Add shared semantic UI/agent actions and stable locators for open/select/reorder/close. Add deterministic fixtures for multiple simultaneous sessions, cross-profile duplicate stored IDs, background streaming and stale receipts, draft reuse, close/promotion and failure recovery. Capture a 1440px source-paired screenshot only after source and target have matched session fixtures.

Acceptance: layout revision and persisted pane order/active selection change only through canonical Dock transactions; session ownership and messages never bleed across tabs/profiles; source close/open rules match the pinned implementation; all tab actions work through both UI and semantic agent control; paired source/target functional receipts and Windows screenshots are reviewable. Do not replace this architecture with a separate session-tab vector or treat the visible-pane slice alone as parity.

### Revision 46 — canonical dynamic session-pane foundation

Added profile-scoped stable session-pane identities as DoofusDock `PanelDescriptor`s, including validated session/profile metadata and bounded titles. Opening a pane uses revision-bound `Operation::AddPanel` targeting the canonical workspace node; fronting and closing use `SelectTab` and `ClosePanel`. Profile application preserves open session panes in the workspace tab group and leaves explicitly closed panes closed; reopening uses the current profile's canonical profile transaction. Tests cover duplicate stored IDs across profiles, bounded/control-character validation, identity metadata, profile changes, close/reopen, active tab selection, and persisted layout restoration.

This is the layout foundation only. No UI action creates these panes yet, the renderer does not show a session chat inside each pane, busy-close confirmation is not wired, and per-pane transcript/draft/runtime event ownership is still open. This slice is not user-visible session-tab parity.
### Revision 47 — profile-keyed session chat state and background stream routing

The app now keeps an in-memory chat view cache keyed by `(profile, stored_session_id)`. Route changes save and restore runtime session identity, transcript/page state and composer draft, so equal stored IDs in different profiles remain independent. Resume snapshots hydrate the matching view. Message deltas/completions and tool start/completion events from a non-visible runtime update only the unique matching cached view; when an event omits profile and runtime identity is ambiguous, it is ignored. The cache targets 32 views, evicts least-recently used idle views first, and protects the visible view, streams, pending transcript reads, submissions, interrupts and unresolved approval/clarify prompts. Automation diagnostics expose only the cached view count.

This remains route-level state, not per-pane rendering. The map is in-memory only; views can be recovered from Hermes after app restart. Hidden-session approval/clarify server-request routing and visual chat widgets inside dynamic DoofusDock panes remain open. No user-visible session-tab parity is claimed.

### Revision 48 — visible profile-scoped session panes

Dynamic Hermes session descriptors now render the transcript, history paging controls, approval/clarify cards and composer inside their own DoofusDock panel. Rendering temporarily activates the keyed profile+stored-ID state and restores the primary chat state afterward. Opening an existing session or deep link creates/fronts its canonical Dock tile; new-session creation requests a blank backend session and keeps it unlisted until its first admitted prompt; the same-profile untouched empty tile is reused, while an edited draft gets a fresh tile. Admitted messages update the owning cached transcript and add a newly used session to the sidebar. DoofusDock tab strips expose session-tile close controls; busy tiles require an explicit close confirmation that leaves Hermes running. A dedicated headless fixture can render the session-pane interaction and screenshot at `artifacts/agent-ui/session-pane-current-20261115.png`.

Local tests cover pane rendering and draft isolation across profiles, hidden-session submission receipt routing, sidebar promotion on first admitted message, untouched-tile reuse versus edited-draft separation, close confirmation policy, and canonical/idempotent panel selection. Remaining gaps include reuse of edited blank drafts, resume failure correlation, scroll-position durability, close-other/right/all commands, close-focus precedence across tool zones, narrow overflow, live gateway acceptance, and matched source screenshots. This is a visible behavior slice, not 1:1 session parity.


### Revision 49 — profile-bound background approval and clarification actions

Approval and batch clarification prompts now retain their originating profile alongside the runtime session ID. Incoming requests still fail closed unless that runtime/profile pair resolves to exactly one open session pane; if ownership is ambiguous or conflicts with the connected endpoint profile, the request is rejected. Prompt response actions validate the same unique owner without requiring the pane to be globally active, and approval/clarification RPCs use the prompt's stored profile. Batch clarification lock RPCs use the same scope.

The hidden-pane fixture now verifies both accessible prompt controls and that semantic UI actions reach the gateway-writer boundary for the background pane. This proves app routing in isolation, not an authenticated live response. Remaining tab lifecycle and full parity gates stay open.


### Revision 50 — primary workspace close and promotion

The native close-tab command now follows the pinned Desktop's primary workspace behavior. `Ctrl+W` (Command+W on macOS) and the registered `close_active_tab` agent action share one handler. When a session tile is selected, the handler uses the existing busy-close confirmation. When the primary workspace is selected, it promotes the nearest live session tile from the workspace tab group (following the workspace tab, then wrapping outward); if no tile is stacked there, it promotes an open session tile from another Dock zone. Promotion closes only the tile through a canonical DoofusDock transaction and loads its profile-scoped cached view into the primary chat. With no open tile, an occupied primary chat becomes a clean New session draft; a blank draft or full-page route is unchanged.

Local tests cover profile-preserving promotion, primary reset, selected busy-tab confirmation and agent action registration. The suite validates app behavior and shortcut matching is wired in `draw`; physical Windows keyboard/focus acceptance is still open. This does not complete close-target precedence, terminal/tool-tab closing, secondary chat groups or narrow tab overflow.


### Revision 51 — exact untouched New session reuse

The New session action now follows the pinned `frontUntouchedDraftTab` rules for the surfaces supported by this native workspace. It first fronts the primary New session draft only when that route has no active session, no pending creation, no typed draft, and its selected profile matches the gateway's current default owner. Otherwise it can front the tracked blank session tile only when its Dock pane is still open, the runtime view is known and idle, transcript/message count and composer are empty, the session is unlisted, and the pane owner matches the current profile. If no candidate qualifies, it starts one new blank session creation; repeated clicks during `session.create` stay single-flight. A typed main draft and an edited tile remain intact.

Local tests cover clean-main reuse, typed-main preservation, exact-profile tile reuse and edited-tile non-reuse. Hermes Bot Mode's exact owner-route ladder, attachment-aware drafts and composer debounce flushing do not exist in this native surface and remain outside this slice.


### Revision 52 — correlated session resume errors and retry

Session resume requests now carry a request ID, stored session ID, and profile through the gateway actor and back in the result event. Only the matching pending request can update its owning cached session view; stale results are ignored, and successful snapshots are checked against both session and profile before hydration. Resume errors stay on the originating tile and render an accessible “Couldn't open this session” message with a Retry action. Retry clears that tile's error and issues a fresh correlated request. A gateway disconnect releases pending request ownership so the pane can resume after reconnect; latched errors are cleared on reconnect for automatic recovery.

Local tests cover matching and stale failures, owner-scoped error rendering and retry locator availability, successful profile-scoped hydration with draft preservation, and stale failure after success. `artifacts/agent-ui/session-pane-resume-error-20261115.png` records the normal isolated pane fixture; it does not depict the error state. No authenticated gateway or physical Windows interaction was used. The source's additional authoritative not-found/removal recovery and paired source screenshot remain open; this slice is not full session-tab parity.


### Revision 53 — resume failure takes precedence over cached transcript

A latched resume failure now renders its pane-local error and Retry action even when that cached pane still has transcript messages from an earlier visit. This matches the source tile's failed-open state instead of leaving the user looking at cached conversation with no visible recovery action. The AccessKit test now seeds cached history before delivering the correlated failure. The headless standard-pane screenshot is unchanged and does not depict this error state. Live gateway and source-paired host evidence remain open.


### Revision 54 — close the active session tab in its Dock group

Session-tab close targeting now follows the pinned Desktop's eligible-group ladder: the currently hovered tab group containing the workspace or a session tile, then the last clicked eligible group, then the workspace group. A focused session tile in another Dock group closes through the existing busy-session confirmation and canonical Dock close path, leaving the workspace chat untouched. The registered agent action and Ctrl+W still share the same handler.

Local tests cover hover/focus/workspace target resolution, stale group fallback, and closing a profile-scoped session pane that shares a Files tab group. Physical pointer/keyboard acceptance is not established. Egui keyboard traversal does not yet update the focused-group marker, and generic tool-tab close/visibility-store handoff remains open; this revision therefore closes one source gap without claiming complete close-target precedence.


### Revision 55 — DoofusDock tab context actions

Extend the exact pinned DoofusDock egui renderer tab-strip context API inside the Hermes repository, preserving the pinned crate source and MIT attribution. Add Close tab, Close others, Close tabs to the right, and Close all commands with per-tab enablement. Hermes Native will map those menu receipts to canonical Dock operations, preserve the uncloseable workspace tab, keep session profile ownership, and serialize busy-session close confirmations. Human right-click and semantic UI interaction must use the same menu controls and app handlers.

The directly pinned DoofusDock checkout is dirty in unrelated projects, so it will remain untouched. The implementation will vendor only the unchanged doofusdock-render-egui crate from the exact pinned commit f6eb49b9e488f4e851d8d3abe18e95d296201bba and patch that renderer locally, leaving DoofusDock core/presentation/host crates pinned. Tests will cover menu visibility/counts, menu action mapping, mixed workspace/session tab groups, busy-close confirmation queueing, and profile-preserving canonical close results. This remains a partial tab-menu slice; complete terminal/tool closer synchronization and source-paired Windows interactions remain open.


### Revision 56 — complete pinned-source session tab context actions

The exact pinned `doofusdock-render-egui` crate is now vendored under the Hermes Native workspace with its MIT license, and Cargo patches only that renderer package to the local source. Its existing `tab_strip` API remains compatible; a callback-based `tab_strip_with_context_menu` API exposes the pinned renderer's actual context menu to Hermes. Tabs provide Pin/Unpin, Close tab, Close others, Close tabs to the right, and Close all. Bulk actions target open session panes only and preserve the workspace tab. Idle panes close through canonical Dock transactions; active panes enter the existing serialized confirmation queue. The agent protocol adds `actions.right_click`, emitting real egui secondary-pointer events so the agent opens the same context menu as a person.

Tests verify real secondary-pointer access to the shared Dock menu and discoverable capability reporting. The full package suite, pinned-source validator and all-target build must pass before publication. Direct app tests cover session-only close target ordering, workspace preservation, duplicate filtering and serialized busy-pane close queue advancement. The protocol fixture verifies a real secondary-pointer menu open. Physical Windows menu interaction and source-paired screenshot evidence remain open. This does not complete tab/tool parity or the full 1:1 replacement.


### Revision 57 — modifier-aware agent keyboard input

Extend `actions.keypress` with an optional bounded list of explicit modifiers (`ALT`, `CTRL`, `SHIFT`, `COMMAND`, `META`). The agent injects the same egui modifier/key event path used by the app, returns the normalized modifier receipt, and rejects malformed, duplicate or unsupported values before input. `COMMAND` represents the platform shortcut modifier (Ctrl on Windows/Linux, Command on macOS); `CTRL` remains the physical Control key. An isolated end-to-end test opens a profile-scoped Hermes session tile, invokes `COMMAND+W` through the protocol and verifies the canonical close handler closes that same tile; unsupported modifier rejection is also covered.

This removes one automation limitation, not the broader native keyboard parity gate. Shortcut coverage, focus ownership across tool panes, physical Windows acceptance and source-paired keyboard evidence remain open.

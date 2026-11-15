# Hermes Native architecture and ownership

This document applies the Hermes-specific parts of the supplied Doofus Apps Agent Control and Development Architecture. It does not expand this task into implementation work on Doofus Image, Doofus Code, DoofusDock itself, or unrelated apps. No AI model is embedded in Hermes Native.

## Ownership

- **Hermes Python gateway:** authoritative session execution, backend requests, approvals, recovery and backend events. Rust must not recreate agent logic.
- **Hermes Native app core:** owns navigation and view-specific state; feature mutations use the same approved Hermes gateway methods as source UI.
- **egui/eframe host:** owns rendered widgets, accessibility integration and native windows. Hermes Native remains a standalone app when development control is disabled.
- **DoofusDock:** owns canonical `LayoutDocument`, revision-checked layout changes, profiles and presentation. App owns panel content. Logical layout acceptance and physical OS behavior are separate evidence.
- **Semantic control service (in progress):** the initial typed app-action registry is shared by human route/layout controls and the agent protocol; remaining actions, permissions, and feature-level DoofusDock/Hermes operations are still open.
- **Development Debug Bridge (planned):** authenticated local transport for bounded state, semantic actions, events and diagnostics. It is distinct from Hermes gateway and disabled/absent in production.
- **Development supervisor (planned):** declarative reload, Rust rebuild/restart, generation/readiness handshake and automation-client recovery.

## Current boundaries

`src/layout.rs` owns a `DockingService` and `LayoutStore`; projected panel geometry is rendered through DoofusDock presentation APIs. The UI manually renders projected rectangles and accepts titlebar/tab drags that center-tab or edge-split through revision-checked DoofusDock transactions. Center-docked groups render a switchable accessible tab strip, and the headless agent protocol emits bounded primary-pointer events through the same egui draw path. Same-group tab reorder uses a revision-checked `ReorderPanel`; split dividers use DoofusDock resize handles and revision-checked `ResizeSplit` transactions. Floating/cross-window behavior and native window lifecycle remain incomplete. `src/gateway.rs` owns the Hermes transport. It restores existing sessions, creates a fresh session, submits its initial prompt using the runtime session ID, and forwards basic message lifecycle/delta events. Full-history search uses a separate bounded worker with a profile-scoped native HTTP ticket, keeping it off the WebSocket event actor; requests are loopback-only and disable ambient proxies and redirects. `src/app.rs` owns UI and route state, including stored-session navigation, 200 ms session-search debounce/request fencing, ranked full-history result mapping with loaded local matches, compact `last_active` display and the active runtime ID used to fence incoming events. The active composer exposes a Stop action that calls `session.interrupt` with the active runtime session ID; correlated success preserves partial assistant text and marks it terminal, while an already-idle receipt unblocks the composer. `approval.request` events render accessible cards with only backend-offered once/session/always/deny decisions; `approval.respond` carries the active runtime session and execution generation. Single-question `clarify.request` events render source choices, free text, multi-select answers and skip; `clarify.respond` carries the prompt's active runtime session and generation. Batch server-request `clarify` cards stage all question answers, restore replay answers as editable controls, and send sequential `clarify.lock` RPCs in source order; Skip returns an empty server-request result. Both prompt flows validate correlated receipts and disable controls while offline or pending; local fixtures cover scope, mismatched receipts, stale actions and accessible offline state. Live gateway behavior is unverified. Remaining chat gaps include full source create options/retry, other server-request methods and in-flight request multiplexing, complete event reconciliation, idempotent submit recovery and reconnect. The opt-in `agent-automation` feature builds `hermes-automation`, a headless egui_kittest harness around the same `HermesNativeApp::draw` path. It disables gateway startup, uses an isolated temporary layout profile for success and fallback paths, and removes that profile on exit. This proves semantic widget input and headless frame capture for current UI code, but bypasses the OS windowing host. The default build excludes the runner and its renderer dependency.

The sidebar session-list refresh follows the fixed Hermes snapshot's Desktop `/api/sessions` contract and obtains `last_active` through a fresh native HTTP ticket. It keeps the initial authenticated Gateway `session.list` rows as fallback and carries forward any Gateway-only row after the HTTP result, so a session created during refresh is not discarded. The target source revision is fixed at `8783054ef6cbafd86f10892241436862c1171a66`; implementation batches do not follow later upstream commits.

The session row menu supports Pin/Unpin, persisted unread state, and Archive. These writes share the source's `/mutation-snapshot` compare-and-set contract: read the current revision/generation, mint a separate ticket for PATCH, include the mutation request ID and profile, then update UI state only after a positive receipt. Archive also clears `pinned`; a successful archive moves the row out of active sessions. The archived view pages `/api/profiles/sessions?profile=all&archived=only`, preserves profile ownership when IDs collide, and offers profile-scoped fenced Unarchive. Foreign-profile rows cannot open until the chat lifecycle can carry profile scope across resume, prompt, interrupt and prompt responses. Remote connection aggregation, transient unread events, rename and delete remain open.

## Planned control path

`external agent -> authenticated local bridge -> semantic control service -> app handler -> Hermes gateway or DoofusDock transaction -> structured event/receipt -> screenshot/evidence`.

The current chat event renderer also upserts `tool.start` and `tool.complete` events into transcript groups keyed by tool ID and execution generation. It preserves start arguments and displays bounded completion fields; each row exposes its combined speaker and body through an AccessKit group label. This remains a fixture-backed event slice. It does not yet reconcile out-of-order replay or provide the full upstream tool/reasoning card interactions.

The WebSocket reader recognizes server→client JSON-RPC request frames and returns the upstream `-32601` method-not-found response for methods without handlers, including while another RPC is pending; the original RPC continues. `client.capabilities.server_requests` remains false until the UI can receive, surface and answer supported prompt requests. The fallback does not count as interactive server-request support.

For idle inbound `approval` requests, the actor forwards the request ID and params to the app event loop. The app scopes the card to the uniquely matching runtime/profile session pane, renders only supported backend choices, and returns the selected `{choice}` as the JSON-RPC result on that same request ID. The prompt retains its profile so a response can be sent while another pane is active. Requests without a unique owner are declined. The local actor and AccessKit fixtures cover approval and batch clarify routing, with a separate RPC fixture for ordered `clarify.lock` receipts. Active-session ownership across windows and other server-request methods remain open, so capability negotiation stays disabled.

Session-resume commands carry a generated request ID plus stored session ID and profile. The actor echoes that owner tuple in `SessionResumed`; the UI applies a result only when the same profile-scoped cached view still owns that pending request. Success validates the returned session/profile before hydration; errors latch to that view and expose Retry. Disconnect clears pending ownership, while reconnect allows a fresh resume. Authenticated live recovery, authoritative not-found handling and multiwindow ownership remain open.

Read-only diagnostics have narrower scope than mutating actions. Every mutation validates schema, actor, permission, expected state/layout revision and idempotency key. Asynchronous acceptance is not success; report terminal outcomes. The bridge must not be an arbitrary shell/filesystem proxy or bypass Hermes approval.

## Security and lifecycle

Development control is loopback-only with an unpredictable token or equivalently protected local transport. Authenticate writes, bound schemas/payloads, redact sensitive values, audit decisions, support revocation and keep the feature out of production packages. A supervisor should retain the last working build where possible, report failed builds, prevent stale clients from targeting an old process generation, reconnect to the existing Hermes gateway, and never duplicate active submissions. State survival must be explicit by category; do not claim process-local state survives restart without evidence.

## Evidence layers

1. **Core/protocol contracts:** schemas, permissions, persistence, gateway correlation/idempotency and error paths.
2. **Semantic UI integration:** app UI/control service, accessible queries, user-equivalent actions, receipts and headless screenshots.
3. **Physical Windows host:** actual executable, focus, keyboard/IME, native window moves, docking, DPI, multi-monitor and screenshot parity.

Headless screenshots or accepted commands are not physical-host evidence. Visual and functional parity are tracked separately; both are required.

## Current route parity slice: Capabilities

The Capabilities page uses the upstream tab names and keeps `?tab=skills|plugins|connectors|toolsets` in route state. Skills and Toolsets inventories are read through the upstream loopback endpoints `/api/skills` and `/api/tools/toolsets`, each with its own one-use, profile-scoped `native-http` ticket. Requests run on the bounded HTTP worker, with body limits and errors surfaced in the UI. Search is local to the loaded collection. Toolsets can be enabled or disabled through `PUT /api/tools/toolsets/{name}` using a fresh scoped ticket; the row toggle and typed agent action share a handler. The app updates optimistically, validates the server's `{ok,name,enabled}` receipt, and rolls back on failure. The worker processes accepted HTTP commands FIFO. Plugins and Connectors are labeled incomplete. Local fixtures cover HTTP reads and mutation success/mismatch, and protocol coverage rejects disconnected updates; live profile behavior and visual equivalence remain unverified.

## Typography

JetBrains Mono is the primary application UI typeface; its Medium face is used for heading and button text styles. Fira Code is the egui monospace family and is explicitly used by the Hermes message composer. Both are bundled from official tagged releases with their OFL 1.1 license texts and archive hashes in `assets/fonts/README.md`. The original egui fonts remain fallbacks for uncovered glyphs.

## Transcript Markdown

The transcript delegates CommonMark/GFM parsing and code syntax highlighting to egui_commonmark 0.26 with syntect. Cache state lives on the native app instance. The vendored egui_commonmark backend keeps its upstream package license and changes only the fenced-code copy affordance: a sized, accessible Copy/Copied button replaces a bundled-font-incompatible emoji. A feature-gated automation fixture covers emphasis, links, block quotes, ordered/unordered lists, GFM tables, code highlighting, and the copy control. Streamdown tail repair, rich math/media/artifacts, transcript directives, and source-paired screenshot parity remain open.


### Background prompt ownership

Approval and batch clarification requests are assigned by runtime session ID plus profile to exactly one open native session pane. Prompt state retains that profile, and both human controls and registered semantic actions respond through the same handler using the stored owner scope, even while another pane is selected. Requests with missing/ambiguous ownership or a profile conflicting with the attached gateway endpoint fail closed. Local tests verify routing and UI actions; live authenticated gateway receipts remain unverified.


### Primary tab close behavior

`Ctrl+W` (Command+W on macOS) and agent action `close_active_tab` share one app handler. Closing the selected session tile uses its existing busy confirmation. Closing an occupied primary workspace promotes the nearest valid profile-scoped session tile from the workspace group, or from another Dock zone when the group has none, using canonical DoofusDock close/selection transactions before loading that cached view into primary. If there is no tile, primary returns to a clean New session draft. Blank drafts and full-page routes are no-ops. Other source close precedence, including focused terminal and tool tabs, remains incomplete.


### New session draft reuse

The Sessions-pane New session action fronts the primary New session only when it is empty and matches the attached profile. It can also front a tracked tile only when its pane is open, runtime view is known and idle, message/transcript/composer state is empty, session is unlisted and profile matches. Edited drafts are preserved; creation requests remain single-flight. The native surface does not yet implement Bot Mode owner routes, attachments or debounce-flush synchronization.

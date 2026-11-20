# Hermes · DoofusDock

The Messaging route reads profile-scoped channels and pairing queues, applies approved requests with matching receipts, and confirms revocation. Its metadata-driven forms keep secret drafts blank and support normalized allowlists. Telegram QR setup now supports start, status polling, QR display, allowlist configuration, apply and cancel, with profile-bound requests and local HTTP fixtures. Gateway restart execution now follows setup receipts and monitors action status. Bot management and source-paired visual acceptance remain open; this is not a one-to-one replacement yet.

Native Rust and egui desktop shell for Hermes, using DoofusDock's persisted `LayoutDocument` as the workspace geometry authority.

## Current state

The active transcript supports optional local timestamps with millisecond precision. Hermes Native reads the selected profile's `display.timestamps` value from the pinned `/api/config` endpoint; `/timestamps`, `/timestamps on`, `/timestamps off`, and `/timestamps status` provide the same window-only override without submitting a model prompt. A distinct offline review capture is available at `artifacts/agent-ui/transcript-timestamps-20261115.png`; Hover details include the full local date, milliseconds and UTC offset; structured tool lifecycle timestamps and source-paired visual acceptance remain open.

This is the first version of the native Hermes agent-workflow UI. It launches a native four-zone workspace, persists the canonical Dock layout, and wires pane drags to center-tab or edge-split operations. The Hermes gateway adapter supports session browsing, creation, resume, streaming prompts, stop, approval and clarification responses, capability discovery, and scoped session mutations. Prompt submission carries a stable submission ID and validates the returned admission identity; uncertain acknowledgements can be retried with the original ID. Active runtime rename uses revision-fenced `session.mutate` with an exact retry after an uncertain receipt; stored-session rename uses a fenced HTTP patch. Cross-profile session rows retain their profile through resume and chat RPCs. The registered agent action `open_session` takes both session ID and profile; path navigation rejects IDs shared by multiple profiles. The active transcript now uses a native CommonMark renderer with GitHub-style tables, ordered/task lists, inline emphasis and links, block quotes, syntax-highlighted fenced code, and an accessible copy control. Long sessions expose a profile-scoped, bounded “Show earlier messages” read path with order validation and stale-session guards; virtualized scroll-window parity and live gateway acceptance remain open. Streaming repairs, media, transcript directives, specialized math, artifact promotion, and full source Streamdown styling remain incomplete. The Dock model has a tested foundation for dynamic, profile-scoped session-pane descriptors and canonical add/select/close operations. Route switching now retains each profile+session transcript and composer draft in a bounded in-memory view cache, and uniquely identified background message/tool events update the correct cached session. Each dynamic Dock pane renders its own transcript and composer from a bounded in-memory profile+session view cache; uniquely identified background events and submission receipts update the matching cached session. Existing sessions open in Dock tiles, blank sessions remain unlisted until their first admitted prompt, and closing a busy tab confirms while leaving Hermes running. Approval and batch clarification requests route to their unique profile-scoped pane, and semantic actions retain that scope when responding. Session resume receipts carry request, session and profile identity; stale results are ignored and recoverable errors render with Retry in their owning pane. Authenticated live receipts, Bot Mode owner-route handling, attachment-aware drafts and full close-focus/overflow parity remain open. New session reuses only an empty profile-matching main draft or a known idle, empty, unlisted tile; edited drafts remain in place. Primary-pane close now follows the pinned source: `Ctrl+W` and agent action `close_active_tab` promote the nearest session tile or reset an occupied primary chat to New session. The close target now prefers a session tile in the hovered or last-clicked eligible Dock tab group before falling back to workspace; keyboard-focus traversal and tool-tab handoff remain incomplete. The Doofus bridge and many Hermes screens/workflows are still incomplete. No sample conversation, task, model, terminal, or vault data is fabricated. Live drag/drop persistence has not yet been exercised.

The first-version target is a usable agent-workflow UI against the downloaded Hermes source snapshot `8783054ef6cbafd86f10892241436862c1171a66`, documented in [the reviewed research package](docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/README.md). Its scope is the features needed to use Hermes as an agent: model/provider connection, chat and prompt lifecycle, tool execution/results/approvals, skills, plugins/MCP, and supporting context and attachments. Pixel matching and unrelated Desktop administration are deferred so visual work can follow functional workflows. The fixed source snapshot remains the behavior reference; later upstream changes do not expand the scope. Portable Codex/Antigravity/Hermes checkpoints and manual handoff are tracked in [docs/AGENT_HANDOFF.md](docs/AGENT_HANDOFF.md).

First-run remote setup now refreshes near-expiry native OAuth tokens and stores OAuth sessions and static session tokens encrypted with Windows DPAPI under Hermes Native's isolated profile. Apply starts an authenticated remote WebSocket attach, and saved credentials can be loaded for reconnect at startup. Remote session/history/search/capability reads and session metadata/toolset writes now use the tested token or OAuth credential. Remaining gaps include other remote HTTP-backed features, live service verification, legacy embedded login, and local bootstrap.

## Run on Windows

Requirements: Rust 1.95 or later, Cargo, and a Windows desktop session.

```powershell
cargo run
```

The saved layout is stored under `%APPDATA%\HermesDoofusDock\layouts\hermes-main.json` using DoofusDock's atomic `LayoutStore`.

## Agent UI control

`cargo run --features agent-automation --bin hermes-automation -- --stdio` starts an isolated headless UI with versioned JSON requests over inherited stdio. It exposes the accessibility tree, registered actions, state, events, semantic scrolling and exact text-field fill, and screenshots without opening a native window or connecting to the Hermes gateway. A successful action advances the UI frame before state or screenshot readback. Native window control and authenticated live-gateway automation remain to be implemented.

## Validation

```powershell
cargo check --all-targets
```

See [the implementation backlog](docs/BACKLOG.md) for the remaining bridge, application widgets, support functions, and source-paired parity work.

Artifacts parity status: `/artifacts` now indexes up to 30 recent sessions on a background gateway worker, extracts transcript artifacts, and provides type filters, search, image previews, file/link tables, pagination, open actions and source-chat navigation. Remote file downloads use authenticated `/api/fs/download` with path, session and profile scope; gateway-local image paths use authenticated `/api/fs/read-data-url` and render as bounded thumbnails. Exact route/query-state parity, live-gateway checks and source-paired acceptance remain open.

Telegram QR onboarding includes per-user allowlist editing with validated Add/Enter input, removable ID chips, detected-owner status, empty-list apply rejection and a configured-bot replacement warning. Credential lifecycle management and source-paired visual acceptance remain open.

Telegram QR onboarding now uses the source's two-column setup layout, placing the 224 px QR and expiry beside the user and allowlist controls.

Scheduled Jobs now has profile-scoped list/details, Pause/Resume, Run now/Stop, latest-20 history, create/edit/delete, delivery targets, model selection, and typed automation blueprints. Mutations use authenticated bounded gateway transport with profile/request fencing; local fixtures and headless form tests cover the core flows. Run-to-session navigation, live authenticated behavior, localization and source-paired acceptance remain open.

Command Center Sessions uses the pinned profile-scoped sidebar API, per-profile truncation state and 50-row Load more requests, retaining cached rows when a profile scan fails. Its four source sections are reachable; exact search debounce, live gateway behavior, Windows and source-paired visuals remain open.

The native PAR-0011 session synchronization remains partial: request/profile-fenced session and transcript refresh, active-status recovery, and single/all-profile project trees are covered by headless tests. Project overview “Show all” hydrates single-profile rows; all-profile entry uses the aggregate tree without a synthetic profile RPC. Expanded rows page by 50 and filter late archived rows by lifecycle generation. A persisted “Show all sessions” preference requests and displays the pinned-source 2000-row tree window, with 3 rows as the default. Local create/rename/completion publishes bounded opaque peer hints. Explicit project removal has confirmation, profile-scoped mutation, stale-read tombstones and rollback; files and sessions remain intact. Auto-project hiding is local, shared across native windows, and offers Undo for eight seconds. Profile-scoped local/remote repository discovery now has headless implementation and tests; live gateway, physical Windows behavior, project-list metadata reconciliation and source-paired acceptance remain open.

The Plugins catalog has a separate, resizable detail pane with persisted width and double-click reset. Its source-paired Windows acceptance remains open.

The session-change path now includes a safe, read-only refresh of the active idle transcript. It retains loaded history and drafts and rejects stale or non-overlapping results; peer transcript/tile sync, busy-deferral, live gateway and source-paired verification remain open.

The Hermes Native parity work now has portable provider-neutral checkpoints in `.doofus/`, plus local peer transcript refresh for matching active/open session views. See [agent handoff](docs/AGENT_HANDOFF.md) and the [Hermes Native completion overview](apps/hermes-native/docs/COMPLETION_OVERVIEW.md) for verified scope and remaining gaps.


PAR-0011 project metadata now uses pinned profile-scoped `projects.list` alongside a separate `projects.tree` snapshot. The active-project marker reads only the list-owned pointer; profile/connection/generation fences and error-cache behavior have headless coverage. This remains partial and does not change the fixed 499-row parity gate.

Hermes Native continuity/progress tooling is documented in [docs/AGENT_HANDOFF.md](docs/AGENT_HANDOFF.md). Use python apps/hermes-native/tools/doofus_task.py status --json, validate, and progress --json to inspect persisted handoff state and separate parity evidence from implementation review.


The native Sessions sidebar and File browser have persisted semantic show/hide controls, toolbar buttons, and shared typed agent actions. The agent toggle accepts the pinned source aliases `chat-sidebar` and `file-browser`. Ctrl/Cmd+B and Ctrl/Cmd+J first fold an eligible root-edge group through DoofusDock's canonical minimize operation, with semantic fallback; minimized groups show selected-tab restore rails. Dedicated Hide/Show left/right titlebar buttons share the shortcut handler. Rows nested under column roots are resolved, while nested split edges use semantic fallback. Additional root/profile shapes, live Windows input, and source-paired acceptance remain open. Semantic-owner cross-restore retains the canonical group and active tab. See the [completion overview](apps/hermes-native/docs/COMPLETION_OVERVIEW.md).

Narrow viewport overlays now include the pinned source's built-in Sessions, Files, Review and Bots panes. Bots is a HideOnly Sessions-group tab with a profile-scoped read-only `profiles.list` roster and transient narrow reveal. The same roster rows are available to native automation state. Conditional Bot Mode lifecycle, boot-time dock enforcement, cross-gateway agent union, bot management, exact zone sizing, physical Windows behavior and source-paired acceptance remain incomplete; see Hermes Native PLAN Revision 210.

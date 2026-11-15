# Source audit

## Pins

- Hermes fixed parity snapshot: `NousResearch/hermes-agent@8783054ef6cbafd86f10892241436862c1171a66` (downloaded 2026-11-14; immutable local ref `refs/codex/parity/hermes-8783054`).
- DoofusDock: `robertheadley/doofusdock@f6eb49b9e488f4e851d8d3abe18e95d296201bba`.
- Historical pins: supplied ZIP `ffdc7eeb25343df1a7ee84503ba552538db9af72`; prior audit `6da444ebda7533429b655aeca2b02d77fd900238`; implementation pin `8358fc7bb4ce18fbee8d4bf6659dd155c2a03783`; previous current pin `12bf6ef4d40691ef207ae3a35f238dec52967662`.
- The `8358fc7` → `491f984` gateway diff contains 19 files with 992 insertions and 61 deletions, including native-session recovery and restored delivery. `491f984` → `12bf6ef` added five Windows installer-loopback test files only. `12bf6ef` → `5163cc6` changed 54 files; the Desktop delta is debug-share privacy copy and a comment, with no gateway changes. Desktop route/layout/mode/workflow source remains unchanged across the audited interval, aside from that user-facing copy. The protocol adapter uses `5163cc6` plus the gateway changes from `491f984`.
- The 2026-11-13 supplied reference package is now preserved at `docs/reference/Hermes_Upstream_UI_DoofusDock_Rust_Parity_2026-11-13/`. Its manifest hashes and offline consistency validation passed (29 listed files; 491 capabilities; 491 requirements and acceptance cases). Its historical map source pin remains `ffdc7eeb`; this pin difference is recorded, not silently rewritten.
- Source checkout: `../../.staging/hermes-upstream-8358fc` from this crate. Its working tree remains at `8358fc7`; `FETCH_HEAD` resolves to `5163cc6`, and it is isolated from the installed Hermes application.

### Snapshot selection audit — 2026-11-14

The source was resolved from Hermes `main` once to select the downloaded parity snapshot. The `5163cc6..a7904dc` Desktop diff is limited to `apps/desktop/src/components/assistant-ui/markdown-text.tsx` and `apps/desktop/src/components/chat/zoomable-image.tsx`; no Capabilities route or gateway source changed. Later upstream changes do not alter this project target.

### Fixed snapshot source review — 2026-11-14

Commit `8783054ef6cbafd86f10892241436862c1171a66` was fetched as the project snapshot. The `a7904dc..8783054` comparison spans 1,070 repository files; 46 Desktop files changed (3,118 additions, 395 deletions), with seven `tui_gateway` files changed. Desktop changes include composer actions and paste limits, profile-scoped session list reads, transcript paging/backfill and Bot Mode session routing. No Capabilities files changed in this interval. The generated `SessionInterruptParams` shape and `tui_gateway/methods_session_interrupt.py` are unchanged from `a7904dc`; the native Stop action uses `session_id` and validates `interrupted|not_interrupted`. This snapshot delta is not fully reconciled, so the capability matrix remains pending and no broader parity status is inferred. No scheduled or per-batch re-resolution of Hermes `main` is part of the plan.

## Advanced default tree

Source: `apps/desktop/src/app/contrib/layout-presets.ts`.

```text
row [1, 3.4, 1.25]
├── group(sessions)
├── group(workspace)
└── column [1.6, 1]
    ├── row [1, 1.2]
    │   ├── group(review)
    │   └── group(files)
    └── group(terminal)
```

`review`, `files`, and `terminal` have independent resting/visibility behavior. A file preview is a dynamic pane beside the file tree. Session/preview IDs are not written into reusable geometry. Source pane IDs are `sessions`, `workspace`, `review`, `files`, and `terminal`.

## Source presets and mode policy

- Advanced default is the source Default preset; Advanced preserves user display preferences.
- Basic: sessions; workspace above a resting terminal; review/files in their own right column.
- Focus: sessions plus workspace/files/review tabs above the terminal.
- Terminal Deck: sessions/workspace/files+review tabs above a bottom terminal.
- Quad: sessions+files tabs and workspace above terminal/review.
- Sidebar left/right are Simple-tier Basic layouts; Demo is unlisted and chat-only.
- Simple is a preference-shadowing mode, separate from the selected layout. Effective visibility is `sessionReveal ?? policy[mode] ?? userPreference`. Simple rests files, review and terminal closed and applies other source policy values; it does not overwrite Advanced preferences.
- Sidebar width defaults to 237px (max 360px); below a 640px dock width the rails become reveal overlays.

## Routes

Workspace pages: `/`, `/:sessionId`, `/capabilities`, `/messaging`, `/artifacts`, and contributed full-page routes. Overlays: `/session-import`, `/agents`, `/command-center`, `/cron`, `/profiles`, `/settings`, `/starmap`, `/webhooks`. Contributed paths must not be parsed as session IDs.

The current-source `apps/desktop/src/app/routes.ts` file is unchanged since `8358fc7`. Reserved pages and overlays are distinct route views; query/hash are removed before classification; contributed full-page routes must be excluded from session-ID parsing. First-party workspace pages retain fixed titlebar controls; overlays hide those clusters. Rust classifies reserved and contributed routes and exposes Capabilities, Messaging, Artifacts, Scheduled jobs and Settings navigation. Capabilities now has source-style tabs and profile-scoped Skills/Toolsets reads, but Plugins/Connectors and capability management workflows remain incomplete. Messaging/Artifacts remain honest offline placeholders; Settings/Cron use a generic modal. Detailed chrome behavior and accessibility parity remain unimplemented.

The installed Hermes v0.22.5 CLI supports `gateway ensure` but not `gateway attach`. Rust therefore uses `ensure` for the credential-free endpoint and the same installed-runtime Python ticket bridge as the pinned Desktop source. It mints a fresh `interactive` grant, sends it only as a WebSocket subprotocol, rejects redirects, verifies the canonical echo, waits for `gateway.ready`, sends `client.capabilities`, verifies `runtime.describe` identity and reads `session.list`. The worker remains available for `session.resume`; the Windows acceptance run loaded two real session rows and rendered a 295-message transcript. Rust submits prompts from resumed sessions and has a partial fresh-session path using `session.create` followed by `prompt.submit`; the runtime ID scopes RPCs and the stored ID is used for navigation. The Sessions pane loads the pinned Desktop `/api/sessions` list through a fresh one-use native HTTP ticket and displays `last_active || started_at`; if that read fails, the authenticated Gateway `session.list` remains visible and a warning appears in the Sessions pane. HTTP rows merge ahead of Gateway-only sessions so a concurrently created session is retained. The pane also searches full stored history through `/api/sessions/search`, preserves backend rank/snippets and appends loaded-only local matches. Local HTTP fixtures exercise list recency and search ranking/DTO decoding. Basic message start/delta/complete notifications are projected. Local WebSocket fixtures verify create then submit, ID roles, existing-session submit, event ordering and fractional timestamp parsing. Full source create options/retry, retained admission identity/idempotency, server-request replies, replay, reconnect, heartbeat, profile switching and full event semantics remain open.

The session-row Pin/Unpin action follows the pinned Desktop mutation path: read `/api/sessions/{id}/mutation-snapshot` under a one-use ticket, then PATCH the pin value with a second ticket and `request_id`, `expected_revision`, `expected_generation`, and explicit named-profile scope. The UI waits for `{ok:true}` before changing local state; failed or mismatched receipts leave the row unchanged. A local HTTP fixture covers the exact profile query, ticket separation, mutation fence and response. Live profile mutation acceptance remains open.

The same fixed-source mutation contract now supports persisted unread state with `unread:true|false`; the HTTP session-list DTO also supplies its initial unread bit. The visible menu toggles that persisted bit, with separate dynamic agent locators. The transient finished-response unread dot and its read acknowledgement behavior are not yet implemented.

Archive parity in the fixed snapshot now uses the same session mutation-snapshot fence and separate one-use tickets as Pin/unread. The PATCH sends `archived:true` and `pinned:false`; only a matching positive receipt removes the row from active sessions and search results. A dedicated view pages `/api/profiles/sessions?archived=only&profile=all` using a new one-use ticket per fixed-offset page and deduplicates pinned rows by profile plus stored ID. Archived rows expose profile-scoped Unarchive through the same fenced mutation; confirmed restores move back to active sessions. Local HTTP/pagination/UI/locator fixtures cover the query and mutation. Foreign-profile rows remain non-openable until all chat RPCs carry profile scope; remote-connection aggregation and live-profile acceptance remain open.

## Baseline and current gaps

The archive maps 491 in-scope stock capabilities and defers 108 out-of-scope records. It supplies source maps and a test matrix but no source/target screenshots. All capability statuses remain pending until source reconciliation and evidence. The previous parent-workspace prototype has four Doofus/Hermes zones, no upstream layout or mode parity, no Hermes Gateway adapter, and an out-of-scope Doofus pane. This crate is separate; no edits were made to that prototype or to the installed Hermes app.

Current DoofusDock APIs selected: `LayoutDocument` + `DockingService` for the one canonical layout; `WorkspaceProfile` / `apply_workspace_profile` / `plan_profile_switch` for source geometry; `Morph` and `render_visual_snapshot` only in the later profile stage; `doofusdock-render-egui` for panel chrome and tabs; `doofusdock-host-egui` for native viewport support. The Hermes app owns all chat, session, terminal, file, Git, settings and other Hermes widgets.

## Source preview setup

The preview at `8358fc7` was installed/launched from the isolated source checkout with Node 26.12.1 and pnpm 12.15.0, a temporary `HERMES_HOME`, `HERMES_DESKTOP_USER_DATA_DIR`, `APPDATA`, and `LOCALAPPDATA`. Initial observation showed the Sessions sidebar, New session workspace and Setup Hermes Desktop overlay with Backend offline. Later the isolated backend start timed out and the source displayed its recovery dialog. The isolated checkout needed a declaration-only `.d.mts` shim because its electron TypeScript build did not resolve an imported test `.mjs` helper; no runtime source file was changed. The shim remains untracked in that isolated preview checkout after local command policy rejected cleanup. No saved first-run/source screenshots or matched target/source captures are in the project; physical parity remains open.


### Fixed-snapshot acceptance inventory classification — 2026-11-15

The preserved 565-row research package combines 499 stock Hermes Desktop rows and 66 Hermes-Doofus rows. The approved HUR-001 scope excludes the Hermes-Doofus plugin UI/backend, so only the 499 Desktop rows define the 1:1 Hermes UI acceptance target. The 66 rows remain preserved and pending as historical supplemental material. Their research source pin remains `0278571393b0ae98fd016b67cfd32f66cee986ee`.

The 499 Hermes Desktop acceptance rows were authored against `415fa335b4d98ecf00e29e88625bb478dc63fb67`; their matrix `source_snapshot` field and package target manifest now identify the selected fixed implementation target `8783054ef6cbafd86f10892241436862c1171a66`. All rows remain pending. This records the intended test target, not per-row source verification. Detailed provenance and limitations are in `docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/planning/TARGET_SNAPSHOT_RECONCILIATION.md`.

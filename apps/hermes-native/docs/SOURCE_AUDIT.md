# Source audit

## Pins

- Hermes fixed parity snapshot: `NousResearch/hermes-agent@8783054ef6cbafd86f10892241436862c1171a66` (downloaded 2026-11-14; immutable local ref `refs/codex/parity/hermes-8783054`).
- DoofusDock: `robertheadley/doofusdock@f23ed02a99c7f8c57f83164d98feadb17cc4ed87`.
- Historical pins: supplied ZIP `ffdc7eeb25343df1a7ee84503ba552538db9af72`; prior audit `6da444ebda7533429b655aeca2b02d77fd900238`; implementation pin `8358fc7bb4ce18fbee8d4bf6659dd155c2a03783`; previous current pin `12bf6ef4d40691ef207ae3a35f238dec52967662`.
- The `8358fc7` → `491f984` gateway diff contains 19 files with 992 insertions and 61 deletions, including native-session recovery and restored delivery. `491f984` → `12bf6ef` added five Windows installer-loopback test files only. `12bf6ef` → `5163cc6` changed 54 files; the Desktop delta is debug-share privacy copy and a comment, with no gateway changes. Desktop route/layout/mode/workflow source remains unchanged across the audited interval, aside from that user-facing copy. The protocol adapter uses `5163cc6` plus the gateway changes from `491f984`.
- The 2026-11-13 supplied reference package is now preserved at `docs/reference/Hermes_Upstream_UI_DoofusDock_Rust_Parity_2026-11-13/`. Its manifest hashes and offline consistency validation passed (29 listed files; 491 capabilities; 491 requirements and acceptance cases). Its historical map source pin remains `ffdc7eeb`; this pin difference is recorded, not silently rewritten.
- Source checkout: `../../.staging/hermes-upstream-8358fc` from this crate. Its working tree remains at `8358fc7`; `FETCH_HEAD` resolves to `5163cc6`, and it is isolated from the installed Hermes application.

### Snapshot selection audit — 2026-11-14

The source was resolved from Hermes `main` once to select the downloaded parity snapshot. The `5163cc6..a7904dc` Desktop diff is limited to `apps/desktop/src/components/assistant-ui/markdown-text.tsx` and `apps/desktop/src/components/chat/zoomable-image.tsx`; no Capabilities route or gateway source changed. Later upstream changes do not alter this project target.

### Fixed snapshot source review — 2026-11-14

Commit `8783054ef6cbafd86f10892241436862c1171a66` was fetched as the project snapshot. The `a7904dc..8783054` comparison spans 1,070 repository files; 46 Desktop files changed (3,118 additions, 395 deletions), with seven `tui_gateway` files changed. Desktop changes include composer actions and paste limits, profile-scoped session list reads, transcript paging/backfill and Bot Mode session routing. No Capabilities files changed in this interval. The generated `SessionInterruptParams` shape and `tui_gateway/methods_session_interrupt.py` are unchanged from `a7904dc`; the native Stop action uses `session_id` and validates `interrupted|not_interrupted`. This snapshot delta is not fully reconciled, so the capability matrix remains pending and no broader parity status is inferred. No scheduled or per-batch re-resolution of Hermes `main` is part of the plan.

### Session peer invalidation — pinned source

Source is `apps/desktop/src/store/session-sync.ts` at `refs/codex/parity/hermes-8783054` in the isolated source repository. It constructs `BroadcastChannel('hermes:sessions')`; `broadcastSessionsChanged()` posts the numeric value `1`, and `onSessionsChanged(handler)` subscribes/unsubscribes or no-ops when BroadcastChannel is unavailable. The message is only an invalidation ping. `apps/desktop/src/app/contrib/hooks/use-desktop-integrations.ts` listens in the primary non-browser window and calls `refreshSessions()` to re-read canonical data. The staged source checkout's worktree HEAD is older (`8358fc7`), so `git show` against the pinned ref is required; staged working files are not evidence for this contract.

Pinned local callers include session creation/title changes in `apps/desktop/src/app/session/hooks/use-session-actions/index.ts`, session-tile upsert in `apps/desktop/src/app/chat/session-tile-actions.ts`, and coalesced turn completion in `apps/desktop/src/app/session/hooks/use-message-stream/index.ts`. The separate `hermes:transcript` channel carries stored-session identity and optional message count and has a distinct transcript/tile hydration contract; it is not implemented by the session-list ping.

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

The current-source `apps/desktop/src/app/routes.ts` file is unchanged since `8358fc7`. Reserved pages and overlays are distinct route views; query/hash are removed before classification; contributed full-page routes must be excluded from session-ID parsing. First-party workspace pages retain fixed titlebar controls; overlays hide those clusters. Rust classifies reserved and contributed routes and exposes Capabilities, Messaging, Artifacts, Scheduled jobs and Settings navigation. Capabilities now has source-style tabs and profile-scoped Skills/Toolsets reads, but Plugins/Connectors and capability management workflows remain incomplete. Messaging now loads a profile-scoped channel inventory through `/api/messaging/platforms` and renders counts, search/status filters, selection, channel details, and explicit loading/offline/error/retry states. Owner changes and disconnects clear in-flight and cached channel data. Configuration, pairing/QR workflows, bot management, source styling/localization, and source-paired Windows acceptance remain open; PAR-0282 is pending. Artifacts indexes connected gateway sessions with search, type filters and source-session navigation; remote file downloads and bounded remote image previews use authenticated path/session/profile scope, while source-paired acceptance remains open. Settings remains a generic modal; Cron now has a profile-scoped searchable list/detail view, Pause/Resume and latest-20 run history with retryable read errors. Trigger/Stop, run-session navigation, create/edit, delete and blueprints remain open. Detailed chrome behavior and accessibility parity remain unimplemented.

Messaging channel enable/disable now uses the pinned `PUT /api/messaging/platforms/{id}` endpoint with profile query scope, token/OAuth header auth for remote gateways, and the one-use ticket header for local loopback. The app validates request ID, platform ID, target enabled state, profile, and returned platform receipt before updating displayed state. `hot_served` determines whether it reports live application or a required restart. Configuration fields, pairing/QR workflows, bot management, and source-paired Windows interaction/visual acceptance remain unimplemented; PAR-0282 remains pending.

The installed Hermes v0.22.5 CLI supports `gateway ensure` but not `gateway attach`. Rust therefore uses `ensure` for the credential-free endpoint and the same installed-runtime Python ticket bridge as the pinned Desktop source. It mints a fresh `interactive` grant, sends it only as a WebSocket subprotocol, rejects redirects, verifies the canonical echo, waits for `gateway.ready`, sends `client.capabilities`, verifies `runtime.describe` identity and reads `session.list`. The worker remains available for `session.resume`; the Windows acceptance run loaded two real session rows and rendered a 295-message transcript. Rust submits prompts from resumed sessions and has a partial fresh-session path using `session.create` followed by `prompt.submit`; the runtime ID scopes RPCs and the stored ID is used for navigation. The Sessions pane loads the pinned Desktop `/api/sessions` list through a fresh one-use native HTTP ticket and displays `last_active || started_at`; if that read fails, the authenticated Gateway `session.list` remains visible and a warning appears in the Sessions pane. HTTP rows merge ahead of Gateway-only sessions so a concurrently created session is retained. The pane also searches full stored history through `/api/sessions/search`, preserves backend rank/snippets and appends loaded-only local matches. Local HTTP fixtures exercise list recency and search ranking/DTO decoding. Basic message start/delta/complete notifications are projected. Local WebSocket fixtures verify create then submit, ID roles, existing-session submit, event ordering and fractional timestamp parsing. Full source create options/retry, retained admission identity/idempotency, server-request replies, replay, reconnect, heartbeat, profile switching and full event semantics remain open.

The session-row Pin/Unpin action follows the pinned Desktop mutation path: read `/api/sessions/{id}/mutation-snapshot` under a one-use ticket, then PATCH the pin value with a second ticket and `request_id`, `expected_revision`, `expected_generation`, and explicit named-profile scope. The UI waits for `{ok:true}` before changing local state; failed or mismatched receipts leave the row unchanged. A local HTTP fixture covers the exact profile query, ticket separation, mutation fence and response. Live profile mutation acceptance remains open.

The same fixed-source mutation contract now supports persisted unread state with `unread:true|false`; the HTTP session-list DTO also supplies its initial unread bit. The visible menu toggles that persisted bit, with separate dynamic agent locators. The transient finished-response unread dot and its read acknowledgement behavior are not yet implemented.

Archive parity in the fixed snapshot now uses the same session mutation-snapshot fence and separate one-use tickets as Pin/unread. The PATCH sends `archived:true` and `pinned:false`; only a matching positive receipt removes the row from active sessions and search results. A dedicated view pages `/api/profiles/sessions?archived=only&profile=all` using a new one-use ticket per fixed-offset page and deduplicates pinned rows by profile plus stored ID. Archived rows expose profile-scoped Unarchive through the same fenced mutation; confirmed restores move back to active sessions. Local HTTP/pagination/UI/locator fixtures cover the query and mutation. Foreign-profile rows remain non-openable until all chat RPCs carry profile scope; remote-connection aggregation and live-profile acceptance remain open.

## Baseline and current gaps

The archive maps 491 in-scope stock capabilities and defers 108 out-of-scope records. It supplies source maps and a test matrix but no source/target screenshots. All capability statuses remain pending until source reconciliation and evidence. The previous parent-workspace prototype has four Doofus/Hermes zones, no upstream layout or mode parity, no Hermes Gateway adapter, and an out-of-scope Doofus pane. This crate is separate; no edits were made to that prototype or to the installed Hermes app.

Current DoofusDock APIs selected: `LayoutDocument` + `DockingService` for the one canonical layout; `WorkspaceProfile` / `apply_workspace_profile` / `plan_profile_switch` for source geometry; `Morph` and `render_visual_snapshot` only in the later profile stage; `doofusdock-render-egui` for panel chrome and tabs; `doofusdock-host-egui` for native viewport support. The Hermes app owns all chat, session, terminal, file, Git, settings and other Hermes widgets.

## Source preview setup

The preview at `8358fc7` was installed/launched from the isolated source checkout with Node 26.12.1 and pnpm 12.15.0, a temporary `HERMES_HOME`, `HERMES_DESKTOP_USER_DATA_DIR`, `APPDATA`, and `LOCALAPPDATA`. Initial observation showed the Sessions sidebar, New session workspace and Setup Hermes Desktop overlay with Backend offline. Later the isolated backend start timed out and the source displayed its recovery dialog. The isolated checkout needed a declaration-only `.d.mts` shim because its electron TypeScript build did not resolve an imported test `.mjs` helper; no runtime source file was changed. The shim remains untracked in that isolated preview checkout after local command policy rejected cleanup. No saved first-run/source screenshots or matched target/source captures are in the project; physical parity remains open.


### Fixed-source transcript paging — 2026-11-15

The selected `apps/desktop/src/api/sessions.ts` contract uses a 120-message `latest` tail and fetches older pages with `include_compacted=true`; `apps/desktop/src/app/chat/index.tsx` expands a bounded transcript window and routes older-history reads through `transcript-backfill.ts`. Hermes Native now implements the bounded stored-message request, validates the session/profile and echoed latest-page cursor, exposes an accessible history action, and discards late results after route/profile changes. This is a first page-loading slice only. Unpaginated legacy full-transcript responses are accepted under the shared 512 KiB transport cap and a 10,000-row guard. It does not implement the source's virtualized transcript rail/window, loading anchored to the scroll boundary, retention, or scroll-position restoration, and it has not been validated against a live Hermes gateway.

### Fixed-snapshot acceptance inventory classification — 2026-11-15

### Fixed-snapshot remote connection contract — 2026-11-16

The 2026-11-16 source review used only the pinned `8783054ef6cbafd86f10892241436862c1171a66` files. `apps/desktop/electron/connection-config.ts` and `main.ts` define remote URL/auth resolution and preflight before configuration apply; `web/src/lib/gatewayClient.ts` uses the shared JSON-RPC WebSocket channel at `/api/ws`. Token mode uses the Hermes session-token HTTP/WS auth paths; OAuth mode uses the authenticated ticket flow for WebSocket upgrades. This differs from local `hermes gateway ensure` and native control-socket ticketing. The native client now has an explicit remote auth context across WebSocket and the listed HTTP read/write calls. Source review plus local fixtures are design/transport evidence only; live runtime behavior and parity rows remain unaccepted.

### Remote HTTP transport implementation status — 2026-11-16

The endpoint auth context is either a zeroizing session token or a shared OAuth session protected by a mutex. OAuth refresh runs before remote HTTP/WebSocket operations when expiry is unknown or within the 60-second window; rotated tokens are saved to the same DPAPI credential envelope. Remote GET/PUT/PATCH helpers send `X-Hermes-Session-Token` or `Authorization: Bearer`, disable redirects, cap JSON responses at 512 KiB, and require same scheme/host/port plus the normalized gateway path prefix. Covered APIs are active/archived/all-profile sessions, older transcript pages, session search, skills, toolsets, display timestamps, toolset enablement, and session pin/unread/archive/rename. Local gateway calls continue to use fresh one-use tickets. Local fixtures verify token auth header, path boundary, mutation snapshot and fenced PATCH; OAuth refresh persistence and a real remote service are not verified. Acceptance stays pending.

### Scope and provenance

The preserved 565-row research package combines 499 stock Hermes Desktop rows and 66 Hermes-Doofus rows. The approved HUR-001 scope excludes the Hermes-Doofus plugin UI/backend, so only the 499 Desktop rows define the 1:1 Hermes UI acceptance target. The 66 rows remain preserved and pending as historical supplemental material. Their research source pin remains `0278571393b0ae98fd016b67cfd32f66cee986ee`.

The 499 Hermes Desktop acceptance rows were authored against `415fa335b4d98ecf00e29e88625bb478dc63fb67`; their matrix `source_snapshot` field and package target manifest now identify the selected fixed implementation target `8783054ef6cbafd86f10892241436862c1171a66`. All rows remain pending. This records the intended test target, not per-row source verification. Detailed provenance and limitations are in `docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/planning/TARGET_SNAPSHOT_RECONCILIATION.md`.

### Remote Artifacts image preview — 2026-11-16

The pinned `apps/desktop/src/lib/media.ts` routes gateway-owned image paths through `/api/fs/read-data-url`; `artifact-utils.ts` delegates Artifact cards to that shared resolver. The native implementation now preserves path/session/profile ownership, sends token or OAuth bearer auth without a ticket in the URL, caps the response and decoded source image, and emits a 512×384-bounded aspect-preserving RGBA thumbnail from a background worker. Local HTTP fixtures verify endpoint query and token header. The pinned source retries external HTTP(S) images through `/api/media/proxy`; the native renderer now uses that authenticated profile-scoped endpoint and decodes its returned data URL. No live remote service or side-by-side source screenshot was used, so acceptance rows remain pending.

The `/cron` route now reads and displays the active profile's bounded job inventory with searchable name/ID/prompt/schedule, sorted list and selected-job detail, plus loading/offline/error/retry/empty states. Remote HTTP fixture verifies profile query and credential header; results are fenced by request and profile. Job mutations, run controls/history, delivery targets, blueprints, localized source visuals and source-paired Windows acceptance remain open.

Scheduled Jobs now supports profile-scoped pause/resume and loads the latest 20 runs for the selected job. The runs read uses GET /api/cron/jobs/{id}/runs?limit=20&profile=<profile>, encoded IDs, remote token/OAuth or verified local ticket auth, bounds and duplicate rejection; headless UI shows loading, empty, retryable error and run rows. Fixtures pass. Trigger/Stop, clickable run navigation, create/edit, delete, delivery/blueprints, live gateway behavior and source-paired Windows acceptance remain open; no capability row advances.

### Transcript peer refresh follow-up (2026-11-17)

The pinned `store/transcript-sync.ts` sends a `hermes:transcript` invalidation with the stored session ID and optional message count after a completed turn. The pinned `use-transcript-peer-sync.ts` listener refreshes only a matching idle active session and matching open session tiles, resolving each tile owner profile and deduplicating equivalent runtime targets. Native now models that as a typed local event with no transcript body, targets the active route and open session-pane descriptors, and re-reads through the existing profile-scoped latest-page endpoint. It checks endpoint/profile/runtime and current pane identity again at receipt, and applies only if the transcript fingerprint and idle state remain unchanged. This is source-shaped headless behavior; it does not prove actual Windows peer delivery or authenticated live refresh.

PAR-0011 pinned live-state refresh (reviewed 2026-11-17T17:42:52Z): `use-background-sync.ts:801-947,1345-1408` uses profile-scoped `session.active_list`, request-time session-state snapshots, same-profile absent-runtime reaping, reconnect/session-change reads, one trailing coalesced pass, and a 30-second visible/focused backstop (1.5 seconds for legacy gateways). Native implements the typed RPC, profile/connection fencing, event revisions, local row indicators and a focused backstop; watchdog settling, method-not-found fallback, source visibility events and live acceptance remain open. Project sync is broader than the native `projects.changed` refresh: pinned `use-background-sync.ts:1410-1590` refreshes list/tree and session/tile data, while `sidebar/index.tsx:800-968` adds grouped-tree/repo-scan focus behavior and `projects.ts:526-625` owns generation/profile/connection stale checks. Native requests authoritative single-profile `projects.tree` or the separate all-profile `/api/profiles/projects/tree` endpoint; aggregate rows retain owning profile, partial profile errors stay visible, and the 60-second bounded read runs on a dedicated worker. Both views render project → repo → group → session previews in the Sessions sidebar with accessible owner-aware open actions. Request/profile/gateway/authority/generation fencing and cache preservation are tested; focus regain refreshes the tree. Entered single-profile projects rehydrate when their overview node changes; archive lifecycle generations prevent a late `projects.project_sessions` response from restoring an archived row. The pinned filter-menu `sidebarShowAllSessions` preference is persisted natively and selects a preview limit of 3 or 2000 in both tree query paths; hydrated project rows independently page by 50. `projects.list` has a separate explicit-profile cache and active pointer; explicit project deletion now has confirmation, rollback and tree-confirmed tombstones. Auto-project dismiss/Undo is local-only and shared between native windows. Full list/tree metadata reconciliation, all-profile nested `project_sessions` hydration, local/remote repo discovery, cross-gateway synthetic project rows, and paired visual/live acceptance remain open.


PAR-0011 project-list and removal behavior (reviewed 2026-11-17): pinned `store/projects.ts#refreshProjects` loads the `{projects, active_id}` snapshot for an explicit profile, with generation and gateway/profile ownership checks; `types/hermes.ts#ProjectInfo` defines the metadata. Native reads this separately from `projects.tree`, refreshes on connection, matching `projects.changed`, and focus recovery, and displays the list-owned active pointer without mutating tree `active_id`. Pinned `project-menu.tsx#useProjectActions` confirms removal before `store/projects.ts#deleteProject` sends `projects.delete` for one writable profile; this removes the Hermes project record and preserves its files and sessions. Native now has the explicit-project confirmation, profile-scoped RPC, optimistic list/tree removal and active-pointer clearing, a late-read tombstone, rollback on errors, and focused headless RPC/state tests. Pinned `project-menu.tsx#removeAuto` and `layout.ts#dismissAutoProject` use a local repo-root ID, persistently hide only auto rows, exit an open project scope, and offer Undo for 8 seconds. Native uses a separate profile-wide atomic state file shared across windows, bounded cross-process locking and refresh polling; headless tests cover concurrent updates, idempotence, reload, explicit-ID collision, accessible Undo, expiry, and no gateway commands. Full metadata-to-tree reconciliation, all-profile nested hydration, repo discovery, live service/Windows behavior and source-paired acceptance remain open.

## Repository discovery source audit — 2026-11-17T20:54:17Z

Pinned sources:
- apps/desktop/src/store/projects.ts scanAndRecordRepos reads active profile config, skips repeated/in-flight signatures, uses generation fencing, stores local scan output via projects.record_repos, and for remote mode calls projects.discover_repos with scan: true.
- apps/desktop/electron/git-repo-scan.ts scanGitRepos scans configured roots only, returns empty for disabled/empty roots, caps depth at 3, normalizes/deduplicates paths, prunes excluded descendants, skips hidden/junk directories, requires readable .git/HEAD, and stops descending at a found repo.
- apps/desktop/src/app/chat/sidebar/index.tsx triggers initial discovery only in single-profile grouped mode and force rescans on visible focus/wake at a 30-second throttle; all-profile mode skips discovery.
- apps/desktop/src/app/settings/config-settings.tsx rescans only when the active profile's workspace discovery policy signature changes.
- ui_gateway/methods_config.py projects.discover_repos performs remote-host scanning and projects.record_repos validates submitted policy against host config.

Native gap: no local scanner, discovery RPC path, or settings screen for the three discovery keys. Do not claim local/remote repo-discovery parity. PLAN Revision 195 defines the implementation order and headless acceptance.

Implementation update (2026-11-17T21:02:16Z): `apps/hermes-native/src/repo_discovery.rs` now implements the local scanner core and six isolated tests. It consumes only explicitly configured roots; it does not read or guess the operator's home. The source-facing policy controls, async execution integration, `projects.record_repos`, remote `projects.discover_repos`, profile/policy generation fencing, and source trigger parity remain absent. This module alone is not user-visible discovery parity.

Repository discovery implementation update (2026-11-17T21:40:42Z): the prior source-audit section described the pre-implementation gap. That gap is now narrowed: src/repo_discovery.rs is wired into the profile-aware app path; app.rs loads/saves per-profile policy, exposes settings, schedules/deduplicates scans, and fences scope/policy generations; gateway.rs provides sparse merge-preserving config transport and local/remote host RPCs. Focused tests cover policy defaults/filtering, local config transport, local scan stale-generation rejection, all-profile remote refusal, traversal and read-only settings. Remaining evidence gaps: live Hermes gateway receipts, remote host success/error/cache preservation, physical Windows focus/wake and filesystem behavior, source-paired visuals, full project metadata reconciliation, and all-profile nested hydration. Strict 499-row acceptance remains pending.

Repo-discovery refresh audit correction (2026-11-17T21:50:58Z): pinned store/projects.ts refreshes the project tree after a successful remote scan or local record. The first native completion handler could skip that refresh when the profile's tree was already marked loaded. Revision 197 now invalidates only the captured concrete profile's load marker and prior request before issuing the normal fenced read; the last good tree remains visible until a successful replacement. Tests cover cached-tree refresh, cache-preserving failure, stale-profile receipt, exact remote scan params and malformed response. The Rust scanner remains sequential while pinned Electron limits traversal to concurrency 32; elapsed-time equivalence is unproven.


## PAR-0013/0014 semantic visibility audit (2026-11-17)

The pinned `layout.ts` binds sessions-sidebar visibility to the chat-sidebar pane and file-browser visibility to the file-browser pane, with source keyboard bindings Ctrl/Cmd+B and Ctrl/Cmd+J. Positional titlebar left/right controls are separate and resolve physical edge groups from the live tree. Native controls persist the semantic visibility states; shortcuts now first inspect the live root row and apply DoofusDock `SetGroupMinimized` to an eligible outer edge tab group, then use the semantic fallback. Restore rails keep the selected tab actionable. The implementation now records semantic pane visibility separately from physical-side collapse. When Files or Sessions is hidden, the canonical root tree retains the exact group and active tab while the presentation projection omits that side; reopening the positional edge resolves the semantic owner and restores it. A focused regression covers Files on the right with a two-tab group and persistent preference round-trip. Remaining partial gaps: additional root/profile shapes, narrow overlay behavior, physical Windows and source-paired acceptance. Dedicated native Hide/Show left and right buttons share the live root-edge handler with the keyboard shortcuts.


## Positional group folding dependency audit (2026-11-17)

Pinned `layout.ts#toggleLeftSide/#toggleRightSide` and `titlebar-controls.tsx#TitlebarControls` implement physical-edge controls over live root-row groups; keybindings dispatch through `use-keybinds.ts`. The native dependency remains pinned to `f6eb49b9e488f4e851d8d3abe18e95d296201bba`; Hermes vendors its exact core/presentation source with a local `minimized` tab-group field, revision-checked `SetGroupMinimized`, persistent group rails and projection support. Hermes' renderer wires rail restore, root-edge shortcuts and dedicated titlebar side buttons. Pinned `tree/store.ts#rootRow` also supports a horizontal main-containing row directly nested under a column root. Hermes Native now resolves that row for edge lookup and side projection, and it uses semantic Files fallback when a root-row edge is itself a nested split. The pinned breakpoint is 640 px (`SIDEBAR_DOCK_MIN_WIDTH_PX`; media query max 639.98 px); below it, `store/layout.ts#revealNarrowPane` dispatches transient pane reveal and `NarrowOverlays` renders edge-hovered, pinnable overlays with group tabs, Escape close and no native backdrop. Hermes implements side-aware Sessions/Files overlays, edge reveal, pin/close/Escape and colocated group tabs as transient presentation, while projecting both sides out of the dock without canonical mutation. Headless tests cover both panes, the 640 px usable-content boundary and wide canonical toggling. This is a local parity slice, not final acceptance: physical Windows pointer/keyboard behavior, mixed non-semantic group overlays, live runtime and source-paired visual evidence remain pending. See PLAN Revisions 200, 202–206.


## PAR-0007/0008 docking source audit (2026-11-18)

The fixed source `components/pane-shell/tree/renderer/drag-session.ts#radialPosition` maps each zone-relative pointer to [-1,1], uses center when `hypot(dx,dy) < 0.62`, then resolves the dominant axis (horizontal on ties). `#startPaneDrag` adds a 4px engage threshold, fixed-layout zone/strip snapshots, tab-strip insertion slots, Shift multi-zone highlights/merge, and Escape cancellation. `tree-group.tsx#ZoneDropOverlay` paints whole eligible zones and their selected subregions. `tree-split.tsx#startSash` owns source-resolved resize targets and width/height clamps.

Native now uses the same radial center/edge geometry in `compute_drop_edge`, then commits center/split operations through DoofusDock; split resizing previews paired weights and commits `ResizeSplit` on release. Headless tests verify center plus all four edges, square/wide/tall zone decisions, radius transition, corner ties, drop transaction persistence and resize clamps. PAR-0007/0008 are therefore partial, not absent. Remaining: source whole-zone eligibility/highlights and proximity behavior, Shift range merging, multi-tab block dragging, exact insertion caret/tear-off behavior, group-level targets, and physical Windows/source-paired acceptance. PAR-0009 detach/re-dock remains unreviewed pending exact source contract. See PLAN Revision 207 and the evidence rows in `status/IMPLEMENTATION_PROGRESS.csv`.


## Narrow collapsible pane overlay source audit (2026-11-18)

Pinned source `components/pane-shell/tree/renderer/narrow-overlays.tsx#collapsibles` resolves every in-tree contribution whose `paneChrome` is collapsible, excludes hidden panes and solo onboarding, accepts pane IDs and `revealAliases`, and shows only collapsible mates from the active tree zone. `app/contrib/controller.tsx` marks Sessions, Files and Review collapsible; `plugins/hermes-bots/plugin.tsx` registers Bots as a collapsible pane co-located with Sessions. The edge hover strip reveals the first collapsible contribution for that side. Explicit `open` pins, `close` closes the active target/zone mate, `toggle` follows source pin semantics, Escape is top-layer scoped, and pointer exit dismisses only an unpinned overlay.

Overlay width comes from the current dock zone via `narrowOverlayWidth` / `fixedTrackSize`, including the live pane width override, then falls back to the pane's declared width and caps at 85vw. The built-in source pane width is 237px. Narrow viewport state comes from the pinned 639.98px media breakpoint and is presentation-only.

Native now handles the built-in Sessions, Files and Review pane IDs, picks an edge candidate from the current canonical tree, exposes same-zone built-in tabs, uses the 237px default with an 85vw cap, and keeps reveal state outside the canonical Dock document. Headless interaction verifies Review visibility and unchanged canonical revision. The typed app action now accepts the source aliases `chat-sidebar` and `file-browser` and normalizes them to Sessions and Files; a native protocol fixture verifies agent-side use of the `file-browser` alias while the headless UI test exercises both aliases. Native has no registered Bots panel/plugin contribution, general dynamic pane contribution registry, explicit `open`/`close` reveal-intent protocol, or persisted dock-zone width override integration. Those are explicit remaining gaps alongside hidden contribution filtering, precise zone tabs with noncollapsible mates, top-layer Escape semantics, physical Windows input, live runtime and paired source screenshots. See PLAN Revisions 208–209.


## Hermes Bots pane and roster audit (2026-11-18)

The pinned plugin registers `hermes-bots:pane` under its plugin identity. `plugins/hermes-bots/plugin.tsx` declares `placement: left`, `width: 260px`, `collapsible: true`, `hideOnly: true`, and an enforced center dock into the Sessions zone. The pane renderer is `BotsPane`. `data.ts#fetchRosterSnapshot` resolves the active gateway connection and profile, then requests `profiles.list` with `{}` through that exact profile route where available; the gateway supplies the profile identity. The roster refreshes every five seconds and retains prior non-ghost rows on refresh failures. A separate `host.agents()` union contributes remote connections; local `profiles.list` rows remain canonical for the active source.

Hermes Native now registers the source pane ID as a `HideOnly` panel in the Sessions tab strip for all built-in layouts, exposes it through a shared toolbar/action path, and supports a transient narrow overlay. Its typed WebSocket call sends `profiles.list` with the explicit attached profile. Parsed rows are bounded and deduplicated; response settlement checks request generation, exact profile, endpoint instance and authority epoch. Same-owner refresh failures keep the last successful rows. The visible row fields are available to native automation state.

This remains read-only partial coverage. Native does not yet enforce re-homing after arbitrary user docking at every boot, conditionally register the pane with Bot Mode, resolve Desktop alias routes, merge `host.agents()` cross-connection rows, or provide bot CRUD, search/filter, groups, activity, avatar, source reachability and ghost behavior. The 260 px source declaration is currently represented as a native minimum, not proven exact zone geometry. No authenticated runtime, physical Windows or source-paired visual acceptance was performed. See PLAN Revision 210 and the partial PAR-0013/0014 evidence.


## Files row activation (2026-11-18)

Pinned `apps/desktop/src/app/right-sidebar/files/tree.tsx#handleActivate` invokes the preview callback for file activation on click/double-click/Enter and suppresses it for the row currently being renamed. Native `FileBrowserState::draw_directory` now routes file `response.clicked()` through `activate_file`; `HermesApp::files_panel` sends that returned path to `Workspace::open_file_preview`, which uses the canonical dynamic DoofusDock preview descriptor. The headless unit test verifies that the shared activation transition selects and returns the same path; a real egui pointer/keyboard event test has not yet been added. Remote filesystem access, attach-file/folder actions, context menus, inline rename and delete confirmation, preview/activation source pairing and Windows runtime remain open. This implementation does not change the fixed-snapshot progress overlay or acceptance count.


## Scheduled Jobs overlay host audit (2026-11-18)

The pinned route contract `.staging/hermes-pinned-878305/apps/desktop/src/app/routes.ts#APP_ROUTES` defines `/cron` as an overlay. Native had full `AppRoute::Cron` data/UI logic, but its modal host showed the generic unavailable message while `workspace_content` rendered the job surface behind the modal. Native now skips background page rendering for every overlay and selects `cron_content` inside the `/cron` modal. The existing profile-owned Scheduled Jobs headless harness now asserts the generic text is absent while job list/details remain visible. This does not complete source visual, live gateway or Windows acceptance. `/command-center` is still recognized as an overlay but lacks its source Sessions/System/Usage/Maintenance renderer.


## Command Center Sessions audit (2026-11-18)

Pinned `apps/desktop/src/app/command-center/index.tsx` defines section order and route state, reads the shared session inventory, sorts by `last_active || started_at`, searches title or ID, opens profile-owned sessions, and also provides pin, export, confirmed delete and conditional load-more actions. Native now renders `/command-center` in the modal host, supports the four section controls, seeds route section query, refreshes the existing profile session list on entry, filters/sorts rows, and opens them through the shared `OpenSession` action. The egui harness tests the real overlay, search filter and profile-preserving open. System/Usage/Maintenance remain explicit incomplete labels; pin, export, delete, cross-profile union, pagination, exact debounce, localization, source visual pairing and live Windows behavior remain open. No fixed progress row is promoted by this slice.


## Command Center pin action update (2026-11-18)

The Command Center Sessions row now exposes Pin/Unpin through native `start_session_pin_update`, reusing profile-scoped request fencing and successful-receipt persistence. The actual overlay harness verifies the action label and that it is disabled offline. This does not verify a connected mutation in a live gateway. Export, confirmed delete, cross-profile union/load-more, remaining sections and source-paired acceptance remain open.


## Command Center System status and logs (2026-11-18)

Pinned `apps/desktop/src/app/command-center/index.tsx` loads `/api/status` and `/api/logs` when the System section is active and refreshes logs for the selected file/level. The pinned API client requests 200 rows from agent, errors, gateway or desktop logs and supports ALL, INFO, WARNING and ERROR. Native now makes both reads through the active profile's authenticated, origin-checked HTTP worker; typed bounded responses are fenced by request, profile, endpoint instance, authority epoch, generation and filter. The overlay renders gateway/version/session status and filtered log lines with local search; the headless fixture verifies rendered status/log content and a separate remote-token HTTP fixture verifies exact profile scope and filtering. Usage/Maintenance, source export/delete/load-more, source visual pairing, live gateway and physical Windows remain open. Fixed-snapshot counts and formal acceptance are unchanged.


## Command Center Usage analytics (2026-11-18)

Pinned `apps/desktop/src/app/command-center/index.tsx` defines 7/30/90-day periods (30 default), fetches when Usage is selected or its period changes, and refreshes on bare `r` when no modifier or editable target is active. `UsagePanel` shows total sessions, API calls, input/output tokens, a stacked daily token chart with date endpoints/tooltips, up to six models ranked as received by the API, and up to six skills with action counts. It has loading, period-specific empty/retry, inline error while retaining existing data, and empty subsections.

The source helper `apps/desktop/src/api/models.ts#getUsageAnalytics` calls `/api/analytics/usage?days=N` through `capabilityScoped`; `apps/desktop/src/types/hermes.ts#AnalyticsResponse` describes daily/model/skill/tool/total fields. The backend also accepts an optional `profile` query and bounds days to 1–365. Hermes Native now uses the active profile's authenticated HTTP worker and validates the pinned UI periods, bounded response arrays/text, unique ISO-shaped day keys, percentages and finite nonnegative costs. Completion is fenced by request, period, profile, endpoint instance, authority epoch and generation; the source-retained last-good response stays visible while a new period is loading or fails, but a late response from an older request cannot replace it.

Local remote-authenticated HTTP, headless overlay rendering and bare-`r` refresh fixtures pass. The newly routed section has no fixed matrix row acceptance evidence; all 499 source-pair states remain pending and live runtime/visual pairing are not verified. Maintenance, remaining Sessions actions, localization and source-paired screenshots remain open.


## Command Center Maintenance (2026-11-18)

Pinned `apps/desktop/src/app/command-center/maintenance.tsx` loads `/api/curator` and `/api/memory` for the active profile, launches `doctor`, `security-audit`, `backup` and `curator-run` background actions that poll `/api/actions/{name}/status?lines=200` at 1200 ms intervals (up to 240 polls), toggles Curator pause via `PUT /api/curator/paused`, captures the profile owner before confirming `POST /api/ops/debug-share` (`{"consent": true}`), and confirms `MEMORY.md` / `USER.md` resets via `POST /api/memory/reset` (with Reset disabled when the file size is 0 bytes).

Hermes Native now runs these operations through the active profile's authenticated HTTP worker, rejects synthetic `all` profile scope, validates bounded receipts and HTTPS share URLs, polls running actions on a 1.2s timer up to 240 attempts with owner fencing, and captures pre-confirmation profile/instance/authority identity for `DebugShare` and `ResetMemory`. Focused remote-authenticated HTTP and headless overlay/confirmation-fencing regressions pass. Remaining Command Center Sessions actions (export, confirmed delete, cross-profile pagination), live gateway/Windows and source-paired visual acceptance remain open; fixed-snapshot review (38/499) and formal acceptance (0/499) are unchanged.


## Command Center Sessions confirmed deletion (2026-11-18)

Pinned `apps/desktop/src/app/command-center/index.tsx` and `delete-confirm.test.tsx` (#99410) require that clicking a session row's Delete button opens a confirmation dialog rather than deleting immediately; confirming invokes `deleteSession` (`apps/desktop/src/api/sessions.ts#mutateSessionHttp`), which reads `/api/sessions/{id}/mutation-snapshot` and sends `DELETE /api/sessions/{id}?profile=...&expected_revision=...&expected_generation=...&request_id=...`.

Native now renders `"Delete…"` (`Delete Command Center session {id}`) on each Command Center session row, opens `"Delete session?"` with captured profile/instance/authority ownership, cancels without side effects, and on confirm runs the snapshot-fenced `DELETE` on the gateway HTTP worker. Matching receipts evict the session across active, archived, command-palette, search, and cached chat views, clear its pinned order entry, record a project-session tombstone, and emit a local peer session-list invalidation hint. Focused HTTP and headless UI confirmation tests pass. Session JSON export, cross-profile pagination, live gateway/Windows and source-paired visual acceptance remain open.


### Command Center Sessions JSON export + Load more & System restart/update actions audit (2026-11-18T21:25:00Z)

- Pinned source `.staging/hermes-upstream-8358fc/apps/desktop/src/lib/session-export.ts` (`exportSession`) and `apps/desktop/src/api/sessions.ts` (`getAllSessionMessages`):
  - Pages `GET /api/sessions/{id}/messages?limit=500&offset=N&order=oldest&include_compacted=true` up to `200` pages and `32_000_000` serialized JSON characters.
  - Sanitizes filename parts by lowercasing, replacing non-`[a-z0-9._-]` runs with `-`, trimming leading/trailing `-`, slicing title to 48 chars and session ID to 8 chars (`{titlePart || 'session'}-{idPart || 'session'}.json`).
- Pinned source `.staging/hermes-upstream-8358fc/apps/desktop/src/app/command-center/index.tsx`:
  - Renders `"Load more (+200)"` when `!search.trim() && sessions.length >= limit`, increasing `limit` by `200`.
  - System section renders `"Restart gateway"` (`POST /api/gateway/restart`) and `"Update Hermes"` (`POST /api/hermes/update`), polling `GET /api/actions/{name}/status?lines=180` up to 18 times at 1200 ms intervals and refreshing System status afterward.
- Native parity status:
  - Command Center Sessions now renders `"Export"` (`Export Command Center session {id}`) and `"Load more (+200)"` (`Load more Command Center sessions`), pages full session transcripts on the authenticated HTTP worker, and atomically writes the source-formatted export JSON under `<data_directory>/exports/`.
  - Command Center System now renders `"Restart gateway"` (`Restart Command Center gateway`), `"Update Hermes"` (`Update Command Center Hermes`), and the `{name} · Running / Done / Failed` status card with 18-poll / 1.2s polling and post-action System refresh.


## Command Center profile-union pagination correction (2026-11-18T22:08:45Z)

The earlier Revision 220 audit incorrectly recorded the pinned Load more step as 200. The fixed source `.staging/hermes-upstream-8358fc/apps/desktop/src/app/command-center/index.tsx#onLoadMore` passes `SIDEBAR_SESSIONS_PAGE_SIZE` into `SidebarLoadMoreRow`, and `.staging/hermes-upstream-8358fc/apps/desktop/src/store/layout.ts` defines that value as 50. The source shows the button only when any `sessionProfilesTruncated` flag is true and the debounced query is empty; pending disables it and a `finally` clears pending. The shared refresh uses `listSidebarSessions`, initially scoped to the active profile or explicit `all` scope, with per-profile truncation from `/api/profiles/sessions/sidebar`.

Hermes Native now sends the selected profile scope and requested row window to the batched route, uses the pinned recents/messaging source exclusions, retains profile-owned row identities, derives truncation on the 404 legacy route, and keeps failed-profile rows plus truncation metadata across partial scans. A profile-union fixture covers A=truncated, B=not-truncated, C=failed; a headless app fixture verifies A updates while C's previous rows/flag survive. Load more grows a 50-row request to 100. Full-source visual pairing, the 180 ms search debounce, live runtime and Windows remain unverified; no fixed-snapshot row is newly accepted by these headless fixtures.

## Command Center search debounce (2026-11-18T22:26Z)

The pinned `index.tsx` applies `useDebouncedValue(query.trim(), 180)` and uses that settled query for local matching, empty-state text, and Load more visibility. Displayed session title follows `sessionTitle(session)`: trimmed `title`, trimmed `preview`, then `Untitled session`; filtering searches the displayed fallback plus `session.id`, case-insensitively. The native route now schedules its next egui repaint for the remaining 180 ms, preserves current results/button state until the query settles (including when clearing), and uses the displayed title contract. Two focused headless tests and the full feature suite pass. This is local implementation evidence only: fixed matrix review is still 38/499, formal acceptance 0/499, and runtime/visual evidence is unverified.

## Command Center section route query state (2026-11-18T22:35Z)

Pinned `useRouteEnumParam('section', SECTIONS, 'sessions')` replaces the route query when a section changes: default Sessions removes `section`, while System, Usage and Maintenance set it. Native overlay tabs already changed the visible content and started its fetch; `current_route_path()` now exposes the corresponding canonical query to the agent controller, and a real-egui harness click verifies both the nondefault and default routes. Native has no browser URL/history; this confirms application route-state/readback rather than browser navigation. No fixed matrix row is accepted by this headless test.

Validation addendum (2026-11-18T22:40Z): full suite passes 337 library + 15 automation protocol + 3 launcher tests; workspace/all-target check passes with only pre-existing dead-code warnings; task continuity passes 18/18; pinned package validation reports 491 pending. Review/acceptance remain 38/499 and 0/499. Route state is verified through headless egui and agent-state readback only.

## Command Center Sessions row presentation (2026-11-18T23:00Z)

Pinned `index.tsx` rows show `sessionTitle(session)` and `fmtDateTime(session.last_active || session.started_at)`; pin/export/delete action buttons become visible on row hover or focus. Native now renders a compact title/date row, uses the source's `||` recency fallback, removes preview/message/profile/ID extras from the overlay, keeps action buttons in the accessible tree while only painting their glyphs on hover/focus, and scrolls within available height. A deterministic eight-session offline fixture and image capture provide distinct layout evidence. The source icon shapes are approximated with text glyphs; timestamp formatting is English medium/short style in the OS local time zone, not full locale-specific Intl output. Runtime and upstream/target visual pairing remain unverified.

Validation addendum (2026-11-18T23:04Z): all 357 native tests pass (339 library + 15 protocol + 3 launcher); workspace check passes with existing dead-code warnings; continuity 18/18; pinned package check has 491 pending; implementation review remains 38/499 and formal acceptance 0/499. Offline fixture screenshot and accessibility-tree inspection pass. No source-paired visual or live-runtime proof is claimed.


## Command Center split navigation, breadcrumb and overlay framing (2026-11-18T23:45:00Z)

Pinned source: `apps/desktop/src/app/overlays/overlay-split-layout.tsx`, `apps/desktop/src/app/overlays/overlay-nav.tsx`, `apps/desktop/src/app/overlays/overlay-view.tsx`, and `apps/desktop/src/app/command-center/index.tsx`. The source keeps the section nav in a 13rem left rail at widths above 47.5rem, switches to a dropdown below that breakpoint, renders the breadcrumb and Sessions search in the section header, and uses a near-full viewport overlay with a floating close button. Native now has matching layout modes, header placement, route query state, and overlay framing. The narrow full-app capture still shows the global toolbar controls colliding behind the overlay; icon shapes are vector approximations.

The full suite found two regressions in the first implementation: a duplicate Maintenance label and a session Delete hit target extending beyond the clipped content region. The page title now comes from the breadcrumb, and the session row reserves width for its actions. Regression tests for Maintenance content and session deletion pass.

Evidence is headless and target-only. Source-paired pixels, installed Windows UI, live gateway behavior, history semantics and formal acceptance remain unverified. Review remains 38/499; formal acceptance remains 0/499.


### Session row modifier gestures — 2026-11-19

Pinned source: `.staging/hermes-pinned-878305/apps/desktop/src/app/chat/sidebar/session-row-gesture.ts` resolves Alt+Shift to archive, Ctrl/Cmd+Shift to window, Ctrl/Cmd to tab, Shift to pin, and no modifier to resume; `session-row.tsx` also maps middle-click to a new tab. `open-session.ts` uses in-place routing for ordinary clicks and profile-aware tab/window intent for modified clicks. Native now implements these precedence and routing outcomes and covers them with the real session-row headless harness. The pinned open-session path also calls `requestSessionResume`; native currently routes/focuses the session but does not yet prove that same explicit resume request for an already-active stored session. Runtime and source-paired visual evidence remain absent.


### Explicit reselect and active-session resume — 2026-11-19

Pinned `.staging/hermes-pinned-878305/apps/desktop/src/app/contrib/wiring.tsx#openStoredSession` requests resume for every plain stored-session row click, including the current row. `.staging/hermes-pinned-878305/apps/desktop/src/app/session/hooks/use-route-resume.ts` matches the request sequence, lets an explicit request bypass the warm `alreadyActive` skip, and invokes `resumeSession`. `open-session.test.ts` and `use-route-resume.test.tsx` cover the active-session case. Native now force-queues its profile-scoped `session.resume` RPC on explicit in-place row reselect and Retry, retains draft/runtime while pending, and fences old completions by request ID. The real-row test proves one resume/no prompt and stale-result rejection. Native row records currently identify profile but not the source `(connectionId, profile)` owner; real multi-connection in-flight reattach and Windows runtime remain unverified.

The loopback `gateway::tests::session_resume_streams_inflight_events_and_recovers_after_refusal` fixture now runs through `gateway_actor` and a real local WebSocket: a denied resume is correlated, a retry forwards in-flight start/delta events while awaiting its snapshot, then applies the snapshot before completion. This proves the native protocol integration under a controlled server. It does not prove live Hermes behavior or eliminate the exact owner-routing gap: pinned rows can carry `connection_id`, while native `SessionSummary` does not yet retain it.


### DoofusDock dependency refresh — 2026-11-24

The DoofusDock repository was fetched from `origin/main`; its exact head at refresh time is `f23ed02a99c7f8c57f83164d98feadb17cc4ed87`. Hermes pins this commit in the root, app and preserved rev1 Cargo manifests and in `Cargo.lock`. The core, presentation and egui renderer vendor snapshots were refreshed to this source. The upstream renderer now provides the context-menu callback previously carried as a local patch, and upstream core now owns group-minimize state and operations. The Hermes-specific `GroupRail` restore projection remains a small documented presentation extension because this DoofusDock revision does not provide restore-rail geometry. Later DoofusDock changes will not be adopted automatically; this pin changes only after an explicit update request.

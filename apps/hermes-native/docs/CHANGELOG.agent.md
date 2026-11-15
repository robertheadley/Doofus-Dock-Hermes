# Agent change journal

## HUR-001-001 — 2026-11-13

- **what:** Started an independent upstream Hermes native Rust app crate, copied the approved plan/pin and exact source layout/routes into project audit notes, and configured DoofusDock public crates.
- **why:** Begin the approved source-parity implementation without changing the earlier Doofus/Hermes prototype or source checkouts.
- **components:** `Cargo.toml`, `src/`, `README.md`, `docs/PLAN.md`, `docs/SOURCE_AUDIT.md`, `docs/BACKLOG.md`.
- **type:** feat
- **validation:** Source pin confirmed at `8358fc7bb4ce18fbee8d4bf6659dd155c2a03783`; source Desktop first-run shell launched from an isolated checkout/profile. Rust compile verification is pending.
- **performance:** Not measured.
- **risks:** First slice is not feature parity. Upstream UI compilation required an isolated type-only declaration shim; screenshot evidence is still pending.
- **follow_up:** Implement and compile source profile trees and the Advanced/Simple shell, then save source/target evidence and expand the acceptance matrix.

- **id:** HUR-001-002
  **timestamp:** 2026-11-13T21:15:58Z
  **what:** Added the first native Hermes shell slice with pinned DoofusDock integration, source-defined profiles, separate Simple/Advanced mode handling, persistent layout/preferences, Sessions and New session panes, and explicit offline/empty states.
  **why:** Establish a testable native Rust host before gateway and stock widget parity work.
  **components:** `src/main.rs`, `src/app.rs`, `src/layout.rs`, `Cargo.toml`, `Cargo.lock`.
  **type:** feature
  **validation:** `cargo fmt`, `cargo check`, `cargo test` (2 passed), Windows UI inspection, Simple/Advanced round trip, Files reveal/hide round trip.
  **performance:** No performance change measured; no benchmark claimed.
  **risks:** This is a partial shell and is not a 1:1 replacement. Gateway, source routes/overlays, real session/chat workflows, and remaining stock widgets are absent. Upstream visual capture later hit its isolated backend timeout.
  **follow_up:** Implement routes and typed gateway before claiming source feature parity; capture matched paired screenshots and complete the source acceptance matrix.
- **id:** HUR-001-003
  **timestamp:** 2026-11-13T21:20:19Z
  **what:** Published the independent crate at `apps/hermes-native`, revalidated the published copy, and removed Cargo build outputs using `cargo clean`.
  **why:** Make the Rust UI implementation easy to locate and avoid shipping generated build artifacts.
  **components:** `apps/hermes-native/`.
  **type:** packaging
  **validation:** `cargo fmt --check`, `cargo check`, and `cargo test` passed at the candidate package path (2 tests); final-path `cargo check` and `cargo test` passed (2 tests).
  **performance:** No benchmark performed or claimed.
  **risks:** Full upstream parity is not complete; a separate Cargo cache remains in the workspace staging folder because local policy rejected its direct cleanup. An isolated preview declaration shim also remains untracked.
  **follow_up:** Continue gateway, route, stock-widget and parity-matrix implementation; re-capture upstream baseline after resolving source backend timeout.
- **id:** HUR-001-004
  **timestamp:** 2026-11-13T21:35:35Z
  **what:** Refreshed the working plan to Hermes `491f984`, preserved and verified the supplied 491-requirement reference package, and added reserved/contributed route classification with sidebar navigation and explicit offline page/modal states.
  **why:** Keep the parity target pinned to current upstream and establish the route shell required before real workflow integration.
  **components:** `src/routes.rs`, `src/app.rs`, `src/main.rs`, `docs/PLAN.md`, `docs/SOURCE_AUDIT.md`, `docs/BACKLOG.md`, `docs/reference/`.
  **type:** feature + source-refresh
  **validation:** 29 manifest file hashes passed; package consistency script passed; `cargo fmt --check`, `cargo check`, and `cargo test` passed (6 tests).
  **performance:** No benchmark performed or claimed.
  **risks:** Pages remain placeholders; overlays are generic; route visuals were not re-captured on Windows; gateway and stock widgets remain incomplete; reference maps retain historical source pin `ffdc7eeb` and all 491 statuses remain pending.
  **follow_up:** Implement pinned gateway attach/recovery and session/chat spine, then continue stock feature crosswalk and acceptance receipts.
- **id:** HUR-001-005
  **timestamp:** 2026-11-13T21:47:45Z
  **what:** Added asynchronous, credential-free upstream gateway discovery and strict endpoint validation to the native Hermes shell.
  **why:** Establish the authoritative backend boundary before adding ticketed WebSocket transport and session workflows.
  **components:** src/gateway.rs, src/app.rs, src/main.rs, README.md, docs/PLAN.md, docs/SOURCE_AUDIT.md, docs/BACKLOG.md.
  **type:** feature
  **validation:** cargo fmt and cargo check passed. The upstream gateway command was not executed because ensure can start the persistent gateway owner.
  **performance:** No benchmark run or claimed; process execution occurs on a named worker thread.
  **risks:** No ticket minting, WebSocket, JSON-RPC, replay/reconnect or session UI exists yet; the app may start the user's gateway on launch.
  **follow_up:** Implement one-use ticket attach, protocol/subprotocol validation, JSON-RPC negotiation and session-list retrieval, then exercise against an isolated Hermes profile.
- **id:** HUR-001-006
  **timestamp:** 2026-11-13T21:48:44Z
  **what:** Tightened endpoint authority validation to parse a bare origin and accept only exact IPv4/IPv6 loopback hosts with a valid nonzero port.
  **why:** Prevent a prefix-only URL check from accepting userinfo or a non-loopback host disguised after the prefix.
  **components:** src/gateway.rs.
  **type:** hardening
  **validation:** cargo fmt and cargo check passed without warnings.
  **performance:** No measurable impact; validation is performed once after startup discovery.
  **risks:** Discovery has not been exercised against the running owner; ticketed transport remains open.
  **follow_up:** Continue with canonical ticket and WebSocket attach protocol.

- **id:** HUR-001-007
  **timestamp:** 2026-11-13T22:14:05Z
  **what:** Added unattended, Playwright-like agent interaction and screenshot capture as an explicit Hermes Native parity gate.
  **why:** The operator wants the coding agent to navigate and inspect the app in the background without interrupting the active desktop, while retaining richer debugging access.
  **components:** docs/PLAN.md, docs/BACKLOG.md, docs/AGENT_AUTOMATION.md, README.md.
  **type:** planning + acceptance
  **validation:** Checked the updated plan/backlog for revision, stage ordering, obsolete gateway status, and complete agent-automation acceptance criteria.
  **performance:** No runtime behavior changed; no benchmark performed.
  **risks:** The semantic action bridge, isolated background runner, and capture pipeline are planned only; this is not yet available behavior.
  **follow_up:** Implement AccessKit-backed stable identities and normal-handler actions, then a background isolated runner with screenshot/trace artifacts.

- **id:** HUR-001-008
  **timestamp:** 2026-11-13T22:32:02Z
  **what:** Added an isolated headless semantic UI runner, shared the native UI drawing path with it, enabled workspace loading from an explicit isolated directory, and incorporated the user-supplied agent-control architecture into the Hermes plan/crosswalk.
  **why:** Let the coding agent navigate and capture UI evidence without taking over the operator's desktop, while keeping Hermes session authority separate from app diagnostics/control.
  **components:** Cargo.toml, Cargo.lock, src/lib.rs, src/main.rs, src/app.rs, src/layout.rs, src/bin/hermes-automation.rs, README.md, docs/PLAN.md, docs/BACKLOG.md, docs/ARCHITECTURE.md, docs/AGENT_AUTOMATION.md, docs/AGENT_CONTROL_CROSSWALK.md.
  **type:** feature + architecture-planning
  **validation:** `cargo fmt --all -- --check` and `cargo check --all-targets` passed. The runner inspected AccessKit controls, clicked Files, rendered a 1440×900 screenshot without a native window or gateway, and the PNG was visually inspected. Strict harness feedback found and corrected a missing font glyph and two unnamed selectors.
  **performance:** No benchmark performed. The runner uses headless egui_kittest rendering and avoids native window startup.
  **risks:** This runner is in-process/headless and does not provide an external authenticated bridge or prove physical Windows host behavior. Accessible header labels introduce a small visual difference that still needs upstream comparison.
  **follow_up:** Implement stable IDs and shared typed semantic actions, then secured development bridge and rebuild supervisor; continue full UI and Windows parity.

- **id:** HUR-001-009
  **timestamp:** 2026-11-13T22:35:00Z
  **what:** Captured a second unattended semantic UI run that navigates to the Capabilities route and saves its state/screenshot with build and source metadata.
  **why:** Demonstrate route navigation in addition to pane reveal through the agent interface without opening or focusing a native window.
  **components:** src/bin/hermes-automation.rs, artifacts/agent-ui/capabilities-route.png, artifacts/agent-ui/capabilities-route.jsonl, artifacts/agent-ui/run-metadata.json, docs/PLAN.md, docs/BACKLOG.md, docs/AGENT_AUTOMATION.md, docs/AGENT_CONTROL_CROSSWALK.md.
  **type:** verification evidence
  **validation:** The runner clicked the accessible `Capabilities` button, the AccessKit tree showed the destination content, the 1440×900 frame was saved and visually inspected, and metadata matched the executable SHA-256.
  **performance:** No benchmark performed; headless capture did not open a native window.
  **risks:** Semantic route evidence does not verify the real Windows host or compare against upstream Hermes source.
  **follow_up:** Extend semantic UI coverage and implement the authenticated bridge; obtain paired source/target and physical host evidence.

- **id:** HUR-001-010
  **timestamp:** 2026-11-13T22:38:47Z
  **what:** Gated the headless automation runner and egui_kittest renderer behind the opt-in `agent-automation` feature, made fallback data paths stay inside each isolated profile, and added verified temp-profile cleanup.
  **why:** Keep development automation out of the default Hermes build and ensure even UI startup failures cannot write outside an automation-owned profile.
  **components:** Cargo.toml, Cargo.lock, src/app.rs, src/layout.rs, src/bin/hermes-automation.rs, README.md, docs/ARCHITECTURE.md, docs/AGENT_AUTOMATION.md, docs/AGENT_CONTROL_CROSSWALK.md, docs/PLAN.md, artifacts/agent-ui/.
  **type:** hardening + packaging
  **validation:** Formatting passed; default and `agent-automation` all-target checks passed; `cargo test --all-targets --features agent-automation` passed 6/6. Both semantic runs were repeated against the feature-gated binary, PNGs were saved, binary SHA was recorded, and both per-run profiles were confirmed removed.
  **performance:** No benchmark performed.
  **risks:** The harness remains headless and lacks an external bridge or physical Windows host verification.
  **follow_up:** Implement the typed semantic service and authenticated development bridge; preserve a production build with no debug controls enabled.
- **id:** HUR-001-011
  **timestamp:** 2026-11-13T22:51:17Z
  **what:** Added `hermes-native-agent-ui/1`, an opt-in inherited-stdio NDJSON child process for semantic UI reads/actions, diagnostics and screenshots. Added bounded protocol contracts, generation-checked actions, structured receipts, duplicate replay, conflicting-ID rejection, expired-ID rejection, traversal-resistant screenshot writes, and documentation.
  **why:** Let a coding agent operate and inspect the native Hermes UI in the background while keeping the operator's active desktop untouched.
  **components:** src/bin/hermes_automation/automation_protocol.rs, docs/AGENT_AUTOMATION_PROTOCOL.md, docs/AGENT_AUTOMATION.md, docs/PLAN.md, docs/BACKLOG.md, docs/AGENT_CONTROL_CROSSWALK.md, README.md, artifacts/agent-ui/stdio-protocol-receipts-v2.jsonl, artifacts/agent-ui/stdio-protocol-metadata.json, artifacts/agent-ui/stdio-bridge-route-v2.png.
  **type:** feature + protocol hardening + documentation
  **validation:** `cargo fmt --all`; default and feature `cargo check --all-targets`; `cargo test --all-targets --features agent-automation` (6 passed); built and exercised 141 NDJSON requests in a child process. Verified diagnostics capability, generation increment, duplicate replay, ID conflict, stale-state failure, traversal rejection, 1440×900 screenshot, rejection of an ID after cache eviction, and clean close. Screenshot was visually inspected. The default build excludes the runner.
  **performance:** No benchmark performed; one isolated headless process is started per session and no native window/network listener is created.
  **risks:** This is a development-only inherited-pipe capability, not an authenticated network service. The typed shared app-action registry, test IDs, select/key/wait actions, supervisor, physical Windows host evidence, upstream comparison, and full UI parity are still incomplete.
  **follow_up:** Continue the 491-item upstream crosswalk, implement shared typed actions and remaining agent controls, then capture matched upstream/native workflows and physical host evidence.

- **id:** HUR-001-012
  **timestamp:** 2026-11-13T22:59:10Z
  **what:** Extended the child protocol with accessible selector choice, egui keypress, bounded wait-for-visible-control, and focused/accessibility bounds reporting. Made tree inspection safe for nodes without an AccessKit rectangle.
  **why:** Give the coding agent more of the same navigation and interaction primitives a person can use, while making headless inspection reliable.
  **components:** src/bin/hermes_automation/automation_protocol.rs, docs/AGENT_AUTOMATION_PROTOCOL.md, docs/AGENT_AUTOMATION.md, docs/PLAN.md, docs/BACKLOG.md, docs/AGENT_CONTROL_CROSSWALK.md, README.md, artifacts/agent-ui/stdio-interaction-receipts-v3.jsonl, artifacts/agent-ui/stdio-interaction-metadata-v3.json, artifacts/agent-ui/stdio-interactions-v3.png.
  **type:** feature + robustness + documentation
  **validation:** Default and feature `cargo check --all-targets` passed; `cargo test --all-targets --features agent-automation` passed 6/6. A live child run selected Simple via AccessKit ComboBox, waited for Files, hit a bounded timeout for an absent control, sent Tab, inspected a tree with 19 nodes lacking bounds, read diagnostics, and captured/visually inspected a 1440×900 screenshot. All 9 responses were validated.
  **performance:** No benchmark performed; UI waits are capped at 5 seconds and sleep in 10 ms intervals while advancing headless frames.
  **risks:** Global key events are currently an allowlisted set without modifier combinations. Headless UI evidence is not a physical Windows interaction or upstream parity comparison.
  **follow_up:** Add stable test IDs and modifier combinations, expand scenario coverage, and continue remaining source workflows and paired Windows acceptance.

- **id:** HUR-001-013
  **timestamp:** 2026-11-13T23:01:32Z
  **what:** Added the `Message composer` accessible name and rejected click/type/select operations on disabled accessibility targets and options.
  **why:** Make the actual composer discoverable to automation and prevent automation from reporting successful input into disabled/offline controls.
  **components:** src/app.rs, src/bin/hermes_automation/automation_protocol.rs, docs/AGENT_AUTOMATION_PROTOCOL.md, docs/PLAN.md, artifacts/agent-ui/stdio-interaction-receipts-v4.jsonl, artifacts/agent-ui/stdio-interaction-metadata-v4.json, artifacts/agent-ui/stdio-interactions-v4.png.
  **type:** accessibility + safety + evidence
  **validation:** Default and feature checks passed and the feature test suite passed 6/6. Protocol runtime inspection found the composer by its role/name and confirmed it is disabled; attempted typing returned `target_disabled`. Captured a fresh 1440×900 headless screenshot and recorded the binary/source identifiers.
  **performance:** No benchmark performed; no runtime cost outside the opt-in automation build, except the accessible label when the UI renders.
  **risks:** The composer remains disabled until gateway/chat workflow parity is implemented; this change does not implement message submission.
  **follow_up:** Implement the actual source-equivalent composer, submit/stream/stop and approval workflows with gateway fixtures and live isolated acceptance.

- **id:** HUR-001-014
  **timestamp:** 2026-11-13T23:16:08Z
  **what:** Implemented existing-session `prompt.submit` from the Rust composer, runtime-session-scoped acknowledgements and basic gateway message event forwarding/projection. Corrected pending-send scoping across session navigation and expanded composer width to the available workspace area.
  **why:** Move the central Hermes chat workflow beyond transcript restore while preserving the Python gateway as the sole execution authority.
  **components:** src/gateway.rs, src/app.rs, docs/PLAN.md, docs/BACKLOG.md, docs/AGENT_CONTROL_CROSSWALK.md, docs/SOURCE_AUDIT.md, docs/ARCHITECTURE.md, README.md, artifacts/agent-ui/chat-composer-shell-v4.png, artifacts/agent-ui/chat-composer-shell-v4.jsonl, artifacts/agent-ui/chat-composer-accessibility-v4.jsonl, artifacts/agent-ui/chat-composer-shell-v4-metadata.json.
  **type:** feature + gateway protocol + documentation + evidence
  **validation:** Default and feature builds passed; default tests passed 7/7; feature tests passed 9/9. A loopback WebSocket fixture verified `prompt.submit` sends the runtime ID and text, forwards message start/delta before its RPC acknowledgement and completion afterward. App tests verified delta accumulation, completion settlement and cross-session event rejection. A headless 1440×900 frame was visually inspected; stdio AccessKit confirmed the named composer is disabled on the new-session route and its width spans 1028 logical units. Temporary profile cleanup was verified.
  **performance:** No benchmark performed; prompts and events continue through the single gateway worker.
  **risks:** New-session creation is still missing; the fresh-chat composer stays disabled. The existing-session slice does not yet send retained admission identity, retry safely after uncertain delivery, handle stop/server prompts/tool/reasoning events, reconcile the full transcript or reconnect. No live gateway write was attempted.
  **follow_up:** Implement source-shaped `session.create` and retained submit identity, then stop/approval/server-request handling, complete stream semantics and isolated live gateway acceptance. Continue the 491-item UI matrix and paired source/Rust visuals.

- **id:** HUR-001-015
  **timestamp:** 2026-11-13T23:21:35Z
  **what:** Added partial fresh-session creation from the New Chat composer. The gateway worker now performs `session.create`, returns both runtime and stored session IDs, navigates with the stored ID, and submits the initial prompt with the runtime ID. Added a loopback WebSocket fixture for ordered create then submit.
  **why:** The new-chat composer was unusable, leaving a core current-source chat route missing after the existing-session send slice.
  **components:** src/app.rs, src/gateway.rs, docs/PLAN.md, docs/BACKLOG.md, docs/SOURCE_AUDIT.md, docs/ARCHITECTURE.md, README.md, this changelog, execution log, artifacts/agent-ui/new-chat-shell-v5.png, artifacts/agent-ui/new-chat-shell-v5-metadata.json.
  **type:** feature; documentation; protocol fixture
  **validation:** Formatting check passed; default and feature checks passed without warnings; default tests passed 8/8 and feature tests passed 10/10. A loopback WebSocket fixture verified create then submit, exact create params, and runtime/stored ID roles. A headless 1440×900 screenshot was visually inspected; AccessKit exposed the composer, Send, and New session. Temporary-profile cleanup was verified.
  **performance:** No benchmark performed. Session creation and submit remain serialized through the single gateway worker.
  **risks:** The installed gateway was not sent a session-create write. Create options are partial and lack CWD/model/provider/reasoning/fast/service-tier settings and upstream unsupported-field retry. Admission ID is local-only correlation, not server idempotency. No composer screenshot has yet been captured with a connected gateway.
  **follow_up:** Implement complete upstream create options/retry and preserve admission identity, then cover stop/server requests/remaining stream semantics and continue the 491-item UI matrix with paired live Windows evidence.

- **id:** HUR-001-016
  **timestamp:** 2026-11-13T23:30:09Z
  **what:** Added accessible local filtering to the Sessions pane. Loaded rows match case-insensitively on title, preview and session ID, and the UI distinguishes a filtered no-match state from an empty session list.
  **why:** The native UI loaded and rendered session history but offered no way to narrow the visible list; upstream Desktop includes session search.
  **components:** src/app.rs, docs/PLAN.md, docs/BACKLOG.md, docs/SOURCE_AUDIT.md, README.md, this changelog, execution log, artifacts/agent-ui/session-search-shell-v6.png, artifacts/agent-ui/session-search-shell-v6-metadata.json.
  **type:** feature; accessibility; documentation; evidence
  **validation:** Formatting check passed; default and automation-feature checks passed; default tests passed 8/8 and feature tests passed 11/11. New test covers title, preview, ID, blank and no-match queries. Headless AccessKit inspection exposed `Search sessions`; screenshot visually inspected and the temporary profile cleanup verified.
  **performance:** Filtering scans the currently loaded session rows on the UI thread; the gateway list is bounded to 200 rows.
  **risks:** This is not upstream search parity: Desktop debounces full-text `/api/sessions/search` through its HTTP bridge and searches unloaded history. No remote full-history query is available through this Rust UI path yet.
  **follow_up:** Reconcile and implement the profile-scoped, debounced full-history search/result contract, then continue unresolved session actions and all other source routes.

- **id:** HUR-001-017
  **timestamp:** 2026-11-13T23:32:48Z
  **what:** Corrected the journal record after the HUR-001-016 entry was appended twice by a repeated tool patch. The two identical HUR-001-016 blocks describe one implementation and one validation run, not two separate changes.
  **why:** Preserve an auditable change history without allowing a duplicated journal block to inflate the apparent implementation count.
  **components:** docs/CHANGELOG.agent.md, docs/EXECUTION_LOGS.md.
  **type:** documentation correction
  **validation:** Confirmed both HUR-001-016 blocks are byte-equivalent and refer to the single source change/test run recorded in the execution log.
  **performance:** None.
  **risks:** The duplicated historical blocks remain present to honor the append-only journal rule; consumers must interpret HUR-001-016 as one change per this correction.
  **follow_up:** Keep future journal IDs unique and validate the journal before appending.

- **id:** HUR-001-018
  **timestamp:** 2026-11-13T23:42:00Z
  **what:** Preserved fractional `started_at` values from Hermes `session.list` and displayed compact recency on session sidebar rows.
  **why:** Align sidebar row metadata with upstream's age display and avoid losing fractional timestamp precision during DTO decoding.
  **components:** src/gateway.rs, src/app.rs, docs/PLAN.md, docs/BACKLOG.md, docs/SOURCE_AUDIT.md, docs/ARCHITECTURE.md, README.md, this changelog, execution log.
  **type:** feature; source-contract alignment; documentation
  **validation:** Formatting check passed; default check and tests passed 9/9; automation-feature check passed and tests passed 12/12. A fixture decodes fractional epoch seconds, and age formatter assertions cover seconds, minutes and hours.
  **performance:** Age formatting is constant-time per visible row; values are rendered from the session list without additional gateway requests.
  **risks:** Upstream prefers `last_active` when present, but current Gateway `SessionListRow` has only `started_at`. The timestamp therefore cannot reflect a session's latest activity until Hermes exposes equivalent data.
  **follow_up:** Continue sidebar parity with source-backed full-text search and row actions; preserve the session listing DTO contract.

- **id:** HUR-001-019
  **timestamp:** 2026-11-13T23:38:56Z
  **what:** Corrected the timestamp of HUR-001-018. That entry was assigned a UTC time later than the actual recorded tool completion time; its source and validation claims remain unchanged.
  **why:** Keep the append-only journal's chronology accurate without rewriting the original entry.
  **components:** docs/CHANGELOG.agent.md, docs/EXECUTION_LOGS.md.
  **type:** documentation correction
  **validation:** Compared the original timestamp against the current UTC clock reading and the appended execution record.
  **performance:** None.
  **risks:** HUR-001-018 remains with its original mistaken timestamp; this correction is authoritative.
  **follow_up:** Use a current clock reading before writing future timestamps.

- **id:** HUR-001-016
  **timestamp:** 2026-11-13T23:30:09Z
  **what:** Added accessible local filtering to the Sessions pane. Loaded rows match case-insensitively on title, preview and session ID, and the UI distinguishes a filtered no-match state from an empty session list.
  **why:** The native UI loaded and rendered session history but offered no way to narrow the visible list; upstream Desktop includes session search.
  **components:** src/app.rs, docs/PLAN.md, docs/BACKLOG.md, docs/SOURCE_AUDIT.md, README.md, this changelog, execution log, artifacts/agent-ui/session-search-shell-v6.png, artifacts/agent-ui/session-search-shell-v6-metadata.json.
  **type:** feature; accessibility; documentation; evidence
  **validation:** Formatting check passed; default and automation-feature checks passed; default tests passed 8/8 and feature tests passed 11/11. New test covers title, preview, ID, blank and no-match queries. Headless AccessKit inspection exposed `Search sessions`; screenshot visually inspected and the temporary profile cleanup verified.
  **performance:** Filtering scans the currently loaded session rows on the UI thread; the gateway list is bounded to 200 rows.
  **risks:** This is not upstream search parity: Desktop debounces full-text `/api/sessions/search` through its HTTP bridge and searches unloaded history. No remote full-history query is available through this Rust UI path yet.
  **follow_up:** Reconcile and implement the profile-scoped, debounced full-history search/result contract, then continue unresolved session actions and all other source routes.

- **id:** HUR-001-020
  **timestamp:** 2026-11-13T23:49:39Z
  **what:** Added titlebar drag/drop panel docking through a revision-bound DoofusDock `DragSession`. Center targets create tabs; edge targets create 40/60 splits; successful drops persist via `LayoutStore`. Updated the parity plan and agent-control docs with shipped behavior and unsupported automation gesture.
  **why:** The Hermes shell had canonical layout projection but no user-operated pointer gesture to reorganize panels, a core DoofusDock interaction and needed parity capability.
  **components:** src/app.rs, docs/PLAN.md, docs/BACKLOG.md, docs/ARCHITECTURE.md, docs/AGENT_CONTROL_CROSSWALK.md, docs/AGENT_AUTOMATION_PROTOCOL.md, README.md, artifacts/agent-ui/panel-drag-layout-v7.png, artifacts/agent-ui/panel-drag-layout-v7-metadata.json, this changelog, execution log.
  **type:** feature; DoofusDock interaction; headless fixture; documentation; evidence.
  **validation:** Formatting check passed after rustfmt; default check passed and tests passed 9/9; automation-feature check passed and tests passed 14/14. Feature tests cover center transaction persistence and center/left/right/top/bottom drop-region selection. The inherited-stdio protocol health/state/screenshot/close requests succeeded with gateway disabled; the 1440×900 New Session screenshot was visually inspected and its SHA-256 recorded. No native window or installed Hermes profile was used.
  **performance:** Drop geometry is constant-time per visible panel; no benchmark requested or performed.
  **risks:** Pointer drag has not been exercised in the physical Windows host, and the agent protocol cannot synthesize pointer drag yet. Resize, tab reorder, float/cross-window docking and paired source/target visual evidence remain open.
  **follow_up:** Add a bounded generation-checked `actions.drag` API using real egui pointer events and shared interaction handling; then automate a center-tab and each edge-split in the isolated harness and capture preview/after screenshots. Continue remaining 491-capability parity matrix.
- **id:** HUR-001-021
  **timestamp:** 2026-11-14T00:11:02Z
  **what:** Added generation-checked `actions.drag` to the agent UI protocol using real egui primary-pointer events and exact AccessKit source locators. Added accessible panel-header groups, disambiguated dock-tab labels, visible switchable tab strips backed by `Operation::SelectTab`, and filtered inactive zero-opacity panel leaves so inactive tabs do not cover the selected panel. Extended diagnostics with the canonical layout document and active tab metadata.
  **why:** Agent automation could not reproduce a person's panel drag, and the first center-drop visual exposed an inaccessible tab group and inactive-panel overdraw. A 1:1 docking interaction requires both the pointer gesture and a reachable result.
  **components:** src/app.rs, src/bin/hermes_automation/automation_protocol.rs, docs/PLAN.md, docs/BACKLOG.md, docs/ARCHITECTURE.md, docs/AGENT_AUTOMATION.md, docs/AGENT_AUTOMATION_PROTOCOL.md, docs/AGENT_CONTROL_CROSSWALK.md, README.md, artifacts/agent-ui/agent-drag-tab-select-v13.png, artifacts/agent-ui/agent-drag-tab-select-v13-metadata.json, this changelog, execution log.
  **type:** feature; accessibility; agent UI protocol; diagnostics; visual acceptance evidence.
  **validation:** Formatting check passed; default check passed; default tests passed 9/9; automation-feature check passed; feature tests passed 16/16 (14 library tests plus two protocol UI tests). Protocol tests reject over-limit drag steps, perform a center drag by accessible source locator, select the other dock tab, verify the inactive panel remains accessible, and perform a left-edge drag that produces a canonical horizontal 40/60 split. A live isolated stdio run repeated center drag and both tab selections; its 1440×900 screenshot was visually inspected and hashed in metadata. No native window, gateway, installed Hermes profile, or older prototype was used.
  **performance:** Pointer interpolation is bounded to 32 events per gesture; tab-group enumeration is linear in the small canonical layout tree. No benchmark was requested or performed.
  **risks:** This is headless UI-path evidence, not physical Windows drag/focus/DPI acceptance. Drag uses viewport logical coordinates for the destination; stable test-ID locators, modifiers, resize handles, tab reorder, floating/cross-window docking and the rest of the 491-capability parity matrix remain open.
  **follow_up:** Add stable test IDs and locator disambiguation beyond role/name; automate all five drop regions and route-specific actions; compare matched upstream/native visuals on real Windows; continue source-to-implementation reconciliation for all remaining capabilities.
- **id:** HUR-001-022
  **timestamp:** 2026-11-14T00:21:42Z
  **what:** Added accessible split divider handles using the pinned DoofusDock `resize_handle`. Dragging previews a temporary canonical-document clone with the adjacent split weights adjusted and clamped to 5–95% of their pair. Pointer release commits one revision-checked `Operation::ResizeSplit` and persists through `LayoutStore`. Handles have unique split-node/divider names and are operable through agent `actions.drag`. Added layout diagnostics for persistence errors and expanded the parity plan/backlog.
  **why:** Resizing workspace splits is a core DoofusDock layout behavior and had remained absent after titlebar docking and tab selection were implemented.
  **components:** src/app.rs, src/bin/hermes_automation/automation_protocol.rs, docs/PLAN.md, docs/BACKLOG.md, docs/ARCHITECTURE.md, docs/AGENT_AUTOMATION_PROTOCOL.md, docs/AGENT_AUTOMATION.md, docs/AGENT_CONTROL_CROSSWALK.md, README.md, artifacts/agent-ui/split-resize-v15.png, artifacts/agent-ui/split-resize-v15-metadata.json, this changelog, execution log.
  **type:** feature; DoofusDock interaction; agent control; diagnostics; visual evidence.
  **validation:** Formatting check passed; default check passed and tests passed 9/9; automation-feature check passed and tests passed 18/18 (15 library and 3 protocol UI tests). Protocol tests validate pointer resize changes canonical weights, advances revision and reports no persistence error; unit tests verify 5–95% pair clamping and preserve unrelated weights. A separate isolated stdio run discovered the accessible divider through `ui.tree`, dragged it, observed weights change from [0.2273, 0.7727] to [0.2875, 0.7125], and visually inspected the 1440×900 screenshot. Gateway was disabled and no native window or installed profile was used.
  **performance:** Split traversal is linear in the layout tree; preview clones only the small document. No benchmark was requested or performed.
  **risks:** This validates the headless egui path, not physical Windows pointer/focus/DPI behavior. Split handles are pointer-operated; keyboard resizing, native host evidence, nested/all-profile acceptance and tab reorder/floating/cross-window interactions remain open.
  **follow_up:** Exercise split resizing across every shipped profile and nested split, compare source and target geometry at matched Windows sizes/DPI, and continue the 491-capability source matrix.
- **id:** HUR-001-023
  **timestamp:** 2026-11-14T00:36:09Z
  **what:** Added same-group DoofusDock tab reordering through revision-checked `Operation::ReorderPanel`; moved-tab identity remains stable, becomes active, and is persisted. Extended `actions.drag` to accept a semantic destination (`to_role`/`to_name`) as the preferred target locator while retaining bounded coordinates for geometry-specific drops. Added live-egui protocol coverage, documented the supplement-aligned status, and captured inspected isolated evidence.
  **why:** The Doofus Apps supplement requires semantic controls as the primary agent interface, and the user's Playwright-like control goal needs tab drag/drop to address its destination by accessibility identity rather than target coordinates.
  **components:** src/app.rs, src/bin/hermes_automation/automation_protocol.rs, docs/PLAN.md, docs/BACKLOG.md, docs/ARCHITECTURE.md, docs/AGENT_AUTOMATION.md, docs/AGENT_AUTOMATION_PROTOCOL.md, docs/AGENT_CONTROL_CROSSWALK.md, README.md, artifacts/agent-ui/tab-reorder-v18.png, artifacts/agent-ui/tab-reorder-v18-metadata.json, this changelog, execution log.
  **type:** feature; agent control; DoofusDock interaction; semantic UI protocol; screenshot evidence; documentation.
  **validation:** Formatting check passed; default `cargo check --all-targets` passed and tests passed 9/9; automation-feature check passed and tests passed 18/18 (15 library tests plus 3 protocol tests). A fresh isolated `hermes-automation --stdio` process center-docked Sessions, reordered it by semantic source and destination locators, reported `[sessions, workspace]`, active index 0, layout revision 3, and successfully closed. Its 1440x900 screenshot was visually inspected; SHA-256 and executable hash are in metadata. Gateway was disabled, no native window was opened, and no installed Hermes profile was touched.
  **performance:** Reorder locator lookup is bounded by the current accessibility tree and layout group; pointer steps remain capped at 32. No benchmark was requested or performed.
  **risks:** This is headless semantic evidence, not physical Windows acceptance. Cross-group/window moves, visual drop feedback, pinned tabs, overflow/narrow-width behavior and tab-specific keyboard support remain open. The supplement's shared typed capability registry, authenticated external Debug Bridge, app-wide diagnostics, development supervisor and reload recovery remain incomplete.
  **follow_up:** Add negative-path tests for leaving the source tab strip and stale layout revisions; implement stable test IDs and remaining DoofusDock gestures, then continue upstream source reconciliation and paired physical Windows visual/interaction acceptance.
- **id:** HUR-001-024
  **timestamp:** 2026-11-14T00:40:44Z
  **what:** Fixed the tab-reorder handoff so a release without a tab target retains the ordinary DoofusDock `DragSession`, allowing center/edge docking to process the drop. Added a regression test for this no-target case and refreshed isolated tab-reorder screenshot evidence against the final binary.
  **why:** Same-strip reorder must not swallow a normal panel move when the pointer leaves the tab strip; the supplement requires agent actions to preserve the real UI gesture behavior.
  **components:** src/app.rs, docs/PLAN.md, docs/BACKLOG.md, docs/AGENT_AUTOMATION.md, artifacts/agent-ui/tab-reorder-v19.png, artifacts/agent-ui/tab-reorder-v19-metadata.json, this changelog, execution log.
  **type:** bugfix; DoofusDock interaction; regression test; screenshot evidence; documentation.
  **validation:** Formatting check passed; default check passed; default tests 9/9 passed; automation-feature check passed; feature tests 19/19 passed (16 library plus 3 protocol UI tests). The no-target regression confirms the generic panel drag remains available. Fresh isolated stdio run reordered Sessions over New session using semantic role/name destinations, verified order `[sessions, workspace]`, active index 0 and revision 3, captured and visually inspected a 1440x900 screenshot, then closed with exit code 0. Gateway disabled; no native window or installed profile used.
  **performance:** No meaningful performance change; no benchmark requested or performed.
  **risks:** Only the app-level state handoff is covered; cross-group docking and actual OS pointer behavior remain unverified. The architecture supplement's bridge, registry, supervisor, diagnostics and reload gates remain open.
  **follow_up:** Continue source-complete UI parity; test docking from a tab to each supported split/drop region through a multi-stage pointer path and perform paired native Windows acceptance when supported.
- **id:** HUR-001-025
  **timestamp:** 2026-11-14T00:43:20Z
  **what:** Synchronized the plan's current-state summary with the delivered semantic tab reorder and split-resize interactions.
  **why:** The plan still listed headless tab reorder as incomplete after revisions 18–19 had implemented and verified it.
  **components:** docs/PLAN.md, this changelog, execution log.
  **type:** documentation correction.
  **validation:** Confirmed the stale phrases were unique before replacement; reread the temporary file, verified both intended replacements and absence of stale wording, then atomically replaced the plan.
  **performance:** No runtime effect.
  **risks:** Historical revision notes intentionally retain the status that applied when those revisions were written.
  **follow_up:** Continue source-to-target parity work; do not use the corrected summary as evidence that physical Windows acceptance or full 1:1 replacement is complete.
- **id:** HUR-001-026
  **timestamp:** 2026-11-14T01:01:56Z
  **what:** Added full-history session search through upstream `/api/sessions/search`, with a 200 ms UI debounce, profile-scoped `native-http` ticket, isolated bounded loopback HTTP worker, result correlation, ranked result mapping and loaded-only fallback matches. Added local HTTP and app merge fixtures; updated plan, backlog, architecture and README.
  **why:** Local filtering over the first loaded session page did not match upstream full-history search and left archived sessions undiscoverable.
  **components:** Cargo.toml, Cargo.lock, src/gateway.rs, src/app.rs, docs/PLAN.md, docs/BACKLOG.md, docs/ARCHITECTURE.md, README.md, this changelog, execution log.
  **type:** feature; parity; gateway transport; security boundary; documentation.
  **validation:** `cargo fmt` passed; `cargo check --all-targets` and automation-feature check passed; default tests passed 10/10. The first automation-feature run had one existing WebSocket fixture timeout (17/18 passed); that exact test passed alone on rerun, then the full automation-feature suite passed 21/21 (18 library, 3 protocol). A local HTTP fixture verified query encoding, `X-Hermes-Gateway-Ticket`, JSON parsing and ranked DTO fields. No installed Hermes profile or gateway was used.
  **performance:** Search work runs off the WebSocket actor. Requests have a 5-second timeout, 512 KiB response cap, at most 100 decoded rows and are coalesced to the most recent queued query. No benchmark run.
  **risks:** Live installed-profile route authorization and paired upstream/native visual acceptance remain unverified. Search error text is generic and no request-rate performance benchmark was run.
  **follow_up:** Verify live search with a separately isolated gateway/profile and capture matched upstream/native result states; continue open full-chat, route, native-window and agent-control parity gates.
- **id:** HUR-001-027
  **timestamp:** 2026-11-14T01:12:57Z
  **what:** Added nullable stable test IDs to the AccessKit tree for fixed first-party controls and enabled test-ID locators for click, type, select, drag and wait. Extended protocol capability reporting, documentation and headless integration coverage.
  **why:** Agent navigation should not depend exclusively on visible labels that can be renamed or localized; user requested Playwright-like operation of the native interface.
  **components:** src/bin/hermes_automation/automation_protocol.rs, docs/AGENT_AUTOMATION_PROTOCOL.md, docs/AGENT_AUTOMATION.md, docs/AGENT_CONTROL_CROSSWALK.md, docs/PLAN.md, docs/BACKLOG.md, this changelog, execution log.
  **type:** feature; agent control; accessibility; protocol; tests; documentation.
  **validation:** `cargo fmt -- --check` and feature-enabled `cargo check --all-targets` passed. Full automation-feature tests passed 22/22 (18 library and 4 protocol). The new protocol test verified stable IDs in the tree, click to route, type in session search, wait for visibility, selector choice and ID-based tab drag/reorder. An earlier test iteration attempted panel docking to a tab ID and failed because panel title drag and tab reorder have distinct semantics; corrected coverage uses a test-ID panel source for center docking and test-ID tab source/destination for reorder. No installed Hermes window/profile was opened.
  **performance:** Stable ID matching searches the bounded current AccessKit tree; no persistent index or runtime overhead was added outside tree/action inspection. No benchmark requested.
  **risks:** Explicit IDs currently cover fixed first-party controls only. Dynamic session rows and contributed/user controls remain label-based; the inherited-stdio protocol is not the authenticated external Debug Bridge or a physical window controller.
  **follow_up:** Assign IDs at dynamic row creation, implement the shared typed capability/action registry and continue background host control and source parity acceptance.
- **id:** HUR-001-028
  **timestamp:** 2026-11-14T01:20:05Z
  **what:** Added a versioned typed app-action registry for navigation, pane toggles, interface mode and layout profile selection. Human toolbar, route, session-row, pane and overlay controls now call shared app-core handlers. Added `app.actions.list` and generation-checked `app.action.execute` to the isolated UI protocol, including strict action deserialization and pane allowlisting.
  **why:** The supplied Doofus Apps architecture calls for shared typed operations so the coding agent can operate app behavior through the same handlers used by a person.
  **components:** src/app.rs, src/bin/hermes_automation/automation_protocol.rs, docs/AGENT_AUTOMATION_PROTOCOL.md, docs/ARCHITECTURE.md, docs/AGENT_AUTOMATION.md, docs/AGENT_CONTROL_CROSSWALK.md, docs/PLAN.md, docs/BACKLOG.md, README.md, this changelog, execution log.
  **type:** feature; shared app core; agent control; protocol; validation; documentation.
  **validation:** Formatting check, default all-target check and agent-automation all-target check passed. Default tests passed 10/10; feature tests passed 23/23 (18 library and 5 protocol). New integration test listed the typed catalog, executed `toggle_pane(files)`, then used the stable-ID UI click for the same Files control and observed the same state transition; an unregistered `workspace` pane action was rejected without changing state. No installed Hermes profile or window was opened.
  **performance:** Registry catalog is static and action dispatch is bounded by request size; no runtime benchmark was needed.
  **risks:** The shared registry is an initial slice, not a complete capability model. Navigation to a session may invoke the existing gateway resume path in the regular app. Per-action user-confirmation metadata, full DoofusDock/Hermes operation coverage, authenticated external bridge, diagnostics expansion and supervisor remain open.
  **follow_up:** Extend typed operations to the remaining panels and gateway workflows with explicit permission/confirmation/revision/idempotency metadata; continue current upstream UI source coverage and physical acceptance.
- **id:** HUR-001-029
  **timestamp:** 2026-11-14T01:21:22Z
  **what:** Removed a duplicate `app.action.execute` entry from the advertised capability catalog and added an integration assertion that capability IDs are unique.
  **why:** The new typed app-action endpoint had been advertised twice even though dispatch behavior worked; the catalog must be unambiguous for discovery clients.
  **components:** src/bin/hermes_automation/automation_protocol.rs, this changelog, execution log.
  **type:** bugfix; capability discovery; regression test.
  **validation:** Feature all-target check passed; agent-automation suite passed 23/23 (18 library and 5 protocol). The app-action integration test now enumerates all capability IDs and asserts uniqueness.
  **performance:** No runtime impact beyond test-time catalog validation.
  **risks:** Capability metadata remains an initial registry and must expand in step with newly implemented operations.
  **follow_up:** Maintain unique-ID assertion as the action catalog grows; proceed with full action coverage.

- **id:** HUR-001-030
  **timestamp:** 2026-11-14T01:31:47Z
  **what:** Replaced the Capabilities offline placeholder with source-style tab navigation, searchable live Skills/Toolsets read workflows, typed shared tab/refresh actions, and stable semantic control IDs. Kept Plugins and Connectors explicitly marked incomplete.
  **why:** Continue the approved 1:1 upstream Hermes UI replacement while honoring the supplemental shared agent-control architecture.
  **components:** src/gateway.rs, src/app.rs, src/bin/hermes_automation/automation_protocol.rs, README.md, docs/PLAN.md, docs/BACKLOG.md, docs/ARCHITECTURE.md, docs/AGENT_CONTROL_CROSSWALK.md, this changelog, execution log.
  **type:** feature; upstream route parity; gateway read path; semantic UI; protocol; documentation.
  **validation:** `cargo fmt --check` passed; `cargo check --all-targets --features agent-automation` passed; `cargo test --all-targets --features agent-automation` passed 24/24 (19 library, 5 protocol). Integration coverage navigates to Capabilities, verifies query route state, selects Toolsets by stable ID, and types into its search control. Local TCP fixture verifies both endpoints and one-use profile-scoped ticket headers. No installed Hermes profile or native desktop window was used.
  **performance:** HTTP work stays on a bounded worker (5-second timeout, redirects/proxy disabled, 512 KiB response bound); search filters the already-loaded in-memory collection. No benchmark requested.
  **risks:** Skills/Toolsets behavior is fixture-tested only; source visual parity and live profile response compatibility remain unverified. Plugins/Connectors, detail and mutation workflows, and full route parity remain incomplete.
  **follow_up:** Compare this page against a fresh upstream capture at the same viewport/theme; validate the local gateway read in an isolated profile; implement each remaining Capabilities subview and continue other source routes.

- **id:** HUR-001-031
  **timestamp:** 2026-11-14T01:35:59Z
  **what:** Captured and visually reviewed a headless 1440×900 Capabilities → Toolsets run; saved route, source pin, executable hash, screenshot hash, viewport and isolated-profile metadata.
  **why:** The supplemental architecture requires observable application behavior and reviewable screenshot evidence from unattended runs.
  **components:** artifacts/agent-ui/capabilities-toolsets-v23.png, artifacts/agent-ui/capabilities-toolsets-v23.json, docs/AGENT_AUTOMATION.md, this changelog, execution log.
  **type:** verification; screenshot evidence; agent automation.
  **validation:** The runner clicked Capabilities and Toolsets, inspected the semantic tree, and captured the image with no native window, desktop focus or gateway. Visual review confirmed the selected tab and search field. The gateway-disabled empty message is expected. SHA-256 values are stored in the metadata sidecar.
  **performance:** One-shot headless capture; no benchmark requested.
  **risks:** This verifies the disconnected route shell only; it does not establish live data rendering, source visual equivalence or Windows host behavior.
  **follow_up:** Capture matched upstream and native screens at same viewport/theme, then validate Skills/Toolsets data against an isolated Hermes profile.

- **id:** HUR-001-032
  **timestamp:** 2026-11-14T01:37:25Z
  **what:** Gated the query-aware route formatter behind the opt-in agent-automation feature that consumes it.
  **why:** Keep the default standalone build free of an unused-function warning while retaining route query state in the semantic automation protocol.
  **components:** src/app.rs, this changelog, execution log.
  **type:** build hygiene; feature boundary.
  **validation:** Default and feature-enabled all-target checks passed without warnings; agent-automation test suite passed 24/24. Screenshot metadata and JSONL evidence parsed successfully and the screenshot SHA-256 matched its sidecar.
  **performance:** No runtime change in either build.
  **risks:** None identified.
  **follow_up:** Keep automation-only state and diagnostics behind the opt-in Cargo feature unless they are also used by standalone UI code.

- **id:** HUR-001-033
  **timestamp:** 2026-11-14T23:07:10Z
  **what:** Added bundled JetBrains Mono UI and Fira Code monospace typography with OFL 1.1 texts and source metadata; set Fira Code on the message composer; refreshed the source pin and revalidated the latest Hermes Desktop delta to `a7904dc`.
  **why:** Apply the user's selected typefaces and keep the 1:1 parity target aligned to current upstream Hermes main.
  **components:** src/fonts.rs, src/app.rs, src/lib.rs, assets/fonts, docs/PLAN.md, docs/BACKLOG.md, docs/SOURCE_AUDIT.md, docs/ARCHITECTURE.md, README.md, artifacts/agent-ui/font-choice-v24.png, artifacts/agent-ui/font-choice-v24.json, this changelog, execution log.
  **type:** user-directed typography; source audit; bundled assets; agent screenshot evidence.
  **validation:** Both official font release archives and license texts were retrieved and hashed. `cargo fmt --check`, default/feature all-target checks, and feature tests passed 25/25 (20 library, 5 protocol). A 1440×900 headless run rendered the updated UI; screenshot and font hashes are stored in its sidecar. Latest Hermes main was read-only fetched to a separate audit ref; the diff from `5163cc6` touches only unrelated Markdown and zoomable-image components. No installed app, Hermes profile or native desktop window was changed.
  **performance:** Bundled font definitions install once per app context. No benchmark requested.
  **risks:** UI typography intentionally follows the user's font preference and therefore overrides the upstream font appearance. Font rasterization on an actual Windows window remains unverified.
  **follow_up:** Compare font scale/legibility on a physical Windows run; continue source-backed Capabilities subviews and the remaining upstream parity inventory.

- **id:** HUR-001-034
  **timestamp:** 2026-11-14T23:17:54Z
  **what:** Added Toolsets enable/disable through the upstream PUT endpoint, a one-use profile-scoped native HTTP ticket, a shared typed human/agent action, optimistic state with rollback, and FIFO mutation handling. Repaired the fixture's fragmented HTTP request parsing and removed an unused-mut warning.
  **why:** Complete the next source-backed Capabilities behavior and make its writes observable and safe to retry.
  **components:** src/gateway.rs, src/app.rs, src/bin/hermes_automation/automation_protocol.rs, README.md, docs/PLAN.md, docs/BACKLOG.md, docs/ARCHITECTURE.md, docs/AGENT_CONTROL_CROSSWALK.md, this changelog, execution log.
  **type:** feature; gateway mutation; semantic UI; regression fix; documentation.
  **validation:** `cargo fmt --all -- --check` passed; `cargo check --all-targets --features agent-automation` passed without warnings; `cargo test --all-targets --features agent-automation` passed 26/26 (21 library, 5 protocol). Local TCP fixture verifies PUT path encoding, ticket header, JSON body, accepted receipt and mismatched-state rejection. Protocol test confirms disconnected mutations fail closed. No installed Hermes profile or native desktop window was used.
  **performance:** Requests remain on the bounded worker (5-second timeout, redirects/proxy disabled, 512 KiB response bound). FIFO avoids losing accepted writes. No benchmark requested.
  **risks:** Live installed-profile behavior and source-paired visual parity remain unverified. Plugins and Connectors are incomplete; overall 1:1 acceptance remains open.
  **follow_up:** Verify the Toolsets toggle against an isolated live Hermes profile and add paired source/native screenshot evidence; continue the remaining Capabilities and chat parity workflows.
- **id:** HUR-001-036
  **timestamp:** 2026-11-14T23:22:10Z
  **what:** Updated the backlog's summary of the shared app-action registry to include Toolsets enable/disable.
  **why:** Keep the high-level automation status consistent with the current registry and detailed protocol documentation.
  **components:** docs/BACKLOG.md, this changelog, execution log.
  **type:** documentation.
  **validation:** Compared the backlog statement with `app_action_catalog()` and HUR-001-034 protocol coverage; execution log was parsed as JSONL before replacement.
  **performance:** Documentation only; no runtime changes.
  **risks:** Overall agent control remains incomplete.
  **follow_up:** Update the backlog whenever the shared action registry gains additional workflows.
- **id:** HUR-001-037
  **timestamp:** 2026-11-14T23:38:54Z
  **what:** Added active-response Stop through the upstream `session.interrupt` RPC; exposed a shared `stop_generation` UI/agent action, accessible `chat.stop` locator, and redacted chat capability state; refreshed the Hermes source audit to `8783054`.
  **why:** Close a core chat control gap while preserving the Python gateway as the authority and keeping background agent state inspectable.
  **components:** src/gateway.rs, src/app.rs, src/bin/hermes_automation/automation_protocol.rs, README.md, docs/PLAN.md, docs/BACKLOG.md, docs/SOURCE_AUDIT.md, docs/ARCHITECTURE.md, docs/AGENT_AUTOMATION.md, docs/AGENT_AUTOMATION_PROTOCOL.md, docs/AGENT_CONTROL_CROSSWALK.md, this changelog, execution log.
  **type:** chat workflow; gateway RPC; accessible control; agent action; source re-audit; documentation.
  **validation:** `cargo fmt --all -- --check`, default and agent-automation `cargo check --all-targets`, and `cargo test --all-targets --features agent-automation` passed; 29 tests passed (24 library, 5 protocol). WebSocket fixtures verified runtime session IDs and both `interrupted` and `not_interrupted`; app tests covered stale correlation, success, idle and error results; headless AccessKit verified the enabled Stop button and stable locator. One parallel run exposed a 3-second fixture timeout under contention; increasing that fixture's bounded wait to 10 seconds made the full parallel suite pass. No installed Hermes profile or native window was used.
  **performance:** Stop remains on the existing WebSocket worker; no benchmark requested.
  **risks:** Live installed-profile behavior, approval/server-request handling, full chat event parity and source-paired screenshots remain open. The upstream delta from `a7904dc` to `8783054` spans 1,070 repository files and is not fully reconciled.
  **follow_up:** Validate Stop against an isolated live Hermes gateway; continue source reconciliation and implement server requests, reconnect, and full transcript/tool event projection.
## HUR-001-038 — Generation-scoped chat approvals

- **id:** HUR-001-038
- **timestamp:** 2026-11-14T23:51:52Z
- **what:** Render current-session pproval.request events as accessible cards; send only offered choices through pproval.respond with the prompt session and execution generation; expose the same decision handler in the typed agent action registry; capture the current offline new-chat screen.
- **why:** Close a user-visible safety workflow gap while keeping Hermes authoritative for prompt validity and approval outcomes.
- **components:** src/gateway.rs, src/app.rs, src/bin/hermes_automation/automation_protocol.rs, README.md, docs/PLAN.md, docs/BACKLOG.md, docs/ARCHITECTURE.md, docs/AGENT_AUTOMATION.md, docs/AGENT_AUTOMATION_PROTOCOL.md, docs/AGENT_CONTROL_CROSSWALK.md, artifacts/agent-ui/chat-approval-v27.png, its metadata, this changelog, execution log.
- **type:** chat workflow; gateway RPC; accessible approval UI; agent action; screenshot evidence; documentation.
- **validation:** cargo fmt --all -- --check, default and agent-automation cargo check --all-targets, and cargo test --all-targets --features agent-automation passed; 31/31 tests passed (26 library, 5 protocol). A local WebSocket fixture verified session/generation/prompt/choice params, accepted a matching resolved receipt, and rejected a mismatched prompt receipt. An app fixture verified canonical event projection, offered-choice restriction, stale prompt rejection, and offline-disabled accessible control. The headless runner captured the offline new-chat view at 1440x900 with isolated profile; it does not show an approval card because no live approval event was sent.
- **performance:** Uses the existing WebSocket worker; no benchmark requested.
- **risks:** Live approval behavior, reconnect/replay, clarify prompts, true server-to-client JSON-RPC request handling, full chat parity, and source-paired/physical Windows visual acceptance remain open. Initial full test run caught an outdated eight-action catalog assertion; it was corrected and the final suite passed.
- **follow_up:** Implement clarify and server-request workflows, complete event reconciliation and current-source audit, then validate on an isolated live Hermes gateway without touching the installed user profile.
## HUR-001-039 — Session-scoped clarify prompts

- **id:** HUR-001-039
- **timestamp:** 2026-11-15T00:03:46Z
- **what:** Render active-session single-question clarify.request events with offered choices, free text, multi-select serialization, skip, and replayed-answer staging; send answers through generation-scoped clarify.respond; expose the same handler to agents; disable controls offline or while a response is pending.
- **why:** Implement the current upstream Desktop clarify event workflow while keeping session, generation and answer validity owned by Hermes.
- **components:** src/gateway.rs, src/app.rs, src/bin/hermes_automation/automation_protocol.rs, README.md, docs/PLAN.md, docs/BACKLOG.md, docs/ARCHITECTURE.md, docs/AGENT_AUTOMATION.md, docs/AGENT_AUTOMATION_PROTOCOL.md, docs/AGENT_CONTROL_CROSSWALK.md, this changelog, execution log.
- **type:** chat workflow; gateway RPC; accessible clarify UI; agent action; source contract reconciliation; documentation.
- **validation:** cargo fmt --all -- --check, default and agent-automation cargo check --all-targets, and cargo test --all-targets --features agent-automation passed; 33/33 tests passed (28 library, 5 protocol). The WebSocket fixture checked multi-select JSON answer and skip params, matching receipts, and mismatched prompt rejection. App/AccessKit fixture checked event projection, answer encoding order, stale/oversized action rejection and offline-disabled choice/skip controls.
- **performance:** Uses the existing WebSocket worker; no benchmark requested.
- **risks:** Batch clarify uses the separate clarify.lock server-request flow and is not implemented here; inbound JSON-RPC server requests, replay/reconnect, live gateway acceptance, and complete chat/visual parity remain open.
- **follow_up:** Implement batch clarify and supported inbound server requests, then validate through an isolated live gateway and continue source-paired Windows UI acceptance.## HUR-001-040 — Generation-scoped tool-call transcript cards

- **id:** HUR-001-040
- **timestamp:** 2026-11-15T00:18:19Z
- **what:** Project active-session `tool.start`/`tool.complete` events into transcript cards, reconcile a reused tool ID within its execution generation, preserve start arguments, show bounded completion fields, and expose the speaker/body in a combined AccessKit group label. Capture an isolated rendered fixture.
- **why:** Add visible and agent-inspectable tool progress to the current chat slice while retaining Hermes as the source of execution state.
- **components:** src/app.rs, README.md, docs/PLAN.md, docs/BACKLOG.md, docs/ARCHITECTURE.md, docs/AGENT_AUTOMATION.md, artifacts/agent-ui/tool-cards-v40.png and its JSON sidecar, this changelog, execution log.
- **type:** chat event projection; semantic UI; screenshot evidence; documentation.
- **validation:** `cargo fmt --all -- --check`, default and `agent-automation` `cargo check --all-targets`, and `cargo test --all-targets --features agent-automation` passed; 34/34 tests passed (29 library, 5 protocol). The app fixture verifies two generations reusing one tool ID and finds both rendered groups in AccessKit. The 1440×900 screenshot and sidecar hashes match.
- **performance:** Uses the existing gateway event path; transcript values are bounded to 6,000 characters. No benchmark requested.
- **risks:** This is basic start/complete projection only. Out-of-order replay/reconnect, full upstream tool/reasoning visuals, live gateway validation, source-paired visuals, and full replacement parity remain open.
- **follow_up:** Continue current-source UI/event reconciliation, including JSON-RPC server requests and out-of-order tool event behavior; then capture matched upstream and native screens against the acceptance matrix.## HUR-001-041 — Fail-fast handling for unhandled server requests

- **id:** HUR-001-041
- **timestamp:** 2026-11-15T00:25:50Z
- **what:** Recognize inbound JSON-RPC method requests during an outstanding gateway call and return the correlated `-32601` response when the native client has no handler; continue processing the original RPC. Apply the same fallback in the idle gateway event loop.
- **why:** Match the pinned upstream channel's unhandled-request behavior and prevent an unexpected server request from stalling the gateway reader until timeout.
- **components:** src/gateway.rs, README.md, docs/PLAN.md, docs/BACKLOG.md, docs/ARCHITECTURE.md, this changelog, execution log.
- **type:** gateway protocol handling; regression coverage; documentation.
- **validation:** `cargo fmt --all -- --check`, default and `agent-automation` `cargo check --all-targets`, and `cargo test --all-targets --features agent-automation` passed; 35/35 tests passed (30 library, 5 protocol). Local WebSocket fixture verified the `-32601` response uses the inbound request ID and the outstanding `runtime.describe` RPC still completes.
- **performance:** Uses the existing WebSocket read loop; no benchmark requested.
- **risks:** Interactive server-request handlers are not implemented and `server_requests` remains false. This fallback does not close the server-request parity gap or establish live gateway behavior.
- **follow_up:** Add typed request routing and app-owned response handling for approval and batch clarify; negotiate support only when those handlers are ready, then cover other source methods individually.## HUR-001-042 — Active-session approval server-request card

- **id:** HUR-001-042
- **timestamp:** 2026-11-15T00:36:21Z
- **what:** Forward idle inbound `approval` requests to the app, show the existing accessible permission card for the active runtime session, honor the offered choices and `allow_permanent`, and answer through the original JSON-RPC request ID. Decline requests for sessions not shown by this single-session native window. Capture an isolated 1440×900 card fixture.
- **why:** Move approval parity from notification-only prompts toward Hermes' current socket-owned server-request contract.
- **components:** src/gateway.rs, src/app.rs, README.md, docs/PLAN.md, docs/BACKLOG.md, docs/ARCHITECTURE.md, docs/AGENT_AUTOMATION.md, artifacts/agent-ui/approval-request-v31.png and its JSON sidecar, this changelog, execution log.
- **type:** gateway server-request routing; accessible UI; screenshot evidence; documentation.
- **validation:** `cargo fmt --all -- --check`, default and `agent-automation` `cargo check --all-targets`, and `cargo test --all-targets --features agent-automation` passed; 37/37 tests passed (32 library, 5 protocol). The WebSocket fixture verified the inbound request reaches the app event channel and a `{choice}` result returns with the same request ID. The UI fixture verified the accessible card, backend choice filtering and offline-disabled control. Screenshot sidecar hash matches the 1440×900 image.
- **performance:** Uses the existing WebSocket actor and UI event loop; no benchmark requested.
- **risks:** `client.capabilities.server_requests` remains false because batch clarify, other methods, replay/ownership and live gateway acceptance remain open. This is local fixture evidence, not active gateway behavior or complete parity.
- **follow_up:** Generalize clarify prompt state to the upstream 1–5 question batch, support ordered answers and replay locks, then test a full server-request interaction before advertising support.
## HUR-001-043 — Upstream batch clarify request card

- **id:** HUR-001-043
- **timestamp:** 2026-11-15T01:00:16Z
- **what:** Add active-session batch `clarify` server-request routing, editable replay staging, upstream progress/selection labels, ordered per-question `clarify.lock` RPC submission, empty-result Skip, pending/error recovery, and a captured 1440×900 UI fixture.
- **why:** Bring the native chat's multi-question prompt path in line with the pinned Hermes Desktop implementation and make it available through the shared accessible UI/action handler.
- **components:** `src/app.rs`, `src/gateway.rs`, `README.md`, `docs/PLAN.md`, `docs/BACKLOG.md`, `docs/ARCHITECTURE.md`, `docs/AGENT_AUTOMATION.md`, `artifacts/agent-ui/batch-clarify-v32.png`, its JSON sidecar, this changelog, and `docs/EXECUTION_LOGS.md`.
- **type:** gateway request handling; accessible chat UI; agent action; fixture evidence; documentation.
- **validation:** `cargo fmt --all -- --check`, `cargo check --all-targets`, `cargo check --all-targets --features agent-automation`, and `cargo test --all-targets --features agent-automation` passed. 41/41 tests passed (36 library, 5 protocol). Local WebSocket fixtures cover idle server-request routing and a `clarify.lock` request/receipt; AccessKit checks the active-session card, editable replay choice, question labels and actions; the pure helper verifies recommendation stripping, multi-select JSON encoding and unanswered null values. The screenshot sidecar hash and 1440×900 PNG dimensions were verified.
- **performance:** No performance optimization or benchmark was part of this UI/protocol slice; no benchmark claim made.
- **risks:** The client keeps `server_requests` disabled while other methods, in-flight request multiplexing and multiwindow ownership are incomplete. Validation is local fixture/headless evidence; no live Hermes gateway, installed profile, native Windows host or paired upstream visual acceptance was exercised.
- **follow_up:** Continue the source-paired route and workflow inventory, implement remaining server-request methods and transport ownership, then compare native screenshots and interaction behavior against the pinned/current upstream Desktop UI before any migration decision.

## HUR-001-044 — Fixed source snapshot and sidebar recency parity

- **id:** HUR-001-044
- **timestamp:** 2026-11-15T01:17:09Z
- **what:** Pinned the downloaded Hermes parity snapshot at `8783054ef6cbafd86f10892241436862c1171a66` in immutable local ref `refs/codex/parity/hermes-8783054`. Added the pinned Desktop `/api/sessions` read with a fresh native HTTP ticket, `last_active || started_at` sidebar rendering, Gateway-list fallback/carry-forward, source and plan language that does not follow later upstream commits, and local request/recency fixtures.
- **why:** Keep the requested 1:1 parity target stable and close the audited sidebar recency gap against that downloaded source.
- **components:** `src/gateway.rs`, `src/app.rs`, `README.md`, `docs/PLAN.md`, `docs/SOURCE_AUDIT.md`, `docs/BACKLOG.md`, `docs/ARCHITECTURE.md`, `docs/CHANGELOG.agent.md`, `docs/EXECUTION_LOGS.md`, isolated source checkout Git ref.
- **type:** fixed source pin; session HTTP read; sidebar parity; fixture validation; documentation.
- **validation:** `cargo fmt --all -- --check`, `cargo check --all-targets`, `cargo check --all-targets --features agent-automation`, and `cargo test --all-targets --features agent-automation` passed. 42/42 tests passed (37 library, 5 automation). The HTTP fixture verifies exact recent-list query, native ticket header, fractional `last_active`, and nullable fields; the app test verifies `last_active` precedence and zero fallback. `git show-ref --verify refs/codex/parity/hermes-8783054` resolves to the specified source SHA.
- **performance:** No performance optimization or benchmark was requested; the list read runs on the existing bounded HTTP worker.
- **risks:** Live installed-profile API behavior and paired source/target visual acceptance remain open. The fallback preserves visible rows if HTTP recency refresh fails.
- **follow_up:** Continue the capability and workflow gaps against the fixed snapshot; do not re-resolve upstream `main` unless the user selects a replacement source.

## HUR-001-045 — Keep session-list warnings in the Sessions pane

- **id:** HUR-001-045
- **timestamp:** 2026-11-15T01:18:56Z
- **what:** Surface session-list refresh failures in the Sessions pane and clear the warning after a successful refresh, without replacing the global app status.
- **why:** Keep the fallback state visible without overwriting unrelated status from active chat or UI actions.
- **components:** `src/app.rs`, `docs/SOURCE_AUDIT.md`, `docs/CHANGELOG.agent.md`, `docs/EXECUTION_LOGS.md`.
- **type:** UI state reporting; retry/fallback clarity.
- **validation:** `cargo fmt --all -- --check`, both default and `agent-automation` all-target checks, and the full `agent-automation` all-target test suite passed (42/42 tests).
- **performance:** No additional requests or polling; the existing session-list event updates one optional UI message.
- **risks:** The warning appears when the Sessions pane is opened; the Gateway session list remains available.
- **follow_up:** Continue parity against the fixed `8783054` snapshot and keep live API acceptance open.

## HUR-001-046 — Stable semantic locators for session rows

- **id:** HUR-001-046
- **timestamp:** 2026-11-15T01:26:13Z
- **what:** Add the stored session ID to each row's accessible name and derive stable automation IDs in the form `session.row.<stored-id>`.
- **why:** Let the agent select the exact visible conversation when titles repeat, using the same accessible widget that a person clicks.
- **components:** `src/app.rs`, `src/bin/hermes_automation/automation_protocol.rs`, `docs/PLAN.md`, `docs/BACKLOG.md`, `docs/AGENT_AUTOMATION.md`, `docs/CHANGELOG.agent.md`, `docs/EXECUTION_LOGS.md`.
- **type:** semantic accessibility; agent locator.
- **validation:** Formatting and default/feature all-target checks passed; feature suite passed 43/43 tests (38 library, 5 automation). App fixture verified the visible AccessKit row name and Button role; protocol fixture verified the dynamic ID mapping and existing stable locator behavior.
- **performance:** Locator IDs are derived from the existing AccessKit name when the tree is read; no new background work or UI state is stored.
- **risks:** IDs are emitted for session identifiers up to 112 ASCII alphanumeric, hyphen, underscore, or period characters; other ID shapes remain accessible by role/name until normalized.
- **follow_up:** Extend stable IDs to remaining dynamic first-party actions and verify locators in the long-running stdio process against a populated isolated session fixture.

## HUR-001-047 — Fenced session Pin/Unpin action

- **id:** HUR-001-047
- **timestamp:** 2026-11-15T01:34:37Z
- **what:** Add accessible per-session Pin/Unpin menu actions and a correlated HTTP workflow that reads the Desktop mutation snapshot, then PATCHes the pin value with a new ticket and snapshot revision/generation. The row changes only after `{ok:true}`.
- **why:** Close a visible session-row action gap in the fixed Hermes snapshot while preserving its mutation fencing and confirmation behavior.
- **components:** `src/gateway.rs`, `src/app.rs`, `src/bin/hermes_automation/automation_protocol.rs`, `README.md`, `docs/PLAN.md`, `docs/BACKLOG.md`, `docs/SOURCE_AUDIT.md`, `docs/AGENT_AUTOMATION.md`, `docs/CHANGELOG.agent.md`, `docs/EXECUTION_LOGS.md`.
- **type:** session API mutation; accessible UI; agent locators.
- **validation:** Formatting and default/feature all-target checks passed; feature test suite passed 44/44 tests (39 library, 5 automation). Local HTTP fixture verifies the profile-scoped snapshot URL, separate ticket headers, expected revision/generation, request ID, pin payload and positive receipt. AccessKit fixture verifies the menu and offline-disabled Pin control; protocol fixture verifies menu/action locators.
- **performance:** Two bounded loopback HTTP requests run on the existing serialized HTTP worker only after user activation.
- **risks:** Live-profile mutation remains untested. Ambiguous network failure leaves the UI unchanged and reports failure; the operator can retry from the menu.
- **follow_up:** Implement the remaining pinned-source session actions (unread, archive, rename and confirmed delete), then validate against an isolated Hermes profile.

## HUR-001-048 — Persisted session unread action

- **id:** HUR-001-048
- **timestamp:** 2026-11-15T01:39:39Z
- **what:** Decode the Desktop session-list `unread` field and add accessible Mark unread/Mark read actions using the shared revision/generation-fenced mutation path, separate native HTTP tickets, correlated receipts, and stable `session.unread.<stored-id>` locators.
- **why:** Match another persisted session-row action in the fixed Hermes source while keeping the visible state consistent with its confirmed backend receipt.
- **components:** `src/gateway.rs`, `src/app.rs`, `src/bin/hermes_automation/automation_protocol.rs`, `README.md`, `docs/PLAN.md`, `docs/BACKLOG.md`, `docs/ARCHITECTURE.md`, `docs/SOURCE_AUDIT.md`, `docs/AGENT_AUTOMATION.md`, `docs/CHANGELOG.agent.md`, `docs/EXECUTION_LOGS.md`.
- **type:** session API mutation; accessible UI; agent locators.
- **validation:** Formatting and default/feature all-target checks passed; feature suite passed 45/45 tests (40 library, 5 automation). Local HTTP fixture verified `unread:true` plus request ID, expected revision and generation; AccessKit fixture verified the Mark unread item is present and disabled offline; locator test verified the dynamic unread action ID.
- **performance:** Uses the existing serialized HTTP worker and only runs after an explicit row-menu selection.
- **risks:** Persisted unread state is covered; transient finished-response unread indicators and `markSessionRead` acknowledgement parity are still open. Live-profile mutation acceptance remains open.
- **follow_up:** Add the source's transient unread/read acknowledgements, then implement archive, rename and confirmed delete actions.

## HUR-001-049 — Fenced session Archive action

- **id:** HUR-001-049
- **timestamp:** 2026-11-15T01:43:47Z
- **what:** Add accessible Archive/Unarchive-labelled session menu control, archived DTO decoding, fixed-source fenced PATCH behavior, archive receipt handling, and stable `session.archive.<stored-id>` agent locator. Archiving clears pin state and removes the confirmed row from active sessions and search results.
- **why:** Implement the next persisted session-row action in the downloaded parity snapshot without expanding scope to moving upstream changes.
- **components:** `src/gateway.rs`, `src/app.rs`, `src/bin/hermes_automation/automation_protocol.rs`, `README.md`, `docs/PLAN.md`, `docs/BACKLOG.md`, `docs/ARCHITECTURE.md`, `docs/SOURCE_AUDIT.md`, `docs/AGENT_AUTOMATION.md`, `docs/CHANGELOG.agent.md`, `docs/EXECUTION_LOGS.md`.
- **type:** session API mutation; accessible UI; agent locator.
- **validation:** `cargo fmt --all -- --check`, default and `agent-automation` all-target checks passed; full feature suite passed 46/46 tests (41 library, 5 protocol). The local HTTP fixture verifies the archive request also clears `pinned`; AccessKit fixture verifies the action and offline-disabled state; protocol test verifies the stable locator. Pinned source ref still resolves to `8783054ef6cbafd86f10892241436862c1171a66`.
- **performance:** Reuses the existing serialized HTTP worker and performs the snapshot read/PATCH only after explicit row-menu action.
- **risks:** Live-profile mutation is unverified; no archived-session view exists, so the labelled Unarchive control is not currently reachable. Source-paired and native-window parity remain open.
- **follow_up:** Implement the source archived-session navigation/query and reachable Unarchive path, then continue remaining session-row parity against the fixed snapshot.

## HUR-001-050 — Paged archived sessions and Unarchive

- **id:** HUR-001-050
- **timestamp:** 2026-11-15T01:56:50Z
- **what:** Add a Sessions view toggle for archived conversations, page the attached profile's `archived=only` endpoint with a fresh ticket for each fixed-size page, deduplicate backend pinned-row backfills, filter loaded archived rows locally, and move a restored row back to active sessions after the fenced Unarchive receipt. Add accessible stable IDs for both view controls and expose archived-view state/count/loading in agent diagnostics.
- **why:** Close the reachability gap left by Archive so users and agents can browse archived sessions and restore one, following the downloaded Hermes source contract.
- **components:** `src/gateway.rs`, `src/app.rs`, `src/bin/hermes_automation/automation_protocol.rs`, `README.md`, `docs/PLAN.md`, `docs/BACKLOG.md`, `docs/ARCHITECTURE.md`, `docs/SOURCE_AUDIT.md`, `docs/AGENT_AUTOMATION.md`, `docs/CHANGELOG.agent.md`, `docs/EXECUTION_LOGS.md`.
- **type:** session query; accessible UI; agent navigation and diagnostics.
- **validation:** `cargo fmt --all -- --check`, default and `agent-automation` all-target checks passed; full feature suite passed 50/50 tests (45 library, 5 protocol). HTTP fixtures verify `archived=only` with offset and total; pagination fixture verifies fixed offsets and duplicate pinned-row handling; AccessKit fixtures verify view switching, visible archived rows, and offline-disabled Unarchive; protocol fixture verifies both toggle locators.
- **performance:** Reads pages serially on the existing background HTTP worker, with at most one bounded 512 KiB JSON response in flight; each page uses a separate one-use ticket.
- **risks:** Current implementation lists the attached profile only, while the source can aggregate archived rows across profiles. Archived search filters loaded rows locally rather than searching all archived history. Live-profile reads/mutations and source-paired visuals remain open.
- **follow_up:** Implement profile aggregation/switching and full archived-history search, then continue the fixed-source capability inventory and paired UI evidence.

## HUR-001-051 — Cross-profile archived sessions and scoped mutations

- **id:** HUR-001-051
- **timestamp:** 2026-11-15T02:11:33Z
- **what:** Change archived browsing to the fixed Desktop `/api/profiles/sessions?profile=all&archived=only` contract with 500-row pages. Preserve profile ownership in session DTOs, deduplicate by profile plus stored ID, carry the owning profile through Pin/Unread/Archive mutation snapshot and PATCH requests, add profile-scoped accessible names/locators, and disable opening foreign-profile rows until chat operations carry profile scope.
- **why:** Match the downloaded source's cross-profile archive behavior while preventing an archived row from being resumed or mutated under the wrong profile.
- **components:** `src/gateway.rs`, `src/app.rs`, `src/bin/hermes_automation/automation_protocol.rs`, `README.md`, `docs/PLAN.md`, `docs/BACKLOG.md`, `docs/ARCHITECTURE.md`, `docs/SOURCE_AUDIT.md`, `docs/AGENT_AUTOMATION.md`, `docs/CHANGELOG.agent.md`, `docs/EXECUTION_LOGS.md`.
- **type:** profile-scoped session query and mutation; accessible UI; agent locators.
- **validation:** `cargo fmt --all -- --check`, default and `agent-automation` all-target checks passed; feature suite passed 50/50 tests (45 library, 5 protocol). HTTP fixture verifies `/api/profiles/sessions`, page size/offset, `profile=all`, and decoded row profile; pagination fixture verifies profile-distinct same-ID rows and pinned-backfill deduplication; archive mutation fixture verifies explicit profile and pin clearing; AccessKit fixture verifies profile identity, disabled foreign-profile opening, and reachable Unarchive; protocol fixture verifies scoped locators.
- **performance:** Keeps the source's 500-row page size; response bytes remain bounded by the existing 512 KiB HTTP limit and each page uses a one-use ticket.
- **risks:** Cross-profile open/resume and subsequent prompt/interrupt/approval/clarify operations are not yet wired, so those rows remain disabled for opening. Remote gateway connections and live-profile evidence are also open.
- **follow_up:** Carry profile scope through session resume and the full active chat lifecycle, then implement archived full-history search and continue the fixed-source inventory.

## HUR-001-052 — Fixed-source CommonMark rendering and repository workspace

- **id:** HUR-001-052
- **timestamp:** 2026-11-15T04:35:56Z
- **what:** Replaced the line parser with egui_commonmark 0.26/pulldown-cmark and syntect; added isolated Markdown fixture screenshots; patched the code copy affordance to use an accessible, correctly sized text button; made Hermes Native the Cargo workspace default and corrected active acceptance documentation to the fixed 565-item snapshot.
- **why:** The pinned source's chat uses structured Markdown and code rendering; the chosen fonts lacked a glyph required by the third-party copy control. Repository root commands must launch the active Rust app while retaining the previous prototype.
- **components:** Cargo workspace/lockfiles, apps/hermes-native/src/app.rs, apps/hermes-native/src/bin/hermes-automation.rs, vendored egui_commonmark_backend, active README/architecture/backlog/plan and screenshot fixture.
- **type:** transcript rendering; isolated visual fixture; workspace setup; parity-scope correction.
- **validation:** Formatting, root default-member build, preserved root-prototype build, feature build and full tests pass (57 library + 5 automation). A live headless runner exposed a Markdown table value and Copy button in AccessKit and captured a 1440x900 screenshot.
- **performance:** Not benchmarked; Markdown cache is retained by the app instance.
- **risks:** Streamdown streaming repair, math, media, transcript directives, artifact promotion, rich cards, and physical Windows source-paired visual acceptance remain open. The vendored backend diff must be reviewed on dependency upgrade.
- **follow_up:** Continue the fixed 565-row snapshot implementation and source-paired Windows acceptance. Later upstream changes do not expand scope.

## HUR-001-053 — Publish the reviewed project to the user-provided repository

- **id:** HUR-001-053
- **timestamp:** 2026-11-15T04:37:44Z
- **what:** Initialized the extracted workspace as the main branch of robertheadley/Doofus-Dock-Hermes and pushed the reviewed 230-file baseline. Root Cargo defaults to hermes-native while the root prototype remains buildable and rev1 remains preserved. Downloaded upstream staging and generated/local files are excluded.
- **why:** The user supplied this repository as the canonical place to continue the approved Hermes Native replacement.
- **components:** repository root, Cargo workspace, README, fixed 565-item source package, apps/hermes-native and both prototype trees.
- **type:** repository setup and publication.
- **validation:** Staged-path audit found no env, target, node_modules, staging or temp paths; cached diff whitespace check passed; commit 3c4f905de63f5d2fae2fe3c1e35645abe539aba9 is on origin/main and local main tracks origin/main cleanly.
- **performance:** Not applicable.
- **risks:** The GitHub repository is public. The exact Hermes source tree is intentionally local-only; the snapshot SHA and acceptance evidence are tracked.
- **follow_up:** Continue source-paired implementation in this checkout without periodic upstream refreshes.



## HUR-001-054

- **id:** HUR-001-054
- **timestamp:** 2026-11-15T04:57:17Z
- **what:** Added bounded older-transcript pagination, profile/session/order validation, stale-response fencing, overlap-safe prepend, an accessible history action, and a headless visual fixture with stable agent locator.
- **why:** Hermes snapshot `8783054ef6cbafd86f10892241436862c1171a66` hydrates a 120-message tail and fetches earlier messages on demand.
- **components:** apps/hermes-native/src/gateway.rs, app.rs, automation runner/protocol, long-session screenshot, active plan/backlog/source audit, README, architecture, execution log.
- **type:** transcript history feature; local protocol and visual evidence.
- **validation:** `cargo fmt --all`; `cargo check -p hermes-native --all-targets --features agent-automation`; `cargo test -p hermes-native --all-targets --features agent-automation` passed (62 library + 5 automation tests). A headless 1440×900 screenshot was rendered and visually inspected.
- **performance:** No benchmark; each page is capped at 120 messages and the existing HTTP response limit is 512 KiB.
- **risks:** Live gateway acceptance is open. Transcript virtualization, source scroll-window and scroll-anchor behavior, and retention remain incomplete.
- **follow_up:** Implement the source-equivalent transcript window and scroll-driven “Show earlier” behavior, then validate against isolated fixed-snapshot gateway data.



## HUR-001-055

- **id:** HUR-001-055
- **timestamp:** 2026-11-15T05:01:08Z
- **what:** Normalized the isolated long-session screenshot from RGBA to RGB after Codex preview showed black; retained the correctly rendered artifact at `apps/hermes-native/artifacts/agent-ui/long-session-history-20261115-visible.png`.
- **why:** Keep visual evidence inspectable and prevent an invalid black preview from being treated as the UI result.
- **components:** long-session agent UI screenshot and execution/change journals.
- **type:** artifact verification.
- **validation:** `ffprobe` reports 1440×900 RGB24; the image was visually inspected and shows the app shell, fixture transcript and disabled offline history action.
- **performance:** Not applicable.
- **risks:** Headless fixture only; it does not prove live gateway behavior.
- **follow_up:** Continue source-driven implementation and matched fixed-snapshot visual checks.



## HUR-001-056

- **id:** HUR-001-056
- **timestamp:** 2026-11-15T05:04:36Z
- **what:** Adjusted transcript history compatibility so the 120-row cap applies only to responses that confirm pagination; legacy complete transcripts may exceed one page under explicit payload and row limits.
- **why:** The pinned Hermes Desktop supports legacy backends that return the entire transcript without pagination metadata.
- **components:** apps/hermes-native/src/gateway.rs, active plan/backlog/source audit, execution log.
- **type:** compatibility fix and negative-path fixture.
- **validation:** `cargo fmt --all`; feature test suite passed (63 library + 5 automation tests); a local HTTP fixture returns 121 complete legacy rows and confirms whole-transcript replacement.
- **performance:** Complete response bounded by 512 KiB and 10,000 rows; normal paginated reads remain 120 rows.
- **risks:** Live source gateway compatibility remains unverified.
- **follow_up:** Exercise current and legacy page shapes against an isolated gateway from the pinned snapshot.

### Revision 45 — source-faithful session-tab architecture plan

- **id:** HUR-001-057
- **timestamp:** 2026-11-15T05:11:13Z
- **what:** Recorded a source-backed implementation sequence for dynamic session panes, independent per-session state, session lifecycle, close/promotion, and agent-operated controls.
- **why:** The current one-item chat strip cannot satisfy the pinned Desktop's dynamic session tiles; duplicating tab state outside DoofusDock would split canonical layout ownership.
- **components:** apps/hermes-native/src/app.rs; DoofusDock pane registry and DockingService; gateway event routing; automation protocol; PLAN.md; BACKLOG.md; root README and architecture.
- **type:** parity architecture plan.
- **validation:** Inspected pinned Hermes files `apps/desktop/src/app/chat/new-session-tab.ts`, `apps/desktop/src/app/chat/close-tab.ts`, `apps/desktop/src/app/chat/session-tile.tsx`, and `apps/desktop/src/components/pane-shell/tree/store.ts`; verified current native strip and route state in `apps/hermes-native/src/app.rs`. Documentation diff, fixed-source pins and package validation checked; no implementation was claimed.
- **performance:** No runtime change or benchmark.
- **risks:** Dynamic pane support and background session event isolation touch shared workspace and gateway state; implement in dependency order with multi-profile tests before exposing capability.
- **follow_up:** Begin with canonical dynamic-pane identity/transaction support, then independent session view models before adding lifecycle controls.
### Revision 46 — canonical dynamic session-pane foundation

- **id:** HUR-001-058
- **timestamp:** 2026-11-15T05:21:18Z
- **what:** Added profile-scoped stable dynamic session-pane descriptors and revision-bound DoofusDock add, select, close, profile-preservation, and reopen behavior.
- **why:** Build session tiles in the canonical Dock layout before exposing them in the UI, avoiding a second tab/layout store.
- **components:** apps/hermes-native/src/layout.rs; DoofusDock LayoutDocument transactions; layout/profile persistence; architecture plan and parity backlog.
- **type:** layout foundation.
- **validation:** `cargo fmt --all -- --check`; `cargo check -p hermes-native --all-targets --features agent-automation`; `cargo test -p hermes-native --all-targets --features agent-automation` passed (65 library + 5 automation tests). New fixtures cover duplicate IDs across profiles, invalid/bounded identities, canonical tab selection, profile changes, close/reopen and persisted restore.
- **performance:** No benchmark; session pane IDs are fixed 16-digit FNV-1a hashes with full identity checked in metadata.
- **risks:** No UI controls or independent session content is wired; layout descriptors can persist after close because the pinned Dock API has no descriptor-removal operation.
- **follow_up:** Add independent per-pane chat view models and gateway event routing before exposing session panes to users.
### Revision 47 — profile-keyed session chat state and background stream routing

- **id:** HUR-001-059
- **timestamp:** 2026-11-15T05:34:17Z
- **what:** Added profile+stored-session keyed in-memory chat views, route-switch draft/transcript/runtime restoration, correlated resume hydration, and unique background message/tool event routing with a protected 32-view idle cache.
- **why:** Independent session state and background streaming must exist before dynamic DoofusDock panes can safely render multiple Hermes conversations.
- **components:** apps/hermes-native/src/app.rs; session route transitions; GatewayEvent::SessionResumed; server message/tool event reduction; diagnostics.
- **type:** chat state and event routing.
- **validation:** `cargo fmt --all -- --check`; `cargo check -p hermes-native --all-targets --features agent-automation`; full `cargo test -p hermes-native --all-targets --features agent-automation` passed (68 library + 5 automation tests). Added profile-collision route restore, ambiguous background runtime rejection, profile-specific background event and bounded-cache tests.
- **performance:** Cache target 32 idle session views; active, streaming, pending-page, submission, interrupt, approval and clarification state is protected from eviction. Cache is in-memory.
- **risks:** Views are not yet rendered inside DoofusDock panes; server-request ownership for hidden sessions is incomplete; this cache is lost on app restart.
- **follow_up:** Render each pane from its keyed view model, route all interactive RPC receipts by profile/session, and add open/create/close UI actions.

### Revision 48 — visible profile-scoped session panes

- **id:** HUR-001-060
- **timestamp:** 2026-11-15T06:06:37Z
- **what:** Render profile-scoped session transcripts/composers in dynamic Dock panes; open existing and blank sessions as canonical tiles with same-profile untouched-tile reuse; add session tile close controls with busy-session confirmation; route hidden submit receipts back to their cached view; add a headless session-pane fixture and screenshot.
- **why:** The prior layout and cache foundations did not expose usable session tabs or independent chat views in the actual workspace.
- **components:** apps/hermes-native/src/app.rs; apps/hermes-native/src/layout.rs; apps/hermes-native/src/bin/hermes-automation.rs; session tile rendering/open/create/close; agent screenshot fixture.
- **type:** user-visible chat workflow and Dock integration.
- **validation:** `cargo fmt --all`; `cargo check -p hermes-native --all-targets --features agent-automation`; `cargo test -p hermes-native --features agent-automation` passed (74 library + 5 automation tests). `cargo run -p hermes-native --features agent-automation --bin hermes-automation -- --session-pane-fixture --screenshot artifacts/agent-ui/session-pane-current-20261115.png` rendered a 1440x900 screenshot. Pinned research package validator passed: 565 behaviors, 565 UI inventory items, 565 pending tests and 14 source receipts; optional PyYAML parsing was skipped because PyYAML is not installed.
- **performance:** Adds one keyed view-state swap per visible session tile during each egui frame; no network calls occur during render.
- **risks:** Hidden approval/clarify request ownership, main-pane promote/reset semantics, reuse of edited blank drafts, resume error correlation, persistent scroll state and exact upstream tab lifecycle remain incomplete. Desktop OpenGL screenshots through Windows Graphics Capture appeared white while the shared headless renderer produced the expected dark UI.
- **follow_up:** Finish hidden request routing and source lifecycle edge cases; exercise blank creation and first-message visibility against an isolated gateway; capture paired source/target Windows behavior and continue all fixed-snapshot workflows.

## HUI-023

- **id:** HUI-023
- **timestamp:** 2026-11-15T06:12:48Z
- **what:** Bound approval and batch clarification prompts to their originating profile and runtime session; permit registered response actions from a hidden session pane and scope response/clarify-lock RPCs by the prompt owner.
- **why:** A visible background prompt was not actionable through agent control while another session was active, and response RPCs could inherit the wrong profile.
- **components:** Hermes Native prompt state, gateway request routing, semantic UI action handler, plan/backlog/architecture documentation.
- **type:** background session lifecycle and action-routing parity.
- **validation:** `cargo fmt --all -- --check`, `cargo check -p hermes-native --all-targets --features agent-automation`, and `cargo test -p hermes-native --features agent-automation` passed (76 library + 5 automation tests); package validator passed 565 target behaviors, 565 inventory items and 565 pending tests. Hidden-pane fixture confirms prompt controls and semantic action scope through the isolated no-writer boundary.
- **performance:** No added per-frame or network work; owner lookup scans the bounded in-memory session view cache.
- **risks:** Live authenticated gateway response receipts, cross-window ownership and runtime reconnect are not proven.
- **follow_up:** Continue the pinned session lifecycle gaps and then proceed through the remaining fixed-snapshot acceptance matrix.

## HUI-024

- **id:** HUI-024
- **timestamp:** 2026-11-15T06:23:40Z
- **what:** Implemented primary workspace close/promotion and a shared `Ctrl+W`/Command+W and agent `close_active_tab` path. The nearest valid session tile is promoted into primary with its profile-scoped cached view; otherwise an occupied primary route resets to New session. A selected busy tile still requires confirmation.
- **why:** The pinned Desktop makes its uncloseable workspace tab behave as a closeable chat context by promoting a neighboring session tile or returning to a fresh draft.
- **components:** Hermes Native Dock tab state, route/view restoration, UI keyboard handling, semantic action catalog, tests and parity ledger.
- **type:** tab lifecycle parity increment.
- **validation:** Formatting and all-target feature check passed; full feature test suite passed (79 library + 5 automation). App tests cover nearest profile-scoped promotion, fresh draft reset and busy tile confirmation; automation catalog verifies the action.
- **performance:** Close selection reads the current Dock snapshot and bounded descriptor/view cache; no network call is added.
- **risks:** Full focused-zone precedence, terminal/tool close behavior, secondary chat groups, physical keyboard/focus and Windows host acceptance remain open.
- **follow_up:** Continue remaining close commands/focus/overflow semantics, draft reuse and resume error correlation before broader UI routes.

## HUI-025

- **id:** HUI-025
- **timestamp:** 2026-11-15T06:29:00Z
- **what:** Expanded New session reuse to front a clean same-profile primary draft or a known idle, empty, unlisted, open session tile. Typed and edited drafts are preserved, and a pending create remains single-flight.
- **why:** The pinned Desktop prevents duplicate blank tabs while refusing to reuse a draft with unknown state, typed content, queued work or a different owner.
- **components:** Hermes Native New session handler, profile-scoped cache and Dock pane validation, isolated app fixtures, parity docs.
- **type:** session creation lifecycle parity.
- **validation:** Formatting and all-target feature check passed; full feature suite passed (81 library + 5 automation). Local cases cover primary reuse, typed primary preservation, tile reuse and edited tile separation.
- **performance:** Reuse checks inspect the existing in-memory view, session list and canonical Dock snapshot; no request is added for a reused draft.
- **risks:** Bot Mode owner routes, attachment-aware emptiness, live composer debounce flush and authenticated create behavior remain unverified or unsupported.
- **follow_up:** Continue close-other/right/all/focus precedence and session resume error correlation against the fixed source.

## HUI-026

- **id:** HUI-026
- **timestamp:** 2026-11-15T06:40:00Z
- **what:** Correlated each session resume request and result by request ID, stored session ID and profile. Matching snapshots hydrate only the owning view; stale results are ignored. Errors latch to the owning pane and expose an accessible Retry action; disconnect/reconnect permits recovery.
- **why:** A global resume error could be shown on the wrong chat, and an uncorrelated late response could affect the wrong tab after navigation or retry.
- **components:** Hermes Native gateway actor, app session-view cache, session-pane error state, AccessKit tests, agent automation fixture, parity docs.
- **type:** session lifecycle correctness and recovery.
- **validation:** cargo fmt --all -- --check passed after formatting; cargo check -p hermes-native --all-targets --features agent-automation passed; cargo test -p hermes-native --features agent-automation passed (83 library + 5 automation). The isolated 1440x900 pane screenshot was rendered and inspected; it shows the standard pane, while the error/retry path is covered by an app test. No live gateway or Windows host acceptance.
- **performance:** Resume result handling remains an in-memory keyed lookup plus bounded transcript state update; no benchmark was run because this does not claim a performance change.
- **risks:** Authoritative not-found/removal recovery, live gateway reconnect acceptance, source-paired screenshots, persistent scroll and complete tab parity remain open.
- **follow_up:** Continue remaining pinned-source session-tab behaviors and broader fixed-snapshot UI coverage without moving the Hermes source pin.

## HUI-027

- **id:** HUI-027
- **timestamp:** 2026-11-15T06:44:00Z
- **what:** Prioritized a latched resume failure over any cached transcript in the pane renderer, so Retry remains visible after a failed reopen even when older messages are cached.
- **why:** Cached history must not hide the source tile's failed-open state or its recovery action.
- **components:** Hermes Native session content renderer, profile-scoped resume error state, AccessKit fixture and parity plan.
- **type:** session resume error-state correctness.
- **validation:** cargo fmt --all -- --check and cargo check -p hermes-native --all-targets --features agent-automation passed; full cargo test -p hermes-native --features agent-automation passed (83 library + 5 automation). The error renderer test now includes cached history.
- **performance:** No new work beyond a keyed cached-view lookup; no performance claim or benchmark.
- **risks:** The test is isolated; authenticated gateway behavior and physical Windows rendering remain unverified.
- **follow_up:** Continue fixed-snapshot parity work and verify the source paired session lifecycle on the isolated live profile.

## HUI-028

- **id:** HUI-028
- **timestamp:** 2026-11-15T07:05:00Z
- **what:** Added hovered/focused Dock tab-group tracking for Ctrl+W and close_active_tab. Eligible session groups take precedence over workspace, followed by the workspace fallback. Closing a focused session in another Dock group uses the existing owner-aware close/confirmation path.
- **why:** The pinned Desktop targets the hovered/focused chat strip before the primary workspace; the native handler previously inspected only the workspace group.
- **components:** Hermes Native Dock group focus state, shared close handler, session pane transactions, app fixture, parity plan and architecture docs.
- **type:** session tab keyboard navigation parity.
- **validation:** cargo check -p hermes-native --all-targets --features agent-automation passed; focused close-target tests passed (2 tests). One initial compile found a moved PanelId and was fixed by cloning. Test setup first tried to center onto the hidden Files panel; it now reveals Files before the Dock transaction. The UI harness click did not reliably move focus in the headless fixture, so the close handler is tested with an explicit focused group and canonical Dock state; physical pointer/keyboard verification remains open.
- **performance:** Per-frame hit test checks the small set of visible Dock tab groups; no measurable performance claim or benchmark.
- **risks:** Egui focus traversal marker is not implemented; generic tool-tab closer/store synchronization is not implemented; physical Windows evidence is absent.
- **follow_up:** Add keyboard-focus ownership and tool-tab handoff, then verify hover/focus behavior on the isolated Windows app.

## HUI-029

- **id:** HUI-029
- **timestamp:** 2026-11-15T07:15:51Z
- **what:** Added source-aligned DoofusDock session-tab context actions and agent right-click input. Vendored the exact pinned egui renderer with MIT attribution, added a backward-compatible context-menu callback API, and wired Pin/Unpin, Close tab, Close others, Close tabs to the right, and Close all to the profile-aware Hermes handlers. Bulk closure preserves workspace and serializes active-pane confirmations.
- **why:** The fixed Hermes snapshot offers tab close and bulk-close controls, while the previously pinned renderer exposed only Pin/Unpin and the automation interface could not open context menus.
- **components:** Hermes Native Dock tab renderer, session tab close queue, agent UI protocol, source plan, architecture and operator documentation.
- **type:** session tab context-menu parity and agent pointer capability.
- **validation:** Focused protocol test passed for `actions.right_click` on the workspace tab and verified `Close all` appears through the real secondary-pointer path; direct app tests cover workspace preservation, session target ordering, duplicate filtering and serialized busy-close advancement. Full cargo suite, all-target build, package inventory and doc/log validation are recorded in the execution log.
- **performance:** Context menu adds no background work; bulk closure is a bounded in-memory queue. No performance benchmark is indicated.
- **risks:** Multi-session menu selection and busy-close queue behavior still need direct app-level tests; physical Windows input and paired fixed-source screenshots are not established.
- **follow_up:** Test context actions against multiple real session panes and busy queues, then continue the fixed-source UI/agent parity backlog without changing the Hermes snapshot.

## HUI-030

- **id:** HUI-030
- **timestamp:** 2026-11-15T07:25:47Z
- **what:** Added bounded modifier support to the agent `actions.keypress` method and exercised platform `COMMAND+W` through the canonical session-tab close handler.
- **why:** The human UI exposed keyboard shortcuts, but the Playwright-like agent could only generate unmodified key events.
- **components:** Hermes Native agent protocol, egui key event adapter, keyboard/tab close behavior, test fixture and protocol documentation.
- **type:** agent keyboard interaction parity.
- **validation:** Focused protocol test passes: an isolated profile-scoped session tile is opened, unsupported modifier input is rejected without advancing state, and `COMMAND+W` closes that exact pane through `HermesNativeApp::draw`. Full validation passed: cargo fmt --all -- --check, cargo check -p hermes-native --all-targets --features agent-automation, and cargo test -p hermes-native --features agent-automation (87 library + 6 automation tests); fixed-source package validator passed (565 behaviors, 565 inventory items, 565 pending tests, 14 source receipts).
- **performance:** Emits one bounded key combination and normal UI frame; no benchmark indicated.
- **risks:** This proves headless shared-handler behavior only; not full shortcut coverage or physical Windows keyboard/focus behavior.
- **follow_up:** Continue source-derived keyboard/navigation parity and add physical host evidence without changing the fixed Hermes source target.

## HUI-031

- **id:** HUI-031
- **timestamp:** 2026-11-15T07:38:33Z
- **what:** Implemented fixed-source session tab cycling shortcuts and repaired nested session-tab layouts. Added Ctrl+Tab/Ctrl+PageDown advance and Ctrl+Shift+Tab/Ctrl+PageUp reverse cycling with hover/focus/workspace targeting, wraparound, profile route restoration, and keyboard-focused tab group tracking. New session panes append to the workspace tab group; opening a session migrates legacy nested ancestor session tiles through canonical TabPanel operations.
- **why:** The pinned Desktop uses these shortcuts and expects session tiles to share one workspace tab strip; earlier AddPanel calls nested repeated tiles, preventing correct cycling.
- **components:** Hermes Native session pane layout, DoofusDock tab response, keyboard shortcut dispatch, modifier-aware agent protocol and source plan/docs.
- **type:** fixed-snapshot keyboard/tab layout parity and persisted layout repair.
- **validation:** Focused tests pass for forward/reverse cycling and page-key aliases via agent keypress, route identity, hover/focus/workspace targeting, wraparound, and migration from a simulated legacy nested layout. Full suite/build/source inventory checks are recorded in the execution log.
- **performance:** Shortcut resolves the small visible tab-group list and performs one canonical tab transaction; layout repair runs only when opening a session and an ancestor group contains a session tile. No benchmark indicated.
- **risks:** Recent-session-switcher fallback when no eligible strip exists, other source shortcuts and physical Windows keyboard/focus acceptance remain open.
- **follow_up:** Continue fixed-source shortcut and tab lifecycle parity; retain the downloaded Hermes commit as the target.
## HUI-032

- **id:** HUI-032
- **timestamp:** 2026-11-15T08:06:13Z
- **what:** Added the fixed-source recent-session fallback to Ctrl+Tab/Ctrl+PageDown and reverse aliases: quick activation, delayed top-center HUD, repeated-key highlight, Ctrl-release commit, Escape/click-away dismissal, profile filtering and accessible rows. Added observable switcher state plus a deterministic screenshot fixture and sidecar.
- **why:** The downloaded Desktop falls back to a recent-session switcher when the hover/focus/workspace ladder has no visible eligible strip; the native path previously returned without switching.
- **components:** Hermes Native keyboard routing, session navigation, recent-session HUD, agent state diagnostics, headless fixture and screenshot evidence.
- **type:** fixed-snapshot keyboard and agent-visible UI parity.
- **validation:** Focused recent-switcher tests pass for quick-cycle activation, profile scope, delayed reveal, repeated-chord selection/commit, and accessible row selection. The opt-in runner produced and visually reviewed the 1440x900 fixture screenshot. Full suite and source inventory validation are recorded in the final execution record.
- **performance:** The overlay snapshots the small active-profile recent-session list on open and requests repaint only for the 220 ms reveal deadline.
- **risks:** Native OS focus loss and physical Ctrl hold/release timing have not been accepted on Windows; screenshot is synthetic and offline, not source-paired.
- **follow_up:** Continue remaining fixed-snapshot routes and workflows; obtain physical keyboard/focus and paired visual evidence during Stage 5 acceptance.
## HUI-032 follow-up — Escape with Ctrl held

The first Escape regression found that egui’s modifier-exact consume call missed Escape while Ctrl was held. The handler now consumes Escape using the active modifier set. The regression passes, preserving the quick-cycle destination while dismissing the HUD; the full suite passes at 94 library and 7 automation tests.

## HUI-033

- **id:** HUI-033
- **timestamp:** 2026-11-15T08:23:18Z
- **what:** Added source-aligned public plugin catalog loading, JSON normalization/search data, single-worker background fetching, local search, selectable entry details, repository navigation, retry control, agent-visible catalog state, typed `refresh_plugin_catalog` action, and a deterministic offline fixture screenshot.
- **why:** Capabilities → Plugins was a placeholder even though the fixed Desktop exposes its public plugin catalog there.
- **components:** Hermes Native plugin catalog module, Capabilities Plugins renderer, registered app action, semantic automation fixture, architecture/backlog/plan documentation and screenshots.
- **type:** fixed-snapshot public catalog discovery parity slice.
- **validation:** Parser tests cover typed source fields, search text, display-name fallback, duplicate replacement, malformed response shape, and response size. Full Hermes Native suite and agent-automation build pass. Headless AccessKit inspection shows the search field, refresh action, selectable entries, detail fields and repository control; 1440×900 fixture screenshot was visually inspected. The published feed returned HTTP 200, 2,378 entries and 5,092,913 bytes in a separate live endpoint check. No installed plugin management or paired source screenshot was exercised.
- **performance:** The network request runs on a dedicated serial worker, has a 60-second deadline and returns a bounded 32 MiB response; UI search scans the loaded catalog in memory.
- **risks:** Feed contents are live because the pinned Desktop fetches the public catalog at runtime; this does not change the fixed Hermes source target. Plugin installation/security review, installed package state and catalog filter/sort/card parity remain incomplete. Synthetic fixture data is not real catalog content.
- **follow_up:** Implement the installed Desktop/Agent package inventory and source catalog browser interactions, then compare against the pinned source using matched fixtures and UI captures.

### HUI-033 visual follow-up

The first rendered fixture showed the Plugins section title and result count clipped against the Dock divider. Added a 4 px leading inset, preserved loaded entries during a failed refresh, kept refresh on the shared typed handler, and cleared selected details when search changes. Rebuilt and visually rechecked the 1440×900 fixture; all package checks passed again.

### HUI-033 live-feed and bounds follow-up

Live agent navigation exposed a CJK/Thai missing-glyph panic, an unbounded 2,378-row render, and an oversized diagnostic snapshot. Added Windows installed-script fallbacks with a non-panicking tofu policy, source-style 60-row paging, and bounded agent catalog summary state. Through the headless agent's typed route action, `ui.wait_for` observed the real catalog's Show more control in 604 ms; `state.get` then returned `plugin_catalog_entry_count=2378`, `plugin_catalog_visible_limit=60`, no error, and no gateway. The live screenshot is `artifacts/agent-ui/plugin-catalog-live-20261115.png`.

### HUI-033 screenshot evidence follow-up

Root cause for repeated-looking captures: every headless run creates a fresh isolated app, and `--screenshot` without explicit navigation or fixture setup captures the default New session route. Some code-only updates also had no visible change on that route. Documented the navigation/wait requirement and separated the synthetic catalog fixture from a real public-feed capture. The live image visibly differs and its sidecar records route, loaded count, wait receipt, and isolation boundary.

### HUI-034 — source Connectors directory read slice

Replaced the Connectors unavailable notice with a profile-bound read flow for the pinned Desktop's `connectors.list`, `connectors.catalog` and `connectors.accounts` RPCs. The serialized gateway actor returns typed connector/catalog/account records under a correlated event. The view now supports search, connection status and expanded account alias/status; refresh shares the human/agent app-action handler. Added a synthetic fixture for review, a WebSocket protocol test for method order/owner params/decoding and an AccessKit test for visible state/details. The 1440×900 capture is `artifacts/agent-ui/connectors-directory-20261115.png`.

This does not complete Connectors parity. Sign-in/connect, local MCP/plugin server reconciliation, tool/policy editors, hosted account operations, source card/dialog details, live profile behavior and source-paired Windows screenshots remain open.

### HUI-034 publication record

The validated Connectors directory slice was committed as `aab2176` and pushed to `origin/main`. The fetched remote was at the expected parent before publication; local `main` and `origin/main` now resolve to the same commit and the worktree is clean.

### HUI-034 source join follow-up

Aligned row inclusion with the pinned Desktop's `hostedSlugs` union: connectors returned by `connectors.list` remain visible even if `connectors.catalog` omits them. Added the fixed-source known display names plus slug-title fallback and account-status participation in the connection label. The offline fixture/test now includes a connected list-only `unlisted_service`; `connectors-directory-union-20261115.png` is a fresh distinct capture. Authentication and all connector mutation/detail workflows remain incomplete.

### HUI-034 source-join validation

The final union/status follow-up passed the full feature suite (98 library + 7 automation tests), all-target automation cargo check, rustfmt, diff check, and fixed-source validator (565/565 inventory rows and 14 source receipts). The revised 1440×900 capture shows the catalog/list union and was visually reviewed. No live Hermes profile or gateway was used.


### HUI-035 — source-aligned Connectors groups and search

Matched the pinned Desktop directory's hosted status derivation and display rules using the profile account/list/catalog snapshot: disabled connectors group as Off, unconnected rows as Available, and connection states as Connected-group rows ordered by source attention rank. The directory exposes All/Connected/Available/Off count controls when more than one status group exists, keeps only name/slug search matches, groups the result in source order, and reports matches hidden by a selected segment. The offline fixture/test covers active, list-only connected, available and disabled rows and confirms description-only text is not searchable. The distinct 1440x900 screenshot and hash-bound sidecar are `artifacts/agent-ui/connectors-grouped-20261115.png` and `.png.json`.

This remains a read-only presentation slice. Hosted/local card merging, connector connect/auth dialogs, MCP server management, tool/policy editors, account operations, live profile acceptance, and source-paired Windows visual comparison remain open.

### HUI-035 validation

Focused connector tests passed; the complete feature suite passed (99 library + 7 automation tests). Default and all-target feature checks, rustfmt, diff check, pinned package validator (565 in-scope/source rows; all still pending), and screenshot sidecar hash checks passed. PyYAML was unavailable, so its optional UCM YAML parse was skipped.


### HUI-035 publication

Published the validated Connectors grouping/search slice directly to `main` as `633b3f7` after fetching `origin/main` and confirming the expected parent `adb7dcd`. The journal follow-up is recorded in the next main commit.


### HUI-036 — installed Agent Plugin inventory and enable toggle

Added the pinned Desktop Plugins page's profile-scoped `plugins.manage` list/toggle slice. The native gateway actor decodes typed installed plugin and linked server rows; the page filters the source's hidden built-ins, displays profile inventory status/version/source/update metadata, and enables eligible rows through canonical-key toggles. The checkbox and typed `set_agent_plugin_enabled` app action share the same handler. Toggle results apply only when request ID, key, profile and returned plugin key match; restart/reload outcome is surfaced. A fixture, curation test, protocol test and 1440x900 screenshot are recorded at `artifacts/agent-ui/plugins-installed-profile-20261115.png` plus its hash-bound sidecar.

Desktop runtime plugin halves/settings, package merge, catalog filtering/install/update/uninstall dialogs, live profile validation, and source-paired Windows visual acceptance remain incomplete. No installed profile or live gateway was changed.

### HUI-036 validation

The full feature suite passed (103 library + 7 automation tests), default and all-target feature checks, rustfmt, pinned-source package validation, diff check, and screenshot sidecar hashes. All 565 acceptance inventory rows remain pending; optional UCM YAML parsing was skipped because PyYAML is unavailable.


HUI-036 profile-safety follow-up: added profile-bound receipts and disabled toggles for stale profile snapshots, invalidated/refetched inventory if a confirmed plugin row is unexpectedly absent, and separated toggle failures from list-load errors. The complete test/build/package validation set was rerun and passed; screenshot sidecar hashes were refreshed.


HUI-036 publication: pushed the validated installed Agent Plugin list/toggle slice to `main` as `6f51a8b` after verifying the expected parent `949ce83`. The publication record is committed separately.


### HUI-037 — confirmed installed Agent Plugin uninstall

Added the pinned Desktop's profile-scoped `plugins.manage` remove workflow for eligible user/git Agent Plugins. The inventory row opens an accessible destructive confirmation, shared with the registered agent action; confirmation revalidates connectivity, profile ownership, current inventory and removable source before issuing the RPC. Correlated success invalidates and reloads the inventory; errors preserve it, and a disconnect reports an unknown outcome. WebSocket contract, AccessKit confirmation and action-catalog tests pass. Package/Desktop-half reconciliation and source-paired live acceptance remain open.

Validation: all 105 library and 7 automation tests passed, default and all-target feature builds passed, `git diff --check` passed, and the pinned-source package validator passed (565 rows remain pending; optional PyYAML check skipped).


HUI-037 publication: commit `08e3eb5` was pushed to `main` after verifying the remote parent matched `c12cf58`. The publication record is committed separately.


### HUI-038 — merged plugin catalog and source facets

Merge installed Agent Plugins into their public catalog rows by `catalog_name`, including installed key/version/source/update metadata and search terms; append installed-only entries. Add source-normalized tiers, category/tag facets, installed/update filters, result counts, conditional Stars/Newest/Recently updated sorting, and clear/reset behavior. The UI selection and bounded diagnostics track filter state. Matching, unmatched-row, combined-facet and sort tests pass; an AccessKit interaction exercises tier filtering. A visually distinct, hash-bound 1440×900 screenshot is recorded at `artifacts/agent-ui/plugin-catalog-filters-20261115.png` with its sidecar.

Validation: all 107 library and 7 automation tests passed; default and all-target feature checks, rustfmt, diff check and pinned-source package validation passed. The package still reports 565 requirements pending; optional UCM YAML parsing was skipped because PyYAML is unavailable. Catalog fixture data is synthetic and live profile acceptance remains open.


HUI-038 publication: implementation commit `8c1996f` was pushed to `main` after verifying parent `cdde57e`; the separate publication log follows.

### HUI-039 — reviewed curated Agent Plugin install

Added an accessible install entry point to uninstalled catalog rows plus the shared `request_catalog_plugin_install` agent action. The native flow asks the selected Hermes profile to inspect the repository, renders its repository URL, full reviewed commit, Agent/Desktop component flags and warnings, and requires explicit confirmation. Installation submits the catalog name and inspected SHA with the same profile; insecure/non-Agent entries and pin mismatches fail closed. Correlated receipts expose activation, missing environment and warnings before refreshing the installed inventory. A timeout/disconnect leaves the result unknown, disables resubmission, and blocks reopening until a fresh profile inventory reconciles the outcome. Isolated WebSocket/AccessKit tests pass, and the review-state screenshot is separate from the former catalog/browser captures.

Validation: 109 library tests and 7 automation tests passed; formatter, default/all-target builds, `git diff --check`, and fixed-source package validator passed. The validator still reports all 565 inventory rows pending; optional PyYAML UCM parsing was skipped. No Hermes gateway/profile was used for install validation. The synthetic review screenshot is 1440×900 and visually inspected. Desktop-half installation, scan-finding force review, update/custom-repository flows, and live profile/source-paired acceptance remain open.

Change record: id `HUI-039`; type `feature`; components `Plugins catalog, gateway actor, agent action registry, headless fixture`; performance `no benchmarked runtime change`; risks `backend install runs up to a minute and disconnect/timeout result is uncertain`; follow-up `implement scan finding consent and remaining package/update workflows against pinned Desktop source`.

HUI-039 publication: implementation commit `c9cd13d` was pushed to `main` after fetching and confirming its parent `6ff7dc0`; this docs-only commit records publication.

### HUI-040 — Agent Plugin update consent and scan review

Added profile-scoped Agent Plugin updates to installed eligible rows and the shared agent-action registry. The flow retains accepted capability consent across subsequent Hermes scan gates, sorts and bounds displayed findings, permits an explicit force retry only for caution verdicts, and leaves dangerous verdicts non-overridable. Correlated applied updates refresh the profile inventory. Isolated WebSocket and AccessKit tests cover the consent-to-caution-to-override sequence and dangerous review state. Desktop plugin-half reconciliation, uncertain live-update resolution, update-all and live/source-paired acceptance remain open.

Validation: 111 library tests and 7 automation tests passed; formatting, default/all-target builds, fixed-source package validation and diff check passed. The validator still reports all 565 inventory rows pending; optional PyYAML parsing was skipped. A synthetic caution review screenshot was visually inspected; no Hermes gateway/profile was used. No runtime performance benchmark was taken.

Change record: id `HUI-040`; type `feature`; components `Plugins catalog, gateway actor, agent action registry, headless fixture`; performance `no benchmarked runtime change`; risks `backend update may produce an unknown outcome after disconnect and requires fresh inventory reconciliation`; follow-up `implement Desktop-half reconciliation and uncertain update handling, then pursue remaining pinned-source workflows and paired acceptance`.

HUI-040 publication: implementation commit `b6e3686` was pushed to `main` after verifying its parent matched `origin/main`; publication evidence is appended separately.

### HUI-041 — profile-scoped automatic Agent Plugin updates

Added the pinned Desktop plugins.manage auto_update read/write control to the Plugins inventory. The accessible checkbox appears only when the backend supports the operation, uses the selected profile, and shows the backend-provided check interval. Shared UI and agent action handling validates profile state, correlates request IDs, disables writes offline or while a write is pending, preserves the previous setting on write errors, and clears scoped state on disconnect. Unsupported backends keep the control hidden.

Validation: 112 library tests and 7 automation tests passed; formatting, default/all-target builds, fixed-source package validation and diff check passed. The 565-row research inventory remains pending and optional PyYAML parsing was skipped. The 1440x900 synthetic auto-update UI screenshot was visually inspected; no Hermes gateway/profile was used. No runtime performance benchmark was taken.

Change record: id HUI-041; type feature; components Plugins inventory, gateway actor, shared UI action registry, headless fixture; performance no benchmarked runtime change; risks setting is unavailable on backend versions without plugins.manage auto_update; follow-up implement update-all, desktop/package reconciliation, remaining pinned-source workflows and source-paired acceptance.

Validation correction for HUI-041: after adding the interval parser boundary test, the final complete suite passed 113 library tests and 7 automation tests. The default and all-target checks, package validation, screenshot and sidecar validation also passed.

HUI-041 publication: implementation commit 88206d5 was pushed to main after fetching and confirming its parent matched origin/main; publication evidence is appended separately.

- **id:** HUI-042
- **timestamp:** 2026-11-15T11:19:42Z
- **what:** Added fixed-source profile-scoped Agent Plugin Update all, a registered UI/agent action, typed outcome summary and a distinct expanded synthetic screenshot fixture.
- **why:** Complete the pinned Plugins workflow and provide evidence that visibly differs from prior captures of the same static Plugins state.
- **components:** apps/hermes-native/src/app.rs, apps/hermes-native/src/gateway.rs, apps/hermes-native/src/bin/hermes-automation.rs, apps/hermes-native/src/bin/hermes_automation/automation_protocol.rs, apps/hermes-native/artifacts/agent-ui/plugin-update-all-report-20261115.png, apps/hermes-native/docs/PLAN.md, apps/hermes-native/docs/BACKLOG.md, apps/hermes-native/docs/ARCHITECTURE.md, apps/hermes-native/README.md.
- **type:** source-parity feature; agent UI fixture.
- **validation:** cargo fmt --all -- --check; cargo test -p hermes-native --features agent-automation (114 library + 7 automation tests); screenshot captured at 1440x900 and visually inspected with all three non-current result types expanded; profile-scoped WebSocket fixture and offline action coverage passed.
- **performance:** No benchmark run. The 600-second RPC uses the serialized WebSocket actor and blocks other WebSocket commands; this remains a known limitation.
- **risks:** Gateway RPC congestion during update-all; synthetic screenshot is not live Hermes acceptance; full pinned-source parity remains open.
- **follow_up:** Add RPC multiplexing or a dedicated authenticated socket, add live profile/source-paired validation, then continue remaining 499 in-scope acceptance rows.
- **id:** HUI-043
- **timestamp:** 2026-11-15T11:33:09Z
- **what:** Moved profile-scoped Agent Plugin Update all onto a dedicated authenticated WebSocket worker; primary gateway RPCs continue while update-all is pending. Added a concurrency fixture and recorded local latency evidence.
- **why:** The previous implementation occupied the serialized gateway actor for as long as ten minutes, preventing unrelated Hermes UI work from completing.
- **components:** apps/hermes-native/src/gateway.rs, apps/hermes-native/docs/PLAN.md, apps/hermes-native/docs/BACKLOG.md, apps/hermes-native/docs/ARCHITECTURE.md, apps/hermes-native/artifacts/agent-ui/plugin-update-all-report-20261115.png.json, docs/ARCHITECTURE.md, docs/BENCHMARKS.md.
- **type:** gateway concurrency and validation.
- **validation:** cargo fmt --all -- --check; full feature suite passed (115 library and 7 automation tests); default and all-target checks passed; fixed-source package validator passed 565 inventory rows (all remain pending); long-RPC concurrency fixture passed while the list RPC completed before update-all release.
- **performance:** Local loopback fixture measured primary plugin-list latency 64.323 ms while update-all was held; update RPC completed in 0.325 ms after release. See docs/BENCHMARKS.md; this is not a live Hermes timing or resource benchmark.
- **risks:** Separate authenticated socket handshake and live endpoint loss/recovery still need installed-Hermes validation; source-paired visual acceptance remains open.
- **follow_up:** Exercise update-all against a live profile in a controlled isolated Hermes profile, validate cancellation/disconnect recovery, continue all remaining in-scope UI ledger rows.
- **id:** HUI-044
- **timestamp:** 2026-11-15T11:36:21Z
- **what:** Advanced the active implementation plan header to revision 72 and added the explicit HUI-043 concurrency revision.
- **why:** Keep the authoritative plan revision synchronized with the verified gateway-worker implementation already published.
- **components:** apps/hermes-native/docs/PLAN.md, apps/hermes-native/docs/CHANGELOG.agent.md, apps/hermes-native/docs/EXECUTION_LOGS.md, docs/CHANGELOG.agent.md, docs/EXECUTION_LOGS.md.
- **type:** plan traceability correction.
- **validation:** Confirmed revision 71 describes update-all and revision 72 describes its isolated authenticated worker; git diff --check passed.
- **performance:** No runtime behavior changed; the HUI-043 concurrency benchmark remains in docs/BENCHMARKS.md.
- **risks:** None beyond the recorded live-gateway acceptance gaps.
- **follow_up:** Continue implementation against the fixed 499-row Hermes Desktop scope; do not mark acceptance from local fixtures alone.
- **id:** HUI-045
- **timestamp:** 2026-11-15T11:56:16Z
- **what:** Added bounded profile-scoped LIFO restore of closed session tabs, active-route handoff, native shortcut, and shared UI-agent action.
- **why:** Match the pinned Hermes Desktop `view.reopenTab` behavior through canonical DoofusDock state.
- **components:** apps/hermes-native/src/app.rs, apps/hermes-native/src/layout.rs, apps/hermes-native/src/bin/hermes_automation/automation_protocol.rs, docs/ARCHITECTURE.md, docs/BACKLOG.md.
- **type:** source-parity behavior.
- **validation:** `cargo test -p hermes-native --features agent-automation`: 117 library tests and 8 automation tests passed; source-paired Windows visual acceptance remains open.
- **performance:** Bounded stack retains at most 64 closed session panes per profile; no network or gateway work added.
- **risks:** Stack is in memory and does not survive app restart, matching the pinned source. Live installed-Hermes acceptance remains open.
- **follow_up:** Continue the fixed 499-row source-paired acceptance scope; do not claim full replacement parity.
- **id:** HUI-046
- **timestamp:** 2026-11-15T12:12:41Z
- **what:** Added native transcript find-in-page with accessible query/count controls, keyboard stepping, message-card highlight, and scroll-to-match.
- **why:** Close fixed-source chat requirement PAR-0058 and provide a usable transcript navigation path for both people and the UI agent.
- **components:** apps/hermes-native/src/app.rs, apps/hermes-native/docs/PLAN.md, apps/hermes-native/docs/AGENT_AUTOMATION_PROTOCOL.md, docs/ARCHITECTURE.md, docs/BACKLOG.md.
- **type:** source-parity behavior slice.
- **validation:** `cargo fmt --all -- --check`; `cargo test -p hermes-native --features agent-automation` (118 library + 8 automation tests); `cargo check -p hermes-native --all-targets --features agent-automation`.
- **performance:** Search scans the currently loaded transcript in memory; no gateway requests or background workers are added.
- **risks:** Search is limited to loaded transcript messages and highlights whole message cards. Exact rendered-text highlighting, non-transcript panes, and source-paired Windows acceptance remain open; PAR-0058 is not accepted.
- **follow_up:** Continue fixed-snapshot chat parity and source-paired UI acceptance; preserve the 499-row gate.
- **id:** HUI-047
- **timestamp:** 2026-11-15T12:15:14Z
- **what:** Added an isolated transcript-find visual fixture and saved a distinct 1440×900 search-state capture.
- **why:** Make the new interaction reviewable and give the automation runner a reproducible non-default screen for visual checks.
- **components:** apps/hermes-native/src/app.rs, apps/hermes-native/src/bin/hermes-automation.rs, artifacts/agent-ui/transcript-find-in-page-20261115.png, apps/hermes-native/docs/PLAN.md, docs/ARCHITECTURE.md.
- **type:** automation fixture and visual evidence.
- **validation:** One-shot runner rendered the fixture at 1440×900; screenshot visually inspected. `cargo fmt --all -- --check`, 118 library tests, 8 automation tests, and all-target feature compile passed.
- **performance:** Fixture is local and offline; no gateway or installed profile is opened.
- **risks:** The image demonstrates a synthetic native state and is not source-paired parity acceptance.
- **follow_up:** Continue implementing and visually validating the fixed Hermes snapshot; keep PAR-0058 unaccepted until source-paired checks pass.
- **id:** HUI-048
- **timestamp:** 2026-11-15
- **what:** Added off-by-default local transcript timestamps, a window-scoped `/timestamps` command, message lifecycle boundaries, an offline visual fixture, and a distinct screenshot.
- **why:** Implement pinned Hermes Desktop `display.timestamps` behavior as the next P0 chat parity slice while keeping the command out of model submissions.
- **components:** `apps/hermes-native/src/app.rs`, `apps/hermes-native/src/bin/hermes-automation.rs`, `apps/hermes-native/Cargo.toml`, `apps/hermes-native/Cargo.lock`, `artifacts/agent-ui/transcript-timestamps-20261115.png`, `README.md`, `docs/BACKLOG.md`, `docs/ARCHITECTURE.md`, `apps/hermes-native/README.md`, `apps/hermes-native/docs/PLAN.md`, parity evidence maps.
- **type:** source-parity implementation slice.
- **validation:** `cargo fmt --all`; `cargo test -p hermes-native --features agent-automation` (121 library + 8 automation tests); `cargo check -p hermes-native --all-targets --features agent-automation`; isolated 1440×900 fixture rendered and inspected.
- **performance:** Per-message local formatting only when timestamps are enabled; no gateway requests, network activity, or benchmarked performance change.
- **risks:** `display.timestamps` config initialization, source locale/date tooltip formatting, tool-group timestamps, live Hermes behavior, and source-paired Windows acceptance remain open. PAR-0065 stays pending.
- **follow_up:** Read the pinned config value, match the source timestamp tooltip and tool-run lifecycle, then validate with paired source/target Windows captures before accepting PAR-0065.
- **id:** HUI-049
- **timestamp:** 2026-11-15
- **what:** Recorded PAR-0065 as partial in the fixed-snapshot inventory and crosswalk, added implementation evidence EV-0016, and taught the package validator to distinguish implementation receipts from source inspections.
- **why:** Keep the user's authoritative parity ledger aligned with shipped behavior without confusing a passing local fixture with source-paired acceptance.
- **components:** `docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/maps/`, its evidence registry, and `tools/validate_package.py`.
- **type:** parity evidence and validator correction.
- **validation:** Package validator passes 565 target behaviors, 565 inventory rows and 565 pending tests; counts 14 source inspection receipts and 1 implementation receipt. PAR-0065 remains pending.
- **performance:** No runtime behavior changed.
- **risks:** The optional PyYAML parse is unavailable in this Python environment; source-paired acceptance remains open.
- **follow_up:** Continue filling the fixed 499-row stock Desktop scope and preserve strict source-paired acceptance gates.

HUI-048/HUI-049 publication: implementation commit `4d247d7ccbee6f31604923e52fc37a799100aa3d` and evidence/validator commit `27504e9c55f31a6cdcc8e47ee075ba712ca87dac` were pushed to `origin/main`. PAR-0065 remains pending.

## HUI-050 — 2026-11-15

- **id:** HUI-050
- **timestamp:** 2026-11-15
- **what:** Load only the selected Hermes profile's `display.timestamps` value from authenticated `/api/config`; preserve a window-local `/timestamps` override across late config responses.
- **why:** Complete the pinned-source config-default behavior without allowing unrelated Hermes configuration into the native UI.
- **components:** `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, timestamp parity docs.
- **type:** source-parity implementation slice.
- **validation:** `cargo fmt --all -- --check`; `cargo test -p hermes-native --features agent-automation` (123 library + 8 automation tests); `cargo check -p hermes-native --all-targets --features agent-automation`; fixed-source package validator (565 target behaviors, 565 inventory items, 565 pending tests); `git diff --check`.
- **performance:** One bounded config read on connect/profile change; no benchmarked performance change.
- **risks:** Exact timestamp locale/tooltips and tool-run lifecycle behavior, live-profile behavior, and source-paired Windows acceptance remain open; PAR-0065 remains pending. Optional PyYAML validation is unavailable.
- **follow_up:** Implement remaining timestamp details and capture matched source/target Windows states before accepting PAR-0065.

## HUI-051 — 2026-11-15

- **id:** HUI-051
- **timestamp:** 2026-11-15
- **what:** Added EV-0017 for the profile-scoped timestamp config read and advanced the active plan's evidence revision.
- **why:** Bind the implementation receipt to the committed source while preserving partial capability status and pending formal acceptance.
- **components:** Fixed-snapshot capability evidence, `capabilities.yaml`, Hermes Native plan.
- **type:** parity evidence update.
- **validation:** Fixed-source package validator passed with all 565 acceptance rows still pending.
- **performance:** No runtime behavior changed.
- **risks:** PyYAML is unavailable; optional YAML parse skipped. PAR-0065 remains pending.
- **follow_up:** Finish timestamp format/tool-run parity and matched Windows evidence.

HUI-050/HUI-051 publication: `cdeffe3b2e2dac737b8c463aea01d19bd73a866c` (implementation) and `86eacb86ccd159c55221ae4667586504c2b38d64` (evidence) are published on `origin/main`. PAR-0065 remains pending.

## HUI-052 — 2026-11-15

- **id:** HUI-052
- **timestamp:** 2026-11-15
- **what:** Added bounded semantic scrolling to the headless agent UI protocol using real egui mouse-wheel input at an accessible target.
- **why:** Let background agents navigate scrollable content through the same pointer input path a person uses.
- **components:** automation protocol, agent automation docs, README, plan and crosswalk.
- **type:** agent UI control capability.
- **validation:** Focused protocol tests prove content movement in an egui ScrollArea and rejection of zero/oversized deltas; full automation-feature suite and all-target check are pending.
- **performance:** One pointer event and one bounded UI frame per scroll request.
- **risks:** This is headless semantic control only; no authenticated bridge, physical window control, or fixed-source visual acceptance.
- **follow_up:** Validate the complete protocol suite and extend shared native-window control without bypassing normal UI actions.

## HUI-053 — 2026-11-15

- **id:** HUI-053
- **timestamp:** 2026-11-15
- **what:** Completed full validation for semantic agent scrolling.
- **why:** Verify the new action in the complete automation feature build, not only its focused tests.
- **components:** Hermes Native agent protocol and automation test suite.
- **type:** validation follow-up.
- **validation:** `cargo fmt --all -- --check`; `cargo test -p hermes-native --features agent-automation` passed 123 library + 10 automation tests; `cargo check -p hermes-native --all-targets --features agent-automation` passed; `git diff --check` passed.
- **performance:** One bounded wheel event and one frame per action; no performance benchmark required.
- **risks:** Headless semantic evidence does not prove native Windows wheel-device or DPI behavior.
- **follow_up:** Continue app-wide route workflows and physical host parity.

HUI-052/HUI-053 publication: `688b6cae39d84e2739df5b23fdc2f33f468e6047` is published on `origin/main`; 123 library and 10 automation tests pass.

## HUI-054 — 2026-11-15

- **id:** HUI-054
- **timestamp:** 2026-11-15
- **what:** Added generation-fenced semantic `actions.fill` for exact replacement or clearing of accessible text fields using AccessKit `SetValue`.
- **why:** Give background agents a Playwright-style exact field operation without requiring pointer positioning or a fragile select-all sequence.
- **components:** Hermes Native automation protocol, AccessKit UI input path, protocol docs and acceptance plan.
- **type:** agent UI control capability.
- **validation:** Focused test passes for initial fill, replacement and empty clear. Full automation suite, all-target build, formatting and diff checks are in progress.
- **performance:** One AccessKit action and one bounded UI frame per fill.
- **risks:** Headless semantics do not prove Windows UI Automation behavior across native text controls or IME composition.
- **follow_up:** Run full validation and extend native app workflows against the pinned source.

## HUI-055 — 2026-11-15

- **id:** HUI-055
- **timestamp:** 2026-11-15
- **what:** Completed full validation for semantic exact-value fill.
- **why:** Verify field replacement in the complete automation build, beyond the focused regression.
- **components:** Hermes Native agent UI protocol and feature tests.
- **type:** validation follow-up.
- **validation:** `cargo fmt --all -- --check`; `cargo test -p hermes-native --features agent-automation` passed 123 library + 11 automation tests; `cargo check -p hermes-native --all-targets --features agent-automation` passed; `git diff --check` passed.
- **performance:** One AccessKit action and one UI frame per fill.
- **risks:** Physical Windows UI Automation and IME acceptance remain open.
- **follow_up:** Continue pinned-source workflow parity and native host evidence.

HUI-054/HUI-055 publication: `13d5ff4d6d26516ecd671dba21eb1bd7bbb86afd` is published on `origin/main`; 123 library and 11 automation tests pass.

- **id:** HUR-001-082
  **timestamp:** 2026-11-15T13:21:08Z
  **what:** Replaced the Files panel's empty state with a session-cwd-rooted, lazy, asynchronous, read-only local directory tree; added folder sorting, source-derived transport/build exclusions, refresh/retry controls, profile-root selection, and generation fencing; preserved session `cwd` from the pinned `/api/sessions` response.
  **why:** Make one of the largest visible placeholder panes perform its first real source-aligned workflow without changing Hermes files or blocking the UI thread.
  **components:** `src/files.rs`, `src/app.rs`, `src/gateway.rs`, `src/lib.rs`, `README.md`, `docs/ARCHITECTURE.md`, `docs/BACKLOG.md`, `docs/PLAN.md`.
  **type:** feature
  **validation:** `cargo fmt --all -- --check`; `cargo check -p hermes-native --all-targets --features agent-automation`; `cargo test -p hermes-native --lib --features agent-automation` (125 passed). Added filesystem sorting/exclusion, asynchronous root-change fencing, and session-list `cwd` decoding coverage. The formal 499-row acceptance matrix remains pending for source-paired evidence.
  **performance:** Directory enumeration is off the egui frame; each directory is capped at 1,000 entries. No benchmark performed or claimed.
  **risks:** Remote filesystem access, `.gitignore`/show-ignored, missing-root fallback, file preview panes/context actions, and source-paired Windows acceptance are still missing. Local session cwd is currently read directly by this native process.
  **follow_up:** Add remote/local filesystem adapter parity, `.gitignore` and ignored-file preferences, preview-pane lifecycle, and source-matched Windows acceptance before closing any Files requirements.

- **id:** HUR-001-083
  **timestamp:** 2026-11-15T13:26:22Z
  **what:** Completed the full automation-feature build, library/protocol test suite, formatting check, and whitespace validation for the Files slice.
  **why:** Validate the integrated native UI and agent-control targets after adding accessible Files tree rows and session-root plumbing.
  **components:** `src/files.rs`, `src/app.rs`, `src/gateway.rs`, `docs/AGENT_CONTROL_CROSSWALK.md`.
  **type:** validation
  **validation:** `cargo fmt --all -- --check` passed; `cargo test -p hermes-native --all-targets --features agent-automation` passed (125 library tests, 11 automation protocol tests); `cargo check -p hermes-native --all-targets --features agent-automation` passed; `git diff --check` passed.
  **performance:** No benchmark performed.
  **risks:** Passing local fixtures does not establish visual parity, remote Files behavior, or Windows host acceptance.
  **follow_up:** Continue with filesystem source behavior and dynamic preview panes; keep Files acceptance pending until paired runtime evidence exists.

- **id:** HUR-001-084
  **timestamp:** 2026-11-15T13:27:24Z
  **what:** Published the validated Files tree implementation to the configured `origin/main` repository.
  **why:** Keep approved parity work delivered in small reviewable increments on the user's chosen repository.
  **components:** `Doofus-Dock-Hermes` repository, `main` branch.
  **type:** publication
  **validation:** Commit `4edd2bf` pushed; remote accepted `9896f8f..4edd2bf`.
  **performance:** Not applicable.
  **risks:** Publication does not close the 1:1 parity objective; Files remote/preview/ignore behavior and all source-paired acceptance remain open.
  **follow_up:** Continue the fixed-source parity backlog from the new published `main` revision.

- **id:** HUR-001-085
  **timestamp:** 2026-11-15T13:38:40Z
  **what:** Added dynamic source-inspired file preview panes to the pinned-source Files panel.
  **why:** Complete the next Files workflow slice while keeping previews temporary and bounded.
  **components:** src/files.rs, src/layout.rs, src/app.rs, README, architecture, backlog, plan.
  **type:** feature
  **validation:** Formatting passed; automation-feature tests passed (128 library and 11 protocol tests); all-target feature check and git diff check passed. Tests cover UTF-8 preview, binary/size rejection, tab grouping and persistence omission.
  **performance:** Preview filesystem reads are isolated on a worker and capped at 1 MiB each; no benchmark performed.
  **risks:** Local-only files, source-paired rendering, remote filesystem, gitignore, richer previews and physical Windows host acceptance remain open.
  **follow_up:** Continue pinned-source parity; capture only after navigating to the exact route/fixture and keep the acceptance matrix pending until source-paired evidence is reviewed.

- **id:** HUR-001-086
  **timestamp:** 2026-11-15T13:38:40Z
  **what:** Recorded the verified full automation-feature target matrix for dynamic file previews.
  **why:** Preserve reproducible evidence for the current implementation increment.
  **components:** src/files.rs, src/layout.rs, src/app.rs, docs/EXECUTION_LOGS.md.
  **type:** validation
  **validation:** cargo fmt --all -- --check, cargo test -p hermes-native --all-targets --features agent-automation (128 + 11 passed), cargo check -p hermes-native --all-targets --features agent-automation, and git diff --check all passed.
  **performance:** No benchmark performed.
  **risks:** Local automated coverage does not establish visual parity or physical Windows acceptance.
  **follow_up:** Source-pair the Files tree and previews against the fixed snapshot before closing those matrix rows.

- **id:** HUR-001-087
  **timestamp:** 2026-11-15T13:42:30Z
  **what:** Published the reviewed transient Files preview slice to origin/main.
  **why:** Deliver approved parity work in small, validated increments on the selected repository.
  **components:** Doofus-Dock-Hermes repository, main branch.
  **type:** publication
  **validation:** Commit 121b486 pushed successfully; full feature test/check and diff validation completed before commit.
  **performance:** Not applicable.
  **risks:** Publication does not establish source-paired visual parity or complete Files behavior.
  **follow_up:** Continue from the fixed source pin; navigate to target states before capturing screenshots.

- **id:** HUR-001-088
  **timestamp:** 2026-11-15T13:48:48Z
  **what:** Added a navigated Files-preview headless fixture so agent captures can show an actual non-default route.
  **why:** Repeated screenshots happened because every one-shot run starts a new app at New session; the capture runner needs an explicit state fixture for pane-level review.
  **components:** src/app.rs, src/bin/hermes-automation.rs, README, docs/AGENT_AUTOMATION.md, docs/ARCHITECTURE.md, docs/BACKLOG.md, docs/PLAN.md.
  **type:** feature
  **validation:** CLI inspection found fixture session, Files rows, preview content/title and close control; 1440x900 screenshot visually shows the separate preview and Files pane. The screen is native-target-only evidence.
  **performance:** Fixture waits at most two seconds for its asynchronous worker reads; no benchmark performed.
  **risks:** Screenshot does not prove Hermes source parity or Windows native-host behavior.
  **follow_up:** Use explicit navigation/fixtures for every pane capture and compare against the same pinned Hermes scenario before accepting a matrix row.

- **id:** HUR-001-089
  **timestamp:** 2026-11-15T13:48:48Z
  **what:** Validated the navigated Files-preview fixture and feature-enabled build.
  **why:** Verify the new screenshot path and preserve full agent automation coverage.
  **components:** src/app.rs, src/bin/hermes-automation.rs, docs/EXECUTION_LOGS.md.
  **type:** validation
  **validation:** cargo fmt --all -- --check, cargo test -p hermes-native --all-targets --features agent-automation (128 library + 11 protocol), cargo check -p hermes-native --all-targets --features agent-automation and git diff --check passed.
  **performance:** No benchmark performed.
  **risks:** Automated target fixture is not source-paired visual acceptance.
  **follow_up:** Continue source-based parity implementation and record distinct scenario receipts.

- **id:** HUR-001-090
  **timestamp:** 2026-11-15T13:53:44Z
  **what:** Matched the pinned file-preview text threshold and Markdown default more closely.
  **why:** The downloaded source defaults to rendered Markdown and a 512 KiB read ceiling with an explicit Preview anyway action.
  **components:** src/files.rs, src/app.rs, README, docs/ARCHITECTURE.md, docs/BACKLOG.md, docs/PLAN.md.
  **type:** fidelity
  **validation:** Tests cover normal UTF-8 text, default binary/oversized refusal, explicit lossy binary preview and forced oversized truncation. Full automation-feature suite and check passed.
  **performance:** Worker reads are bounded to 512 KiB plus a one-byte limit sentinel; no benchmark performed.
  **risks:** Image, PDF, rendered HTML, diff/edit mode, source styling and source-paired Windows evidence remain open.
  **follow_up:** Continue the fixed-source preview variants and compare the native fixture to the same pinned source scenario.

- **id:** HUR-001-091
  **timestamp:** 2026-11-15T13:53:44Z
  **what:** Recorded full feature-enabled validation for source-aligned Markdown and bounded preview behavior.
  **why:** Confirm the preview fidelity changes preserve the existing app and agent harness.
  **components:** src/files.rs, src/app.rs, docs/EXECUTION_LOGS.md.
  **type:** validation
  **validation:** cargo fmt --all -- --check, cargo test -p hermes-native --all-targets --features agent-automation (128 + 11 passed), cargo check -p hermes-native --all-targets --features agent-automation and git diff --check passed.
  **performance:** No benchmark performed.
  **risks:** Unit/protocol validation does not prove visual parity.
  **follow_up:** Capture the updated fixture and keep all Files acceptance IDs pending pending review.

- **id:** HUR-001-092
  **timestamp:** 2026-11-15T13:55:37Z
  **what:** Removed a per-frame clone of cached file preview content.
  **why:** The source-sized preview buffer should not be copied for every egui render pass.
  **components:** src/files.rs.
  **type:** performance
  **validation:** Full automation-feature matrix passed: formatting, 128 library tests, 11 protocol tests, all-target check and diff check.
  **performance:** Avoids copying up to the 512 KiB preview buffer each frame; no benchmark performed.
  **risks:** None identified; operation stays within the bounded content store.
  **follow_up:** Measure only if later preview workloads show a material rendering bottleneck.

- **id:** HUR-001-093
  **timestamp:** 2026-11-15T13:55:37Z
  **what:** Recorded complete validation for the borrow-only preview rendering change.
  **why:** Confirm the optimization preserves interactive and worker behavior.
  **components:** src/files.rs, docs/EXECUTION_LOGS.md.
  **type:** validation
  **validation:** Formatting, 128 library tests, 11 protocol tests, all-target feature check and git diff check passed.
  **performance:** No benchmark performed.
  **risks:** Test evidence does not establish visual parity.
  **follow_up:** Keep the preview slice open for exact source comparisons and acceptance tests.

- **id:** HUR-001-094
  **timestamp:** 2026-11-15T13:58:30Z
  **what:** Improved the Files preview and root labels to remain readable in the 1440×900 target state.
  **why:** Visual review showed the temporary absolute path clipped the preview and project header.
  **components:** src/app.rs, src/files.rs, docs/ARCHITECTURE.md, docs/PLAN.md.
  **type:** fidelity
  **validation:** Feature-enabled full test/check matrix passed; the navigated preview fixture produced and visually rendered a distinct 1440×900 image.
  **performance:** No benchmark performed.
  **risks:** Target screenshot is synthetic and not source-paired.
  **follow_up:** Bind the committed PNG sidecar to the final commit and keep source parity open.

- **id:** HUR-001-095
  **timestamp:** 2026-11-15T13:58:30Z
  **what:** Validated the readable Files-preview screenshot fixture after the UI refinement.
  **why:** Confirm the screenshot reflects the real preview route instead of the default New session screen.
  **components:** src/app.rs, src/files.rs, docs/EXECUTION_LOGS.md.
  **type:** validation
  **validation:** Formatting, 128 library tests, 11 protocol tests, all-target check and whitespace validation passed. The explicit fixture rendered a distinct screenshot whose AccessKit tree contains preview and Files controls.
  **performance:** No benchmark performed.
  **risks:** Screenshot proves the native fixture only, not pinned-source parity or physical Windows host behavior.
  **follow_up:** Use the sidecar and exact scenario when collecting source-paired evidence.

- **id:** HUR-001-096
  **timestamp:** 2026-11-15T13:59:30Z
  **what:** Captured a distinct Files-tree plus Markdown-preview target screen with exact build/source metadata.
  **why:** Each agent process is fresh; explicit fixture navigation is required to capture meaningful pane state.
  **components:** artifacts/agent-ui/files-preview-v1.png, artifacts/agent-ui/files-preview-v1.json, docs/EXECUTION_LOGS.md.
  **type:** evidence
  **validation:** AccessKit tree contained the session, file rows, README preview content and close control; PNG visually inspected at 1440×900; sidecar binds source SHA, target commit and binary/image hashes.
  **performance:** No benchmark performed.
  **risks:** This is target-only synthetic evidence; no paired Hermes source capture or physical Windows host result is implied.
  **follow_up:** Collect a matching source screen and record a reviewer decision before changing any Files capability status.

- **id:** HUR-001-097
  **timestamp:** 2026-11-15T13:59:30Z
  **what:** Prepared the source-bound screenshot and sidecar for repository publication.
  **why:** Preserve the distinct visual review artifact beside its exact target build metadata.
  **components:** Doofus-Dock-Hermes repository, artifacts/agent-ui.
  **type:** publication
  **validation:** Screenshot signature/dimensions and JSON sidecar were validated; content SHA-256 matches metadata.
  **performance:** Not applicable.
  **risks:** Artifact documents only the native target fixture.
  **follow_up:** Publish artifact and journal record, then continue the fixed-source parity queue.

- **id:** HUR-001-098
  **timestamp:** 2026-11-15T14:51:27Z
  **what:** Added pinned-source `.gitignore` filtering and a persisted per-project Show/Hide gitignored files control to the native Files tree; retained nested repository roots and source-defined exclusions; made refresh preserve and reload expanded directories.
  **why:** The downloaded Hermes Files source filters ignored paths by default and allows each project to reveal ignored files, while the native tree previously omitted this behavior.
  **components:** apps/hermes-native/src/files.rs, apps/hermes-native/src/app.rs, apps/hermes-native/src/layout.rs, app docs and agent UI capture.
  **type:** feature and validation
  **validation:** `cargo fmt --all -- --check`; `cargo test -p hermes-native --all-targets --features agent-automation` passed 134 library + 11 protocol tests; all-target feature check and `git diff --check` passed. Tests cover root/nested ignore rules, negations, nested repositories, reveal-floor exclusions and persisted project scoping. An accessible agent click revealed `ignored-preview.txt`; the distinct 1440×900 screen was visually inspected.
  **performance:** Directory enumeration and matching run on the Files worker; reveal mode skips Gitignore matching; each listing is bounded to 1,000 entries.
  **risks:** Ignore matchers are rebuilt per directory request rather than cached; remote filesystem semantics and live/source-paired Windows acceptance remain open. Synthetic screenshots do not close parity rows.
  **follow_up:** Add cache invalidation and source-paired acceptance while progressing remaining Files features.

- **id:** HUR-001-099
  **timestamp:** 2026-11-15T14:54:33Z
  **what:** Captured the Files tree after the accessible Show gitignored files action and added an exact-build sidecar.
  **why:** Verify the filter interaction and ignored-file reveal visually, with a route-specific artifact distinct from the default preview capture.
  **components:** artifacts/agent-ui/files-show-ignored-20261115.png, artifacts/agent-ui/files-show-ignored-20261115.json.
  **type:** evidence
  **validation:** AccessKit action succeeded; `ignored-preview.txt` is visible; the PNG is 1440×900 and SHA-256 metadata is bound to the screenshot, automation binary, pinned Hermes/DoofusDock SHAs, and implementation commit `70c9d33384e1e73542c8d8f42680a6924d230e2b`.
  **performance:** Capture completed in the isolated local runner.
  **risks:** Synthetic target-only screenshot; no paired Hermes source or installed Windows host acceptance.
  **follow_up:** Publish the artifact and continue source-paired Files review.

- **id:** HUR-001-100
  **timestamp:** 2026-11-15T15:00:00Z
  **what:** Replaced the PDF preview's isolated page display with a fit-to-pane continuous page stack, accessible page jumps and zoom; added all-page dimensions, visible-neighbor requests and an eight-page raster cache.
  **why:** The pinned Hermes file preview opens PDFs in Chromium's embedded document viewer, whose normal interaction is a continuous document surface rather than one bitmap page at a time.
  **components:** apps/hermes-native/src/files.rs, apps/hermes-native/src/app.rs, apps/hermes-native/src/bin/hermes-automation.rs, Files docs and PDF automation fixtures.
  **type:** fidelity feature slice
  **validation:** Focused PDF rendering test passed with dimensions for every page; all-target agent feature check passed. The six-page fixture captured a distinct 1440×900 scrolled document after four accessible Next page clicks and Zoom in; visible page colors confirm a continuous sequence, and AccessKit exposes page and zoom controls.
  **performance:** PDF work stays on the worker; only visible/nearby pages are requested; rendered PDF page cache is bounded to eight images.
  **risks:** The viewer is raster-based and does not yet provide Chromium's text selection/search, full toolbar or PDF annotations. PDFium runtime remains Windows x64; source-paired and installed-host acceptance remain open.
  **follow_up:** Validate manual wheel scrolling and add source-paired PDF interaction/screenshots; continue other Files features.

- **id:** HUR-001-101
  **timestamp:** 2026-11-15T15:20:00Z
  **what:** Added bounded semantic scroll actions to the offline UI runner and captured the PDF preview after five 100-point scroll actions.
  **why:** The previous fixture screenshots shared the same initial viewport, making scroll behavior difficult to distinguish visually.
  **components:** apps/hermes-native/src/bin/hermes-automation.rs, artifacts/agent-ui/pdf-agent-scrolled-five-20261115.png.
  **type:** agent interaction and evidence.
  **validation:** Full agent feature tests passed (134 library and 11 automation tests); formatting and all-target feature check passed. The semantic scroll action returned success and screenshot shows Page 3 of 6 with later pages in view at 100%.
  **performance:** Five local semantic scroll actions and one frame capture completed in under three seconds on the development host.
  **risks:** Offline synthetic PDF fixture; it does not prove native-window wheel routing, Chromium text features, or installed Hermes integration.
  **follow_up:** Capture the rest of the feature evidence from different actual UI states; continue source-paired Files parity.

- **id:** HUR-001-102
  **timestamp:** 2026-11-15T15:16:00Z
  **what:** Bound the distinct Page 3 PDF scroll-state screenshot to the pinned sources, implementation commit and automation binary with a validated JSON sidecar.
  **why:** Make screenshot state and provenance reviewable, and ensure the artifact records an actual changed viewport.
  **components:** artifacts/agent-ui/pdf-agent-scrolled-five-20261115.png, artifacts/agent-ui/pdf-agent-scrolled-five-20261115.png.json.
  **type:** visual evidence.
  **validation:** Sidecar JSON parses; target commit, executable SHA-256 and screenshot SHA-256 are validated against current files; the 1440×900 screenshot was visually inspected.
  **performance:** Hashing and atomic sidecar write completed locally in under one second.
  **risks:** Synthetic target-only capture; no paired Hermes source or installed desktop acceptance.
  **follow_up:** Continue source-paired Files parity and native-window interaction validation.

- **id:** HUR-001-103
  **timestamp:** 2026-11-15T15:35:00Z
  **what:** Added HTML Source/Preview controls and a lazy Windows WebView2 child surface for local HTML files; added a distinct HTML fixture and navigation controls to the headless runner.
  **why:** The pinned Files preview exposes syntax-highlighted source and an embedded rendered view for local HTML.
  **components:** Cargo.toml, Cargo.lock, apps/hermes-native/src/files.rs, apps/hermes-native/src/app.rs, apps/hermes-native/src/bin/hermes-automation.rs, Files docs.
  **type:** preview fidelity feature slice.
  **validation:** Wry 0.57.2 Windows all-target check passed; 136 library and 11 automation tests passed, including HTML extension/path escaping; the isolated runner reached the accessible Preview mode and captured a distinct HTML pane state.
  **performance:** WebView2 initializes only when the user selects Preview; it is hidden when that file pane is inactive.
  **risks:** The headless fixture has no native HWND and verifies the explicit fallback, not the actual rendered document. Native Windows validation of WebView2 rendering, relative assets, scripts and active-pane bounds remains open. External navigation is blocked.
  **follow_up:** Run an isolated hidden native-window smoke against the fixture and verify DOM title, relative asset load, and hide/reopen behavior before treating HTML rendered mode as validated.

- **id:** HUR-001-104
  **timestamp:** 2026-11-15T15:50:43Z
  **what:** Corrected Windows extended-length file URL normalization and passed an isolated native WebView2 smoke that loaded local HTML, executed JavaScript, and resolved a relative SVG asset.
  **why:** The first native smoke exposed `\\?\` being encoded as a URL path; HTML preview needs a usable canonical `file:///` URL for local content and relative resources.
  **components:** apps/hermes-native/src/files.rs, apps/hermes-native/src/bin/html-preview-smoke.rs, apps/hermes-native/docs/PLAN.md, artifacts/agent-ui/html-preview-smoke-result-verified-20261115.json, docs/EXECUTION_LOGS.md.
  **type:** native preview validation and path correction.
  **validation:** `cargo fmt --all -- --check`; `cargo test -p hermes-native --all-targets --features agent-automation` (136 library + 11 automation tests); `cargo build -p hermes-native --features agent-automation --bin html-preview-smoke`; `cargo check -p hermes-native --all-targets --features agent-automation`; isolated-desktop WebView2 smoke returned title `asset-loaded:16`, proving the relative 16×16 SVG loaded and the document script ran.
  **performance:** Native smoke completed in under three seconds after process startup on the development host.
  **risks:** The smoke hosts Wry directly and does not prove Hermes Files pane docking bounds, hide/reopen behavior, or source-paired Windows visuals. The semantic headless fixture only verifies the accessible mode and its no-HWND fallback.
  **follow_up:** Validate actual Hermes Files-pane WebView2 bounds and visibility transitions in a non-foreground native run; continue fixed-source row acceptance.
- **id:** HUR-001-105
  **timestamp:** 2026-11-15T15:55:38Z
  **what:** Restricted HTML preview top-level navigation to the active local document and `about:blank`; relative subresources remain available.
  **why:** A generic `file:///` allow rule admitted navigation to unrelated local files. The preview now denies both another file URL and remote HTTPS navigation.
  **components:** apps/hermes-native/src/files.rs, apps/hermes-native/README.md, apps/hermes-native/docs/AGENT_AUTOMATION.md, apps/hermes-native/docs/ARCHITECTURE.md, apps/hermes-native/docs/PLAN.md, artifacts/agent-ui/html-preview-smoke-result-allowlist-20261115.json, docs/EXECUTION_LOGS.md.
  **type:** preview security and validation.
  **validation:** Added allow/deny unit cases for `about:blank`, selected file, a different local file, and HTTPS. `cargo fmt --all -- --check`; 137 library + 11 automation tests passed; all-target feature check passed; isolated native Wry/WebView2 smoke passed again with title `asset-loaded:16` and relative SVG natural width 16; `git diff --check` passed.
  **performance:** All automated tests completed in under two seconds; native smoke completed in under three seconds after build.
  **risks:** Native smoke does not exercise the real Hermes Files pane's Dock bounds or hide/reopen lifecycle; source-paired visual acceptance remains open.
  **follow_up:** Run the Hermes app's own Files-pane fixture on a non-foreground native desktop, capture the rendered content and verify resize/hide/reopen transitions.

- **id:** HUR-001-106
  **timestamp:** 2026-11-15T15:57:55Z
  **what:** Synchronized user-facing documentation with the standalone native WebView2 smoke's actual scope and retained only the successful, current smoke evidence artifacts.
  **why:** Make the distinction clear between verified WebView2 loading and the still-unverified Hermes pane integration, and avoid presenting blank or failed exploratory captures as current evidence.
  **components:** apps/hermes-native/README.md, apps/hermes-native/docs/AGENT_AUTOMATION.md, artifacts/agent-ui/html-preview-headless-fallback-20261115.png, artifacts/agent-ui/html-preview-smoke-result-allowlist-20261115.json.
  **type:** documentation and evidence hygiene.
  **validation:** Documentation now says standalone local HTML, JavaScript and relative SVG loading was verified; Files-pane bounds and hide/reopen remain open. `git diff --check` passed; retained smoke JSON reports `asset-loaded:16`.
  **performance:** Documentation and artifact cleanup completed locally in under one second.
  **risks:** The retained headless screenshot depicts the no-HWND retry fallback and is not rendered HTML evidence.
  **follow_up:** Capture actual Files-pane rendering when the native fixture path is available; maintain distinct route/state screenshots.
- **id:** HUR-001-107
  **timestamp:** 2026-11-15T16:08:23Z
  **what:** Upgraded the HTML smoke to launch Hermes Native itself with an isolated Files fixture, and verified the real child WebView2 loads, fits inside the active Dock preview, hides when rendered mode is left, and reopens successfully.
  **why:** The previous smoke tested Wry independently and could not prove the product's FilePreviewStore and DoofusDock integration.
  **components:** apps/hermes-native/src/files.rs, apps/hermes-native/src/app.rs, apps/hermes-native/src/bin/html-preview-smoke.rs, apps/hermes-native/docs/PLAN.md, apps/hermes-native/docs/AGENT_AUTOMATION.md, artifacts/agent-ui/html-preview-files-pane-hide-reopen-20261115.json, docs/EXECUTION_LOGS.md.
  **type:** integrated native UI validation.
  **validation:** `cargo fmt --all -- --check`; `cargo test -p hermes-native --all-targets --features agent-automation` (137 library + 11 automation); `cargo check -p hermes-native --all-targets --features agent-automation`; native smoke build and isolated-desktop run passed. Recorded initial/hidden/reopened states: initial and reopened visible at `[792.125,157.34375,329.28125,742.65625]`, hidden state false, document title `asset-loaded:16`, active file path, and no WebView error. Hermes gateway remained disabled and the user's foreground desktop was not switched.
  **performance:** Integrated native run completed in under five seconds after build.
  **risks:** Synthetic fixture only; no paired upstream screenshot or acceptance rows are closed. External navigation behavior and other preview-specific parity remain open.
  **follow_up:** Capture the rendered native pane if reliable isolated-desktop screenshot acquisition is added; continue fixed-source acceptance against the downloaded Hermes snapshot.
- **id:** HUR-001-108
  **timestamp:** 2026-11-15T16:16:24Z
  **what:** Added a native PNG capture to the Hermes Files-pane smoke and retained both its raw PrintWindow result and a separate crop containing the rendered HTML child WebView.
  **why:** Produce a visibly distinct, native-rendered HTML artifact without switching the operator desktop, while documenting the capture boundary.
  **components:** apps/hermes-native/src/bin/html-preview-smoke.rs, apps/hermes-native/docs/PLAN.md, apps/hermes-native/docs/AGENT_AUTOMATION.md, artifacts/agent-ui/html-preview-files-pane-native-capture-20261115.png, artifacts/agent-ui/html-preview-files-pane-native-capture-20261115.json, artifacts/agent-ui/html-preview-rendered-child-20261115.png, artifacts/agent-ui/html-preview-rendered-child-20261115.png.json, docs/EXECUTION_LOGS.md.
  **type:** native visual evidence.
  **validation:** The isolated Hermes app smoke passed with visible-hidden-reopened state and loaded title `asset-loaded:16`; raw PNG is 2182×1406; inspected crop is 495×1114 and visibly shows the local HTML page and its loaded SVG. Sidecar binds both image hashes, smoke binary, fixed source commits, route, preview bounds and capture limitation. Full Rust tests/check and screenshot PNG decode passed.
  **performance:** Capture, PNG validation and crop verification completed in under one second after native smoke.
  **risks:** PrintWindow omitted the parent OpenGL-rendered Hermes shell. The crop shows only the actual child WebView, not full app chrome, and remains target-only evidence.
  **follow_up:** Use renderer-aware capture for full native app screenshots and compare to matched pinned-source states; continue all remaining parity rows.
- **id:** HUR-001-109
  **timestamp:** 2026-11-15T16:24:09Z
  **what:** Replaced the OpenGL-blind PrintWindow shell capture with egui's renderer framebuffer screenshot callback and captured the full Hermes shell on a separate desktop.
  **why:** Full-app visual evidence must include Hermes' renderer output while the agent works without switching the operator's foreground desktop.
  **components:** apps/hermes-native/src/bin/html-preview-smoke.rs, apps/hermes-native/docs/PLAN.md, apps/hermes-native/docs/AGENT_AUTOMATION.md, artifacts/agent-ui/html-preview-files-pane-egui-capture-20261115.png, artifacts/agent-ui/html-preview-files-pane-egui-capture-20261115.json, artifacts/agent-ui/html-preview-files-pane-egui-capture-20261115.png.json, docs/EXECUTION_LOGS.md.
  **type:** background native screenshot capture.
  **validation:** `cargo fmt --all -- --check`; 137 library + 11 automation tests; all-target check/build; isolated Hermes Native run returned the Files HTML state and completed visible-hidden-reopened checks; egui callback produced a validated 2160×1350 PNG which was visually inspected. Sidecar binds source SHA, DoofusDock SHA, target commit, smoke binary, output image and state JSON.
  **performance:** Background app lifecycle and capture completed in under six seconds.
  **risks:** egui renderer capture includes the full Hermes shell but not its separate native WebView2 child. The HTML body is held in a separately labeled child-render capture; neither is source-paired evidence.
  **follow_up:** Add a dual-renderer composition or viewport-capture path that records the native child alongside the full shell; continue the 499-row fixed-source acceptance.
- **id:** HUR-001-110
  **timestamp:** 2026-11-15T16:34:09.3378911Z
  **what:** Captured the session pane after activating Show archived sessions and recorded the rendered offline archived view with the click receipt and content hash.
  **why:** Scenario-labeled screenshots had identical pixels because the capture runner was launched on the unchanged initial fixture; visual evidence must correspond to the actual exercised state.
  **components:** apps/hermes-native/docs/PLAN.md, apps/hermes-native/docs/AGENT_AUTOMATION.md, apps/hermes-native/docs/CHANGELOG.agent.md, artifacts/agent-ui/session-pane-archived-view-20261115.png, artifacts/agent-ui/session-pane-archived-view-20261115.json, docs/EXECUTION_LOGS.md.
  **type:** target-only workflow screenshot evidence.
  **validation:** One-shot AccessKit click returned success; the 1440x900 screenshot was visually inspected and SHA-256 differs from the unchanged session pane capture; JSON sidecar parsed and binds action, image, binary, fixed sources and target commit.
  **performance:** Semantic interaction and capture completed in under three seconds after incremental build.
  **risks:** Gateway is intentionally disabled, so archived sessions show unavailable/no items. This does not prove pinned-source parity or live gateway behavior.
  **follow_up:** Build matched source and Rust captures for this workflow, then continue the fixed 499-row parity matrix.

- **id:** HUR-001-111
  **timestamp:** 2026-11-15T17:01:48.9134165Z
  **what:** Captured the exact pinned Hermes source first-run onboarding with its E2E Playwright harness and a same-size native launch candidate; added hash-bound source/target sidecars and updated the source screenshot gap record.
  **why:** Source pairing must use the selected downloaded commit, a real isolated app route, and a visibly identified target gap instead of later upstream code or repeated target fixture images.
  **components:** .staging/hermes-pinned-878305, .staging/tools/node-v26.13.1-win-x64, apps/hermes-native/docs/BACKLOG.md, apps/hermes-native/docs/PLAN.md, apps/hermes-native/docs/AGENT_AUTOMATION.md, apps/hermes-native/docs/CHANGELOG.agent.md, artifacts/agent-ui/hermes-upstream-first-run-878305-20261115.png, artifacts/agent-ui/hermes-upstream-first-run-878305-20261115.json, artifacts/agent-ui/hermes-native-launch-1220x800-20261115.png, artifacts/agent-ui/hermes-native-launch-1220x800-20261115.json, docs/EXECUTION_LOGS.md.
  **type:** pinned-source and native target visual gap evidence.
  **validation:** Exact-source package commit/tree and build stamp verified; frozen pnpm install completed using verified Node 26; Electron app ran on a separate Windows desktop with isolated no-provider home; upstream onboarding E2E passed 1/1 and asserted provider setup labels; CSS-scale source PNG decoded at 1220×800 and was visually inspected; native headless capture decoded at 1220×800 and was visually inspected; both sidecars bind their PNG hashes. No parity row was accepted.
  **performance:** Pinned Electron build completed; source E2E screenshot completed in 10.3s after build.
  **risks:** Source setup uses an empty provider configuration and random temporary install path; target uses a disabled gateway. They show different visual states and are not source-paired acceptance. The source checkout has a strict pnpm/Node≥26 install requirement; Bun runtime compatibility was not tested in this pass.
  **follow_up:** Implement the full source first-run flow and side-by-side card layout/choices in Hermes Native, then capture the same transitions and negative paths before accepting PAR-0001.

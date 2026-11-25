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

- **id:** HUR-001-112
  **timestamp:** 2026-11-15T17:19:48Z
  **what:** Added a pinned-source-style first-run setup modal on initial gateway discovery failure, a remote URL entry surface, explicit unsupported-flow errors, an isolated AccessKit action test and a distinct 1220x800 screenshot with metadata.
  **why:** The previous target screenshot showed a different welcome layout, and its buttons did not perform their advertised actions. The native app now presents the source choice and clearly reports missing backend workflows instead of implying success.
  **components:** apps/hermes-native/src/app.rs, apps/hermes-native/README.md, apps/hermes-native/docs/ARCHITECTURE.md, apps/hermes-native/docs/BACKLOG.md, apps/hermes-native/docs/PLAN.md, apps/hermes-native/docs/AGENT_AUTOMATION.md, apps/hermes-native/docs/CHANGELOG.agent.md, artifacts/agent-ui/hermes-native-first-run-setup-1220x800-20261115.png, artifacts/agent-ui/hermes-native-first-run-setup-1220x800-20261115.png.json, docs/EXECUTION_LOGS.md.
  **type:** native onboarding UI implementation and target-only evidence.
  **validation:** `cargo check --all-features` passed; `cargo test --all-features` passed 138 library and 11 automation tests; the focused AccessKit test clicked Install, Connect and Apply and checked explicit failure states; PNG dimensions/hash and JSON sidecar were validated; screenshot was visually inspected. An earlier full suite exposed a NaN layout when the modal rendered for headless tests; setup display was then gated on an actual initial gateway discovery failure, and the suite passed after the fix.
  **performance:** No benchmark run; the modal renders on the UI thread with no network or installer work.
  **risks:** Remote gateway probing, authentication, secure config persistence, local Hermes installation/start/cancel/recovery and physical Windows visual parity remain unimplemented. The screenshot is target-only, PAR-0001 remains pending, and no requirement was accepted.
  **follow_up:** Implement source-compatible connection and bootstrap adapters before treating the setup flow as complete; capture same-state source/target transitions and negative receipts.

- **id:** HUR-001-113
  **timestamp:** 2026-11-15T17:43:33Z
  **what:** Matched the native first-run setup card's outer geometry to the pinned 1220×800 source screenshot and refreshed setup, remote form, and unsupported Test-state captures with source/hash-bound sidecars.
  **why:** Prior target captures used stale layout dimensions after adding the exact Hermes brand asset and needed repeatable, current evidence for the source-sized modal.
  **components:** apps/hermes-native/src/app.rs; apps/hermes-native/docs/PLAN.md; apps/hermes-native/docs/CHANGELOG.agent.md; artifacts/agent-ui/hermes-native-first-run-setup-1220x800-20261115.png; artifacts/agent-ui/hermes-native-first-run-setup-1220x800-20261115.png.json; artifacts/agent-ui/hermes-native-first-run-remote-setup-1220x800-20261115.png; artifacts/agent-ui/hermes-native-first-run-remote-setup-1220x800-20261115.png.json; artifacts/agent-ui/hermes-native-first-run-remote-error-1220x800-20261115.png; artifacts/agent-ui/hermes-native-first-run-remote-error-1220x800-20261115.png.json; docs/EXECUTION_LOGS.md.
  **type:** visual-fidelity and evidence refresh.
  **validation:** cargo fmt --check passed; cargo check --all-features passed; cargo test --all-features passed 138 library and 11 automation tests before the final margin-only adjustment; all three final semantic captures rendered as valid 1220×800 PNGs and were visually inspected. Final margin-only code also passed cargo check and formatting.
  **performance:** No runtime behavior or performance-sensitive path changed.
  **risks:** Headless capture does not prove Windows DPI behavior. Source choice icons are still absent from native cards, and remote/local setup behavior is not implemented. The target screenshots are not accepted parity diffs.
  **follow_up:** Implement source-equivalent bootstrap-state detection and actual remote/local setup; capture source and native remote transition pairs; keep PAR-0001 pending.

- **id:** HUR-001-114
  **timestamp:** 2026-11-15T17:43:33Z
  **what:** Constructed the Test connection and Apply remote widgets independently and extended the onboarding AccessKit test to ensure Test remains present after Apply is clicked.
  **why:** The prior boolean short-circuit could omit construction of the second action widget in the frame where Apply was clicked.
  **components:** apps/hermes-native/src/app.rs; apps/hermes-native/docs/PLAN.md; apps/hermes-native/docs/CHANGELOG.agent.md; artifacts/agent-ui/hermes-native-first-run-setup-1220x800-20261115.png.json; artifacts/agent-ui/hermes-native-first-run-remote-setup-1220x800-20261115.png.json; artifacts/agent-ui/hermes-native-first-run-remote-error-1220x800-20261115.png.json; docs/EXECUTION_LOGS.md.
  **type:** UI interaction correctness.
  **validation:** cargo fmt passed; cargo test --all-features passed 138 library and 11 automation tests; the new AccessKit existence check runs after Apply. All three semantic captures were regenerated and metadata source hashes were refreshed.
  **performance:** No material performance impact; two buttons are constructed on each remote setup frame.
  **risks:** Both controls still intentionally return an explicit unsupported-flow message; no network, authentication, or remote state mutation is implemented.
  **follow_up:** Implement source-equivalent remote test/apply handlers and state; keep PAR-0001 pending until behavior and visual pairing pass.

- **id:** HUR-001-115
  **timestamp:** 2026-11-15T17:53:09Z
  **what:** Added first-run remote URL gating: Test requires a minimally valid HTTP(S) scheme and host, while Apply remains disabled pending a successful connection-test receipt.
  **why:** The prior placeholder allowed remote actions with an empty URL and made Apply appear available even though no valid test or secure persistence path existed.
  **components:** apps/hermes-native/src/app.rs; apps/hermes-native/README.md; apps/hermes-native/docs/ARCHITECTURE.md; apps/hermes-native/docs/BACKLOG.md; apps/hermes-native/docs/PLAN.md; artifacts/agent-ui/hermes-native-first-run-setup-1220x800-20261115.png; artifacts/agent-ui/hermes-native-first-run-setup-1220x800-20261115.png.json; artifacts/agent-ui/hermes-native-first-run-remote-setup-1220x800-20261115.png; artifacts/agent-ui/hermes-native-first-run-remote-setup-1220x800-20261115.png.json; artifacts/agent-ui/hermes-native-first-run-remote-error-1220x800-20261115.png; artifacts/agent-ui/hermes-native-first-run-remote-error-1220x800-20261115.png.json; docs/EXECUTION_LOGS.md.
  **type:** first-run workflow state fidelity.
  **validation:** URL accept/reject tests and AccessKit disabled/enabled state assertions pass; `cargo test --all-features` passes 139 library tests and 11 automation tests; three post-change 1220×800 states were visually inspected and sidecar hashes regenerated.
  **performance:** The URL gate is a small synchronous string check on render; no network work is performed.
  **risks:** HTTP URL validation is deliberately shallow and does not replace a standards-compliant parser. The remote probe/auth/test/commit path and local bootstrap remain unimplemented.
  **follow_up:** Implement source-compatible probe/authentication/config receipt handling and the local bootstrap lifecycle; keep PAR-0001 pending.

- **id:** HUR-001-116
  **timestamp:** 2026-11-15T17:57:23Z
  **what:** Tightened first-run URL authority validation for DNS/localhost hosts, bracketed IPv6, and numeric ports, with explicit malformed-authority rejection cases.
  **why:** URL enablement should reject syntactically malformed authorities before exposing Test, even while the real source connection probe is still unavailable.
  **components:** apps/hermes-native/src/app.rs; apps/hermes-native/docs/PLAN.md; apps/hermes-native/docs/CHANGELOG.agent.md; artifacts/agent-ui/hermes-native-first-run-setup-1220x800-20261115.png.json; artifacts/agent-ui/hermes-native-first-run-remote-setup-1220x800-20261115.png.json; artifacts/agent-ui/hermes-native-first-run-remote-error-1220x800-20261115.png.json; docs/EXECUTION_LOGS.md.
  **type:** URL validation and regression coverage.
  **validation:** `cargo test --all-features` passes 139 library tests and 11 automation tests; final three 1220×800 screenshots were regenerated and their JSON sidecars bind the latest app hash.
  **performance:** Host/port validation uses bounded string parsing and standard IPv6 parsing only.
  **risks:** This remains a UI gate and intentionally shallow parser, not the source's actual URL normalization, network probe, authentication, or remote apply.
  **follow_up:** Replace the shallow check with a standards-compliant URL parser as part of source-compatible remote setup; keep PAR-0001 pending.

- **id:** HUR-001-117
  **timestamp:** 2026-11-15T18:03:10Z
  **what:** Exposed explicit visible/enabled state and parent IDs in every AccessKit node returned by the native agent `ui.tree` method.
  **why:** Agents need queryable semantic state and hierarchy so Playwright-like navigation and inspection do not depend on guessing coordinates or rebuilding the tree relationships client-side.
  **components:** apps/hermes-native/src/bin/hermes_automation/automation_protocol.rs; apps/hermes-native/docs/AGENT_AUTOMATION_PROTOCOL.md; apps/hermes-native/docs/AGENT_AUTOMATION.md; apps/hermes-native/docs/PLAN.md; docs/EXECUTION_LOGS.md.
  **type:** agent locator and accessibility tree contract.
  **validation:** `cargo test --all-features` passed 139 library and 11 automation tests; AccessKit assertions checked visible, enabled, positive bounds and parent/child linkage; an actual `--stdio` process served `ui.tree` and returned the expected visible/enabled Capabilities locator with bounds and parent ID without opening a native window or starting Hermes.
  **performance:** Tree serialization adds one visibility calculation and parent ID string per AccessKit node.
  **risks:** Visibility means non-hidden with positive bounds, not confirmed unobscured pixels. Native window control and physical desktop acceptance remain open.
  **follow_up:** Add a query/filter API if needed, then provide isolated background native-window capture/control and verify workflows on a separate desktop; preserve the no-focus and no-installed-profile invariants.

- **id:** HUR-001-118
  **timestamp:** 2026-11-15T18:59:20Z
  **what:** Added actual foreground-process evidence to the native Files HTML preview smoke and made smoke result, screenshot and sidecar publication write-once. The successful off-screen run keeps the prior foreground PID and verifies WebView2 load, relative SVG, Dock bounds and hide/reopen lifecycle.
  **why:** The previous isolated-desktop run could not initialize an OpenGL window, while “active false requested” alone did not prove that the smoke left the operator's foreground window untouched.
  **components:** apps/hermes-native/src/bin/html-preview-smoke.rs; apps/hermes-native/docs/PLAN.md; apps/hermes-native/README.md; apps/hermes-native/docs/ARCHITECTURE.md; apps/hermes-native/docs/BACKLOG.md; docs/ARCHITECTURE.md; docs/BACKLOG.md; docs/EXECUTION_LOGS.md; artifacts/agent-ui/native-offscreen-window-focus-proof-verified-20261115.json; artifacts/agent-ui/native-offscreen-window-focus-proof-verified-20261115.png; artifacts/agent-ui/native-offscreen-window-focus-proof-verified-20261115.png.json.
  **type:** Windows native smoke evidence and no-clobber output safety.
  **validation:** Formatting passed; all-target feature tests passed (139 library + 12 automation); all-target feature check passed; native smoke passed and the actual before/after foreground owner PID remained 5220 while smoke PID 6720 never became foreground. WebView2 reported `asset-loaded:16`, non-zero Dock bounds and visible → hidden → reopened states. Sidecar image SHA-256 matches the PNG. Pinned-source acceptance remains 0/499 (0%); all 565 matrix rows remain pending.
  **performance:** One isolated startup and framebuffer capture; no normal runtime overhead.
  **risks:** Isolated desktop has no compatible OpenGL config; off-screen one-shot smoke is not a persistent native UI control host. The egui capture is byte-identical to the previous fixed-fixture shell capture and omits WebView2 pixels.
  **follow_up:** Add a persistent agent-controlled native host, user-selected native route actions and source-paired acceptance; retain the 499-row fixed denominator.

- **id:** HUR-001-119
  **timestamp:** 2026-11-15T19:17:36Z
  **what:** Added an explicit Windows `--native-window` launch mode for the actual HermesNativeApp using an isolated temporary profile, gateway-disabled constructor, off-screen viewport and `WS_EX_NOACTIVATE` style.
  **why:** Provide a non-foreground native accessibility inspection target so the future agent controller can inspect actual Windows-rendered control semantics.
  **components:** apps/hermes-native/src/bin/hermes-automation.rs; apps/hermes-native/README.md; apps/hermes-native/docs/PLAN.md; apps/hermes-native/docs/AGENT_AUTOMATION.md; apps/hermes-native/docs/BACKLOG.md; apps/hermes-native/docs/ARCHITECTURE.md; docs/ARCHITECTURE.md; docs/BACKLOG.md; docs/CHANGELOG.agent.md; docs/EXECUTION_LOGS.md.
  **type:** native UI automation groundwork.
  **validation:** Formatting passed; `cargo test -p hermes-native --all-targets --features agent-automation` passed 139 library and 12 automation tests; all-target feature check/build passed; Node syntax/tests passed 3/3. Actual app window was found by process ID; Windows UI Automation returned its real 32-element tree and `Capabilities` route button; `WS_EX_NOACTIVATE` was present; read-only tree inspection preserved foreground PID 25840.
  **performance:** No normal runtime overhead; isolated feature launch only.
  **risks:** UIA's `InvokePattern` is unsupported for egui buttons; `TogglePattern` changed route but activated the off-screen process even with `WS_EX_NOACTIVATE`. Posted mouse messages preserved foreground but did not navigate. Native UIA mutations are therefore intentionally undocumented as an allowed operation; no shared JS protocol or live native screenshot is connected.
  **follow_up:** Add a process-local semantic action/screenshot channel with generation fencing, then integrate it into `HermesPage` and validate that mutations preserve the user's foreground application.
- **id:** HUR-001-120
  **timestamp:** 2026-11-15T20:02:45Z
  **what:** Added native off-screen route state, generation-checked typed app actions, screenshot capture and acknowledged close over inherited NDJSON.
  **why:** Give the coding agent a persistent real-window target for distinct route inspection and captures without UIA activation or desktop focus theft.
  **components:** `src/app.rs`; `src/bin/hermes-automation.rs`; native screenshot evidence; `PLAN.md`; `AGENT_AUTOMATION.md`; `ARCHITECTURE.md`; `BACKLOG.md`.
  **type:** native UI automation and evidence.
  **validation:** Formatting and all-target feature check passed. Actual native process transitioned `/` → `/capabilities` through `RegisteredUiAction::Navigate`, produced a 2160×1350 image with matching SHA-256 sidecar, closed cleanly, and left foreground PID 5220 unchanged.
  **performance:** Opt-in automation feature only; 12 MiB response cap.
  **risks:** No live gateway; semantic native locators and reliable raw pointer effects remain unsupported. Screenshot is target-only and does not imply source parity.
  **follow_up:** Add UIA tree-to-Playwright locator reads and verify human-equivalent native input without activation; then compare against the pinned source.
- **id:** HUR-001-121
  **timestamp:** 2026-11-15T20:07:57Z
  **what:** Documented the native-window stdio protocol separately from the semantic headless protocol and clarified what repeated default-shell captures prove.
  **why:** Make route-specific screenshot evidence and the native locator/click limitations explicit to agent clients.
  **components:** `docs/AGENT_AUTOMATION_PROTOCOL.md`; `docs/AGENT_AUTOMATION.md`.
  **type:** protocol documentation and evidence quality.
  **validation:** Confirmed the documented methods, response version, request ID and 4,096-request bounds against the native host; confirmed default route and screenshot limitations against the recorded isolated capture.
  **performance:** Documentation only.
  **risks:** Native semantic tree and verified user-equivalent native clicks remain unavailable.
  **follow_up:** Add a trustworthy native accessibility/action adapter and capture distinct states only after checking route/action receipts.
- **id:** HUR-001-122
  **timestamp:** 2026-11-15T20:17:26Z
  **what:** Exposed bounded application diagnostics and before/after diagnostic snapshots through the native-window stdio protocol.
  **why:** Let headless agents inspect layout, profile, pane, gateway and session lifecycle state while operating the real renderer window.
  **components:** `src/bin/hermes-automation.rs`; `src/app.rs`; native automation documentation.
  **type:** native agent diagnostics.
  **validation:** Pending direct native-process protocol validation of initial snapshot, diagnostics bundle, route action before/after state and clean close.
  **performance:** Snapshot serialization occurs only on an explicit read/action request.
  **risks:** The isolated native host remains offline; semantic locators and generic input remain unavailable.
  **follow_up:** Validate the new protocol fields and later connect reliable native semantic locators without focus activation.
- **id:** HUR-001-123
  **timestamp:** 2026-11-15T20:21:28Z
  **what:** Verified native diagnostic snapshots and action deltas in the actual off-screen Windows process.
  **why:** Confirm the native agent can inspect layout and lifecycle state and prove which route/tab a typed action selected.
  **components:** `src/bin/hermes-automation.rs`; native action protocol; execution evidence.
  **type:** native protocol validation.
  **validation:** `cargo fmt --all -- --check`, all-target feature `cargo check`, feature-gated binary build, and actual stdio execution passed. Snapshot included the layout document and pane/gateway/chat summaries; diagnostics identified the native offline window; Navigate returned `/`→`/capabilities` and full state `/capabilities?tab=skills` at generation 1; clean close exit 0; foreground PID 5220 remained unchanged.
  **performance:** Snapshot is serialized only when requested.
  **risks:** This does not validate UIA semantic locators, generic input, a live gateway, or source-paired UI parity.
  **follow_up:** Connect native semantic locators and user-equivalent input, then exercise one matched source workflow end-to-end.
- **id:** HUR-001-124
  **timestamp:** 2026-11-15T20:28:40Z
  **what:** Removed the non-source `Hermes Desktop · native Rust shell` label from the pane-control bar.
  **why:** The development label is absent from the fixed Hermes source and clipped at the screenshot's right edge.
  **components:** `src/app.rs`; plan and execution evidence.
  **type:** visible UI fidelity correction.
  **validation:** Pending feature build and headless accessibility-tree check for retained Review/Files/Terminal controls and absent development label.
  **performance:** No expected runtime cost or behavioral effect.
  **risks:** Source-paired screenshot acceptance remains open.
  **follow_up:** Compare the remaining toolbar and pane geometry to the fixed-source capture at matched scale.
- **id:** HUR-001-125
  **timestamp:** 2026-11-15T20:29:59Z
  **what:** Verified the corrected pane bar through the headless app's semantic tree.
  **why:** Confirm retained app navigation controls and removal of the clipped development label without another redundant image.
  **components:** Native Hermes app automation protocol and pane toolbar.
  **type:** semantic UI verification.
  **validation:** Formatting, all-target feature check, feature binary build, `ui.tree` assertions for visible Review/Files/Terminal buttons and absent `Hermes Desktop · native Rust shell`, acknowledged close, exit code 0, empty stderr.
  **performance:** No runtime change beyond removed label construction.
  **risks:** This does not provide source-paired screenshot acceptance.
  **follow_up:** Compare the remaining visible toolbar against a matched state from the pinned source.
- **id:** HUR-001-126
  **timestamp:** 2026-11-15T20:33:54Z
  **what:** Changed unverified native coordinate clicks to return `failed: target_effect_unverified` after dispatch.
  **why:** Prevent background automation from mistaking pointer delivery for a confirmed UI action and capturing unchanged state as progress.
  **components:** `src/bin/hermes-automation.rs`; native protocol documentation; `docs/BACKLOG.md`.
  **type:** action receipt correctness.
  **validation:** Pending native process execution of `ui.click`, state reread, and clean close with foreground PID check.
  **performance:** No additional work beyond existing pointer handling.
  **risks:** Generic user-equivalent clicks remain unsupported; the error means the action may have occurred, so retry is unsafe until state is reread.
  **follow_up:** Build native semantic locators and verified user-like input without foreground activation.
- **id:** HUR-001-127
  **timestamp:** 2026-11-15T20:35:02Z
  **what:** Verified the native pointer probe returns a fail-closed unverified receipt and leaves state inspectable.
  **why:** Ensure screenshot automation cannot treat unverified pointer delivery as successful navigation.
  **components:** Native Windows agent protocol and foreground-safety check.
  **type:** native action receipt validation.
  **validation:** Formatting, all-target feature check, feature binary build and actual off-screen stdio run passed. `ui.click` returned `failed` / `target_effect_unverified`, `input_dispatched: true`, generation 1; reread still showed `/`; clean close exit 0; foreground PID 5220 before/after; empty stderr.
  **performance:** No additional runtime cost.
  **risks:** The click itself remains unsupported as a verified user-equivalent action.
  **follow_up:** Implement native semantic targets and effect-verified action dispatch; keep using typed handlers where available.
- **id:** HUR-001-128
  **timestamp:** 2026-11-15T20:36:01Z
  **what:** Marked the fail-closed native pointer receipt behavior in the feature backlog.
  **why:** Keep native semantic-action limitations and retry guidance visible at the implementation entry point.
  **components:** `docs/BACKLOG.md`.
  **type:** backlog synchronization.
  **validation:** Confirmed the backlog describes `target_effect_unverified`, input dispatch context, generation advance and state reread before retry, matching the protocol implementation.
  **performance:** Documentation only.
  **risks:** Native semantic locators and effect-verified user-like actions remain open.
  **follow_up:** Keep the unchecked native UIA/input item open until target activation is verified without focus theft.

- **id:** HUR-001-129
  **timestamp:** 2026-11-15T20:41:20Z
  **what:** Added a bounded read-only UI Automation tree query to the off-screen native process protocol.
  **why:** Let a background coding agent inspect native controls semantically while preserving the non-activating window boundary.
  **components:** `src/bin/hermes-automation.rs`; `src/bin/hermes_automation/native_uia_tree.ps1`; protocol and architecture documentation.
  **type:** native agent accessibility.
  **validation:** Pending direct native-process query, route transition and foreground-owner check.
  **performance:** The hidden UI Automation helper is asynchronous, capped at three seconds, 256 nodes and 16 levels.
  **risks:** Tree names can still expose control labels; values and text-role contents are omitted. UIA mutation remains disabled because it can activate the window.
  **follow_up:** Connect a verified, non-activating native locator action path and the HermesPage client.
- **id:** HUR-001-130
  **timestamp:** 2026-11-15T20:47:30Z
  **what:** Verified native tree reads on the initial and Capabilities routes around a typed navigation action.
  **why:** Prove semantic control discovery against the actual off-screen native app without relying on another screenshot.
  **components:** Native Windows UI Automation worker; inherited stdio protocol; route action handler.
  **type:** native process validation.
  **validation:** Formatting, feature-gated all-target check and binary build passed. The actual process returned 26 nodes on `/`, navigated through `app.action.execute`, then returned 30 nodes on `/capabilities`; `protocol.close` exited 0 and foreground PID remained 5220. No screenshot was generated. An initial idle-loop timeout revealed a missing repaint wake-up and was fixed; a harness-only PascalCase action typo was corrected before the passing run.
  **performance:** Tree query bounded to three seconds; observed protocol roundtrip completed within the 8-second harness bound.
  **risks:** This enables read-only locator discovery only; native locator mutations and source-paired acceptance remain open.
  **follow_up:** Implement action semantics that can be verified without UIA focus activation; preserve user confirmation paths.
### Revision 120 — native semantic locator actions

- **id:** HUR-001-131
- **timestamp:** 2026-11-15T21:21:57Z
- **what:** Added exact-locator native `ui.action` dispatch for click, focus, full-field set, and selected-text replacement through egui/AccessKit; increased the async UIA tree timeout to eight seconds; documented generation invalidation and verified receipts. Added a current 1440×900 offline Plugins-state screenshot.
- **why:** Let an unattended coding agent inspect and operate common native controls without activating the user's desktop, and show a genuinely populated UI state.
- **components:** `src/bin/hermes-automation.rs`, `src/bin/hermes_automation/native_uia_tree.ps1`, native protocol/architecture/backlog/README/plan docs, `artifacts/agent-ui/plugins-busy-current-20261115.png`.
- **type:** Native accessibility and agent-control capability.
- **validation:** Formatting, feature-gated all-target check and binary build passed. The real Windows process returned 26 controls on `/` and 30 on `/capabilities`; it verified focus/set/selected-text replacement, navigated to Capabilities, rejected a stale action, closed cleanly, returned no entered text, and its PID was never sampled as foreground. The 1440×900 Plugins fixture screenshot was captured and visually inspected. Source-paired parity count remains 0/499.
- **performance:** Full protocol flow completed in under eight seconds; each UIA tree query is bounded to eight seconds, 256 nodes and depth 16.
- **risks:** UIA focus metadata does not mirror egui focus; app-side receipts are authoritative for supported operations. Native keyboard/menu/drag actions, connected HermesPage integration, live gateway behavior and source-paired acceptance remain open.
- **follow_up:** Extend native control coverage by input class, connect the Playwright-shaped client, and validate source-matched workflows against the fixed Hermes download.

## HUR-001-132
- **id:** HUR-001-132
- **timestamp:** 2026-11-15T21:40:00Z
- **what:** Added a native command-palette HUD with Ctrl/Cmd+K and Ctrl/Cmd+P shortcuts, bounded registered navigation/layout results, query filtering, keyboard selection, Escape dismissal, and automation-state visibility.
- **why:** Implement a distinct pinned-source interaction surface and make it operable by unattended agents.
- **components:** `src/app.rs`, `docs/PLAN.md`, `docs/BACKLOG.md`, `docs/ARCHITECTURE.md`, `README.md`, root architecture/backlog.
- **type:** Native UI and agent-control capability.
- **validation:** `cargo fmt --all -- --check`, focused command-palette automation test (1 passed), and `cargo check -p hermes-native --all-targets --features agent-automation` passed. Full source command contributions, exact fuzzy ranking, settings context, matched screenshots, and physical Windows key input remain unverified; parity acceptance remains 0/499.
- **performance:** No performance claim; filtering a bounded in-memory list.
- **risks:** The current list and token/substring scoring do not match all upstream command sources or ranking semantics.
- **follow_up:** Add source-complete contributions and settings context, pair the HUD against the fixed source at the same viewport, and verify Windows keyboard behavior.
## HUR-001-133
- **id:** HUR-001-133
- **timestamp:** 2026-11-15T21:43:59Z
- **what:** Replaced the native palette's simplified search ranker with the pinned Desktop's AND semantics and exact/prefix/word/substring/scattered/keyword score tiers, including Unicode alphanumeric word splitting.
- **why:** Make query results and keyboard auto-selection follow the downloaded source ordering.
- **components:** `src/app.rs`, native README/architecture/backlog/plan.
- **type:** Source interaction parity.
- **validation:** Formatting and all-target feature check passed; dedicated ranking tier test and open/search/navigate/close test passed. Initial test inputs accidentally matched stronger tiers; corrected fixtures isolated the intended tiers and then passed.
- **performance:** Bounded in-memory scan; no benchmark claim.
- **risks:** Full palette contributions and settings context remain incomplete; no source-paired visual receipt.
- **follow_up:** Implement the missing source command groups and settings hand-off; retain PAR-0025 pending until end-to-end source pairing.
## HUR-001-134
- **id:** HUR-001-134
- **timestamp:** 2026-11-15T21:48:08Z
- **what:** Added the pinned-order native-supported Go to destinations for Profiles, Agents, and Star map and reordered the group to match the source. Added an action-level route test across all nine destinations.
- **why:** Bring available route navigation closer to the fixed Desktop palette and ensure results actually navigate.
- **components:** `src/app.rs`, native README/architecture/backlog/plan.
- **type:** Navigation parity.
- **validation:** Go to ordering and all route dispatch assertions passed; ranker test, formatting, all-target feature check and diff check passed.
- **performance:** No performance claim.
- **risks:** New-window action and source dynamic/contributed command groups are absent; several destinations themselves remain incomplete UI routes.
- **follow_up:** Build source dynamic groups and route content; keep PAR-0025 pending until paired screenshot and native keyboard evidence pass.
## HUR-001-135
- **id:** HUR-001-135
- **timestamp:** 2026-11-15T21:51:40Z
- **what:** Added query-scoped palette results for pinned, active and archived cached sessions, matching title/preview/ID/profile fields, and direct jumps for pinned-format session IDs.
- **why:** Let an agent find and open session work from the same searchable surface used by the person.
- **components:** `src/app.rs`, native README/architecture/backlog/plan.
- **type:** Session navigation parity.
- **validation:** Session fixture test covers all three groups, preview search, direct-ID match and actual profile-scoped session action; formatting and all-target feature check pass.
- **performance:** Scans current in-memory records only; no benchmark claim.
- **risks:** Source refreshes all-profile sessions and archived results on each palette open; native uses whatever is currently loaded and may have incomplete pin ordering.
- **follow_up:** Add request-correlated all-profile session/archived reads and the source's full keyword metadata before accepting PAR-0025.
## HUR-001-136
- **id:** HUR-001-136
- **timestamp:** 2026-11-15T22:01:38Z
- **what:** Added an asynchronous, request-ID-fenced all-profile session refresh for the command palette using the pinned 200-row/minimum-one-message/recent query. Palette opening also refreshes archived inventory; cached results remain available during refresh. Added owner-profile fallback and observable result/pending/error flags.
- **why:** Match the pinned Desktop's session discovery rather than limiting the command palette to whatever the active-profile sidebar already loaded.
- **components:** `src/app.rs`, `src/gateway.rs`, native protocol, README/architecture/backlog/plan.
- **type:** Authenticated read and session navigation.
- **validation:** Five focused command-palette tests passed; HTTP fixture asserts the exact all-profile URL, ticket, response fields and active-owner fallback. All-target feature check and diff check passed.
- **performance:** One active all-profile HTTP request plus archived refresh on each open; active results bounded to 200 and archived palette rows bounded to the first 200.
- **risks:** Archived fetch currently loads pages through existing API; local ordered pin IDs and authenticated live endpoint behavior remain unverified.
- **follow_up:** Add canonical local pin ordering; exercise refresh/timeout/reopen races through the actual gateway bridge and compare source/target session groups.
## HUR-001-137
- **id:** HUR-001-137
- **timestamp:** 2026-11-15T22:12:10Z
- **what:** Added validated, bounded, persistent pinned-session order per gateway scope in UiPreferences. Palette resolves the user's order before server-flagged fallback. Pin/unpin success receipts update the list; archive success removes the pin and updates cached session metadata.
- **why:** Match fixed Desktop's persisted local pin ordering without sharing it between independent gateway instances.
- **components:** `src/layout.rs`, `src/app.rs`, native plan/backlog/README/architecture.
- **type:** Session navigation and persistent preferences.
- **validation:** Layout test verifies order persistence across reload, gateway-scope isolation and unpin. App test verifies local order precedes server pin order. Five palette tests, all-target feature check and diff check passed.
- **performance:** Bounded to 20,000 IDs per scope and persisted through the existing atomic preference writer; no benchmark claim.
- **risks:** Manual drag reorder is missing. API origin plus instance ID is the current gateway key; exact equivalence to source local bare key/remote base-URL scope is unverified.
- **follow_up:** Implement manual reordering and verify key stability across gateway restarts and remote profile switches.

- **id:** HUR-001-138
- **timestamp:** 2026-11-15T22:30:00Z
- **what:** Added a separate saved-order pinned-session sidebar group and accessible row drag handles; added subset re-slot persistence that preserves unloaded pins.
- **why:** Match the pinned Desktop's hand-reorderable Pinned list and retain IDs whose session rows are not currently loaded.
- **components:** `src/app.rs`, `src/layout.rs`, automation protocol and native docs.
- **type:** Session navigation and persistent preferences.
- **validation:** Pending final full feature tests and runner build. The focused headless UI drag already persisted the expected order while retaining an unresolved pin.
- **performance:** Reorders the bounded visible pin list and uses the existing atomic preferences writer; no benchmark claim.
- **risks:** Source-paired Windows visual and physical interaction, authenticated live gateway behavior and exact gateway scope equivalence remain unverified; PAR-0075 stays pending.
- **follow_up:** Complete source-paired sidebar acceptance and continue the remaining 499 in-scope behaviors.
- **id:** HUR-001-139
- **timestamp:** 2026-11-15T22:33:42Z
- **what:** Recorded successful full validation for the pinned-session sidebar reorder slice.
- **why:** Keep the tested state and its remaining source/native acceptance limits explicit.
- **components:** `src/app.rs`, `src/layout.rs`, `src/bin/hermes_automation/automation_protocol.rs`, native docs.
- **type:** Validation record.
- **validation:** `cargo fmt --all -- --check`, 147 library tests, 13 automation protocol tests, all-target feature check, automation runner build and `git diff --check` passed. The pointer test dragged an accessible pinned-row handle and read back the exact saved order, including an unresolved pin.
- **performance:** No performance change or benchmark claimed.
- **risks:** Headless egui proof does not establish source-paired Windows visuals, physical pointer behavior or authenticated live gateway semantics.
- **follow_up:** Continue with source-paired acceptance and the remaining parity inventory; do not mark PAR-0075 accepted yet.
- **id:** HUR-001-140
- **timestamp:** 2026-11-15T22:37:36Z
- **what:** Added stale-drag fencing so a concurrent pin-order or gateway-scope change cancels the pending sidebar reorder.
- **why:** Prevent an old pointer gesture from overwriting a newer pin or unpin receipt.
- **components:** `src/app.rs`, native plan/architecture/automation docs.
- **type:** Session ordering safety.
- **validation:** `cargo fmt --all -- --check`, 148 library tests, 13 automation protocol tests, all-target feature check, automation runner build and `git diff --check` passed. Tests cover persisted pointer reorder, unresolved pin retention and concurrent-order cancellation.
- **performance:** No benchmark claim.
- **risks:** Source-paired Windows visual/physical acceptance and authenticated live gateway behavior remain open.
- **follow_up:** Continue the remaining parity inventory; PAR-0075 remains pending.
- **id:** HUR-001-141
- **timestamp:** 2026-11-15T23:07:58Z
- **what:** Added source-shaped recency date groups and profile-scoped manual ordering for recent Hermes sidebar sessions.
- **why:** Preserve the downloaded Desktop snapshot's chronological date groups while letting users reorder sessions within a group.
- **components:** `src/app.rs`, `src/layout.rs`, `src/bin/hermes-automation.rs`, `src/bin/hermes_automation/automation_protocol.rs`, `README.md`, native architecture/backlog/plan/automation docs, `artifacts/agent-ui/recent-session-groups-20261115-v3.png` and sidecar.
- **type:** Native sidebar behavior and offline agent fixture.
- **validation:** 154/154 library tests and 14/14 automation protocol tests passed. The protocol test performs a real egui pointer drag, verifies saved order, and confirms cross-group drag leaves order unchanged. Date tests cover 4 a.m. rollover, locale week start, head-run cutoff and fresh ID reconciliation. All-target feature check, agent binary build, rustfmt check and `git diff --check` passed. Headless screenshot plus provenance JSON captured at 1440x900.
- **performance:** No benchmark claim.
- **risks:** Source-paired Windows visuals/physical pointer behavior, live authenticated gateway behavior and accessibility outside the headless semantic path remain unverified; PAR-0076 stays pending.
- **follow_up:** Continue the fixed-source acceptance matrix and gather matched source/native Windows evidence before accepting PAR-0076.
- **id:** HUR-001-142
- **timestamp:** 2026-11-15T23:19:14Z
- **what:** Added the fixed-source 12-color session Appearance submenu, persisted overrides, session-row color dots and clear behavior.
- **why:** Implement PAR-0078's per-session color interaction using the downloaded Hermes Desktop palette.
- **components:** `src/layout.rs`, `src/app.rs`, `src/bin/hermes_automation/automation_protocol.rs`, README, plan, backlog, architecture and execution records.
- **type:** Native sidebar appearance parity.
- **validation:** 155/155 library tests and 15/15 automation protocol tests passed. The semantic UI test opens the actions menu and Appearance submenu, applies `hsl(120 68% 58%)`, checks persisted JSON, clears it and verifies removal. All-target feature check, agent binary build, rustfmt and `git diff --check` passed.
- **performance:** No benchmark claim.
- **risks:** Overrides currently key by stored session ID rather than upstream compression lineage; clearing uses a neutral dot because project-color inheritance is not implemented. Matched Windows source/target visuals and authenticated behavior remain open; PAR-0078 remains pending.
- **follow_up:** Add lineage-root transport and project color fallback, then collect source-paired Windows interaction evidence before acceptance.
- **id:** HUR-001-143
- **timestamp:** 2026-11-15T23:28:57Z
- **what:** Matched the upstream six-column color swatch presentation and captured the populated session Appearance state.
- **why:** Make the new color control visually reviewable and visibly distinct from the prior empty or menu-closed screenshots.
- **components:** `src/app.rs`, `README.md`, `docs/AGENT_AUTOMATION.md`, `artifacts/agent-ui/recent-session-appearance-20261115.png` and provenance sidecar.
- **type:** Visual fixture and evidence.
- **validation:** Full 155-test library suite, 15-test automation suite, all-target feature check, runner build, rustfmt and diff checks passed. Headless capture opened the session menu and Appearance grid, and was visually inspected at 1440×900.
- **performance:** No benchmark claim.
- **risks:** Synthetic offline target-only evidence; no fixed-source paired screenshot, authenticated gateway, or physical DPI proof. PAR-0078 remains pending.
- **follow_up:** Preserve the screenshot's source, target binary and image hashes; collect paired source/native behavior for acceptance.
- **id:** HUR-001-144
- **timestamp:** 2026-11-15T23:32:00Z
- **what:** Corrected palette rendering to convert the source HSL swatches as HSL and regenerated the Appearance screenshot.
- **why:** Ensure the 12 visible native colors match the downloaded CSS HSL values rather than a visually similar HSV palette.
- **components:** `src/app.rs`, HSL conversion test, and `artifacts/agent-ui/recent-session-appearance-20261115.png` with provenance.
- **type:** Color fidelity correction and visual evidence.
- **validation:** RGB checks for the 0°, 120° and 240° source hues passed. Full suite passed with 156 library and 15 automation tests; all-target check, runner build, formatting and diff checks passed. Screenshot recaptured and visually reviewed at 1440×900.
- **performance:** No benchmark claim.
- **risks:** Compression-lineage persistence and project-color fallback remain open. Matched source/native Windows and authenticated gateway acceptance remain pending.
- **follow_up:** Complete remaining PAR-0078 behaviors and source-paired acceptance before marking the row accepted.
- **id:** HUR-001-145
- **timestamp:** 2026-11-15T23:40:39Z
- **what:** Carried the pinned REST `_lineage_root_id` through native session summaries and keyed color overrides by the durable lineage identity.
- **why:** Preserve a chosen session color when Hermes rotates the visible session ID after compression, matching the pinned `sessionPinId` behavior.
- **components:** `src/gateway.rs`, `src/app.rs`, semantic automation test, `README.md`, plan, backlog and architecture docs, current Appearance capture.
- **type:** Session identity parity.
- **validation:** Session-list fixture decodes `_lineage_root_id`; semantic UI test selects and clears a color under `lineage-recent-a` while the visible row ID is `recent-a`. 156 library and 15 automation tests passed; all-target check, runner build, rustfmt and diff checks passed. Screenshot recaptured and visually reviewed.
- **performance:** No benchmark claim.
- **risks:** Project-color fallback and source-paired Windows/live acceptance remain open; PAR-0078 remains pending.
- **follow_up:** Implement inherited project color resolution, then complete source-paired visual and authenticated behavior acceptance.
- **id:** HUR-001-146
- **timestamp:** 2026-11-15T23:52:34Z
- **what:** Added profile-scoped project-tree color inheritance for native session row markers, below explicit durable session overrides.
- **why:** Match the pinned Desktop's `sessionProjectColor` ownership fallback and neutral behavior for uncolored projects.
- **components:** `src/gateway.rs`, `src/app.rs`, session color tests, README, plan, backlog, architecture docs.
- **type:** Profile-scoped appearance parity.
- **validation:** Full target suite passed with 158 library and 15 automation protocol tests; all-target feature check, automation runner build, rustfmt and diff checks passed. A WebSocket fixture asserts `projects.tree` method/profile scope and decodes an owning session color; parser/color tests cover preview/lane rows, HSL, hex, explicit override precedence and neutral fallback.
- **performance:** No benchmark claim; project data is requested once per visible profile per gateway connection and refreshed after reconnect/session-list updates.
- **risks:** Authenticated live gateway and matched Windows source/native screenshots remain unverified; PAR-0078 remains pending.
- **follow_up:** Capture source-paired Appearance states and verify project ownership/color on an authenticated profile before accepting PAR-0078.
- **id:** HUR-001-147
- **timestamp:** 2026-11-16T00:10:00Z
- **what:** Added stale-receipt rejection and cache-preserving failure handling for profile-scoped project colors.
- **why:** Prevent delayed or denied project-tree reads from replacing valid row colors or crossing profile boundaries.
- **components:** `src/app.rs`, project color receipt lifecycle test, plan and architecture documentation.
- **type:** Async state correctness.
- **validation:** Focused test rejects stale IDs, preserves the pending latest request, preserves last known good colors after failure, isolates profiles and applies a fresh replacement.
- **performance:** No benchmark claim.
- **risks:** PAR-0078 still requires authenticated live and source-paired Windows evidence.
- **follow_up:** Run the full feature suite and finish source-paired/live acceptance when a safe test profile is available.
- **id:** HUR-001-148
- **timestamp:** 2026-11-16T00:16:35Z
- **what:** Added generation-fenced `ui.keypress` to the off-screen native window protocol and verified Ctrl+P/Escape palette navigation by state readback.
- **why:** Let agents exercise actual native-window keyboard workflows headlessly without activating the desktop window.
- **components:** `src/bin/hermes-automation.rs`, native protocol parser visibility, protocol/architecture/README/backlog/plan documentation.
- **type:** Native agent keyboard control.
- **validation:** 159 library tests and 15 automation protocol tests passed; all-target feature check, automation binary build, rustfmt and diff check passed. Actual `--native-window --stdio` process opened then closed the command palette; both key receipts remained fail-closed, `protocol.close` exited 0, and stderr was empty.
- **performance:** No benchmark claim; input is queued in the existing egui frame path.
- **risks:** Generic key effects require state readback; selector menus, drag/drop, HermesPage integration, live gateway access and source-paired parity remain open.
- **follow_up:** Extend native selector/menu and drag/drop operations, then connect the Playwright-shaped client to this native host.
- **id:** HUR-001-149
- **timestamp:** 2026-11-16T00:28:17Z
- **what:** Added `HermesPage.launchNative()` and native locator/state/key/screenshot support for the off-screen Hermes window.
- **why:** Give coding agents one Playwright-shaped API for both headless UI tests and actual native-window interaction.
- **components:** `agent/hermes-page.mjs`, native-client fixtures, protocol/README/architecture/backlog/plan docs.
- **type:** Agent UI client integration.
- **validation:** Four Node client tests passed. A real Windows process used `HermesPage.launchNative()` to resolve/click Capabilities, verify `/capabilities`, dispatch Ctrl+P/Escape and read back palette state, capture a valid 2160×1350 PNG with SHA-256, and close with exit 0.
- **performance:** No benchmark claim; UIA tree queries use the existing eight-second native bound.
- **risks:** Native select, scroll and drag are unsupported; live gateway and source-paired parity remain unverified.
- **follow_up:** Add native selector/menu and pointer gestures, then keep the screenshot matrix tied to pinned-source states.
- **id:** HUR-001-150
- **timestamp:** 2026-11-16T00:42:19Z
- **what:** Added native ComboBox option selection through `HermesPage.selectOption()` and sanitized non-finite UIA popup bounds.
- **why:** Make actual off-screen interface-mode and Dock-layout selectors agent-operable while keeping the UIA tree valid JSON.
- **components:** `src/bin/hermes_automation/native_uia_tree.ps1`, `agent/hermes-page.mjs`, native client fixture tests, automation/control-crosswalk/architecture/backlog/plan docs.
- **type:** Native selector control and UIA robustness.
- **validation:** 159 Rust library and 15 protocol tests passed; all-target feature check/build, rustfmt, Node syntax and 4 client tests passed. Actual native HermesPage selected Simple/Advanced and Basic/Default, read each resulting state, and captured a PNG; process exited cleanly.
- **performance:** No benchmark claim; each selection is bounded by current UIA tree and protocol request limits.
- **risks:** The option label must resolve to one clickable item within the ComboBox and uniquely across the window. Native scroll/drag, live gateway and source-paired acceptance remain open.
- **follow_up:** Extend native scroll and drag gestures, then validate more source-matched selector workflows.
- **id:** HUR-001-151
- **timestamp:** 2026-11-16T01:10:40Z
- **what:** Added generation-fenced native viewport wheel input and `HermesPage.wheelAt()` for headless scrolling on dense screens.
- **why:** Keep agents able to scroll the actual native renderer when a populated route complete Windows UIA tree exceeds the bounded locator-read time.
- **components:** `src/bin/hermes-automation.rs`, `agent/hermes-page.mjs`, HermesPage protocol fixtures, native automation, architecture, backlog, README and parity plan.
- **type:** Native agent pointer input.
- **validation:** 159 Rust library tests and 15 automation protocol tests passed; all-target feature check/build, rustfmt, diff check and 4 Node client tests passed. The actual off-screen Hermes process loaded 2,379 plugin entries, dispatched `ui.wheel` at (900,700) with deltaY -640, and captured a different 2160×1350 image (SHA-256 before `cb5598815652630de43f03ff6bb6e94bf836c640802e96be6eccfd54ef53d5d0`, after `8ff6976c4011133a151550d4847a27b86fc6a1ad168de8578876731d2041283c`); visual inspection confirmed the visible catalog cards changed. The exact-locator UIA tree still times out on this dense view.
- **performance:** No performance improvement claimed; the existing eight-second complete-tree bound remains.
- **risks:** Viewport wheel uses explicit logical coordinates and cannot prove its target effect from dispatch alone; screenshot/state readback is required. Dense complete UIA trees and native drag/drop remain open.
- **follow_up:** Add native drag, then optimize UIA traversal/virtualization for dense pages and expand source-matched interaction acceptance.
- **id:** HUR-001-152
  **timestamp:** 2026-11-16T01:25:51Z
  **what:** Reworded the parity status snapshot to report accepted requirements as a count and removed the ambiguous 0% label. Corrected the plan header's working revision to match its appended revision history.
  **why:** Keep source-parity acceptance distinct from partial engineering work; no overall implementation percentage has a defensible denominator.
  **components:** `docs/BACKLOG.md`, `apps/hermes-native/docs/BACKLOG.md`, `apps/hermes-native/docs/PLAN.md`.
  **type:** documentation / progress reporting.
  **validation:** Exact legacy lines were checked before replacement. The acceptance scope remains 499 in-scope requirements plus 66 excluded research rows. No feature or acceptance row changed.
  **performance:** No runtime change or performance claim.
  **risks:** Accepted parity count remains 0 of 499; this edit does not change implementation or acceptance evidence.
  **follow_up:** Report accepted parity counts separately from implemented features; do not give an aggregate percentage without a defined, validated measure.
- **id:** HUR-001-153
  **timestamp:** 2026-11-16T01:25:51Z
  **what:** Updated the plan's acceptance snapshot to report a row count without a percentage and synchronized the working revision with the appended plan history.
  **why:** Prevent the accepted-parity count from being mistaken for overall engineering completeness.
  **components:** `apps/hermes-native/docs/PLAN.md`.
  **type:** documentation / progress reporting.
  **validation:** Plan history ends at revision 137, and the fixed scope remains 499 in-scope requirements plus 66 excluded rows. No acceptance status changed.
  **performance:** No runtime change or performance claim.
  **risks:** Accepted parity count remains 0 of 499; this edit does not change implementation or acceptance evidence.
  **follow_up:** Keep acceptance counts separate from any future evidence-based implementation measure.
- **id:** HUR-001-154
  **timestamp:** 2026-11-16T01:42:09Z
  **what:** Added credential-free, asynchronous first-run remote gateway discovery for public status and optional OAuth-provider endpoints, with URL debounce, timeout/size/redirect bounds, stale-result fencing, and visible auth/provider/version state.
  **why:** Replace a dead first-run discovery surface with the pinned Desktop's real public auth-mode probe while keeping credentials out of discovery requests.
  **components:** `src/remote_probe.rs`, `src/app.rs`, `Cargo.toml`, `Cargo.lock`, `README.md`, `docs/ARCHITECTURE.md`, `docs/BACKLOG.md`, `docs/PLAN.md`.
  **type:** feat / parity workflow.
  **validation:** 163 Rust library tests and 15 automation protocol tests passed; all-target feature check and automation binary build passed; rustfmt passed. A local HTTP fixture confirmed `/api/status` and `/api/auth/providers` discovery, OAuth provider/version rendering, and absence of Authorization or X-Hermes-Session-Token headers. An egui harness verified the visible OAuth provider status.
  **performance:** Network probing is off the UI thread, starts after a 500 ms idle debounce, and uses an eight-second request timeout; no benchmark claim.
  **risks:** Probe discovery does not authenticate or establish the live gateway. OAuth/token login, authenticated HTTP+WebSocket test, apply/config persistence, and local bootstrap remain incomplete; PAR-0001 stays pending.
  **follow_up:** Implement source-compatible credential acquisition, exact tested-payload receipt, authenticated HTTP+WebSocket check, remote config persistence/apply, then local bootstrap lifecycle.
- **id:** HUR-001-155
  **timestamp:** 2026-11-16T16:09:56Z
  **what:** Added token-mode first-run connection testing for authenticated Hermes HTTP status and WebSocket handshake, plus a masked zeroizing token field, worker-thread execution, stale-result fencing and diagnostics.
  **why:** Move first-run remote setup toward the pinned Desktop's rule that a usable connection requires authenticated HTTP and live WebSocket transport, rather than public discovery alone.
  **components:** `src/remote_probe.rs`, `src/app.rs`, `README.md`, `docs/ARCHITECTURE.md`, `docs/BACKLOG.md`, `docs/PLAN.md`.
  **type:** feat / parity workflow.
  **validation:** `cargo test -p hermes-native --all-targets --features agent-automation --quiet` passed (165 library tests, 15 protocol tests); all-target feature check, rustfmt check and `git diff --check` passed. A local fixture verified the token header on `/api/status`, token query on `/api/ws`, and both handshake results; an egui harness verified the password field and credential-gated Test button.
  **performance:** HTTP requests use the existing eight-second bound and network work runs off the UI thread; no benchmark claim.
  **risks:** The UI token remains in zeroizing memory only during the form; no credential/config is persisted. OAuth login, exact-payload result binding, Apply/persistence, live remote attachment and local bootstrap remain incomplete. The 499-row source acceptance matrix remains unchanged and pending.
  **follow_up:** Implement OAuth browser sign-in and bind each passing test to the exact URL/auth/token payload; then implement isolated config persistence, remote gateway attach and first-run bootstrap parity.
- **id:** HUR-001-156
  **timestamp:** 2026-11-16T16:27:55Z
  **what:** Added source-shaped native PKCE OAuth sign-in for compatible first-run gateways and OAuth transport testing through authenticated status, one-use WebSocket-ticket minting and WebSocket upgrade.
  **why:** Complete the compatible RFC 8252 sign-in route required by pinned Desktop first-run remote setup.
  **components:** `src/native_oauth.rs`, `src/remote_probe.rs`, `src/app.rs`, `src/lib.rs`, `Cargo.toml`, `Cargo.lock`, README and architecture/backlog/plan docs.
  **type:** feat / parity workflow.
  **validation:** All-target tests passed (167 library tests, 15 protocol tests), all-target feature check, rustfmt check and `git diff --check`. Local fixtures exercised PKCE verifier exchange, loopback state validation, bearer HTTP status, ticket mint and ticket-authenticated WebSocket; egui harness verified enabled provider-specific Sign in control.
  **performance:** Login/network operations run off the UI thread; loopback flow is capped at five minutes, token exchange at 15 seconds, and HTTP response bodies at 512 KiB; no benchmark claim.
  **risks:** User-triggered Sign in opens the system browser. Tokens remain zeroizing in memory only and are lost when the form closes; refresh, secure persistence, older/password-provider embedded login, exact-payload test receipts, Apply and live remote attachment remain incomplete. PAR-0001 remains pending.
  **follow_up:** Add refresh and OS-secure token persistence, source-compatible fallback login, exact-payload receipts, isolated remote configuration/apply and remote gateway attachment.
- **id:** HUR-001-157
  **timestamp:** 2026-11-16T16:32:51Z
  **what:** Matched mixed-provider PKCE eligibility to the pinned source, added OAuth URL normalization at the worker boundary, redacted tokens from OAuth-session Debug output, and covered callback state mismatch.
  **why:** Harden the native sign-in path against cross-site callback injection, unsafe direct worker inputs and accidental secret disclosure while preserving source provider-selection behavior.
  **components:** `src/native_oauth.rs`, `src/app.rs`, `docs/PLAN.md`.
  **type:** fix / security hardening.
  **validation:** Full all-target suite passed (169 library tests, 15 protocol tests); all-target feature check, rustfmt check and `git diff --check` passed. New tests reject mismatched callback state, ensure token Debug redaction and verify password-only/mixed/unknown provider cases.
  **performance:** No material runtime change.
  **risks:** Native OAuth tokens still lack refresh, persistent OS-secure storage, legacy embedded fallback and remote runtime attach. No parity rows were accepted.
  **follow_up:** Implement secure credential persistence/refresh and remote attach before enabling Apply.
- **id:** HUR-001-158
  **timestamp:** 2026-11-16T16:34:14Z
  **what:** Replaced raw WebSocket client errors with a bounded generic failure so a handshake error cannot echo a one-use OAuth ticket from the query URL.
  **why:** Keep OAuth ticket values out of UI state and automation diagnostics.
  **components:** `src/remote_probe.rs`, `docs/PLAN.md`.
  **type:** fix / secret redaction.
  **validation:** Full suite, all-target feature check, format and diff checks rerun after this change; the OAuth and token WebSocket fixture paths remain covered.
  **performance:** No material runtime change.
  **risks:** Detailed socket failure classification is reduced to protect the ephemeral ticket; HTTP/auth error context remains reported separately.
  **follow_up:** Keep credential-bearing request URLs and values out of all surfaced error paths.

- **id:** HUR-001-159
**timestamp:** 2026-11-16T16:37:36Z
**what:** Bound first-run Apply eligibility to the exact current remote-test payload using a private SHA-256 fingerprint of normalized URL, auth mode and credential; credential edits revoke the gate.
**why:** Match the pinned editor rule that Apply requires a passing test of the exact values currently shown.
**components:** `src/app.rs`, `README.md`, `docs/ARCHITECTURE.md`, `docs/BACKLOG.md`, `docs/PLAN.md`.
**type:** feat / parity workflow.
**validation:** All-target test suite passed (169 library tests and 15 protocol tests); all-target feature check, rustfmt check and diff check passed. The egui harness verified Apply stays disabled before a receipt, enables for a matching receipt, then disables after the token changes.
**performance:** Fingerprint is computed locally from a short payload; no network overhead.
**risks:** Apply's persistence and live attach handler remain unimplemented; clicking it reports that limitation. No source-parity row accepted.
  **follow_up:** Implement isolated remote configuration persistence and attach before treating Apply as complete.

- **id:** HUR-001-160
  **timestamp:** 2026-11-16T17:05:02Z
  **what:** Refresh near-expiry native OAuth sessions before first-run transport tests and persist OAuth credentials with Windows DPAPI in the separate Hermes Native profile directory.
  **why:** The pinned Desktop rotates native tokens within 60 seconds of expiry and retains the encrypted session; memory-only sign-in could not survive expiry or restart.
  **components:** `src/native_oauth.rs`, `src/remote_probe.rs`, `src/native_token_store.rs`, `src/app.rs`, `src/layout.rs`, Cargo metadata and parity docs.
  **type:** feat / remote authentication.
  **validation:** 173 library tests and 15 protocol tests passed; all-target feature check, rustfmt and diff checks passed. Local HTTP/WebSocket fixture verifies refresh → rotated bearer status → ticket mint → WebSocket; Windows DPAPI test verifies encrypted disk state, replacement, load and removal. Package validator passes. UCM checker reports existing EV-0016/17 schema errors and one unresolved evidence reconciliation; historical records were left unchanged.
  **performance:** One bounded refresh request only when expiry is unknown or within 60 seconds; DPAPI and file writes occur during first-run auth/test.
  **risks:** Startup restoration and ordinary runtime refresh are not wired; token-auth persistence, config apply and remote attach remain open. No parity row accepted.
  **follow_up:** Restore stored credentials during setup, then save/select the remote configuration and attach the native gateway actor.

- **id:** HUR-001-160
  **timestamp:** 2026-11-16T17:01:12Z
  **what:** Refresh near-expiry native OAuth sessions before the first-run transport test and persist OAuth credentials with Windows DPAPI in the separate Hermes Native profile directory.
  **why:** The pinned Desktop rotates native tokens within a 60-second expiry window and retains the encrypted session for future use; in-memory-only sign-in could not preserve that flow.
  **components:** `src/native_oauth.rs`, `src/remote_probe.rs`, `src/native_token_store.rs`, `src/app.rs`, `src/layout.rs`, Cargo metadata and parity docs.
  **type:** feat / remote authentication.
  **validation:** 173 library tests and 15 protocol tests passed; all-target feature check, rustfmt and diff checks passed. Local HTTP/WebSocket fixture verifies refresh → rotated bearer status → ticket mint → WebSocket; Windows DPAPI test verifies encrypted disk state, replacement, load and removal. Package validator passes. UCM checker reports existing EV-0016/17 schema errors and one unresolved evidence reconciliation; those historical records were left unchanged.
  **performance:** One bounded refresh request only when expiry is unknown or within 60 seconds; DPAPI and file writes occur on first-run authentication/test results.
  **risks:** Startup restoration and refresh for ordinary runtime requests are not wired; token-auth persistence, connection-config apply and remote attach remain open. No parity row accepted.
  **follow_up:** Restore stored credentials during setup, then atomically save/select the remote configuration and attach the native gateway actor.

## HUR-001-161 — Save and restore remote setup URL

**id:** HUR-001-161
**timestamp:** 2026-11-16T17:17:16Z
**what:** Store the normalized gateway URL in Hermes Native UI preferences after the exact-payload Apply gate; prefill and probe that URL on next Connect, then restore a matching DPAPI OAuth session after status discovery.
**why:** Preserve successful setup progress without placing OAuth credentials in preferences or touching the installed Hermes profile.
**components:** `src/app.rs`, `src/layout.rs`, README, backlog, architecture and plan docs.
**type:** feature / parity.
**validation:** Workspace persistence test validates normalization, unsafe scheme rejection and reload. Targeted test passed. Full suite pending. PAR-0001 remains pending.
**performance:** One small validated preference write on Apply; secure token-store read occurs only on setup re-entry after matching gateway discovery.
**risks:** Does not auto-attach, persist token-mode credentials or replace the local-only gateway actor.
**follow_up:** Implement source-compatible remote gateway attach and connection lifecycle.

Validation update for HUR-001-161 (2026-11-16T17:21:50Z): full feature suite passed (174 library tests, 15 protocol tests), plus cargo check, rustfmt, diff check and package validator. Headless egui coverage verifies exact-payload Apply persistence and URL restoration on reopening Connect. PAR-0001 remains pending.

## HUR-001-162 — Protect token-mode connection credentials

**id:** HUR-001-162
**timestamp:** 2026-11-16T17:21:50Z
**what:** Encrypt static session tokens with DPAPI on exact-payload Apply and restore a credential only when the saved URL and discovered auth mode match.
**why:** Preserve setup state while keeping static credentials out of UI preferences.
**components:** src/app.rs, src/native_token_store.rs, README and parity docs.
**type:** feature / security / parity.
**validation:** DPAPI test covers plaintext absence and prevents token/OAuth cross-loading; headless app test covers Apply persistence. Full suite pending.
**performance:** Bounded local encryption and file I/O on Apply; lookup follows setup probe.
**risks:** Remote runtime attachment and automatic startup reconnect remain unimplemented.
**follow_up:** Add pinned-source remote WebSocket and REST attach, then verify the full connection lifecycle.

Validation update for HUR-001-162 (2026-11-16T17:35:35Z): 175 library tests and 15 protocol tests pass; cargo check, rustfmt, git diff --check and package validation pass. The package validator reports 491 pending items; PAR-0001 remains pending. Timestamp note: the HUR-001-162 timestamp field reused the preceding entry time; this addendum is the accurate validation time.
### HUR-001-163

- **id:** HUR-001-163
- **timestamp:** 2026-11-16T17:45:31Z
- **what:** Wire exact-test Apply to token/OAuth remote WebSocket attachment, read runtime identity and the initial session list, and restore saved remote credentials at startup.
- **why:** Persisted remote settings were not used to connect the native workspace.
- **components:** First-run UI, GatewayDiscovery, remote WebSocket attach, encrypted credential store, architecture/backlog/plan docs.
- **type:** feature
- **validation:** cargo check passed; all-target tests passed at 175 library and 15 protocol tests before two additional auth tests. A stale UI assertion failed and was updated. Final rerun pending; live-service verification remains open.
- **performance:** Network work stays on the background gateway worker.
- **risks:** Remote HTTP operations remain loopback-only; runtime OAuth refresh persistence is incomplete; attach lacks a dedicated server fixture.
- **follow_up:** Add authenticated remote HTTP operations and test the complete attach lifecycle; source-paired acceptance remains pending.
Validation update for HUR-001-163 (2026-11-16T17:45:31Z): 177 library tests and 15 protocol tests pass; cargo check, rustfmt check, git diff check, and package validation pass. No live remote service was contacted; remote HTTP-backed operations remain open.
### HUR-001-164

- **id:** HUR-001-164
- **timestamp:** 2026-11-16T18:05:35Z
- **what:** Add authenticated remote HTTP support for session/history/search/capability reads and session metadata/toolset writes, with gateway-prefix validation, redirect blocking, bounded responses and persisted OAuth refresh.
- **why:** Remote WebSocket attachment was connected, but HTTP-backed views and mutations still refused remote endpoints.
- **components:** Remote gateway auth, REST transport, session/archive/search/capability UI paths, DPAPI credential rotation, source audit and plan.
- **type:** feature
- **validation:** 181 library tests and 15 protocol tests pass; cargo check, rustfmt check, git diff check and pinned-package validator pass. Local HTTP fixtures check token/OAuth headers, target path boundaries and session snapshot/PATCH fencing. No live remote service was contacted.
- **performance:** HTTP work stays on the background gateway worker; responses remain capped at 512 KiB and requests use an eight-second timeout.
- **risks:** Other remote API routes and live source-paired behavior remain unverified; parity acceptance stays 0/499.
- **follow_up:** Port remaining source-defined remote HTTP calls, verify remote OAuth refresh rotation, then exercise against a real configured gateway and source-paired UI flows.

- **id:** HUI-2026-11-16-artifacts-extraction-core
- **timestamp:** 2026-11-16T18:20:29Z
- **what:** Added a Rust artifact model and extraction core for explicit MEDIA tags, Markdown media/link targets, URLs, file paths and producer-tool payloads, with type filtering, deduplication, timestamp normalization and Python package-download suppression.
- **why:** The pinned Hermes Artifacts route scans recent transcript messages and exposes searchable typed results; this establishes a testable Rust port core.
- **components:** apps/hermes-native/src/artifacts.rs; lib export; regex dependency; docs and execution records.
- **type:** feature foundation
- **validation:** 3 focused artifacts tests passed; cargo fmt completed. Full app tests and visible route validation not run.
- **performance:** Pure in-memory parsing over supplied transcript message slices; transcript paging/index orchestration is not yet connected.
- **risks:** Artifacts screen remains a placeholder; source page loading, exact producer-key mapping, shared media resolution, preview/open actions, query/tab route persistence and source-paired acceptance remain open. PAR requirements unchanged.
- **follow_up:** Wire the fixed-source recent-session page and 100-message transcript scan into the background HTTP worker, then build search/filter/paging, preview/open, and origin-session navigation.

- **id:** HUI-2026-11-16-artifacts-extraction-core
- **timestamp:** 2026-11-16T18:20:29Z
- **what:** Added a Rust artifact model and extraction core for explicit MEDIA tags, Markdown media/link targets, URLs, file paths and producer-tool payloads, with type filtering, deduplication, timestamp normalization and Python package-download suppression.
- **why:** The pinned Hermes Artifacts route scans recent transcript messages and exposes searchable typed results; this establishes a testable Rust port core.
- **components:** apps/hermes-native/src/artifacts.rs; lib export; regex dependency; docs and execution records.
- **type:** feature foundation
- **validation:** 3 focused artifacts tests passed; cargo fmt completed. Full app tests and visible route validation not run.
- **performance:** Pure in-memory parsing over supplied transcript message slices; transcript paging/index orchestration is not yet connected.
- **risks:** Artifacts screen remains a placeholder; source page loading, exact producer-key mapping, shared media resolution, preview/open actions, query/tab route persistence and source-paired acceptance remain open. PAR requirements unchanged.
- **follow_up:** Wire the fixed-source recent-session page and 100-message transcript scan into the background HTTP worker, then build search/filter/paging, preview/open, and origin-session navigation.

- **id:** HUI-2026-11-16-artifacts-route-hardening
- **timestamp:** 2026-11-16T18:59:53Z
- **what:** Completed the connected Artifacts route slice: bounded background indexing of recent sessions, oldest-first transcript paging, source-aware extraction for MEDIA/#media, browser screenshots and producer/terminal payloads, searchable type tabs, image previews, file/link tables, pagination and source-chat navigation. Fixed URL display labels and removed duplicate automation gating.
- **why:** Replace the stale placeholder implementation note with observable progress against the pinned Hermes Desktop Artifacts behavior while preserving strict acceptance evidence.
- **components:** apps/hermes-native/src/artifacts.rs; apps/hermes-native/src/app.rs; apps/hermes-native/src/gateway.rs; artifact fixture captures; README, architecture, backlog, source audit and plan.
- **type:** feature and correctness
- **validation:** 187 library tests and 15 automation protocol tests passed; all-target feature cargo check, formatting check, diff check and pinned package validator passed. Headless 1600x1000 gallery and links-filter captures were rendered and visually inspected.
- **performance:** Transcript indexing remains on a dedicated worker; each session is paged in batches of 100, session count is bounded at 30, and page payloads have a fixed UTF-16 size ceiling.
- **risks:** Remote artifact file resolution/open remains unavailable; exact route query state, matched source visuals, live gateway behavior and source-paired acceptance remain open. The acceptance matrix was not promoted from synthetic fixtures.
- **follow_up:** Implement gateway-scoped remote artifact resolution; compare connected source and native UI states side by side; add failure and interaction acceptance for the complete artifact route.

- **id:** HUI-2026-11-16-remote-artifact-downloads
- **timestamp:** 2026-11-16T19:19:46Z
- **what:** Added source-scoped remote artifact file downloads using `/api/fs/download`, profile/session/path query scope, token or OAuth bearer headers, disabled redirects, safe response filename handling, a background native Save File flow, and atomic streamed file replacement. Correlated completion, cancellation and failure with the initiating request.
- **why:** Complete the artifact open action for remote sessions while keeping credential transport out of URLs and preventing partial destination files.
- **components:** hermes-native gateway download worker, artifact-open UI, Windows Save dialog, architecture/backlog/source-audit/plan and README status.
- **type:** feature and transport correctness
- **validation:** 190 library tests and 15 automation protocol tests passed; all-target feature cargo check, rustfmt check, `git diff --check`, and fixed-source package validator passed. Local HTTP fixtures verify exact route/query/auth, RFC 5987 filename handling, atomic overwrite and picker cancellation. Active 499-row acceptance remains 0 accepted; supplemental package reports 491 pending.
- **performance:** Runs on a dedicated worker and streams the response body to disk; no whole-file buffering.
- **risks:** No authenticated live remote gateway was tested. Remote image previews, source-paired UI comparison and agent-specified headless save destinations remain open.
- **follow_up:** Add explicit agent destination-path action for modal-free download control; implement remote image preview through source-equivalent media resolution; continue per-row source-paired acceptance.

- **id:** HUI-2026-11-16-remote-artifact-image-preview
- **timestamp:** 2026-11-16T19:31:30Z
- **what:** Added authenticated, source-scoped `/api/fs/read-data-url` loading for gateway-owned Artifacts images, file-URL-to-gateway-path conversion, background image decode, aspect-preserving thumbnails bounded to 512×384, request/artifact correlation, and a 32-entry content/texture cache.
- **why:** Complete preview rendering for image artifacts that reside on a remote Hermes gateway.
- **components:** `hermes-native` gateway transport, image decoder and `FilePreviewStore`, Artifacts gallery, source audit, architecture, plan, backlog and README status.
- **type:** feature and bounded media transport
- **validation:** Focused HTTP/data-url tests passed; full feature suite passed with 192 library and 15 protocol tests; all-target check, format, diff, and pinned package validation passed. Supplemental validator reports 491 pending; source-paired acceptance was not promoted.
- **performance:** Dedicated worker; response bounded to 24 MiB encoded and image input to 16 MiB; thumbnails capped at 512×384; content and GPU textures evicted after 32 artifact images.
- **risks:** Inline HTTPS images do not yet use the source gateway proxy path; no live remote gateway or side-by-side visual check was run.
- **follow_up:** Implement authenticated gateway proxy fallback for external HTTPS image artifacts, then compare the complete Artifacts route against the pinned source and continue row-level acceptance.

- **id:** HUI-2026-11-16-remote-artifact-media-proxy
- **timestamp:** 2026-11-16T19:36:47Z
- **what:** Added the pinned Desktop remote media fallback for external HTTP(S) image artifacts through authenticated `/api/media/proxy`, preserving the encoded source URL and profile, rejecting embedded credentials, and reusing bounded image decoding and thumbnail caching.
- **why:** Render external artifact image URLs when they need the Hermes gateway's media proxy, as the pinned Desktop does.
- **components:** `hermes-native` gateway media transport, Artifacts source selection, README, architecture, backlog, plan and source audit.
- **type:** feature and authenticated media transport
- **validation:** Local HTTP proxy fixture verifies encoded target query, profile, token header, returned data URL and thumbnail. Full feature suite passed with 193 library and 15 protocol tests; all-target check, rustfmt, diff and source package validation passed. No parity acceptance rows were advanced.
- **performance:** Reuses the background image worker, 24 MiB encoded response limit, 16 MiB decoded input limit, 512×384 aspect-preserving thumbnails and 32-item cache.
- **risks:** No live gateway/remote origin or source-paired screenshot was verified. Other Artifacts UI/query behavior still needs source comparison.
- **follow_up:** Capture source/native Artifacts image states from the fixed snapshot with matched fixture data and continue per-row acceptance; verify the full endpoint against a live remote gateway when available.

- **id:** HUR-001-165
- **timestamp:** 2026-11-16T19:59:52Z
- **what:** Added profile-scoped Messaging channel inventory, source status mapping, counts, search, status filters, selection/details, explicit loading/offline/error/retry states, and stale-profile cache/request clearing.
- **why:** Replace the empty Messaging route with the first directly source-backed workflow while preserving profile ownership and fail-closed behavior.
- **components:** `apps/hermes-native/src/app.rs`, `apps/hermes-native/src/gateway.rs`, Messaging architecture/backlog/source audit, root README/backlog/architecture, `docs/PLAN.md`.
- **type:** feature
- **validation:** Full all-target feature suite passed: 197 library tests and 15 protocol tests. A headless egui fixture verified all four status counts, the accessible search field, and status filtering. All-target feature check, rustfmt check, diff check, and the fixed-source package validator passed; the supplemental package still reports 491 pending requirements.
- **performance:** Not measured.
- **risks:** Channel configuration, pairing/QR, bot management, localization, live gateway acceptance, and source-paired Windows visual/interaction evidence are still absent. PAR-0282 remains pending; no source acceptance row advanced.
- **follow_up:** Implement the remaining source channel management workflows and capture source-paired Windows evidence before evaluating PAR-0282.

- **id:** HUR-001-166
- **timestamp:** 2026-11-16T20:15:27Z
- **what:** Added confirmed source-compatible enable/disable for Messaging channels via the profile-scoped Hermes PUT endpoint, with local one-use ticket and remote token/OAuth auth paths, receipt fencing, relay-managed control disabling, restart/live feedback, and plan revision synchronization.
- **why:** Make channel state actions work through the native interface while preserving the gateway as the configuration authority and avoiding optimistic UI changes.
- **components:** `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, Messaging architecture/backlog/plan/source audit, root README/backlog/architecture.
- **type:** feature / gateway mutation
- **validation:** All-target feature suite passed with 197 library and 15 protocol tests. Messaging fixtures verify exact encoded profile-scoped PUT path, auth header, body, receipt, and invalid platform ID rejection. Headless UI coverage verifies status summaries, search, disabled-offline control, and filtering. All-target check, rustfmt, diff check and source package validator passed; plan header now matches revision 154.
- **performance:** Mutation runs on the existing gateway HTTP worker with five-second local and eight-second remote request timeout; no benchmark claim.
- **risks:** Configuration field editing, pairing/QR, bot management, gateway restart execution, source localization/visual parity, and live authenticated acceptance remain incomplete. PAR-0282 stays pending.
- **follow_up:** Implement configuration editing and pairing/review flows from the pinned source, then gather source-paired Windows acceptance evidence.

- **id:** HUR-001-167
- **timestamp:** 2026-11-16T20:31:30Z
- **what:** Added source-metadata-driven Messaging channel config forms, including required/recommended/advanced grouping, blank password drafts, allowlist editing/normalization, explicit clears, field docs links, and confirmed save with schema refresh.
- **why:** Replace the remaining static channel detail with the pinned Desktop's profile-scoped configuration workflow while keeping the gateway authoritative and preventing stale profile results from applying.
- **components:** `apps/hermes-native/src/app.rs`, `apps/hermes-native/src/gateway.rs`, Messaging architecture/backlog/plan, root README/backlog/architecture.
- **type:** feature / gateway mutation
- **validation:** All-target feature suite passed: 198 library tests and 15 automation protocol tests. Focused config mapping, HTTP fixture, and headless UI checks passed. All-target check, rustfmt, diff check, and pinned-source package validator passed; validator reports 491 supplemental pending capabilities. No acceptance row advanced.
- **performance:** Uses existing bounded gateway HTTP worker and response limit; configuration request body capped at 256 KiB; no benchmark claim.
- **risks:** Pairing/QR, bot management, restart execution, localization/source visual parity, and live authenticated acceptance remain incomplete. PAR-0282 remains pending.
- **follow_up:** Implement source pairing and bot-management workflows, then capture source-paired Windows behavior and visual evidence.

- **id:** HUR-001-168
- **timestamp:** 2026-11-16T20:45:11Z
- **what:** Added profile-scoped Messaging pairing inventory, pending counts, confirmed approval by request ID, and revocation after explicit confirmation by user ID.
- **why:** Match the pinned Desktop's pairing access workflow while ensuring requests and mutations stay bound to the active Hermes profile.
- **components:** `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, Messaging plan/architecture/backlog, root README/backlog/architecture.
- **type:** feature / gateway mutation
- **validation:** Full all-target feature suite passed: 199 library tests and 15 protocol tests. The remote HTTP fixture verifies profile query/body, token header, target IDs, approve/revoke receipts; the headless UI fixture checks pending/approved sections, badge, and offline-disabled actions. All-target check, rustfmt, diff, execution-log parsing and pinned-source package validator passed. No acceptance row advanced.
- **performance:** Uses the existing bounded gateway HTTP worker and 512 KiB response limit; no benchmark claim.
- **risks:** Telegram QR onboarding, bot management, gateway restart execution, localization/source visual parity, and live authenticated acceptance remain incomplete. PAR-0282 remains pending.
- **follow_up:** Implement pinned Telegram QR onboarding and then continue into bot management, with profile ownership and restart outcomes preserved.

- **id:** HUR-001-169
- **timestamp:** 2026-11-16T21:09:43Z
- **what:** Added profile-bound Telegram QR onboarding start, status polling, allowlist apply and cancel to Messaging, including QR rendering, expiry, safe Telegram deep links and stale-session receipt fencing.
- **why:** Implement the pinned Desktop's Telegram bot setup path without leaking one-time QR data across Hermes profiles.
- **components:** `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, `apps/hermes-native/Cargo.toml`, `Cargo.lock`, Messaging docs and backlogs.
- **type:** feature / gateway workflow
- **validation:** Two Telegram HTTP/input tests, Messaging headless QR/accessibility test, owner-reset test, and full all-target feature suite passed (201 library tests, 15 protocol tests). All-target feature check passed. Formatting, diff, and pinned-source package validation are recorded in the paired execution log. No source acceptance row advanced.
- **performance:** Uses the existing bounded background HTTP worker, 512 KiB response cap, one in-flight status poll, and 2-second poll cadence; no benchmark claim.
- **risks:** Post-setup gateway restart execution, bot management, localization, live authenticated behavior and source-paired Windows acceptance remain open. PAR-0282 remains pending.
- **follow_up:** Implement the gateway restart action and status flow, then continue Telegram bot management and remaining Messaging parity.

- **id:** HUR-001-170
- **timestamp:** 2026-11-16T21:23:52Z
- **what:** Added profile-scoped gateway restart and action-status polling, wired Telegram setup receipts into restart start/monitor, and displayed bounded progress, terminal outcomes and retry controls on Messaging.
- **why:** Match the pinned Desktop's restart follow-through while keeping the native UI responsive and the restart bound to the profile that changed.
- **components:** `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, Messaging plan/architecture/backlog and root summaries.
- **type:** feature / gateway lifecycle
- **validation:** Remote HTTP fixture verifies POST restart and GET action status route/query/token scope and terminal success. Headless Messaging fixture verifies the restart control is disabled offline. Full all-target feature suite passed: 202 library tests and 15 protocol tests; all-target feature check and format/diff passed. No acceptance row advanced.
- **performance:** Status polling is asynchronous, one request at a time, at 1.2-second intervals, bounded to 18 attempts; no benchmark claim.
- **risks:** Live restart behavior, transient outage/reconnect cases, bot management, localization and source-paired Windows acceptance remain open. PAR-0282 remains pending.
- **follow_up:** Validate restart lifecycle against a live Hermes gateway when available, then continue Telegram bot management and remaining Messaging parity.

- **id:** HUR-001-171
- **timestamp:** 2026-11-16T21:38:07Z
- **what:** Added source-style per-user Telegram QR allowlist editing, owner indication, source-style empty-list apply validation and configured-bot replacement warning.
- **why:** Close a concrete interaction gap in the pinned Desktop quick-setup card while retaining profile-bound apply semantics.
- **components:** `apps/hermes-native/src/app.rs`, Telegram QR Messaging UI, focused tests and parity documentation.
- **type:** feature / messaging interaction parity
- **validation:** Allowlist mutation test and headless Messaging/accessibility fixture passed; full suite, all-target check, rustfmt, diff and pinned-source package validator pending final run.
- **performance:** Local UI-only operations; no network or benchmark impact.
- **risks:** Existing manual bot credential management, localization, live authenticated behavior and source-paired Windows acceptance remain open. PAR-0282 remains pending.
- **follow_up:** Continue remaining pinned Messaging behavior, then proceed to the next highest-impact unimplemented source route.

- **id:** HUR-001-172
- **timestamp:** 2026-11-16T21:41:36Z
- **what:** Reordered the Telegram QR flow into source-aligned setup and QR columns and placed Cancel with the relevant waiting or ready actions.
- **why:** Correct a major visible mismatch within the pinned Hermes Desktop Messaging quick-setup card.
- **components:** `apps/hermes-native/src/app.rs`, Messaging headless fixture and parity documentation.
- **type:** feature / visual interaction parity
- **validation:** Focused ready-state headless Messaging fixture and full all-target feature suite passed (203 library plus 15 protocol tests); all-target feature check, rustfmt, diff check and validator passed (491 capability rows pending).
- **performance:** No added redraw timers, polling or network requests.
- **risks:** Pixel-paired Windows and multi-DPI acceptance remain open; PAR-0282 stays pending.
- **follow_up:** Continue closing source-backed Messaging gaps, then move to the next unimplemented source route.

- **id:** HUR-001-173
- **timestamp:** 2026-11-16T21:51:24Z
- **what:** Replaced the unavailable Scheduled Jobs route with a profile-scoped authenticated job inventory, search, sorted list and selected-job detail surface.
- **why:** Close a first-party route gap against the pinned Hermes Desktop source.
- **components:** `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, scheduled-jobs route tests and parity docs.
- **type:** feature / route implementation
- **validation:** Remote profile/auth/DTO fixture and headless master/detail fixture pass; full suite passed (205 library plus 15 protocol tests); all-target check, format and diff passed; validator reports 491 capabilities pending.
- **performance:** Fetch runs on the existing bounded HTTP worker; no UI-thread networking or polling added.
- **risks:** Job mutation, run/history, delivery/blueprint workflows, live authentication and source-paired Windows acceptance remain open; PAR-0282 remains pending.
- **follow_up:** Implement source job actions and run history, then continue remaining first-party routes.

- **id:** HUR-001-174
- **timestamp:** 2026-11-16T21:57:42Z
- **what:** Added profile-scoped Scheduled Job Pause/Resume controls and authenticated empty-body POST transport with returned identity/state validation.
- **why:** Implement the first lifecycle action from the pinned Hermes Scheduled Jobs route.
- **components:** `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, action fixture and parity records.
- **type:** feature / scheduled-job lifecycle
- **validation:** Focused pause/resume transport and headless UI fixtures pass; full suite passed (206 library plus 15 protocol tests); all-target check, format and diff passed; validator reports 491 capabilities pending.
- **performance:** Uses existing async HTTP worker; no blocking UI or polling added.
- **risks:** Create/edit, trigger/stop, delete, run history, live authentication and source-paired Windows validation remain open.
- **follow_up:** Implement Trigger/Stop and run history, then create/edit/delete and blueprint flows.

- **id:** HUR-001-175
- **timestamp:** 2026-11-16T22:09:12Z
- **what:** Added profile-scoped Scheduled Job run-history reads and selected-detail presentation for the latest 20 records.
- **why:** Match the pinned Hermes Desktop job detail, which loads run history when a job is selected.
- **components:** Native gateway DTO/worker, request/profile/job correlation, Scheduled Jobs detail UI, HTTP and headless fixtures, parity plan and audit docs.
- **type:** feature / scheduled-job history
- **validation:** Remote HTTP fixture verifies encoded job path, exact limit/profile query, token header, unique bounded rows and duplicate rejection; headless fixture verifies rendered run title. Full suite passed: 207 library tests and 15 protocol tests. All-target cargo check passed before the final small retry-owner correction; rerun final check/format/package validation.
- **performance:** Uses existing bounded HTTP worker; one request per selected job, no polling or UI-thread networking.
- **risks:** Trigger/Stop, clickable session navigation, create/edit, delete, delivery/blueprints, live gateway proof and source-paired Windows acceptance remain open; no parity row advanced.
- **follow_up:** Implement source-compatible Run now/Stop and run-to-session navigation, then job editing/deletion and remaining route workflows.

### HUR-001-175 validation addendum — 2026-11-16T22:12:38Z
- **what:** Final validation completed after the run-history owner-fencing change.
- **validation:** Full all-target feature suite passed (207 library + 15 protocol tests); all-target cargo check, rustfmt check, git diff check and fixed-source package validator passed. Validator reports 491 pending capabilities.
- **follow_up:** Continue with Run now/Stop and session navigation. PAR-0282 remains pending; no requirement was promoted.

- **id:** HUR-001-175-VALIDATION
- **timestamp:** 2026-11-16T22:12:38Z
- **what:** Recorded final run-history validation after the owner-fencing correction.
- **why:** Close the validation follow-up recorded with the implementation entry and preserve the exact final suite evidence.
- **components:** Hermes native Cron gateway fixture, headless Scheduled Jobs fixture, Rust feature suite and pinned-source package validator.
- **type:** validation / scheduled-job history
- **validation:** 207 library tests and 15 protocol tests passed; all-target cargo check, rustfmt check, git diff check and fixed-source package validator passed. The validator reports 491 pending capability rows; no source acceptance row or PAR-0282 state was advanced.
- **performance:** No benchmark change; run-history retrieval is one request on the existing bounded worker and adds no polling.
- **risks:** Live gateway behavior, source-paired screenshots, trigger/stop, run navigation and remaining job management are still unverified or incomplete.
- **follow_up:** Continue with Run now/Stop, session navigation, and remaining source-pinned routes.

- **id:** HUR-001-175-FINAL
- **timestamp:** 2026-11-16T22:14:55Z
- **what:** Revalidated Scheduled Job run history after the final owner/error-state fence.
- **why:** Ensure the committed working result remains testable after final state handling was adjusted.
- **components:** Hermes native Scheduled Jobs app, gateway transport, HTTP fixture, headless UI fixture and fixed-source validator.
- **type:** validation / scheduled-job history
- **validation:** 207 library tests and 15 automation protocol tests passed; all-target check, rustfmt, git diff check and pinned-source package validator passed. The validator reports 491 pending capabilities.
- **performance:** No change from previous entry; no polling or UI-thread networking was added.
- **risks:** Full source parity, trigger/stop, session navigation, live gateway proof and source-paired Windows acceptance remain incomplete; no source requirement was advanced.
- **follow_up:** Continue with Run now/Stop and session navigation, then the remaining pinned-source routes.

- **id:** HUR-001-176
- **timestamp:** 2026-11-16T22:40:11Z
- **what:** Added profile-scoped Scheduled Job Run now/Stop with independently completing trigger and cancellation, live-state polling, queued-history reconciliation and execution-aware stop outcomes.
- **why:** Implement the pinned Desktop's selected-job run controls while allowing Stop to complete even when a trigger request is still open.
- **components:** Hermes Native gateway trigger actor, HTTP command worker, Scheduled Jobs app state/UI, HTTP fixture and headless route fixture.
- **type:** feature / scheduled-job run control
- **validation:** Full feature suite passed (208 library + 15 protocol tests); all-target feature cargo check passed; rustfmt ran. Focused HTTP fixture confirms cancel completes before a held trigger response; headless UI fixture covers Running, Stop, Queued and run history. Package validator, final rustfmt check and diff check are recorded in the validation addendum below.
- **performance:** Trigger work is isolated from the shared HTTP worker with a bounded capacity of eight; live list polling is 1.5 seconds while active, and history polls at 1 second while queued then 8 seconds normally.
- **risks:** Live gateway, source-paired Windows visuals, run-to-session navigation, create/edit/delete and blueprints/delivery remain open. PAR-0282 and all source acceptance rows remain pending.
- **follow_up:** Validate package and repository integrity; then add run-to-session navigation and remaining job lifecycle actions.

- **id:** HUR-001-177
- **timestamp:** 2026-11-16T22:40:11Z
- **what:** Added a workstream-level completion overview that distinguishes partial implementation, headless evidence, live-runtime proof and strict source-paired acceptance.
- **why:** The prior 0/499 acceptance-only counter omitted useful partial engineering progress and the 491 supplemental validator was easy to misread as a completion score.
- **components:** Hermes Native completion documentation, plan, backlog and README.
- **type:** documentation / progress reporting
- **validation:** Claims were checked against the current plan/backlog and source audit; final feature suite, all-target cargo check, rustfmt check, diff check and supplemental package validator are recorded in execution logs. The 0/499 acceptance count is unchanged.
- **performance:** No runtime change.
- **risks:** This is a curated snapshot, not a generated dashboard. The Nov 12 fixed 499-row scope and Nov 13 supplemental 491-row package still need a row-by-row crosswalk before row-level partial counts or an aggregate percentage can be trusted.
- **follow_up:** Reconcile source inventories, add requirement-level dimensions and evidence locators, and update the overview after substantial feature changes.

### HUR-001-176/177 validation addendum — 2026-11-16T22:42:36Z
- **what:** Final verification for Run now/Stop and the completion overview.
- **validation:** `cargo test -p hermes-native --features agent-automation` passed: 208 library tests and 15 protocol tests. `cargo check -p hermes-native --all-targets --features agent-automation`, `cargo fmt --all -- --check`, and `git diff --check` passed. The supplemental reference validator passed and reported 491 pending rows. One initial `--exact` filter selected zero tests; rerun matched the intended tests. A headless UI fixture initially cleared its rows on first-frame polling; its test polling deadline was fixed and the fixture then passed.
- **performance:** No benchmark change; trigger isolation and bounded polling parameters are recorded in HUR-001-176.
- **risks:** Validation is local/fixture-based. Live Hermes runtime and source-paired visual acceptance remain unverified; strict parity remains 0/499.
- **follow_up:** Reconcile the 499 fixed-scope and 491 supplemental inventories before publishing row-level partial counts; continue the pinned source implementation.

- **id:** HUR-001-178
- **timestamp:** 2026-11-16T22:57:19Z
- **what:** Added a reproducible 599-relation crosswalk and a 499-row evidence-backed implementation overlay. Corrected stale Scheduled Jobs acceptance-ID references in the active status: Cron uses PAR-0300–0309; PAR-0282 is Messaging.
- **why:** The supplemental 491-row package omits 42 rows still in the fixed source scope and adds 34 new requirements. Partial implementation needs exact pinned IDs and evidence rather than a binary package validator count.
- **components:** `apps/hermes-native/tools/build_requirement_crosswalk.py`, `apps/hermes-native/tools/build_implementation_progress.py`, `apps/hermes-native/docs/status/REQUIREMENT_CROSSWALK.csv`, `apps/hermes-native/docs/status/IMPLEMENTATION_PROGRESS.csv`, completion overview, active backlog and plan correction.
- **type:** evidence / scope crosswalk and implementation status
- **validation:** Both generators validate unique IDs, source pins, join counts and evidence locators before atomic output. Current implementation counts: 5 implemented locally, 6 partial, 1 not implemented, 487 unknown. All 499 fixed source-pair results remain pending.
- **performance:** Offline deterministic CSV generation; no app/runtime change.
- **risks:** The 457 exact capability-ID matches are structural candidates only; semantic review and all evidence transfer remain pending. The 12 seeded implementation rows do not imply live or source-paired acceptance.
- **follow_up:** Expand evidence review over the remaining 487 rows; preserve source acceptance gates and continue missing Hermes UI behavior.

- **id:** HUR-001-179
- **timestamp:** 2026-11-16T23:42:35Z
- **what:** Added the Scheduled Jobs editor lifecycle: profile-scoped create/edit/delete, delivery targets, model options, typed automation blueprints, script-only-safe updates, catch-up choices, last-result labels and confirmed deletion. Updated the evidence overlay and completion overview from the tested implementation evidence.
- **why:** Close the next concrete Scheduled Jobs parity gaps in the pinned Hermes Desktop snapshot and expose partial implementation state accurately.
- **components:** `apps/hermes-native/src/app.rs`, `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/tools/build_implementation_progress.py`, Scheduled Jobs plan/backlog/architecture/README/status artifacts, root backlog/architecture/README.
- **type:** feature / evidence-backed progress
- **validation:** Full `cargo test --features agent-automation` passed (217 library tests + 15 automation protocol tests); all-target feature check and format check passed. Focused Cron HTTP coverage passed (9 tests); Scheduled Jobs headless form/blueprint/delete tests passed. Fixed and supplemental package validators passed; both status generators reproduced byte-identical output; diff check passed. Current reviewed overlay: 6 implemented, 9 partial, 484 unknown; all 499 source-pair rows remain pending. UCM YAML parsing was skipped because PyYAML is unavailable.
- **performance:** No benchmark performed or claimed. Cron HTTP requests remain on the bounded gateway HTTP worker; existing trigger isolation is unchanged.
- **risks:** Authenticated live Hermes and source-paired Windows visuals remain unverified. PAR-0302's matrix says timezone editing but pinned Desktop exposes no timezone field. Run-to-session navigation, localization and PAR-0309 remain open.
- **follow_up:** Test the complete create/edit/delete flows against an isolated authenticated Hermes runtime, capture matched pinned/native states, resolve the PAR-0302 contract discrepancy, and continue the remaining 484 unreviewed requirements.

- **id:** HUR-001-180
- **timestamp:** 2026-11-16T23:52:03Z
- **what:** Reviewed fixed-source P0 session/window requirements PAR-0002 through PAR-0006 and expanded the evidence overlay. Marked separate native window creation explicitly not implemented; existing new-session, close, restore and reorder behavior is partial.
- **why:** Replace unknown/binary status with current implementation and evidence gaps, and identify a high-priority missing capability for the 1:1 replacement.
- **components:** `apps/hermes-native/tools/build_implementation_progress.py`, `apps/hermes-native/docs/status/IMPLEMENTATION_PROGRESS.csv`, `apps/hermes-native/docs/COMPLETION_OVERVIEW.md`, native/root backlogs and plan.
- **type:** evidence review / progress correction
- **validation:** Evidence locator generator passed over all 20 reviewed rows and the fixed 499-row pin; all source-pair results remain pending. Headless runtime tests were not run because this slice changes only evidence/documentation.
- **performance:** No runtime change or benchmark claim.
- **risks:** eframe root-window code inspection proves the current host lacks a child-window registry but does not prove native window behavior on Windows. The 20 reviewed rows remain a small sample of 499.
- **follow_up:** Implement and validate native child windows, then review PAR-0010/0011; keep viewport ownership separate from the shared Hermes session/event lifecycle.

- **id:** HUR-001-181
- **timestamp:** 2026-11-17T00:17:12Z
- **what:** Added an Open new window toolbar/action path that starts an isolated Hermes Native process with the active route/profile. Each process uses its own Dock layout key and preference file while sharing gateway credentials. Enabled `serde_json/float_roundtrip` and added separate-window layout reload coverage.
- **why:** Advance P0 PAR-0002 and prevent child windows from sharing a mutable UI loop, Dock snapshot or preference file with the parent. Fix the checksum failure observed when persisted split weights were parsed and reloaded.
- **components:** `apps/hermes-native/src/app.rs`, `src/layout.rs`, `src/main.rs`, `Cargo.toml`, `docs/COMPLETION_OVERVIEW.md`, `docs/PLAN.md`, `docs/BACKLOG.md`, `docs/ARCHITECTURE.md`, `README.md`, progress generator and ledger.
- **type:** feature / persistence fix / evidence update
- **validation:** Focused all-target `cargo test --features agent-automation window` passed (3 library matches and 2 binary argument tests); the layout test reloads both main and child checksummed layouts and verifies isolated preferences. Native OS window creation was not launched in this headless pass.
- **performance:** No benchmark performed or claimed. Each separate window is a process with its own gateway connection.
- **risks:** Physical Windows launch/close, duplicate gateway connection behavior, cross-window pane transfer, session-event synchronization, child recovery and source-paired acceptance remain open. PAR-0002 remains pending.
- **follow_up:** Validate visible native window creation on Windows, then implement pane transfer and cross-window event synchronization for PAR-0010/0011 without changing the installed Hermes app.

## HUR-001-178 — 2026-11-17

- **id:** HUR-001-178
- **timestamp:** 2026-11-17T01:06:48Z
- **what:** Implemented portable checkpoint, ownership lease, provider-handoff preview/resume, explicit reconciliation, and evidence-separated progress reporting. Bootstrapped canonical `.doofus/` records for this checkout and updated parity reporting to keep the user-supplied 33/59 measure separate from the fixed 499-row source scope.
- **why:** Preserve task continuity across Codex, Antigravity and Hermes while showing partial engineering status without conflating it with accepted 1:1 UI parity.
- **components:** `.doofus/`, `apps/hermes-native/tools/doofus_task.py`, its isolated tests and schemas, `docs/AGENT_HANDOFF.md`, `docs/architecture/AGENT_TASK_CONTRACTS.md`, progress and backlog documentation.
- **type:** workflow foundation and progress documentation
- **validation:** 13 Python tests passed; Rust feature suite passed (220 library, 15 protocol, 2 launcher); versioned state validated; fixed-source progress generator passed (499 rows); cargo fmt and git diff checks passed. Source acceptance remains 0/499.
- **performance:** No runtime performance claim; ETA stays unavailable because phase-specific samples are insufficient.
- **risks:** Provider transfer is manual; a preview is not a delivery receipt. Live runtime, physical Windows multi-window behavior and source-paired visual acceptance remain open.
- **follow_up:** Keep the active lease, checkpoint verified work with exact evidence locators, then proceed through the pinned 499 requirements by dependency and user-visible impact.

## HUR-001-179 — 2026-11-17

- **id:** HUR-001-179
- **timestamp:** 2026-11-17T01:10:48Z
- **what:** Corrected handoff preview reporting and added an output-level regression test.
- **why:** The preview was non-mutating, but its CLI result said `handed_off` and showed a null recipient, which could falsely imply ownership transfer.
- **components:** `apps/hermes-native/tools/doofus_task.py` and `apps/hermes-native/tools/test_doofus_task.py`.
- **type:** correctness fix
- **validation:** Python syntax check passed; all 14 isolated provider-handoff and recovery tests passed; the new assertion confirms owner/revision remain unchanged.
- **performance:** Negligible CLI formatting and test-only changes.
- **risks:** None identified; provider/API transfer remains manual by design.
- **follow_up:** Preserve preview versus actual handoff semantics in any provider adapter; continue pinned UI parity under the active Codex lease.

## HUR-001-180 — 2026-11-17

- **id:** HUR-001-180
- **timestamp:** 2026-11-17T01:11:40Z
- **what:** Strengthened the PAR-0003 evidence mapping against the pinned source and corrected a stale reviewed-row count in the completion overview.
- **why:** Preserve a useful partial state with exact evidence and explicit remaining behaviors.
- **components:** `apps/hermes-native/tools/build_implementation_progress.py`, `apps/hermes-native/docs/status/IMPLEMENTATION_PROGRESS.csv`, `apps/hermes-native/docs/COMPLETION_OVERVIEW.md`.
- **type:** evidence review and documentation correction
- **validation:** The progress generator validated the source/native locators; PAR-0003 remains partial, total reviewed rows remain 21, and all 499 source-acceptance rows remain pending.
- **performance:** No runtime impact.
- **risks:** Bot Mode route/fallback, first-message listing proof, live gateway behavior, and paired Windows acceptance remain open.
- **follow_up:** Continue reviewing fixed-scope rows by source behavior and evidence; do not infer accepted parity.

## HUR-001-181 — 2026-11-17

- **id:** HUR-001-181
- **timestamp:** 2026-11-17T01:21:24Z
- **what:** Tightened Scheduled Jobs blueprint defaults and time-field fidelity, including missing/invalid-time rejection, scalar weekday selection and preservation of a saved model omitted from the refreshed catalog.
- **why:** Avoid silently changing source defaults or saving malformed schedule data.
- **components:** `apps/hermes-native/src/app.rs`, progress evidence generator/CSV, plan, completion overview, backlog and native README.
- **type:** parity correctness fix
- **validation:** Focused blueprint tests passed 2/2, saved unavailable-model UI test passed 1/1, formatter passed and progress locator generator passed. PAR-0301/0302 remain partial; 0/499 source acceptance.
- **performance:** Local validation only; no runtime performance claim.
- **risks:** Live gateway, source-paired visuals and PAR-0302 timezone contract discrepancy remain open.
- **follow_up:** Run full native suite with the next window lifecycle slice; continue source-paired acceptance.

## HUR-001-182 — 2026-11-17

- **id:** HUR-001-182
- **timestamp:** 2026-11-17T01:36:53Z
- **what:** Added child-window first-frame readiness receipts and guarded session moves; corrected Scheduled Jobs blueprint defaults/time validation; expanded requirement evidence for PAR-0002, PAR-0010 and PAR-0301/0302 while preserving partial states.
- **why:** Treat process startup as distinct from a usable window and prevent silent schedule-field changes; keep engineering progress distinct from fixed-source acceptance.
- **components:** `apps/hermes-native/src/native_window.rs`, native app/CLI entry points, progress generator/CSV, plan, completion overview, architecture, backlogs, READMEs and continuity status.
- **type:** parity behavior and evidence-quality improvements
- **validation:** Full Rust suite passed (230 library, 15 automation protocol, 3 launcher tests); 14 Python handoff tests passed; targeted readiness and Scheduled Jobs tests passed; formatter and diff checks passed; fixed 499-row progress generator and handoff-state validator passed.
- **performance:** Readiness receipt is bounded to 512 bytes; local poll interval is 100 ms and deadline is 20 seconds. No user-facing latency benchmark was run.
- **risks:** Actual Windows window creation/move, cross-window event synchronization, authenticated live gateway behavior and source-paired visual acceptance remain open. PAR-0002/0010 remain partial; strict acceptance remains 0/499.
- **follow_up:** Implement profile-fenced cross-window session/event synchronization for PAR-0011, then complete physical Windows and source-paired acceptance across the fixed 499 rows.

## HUR-001-183 — 2026-11-17

- **id:** HUR-001-183
- **timestamp:** 2026-11-17T15:25:55Z
- **what:** Implemented PAR-0011 backend `sessions.changed` refresh with profile/request fencing, burst coalescing and one trailing refresh; replaced refreshed-profile rows from the returned snapshot, retained other profile rows and recovered stale in-flight state after gateway profile changes. Added deterministic headless regression coverage and advanced the evidence ledger to 22 reviewed rows.
- **why:** Keep separate native Hermes windows synchronized with backend session-list changes without applying stale results or retaining removed rows.
- **components:** `apps/hermes-native/src/app.rs`, `src/gateway.rs`, progress generator/CSV, plan, architecture, completion overview, backlogs, READMEs, and `.doofus/` checkpoint/task ledger.
- **type:** parity behavior and resumability improvement
- **validation:** Focused Rust 5/5; full Rust 232 library + 15 automation protocol + 3 launcher tests; Python handoff 14/14; all-target check, format, diff, progress generator and handoff-state validation passed. PAR-0011 remains partial; source acceptance remains 0/499.
- **performance:** Uses existing background HTTP reader and a single in-flight refresh plus at most one trailing refresh; no benchmark requested or claimed.
- **risks:** Local peer-window `hermes:sessions` broadcasts, open transcript/project reconciliation, focus/reconnect backstops, physical Windows behavior, live service behavior and source-paired acceptance remain open.
- **follow_up:** Implement the remaining PAR-0011 synchronization behaviors and then validate physical child-window lifecycle and source-paired Windows workflows.

## HUR-001-184 — 2026-11-17

- **id:** HUR-001-184
- **timestamp:** 2026-11-17T16:36:51Z
- **what:** Added a bounded cross-process session-list hint bus using opaque per-user event receipts; native peers re-read the profile-scoped canonical gateway list. Successful create, rename and matching transcript completion publish receipts. Added transport and app-level headless coverage and updated parity progress documentation.
- **why:** Match the pinned `hermes:sessions` invalidation behavior across independent native processes without sending session data or credentials in the local signal.
- **components:** `src/session_peer_sync.rs`, `src/lib.rs`, `src/app.rs`, `docs/PLAN.md`, `docs/SOURCE_AUDIT.md`, `docs/ARCHITECTURE.md`, `docs/COMPLETION_OVERVIEW.md`, `docs/BACKLOG.md`, `README.md`, and root project journals.
- **type:** parity behavior and local synchronization
- **validation:** Full Rust feature suite passed (243 library, 15 automation protocol, 3 launcher tests); a separate-process receipt test passed; 14 continuity tests passed; all-target cargo check, rustfmt and diff checks passed; progress generator passed (23/499 reviewed, 0/499 accepted); `.doofus` state validated. The optional fixed-source package validator was not present in this checkout and was not run. PAR-0011 remains partial.
- **performance:** Native windows rescan a capped (512 final receipts) local directory every 250 ms while open; no CPU/latency benchmark or physical window interaction test was run.
- **risks:** The existing `%APPDATA%\HermesNative` parent ACL grants the current user, SYSTEM and Administrators full control and the Codex sandbox group read/execute only. Physical parent/child window delivery and authenticated live behavior remain unverified. Process suspension longer than the bounded event retention can miss individual pings; periodic/initial canonical refresh must close that gap. Archive/pin and additional source mutation callers, transcript/tile/project peer sync, focus/reconnect and source-paired acceptance remain open.
- **follow_up:** Inspect and wire remaining source mutation callers, then implement `hermes:transcript` tile reconciliation and verify the actual Windows parent/child interaction.

## HUR-001-185 — 2026-11-17

- **id:** HUR-001-185
- **timestamp:** 2026-11-17T17:06:00Z
- **what:** Implemented typed transcript-change peer events and request-fenced latest-page refresh for matching active routes and open profile-owned Dock session panes. Refreshed PAR-0011 evidence without changing the fixed 499-row scope.
- **why:** Synchronize independent Hermes Native windows after a completed turn while retaining the backend as canonical and preserving resumable provider-neutral task state.
- **components:** `apps/hermes-native/src/session_peer_sync.rs`, `src/app.rs`, `src/gateway.rs`, `src/layout.rs`, progress generator/CSV, plan, source audit, architecture, completion overview, backlogs, READMEs and root/crate journals.
- **type:** parity behavior and continuity validation
- **validation:** Full Rust feature suite passed (245 library, 15 automation protocol, 3 launcher tests); transport tests 5/5; tile refresh test 1/1; Python handoff suite 14/14 including simulated Codex → Antigravity → Hermes without transcript transfer; all-target check, formatter, progress generator (23/499 reviewed; 0/499 accepted), and `git diff --check` passed. PAR-0011 remains partial.
- **performance:** Uses the existing 250 ms peer poll and background gateway HTTP reader; no runtime benchmark was performed.
- **risks:** Physical Windows peer delivery, authenticated live gateway behavior, project-tree/focus recovery and source-paired acceptance remain unverified. ETA is unavailable due insufficient phase samples; critical path is not yet mapped from authoritative dependency data.
- **follow_up:** Continue the next dependency-ready PAR-0011 gap; keep physical Windows, live gateway and source-pair proof as separate evidence gates.

## HUR-001-186 — 2026-11-17

- **id:** HUR-001-186
- **timestamp:** 2026-11-17T17:10:00Z
- **what:** Expanded isolated provider-handoff validation to exercise all six directed Codex/Antigravity/Hermes pairings, requiring relinquishment followed by an explicit recipient resume for each transfer. Updated the portable handoff and architecture contracts with the test boundary.
- **why:** Verify that task continuity does not depend on one provider ordering or pass conversation history between agents.
- **components:** `apps/hermes-native/tools/test_doofus_task.py`, `docs/AGENT_HANDOFF.md`, `docs/architecture/AGENT_TASK_CONTRACTS.md`, root/crate change and execution journals.
- **type:** task continuity validation
- **validation:** Full Python continuation suite passed 15/15, including the six-pair transfer test and Codex → Antigravity → Hermes state-preservation simulation. Rust validation for HUR-001-185 remains 245 library + 15 automation protocol + 3 launcher tests. Live provider delivery and actual takeover are not claimed.
- **performance:** Temporary Git repositories only; no product runtime benchmark.
- **risks:** Provider APIs are not integrated; recipient delivery remains a copyable manual prompt and requires an explicit resume receipt.
- **follow_up:** Continue the pinned 499-row migration and retain the manual handoff workflow until a provider connector is explicitly implemented and verified.

## HUR-001-187 — 2026-11-17

- **id:** HUR-001-187
- **timestamp:** 2026-11-17T17:23:47Z
- **what:** Added throttled focus-regain recovery for the canonical session list and active/open transcript refresh, plus transcript recovery after gateway reconnect. Added a focused deterministic unit test and planned the remaining project-tree sync as PAR-0011 revision 185.
- **why:** Continue the pinned Hermes focus/reconnect behavior without changing the 499-row scope or treating local test evidence as live UI acceptance.
- **components:** `apps/hermes-native/src/app.rs`, `apps/hermes-native/docs/PLAN.md`, `apps/hermes-native/docs/COMPLETION_OVERVIEW.md`, `apps/hermes-native/docs/BACKLOG.md`, root execution/change journals.
- **type:** parity behavior
- **validation:** Focused test `focus_recovery_is_edge_triggered_throttled_and_gateway_scoped` passed. Full suite and remaining repository checks are pending in this checkpoint.
- **performance:** Adds no periodic work; checks focus state during frames and performs a gateway refresh only on a throttled focus edge or reconnect. No runtime benchmark recorded.
- **risks:** Real OS focus/wake delivery, status rehydration, project-tree reconciliation, live gateway, Windows peer behavior and source-paired acceptance remain open.
- **follow_up:** Complete source-paired focus/reconnect state handling, then implement project-tree event/backstop refresh and stale-response guards under PAR-0011 revision 185.

### HUR-001-187 validation continuation (2026-11-17T17:25:59Z)

The full native feature suite passes: 246 library, 15 automation protocol, and 3 launcher tests. The focused focus-recovery test is included. The portable continuity suite passes 15/15; all-target `cargo check`, rustfmt check, progress locator generation (23/499 reviewed; 0/499 accepted), and task checkpoint validation pass. `git diff --check` exits successfully with only existing CRLF conversion warnings. The pinned-source 499-row denominator and all acceptance states remain unchanged.

## HUR-001-188 — 2026-11-17

- **id:** HUR-001-188
- **timestamp:** 2026-11-17T17:42:52Z
- **what:** Implemented typed profile-scoped active-session status rehydration and accessible working/waiting indicators; added `projects.changed` invalidation for the existing color projection only. Added deterministic parser, exact RPC, stale response, profile-reap, event-trigger, project invalidation and accessible UI coverage.
- **why:** Recover source-visible active-turn state across reconnects and events while explicitly preserving the remaining structural project-tree gap and 499-row scope.
- **components:** `apps/hermes-native/src/gateway.rs`, `src/app.rs`, `docs/PLAN.md`, `docs/SOURCE_AUDIT.md`, `docs/ARCHITECTURE.md`, `docs/BACKLOG.md`, `docs/COMPLETION_OVERVIEW.md`, root architecture and change/execution journals.
- **type:** parity behavior
- **validation:** Focused parser, wire-contract, app snapshot/fencing, project-change and accessible-row tests pass. Full-suite/build/continuity checks are being recorded in the accompanying execution log.
- **performance:** Reads are coalesced and capped at one focused 30-second backstop; RPC timeout is 10 seconds. No runtime benchmark recorded.
- **risks:** Unsupported old gateway methods preserve cached state but do not yet reproduce source fallback/settle behavior; actual live status, OS focus/wake, structural project tree and source-paired acceptance remain open.
- **follow_up:** Complete project tree/list structure and ownership/fencing, then validate through live gateway and matched source/native Windows flows.

## HUR-001-189 — 2026-11-17

- **id:** HUR-001-189
- **timestamp:** 2026-11-17T17:56:43Z
- **what:** Added bounded typed profile-scoped `projects.tree` parsing and refresh, a collapsible project → repository → group → session hierarchy in the Sessions sidebar, and accessible actions to open source-owned preview/group sessions. Added generation/profile/connection/cache-preservation coverage and refreshed PAR-0011 implementation evidence locators.
- **why:** Close the first structural project-tree gap while improving partial completion reporting with precise implementation evidence and explicit remaining source/live dimensions.
- **components:** `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, `apps/hermes-native/tools/build_implementation_progress.py`, PAR-0011 plan/source/architecture/backlog/overview, root architecture/backlog.
- **type:** parity behavior and progress evidence
- **validation:** Focused exact RPC/parser and request/profile/generation-fence tests pass; all-target feature suite and continuity suite remain to be rerun for this change. Implementation overlay stays 23/499 reviewed, 6 implemented, 17 partial, 476 unknown; formal acceptance 0/499.
- **performance:** Project tree read has a 60-second RPC timeout; previews are limited to three per project. No live service benchmark recorded.
- **risks:** All-profile ownership, project-list consistency, repo discovery, deletion tombstones, live gateway, physical Windows behavior and paired visuals are not implemented/verified.
- **follow_up:** Run complete automated validation, update the current durable checkpoint, then continue PAR-0011 against the pinned live project/list/focus contracts.

HUR-001-189 validation addendum (2026-11-17T18:02:52Z): full validation passed: 254 library, 15 automation protocol, and 3 launcher tests; Python continuity 15/15; all-target feature check; formatter; progress-locator generator; local handoff validation; and diff check. The durable local checkpoint is revision 33 and remains partial, with 23/499 evidence-reviewed and 0/499 source accepted.

## HUR-001-190 — 2026-11-17

- **id:** HUR-001-190
- **timestamp:** 2026-11-17T18:39:07Z
- **what:** Added source-shaped refresh of the entered project when its overview node changes and profile-scoped archive/restore lifecycle generations that filter stale project-session responses.
- **why:** Prevent stale hydrated project details and archived session resurrection during asynchronous `projects.project_sessions` reads.
- **components:** `apps/hermes-native/src/app.rs`, `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/tools/build_implementation_progress.py`, PAR-0011 plan/source audit/architecture/backlog/overview, root architecture/backlog/README.
- **type:** parity behavior and evidence reporting
- **validation:** Full suite passed: 259 Rust library, 15 automation protocol, 3 launcher tests; continuity suite 15/15; all-target feature check; formatter; progress locator builder; local task-state validation; fixed 565-row package validator; `git diff --check`.
- **performance:** Project session read stays bounded by the existing gateway timeout; no live network benchmark was recorded.
- **risks:** All-profile hydration and 50-row preview paging remain unimplemented; archive behavior is covered, but non-archive deletion tombstones, live gateway, physical Windows behavior and source-paired acceptance remain open.
- **follow_up:** Implement source-faithful all-profile project entry and preview paging, then project-list consistency/repository discovery; continue paired live and Windows acceptance without changing 499-row scope.


## HUR-001-191 — 2026-11-17

- **id:** HUR-001-191
- **timestamp:** 2026-11-17T19:00:00Z
- **what:** Implemented pinned-source project overview “Show all” behavior: profile-scoped hydration, local all-profile aggregate entry, lifecycle-filtered merge, accessible 50-row paging, and retry/loading states. Updated PAR-0011 source evidence paths to the pinned `8783054` snapshot and refreshed progress and completion documentation.
- **why:** Complete the next reviewable Sessions → Projects slice while preventing synthetic all-profile RPCs and archived rows from appearing.
- **components:** `apps/hermes-native/src/app.rs`, `apps/hermes-native/tools/build_implementation_progress.py`, `apps/hermes-native/docs/status/IMPLEMENTATION_PROGRESS.csv`, PAR-0011 plan/backlog/architecture/overview, root README/handoff/architecture.
- **type:** parity behavior and evidence reporting
- **validation:** Focused Show all tests passed; full suite passed: 262 Rust library, 15 automation protocol, 3 launcher tests; continuity 15/15; all-target cargo check; rustfmt; progress locator builder; local task validator; 565-row pinned package validator; `git diff --check`.
- **performance:** Project-session read remains bounded by existing gateway limits; no live service benchmark was recorded.
- **risks:** Full result-ceiling behavior, projects.list consistency, dedicated all-profile nested hydration, repository discovery, non-archive deletion, live runtime/Windows behavior, and source-paired acceptance remain open.
- **follow_up:** Continue PAR-0011 with project-list consistency, full pinned-source result ceiling, repository discovery and deletion tombstones, then pursue authenticated live and source-paired Windows evidence.


## HUR-001-192 — 2026-11-17

- **id:** HUR-001-192
- **timestamp:** 2026-11-17T19:23:54Z
- **what:** Added the pinned-source global “Show all sessions” preference, persisted it in native UI preferences, and wired the accessible Sessions → Projects control to request preview limits 3/2000 in single-profile and all-profile tree paths.
- **why:** Match the downloaded snapshot's global project-preview behavior in addition to the per-project 50-row hydration paging already implemented.
- **components:** `apps/hermes-native/src/layout.rs`, `apps/hermes-native/src/app.rs`, `apps/hermes-native/src/gateway.rs`, PAR-0011 progress builder and status CSV, completion/plan/source-audit/architecture/backlog docs, root and native README.
- **type:** parity behavior and persistence
- **validation:** Full suite passed: 265 Rust library + 15 automation protocol + 3 launcher tests; continuity 15/15; all-target cargo check; rustfmt; progress builder; task validator; pinned 565-row package validator; `git diff --check`. Focused tests cover persisted preference, active-profile 3/2000 UI refresh, aggregate 2000 preview request, and expanded tree rendering.
- **performance:** Opt-in tree preview widens from 3 to 2000 per the pinned contract. No live-service latency or production memory benchmark was recorded.
- **risks:** Very large preview trees can increase native UI work; project-list metadata consistency, repo discovery, non-archive deletion, live behavior and source-paired acceptance remain open.
- **follow_up:** Implement `projects.list` metadata/active-pointer reconciliation against the tree, with profile/connection/generation fencing and stale/error fixtures; then continue repository discovery and deletion semantics.


## HUR-001-193 — 2026-11-17

- **id:** HUR-001-193
- **timestamp:** 2026-11-17T19:46:40Z
- **what:** Added the pinned profile-scoped `projects.list` read and bounded parser; refreshed on connection, matching project-change events and focus recovery; fenced receipts by request, generation, endpoint instance and authority epoch; preserved same-owner cache on failures; and displayed the list-owned active pointer separately from the structural tree.
- **why:** Match the pinned active-project behavior without merging distinct source snapshots or letting stale profile/gateway results affect the project overview.
- **components:** app/gateway, PAR-0011 plan/source audit/architecture/backlog/completion status, progress evidence builder, root/native README/architecture/backlog, checkpoint.
- **type:** parity behavior and evidence reporting
- **validation:** `cargo test -p hermes-native --all-targets --features agent-automation --quiet` passed 270 library + 15 automation-protocol + 3 launcher tests; `python apps/hermes-native/tools/test_doofus_task.py` passed 15/15; all-target feature `cargo check` passed; formatter passed; progress evidence builder passed with 23 reviewed / 6 implemented / 17 partial / 476 unknown / 0 accepted; pinned package validation passed 565 rows, 14 source receipts and 2 implementation receipts (optional PyYAML unavailable); task validation passed at revision 39; `git diff --check` passed.
- **performance:** Adds one bounded list read on connect, matching project-change and focus-recovery paths; no live-service benchmark was recorded.
- **risks:** Live gateway, physical Windows focus, full metadata reconciliation, aggregate completeness, repository discovery, non-archive deletion, source-paired visuals and formal acceptance remain open.
- **follow_up:** Continue PAR-0011 with repository discovery and deletion semantics, then live/source-paired validation; preserve the 499-row scope.


## HUR-001-194 — 2026-11-17

- **id:** HUR-001-194
- **timestamp:** 2026-11-17T20:10:30Z
- **what:** Added explicit project deletion with an accessible confirmation, concrete-profile `projects.delete`, optimistic project-list/tree removal and separate active-pointer clearing, profile-keyed late-read tombstones, authoritative-tree reconciliation, and failure rollback including project drill-in restoration. Auto-discovered and no-project nodes cannot use this action.
- **why:** Match the pinned Hermes project menu/store behavior while preserving files and sessions and preventing cross-profile or stale-read mutations.
- **components:** `apps/hermes-native/src/app.rs`, `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/tools/build_implementation_progress.py`, PAR-0011 plan/source audit/architecture/backlog/completion overview, README files, progress overlay, checkpoint and execution journals.
- **type:** parity behavior and partial-evidence reporting
- **validation:** Focused deletion tests passed; full Rust suite passed 273 library + 15 automation-protocol + 3 launcher tests; provider continuity passed 15/15; all-target feature `cargo check` and formatter passed; progress builder kept 23 reviewed / 6 implemented / 17 partial / 476 unknown / 0 of 499 accepted; pinned package validator passed 565 behaviors, 565 UI items, 565 pending tests, 14 source receipts and 2 implementation receipts (optional PyYAML unavailable); task state validated at pre-checkpoint revision 44; `git diff --check` passed with existing LF-to-CRLF notices.
- **performance:** Adds one bounded delete RPC and one authoritative tree refresh after confirmed success; no live-service latency benchmark was recorded.
- **risks:** Delete behavior has local headless evidence only; live gateway, auto-project dismiss/Undo, repository discovery, Windows interaction, all-profile project parity and source-paired acceptance remain open. One initial focused test failed because the fixture used `is_auto` rather than pinned JSON `isAuto`; corrected fixture and retry passed.
- **follow_up:** Continue PAR-0011 with local repo discovery and auto-project dismiss/Undo; then verify authenticated live gateway and source-paired Windows behavior without changing the pinned 499-row scope.

### HUR-001-195 — 2026-11-17

- **id:** HUR-001-195
- **timestamp:** 2026-11-17T20:43:51Z
- **what:** Corrected phase-aware effort reporting so a numeric ETA requires three positive measured samples and positive size estimates for every remaining task. Missing task sizes now yield a null ETA and list exact task IDs; tested/verified/accepted tasks no longer count as remaining work. Documented handoff/progress limits and updated the partial completion overview.
- **why:** Preserve the user's requirement for evidence-backed partial progress and avoid a false zero-minute or complete-looking ETA when task size is unknown.
- **components:** doofus_task.py, isolated continuity tests, PLAN, ARCHITECTURE, BACKLOG, COMPLETION_OVERVIEW, README files, root architecture/backlog and execution/change journals.
- **type:** progress accuracy, task continuity
- **validation:** Provider continuity suite 16/16 passed, including three measured samples with a missing remaining size and the calculated EWMA after size evidence is added; fixed-scope progress generator passed at 23/499 reviewed, 6 implemented, 17 partial, 476 unknown, 0 accepted; handoff validator passed revision 49; machine-readable progress kept project ETA and critical path unavailable without evidence; git diff --check passed after removing one trailing blank line.
- **performance:** Local progress calculation remains a linear pass over 502 task nodes and 499 parity rows; no benchmark was requested or recorded.
- **risks:** Current repo has no normalized remaining task sizes, project start time, or dependency-backed critical path, so project-wide elapsed time, ETA, and critical path stay unavailable. Automatic process-boundary checkpoints and live external provider receipt/takeover remain unimplemented; isolated provider simulations do not prove live handoff delivery.
- **follow_up:** Add authoritative dependency and size evidence; add safe provider process/checkpoint triggers when process ownership can be observed; continue pinned Hermes parity under the same handoff protocol.

### HUR-001-196 — 2026-11-17

- **id:** HUR-001-196
- **timestamp:** 2026-11-17T20:54:17Z
- **what:** Audited local/remote repo discovery in the pinned Hermes source and wrote an ordered implementation plan with file targets and isolated acceptance cases.
- **why:** PAR-0011 currently lacks a native local scanner and remote host-discovery flow; source parity requires policy settings, bounded scanning, profile fencing and all-profile exclusion.
- **components:** Pinned projects.ts, Electron git-repo-scan.ts, sidebar triggers, Workspace config settings, host projects.discover_repos/projects.record_repos, native plan/source audit/architecture/backlog.
- **type:** source audit and implementation plan
- **validation:** Read-only audit against pinned snapshot 8783054ef6cbafd86f10892241436862c1171a66; no implementation or runtime acceptance claimed. Existing progress remains 23/499 reviewed and 0/499 accepted.
- **performance:** No product performance changes or benchmarks in this planning step.
- **risks:** Current native build cannot discover repos; the planned scanner must use configured roots only and must not scan the user's home during tests.
- **follow_up:** Implement PLAN Revision 195 through settings policy, local scanning, remote scanning and profile-safe refresh; keep all-profile discovery disabled.

### HUR-001-197 — 2026-11-17

- **id:** HUR-001-197
- **timestamp:** 2026-11-17T21:02:16Z
- **what:** Added and wired a deterministic local Git repository scanner for configured roots, including depth, normalization, exclusions, hidden/junk pruning and nested-repository boundary behavior.
- **why:** Begin the highest-priority PAR-0011 repository discovery gap from the pinned source without ever guessing or crawling the operator's home directory.
- **components:** `apps/hermes-native/src/repo_discovery.rs`, `apps/hermes-native/src/lib.rs`, PLAN Revision 195, SOURCE_AUDIT, BACKLOG, COMPLETION_OVERVIEW, execution journals.
- **type:** parity implementation slice
- **validation:** `cargo test -p hermes-native --lib repo_discovery -- --nocapture` passed 6/6 isolated temporary-fixture tests. No live gateway, settings UI, remote discovery, or strict source-pair acceptance is claimed.
- **performance:** Deterministic single-thread traversal bounds concurrent filesystem operations to one; caller integration must keep scanning off the egui frame thread. No benchmark recorded.
- **risks:** The scanner core is not yet invoked by the app; configured policy UI, profile-safe recording, remote-host scan and lifecycle triggers remain open. Worktree retains unrelated pre-existing edits.
- **follow_up:** Continue Revision 195 with profile-scoped config read/save, local worker and `projects.record_repos`; then add remote `projects.discover_repos`, generation fences and single-profile triggers.

Validation addendum (2026-11-17T21:04Z): the first repository-wide format check identified rustfmt differences in the new module. Two package-scoped format invocations used an incorrect workspace-relative path and changed nothing; formatting the owned module directly with Rust 2024 rustfmt resolved the issue. The complete workspace `cargo fmt --all -- --check` and the focused six-test scanner run then passed. `git diff --check` had already passed.

- id: hermes-par-0011-repo-discovery-integration-20261117
- timestamp: 2026-11-17T21:40:42Z
- what: Added profile-scoped repo-discovery settings/config transport, configured-root local worker and recording RPC, remote host scan path, generation/profile fencing, single-profile triggers and read-only all-profile behavior; repaired the all-profile UI test to assert observable no-load/no-control behavior.
- why: Continue the pinned-source PAR-0011 implementation and make the true partial state durable and reviewable.
- components: src/app.rs, src/gateway.rs, src/repo_discovery.rs, status/IMPLEMENTATION_PROGRESS.csv, PLAN, SOURCE_AUDIT, BACKLOG, COMPLETION_OVERVIEW, execution logs.
- type: parity implementation and progress evidence update.
- validation: 294 Rust library + 15 automation protocol + 3 launcher tests passed; continuity 16/16; focused scanner 7/7; config/local-scan 4/4; settings 2/2; cargo check, workspace format, pinned package validation, overlay generation, and diff check passed. No live gateway or source-paired acceptance claimed.
- performance: local scanning is off the UI thread; no reproducible benchmark measured.
- risks: remote live behavior, Windows-specific discovery/focus behavior, complete PAR-0011 reconciliation and all visual/source acceptance remain open.
- follow_up: inspect exact remaining tests/source requirements, exercise live profile-bound gateway where safely available, complete source-paired Windows states, update reviewed requirement evidence conservatively.

- id: hermes-par-0011-repo-discovery-refresh-20261117
- timestamp: 2026-11-17T21:50:58Z
- what: Forced a project-tree refresh after successful discovery even when an older tree is already marked loaded; retained cached tree during refresh and kept profile/request fencing; added remote wire-shape and app-level success/failure/stale-profile tests.
- why: Ensure newly recorded/discovered repository roots actually become visible and match the pinned refresh behavior.
- components: src/app.rs, src/gateway.rs, PLAN Revision 197, SOURCE_AUDIT, COMPLETION_OVERVIEW, execution logs.
- type: parity bug fix and regression coverage.
- validation: 298 library + 15 protocol + 3 launcher tests passed; focused gateway and app discovery tests passed. Live gateway and source-paired acceptance remain open.
- performance: No scanner timing benchmark; Rust traversal is currently sequential versus source concurrency 32.
- risks: Runtime visibility still depends on live host response; Windows and visual parity remain unverified.
- follow_up: Run remaining validation and continue PAR-0011; assess scanner throughput with reproducible benchmark before asserting latency parity.

Validation addendum (2026-11-17T21:52:39Z): the full feature suite passed 298 library + 15 automation protocol + 3 launcher tests; continuity passed 16/16; cargo check, workspace format, progress generation, pinned package validation, and git diff --check passed. Optional PyYAML validation was skipped because the package is not installed. Strict acceptance remains 0/499.

- id: hermes-parity-evidence-review-layout-pane-20261117
- timestamp: 2026-11-17T22:04:27Z
- what: Expanded the generator-backed fixed-scope evidence map from 23 to 34 reviewed rows, adding conservative partial classifications for layout profiles, Simple/Advanced arrangements, session pane lifecycle, context menus, active-tab persistence, and close policy; refreshed current progress summary and backlog counts.
- why: Give the operator useful partial status while preserving the strict 499-row parity gate and not treating source/tests as live or visual acceptance.
- components: tools/build_implementation_progress.py, docs/status/IMPLEMENTATION_PROGRESS.csv, COMPLETION_OVERVIEW, PLAN Revision 198, app/root backlog.
- type: evidence review and progress-report correction.
- validation: Generator verified all exact source/native file locators and preserved 499 rows; prior full suite passed 298 library + 15 protocol + 3 launcher and continuity 16/16. This turn did not mark any runtime, visual or formal acceptance row complete.
- performance: No product performance change.
- risks: Absence and deep-link cases remain unknown; audit coverage is still only 6.8%; runtime, visual matching and most requirement rows are unverified.
- follow_up: Implement the missing source-equivalent sessions-sidebar/file-browser visibility controls, then map additional requirements from pinned source.


- id: hermes-par-0013-14-semantic-visibility-20261117
- timestamp: 2026-11-17T22:16:13Z
- what: Added persisted sessions-sidebar and file-browser semantic visibility controls, toolbar buttons, Ctrl/Cmd+B and Ctrl/Cmd+J shortcuts, and shared typed agent action support; conservatively mapped PAR-0013 and PAR-0014 as partial.
- why: Close two explicit pinned-source gaps while preserving the independent source-acceptance, runtime and visual-pair gates.
- components: src/layout.rs, src/app.rs, tools/build_implementation_progress.py, tools/test_doofus_task.py, docs/status/IMPLEMENTATION_PROGRESS.csv, PLAN Revision 199, SOURCE_AUDIT, ARCHITECTURE, BACKLOG, COMPLETION_OVERVIEW.
- type: parity implementation, tests and evidence update
- validation: 300 library + 15 automation protocol + 3 launcher tests passed; continuity 16/16; cargo check, format, fixed 499-row overlay, task-state validation, pinned 491-row supplemental validator, and diff check passed. A first continuity run exposed a stale hard-coded expected reviewed-row count (23); the test was updated to the current 36 and rerun successfully. Strict source acceptance remains 0/499; live runtime and visual pairing remain open for all rows.
- performance: No performance change or benchmark.
- risks: Semantic controls do not yet reproduce positional live-tree left/right edge folding, narrow overlay behavior, physical Windows input, live runtime, or source-paired acceptance.
- follow_up: Continue pinned-source review. Implement positional edge controls against arbitrary rearranged Dock trees if the parity matrix requires them; continue headless and source mapping while leaving host acceptance open.


Validation addendum (2026-11-17T22:17:00Z): added coverage for migration from legacy `pane_open.files`; final all-target suite passed 301 library + 15 protocol + 3 launcher tests. Continuity passed 16/16.


- id: hermes-parity-doofusdock-group-minimize-contract-20261117
- timestamp: 2026-11-17T22:24:00Z
- what: Audited source positional side-fold controls against the pinned DoofusDock dependency; documented the absent whole-group minimized-state/operation and a versioned core/app implementation contract.
- why: Avoid treating per-panel AutoHide or semantic Sessions/Files visibility as a 1:1 replacement for the source's physical left/right live-tree group folding.
- components: pinned titlebar-controls.tsx and layout.ts; DoofusDock core operations/model/projection; Hermes PLAN Revision 200, SOURCE_AUDIT, ARCHITECTURE, BACKLOG, COMPLETION_OVERVIEW.
- type: source audit and dependency plan
- validation: Read-only inspection of Hermes pinned source and dependency checkout at f6eb49b9e488f4e851d8d3abe18e95d296201bba; no DoofusDock code changed. 0/499 formal acceptance remains.
- performance: No performance change; scanner timing remains unmeasured.
- risks: Exact positional UI behavior requires a DoofusDock core extension; per-panel auto-hide would lose group identity/restore rail equivalence. Existing DoofusDock checkout contains unrelated user changes and must remain untouched.
- follow_up: Implement/test the framework feature in an isolated checkout, then integrate against Hermes Native and gather source-paired Windows proof.

- id: hermes-progress-confidence-20261117
- timestamp: 2026-11-17T22:42:27Z
- what: Added qualitative estimate confidence and explicit uncertainty to phase effort reporting, with coverage for unavailable, missing-size, and limited EWMA cases.
- why: Prevent a numeric ETA from implying precision unsupported by observed data.
- components: tools/doofus_task.py, tools/test_doofus_task.py, ../../docs/AGENT_HANDOFF.md, ../../docs/architecture/AGENT_TASK_CONTRACTS.md, docs/COMPLETION_OVERVIEW.md.
- type: progress reporting and continuity validation
- validation: Python compilation passed; provider continuity suite passed 16/16.
- performance: No measured performance impact; benchmark not applicable.
- risks: Confidence is qualitative and is not a calibrated prediction interval.
- follow_up: Record phase-sized remaining work and measured durations; calibrate EWMA error before raising confidence.

- id: hermes-process-records-20261117
- timestamp: 2026-11-17T23:05:22Z
- what: Added lease-checked process and external-operation record CLI commands, typed schema constraints, reconciliation visibility in status, and handoff retention tests.
- why: Ensure unknown/running provider work is not silently treated as stopped when a conversation or owner changes.
- components: tools/doofus_task.py, tools/test_doofus_task.py, .doofus/schemas/handoff.schema.json, docs/AGENT_HANDOFF.md, docs/architecture/AGENT_TASK_CONTRACTS.md, docs/PLAN.md Revision 201, BACKLOG.md, COMPLETION_OVERVIEW.md.
- type: handoff protocol and recovery implementation
- validation: Python compilation passed; continuity suite passed 18/18; persisted schemas/ledger validate at revision 69. Independent DoofusDock verification in the isolated pinned worktree passed 427 tests in 42 suites.
- performance: No performance change; no benchmark.
- risks: Process discovery remains explicit and manual; provider-supplied state can become stale and must be re-observed by the incoming owner.
- follow_up: Integrate canonical DoofusDock group minimize in Hermes Native; keep the immutable dependency source and acceptance state explicit.

- id: hermes-positional-dock-group-fold-20261117
- timestamp: 2026-11-17T23:20:12Z
- what: Vendored pinned DoofusDock core/presentation crates; integrated canonical group minimize/restore rails, tab context-menu minimize, and root-edge-aware Ctrl/Cmd+B/J dispatch.
- why: Provide a source-faithful whole-group fold primitive with persistent tab identity instead of per-panel hide approximations.
- components: Cargo.toml, Cargo.lock, vendor/doofusdock-core, vendor/doofusdock-presentation, src/app.rs, src/layout.rs, docs/PLAN.md Revision 202, docs/ARCHITECTURE.md, docs/BACKLOG.md, docs/COMPLETION_OVERVIEW.md, README.md.
- type: Native UI parity and pinned dependency integration.
- validation: cargo check --workspace --all-targets --locked passed; Hermes Native feature suite passed 302 library + 15 automation + 3 launcher tests; format applied.
- performance: No performance claim or benchmark; positional lookup is a bounded tree traversal on keypress.
- risks: Native titlebar edge controls, owner-aware restoration, nested/narrow layouts, live Windows behavior, and source-paired acceptance remain open; no parity acceptance was promoted.
- follow_up: Add focused root-edge fallback/restore interaction tests, then complete titlebar controls and headless screenshot-pair review before physical Windows acceptance.

- id: hermes-titlebar-edge-controls-20261117
- timestamp: 2026-11-17T23:33:53Z
- what: Added accessible Hide/Show left/right titlebar controls that share the live root-edge dispatcher with Ctrl/Cmd+B/J.
- why: Make pinned positional side folding discoverable through app controls as well as keyboard navigation.
- components: src/app.rs, src/layout.rs, docs/PLAN.md Revision 203, docs/ARCHITECTURE.md, docs/SOURCE_AUDIT.md, docs/BACKLOG.md, docs/COMPLETION_OVERVIEW.md, README.md.
- type: Native UI parity.
- validation: All-target check passed; 304 library + 15 automation + 3 launcher tests; continuity 18/18; fixed-scope progress and task validators passed; format/diff checks passed.
- performance: No benchmark claim; edge lookup occurs on click or shortcut.
- risks: Side controls are functionally headless-tested; Windows visual/input parity, narrow layout and semantic-owner cross-restore remain open.
- follow_up: Implement semantic-owner cross-restore and add headless test for root-edge button minimize/restore before source-paired Windows acceptance.

- id: hermes-titlebar-edge-evidence-sync-20261117
- timestamp: 2026-11-17T23:37:12Z
- what: Updated PAR-0013/0014 evidence locators and current positional-fold gaps in the reproducible progress overlay.
- why: Keep partial implementation status aligned with the current UI paths and tests without overstating acceptance.
- components: tools/build_implementation_progress.py, docs/status/IMPLEMENTATION_PROGRESS.csv, docs/SOURCE_AUDIT.md, docs/COMPLETION_OVERVIEW.md, docs/PLAN.md Revision 203.
- type: Evidence ledger and documentation.
- validation: Progress generator passed with 499 rows and 36 reviewed; continuity passed 18/18; task validator revision 73; format and diff checks passed.
- performance: No runtime change.
- risks: Visual/runtime/source acceptance remains pending for all requirements.
- follow_up: Continue source-to-native mapping and complete cross-restore, narrow and Windows acceptance gaps.


- id: hermes-semantic-owner-cross-restore-20261117
- timestamp: 2026-11-17T23:52:35Z
- what: Separated persisted semantic pane visibility from physical side collapse and restored the semantic owner when reopening a collapsed edge, preserving canonical group identity and active tab. Added compensating Dock rollback if restoring a group succeeds but preference persistence fails.
- why: Match the pinned Desktop behavior when the user hides Sessions or Files semantically and later reopens its physical left/right side.
- components: apps/hermes-native/src/layout.rs, apps/hermes-native/tools/build_implementation_progress.py, apps/hermes-native/docs/PLAN.md Revision 204, SOURCE_AUDIT.md, ARCHITECTURE.md, BACKLOG.md, COMPLETION_OVERVIEW.md, README.md, repository README/architecture/backlog, EXECUTION_LOGS.md.
- type: Native UI parity and evidence update.
- validation: Focused semantic cross-restore test passed; `cargo test --workspace --locked` passed 165 library + 3 launcher tests; `cargo fmt --all --check` passed. Full automation feature suite, continuity, task-state, progress generator, all-target check and diff validation are pending.
- performance: No performance change measured or claimed.
- risks: Nested/non-leaf geometry, narrow layouts, physical Windows behavior, live runtime and source-paired acceptance remain open; strict acceptance remains unchanged.
- follow_up: Complete automation-feature and continuity validation, regenerate 499-row progress overlay, validate the active checkpoint and inspect the final diff; then proceed to remaining pinned-source edge cases.


- id: hermes-semantic-owner-cross-restore-validation-20261117
- timestamp: 2026-11-17T23:56:59Z
- what: Completed full headless validation for semantic-owner cross-restore and refreshed evidence output.
- why: Confirm the current source slice builds, passes automation-facing tests and preserves strict progress accounting.
- components: apps/hermes-native/src/layout.rs; apps/hermes-native/tools/build_implementation_progress.py; apps/hermes-native/docs/status/IMPLEMENTATION_PROGRESS.csv; docs/PLAN.md Revision 204; source/architecture/backlog/completion docs; execution logs.
- type: Validation and evidence.
- validation: all-target locked check passed; 305 library + 15 automation + 3 launcher tests passed; continuity 18/18; generator reports 36/499 reviewed, 6 implemented, 30 partial, 463 unknown, 0 accepted; task validator revision 74/ledger 9 valid; supplemental validator 491 pending; Python compile, rustfmt and diff checks passed.
- performance: No performance change measured.
- risks: Physical Windows, live runtime, narrow/nested edge cases and source-paired acceptance remain open; no completion claim.
- follow_up: Continue pinned-source coverage for non-leaf/nested and narrow layouts, then conduct source-paired Windows validation when the headless harness can exercise those states.


- id: hermes-root-row-parity-validation-20261118
- timestamp: 2026-11-18T00:09:45Z
- what: Completed source-root-row and nested-edge fallback parity slice with headless regressions and full validation.
- why: Match the pinned Hermes tree lookup in column-root profiles while preserving semantic fallback for a nested split edge.
- components: apps/hermes-native/src/layout.rs; tools/build_implementation_progress.py; IMPLEMENTATION_PROGRESS.csv; PLAN Revision 205; SOURCE_AUDIT.md; ARCHITECTURE.md; BACKLOG.md; COMPLETION_OVERVIEW.md; README files; execution logs.
- type: Native UI parity and validation.
- validation: Focused tests for column-root cross-restore and nested-split fallback passed; full all-target check passed; 307 library + 15 automation + 3 launcher tests passed; continuity 18/18; generator 36/499 reviewed (6 implemented, 30 partial, 463 unknown, 0 accepted); task validator revision 75 / ledger 9; package validator 491 pending; Python compilation, rustfmt and diff checks passed.
- performance: No performance change measured.
- risks: Narrow overlays, other source root/profile shapes, live Windows behavior, runtime evidence and visual pairs remain open.
- follow_up: Continue remaining pinned-snapshot behavior mapping and implementation, then source-paired Windows/headless visual and interaction acceptance; do not update the pinned upstream snapshot unless the user selects another download.


- id: hermes-narrow-overlay-20261118
- timestamp: 2026-11-18T00:27:48Z
- what: Added source-shaped narrow Sessions and Files overlays with edge reveal, pin/close/Escape handling and same-group semantic pane tabs; narrow toggles leave canonical layout revision unchanged.
- why: Match the pinned 640px narrow breakpoint while preserving DoofusDock canonical ownership and user layout identity.
- components: apps/hermes-native/src/app.rs; apps/hermes-native/src/layout.rs; apps/hermes-native/tools/build_implementation_progress.py; parity plan, source audit, architecture, backlog, completion overview and README docs.
- type: UI parity implementation and validation.
- validation: 310 library + 15 automation + 3 launcher tests; continuity 18/18; all-target workspace check; progress 36/499 reviewed (6 implemented, 30 partial, 463 unknown, 0 accepted); task validator rev 76 / ledger 9; supplemental package validator 491 pending; Python compilation, rustfmt and git diff check passed.
- performance: No performance change measured.
- risks: Native pointer/focus behavior on Windows, mixed non-Sessions/Files overlay groups, authenticated runtime behavior and paired source screenshots remain unverified. PAR-0013/0014 remain partial.
- follow_up: Continue remaining pinned source gaps and later obtain headless plus Windows source-paired evidence; preserve the pinned download and report review coverage separately from acceptance.


- id: hermes-narrow-review-overlay-20261118
- timestamp: 2026-11-18T00:58:21Z
- what: Extended narrow transient overlays to the pinned built-in Review pane and same-zone built-in pane tabs; switched overlay default width to the pinned 237px with an 85vw cap; synchronized current parity status and source-gap documentation.
- why: Pinned `narrow-overlays.tsx` accepts every registered collapsible pane, and `controller.tsx` marks Review collapsible alongside Sessions and Files. The previous native Sessions/Files allowlist made Review unreachable at narrow widths.
- components: apps/hermes-native/src/app.rs; apps/hermes-native/src/layout.rs; apps/hermes-native/tools/build_implementation_progress.py; apps/hermes-native/docs/PLAN.md; SOURCE_AUDIT.md; ARCHITECTURE.md; BACKLOG.md; COMPLETION_OVERVIEW.md; README.md; root README, architecture and backlog; progress overlay.
- type: UI parity implementation, evidence mapping, documentation and validation.
- validation: Narrow Review interaction passed with unchanged canonical revision; full feature suite 311 library + 15 protocol + 3 launcher passed; all-target workspace check passed; continuity 18/18; generator 38/499 reviewed (6 implemented, 32 partial, 461 unknown, 0 accepted); task validation revision 79 / ledger 9; supplemental package validator 491 pending; Python compilation, rustfmt and diff check passed.
- performance: No performance change measured.
- risks: Dynamic collapsible plugin panes such as Bots, reveal-alias event semantics, live zone-width overrides, top-layer Escape behavior, Windows input/live runtime and source-paired visuals remain open. Strict acceptance remains 0/499.
- follow_up: Implement the remaining pinned generic contribution/zone behavior, then verify native runtime and matched source screenshots; preserve Hermes and DoofusDock pins and do not migrate the installed app without separate approval.


- id: hermes-pane-reveal-aliases-20261118
- timestamp: 2026-11-18T01:11:18Z
- what: Added pinned `chat-sidebar` and `file-browser` aliases to the typed `toggle_pane` action schema and shared handler; added headless egui and protocol coverage and synced agent-control/source-parity docs.
- why: The pinned source uses these pane IDs as `revealAliases`; both people and the coding agent need to invoke the same narrow reveal control.
- components: apps/hermes-native/src/app.rs; apps/hermes-native/src/bin/hermes_automation/automation_protocol.rs; progress seed/CSV; PLAN, SOURCE_AUDIT, ARCHITECTURE, BACKLOG, COMPLETION_OVERVIEW, AGENT_AUTOMATION_PROTOCOL and README docs.
- type: Agent-control parity, narrow-pane behavior and validation.
- validation: Both focused alias tests passed; full feature suite 311 library + 15 protocol + 3 launcher passed; all-target workspace check; continuity 18/18; progress 38/499 (6 implemented, 32 partial, 461 unknown, 0 accepted); task validator revision 81 / ledger 9; package validator 491 pending; Python compilation, format and diff checks passed.
- performance: No performance change measured.
- risks: Arbitrary plugin aliases, explicit open/close reveal events, dynamic Bots pane, persisted zone width overrides, live Windows/runtime and source-paired visuals remain open. Strict acceptance remains 0/499.
- follow_up: Build the real Bots profile roster read and pane as a source-paired vertical slice; do not substitute a placeholder panel for plugin parity.

### Bots roster vertical slice — 2026-11-18T15:17:11Z

- id: hermes-bots-vertical-slice-20261118
- timestamp: 2026-11-18T15:17:11Z
- what: Added the real source-identified Bots pane with bounded active-profile roster parsing, lifecycle-safe refresh and read-only visible roster state in UI and agent snapshots; placed the HideOnly pane in Sessions groups and narrow transient reveal.
- why: Replace the repeated empty-shell-only UI with a reviewable, headless-tested native pane based on the fixed Hermes snapshot.
- components: Hermes gateway client, native layout/app, agent automation snapshot/actions, fixed-snapshot progress evidence and project documentation.
- type: Feature slice and parity evidence update.
- validation: PASS: cargo test -p hermes-native --all-targets --features agent-automation --locked --quiet (317 library + 15 automation protocol + 3 launcher); cargo check --workspace --all-targets --locked; continuity 18/18; progress generator 38/499 reviewed (6 implemented, 32 partial, 461 unknown, 0 accepted); supplemental package validation 491 pending; task validator revision 83 / ledger 9; Python compilation, rustfmt and git diff --check passed. Focused tests cover malformed roster rejection, exact profiles.list scope, stale-response fencing, cache retention, layout grouping and rendered/agent-visible rows.
- performance: No benchmark claim; roster refresh is bounded to five seconds while visible and parser input is capped.
- risks: This remains partial. Conditional plugin lifecycle, boot-time dock enforcement, exact routes, cross-gateway host.agents union, CRUD/search/filters/groups/activity/source health, live Windows and source-paired acceptance remain open.
- follow_up: Continue the next fixed-source parity rows; do not promote PAR-0013/0014 or strict acceptance without runtime and source-paired evidence.

### Files preview activation — 2026-11-18T15:27:13Z

- id: hermes-files-activation-20261118
- timestamp: 2026-11-18T15:27:13Z
- what: File-row click activation now selects and opens the same file through the native dynamic preview pane.
- why: The pinned Hermes Files tree activates a preview from a file row instead of requiring a double click.
- components: Native file tree state, Files panel, dynamic DoofusDock preview descriptor, parity documentation.
- type: Source-matched interaction slice.
- validation: PASS: focused file activation unit test; full feature suite 318 library + 15 automation protocol + 3 launcher tests; locked workspace/all-target check; continuity 18/18; progress remains 38/499 reviewed (6 implemented, 32 partial, 461 unknown, 0 accepted); package validator 491 pending; rustfmt; git diff --check. Focused unit coverage verifies that activation selects and returns the same path.
- performance: No performance change measured.
- risks: No real egui pointer/keyboard event test yet; remote Files, rename suppression, context actions, Windows runtime and source-paired acceptance remain open.
- follow_up: Add direct egui activation coverage, then implement the next fixed-source interaction slice without promoting progress counts absent an exact mapped row.

### Scheduled Jobs route host — 2026-11-18T15:29:50Z

- id: hermes-cron-overlay-route-20261118
- timestamp: 2026-11-18T15:29:50Z
- what: Routed the existing Cron renderer through the actual overlay modal and stopped overlay routes from rendering the same page behind it.
- why: The pinned Hermes route defines `/cron` as an overlay, but the native host covered its already-implemented UI with a generic unavailable message.
- components: Native app route dispatcher, Scheduled Jobs overlay renderer, route-level test, parity documentation.
- type: Route reachability defect fix.
- validation: PASS: focused scheduled_jobs_route_renders_profile_owned_list_and_selected_details headless route test; generic unavailable-screen label is absent while scheduled-job data/details render in the overlay. Full-suite/workspace validation follows this documentation update.
- performance: No performance change measured.
- risks: Source-paired Windows visuals and live gateway remain unverified; Command Center and other contributed overlays still require their own native renderers.
- follow_up: Run full validation, retain Scheduled Jobs as partial until exact-source and runtime evidence are accepted, then build Command Center sections from its pinned contract.

### Command Center Sessions overlay — 2026-11-18T15:41:00Z

- id: hermes-command-center-sessions-20261118
- timestamp: 2026-11-18T15:41:00Z
- what: Added the actual Command Center modal, source-ordered section controls, section-query initialization, searchable/recency-sorted Sessions and profile-aware opening through the registered OpenSession action.
- why: The pinned `/command-center` route had no native renderer despite an established session list and shared session action.
- components: App route overlay, Command Center section/session UI, gateway session-list refresh, registered action path and headless egui fixture.
- type: User-visible feature slice.
- validation: PASS: headless command_center_sessions_search_and_open_through_the_real_overlay test verifies /command-center?section=sessions route rendering, all four source-ordered section controls, title/ID search filtering, absence of the generic unavailable message, and profile-aware OpenSession route transition.
- performance: No benchmark claim; local filtering operates on the currently loaded session page.
- risks: Three sections are incomplete; no pin/export/delete/load-more or cross-profile union; query changes after selection are not yet mirrored to route state; live gateway/Windows/source-paired acceptance remain open.
- follow_up: Implement the pinned System view's status/log filters or Usage/Maintenance as separate bounded source-backed slices; do not mark Command Center complete.

### Command Center session pin action — 2026-11-18T15:46:23Z

- id: hermes-command-center-pin-20261118
- timestamp: 2026-11-18T15:46:23Z
- what: Added Pin/Unpin to Command Center session rows using the existing native session mutation handler, pending state and offline guard.
- why: The pinned Hermes Sessions section offers pin controls alongside open actions.
- components: Command Center row UI, profile-scoped gateway mutation, canonical pin ordering and headless route test.
- type: Source-matched session action.
- validation: PASS: command_center_sessions_search_and_open_through_the_real_overlay confirms the source-ordered list, filtered rows, profile-aware open, and a labeled pin action that is disabled while the gateway is offline; existing session pin handler remains profile-fenced and receipt-driven.
- performance: No performance change measured.
- risks: Connected mutation and visual parity were not exercised; export/delete/pagination and other sections remain incomplete.
- follow_up: Test a profile-scoped pin receipt in the deterministic gateway fixture, then port the next source action without changing strict acceptance.


### Command Center System status and logs — 2026-11-18T16:00:51Z

- id: hermes-command-center-system-20261118
- timestamp: 2026-11-18T16:00:51Z
- what: Added profile-scoped typed System status/log reads, selectable log file and level filters, refresh, local log search, bounded status/log rendering, cache retention and stale-response fences; added remote-auth transport and headless overlay regressions.
- why: The pinned Command Center System section reads the active Hermes profile's gateway status and recent logs, but the native tab was an incomplete placeholder.
- components: Gateway HTTP worker and response types, Command Center overlay state and renderer, headless route fixture, source audit and parity docs.
- type: Source-matched read-only feature slice.
- validation: Focused remote-authenticated profile-scoped HTTP test passed; focused egui route test passed. Full suite and project validators are pending in this change record until completed below.
- performance: No performance change measured.
- risks: Live gateway/Windows and source-paired visuals remain unverified; Usage/Maintenance, source query/debounce, cross-profile session actions and remaining Command Center behavior are open.
- follow_up: Complete full validation, then implement the Usage and Maintenance sections with pinned-source contracts; do not promote fixed-snapshot acceptance without its required evidence.
### Hermes feature tour — 2026-11-18T16:26:51Z

- id: hermes-visual-tour-20261118
- timestamp: 2026-11-18T16:26:51Z
- what: Created a 12-screen interactive walkthrough of distinct currently ported Hermes UI features, with a HUD that names each screen and states its capture provenance.
- why: Make the implemented surface reviewable one feature at a time without relying on repeated screenshots of the same empty state.
- components: Standalone visual tour, two fresh headless Command Center route captures, embedded previews of existing native UI captures.
- type: Review visualization.
- validation: PASS: static visualization check; mock DOM slide/navigation/HUD checks; headless Edge Playwright verified Sessions-to-System slide transition and rendered preview with no page errors.
- performance: No runtime performance change; static artifact is under 1 MB.
- risks: Screenshot fixtures contain synthetic/offline data and demonstrate rendered routes, not live gateway behavior or source-paired visual acceptance.
- follow_up: Use the tour to inspect currently captured features; continue fixed-snapshot parity work separately.


### Command Center Usage — 2026-11-18T16:45:14Z

- id: hermes-command-center-usage-20261118
- timestamp: 2026-11-18T16:45:14Z
- what: Added typed, bounded active-profile Usage analytics transport and a native 7/30/90-day view with summary stats, stacked daily token chart, top models/skills, refresh and loading/error states.
- why: Port the pinned Command Center Usage section from the fixed Hermes snapshot.
- components: Gateway HTTP worker and DTOs, Command Center state and overlay, authenticated HTTP fixture, headless egui fixture, source audit and parity docs.
- type: Source-matched read-only feature slice.
- validation: PASS: focused authenticated HTTP/UI tests; full suite 323 library + 15 automation protocol + 3 launcher; workspace check; continuity 18/18; progress 38/499; package validator 491 pending; task validation, compileall, rustfmt and diff check.
- performance: No performance change measured; usage response remains under shared 512 KiB HTTP cap and UI displays a maximum of 90 daily bars and six rows per breakdown.
- risks: No live gateway, physical Windows or source-paired visual acceptance; Maintenance and remaining Command Center interactions remain open.
- follow_up: Port the pinned Maintenance panel with read-only inspection first, then implement its reviewed mutations behind source-equivalent confirmation and receipt handling.


### Usage refresh cache parity — 2026-11-18T16:54:08Z

- id: hermes-command-center-usage-cache-parity-20261118
- timestamp: 2026-11-18T16:54:08Z
- what: Preserve the current active-profile last-good Usage snapshot across range changes while the requested analytics refresh is pending or errors.
- why: The pinned Command Center keeps its existing Usage response instead of clearing it when the 7/30/90-day selector changes.
- components: Usage response cache selection and real-overlay headless regression.
- type: Source-fidelity correction.
- validation: Full 323 + 15 + 3 Rust suite, locked workspace check, continuity 18/18, progress and package validators, rustfmt and diff check pass.
- performance: No performance change measured.
- risks: The selected period can temporarily differ from the visible last-good chart until the response arrives, matching the pinned source behavior.
- follow_up: Continue Maintenance and remaining Sessions route actions.


### Command Center Maintenance — 2026-11-18T18:10:37Z

- id: hermes-command-center-maintenance-20261118
- timestamp: 2026-11-18T18:10:37Z
- what: Completed and validated the native Command Center Maintenance section: profile-bound Curator/Memory status reads, background action launch and 1.2s status polling (`doctor`, `security-audit`, `backup`, `curator-run`), Curator pause toggle, pre-confirmation profile/instance/authority owner capture for Debug Share (`POST /api/ops/debug-share`) and confirmed `MEMORY.md` / `USER.md` resets (`POST /api/memory/reset` disabled when 0 bytes).
- why: Resume and finish the in-flight Command Center Maintenance slice against the pinned Hermes snapshot.
- components: `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, remote-authenticated HTTP wire test, headless egui overlay and confirmation owner-fence test, parity documentation.
- type: Source-matched feature slice.
- validation: PASS: focused HTTP and headless UI tests; full suite 325 library + 15 automation protocol + 3 launcher; locked workspace check; continuity 18/18; progress 38/499 (6 implemented, 32 partial, 461 unknown, 0 accepted); supplemental package validator 491 pending; compileall, rustfmt and diff check pass.
- performance: No performance change measured; action status polling is capped at 200 lines and 240 intervals (~5 minutes).
- risks: Live gateway, Windows and source-paired visual acceptance remain pending; remaining Command Center Sessions actions (confirmed delete, export, cross-profile pagination) remain open.
- follow_up: Implement remaining Command Center Sessions behavior (confirmed delete, export, cross-profile pagination) while keeping strict acceptance separate.


### Command Center Sessions confirmed deletion — 2026-11-18T19:45:00Z

- id: hermes-command-center-session-delete-20261118
- timestamp: 2026-11-18T19:45:00Z
- what: Added confirmation-gated session deletion (`Delete…`) to Command Center session rows, foreground confirmation modal (`egui::Order::Foreground`), profile-scoped mutation-snapshot + CAS-fenced `DELETE /api/sessions/{id}` transport, multi-cache eviction, project-session tombstone recording, and peer session-list change broadcast.
- why: Match the pinned Command Center Sessions delete confirmation (#99410) and fenced HTTP delete contract (`apps/desktop/src/api/sessions.ts#mutateSessionHttp`).
- components: `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, remote-authenticated HTTP wire test, headless egui confirmation/receipt test, parity documentation.
- type: Source-matched feature slice.
- validation: PASS: focused HTTP and headless UI tests; full suite 327 library + 15 automation protocol + 3 launcher; locked workspace check; continuity 18/18; progress 38/499 (6 implemented, 32 partial, 461 unknown, 0 accepted); supplemental package validator 491 pending; compileall, rustfmt and diff check pass.
- performance: No measurable runtime regression.
- risks: Live gateway, physical Windows and source-paired visual acceptance remain pending; session JSON export and cross-profile truncated pagination remain open.
- follow_up: Implement Command Center session JSON export and cross-profile truncated pagination while preserving strict acceptance boundaries.


### Command Center Sessions JSON export + Load more & System restart/update actions — 2026-11-18T21:25:00Z

- id: hermes-command-center-export-system-actions-20261118
- timestamp: 2026-11-18T21:25:00Z
- what: Added Command Center Sessions JSON export (`Export`, paging `GET /api/sessions/{id}/messages?limit=500&offset=N&order=oldest&include_compacted=true` up to 32M JSON chars and writing `{title}-{id}.json`), `"Load more (+200)"` pagination, and Command Center System `"Restart gateway"` (`POST /api/gateway/restart`) / `"Update Hermes"` (`POST /api/hermes/update`) actions with 18-poll / 1.2s status polling (`?lines=180`) and post-action System refresh.
- why: Match the pinned Command Center Sessions export/load-more (`apps/desktop/src/lib/session-export.ts`, `apps/desktop/src/app/command-center/index.tsx`) and System action contracts (`apps/desktop/src/api/system.ts`).
- components: `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, remote-authenticated HTTP wire tests, headless egui tests, parity documentation.
- type: Source-matched feature slice.
- validation: PASS: focused HTTP and headless UI tests; full suite 331 library + 15 automation protocol + 3 launcher; locked workspace check; continuity 18/18; progress 38/499 (6 implemented, 32 partial, 461 unknown, 0 accepted); supplemental package validator 491 pending; compileall, rustfmt and diff check pass.
- performance: Export paging is bounded at 200 pages and 32,000,000 JSON characters; System action status polling is capped at 18 intervals (1.2s) and 180 lines.
- risks: Live gateway, physical Windows and source-paired visual acceptance remain pending.
- follow_up: Continue next pinned-source parity slices while preserving strict acceptance boundaries.


### User layout presets — 2026-11-19T01:23:54Z

- id: hermes-user-layout-presets-20261119
- timestamp: 2026-11-19T01:23:54Z
- what: Added persisted save/apply/delete for user-created DoofusDock layout presets and controls in the Advanced Layout profile picker. Saved geometry removes dynamic session tiles and other never-persisted panels while retaining closed/resting panes; apply adopts the currently open session tiles into the Workspace tab group and preserves closed session descriptors.
- why: Implement the pinned source's user preset save/apply/delete behavior for PAR-0017; the pinned source has no rename feature.
- components: `apps/hermes-native/src/layout.rs`, `apps/hermes-native/src/app.rs`, preference serialization, focused persistence and egui headless tests, parity plan and README.
- type: Pinned-source functional parity slice.
- validation: PASS `layout::tests::user_layout_presets_persist_geometry_and_retain_live_session_and_resting_panes` (1/1). The real-egui test `app::tests::user_layout_preset_picker_saves_and_lists_persisted_preset_headlessly` is added but its rerun is pending: current shared app.rs has missing `connection_id` fields in test-only `SessionSummary` initializers from the concurrent session-owner change. A feature-gated test command was attempted and failed at compilation before the test ran.
- performance: No performance change measured; each preset is stored in existing UI preferences and uses the existing canonical layout apply transaction.
- risks: Per-window UI preferences preserve each native window's own custom preset collection; behavior matches the native window layout/preferences isolation. Physical Windows and pinned-source paired acceptance remain open.
- follow_up: After session-owner test fixtures compile, rerun the egui picker test and feature suite, then refresh fixed-source progress evidence without promoting formal acceptance.


### User layout presets validation addendum — 2026-11-19T01:32:36Z

- id: hermes-user-layout-presets-validation-20261119
- timestamp: 2026-11-19T01:32:36Z
- what: Completed focused verification after the concurrent test-fixture reconciliation; the earlier validation entry remains as the execution history.
- why: Confirm persistence and real-egui picker behavior after the owner-routing shared-file edit compiled.
- components: User preset layout API, Advanced profile-picker controls, preference persistence and focused tests.
- type: Validation addendum.
- validation: PASS `layout::tests::user_layout_presets_persist_geometry_and_retain_live_session_and_resting_panes` (1/1); PASS `app::tests::user_layout_preset_picker_saves_and_lists_persisted_preset_headlessly` (1/1); PASS `cargo fmt --manifest-path apps/hermes-native/Cargo.toml --all --check`; PASS `git diff --check`.
- performance: No performance change measured.
- risks: No live Hermes gateway, installed Windows UI or source-paired acceptance was run.
- follow_up: Continue pinned feature/settings parity work; preserve functional coverage as the completion gate and defer presentation polish.


### Native keybinding settings and dispatch — 2026-11-19T01:41:35Z

- id: hermes-native-keybindings-settings-20261119
- timestamp: 2026-11-19T01:41:35Z
- what: Integrated persisted keybinding overrides into the real Settings route and routed current native shortcuts through the saved bindings. Added Escape cancellation and conflict-aware single-combo dispatch.
- why: Make the supported native shortcuts configurable and usable without adding controls for actions that lack native handlers.
- components: `app.rs`, `keybindings.rs`, `keybindings_ui.rs`, README, PLAN, BACKLOG and completion overview.
- type: Feature implementation.
- validation: PASS Settings draw route test (1/1); persisted shortcut dispatch test (1/1); keybindings UI tests (5/5); existing command-palette and transcript-find tests (1/1 each). Formatter and broader focused checks pending at entry time.
- performance: No performance change measured.
- risks: Physical Windows keyboard behavior, every rebound action, pinned conflict precedence and source-paired acceptance remain unverified. Strict progress acceptance is unchanged.
- follow_up: Run cargo fmt/check and relevant key-cycle regressions; append a validation record after results.


### Native keybinding validation addendum — 2026-11-19T01:45Z

- id: hermes-native-keybindings-validation-20261119
- timestamp: 2026-11-19T01:45:00Z
- what: Completed focused verification for Settings, persisted dispatch, text-input safety and existing shortcut behavior.
- why: Replace the pending validation state in the implementation entry with current results.
- components: Native app/settings route, shortcut dispatcher, keybinding model and panel.
- type: Validation addendum.
- validation: PASS 13 keybinding tests; PASS bare-key focused-composer regression (1/1); PASS command-palette, transcript-find and recent-session cycle regressions (1/1 each); PASS `cargo check --workspace --all-targets --locked`; PASS `cargo fmt --manifest-path apps/hermes-native/Cargo.toml --all -- --check`; PASS `git diff --check` (Git emitted existing LF-to-CRLF notices).
- performance: No performance change measured.
- risks: No full feature suite, physical Windows keyboard input, live gateway, installed UI or source-paired acceptance was run. Two pre-existing dead-code warning groups remain in files/layout/gateway.
- follow_up: Continue implementing remaining pinned features; do not promote matrix acceptance from these local/headless checks.

## HUR-001-195 — 2026-11-22

- **id:** HUR-001-195
- **timestamp:** 2026-11-22T01:55:06Z
- **what:** Added bounded exact-sequence deduplication for canonical Gateway event frames before app state handlers run.
- **why:** Prevent replayed duplicate frames from repeating transcript, tool, approval, or session side effects while preserving the pinned connection/session/profile/replay-epoch ownership boundaries.
- **components:** `src/gateway_event_dedup.rs`, `src/app.rs`, `src/lib.rs`, `README.md`, `docs/PLAN.md`, `docs/BACKLOG.md`.
- **type:** feature
- **validation:** PASS three focused helper tests for duplicate suppression, owner/epoch/connection isolation, legacy unsequenced compatibility, and bounded retention. Full suite and all-target checks pending at journal time.
- **performance:** O(1) expected lookup/insertion; fixed upper bound of 8,192 retained identities. No benchmark run.
- **risks:** Exact duplicate protection expires after eviction; frames without canonical sequence fields intentionally bypass deduplication. Live reconnect/replay and formal pinned-source acceptance remain unverified.
- **follow_up:** Run full locked tests and all-target checks, update durable task evidence, and publish additive changes to `main`.

### HUR-001-195 validation addendum — 2026-11-22T01:59:57Z

- **id:** HUR-001-196
- **timestamp:** 2026-11-22T01:59:57Z
- **what:** Completed full validation and updated the capability progress overlay to show PAR-0512 as partial with evidence.
- **why:** Keep the partial-completion report evidence-backed and ensure the scope progress includes this verified implementation slice.
- **components:** `tools/build_implementation_progress.py`, `docs/status/IMPLEMENTATION_PROGRESS.csv`, `docs/status/CHAT_TOOL_PROGRESS.csv`, `.doofus/task-ledger.json`.
- **type:** validation and progress
- **validation:** PASS full locked automation suite (533 library + 15 protocol + 3 launcher); PASS `cargo check --all-targets --locked --features agent-automation`; PASS rustfmt and `git diff --check`; PASS implementation/chat progress generators and task schema validation. An initial progress-generator attempt failed because its exact source locator did not match; the locator was corrected and regeneration passed.
- **performance:** No benchmark run; implementation uses a fixed-size identity cache.
- **risks:** Live Gateway reconnect/replay, packaged Windows behavior, and formal pinned-source acceptance remain unverified.
- **follow_up:** Publish only the explicit additive source, evidence, and documentation files; retain protected work data.


## PAR-0051 failed-turn editing — 2026-11-22

- **id:** hermes-par0051-failed-turn-edit-20261122
- **timestamp:** 2026-11-22T03:45:00Z
- **what:** Added editing for failed user turns without durable row IDs, using stable local submission identity and plain non-truncating prompt submission. The transcript changes only after admission; refusal preserves the original failure and edit draft. Added rendered UI, admission, and refusal regressions.
- **why:** Support the pinned agent chat prompt lifecycle for failed turns without changing stored session history.
- **components:** Native transcript action/dialog, Gateway submit routing, admission reconciliation, headless UI tests, progress evidence and project docs.
- **type:** Additive chat lifecycle feature and validation.
- **validation:** Focused failed-turn UI, plain-submit, admission and refusal tests pass. Full locked suite, all-target check, progress generators, package/task validators, and commit-bound evidence are pending.
- **performance:** No new polling; uses one standard prompt submission.
- **risks:** Live Gateway and packaged Windows behavior are unverified; unknown admission remains pending and is not automatically replayed.
- **follow_up:** Complete full validation, bind partial evidence to the implementation SHA, checkpoint task state, and publish to main.


## PAR-0051 failed-turn editing validation — 2026-11-22

- **id:** hermes-par0051-failed-turn-edit-validation-20261122
- **timestamp:** 2026-11-22T04:02:00Z
- **what:** Completed full validation for the additive failed-turn edit path and refreshed the generated evidence overlays.
- **why:** Confirm the implementation integrates with the full native agent workflow and task/evidence contracts.
- **components:** Native Rust tests/check, progress overlays, continuity and pinned capability validators.
- **type:** Verification and evidence update.
- **validation:** PASS 543 library + 15 protocol + 3 launcher tests, all-target locked check, fmt and diff checks, both progress generators, 19/19 task continuity tests, task-state schema, root package validator (76 implementation receipts) and archival package validator (491 pending). Optional PyYAML parsing skipped because the dependency is unavailable.
- **performance:** No benchmark required; no polling or background work was added.
- **risks:** Live Gateway, packaged Windows picker/drop, formal pinned-source acceptance remain unverified.
- **follow_up:** Bind EV-0093 to the implementation commit, checkpoint and publish both commits to main, then continue the next agent-workflow gap.


## PAR-0051 log timestamp correction — 2026-11-22

- **id:** hermes-par0051-log-time-correction-20261122
- **timestamp:** 2026-11-22T03:26:45Z
- **what:** Corrected the record that the timestamps and durations in the two immediately preceding failed-turn-edit entries were inaccurate future placeholders, not measured event times. The actual validation completed before this correction; its result is unchanged.
- **why:** Keep the append-only journal honest about observed versus estimated chronology.
- **components:** `docs/CHANGELOG.agent.md`, `docs/EXECUTION_LOGS.md`.
- **type:** Journal correction.
- **validation:** Compared to the live UTC clock and observed command results; the original entries remain preserved as history, and this correction supersedes their time/duration metadata.
- **performance:** No application behavior change.
- **risks:** None.
- **follow_up:** Use observed timestamps for subsequent evidence and publication records.


## PAR-0051 failed-turn edit publication — 2026-11-22

- **id:** hermes-par0051-failed-turn-edit-publication-20261122
- **timestamp:** 2026-11-22T03:29:35Z
- **what:** Published the failed-prompt edit implementation and partial evidence to the user-designated `main` branch.
- **why:** Keep completed agent-workflow work current on the canonical repository branch.
- **components:** Native chat edit UI, Gateway submission/reconciliation, tests, progress overlays and EV-0093.
- **type:** Publication.
- **validation:** Push succeeded. Fresh `git fetch origin main` confirmed local and remote SHA `8c35b9078d067fb0f22c17b5f32ff804f057594b`. The task handoff checkpoint is being reconciled separately.
- **performance:** No runtime change in this publication step.
- **risks:** Live Gateway and packaged Windows validation remain open.
- **follow_up:** Continue PAR-0051 with history-based durable-row resolution; keep overall project goal active.


## PAR-0051 missing durable row-ID resolution — 2026-11-22

- **id:** hermes-par0051-history-row-resolution-20261122
- **timestamp:** 2026-11-22T03:58:54Z
- **what:** Added bounded profile-scoped persisted-history lookup for prior-message edits whose local user row lacks a durable ID. Exact unique or proven-newest duplicate matches continue through the truncation flow; all uncertain/error outcomes fall back to non-truncating submit, with stale owner receipts rejected.
- **why:** Support editing ordinary Hermes prompts whose local projection omitted the durable row ID without guessing at destructive history boundaries.
- **components:** Native edit UI and lifecycle, authenticated Gateway HTTP worker, pinned history API contract, tests, plan/backlog/completion/architecture docs.
- **type:** Additive chat workflow with fail-closed history mutation.
- **validation:** PASS 25 edit-related regressions and full locked suite (550 library + 15 protocol + 3 launcher). All-target check, formatting/diff, task continuity/state and root/archival package validators remain to be run for this slice. No live Gateway was contacted.
- **performance:** Lookup runs on the bounded HTTP worker; at most 100 requests, 50,000 rows and 16 MB are accepted.
- **risks:** REST offset paging is not snapshot-bound; a changing history can make matching ambiguous. The implementation falls back without truncation when the bounded lookup cannot prove a safe target.
- **follow_up:** Run final validators, create implementation-bound partial evidence, publish to main, then continue with remaining PAR-0051 acceptance gaps.


## PAR-0051 history-row resolution validation — 2026-11-22

- **id:** hermes-par0051-history-row-resolution-validation-20261122
- **timestamp:** 2026-11-22T04:01:58Z
- **what:** Recorded final validation for the bounded history-based prior-message edit fallback.
- **why:** Distinguish complete local headless validation from the still-open runtime and formal acceptance gates.
- **components:** Native edit path, progress overlays, continuity state, capability package validation.
- **type:** Validation update.
- **validation:** PASS 25 edit-filtered tests; 550 library + 15 protocol + 3 launcher; all-target locked check; rustfmt; diff check; both progress generators; 19 continuity tests; task-state validation; root package validation with 77 receipts (YAML parse skipped because PyYAML is unavailable); archival package validation with 491 pending.
- **performance:** No runtime benchmark was requested or changed.
- **risks:** Live Gateway, packaged Windows, concurrent offset-page stability and formal pinned-source acceptance remain unverified.
- **follow_up:** Bind partial evidence to the implementation commit, publish to main, and continue remaining agent-workflow gaps.


## PAR-0051 history-row resolution publication — 2026-11-22

- **id:** hermes-par0051-history-row-resolution-publication-20261122
- **timestamp:** 2026-11-22T04:07:04Z
- **what:** Published implementation commit 5eaf03190a498e0bbf45d0c3bfdde6e3cd4860df and EV-0094 evidence commit to main.
- **why:** Keep the validated partial agent-chat workflow current on the canonical branch.
- **components:** Prior-message edit lifecycle, evidence receipt, capability crosswalk/progress, task handoff checkpoint.
- **type:** Main publication and checkpoint.
- **validation:** Push succeeded; fresh fetch confirms local main and origin/main both equal 9867567ce156337c3a29e4caad151b10770c4d46. Task checkpoint revision 651 / ledger revision 104 validates, with PAR-0051 still in progress.
- **performance:** No runtime behavior changed during publication.
- **risks:** Live Gateway, packaged Windows, concurrent offset-page stability and formal source acceptance remain open.
- **follow_up:** Continue the next dependency-ready agent-workflow gap; keep the overall goal active and protect apps/hermes-native/work/.


### PAR-0060 independent composer drafts — 2026-11-22

Added atomic bounded text-draft persistence keyed by Gateway connection, profile and session; New Chat drafts are scoped too. Empty drafts clear their persisted record, app-instance restart restores unsent text, and nonempty draft/attachment views survive beyond the ordinary 32-view cache limit. Attachment data remains memory-only. Validation: 556 library + 15 protocol + 3 launcher tests, all-target check, formatting and diff checks pass. The initial full run exposed first-render restoration overwriting an already-populated draft; that guard is fixed and the rerun passes. Live Gateway and packaged Windows checks remain separate.


PAR-0060 evidence update: EV-0095 is bound to implementation commit 32d59b04453134876562ab4de8a87f70d0167245. Generated progress now records 98/499 overall requirements reviewed (6 implemented, 91 partial, 1 not implemented, 401 unknown) and 64/176 chat/tool requirements mapped (1 implemented, 62 partial, 1 not implemented, 112 unknown). These are evidence coverage counts, not percent completion.


PAR-0060 publication: implementation `32d59b04453134876562ab4de8a87f70d0167245` and EV-0095/checkpoint `618403c88fd5979659937bd7c4571ba44d5fc4ae` are published to `origin/main`; fresh fetch confirmed exact SHA equality. The operator-owned `apps/hermes-native/work/` remains untracked and untouched.


### PAR-0061 model-change composer preservation — 2026-11-22

Added a rendered model-picker regression proving that an unsent prompt and attached file remain intact after model selection. Matching success changes the model; refusal and confirmation-required receipts preserve composer state; stale-profile receipts are ignored before the active request is consumed. Validation passes: 557 library + 15 protocol + 3 launcher tests, all-target check, rustfmt/diff, progress and task validators, and both capability packages. Live Gateway and packaged Windows behavior remain unverified.


PAR-0061 evidence update: EV-0096 is bound to implementation commit 8c19b9cf5ec5f242dd2d31201176b1a230dafc24. Generated evidence coverage now reads 99/499 overall and 65/176 chat/tool requirements; those counts measure mapped review evidence, not feature completion. Task hermes-par-0061 is tested at checkpoint revision 659 and remains partial.


PAR-0061 publication: implementation `8c19b9cf5ec5f242dd2d31201176b1a230dafc24` and EV-0096/task checkpoint are published to `origin/main`; fresh fetch verified `10968d89f501073e1521751e2d07153e3953a633`. The operator-owned `apps/hermes-native/work/` remains untracked and untouched.

### PAR-0045 inline tool diffs and completion status — 2026-11-22

- **id:** hermes-par0045-inline-diff-20261122
- **timestamp:** 2026-11-22T05:14:14Z
- **what:** Rendered bounded inline diffs in an accessible, expanded-by-default Changes section beside tool summaries. Added/removed lines are distinguished; Advanced tool payload details include the diff. Completion status now reflects explicit structured failures and success, with success taking precedence over stale error metadata.
- **why:** Tool edits and their outcomes are direct agent-workflow information and were being hidden or rendered as generic text.
- **components:** Native transcript renderer, tool completion reducer, implementation evidence generator, progress views, architecture/backlog/completion documentation.
- **type:** Agent feature implementation and evidence mapping.
- **validation:** PASS focused render/status/diff tests; full locked suite 560 library + 15 protocol + 3 launcher; all-target check; rustfmt and diff checks; both progress generators; 19 continuity tests; task-state validator; root package validator (80 prior implementation receipts); archival package validator (491 pending). PAR-0045 recorded partial; broad reference evidence 100/499 and chat/tool evidence 66/176.
- **performance:** Diff preview is bounded to 80,000 characters and 1,500 rendered rows; no benchmark was required or run.
- **risks:** Fixtures do not prove live Gateway result envelopes or packaged Windows rendering. Formal pinned-source acceptance remains pending. Previously working features and operator-owned work were preserved.
- **follow_up:** Create commit-bound partial evidence, update task checkpoint, validate package receipts, publish to main, then continue with the next agent-workflow P0 gap.

PAR-0045 evidence update: EV-0097 is bound to implementation commit `a1570b2030b87c3a1911ab7552f29dbcbd38c41b`. The task is tested at checkpoint revision 663 / ledger revision 111. The generated reference overlays now show 100/499 and 66/176 rows with mapped evidence; these are evidence-review counts, not feature completion. Both capability package validators pass with 81 implementation receipts and 491 archival rows still pending. Live Gateway, packaged Windows, and formal source-paired acceptance remain open.

PAR-0045 main publication: implementation `a1570b2030b87c3a1911ab7552f29dbcbd38c41b` and EV-0097/task checkpoint commit `bff1c6ed57e60b712ecc9ad06d5234241f50776d` were pushed to `origin/main`; fresh fetch confirmed exact equality at bff1c6e. The task remains partial pending live Gateway, packaged Windows and formal source-paired acceptance. The committed task snapshot is consistent; its recorded repository fingerprint predates this publication commit and is reported as Git drift.

### PAR-0059 — Persistent background completion in chat — 2026-11-22T05:39:39Z

- **id:** hermes-par-0059-background-completion-20261122
- **timestamp:** 2026-11-22T05:39:39Z
- **what:** Routed `background.complete` events to the originating active or uniquely cached session transcript as bounded persistent system rows. Empty text is suppressed, repeated task IDs are ignored, late results after Stop are retained, and missing/ambiguous session-profile ownership fails closed.
- **why:** Background agent output must remain visible in the chat that started the work.
- **components:** Hermes Gateway event router, native transcript reducer and presentation projection, progress evidence generator, architecture/backlog/overview docs.
- **type:** Agent workflow implementation and evidence mapping.
- **validation:** PASS 562 library + 15 protocol + 3 launcher tests; all-target locked check; rustfmt/diff; 19 continuity tests; task-state validator; root capability package validator; archival package validator. Generated evidence: 101/499 broad and 67/176 chat/tool rows. These are review coverage, not completion.
- **performance:** Event result text is bounded to 12,000 bytes and task IDs to 512 bytes; no benchmark was requested or run.
- **risks:** Headless fixtures do not prove live Gateway delivery or packaged Windows behavior. Cron history and persistent subagent completion notices are outside the pinned `background.complete` contract; formal source-paired acceptance remains open. Existing functional work and operator-owned runtime files were preserved.
- **follow_up:** Bind partial evidence to the implementation commit, update PAR-0059 checkpoint, validate receipt packages, publish to `main`, then continue with the next dependency-ready P0 agent-workflow feature.

### PAR-0059 evidence checkpoint — 2026-11-22T05:41:50Z

EV-0098 is bound to implementation commit `812292c212627e7f4fabd3b857785de1715f704f`. Task `hermes-par-0059` is tested at checkpoint revision 668 / ledger revision 113. Generated progress reports 101/499 broad rows and 67/176 chat/tool rows with mapped evidence; these counts measure review coverage, not product completion. Root package validation passes with 82 implementation receipts; archival package validation reports 491 pending rows. Live Gateway, packaged Windows, and formal source-paired acceptance remain open. The operator-owned `apps/hermes-native/work/` directory is excluded.

PAR-0059 main publication: implementation commit `812292c212627e7f4fabd3b857785de1715f704f` and EV-0098/task checkpoint commit `f062cf2f5338aa2282426429636508a07d78cb66` were pushed to `origin/main`; fresh fetch confirmed exact local/origin equality at f062cf2. The implementation remains partial pending live Gateway, packaged Windows, and formal source-paired acceptance. The task checkpoint is being refreshed against the published head before work continues.

### PAR-0053 — Verify destructive rewind consent — 2026-11-22T05:57:26Z

- **id:** hermes-par-0053-rewind-confirmation-evidence-20261122
- **timestamp:** 2026-11-22T05:57:26Z
- **what:** Added an accessible headless interaction assertion proving a deep-rewind submit control is disabled before consent and enabled only after explicit checkbox consent and a changed replacement prompt. Existing confirmed-payload and stale-target tests remain the request-side proof.
- **why:** Deep transcript truncation can archive later user turns and must require explicit user consent.
- **components:** Prior-message edit dialog, Gateway truncation request, progress mapping and completion overview.
- **type:** Regression coverage and evidence mapping for preserved functionality.
- **validation:** PASS focused consent test; full locked suite 562 library + 15 protocol + 3 launcher; all-target check, rustfmt/diff, 19 continuity tests, task schema, both generators, root package validator (82 existing receipts), archival package validator (491 pending). Coverage is now 102/499 broad and 68/176 chat/tool; these are evidence counts, not completion.
- **performance:** No runtime behavior changed; no benchmark needed.
- **risks:** Local fixtures do not prove live Gateway or packaged Windows behavior; formal pinned-source acceptance remains open. All prior functional implementation and operator-owned work were preserved. PAR-0066 stays planned until a real terminal surface exists.
- **follow_up:** Bind EV-0099 to the test commit, checkpoint PAR-0053, publish to main, then continue the next dependency-ready agent-workflow feature.

### PAR-0053 revalidation — 2026-11-22T06:01:43Z

The progress mapping now cites PLAN Revision 318, whose tested consent assertion covers the rendered disabled-before-consent and enabled-after-consent interaction. Re-ran the full locked suite (562 library + 15 protocol + 3 launcher), all-target check, format/diff, task continuity (19), task state validation, both progress generators and root/archival capability package validators. Counts remain 102/499 broad and 68/176 scoped; they describe mapped evidence, not feature completion. The root validator reports 82 implementation receipts; the archival validator reports 491 pending. Live Gateway, packaged Windows and formal pinned-source acceptance remain open. Existing `handle_server_event` dead-code warning remains; optional YAML validation was skipped because PyYAML is not installed.

### PAR-0053 evidence and map checkpoint — 2026-11-22T06:06:59Z

EV-0099 binds the passing headless interaction test to implementation commit `c6d355dc6cf6dbc5af2e0238638ef68f42e9b0cb`. The capability now cites EV-0099 and remains `partial` (UI partial, tests verified, backend unknown) because authenticated live Gateway, packaged Windows and formal pinned-source acceptance are still open. Root and archival package validators pass with 83 implementation receipts and 491 archival rows pending. The installed Universal Capability Map checker was run with temporary PyYAML: the repository-wide check still exits 1 with 159 legacy errors and 11 reconciliation items; no capability-specific errors remain for PAR-0053, though it warns that the partial status is lower than a passing headless test result and that repository-root locators are unresolved from the maps-directory invocation. This warning does not establish live integration. Task `hermes-par-0053` is tested at revision 675 / ledger revision 117.

### PAR-0053 task checkpoint — 2026-11-22T06:07:49Z

`hermes-par-0053` is tested with EV-0099 at state revision 676 / task ledger revision 117. The durable progress report now records the current validation run (312 pass, 0 fail, 2 other) and the next step is selecting the next dependency-ready P0 chat/tool requirement. Broad evidence mapping remains 102/499 and scoped chat/tool evidence 68/176; neither count is overall completion. Root and archival package validators pass. The project-owned UCM package remains valid under its local validator; the canonical UCM checker still exits 1 on pre-existing package-wide legacy issues, while reporting no PAR-0053 errors.

### PAR-0053 published to main — 2026-11-22T06:09:24Z

Published implementation/test commit `c6d355dc6cf6dbc5af2e0238638ef68f42e9b0cb` and evidence/task checkpoint commit `1bc2312ca281983c7092a607370b36725d0a1095` to `origin/main`. A fresh fetch confirmed exact local/remote equality at `1bc2312ca281983c7092a607370b36725d0a1095`. EV-0099 remains partial evidence: it proves the explicit confirmation UI and request contract headlessly; live Gateway, packaged Windows and formal pinned-source acceptance are still open. The operator-owned `apps/hermes-native/work/` path remains untouched and excluded.

### PAR-0054 — Follow live transcript growth — 2026-11-22T06:25:50Z

- **id:** PAR-0054
- **timestamp:** 2026-11-22T06:25:50Z
- **what:** Added stable session/profile-keyed egui transcript scroll identity and native stick-to-bottom behavior; added a headless regression for following stream growth, preserving a history offset, and resuming after return to bottom.
- **why:** The pinned Desktop follows new content only while the reader is at the bottom and yields to explicit history reading.
- **components:** `apps/hermes-native/src/app.rs`, progress generator, status overlays, PLAN/BACKLOG/COMPLETION_OVERVIEW.
- **type:** feature and regression evidence.
- **validation:** 563 library + 15 protocol + 3 launcher tests; all-target locked check; rustfmt/diff; 19 continuity tests; task validation; progress generators. Package validators pending EV-0100.
- **performance:** No benchmarked performance change; scroll behavior delegates to egui's native ScrollArea.
- **risks:** No physical Windows wheel, live Gateway stream, packaged app, or formal pinned-source acceptance yet. Existing `handle_server_event` dead-code warning remains.
- **follow_up:** Bind EV-0100 to the implementation commit, validate the capability package and task ledger, publish to main, then continue the next dependency-ready agent workflow.

### PAR-0054 evidence and task checkpoint — 2026-11-22T06:31:37Z

EV-0100 binds the transcript follow regression to implementation commit `04743204c45ea5b4b9894166dbebacf43ca75ef6`. The capability map records overall partial, UI partial, tests verified, and backend unknown. The headless test uses egui scroll state to prove follow/preserve/relock; it does not synthesize physical Windows input. Root capability package passes with 84 implementation receipts; archival package passes with 491 pending. Canonical UCM checker exits 1 on 159 legacy package-wide errors and 11 reconciliation items; it reports no PAR-0054 errors, but warns about the repository-relative locator when invoked from `maps/` and that headless pass evidence could support verified. We keep PAR-0054 partial because live Gateway streaming, packaged Windows, physical input and formal source acceptance are unverified. The task is tested at revision 683 / ledger 119. Progress is 103/499 broad and 69/176 chat/tool evidence rows; these are mapping counts, not completion.

### PAR-0054 published to main — 2026-11-22T06:32:23Z

Published transcript-follow implementation commit `04743204c45ea5b4b9894166dbebacf43ca75ef6` and evidence/task checkpoint commit `e22e4e1c774015f1ae1528a0e0e24c2fc963cbf2` to `origin/main`. Fresh fetch confirms exact local/remote equality at `e22e4e1c774015f1ae1528a0e0e24c2fc963cbf2`. PAR-0054 remains partial: headless egui state proves bottom-follow, history offset preservation, and relock, while physical Windows input, live Gateway streaming, packaged behavior and formal source acceptance remain open. `apps/hermes-native/work/` remains operator-owned and untouched.


### PAR-0527 — Reject malformed successful session refreshes — 2026-11-22

- **id:** PAR-0527-malformed-sidebar-response
- **timestamp:** 2026-11-22T11:33:14Z
- **what:** Required a valid recent-session slice or explicit source-shaped failure details before accepting a sidebar response. Malformed refreshes now return an error without replacing cached sessions; valid empty results and explicit failures retain their semantics.
- **why:** A JSON `{}` response previously defaulted into an empty success and caused all-profile refresh to hide cached session rows.
- **components:** `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, session refresh docs and tests.
- **type:** behavior and regression coverage
- **validation:** PASS two focused headless regressions; full locked suite 592 library + 15 protocol + 3 launcher; all-target check; 19 task continuity tests; task/package validators; progress generators; rustfmt and diff check. YAML parsing was skipped because PyYAML is unavailable.
- **performance:** No additional requests or runtime work; one in-memory response-shape check per sidebar refresh.
- **risks:** Live authenticated Gateway and packaged Windows behavior are unverified.
- **follow_up:** Bind commit-specific partial EV-0113, checkpoint PAR-0527, and publish to `main`.


### PAR-0527 evidence — 2026-11-22

- **id:** PAR-0527-EV-0113
- **timestamp:** 2026-11-22T11:39:29Z
- **what:** Bound the app cache-preservation regression to implementation commit `b1baf05e20c02b8e28638c93db0236a2119b6eb2` and added EV-0113 for `malformed-sidebar-response-preserves-sessions`. Updated the crosswalk and progress generator; the map remains partial.
- **why:** Make the tested slice visible in current completion reporting without implying full capability completion.
- **components:** UCM capability/evidence, implementation progress generator, crosswalk, completion overview, task ledger.
- **type:** evidence and progress record
- **validation:** Scenario passes; task checkpoint revision 763 validates. Progress now reports 112/499 broad and 78/176 chat/tool evidence mappings. Root package validator passes 565 behaviors, 15 source inspections and 97 implementation receipts; archival package passes with 491 pending.
- **performance:** No runtime behavior changed.
- **risks:** Canonical UCM package-wide check retains 162 legacy errors and 11 unrelated reconciliation items; no EV-0113-specific issue. Live Gateway and packaged Windows remain unverified.
- **follow_up:** Publish implementation and evidence to main, then verify exact origin/main equality.


### PAR-0527 publication — 2026-11-22

- **id:** PAR-0527-PUBLISH
- **timestamp:** 2026-11-22T11:41:13Z
- **what:** Published implementation commit `b1baf05e20c02b8e28638c93db0236a2119b6eb2` and EV-0113/progress commit `4559146774428da8c59eaa41b96afdb394a90740` to `origin/main`.
- **why:** Deliver the verified malformed-response safety slice and its partial capability evidence to the requested branch.
- **components:** Native Gateway sidebar response decoder, session refresh UI, completion progress, UCM and task ledger.
- **type:** publication
- **validation:** Fresh fetch confirmed local main and origin/main both equal `4559146774428da8c59eaa41b96afdb394a90740`; task checkpoint revision 764 validates.
- **performance:** No runtime behavior changed during publication.
- **risks:** Authenticated live Gateway and packaged Windows acceptance remain open; the capability stays partial.
- **follow_up:** Select the next dependency-ready agent workflow task from the current progress report.


### PAR-0528 profile main-model settings — 2026-11-22

- **id:** PAR-0528-MAIN-MODEL
- **timestamp:** 2026-11-22T12:34:15Z
- **what:** Added Settings → Main model for the selected Hermes profile. It reads the model catalog and writes the profile-wide `scope=main` assignment with an explicit confirmation step when Hermes requires it. New-chat overrides remain session-local.
- **why:** The first usable Hermes agent UI needs both a configurable model default for future sessions and a separate per-chat override.
- **components:** Native Rust/egui Settings, Gateway HTTP worker, model assignment receipt handling, UCM capability evidence.
- **type:** feature
- **validation:** Rendered egui test and profile-authenticated Gateway HTTP fixture pass for confirmation, success, denial, and stale-connection refusal; full locked suite 596 + 15 + 3, all-target cargo check, rustfmt, and 19 continuity tests pass. Implementation commit: `294a88e`.
- **performance:** Requests use the existing bounded HTTP worker and 8-second request timeout; no benchmarked performance change claimed.
- **risks:** Live Gateway, packaged Windows behavior, and formal pinned-source acceptance remain open; capability stays partial.
- **follow_up:** Validate updated evidence and project package, publish docs/evidence/checkpoint to main, then select the next goal-relevant agent workflow gap.


### PAR-0528 main publication — 2026-11-22

- **id:** PAR-0528-PUBLISH-MAIN-MODEL
- **timestamp:** 2026-11-22T12:45:03Z
- **what:** Published implementation `294a88eb73760c14f0e977182d6f4331f956e34e` and evidence/docs/checkpoint commit `1a7f222a211e65745208dad0e7b3507a199360f6` to `origin/main`.
- **why:** Deliver the verified profile-default model settings slice and its evidence while preserving chat-local model overrides.
- **components:** `robertheadley/Doofus-Dock-Hermes`, `main`, PAR-0528, UCM EV-0115, task checkpoint revision 779.
- **type:** publication
- **validation:** Push succeeded; fresh fetch confirmed local main and origin/main both equal `1a7f222a211e65745208dad0e7b3507a199360f6`.
- **performance:** No runtime performance change claimed.
- **risks:** Live Gateway and packaged Windows behavior remain unverified; source-acceptance inventory remains pending; `apps/hermes-native/work/` remains operator-owned and untracked.
- **follow_up:** Inspect and implement the next goal-relevant P0 chat/prompt lifecycle gap, currently PAR-0051.


### PAR-0051 offset-page mutation guard — 2026-11-22

- **id:** PAR-0051-HISTORY-PAGE-GUARD
- **timestamp:** 2026-11-22T13:03:21Z
- **what:** Added one-row overlap checks between adjacent bounded oldest-first edit-history pages. A changed boundary cannot resolve a truncating edit target; the existing app falls back to a non-truncating prompt.
- **why:** Offset pagination has no pinned-source snapshot cursor, so page shifts must fail closed instead of joining different history views.
- **components:** Native Gateway history lookup, headless HTTP fixtures, PAR-0051 docs and EV-0116. Existing edit, attachment, consent, recovery and composer behavior is retained.
- **type:** safety and reliability
- **validation:** Full locked suite 598 + 15 + 3, all-target cargo check, rustfmt/diff, and 19 task continuity tests pass. Implementation commit: `eb41edfa7a6af9439afb7d9143b61fedee6d5f13`.
- **performance:** Each history page after the first shifts the next request by one row; page count and payload remain within the existing 100-page, 50,000-unique-row and 16 MB bounds. No benchmark was run.
- **risks:** The endpoint cannot provide a stable snapshot; writes within a single-page read or after lookup, live Gateway races, packaged Windows behavior and formal source-paired acceptance remain open.
- **follow_up:** Validate package/UCM evidence, checkpoint PAR-0051, and publish implementation plus evidence/docs to main.


### PAR-0051 main publication — 2026-11-22

- **id:** PAR-0051-PUBLISH-MAIN
- **timestamp:** 2026-11-22T13:11:51Z
- **what:** Published implementation `eb41edfa7a6af9439afb7d9143b61fedee6d5f13` and EV-0116/progress/docs commit `9efc7d25ee8f6f16458402a3561e9b5be6653284` to `origin/main`.
- **why:** Deliver the verified additive pagination guard and preserve auditable partial evidence on the user's requested branch.
- **components:** `robertheadley/Doofus-Dock-Hermes`, `main`, PAR-0051, UCM EV-0116, task checkpoint revision 792.
- **type:** publication
- **validation:** Push succeeded; fresh fetch confirmed local main and origin/main both equal `9efc7d25ee8f6f16458402a3561e9b5be6653284`.
- **performance:** No runtime benchmark was run.
- **risks:** Single-page/post-lookup history writes, live Gateway, packaged Windows and formal source acceptance remain open; operator-owned `apps/hermes-native/work/` remains untouched.
- **follow_up:** Continue the first usable agent-workflow goal with the next dependency-ready P0 feature; retain PAR-0051's remaining external acceptance gates.
- **id:** hermes-par0514-approval-ui-gateway-20261122
- **timestamp:** 2026-11-22T13:24:51Z
- **what:** Recorded partial evidence that the approval UI sends only Hermes-permitted choices to the originating Gateway request.
- **why:** Preserve auditable evidence for existing approval functionality and its new app-to-Gateway regression.
- **components:** PAR-0514, `app.rs`, Gateway request actor, EV-0117, progress overlay.
- **type:** evidence_and_docs
- **validation:** Full locked suite 599 library + 15 protocol + 3 launcher; all-target check; rustfmt/diff; 19 task-continuity checks; progress generator passed.
- **performance:** No runtime benchmark was run.
- **risks:** Live Gateway, packaged Windows, and formal source acceptance remain open; existing functionality is retained.
- **follow_up:** Implement the next dependency-ready scoped agent workflow gap.

- **id:** hermes-par0515-clarify-ui-gateway-20261122
- **timestamp:** 2026-11-22T13:40:00Z
- **what:** Added partial evidence for the batch clarification UI sending ordered answers to Gateway and skipping the exact originating request.
- **why:** Close the missing interaction proof for an existing agent workflow while retaining all current behavior.
- **components:** PAR-0515, `app.rs`, `gateway.rs`, EV-0118, generated progress records.
- **type:** evidence_and_docs
- **validation:** Full locked suite 601 library + 15 protocol + 3 launcher; all-target check; formatting/diff; 19 task continuity tests.
- **performance:** No runtime benchmark was run.
- **risks:** Live Gateway, packaged Windows and source-paired acceptance remain open.
- **follow_up:** Keep PAR-0515 partial and continue the next dependency-ready workflow feature.

- **id:** hermes-par0516-sudo-ui-gateway-20261122
- **timestamp:** 2026-11-22T14:08:50Z
- **what:** Routed Gateway sudo requests to a masked, owner-scoped password prompt and exact JSON-RPC response; cleared egui undo snapshots and rejected ambiguous cancellation/retry paths.
- **why:** Hermes agent workflows need secure, explicit handling of backend administrator prompts instead of automatic method-not-found refusal.
- **components:** PAR-0516, sudo card, Gateway actor, request/receipt lifecycle, EV-0119.
- **type:** feature_and_test
- **validation:** Full locked suite 606 library + 15 protocol + 3 launcher; all-target cargo check; rustfmt/diff checks; focused sudo tests pass.
- **performance:** No runtime benchmark was run.
- **risks:** Secret is transiently allocated while JSON-RPC response is serialized and sent; live Gateway, packaged Windows and formal pinned-source acceptance remain open.
- **follow_up:** Continue the next dependency-ready agent-workflow capability; preserve this feature and its partial evidence.


- **id:** hermes-native-par0517-vault-unlock-20261122
- **timestamp:** 2026-11-22T22:17:00Z
- **what:** Added masked native vault unlock requests and explicit server-request capability advertisement for local and remote Gateway attachments.
- **why:** Hermes password-manager tool flows must be answerable through the correct profile/session/request without persisting or leaking the master password.
- **components:** PAR-0517, PAR-0520, Gateway actor, vault unlock prompt, EV-0120, EV-0121, generated progress.
- **type:** feature_and_test
- **validation:** PASS 613 library + 15 protocol + 3 launcher tests; all-target locked cargo check; rustfmt/diff; 19 task-continuity tests; progress and package validators.
- **performance:** No runtime benchmark was run.
- **risks:** A normal JSON string copy exists transiently during response serialization/send; live authenticated Gateway, packaged Windows and formal pinned-source acceptance remain open.
- **follow_up:** Verify authenticated local/remote Gateway behavior and continue the next dependency-ready agent-workflow gap.

- **id:** hermes-par0508-gateway-reconnect-20261122
- **timestamp:** 2026-11-22T23:02:33Z
- **what:** Added pinned-policy Gateway reconnect, profile/session event replay, and no prompt resubmission after uncertain transport loss.
- **why:** Preserve agent chat and tool workflows across ordinary Gateway network drops without duplicating user prompts.
- **components:** PAR-0508, PAR-0511, `GatewayDiscovery`, `session.events.since`, EV-0122, EV-0123.
- **type:** feature_and_test
- **validation:** Full locked suite 617 library + 15 protocol + 3 launcher; all-target cargo check; rustfmt/diff; task continuity 19/19.
- **performance:** No runtime benchmark was run.
- **risks:** Live Gateway and packaged Windows behavior are unverified; auxiliary HTTP/profile workers do not rebind after endpoint changes.
- **follow_up:** Verify live local/remote reconnect and rebind secondary workers when the endpoint changes.

- **id:** hermes-par0503-attach-scope-audit-20261122
- **timestamp:** 2026-11-22T23:21:06Z
- **what:** Documented the difference between pinned Desktop's host-scoped local Gateway attach and the public profile-scoped `gateway attach --json` CLI.
- **why:** Avoid replacing working cross-profile UI access with a narrower ticket while implementing agent use.
- **components:** PAR-0503, `GatewayDiscovery`, `gateway ensure`, public `gateway attach`, profiles.list and profile registry access.
- **type:** architecture_audit
- **validation:** Read-only comparison of pinned `gateway_runtime_cli.py`, `gateway_runtime_attach.py`, Gateway internals docs, Desktop local Gateway path, and native `gateway.rs`; durable task state validates at revision 841.
- **performance:** No runtime benchmark was run.
- **risks:** Public attach subprocess JSON contract and authenticated live Gateway behavior remain untested.
- **follow_up:** Inspect native profile registry scopes, then add the public attach path only if host-scoped capability can be preserved or cleanly separated.

### PAR-0064 credits opt-in and retry receipt — 2026-11-23

- **id:** hermes-par0064-credits-opt-in-20261123
- **timestamp:** 2026-11-23T17:05:02Z
- **what:** Added the explicit usage_limit.credits.action=use_credits action, profile-scoped billing.keep_going_on_credits { enabled: true } RPC, success-receipt-gated retry of the original failed prompt, and unknown-outcome handling after disconnect.
- **why:** Implement the pinned Hermes recovery action while ensuring a billing refusal, stale owner receipt, or uncertain mutation cannot replay a prompt.
- **components:** Hermes Native chat recovery, Gateway actor, PAR-0064, UCM EV-0139.
- **type:** feature, test, evidence, documentation.
- **validation:** Full locked suite passes 653 library + 15 automation protocol + 3 launcher tests; all-target check, Windows debug build, rustfmt, and diff checks pass. Source commit: 46bd1e75a1ac134645e451ea7306635bf75f00b6.
- **performance:** No benchmark claim; the opt-in uses the existing Gateway actor and prompt-submission path.
- **risks:** PAR-0064 remains partial. Broader billing settings, remaining error codes, authenticated live Gateway, packaged Windows, and formal source-paired acceptance remain open.
- **follow_up:** Regenerate progress, validate the parity package and task ledger, then publish the evidence/documentation commit to main.

### PAR-0064 credits recovery publication — 2026-11-23

- **id:** hermes-par0064-credits-publication-20261123
- **timestamp:** 2026-11-23T17:14:40Z
- **what:** Published source commit 46bd1e75 and evidence/documentation commit e3880106 to origin/main.
- **why:** Keep the user-selected main branch current with the tested credits opt-in and its partial evidence.
- **components:** Doofus-Dock-Hermes main, PAR-0064, EV-0139, progress overlays.
- **type:** publication.
- **validation:** Fresh fetch confirmed local main and origin/main both equal e3880106d140609d8a170da2a418a523ed2fa151; package validator reports 123 implementation receipts.
- **performance:** Not applicable.
- **risks:** Live Gateway, packaged Windows, broader billing settings, complete error taxonomy, and source-paired acceptance remain open; PAR-0064 stays partial.
- **follow_up:** Continue the remaining pinned PAR-0064 error actions.

### PAR-0064 safe billing-error fallback — 2026-11-23

- **id:** hermes-par0064-safe-billing-fallback-20261123
- **timestamp:** 2026-11-23T17:25:54Z
- **what:** Added an accessible Nous billing-portal route for structured billing failures when provider is Nous or missing; other providers fail closed to model choice.
- **why:** Cover the pinned billing recovery path without misrouting a third-party provider to an unrelated account portal.
- **components:** Hermes Native failed-turn actions, PAR-0064, UCM EV-0140.
- **type:** feature, test, evidence, documentation.
- **validation:** Full locked suite passes 653 library + 15 protocol + 3 launcher tests; all-target check, Windows debug build, rustfmt, and diff checks pass. Source commit: e47b777ee5a226f5863d0d540e9bf031f6d42437.
- **performance:** No benchmark claim; the action routes through the existing native navigation command.
- **risks:** This fallback lacks the pinned same-provider billing-block descriptor, in-app Billing surface, and billing recovery retry. Live Gateway, packaged Windows, remaining error actions, and source-paired acceptance remain open.
- **follow_up:** Record EV-0140, validate generated progress/package/task state, and publish to main.

### PAR-0064 safe billing fallback publication — 2026-11-23

- **id:** hermes-par0064-billing-publication-20261123
- **timestamp:** 2026-11-23T17:28:23Z
- **what:** Published source commit e47b777 and evidence/documentation commit 3706f959 to origin/main.
- **why:** Keep the selected main branch current with the tested billing-error fallback and partial evidence.
- **components:** Doofus-Dock-Hermes main, PAR-0064, EV-0140, progress overlays.
- **type:** publication.
- **validation:** Fresh fetch confirmed local main and origin/main both equal 3706f9597751ddfaab42b800d2040d245ea384e3; package validation reports 124 implementation receipts.
- **performance:** Not applicable.
- **risks:** Provider/session billing-block recovery, full billing settings, live Gateway, packaged Windows, remaining error actions, and source-paired acceptance remain open; PAR-0064 stays partial.
- **follow_up:** Continue the remaining pinned PAR-0064 error actions.

### PAR-0064 agent initialization recovery — 2026-11-23

- **id:** hermes-par0064-agent-init-recovery-20261123
- **timestamp:** 2026-11-23T17:37:56Z
- **what:** Added a dedicated `agent_init_failed` error card and a recovery action that opens the active chat's model picker.
- **why:** Match the pinned secondary `open_model_settings` action while keeping Retry governed by Hermes' `retryable` flag.
- **components:** Hermes Native transcript error card, PAR-0064, egui recovery action.
- **type:** feature, validation, evidence.
- **validation:** Focused mapping and rendered UI regressions pass; full locked suite 654 + 15 + 3, all-target check, Windows debug build, rustfmt/diff, package validator (125 receipts), task-continuity 19/19 and persisted task validation pass.
- **performance:** No benchmark claim; the action uses the existing in-app model-picker route.
- **risks:** Provider API-key and OAuth recovery remain unavailable without a native provider-credential API. Live Gateway, packaged Windows and formal source-paired acceptance remain open.
- **follow_up:** Published source and EV-0141 to origin/main; fresh fetch confirmed exact refs.

### PAR-0064 agent initialization recovery publication — 2026-11-23

- **id:** hermes-par0064-agent-init-publication-20261123
- **timestamp:** 2026-11-23T17:47:40Z
- **what:** Published source commit d80fc07 and EV-0141/progress commit c9f37fe, followed by plan publication record fa9b13b.
- **why:** Keep the user-selected main branch current with the validated agent-init recovery and its evidence.
- **components:** Hermes Native, PAR-0064, EV-0141, progress ledger.
- **type:** publication.
- **validation:** Fresh fetch confirms local main and origin/main both equal fa9b13bb687c78a25f627935147b145fad5f7588d.
- **performance:** Not applicable.
- **risks:** PAR-0064 remains partial; provider credential recovery, live Gateway, packaged Windows and formal source-paired acceptance remain open.
- **follow_up:** Continue the remaining pinned PAR-0064 recovery actions.

### PAR-0064 payload size and TLS recovery — 2026-11-23

- **id:** hermes-par0064-payload-tls-recovery-20261123
- **timestamp:** 2026-11-23T17:58:00Z
- **what:** Mapped `payload_too_large` to owner-scoped context compression with Start new session fallback, and `ssl_cert_verification` to the pinned TLS trust guide.
- **why:** Match pinned Hermes error actions using existing safe chat compression and a fixed documentation URL.
- **components:** Hermes Native transcript errors, PAR-0064, Gateway compression command, host URL navigation.
- **type:** feature, validation, evidence.
- **validation:** Full locked suite 656 library + 15 protocol + 3 launcher tests pass; all-target check, Windows debug build, rustfmt, focused rendered tests and diff checks pass.
- **performance:** No benchmark claim; compression uses the existing asynchronous Gateway writer.
- **risks:** Attachment removal remains user-directed; live Gateway, real TLS remediation, packaged Windows and formal source-paired acceptance remain open.
- **follow_up:** Record EV-0142/0143, update generated progress and publish to main.

### PAR-0064 format and upstream recovery — 2026-11-23

- **id:** hermes-par0064-format-upstream-recovery-20261123
- **timestamp:** 2026-11-23T18:08:09Z
- **what:** Added specific `format_error` recovery with model choice and fresh-session fallback, and mapped `upstream_blocked` to the active chat model picker.
- **why:** Match pinned `switch_provider` behavior through the native session model controls without changing profile defaults.
- **components:** Hermes Native failed-turn card, PAR-0064, model-picker recovery.
- **type:** feature, validation, evidence.
- **validation:** Full locked suite 657 + 15 + 3, all-target check, Windows debug build, rustfmt, focused rendered test and diff check pass.
- **performance:** No benchmark claim; both actions use existing UI navigation.
- **risks:** The source free-tier sign-in flow requires a separate desktop OAuth/session host and is not represented by a model change. Live Gateway and source-paired acceptance remain open.
- **follow_up:** EV-0144 is recorded with source commit 67fa656046beedddc75a24aaee7447d7c9c4f5c8; publish the validated slice to main.
### PAR-0064 Hermes Free error recovery — 2026-11-23

- **id:** hermes-par0064-free-tier-recovery-20261123
- **timestamp:** 2026-11-23T18:16:53Z
- **what:** Added code-specific explanations for all seven pinned free_tier_* failures, preserved backend guidance, and routed to the session model picker.
- **why:** Give users a usable native recovery path without pretending the Hermes Free sign-in host exists.
- **components:** Hermes Native chat error cards, PAR-0064.
- **type:** feature, validation, evidence.
- **validation:** 658 library + 15 automation-protocol + 3 launcher tests; all-target check, Windows debug build, rustfmt and diff check pass.
- **performance:** No benchmark claim.
- **risks:** Free-tier sign-in and live Gateway acceptance remain open.
- **follow_up:** Published source and evidence commits to origin/main at 5a0b983256daa7b611086de447d81e9806f05422; local and remote main match.
### PAR-0069/0070 session listing and resume evidence — 2026-11-23

- **id:** hermes-par0069-0070-session-evidence-20261123
- **timestamp:** 2026-11-23T18:29:10Z
- **what:** Reviewed existing session list/archive/create/resume behavior and mapped both P0 requirements as partial with EV-0146 and EV-0147.
- **why:** The scoped progress view had treated working native behaviors as unknown because the requirements lacked explicit evidence mapping.
- **components:** Hermes Native Gateway/session lifecycle, chat/tool progress, pinned capability map.
- **type:** evidence, validation, documentation.
- **validation:** Focused session-list, archive-page, create-first-prompt, saved-session open, and resume ownership tests pass; progress generator now maps 88/176 scoped requirements; package validator accepts 131 implementation receipts.
- **performance:** No benchmark claim.
- **risks:** Headless fixtures do not prove authenticated live Gateway or packaged Windows/source-paired behavior.
- **follow_up:** Continue review with remaining P0 session lifecycle and agent tool workflow requirements.

### PAR-0071..0082 session lifecycle evidence review — 2026-11-23

- **id:** hermes-par0071-0082-session-lifecycle-evidence-20261123
- **timestamp:** 2026-11-23T18:48:30Z
- **what:** Mapped existing session archive distinction, title/ID search, pin, rename, archive/restore, confirmed deletion and paged export as partial. Added valid evidence EV-0148..EV-0155 and appended EV-0156/EV-0157 to correct the active pass evidence without rewriting published history.
- **why:** The feature ledger was leaving already-tested native session behaviors as unknown or was inheriting invalid historical evidence outcomes.
- **components:** Hermes Native session lifecycle, capability map, scoped and broad progress generators.
- **type:** evidence, validation, documentation.
- **validation:** Focused session tests passed in the preceding run. Progress generators report 130/499 broad reviewed and 96/176 chat/tool mapped (1 implemented, 94 partial, 1 not implemented, 80 unknown). Package validator accepts 141 implementation receipts; YAML/JSON parsing and task state validation pass.
- **performance:** No benchmark claim.
- **risks:** Session import and fork/branch remain gaps. All reviewed session rows remain partial pending authenticated live Gateway, packaged Windows and source-paired acceptance. UCM still reports historical schema errors and 9 reconciliation items.
- **follow_up:** Implement owner-fenced secure `vault.code` and `vault.save_login` prompts; then address session import/fork, authenticated Gateway and packaged acceptance. Antigravity headless command execution was denied by policy; no blanket bypass was used.

### Revision 360 — Secure vault code and saved-login requests — 2026-11-23

- **id:** hermes-vault-code-save-login-20261123
- **timestamp:** 2026-11-23T19:22:00Z
- **what:** Added native `vault.code` and `vault.save_login` prompt handling through the same-socket Gateway actor, including ephemeral input, masked password, request ownership and unknown-outcome fencing.
- **why:** These are direct agent-workflow interactions required for authenticating to tools and websites during Hermes runs.
- **components:** Hermes Native app, Gateway actor, secure prompt lifecycle, completion overview and backlog.
- **type:** implementation, security, validation, documentation.
- **validation:** 664/664 native library tests pass; all-target locked `cargo check`, rustfmt and `git diff --check` pass. The fixed-source package validator passes 565 inventory rows and 141 implementation receipts; its initial run skipped PyYAML parsing, which is being rerun through the package runner.
- **performance:** No performance change measured or claimed.
- **risks:** Authenticated live Gateway, packaged Windows and formal source-paired acceptance remain open. PAR-0517 remains specific to `vault.unlock_prompt`; code/login support is disclosed separately and excluded from its status.
- **follow_up:** Run authenticated Gateway and packaged Windows acceptance; continue session import/fork and remaining P0 agent workflows. Antigravity CLI returned no usable headless audit findings without a permission bypass.

### Revision 361 — Vault prompt lifecycle review fixes — 2026-11-23

- **id:** hermes-vault-prompt-review-fixes-20261123
- **timestamp:** 2026-11-23T19:35:00Z
- **what:** Decline `vault.code` and `vault.save_login` requests for interrupted sessions; ignore duplicate active request IDs; cover Save Login unknown outcomes and no-resend behavior. Stabilized the scheduled-retry test's asynchronous receive and timestamp fixture.
- **why:** Prevent credential prompts after Stop and prevent a duplicate ID from settling the still-active request.
- **components:** Native app server-request router, vault prompt lifecycle, Gateway test transport.
- **type:** security, correctness, tests, documentation.
- **validation:** Full locked suite: 667 library + 15 protocol + 3 launcher pass. All-target locked check, rustfmt, diff check, persisted task validation and PyYAML-enabled fixed-source package validation pass.
- **performance:** No performance change measured or claimed.
- **risks:** Transient JSON values in the outbound Gateway transport are not zeroized; live Gateway and packaged Windows behavior remain unverified.
- **follow_up:** Complete authenticated Gateway and packaged Windows acceptance; continue remaining agent workflow gaps.

### Publication — Secure vault agent prompts — 2026-11-23

- **id:** publish-vault-agent-prompts-d327bc8
- **timestamp:** 2026-11-23T19:38:13Z
- **what:** Published commit `d327bc818ea998767a4215392286bcc483f600ab` to `origin/main`.
- **why:** Make the tested vault code and saved-login agent workflow implementation current on the operator-selected branch.
- **components:** Hermes Native Rust UI, Gateway actor and implementation documentation.
- **type:** publication.
- **validation:** Post-push fetch confirmed local `main` equals `origin/main` at `d327bc818ea998767a4215392286bcc483f600ab`. Full suite (667 + 15 + 3), all-target locked check and package/task validators passed before push.
- **performance:** No performance change measured.
- **risks:** `.doofus` task runtime and `apps/hermes-native/work/` remain local and uncommitted by design.
- **follow_up:** Continue the first-version agent workflow gaps; live Gateway and packaged Windows acceptance remain open.


### 2026-11-23 — Native secret server prompt

- **id:** hermes-secret-request-20261123
- **what:** Added masked, ephemeral handling for the pinned Gateway `secret` request and documented it with the existing vault code and saved-login prompts.
- **why:** Skills and tool workflows can require a secret mid-turn; the agent UI must answer that exact request without exposing the value in accessibility text, status, or replay.
- **components:** `apps/hermes-native/src/app.rs`, `apps/hermes-native/src/gateway.rs`, agent workflow documentation.
- **type:** feature
- **validation:** 670 library, 15 protocol and 3 launcher tests; all-target locked check; rustfmt and diff checks.
- **performance:** No benchmark required; prompt rendering remains local.
- **risks:** JSON-RPC serialization temporarily holds a normal JSON value copy; live authenticated Gateway and packaged Windows behavior remain unverified.
- **follow_up:** Validate against an authenticated Gateway and packaged Windows app; continue session import and branch/fork workflows.


### 2026-11-23 — Branch saved Hermes sessions

- **id:** hermes-stored-session-branch-20261123
- **what:** Added an accessible Sessions-row branch action with exact revision/generation fencing, safe unknown-outcome retry, child navigation, and parent preservation.
- **why:** Forking a conversation is part of the pinned agent conversation lifecycle.
- **components:** `apps/hermes-native/src/app.rs`, `apps/hermes-native/src/gateway.rs`, session workflow docs.
- **type:** feature
- **validation:** Full locked suite: 675 library + 15 protocol + 3 launcher tests; all-target locked check; rustfmt/diff checks.
- **performance:** No benchmark required; the branch flow reads a lightweight session snapshot before the mutation.
- **risks:** Branch-from-message and authenticated live Gateway/packaged Windows acceptance remain open.
- **follow_up:** Implement session import and branch-from-message; verify the branch against an authenticated Gateway and packaged Windows build.

- **id:** hermes-par0081-evidence-20261123
- **what:** Bound stored-session branch tests to EV-0158 and updated the source capability definition, crosswalk, UI inventory, full map, and generated progress ledgers.
- **why:** The evidence ledger still marked a newly implemented pinned workflow as unknown.
- **components:** `apps/hermes-native/tools/build_implementation_progress.py`, `apps/hermes-native/docs/status/`, `apps/hermes-native/docs/COMPLETION_OVERVIEW.md`, pinned capability maps.
- **type:** evidence
- **validation:** Full locked Rust suite 675+15+3; all-target locked check; package validator; regenerated ledgers report 97/176 scoped and 131/499 broad mapped/reviewed.
- **performance:** No runtime code or benchmark impact.
- **risks:** Headless evidence only; authenticated Gateway, packaged Windows, transcript-position branching, and formal source-paired acceptance remain open.
- **follow_up:** Implement foreign session import and continue live runtime acceptance.

### Revision 365 — Import foreign Hermes sessions — 2026-11-23

- **id:** hermes-par0082-foreign-session-import-20261123
- **what:** Added paged Claude Code/Codex foreign-session listing, bounded preview and an explicit Continue/Open import flow reachable from Sessions and the command palette.
- **why:** Importing prior agent conversations is part of the requested Hermes agent workflow.
- **components:** Native Gateway RPC actor, `/session-import` route and route lifecycle, Sessions sidebar, command palette, profile/connection ownership fencing.
- **type:** feature
- **validation:** Focused foreign-session suite 7/7; full locked suite 682 library + 15 protocol + 3 launcher; locked all-target check, rustfmt check and diff check pass.
- **performance:** No performance behavior changed; page and preview sizes remain bounded by the pinned source contract.
- **risks:** Authenticated Gateway behavior, packaged Windows interaction and formal source-paired acceptance remain unverified. PAR-0082 remains partial.
- **follow_up:** Bind capability-map evidence after the source commit; validate against an authenticated Gateway and packaged Windows build.

### Revision 366 — Render foreign session previews as safe Markdown — 2026-11-23

- **id:** hermes-par0082-markdown-preview-20261123
- **what:** Reused the native transcript CommonMark renderer for foreign preview messages and discarded file-preview destinations for this view.
- **why:** Foreign sessions must render Markdown consistently without treating their paths as belonging to the currently active local project.
- **components:** Session Import preview renderer and focused headless UI test.
- **type:** fix
- **validation:** Preview regression passes for Markdown table/code rendering and for blocked `javascript:` and relative-file navigation; full locked suite 682+15+3; all-target locked check, rustfmt check and focused diff check pass.
- **performance:** Reuses the app's existing CommonMark cache; message sizes remain bounded by the pinned preview contract.
- **risks:** Authenticated Gateway and packaged Windows verification remain open.
- **follow_up:** Continue PAR-0082 live/source-paired acceptance.

### Revision 367 — Preserve newer foreign session requests — 2026-11-23

- **id:** hermes-par0082-stale-request-fencing-20261123
- **what:** Foreign list and preview responses now match the current request and owner before clearing pending state; stale replies cannot cancel newer source or selection requests.
- **why:** An old source-list response or earlier row preview could otherwise consume the pending state for the current request and leave the view stuck.
- **components:** Foreign-session list/preview event handlers and two async race regressions.
- **type:** fix
- **validation:** Focused stale-response tests 2/2; full locked suite 684 library + 15 protocol + 3 launcher; all-target check, rustfmt and diff checks pass.
- **performance:** No runtime performance impact.
- **risks:** Live Gateway and packaged Windows behavior remain open.
- **follow_up:** Continue PAR-0082 live/source-paired acceptance.

### Revision 368 — Exercise clipboard image paste through the composer — 2026-11-24

- **id:** hermes-par0062-clipboard-image-integration-2026-11-24
- **what:** Added a headless UI integration regression that pastes an image into the focused composer and verifies the profile/session-scoped Gateway attachment command and staged PNG.
- **why:** Clipboard image handling existed in helpers and app code, but evidence did not exercise the complete user-facing composer path.
- **components:** Composer UI, clipboard image staging, Gateway attachment dispatch, UCM evidence and progress records.
- **type:** test
- **validation:** Focused integration regression passed; full locked workspace suite passed (716 library, 15 automation protocol, 3 launcher); all-target locked cargo check, rustfmt, diff, 19 task-continuity checks and capability package validation passed.
- **performance:** Test-only change; no runtime performance impact.
- **risks:** Real Windows clipboard delivery, authenticated Gateway upload outcomes and packaged-app behavior remain unverified; PAR-0062 stays partial.
- **follow_up:** Exercise paste, picker and drag/drop against the installed Windows app and authenticated Hermes Gateway.
### Revision 369 — Match native UI typography to pinned Hermes — 2026-11-24

- **id:** hermes-source-typography-20261124
- **what:** Switched proportional UI text to the pinned Hermes platform font stack (Segoe UI first on Windows), made JetBrains Mono the first monospace face, retained Fira Code as the secondary monospace face, and aligned heading family, sizes, line rhythms and transcript indent documentation with the source CSS.
- **why:** The native app used JetBrains Mono for the entire interface, while pinned Hermes uses a platform UI face for chrome/prose and JetBrains Mono for monospace surfaces.
- **components:** egui font definitions, source typography tokens, transcript/list indentation, architecture/setup documentation.
- **type:** visual alignment
- **validation:** Focused font contract passed with `agent-automation`: 1/1. `cargo fmt --all --check` passed after formatting. Headless transcript screenshot captured for visual inspection.
- **performance:** Windows system UI font is read once while building font definitions; no frame-loop filesystem reads were added.
- **risks:** Windows UI font loading is headless-tested only; other platforms use egui's bundled proportional fallback. Live Hermes Gateway behavior is unaffected.
- **follow_up:** Compare the captured native transcript with the pinned Hermes surface at matched DPI and continue PAR-0084.

### Revision 370 — Change a session's working directory — 2026-11-24

- **id:** hermes-par0084-working-directory-20261124
- **what:** Added a chat composer directory action with local folder browsing, remote path entry, idle-session checks, profile/session/connection-fenced `session.cwd.set`, and confirmation-only cwd/branch refresh. Added an offline session/dialog screenshot fixture.
- **why:** Agent sessions need to move to the intended project workspace without losing owner isolation or displaying an unconfirmed local path as active Hermes state.
- **components:** Native chat composer and dialog, session view metadata, Gateway actor/result parser, automation fixture, PAR-0084 evidence.
- **type:** agent workflow feature
- **validation:** Full locked workspace suite passes (721 library + 15 automation protocol + 3 launcher); all-target locked cargo check, focused busy/stale-owner/protocol tests, rustfmt, diff and package/progress validation pass. The screenshot is an offline synthetic fixture.
- **performance:** No additional background process or frame-loop filesystem access; local file selection is invoked only on user request.
- **risks:** Authenticated live Gateway, packaged Windows folder picker, denied/offline response behavior and formal source-paired acceptance remain unverified.
- **follow_up:** Test against the authenticated Hermes Gateway and installed Windows app; retain partial status until those checks pass.

### Revision 379 — Add cursor history pages and source chat typography — 2026-11-24

- **id:** hermes-par0090-cursor-and-chat-typography-20261124
- **what:** Added bounded, capability-gated `session.transcript` opaque-cursor paging for unresolved-owner read-only history, with request/endpoint/cursor fences, deduplication, reset/exhaustion behavior, and explicit retry. Applied the pinned Desktop's default 110% scale and transcript spacing to chat content, independent of window zoom; preserved the fixed message gutter and the requested JetBrains Mono/Fira Code stack.
- **why:** Let the user inspect older stored history without guessing a session owner, and match the downloaded source's default conversation typography and spacing.
- **components:** `src/app.rs`, `src/gateway.rs`, `src/fonts.rs`, UCM evidence and progress overlay, README, architecture, backlog, and plan.
- **type:** agent workflow + visual alignment
- **validation:** Full locked workspace suite passes (752 library + 15 automation protocol + 3 launcher tests); `cargo fmt --all -- --check`, `git diff --check`, both progress builders, package validator with PyYAML, independent YAML parse, and `doofus_task.py validate` pass. Headless capture: `artifacts/agent-ui/typography-chat-scale-110-20261124.png`.
- **performance:** No benchmark performed. Transcript reads use a separate worker lane with a tested two-reader concurrency cap; UI spacing adds no frame-loop I/O.
- **risks:** Opaque cursor reads are currently used for unresolved-owner read-only history; known-owner native paging remains offset-based. Authenticated live Gateway, packaged Windows behavior, and formal pinned-source acceptance remain unverified; PAR-0090 stays partial.
- **follow_up:** Exercise the cursor flow against the authenticated Gateway and packaged Windows app; assess extending it to known-owner pagination before claiming broader opaque-cursor support.

### Revision 380 — Match composer and Markdown typography to pinned source — 2026-11-24

- **id:** hermes-chat-composer-source-typography-20261124
- **what:** Applied Hermes Desktop's 110% chat text scale to the main composer and message-edit fields, set assistant Markdown to the source 1.5 line-height, and corrected transcript gap mapping so adjacent tool rows use 6px while scaffold-block pairs use 4px.
- **why:** The prior typography pass covered transcript text but missed the source's composer scope and assistant Markdown line-height; it also grouped tool rows with the tighter scaffold gap.
- **components:** Native chat composer and edit dialog, Markdown renderer, transcript rhythm classifier, font/spacing unit tests, README and architecture docs.
- **type:** visual alignment
- **validation:** Focused chat font and transcript rhythm tests passed; full locked workspace suite passed (753 library + 15 automation protocol + 3 launcher tests); rustfmt/diff checks passed; headless transcript + typed composer screenshot captured at 1440x900.
- **performance:** No frame-loop I/O, network calls, or additional allocations beyond per-frame style construction in existing scoped UI rendering.
- **risks:** Native has no distinct HUD/floating rich-composer surface; session panes reuse the same scaled composer. Live Gateway and packaged Windows visual acceptance remain open.
- **follow_up:** Compare transcript and composer against the pinned Desktop at matched DPI; preserve HUD-only compact spacing as out of scope until Hermes Native has a HUD context.

### Revision 381 — Connect replay-gap detection to authoritative resume — 2026-11-24

- **id:** hermes-par-0513-replay-gap-transport-20261124
- **what:** Convert explicit Hermes `snapshot_required: true` replay responses into owner-scoped `session.replay_gap` events, clear the stale replay cursor, and suppress unusable replay rows.
- **why:** The native UI already had safe owner-fenced snapshot recovery, but the reconnect transport dropped the signal that starts it.
- **components:** Gateway reconnect replay transport, owner-fenced session resume, PAR-0513 progress and UCM evidence.
- **type:** agent workflow reliability
- **validation:** Focused real-loopback WebSocket regression passes; full locked workspace suite, all-target check, formatting, progress builders, package validation, and task-state validation recorded in the execution log.
- **performance:** No additional network calls; one event is sent only for an explicit gap response and the stale cursor entry is removed.
- **risks:** Authenticated live Gateway and packaged Windows behavior are not verified; PAR-0513 remains partial.
- **follow_up:** Validate reconnect and automatic resume against the authenticated Gateway and packaged app.

### Revision 382 — Match composer typeface to pinned Hermes — 2026-11-24

- **id:** hermes-composer-proportional-font-20261124
- **what:** Changed main and message-edit composer inputs to the scaled proportional UI face. JetBrains Mono remains first for monospace surfaces, with Fira Code as the requested fallback.
- **why:** The pinned Hermes composer inherits the platform UI font; native composer text was still monospace.
- **components:** `src/fonts.rs`, font contract tests, README, architecture, plan, automation evidence notes and headless session-pane capture.
- **type:** visual alignment
- **validation:** Focused chat typography test passed; `cargo fmt --all -- --check` and `git diff --check` passed; 1440×900 isolated session-pane screenshot captured and visually inspected.
- **performance:** Uses the existing registered proportional font family; no frame-loop I/O or extra font loading.
- **risks:** Cross-platform font rasterization and packaged Windows visual behavior remain unverified.
- **follow_up:** Keep theme-configurable and platform-wide font-stack parity as distinct follow-up work.

### Typography documentation correction — 2026-11-25

- **id:** hermes-font-role-doc-correction-20261125
- **what:** Corrected the bundled font inventory to document Segoe UI as the Windows proportional UI face and JetBrains Mono followed by Fira Code for monospace content.
- **why:** The prior inventory described an obsolete monospace composer mapping that contradicted the pinned Hermes source and current implementation.
- **components:** `apps/hermes-native/assets/fonts/README.md`.
- **type:** documentation correction.
- **validation:** Focused `fonts::tests` pass (2 tests); documentation assertions and `git diff --check` pass.
- **performance:** No runtime or build behavior changed.
- **risks:** Non-Windows font rasterization and packaged Windows visual acceptance remain unverified.
- **follow_up:** None for this documentation correction.

### Revision 383 — Panel-scoped inline skill suggestions — 2026-11-25

- **id:** hermes-par-0067-inline-skill-suggestions-20261125
- **what:** Added caret-aware inline skill suggestions after whitespace, token-only replacement, panel-scoped completion ownership and stale-caret clamping; request/response results are fenced by the focused composer ID.
- **why:** Skill invocation is a core agent workflow, and visible session panes must not overwrite one another's completion state.
- **components:** `apps/hermes-native/src/app.rs`, slash completion headless tests, PAR-0067 progress/evidence and architecture/backlog notes.
- **type:** agent workflow feature
- **validation:** Focused slash suite 12/12; full locked workspace tests 761+15+3; all-target check, rustfmt and diff checks pass. EV-0182 records partial implementation evidence.
- **performance:** Existing 60 ms debounce remains; no additional worker or network request is introduced.
- **risks:** Suggestions may remain visible on the last owning pane after blur; full rendered two-pane focus-switch and late-response UI tests remain open. Live Gateway and packaged Windows/source-paired behavior are unverified.
- **follow_up:** Implement directive-chip interactions, suggestion caching/invalidation and local session/skin commands; validate with live Gateway and a rendered multi-pane focus-switch scenario.

## Revision 384 — PAR-0067 UCM evidence correction
- id: hermes-par-0067-ucm-evidence-fix-20261125
- timestamp: 2026-11-25T00:59:03Z
- what: Declared the inline skill completion scenario and made EV-0182's locator resolve from the capability map root.
- why: The installed Universal Capability Map checker requires declared scenario IDs and repository-relative file locators.
- components: maps/capabilities.yaml; maps/capabilities/evidence/EV-0182.json
- type: evidence correction
- validation: EV-0182 YAML/JSON, scenario, and locator checks pass. Full-map checking still reports 187 historical schema errors and 8 reconciliation items outside this entry.
- performance: No runtime change.
- risks: Capability map remains globally inconsistent due to existing historical evidence; no historical entries were rewritten.
- follow_up: Reconcile older map records in a separate scoped task.

## Revision 385 — Rendered inline skill focus handoff
- id: hermes-par-0067-rendered-focus-handoff-20261125
- timestamp: 2026-11-25T01:11:42Z
- what: Added a two-pane rendered interaction regression and EV-0183 evidence; refreshed PAR-0067 progress references and documented the remaining live/late-result limits.
- why: The prior evidence checked focus ownership with helpers but did not prove behavior across two simultaneously rendered session panes.
- components: apps/hermes-native/src/app.rs; apps/hermes-native/docs/BACKLOG.md; apps/hermes-native/docs/PLAN.md; apps/hermes-native/tools/build_implementation_progress.py; capability map EV-0183
- type: verification coverage
- validation: focused slash suite 12/12; full locked suite 761 library + 15 protocol + 3 launcher; all-target cargo check, rustfmt, and diff checks pass.
- performance: No runtime-path change; one headless UI test added.
- risks: Gateway response arriving after blur still lacks a direct event-level regression; no live or packaged acceptance was performed.
- follow_up: Verify authenticated live Gateway and add late-response fence coverage.

## Revision 388 — Hermes agent terminal output
- id: `hermes-par-0066-agent-terminal-stream-20261125`
- timestamp: 2026-11-25T21:50:32Z
- what: Made native local terminal output selectable and appendable to the unsent chat draft; added a collapsed-by-default read-only Agent output panel for bounded, profile/session-scoped process output events; documented DoofusDock as the reusable windowing framework and Hermes as owner of agent-pane behavior.
- why: Keep the coding agent's process output visible on demand while preserving the separation between the local shell and Hermes-managed processes. Clarify the app/library architecture from the operator's intended egui analogy.
- components: `apps/hermes-native/src/app.rs`; `apps/hermes-native/docs/ARCHITECTURE.md`; `apps/hermes-native/docs/BACKLOG.md`; `apps/hermes-native/docs/PLAN.md`; `apps/hermes-native/docs/status/CHAT_TOOL_PROGRESS.csv`; `apps/hermes-native/README.md`.
- type: Agent workflow feature and architecture clarification.
- validation: Focused terminal suite 13/13; full locked workspace suite 797 library + 15 automation protocol + 3 launcher tests; rustfmt check and all-target Cargo check recorded in execution log.
- performance: Output storage is bounded per chunk, process, and session owner; no benchmark claim.
- risks: `process.list` snapshot hydration and live Gateway, packaged Windows, and formal source-paired behavior remain unverified.
- follow_up: Add owner-fenced `process.list` hydration or an equivalent authoritative snapshot path, then validate stream reconnection and visible behavior against the pinned Gateway.

## Revision 389 — Hydrate Hermes Agent terminal processes — 2026-11-25

- **id:** `hermes-par-0066-process-snapshot-hydration-20261125`
- **what:** Added bounded `process.list` snapshot parsing and owner-fenced loading for the collapsed-by-default Hermes Agent output section. Opening the section loads existing process tails; while open it refreshes every five seconds. Snapshot output merges with live chunks without duplication and restores finished process exit codes.
- **why:** Streaming events alone omit processes already running when the panel opens or reconnects.
- **components:** `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, PAR-0066 docs and progress evidence.
- **type:** Agent workflow reliability.
- **validation:** Full locked workspace suite passed (802 library + 15 automation protocol + 3 launcher tests); all-target locked Cargo check, rustfmt check, and `git diff --check` passed.
- **performance:** Snapshot requests are bounded to 12 seconds and 2 MiB; UI polling runs only while the section is open and is limited to one request per five seconds.
- **risks:** Authenticated live Gateway, packaged Windows behavior, and formal source-paired acceptance remain unverified; PAR-0066 remains partial.
- **follow_up:** Validate snapshot-to-stream continuity against an authenticated Gateway and confirm the packaged Windows surface.

## Revision 390 — Record terminal workflow evidence — 2026-11-25

- **id:** `hermes-par-0066-terminal-evidence-20261125`
- **what:** Added PAR-0066 implementation evidence to the capability progress builder and the pinned-source capability map. The capability remains `partial` because authenticated live Gateway, packaged Windows, and formal source-paired acceptance remain open.
- **why:** The previous generated status omitted the new process snapshot and terminal-selection tests, leaving the row `unknown` despite successful validation.
- **components:** `apps/hermes-native/tools/build_implementation_progress.py`, `apps/hermes-native/docs/status/IMPLEMENTATION_PROGRESS.csv`, `apps/hermes-native/docs/status/CHAT_TOOL_PROGRESS.csv`, capability map EV-0184.
- **type:** Evidence and status reconciliation.
- **validation:** Both progress generators pass. The scoped UCM report classifies the capability as partial with scenario coverage and reports no EV-0184-specific errors. The map-wide checker still exits nonzero for 187 pre-existing schema errors and eight reconciliation items.
- **performance:** No runtime behavior changed.
- **risks:** Historical map inconsistencies remain and are not rewritten by this update.
- **follow_up:** Reconcile historical map errors separately; complete live Gateway and packaged Windows acceptance.

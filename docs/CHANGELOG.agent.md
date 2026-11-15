# Agent change journal

## HUI-001 — 2026-11-13

- **what:** Added the initial native Rust/egui Hermes shell with a four-zone layout, a revision-checked panel-to-tab interaction, and saved DoofusDock layout state. Imported the ZIP reference package into the new workspace and passed its package consistency checker.
- **why:** Establish a native client foundation for the requested Hermes UI replacement while preserving the existing Hermes runtime and Doofus authority.
- **components:** `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `src/app.rs`, `src/layout.rs`, `README.md`, `docs/ARCHITECTURE.md`, `docs/BACKLOG.md`, `docs/reference/`, `docs/EXECUTION_LOGS.md`.
- **type:** feat
- **validation:** Package checker passed: 565 target behaviors, 565 UI inventory items, 565 pending tests, 14 source inspection receipts. `cargo check --all-targets` passed after correcting initial egui API errors.
- **performance:** Not measured; no performance claim.
- **risks:** The Hermes source baseline is 1,474 commits behind current main. Gateway and bridge are not connected; current UI is a partial shell only. Existing source checkout changes were not touched.
- **follow_up:** Refresh upstream inventories, implement typed gateway and bridge adapters, then fill the parity matrix by complete workflows and evidence.

## HUI-002 — 2026-11-13

- **what:** Restored revision-checked docking command handling and corrected the outer egui panel layout. The native window now presents Chat, Sessions, Models, and Doofus navigation over the saved four-zone DoofusDock workspace.
- **why:** Resolve final compile regressions and make the first runnable UI slice reviewable in the native Windows window.
- **components:** `src/app.rs`, `README.md`, `docs/BACKLOG.md`, `docs/EXECUTION_LOGS.md`.
- **type:** fix
- **validation:** `cargo fmt --all` and `cargo check --all-targets` passed. `cargo run` launched; live screenshot showed the four-zone layout and the Models navigation content. Gateway offline status was explicit. Drag/drop was not exercised.
- **performance:** Not measured.
- **risks:** This remains a partial shell, with no Hermes gateway or authenticated Doofus bridge adapter. Full 565-item UI parity is not achieved. The upstream Hermes baseline is materially behind current main.
- **follow_up:** Add and validate typed gateway/bridge adapters, then implement each parity workflow against refreshed upstream behavior. HUI-002's execution record contains the measured finalization timestamp; the HUI-001 duration was an initial estimate.

## HUI-003 — 2026-11-13

- **what:** Corrected README and backlog wording to reflect wired center-tab and edge-split DockingService commands while clearly marking live drag/drop persistence as unverified.
- **why:** Keep the public project status aligned with implemented code and observed validation.
- **components:** `README.md`, `docs/BACKLOG.md`, `docs/EXECUTION_LOGS.md`.
- **type:** docs
- **validation:** Compared documented claims with `src/app.rs`; parsed all execution-log records as JSONL.
- **performance:** Not applicable.
- **risks:** UI parity remains incomplete.
- **follow_up:** Verify drag/drop and persistence in the live native window during the next interaction review.

## HUI-004

- **id:** HUI-004
- **timestamp:** 2026-11-14T20:23:59-06:00
- **what:** Routed cross-profile session resume and chat operations with the selected profile, kept same-ID sessions separate by profile, and allowed archived foreign-profile rows to open. Exposed active profile in automation state and aligned project scope docs to the pinned Hermes snapshot.
- **why:** Make the fixed downloaded source snapshot the only parity target and allow sessions returned by its all-profile archive route to be used safely in the native chat.
- **components:** apps/hermes-native/src/app.rs, apps/hermes-native/src/gateway.rs, README.md, docs/ARCHITECTURE.md, docs/BACKLOG.md, docs/CHANGELOG.agent.md, docs/EXECUTION_LOGS.md.
- **type:** feat
- **validation:** cargo fmt -- --check passed. cargo test --all-targets passed (27 tests). cargo test --features agent-automation passed (47 library tests and 5 automation tests). WebSocket fixtures verified profile on session.resume and prompt.submit; an app test verified a cross-profile archived row is openable and preserves its profile.
- **performance:** Not measured; no performance claim.
- **risks:** Runtime event correlation still relies on unique runtime session IDs; path-only navigation cannot disambiguate duplicate stored IDs, so profile-labeled session rows remain the reliable selection surface. Full 565-behavior parity and live Hermes runtime interaction remain incomplete.
- **follow_up:** Continue source-paired session lifecycle, event recovery, transcript editing and attachments against the fixed snapshot; then proceed through the remaining 565 acceptance cases.
## HUI-005

- **id:** HUI-005
- **timestamp:** 2026-11-14T20:29:22-06:00
- **what:** Added a registered open_session agent action carrying stored session ID and profile. Human row clicks use that handler. Path-only navigation now rejects IDs that occur in multiple profiles.
- **why:** Give the agent control surface an unambiguous way to perform the same cross-profile navigation as a person.
- **components:** apps/hermes-native/src/app.rs, apps/hermes-native/src/bin/hermes_automation/automation_protocol.rs, README.md, docs/ARCHITECTURE.md, docs/BACKLOG.md, docs/CHANGELOG.agent.md, docs/EXECUTION_LOGS.md.
- **type:** feat
- **validation:** cargo check --all-targets --features agent-automation passed; 48 library tests passed; the automation catalog test was updated for the new action and passed on rerun; cargo fmt -- --check passed. The new test verifies explicit profile selection and ambiguous path rejection.
- **performance:** Not measured; no performance claim.
- **risks:** Live Windows interaction and source-paired acceptance of the full session workflow remain unverified; overall 565-behavior parity remains incomplete.
- **follow_up:** Exercise the registered action through the live automation endpoint with a fixture or safe runtime session, then continue remaining snapshot behaviors.
## HUI-006

- **id:** HUI-006
- **timestamp:** 2026-11-14T20:33:38-06:00
- **what:** Made registered automation actions advance the egui frame before state and screenshot readback. Corrected the disconnected session view to show a connection message rather than indefinite loading. Exercised the built automation process through live stdio and captured the updated UI.
- **why:** The live process reported a new route and profile while its screenshot still showed the prior frame; the displayed pixels needed to reflect the completed action.
- **components:** apps/hermes-native/src/bin/hermes_automation/automation_protocol.rs, apps/hermes-native/src/app.rs, README.md, docs/ARCHITECTURE.md, docs/BACKLOG.md, docs/CHANGELOG.agent.md, docs/EXECUTION_LOGS.md, apps/hermes-native/artifacts/agent-ui/agent-stdio-empty-session-20261115-023316.png.
- **type:** fix
- **validation:** cargo fmt -- --check and cargo build --features agent-automation --bin hermes-automation passed. The focused automation handler test passed. A live stdio process returned ready, executed open_session for profile work, exposed the selected route in its accessibility tree, captured a PNG showing the updated Session screen and correct disconnected state, and exited cleanly.
- **performance:** Not measured; no performance claim.
- **risks:** The stdio process is headless and gateway isolated. The screenshot verifies agent control and rendered state, not authenticated Hermes runtime parity or native window behavior.
- **follow_up:** Extend automation to the actual native window and authenticated gateway when supported, then continue source-paired UI workflows against the fixed snapshot.
## HUI-007

- **id:** HUI-007
- **timestamp:** 2026-11-14T20:40:30-06:00
- **what:** Added a profile-scoped Rename session menu and accessible dialog. The gateway worker now performs a fenced HTTP title patch for persisted sessions and applies the confirmed title to active, archived, and search rows.
- **why:** The pinned Hermes source includes session rename as a core sidebar workflow, and the native UI had no equivalent.
- **components:** apps/hermes-native/src/app.rs, apps/hermes-native/src/gateway.rs, README.md, docs/ARCHITECTURE.md, docs/BACKLOG.md, docs/CHANGELOG.agent.md, docs/EXECUTION_LOGS.md.
- **type:** feat
- **validation:** cargo check --all-targets --features agent-automation passed. The full feature suite passed with 49 library tests and 5 automation tests after the gateway change. A focused AccessKit UI test passed after adding the dialog. A fixture HTTP test confirmed title, profile, request ID, revision, generation, and returned title.
- **performance:** Not measured; no performance claim.
- **risks:** The source uses session.title RPC for draft sessions without a persisted row; this implementation currently uses the persisted-row HTTP path. Authenticated live Hermes rename and full 565-behavior parity remain unverified.
- **follow_up:** Add session.title RPC for unpersisted active sessions, verify source-paired rename behavior in the authenticated runtime, and continue the remaining UI workflows.
## HUI-008

- **id:** HUI-008
- **timestamp:** 2026-11-14T20:43:04-06:00
- **what:** Added the pinned source's session.title RPC path for the active runtime session. Rename now sends runtime ID, profile, title, and request ID through the gateway actor, then reports the stored ID in the UI event.
- **why:** New and branched sessions may not yet have a persisted row, so the HTTP rename path can return 404 for a session that exists in gateway memory.
- **components:** apps/hermes-native/src/app.rs, apps/hermes-native/src/gateway.rs, README.md, docs/ARCHITECTURE.md, docs/BACKLOG.md, docs/CHANGELOG.agent.md, docs/EXECUTION_LOGS.md.
- **type:** feat
- **validation:** cargo fmt passed. A WebSocket fixture test passed and verified session.title method, runtime ID, profile, title, request ID, confirmed title, and stored-ID event correlation.
- **performance:** Not measured; no performance claim.
- **risks:** Deferred session.title acknowledgements and lost-response recovery remain incomplete. The RPC path has not been verified against an authenticated live Hermes gateway; full 565-behavior parity remains incomplete.
- **follow_up:** Correlate deferred title events and retained request identity, then verify the complete rename workflow against the pinned source and live gateway.
## HUI-009

- **id:** HUI-009
- **timestamp:** 2026-11-14T20:50:01-06:00
- **what:** Corrected active runtime rename to the pinned source's profile-scoped session.mutate rename contract. The gateway obtains a revision from session.resume, sends the canonical mutation, and retains the exact request payload for same-actor retry when a receipt is uncertain. The UI offers an exact retry and blocks a fresh mutation after a lost response.
- **why:** The previous session.title wire method was a Desktop adapter name, not the canonical gateway method. A fresh request ID after an uncertain outcome could apply a second edit.
- **components:** apps/hermes-native/src/gateway.rs, apps/hermes-native/src/app.rs, README.md, docs/ARCHITECTURE.md, docs/BACKLOG.md, docs/CHANGELOG.agent.md, docs/EXECUTION_LOGS.md.
- **type:** fix
- **validation:** cargo fmt passed. The canonical wire fixture and exact retry fixture passed. cargo test --features agent-automation passed with 52 library tests and 5 automation tests.
- **performance:** Not measured; no performance claim.
- **risks:** Retained mutation state is process local. After reconnect, an exact retry is unavailable and the client stops before a fresh mutation. HTTP rename uncertainty and authenticated live-gateway behavior remain unverified. Full 565-behavior parity is incomplete.
- **follow_up:** Persist the canonical mutation tuple across reconnect with source-paired recovery, close the HTTP rename uncertainty gap, and verify in an authenticated live Hermes runtime.

## HUI-010

- **id:** HUI-010
- **timestamp:** 2026-11-14T22:00:26-06:00
- **what:** Prompt submit now sends stable submission identity, validates the canonical admission receipt and session destination, and correlates completion by admission ID. Ambiguous transport failures preserve the optimistic row and expose an accessible retry with the same submission ID.
- **why:** The previous adapter omitted submission identity, accepted a noncanonical boolean acknowledgement, and could remove a prompt after an uncertain response. Completion events could also clear the wrong pending turn.
- **components:** apps/hermes-native/src/app.rs, apps/hermes-native/src/gateway.rs, README.md, docs/ARCHITECTURE.md, docs/BACKLOG.md, docs/CHANGELOG.agent.md, docs/EXECUTION_LOGS.md.
- **type:** fix
- **validation:** cargo check --all-targets --features agent-automation passed; cargo test --features agent-automation passed with 55 library tests and 5 automation tests; the added accessible retry UI test passed.
- **performance:** Not measured; no performance claim.
- **risks:** Only one pending submission is represented globally; unknown admission resolution and authenticated live-gateway behavior remain incomplete. Full fixed-snapshot parity is incomplete.
- **follow_up:** Add per-session admission collections, unknown-admission recovery and replay, then continue source-paired chat rendering and controls.

## HUI-011

- **id:** HUI-011
- **timestamp:** 2026-11-14T22:06:27-06:00
- **what:** Replaced the singleton pending prompt fields with profile and runtime-session keyed records. Each session now retains its submission ID, retry text, uncertainty state, and admission ID independently; receipts, completions, retries, stop eligibility, and automation state read from those records.
- **why:** Switching between chats could otherwise block new sends or overwrite the unresolved prompt identity.
- **components:** apps/hermes-native/src/app.rs, docs/ARCHITECTURE.md, docs/BACKLOG.md, docs/CHANGELOG.agent.md, docs/EXECUTION_LOGS.md.
- **type:** fix
- **validation:** cargo fmt passed. cargo test --features agent-automation passed with 55 library tests and 5 automation tests, including uncertain retry, completion-before-receipt, admission matching, and accessible stop/retry controls.
- **performance:** Not measured; no performance claim.
- **risks:** Unknown-admission recovery, background-session transcript replay, and persistence across app restart remain incomplete. Full fixed-snapshot parity is incomplete.
- **follow_up:** Add canonical unknown-admission resolution and replay for background sessions, then continue the fixed-snapshot chat UI workflows.

## HUI-012

- **id:** HUI-012
- **timestamp:** 2026-11-14T22:09:24-06:00
- **what:** Added canonical prompt.resolve_unknown support. Unknown submit receipts now retain execution generation and admission identity; the active chat sends a profile-scoped resolution request and applies the returned status to its matching pending record.
- **why:** The pinned source requires explicit recovery for an admission whose execution outcome is unknown.
- **components:** apps/hermes-native/src/app.rs, apps/hermes-native/src/gateway.rs, docs/ARCHITECTURE.md, docs/BACKLOG.md, docs/CHANGELOG.agent.md, docs/EXECUTION_LOGS.md.
- **type:** feat
- **validation:** cargo fmt passed; cargo test --features agent-automation passed with 55 library tests and 5 automation tests.
- **performance:** Not measured; no performance claim.
- **risks:** No authenticated live-gateway verification; durable recovery across app restart remains incomplete.
- **follow_up:** Add source-paired resolve_unknown fixtures and verify event/receipt races against an authenticated gateway.

## HUI-013

- **id:** HUI-013
- **timestamp:** 2026-11-14T22:10:12-06:00
- **what:** Added a WebSocket fixture for prompt.resolve_unknown that checks its canonical method, profile, session, admission ID, execution generation, and correlated terminal receipt.
- **why:** The source-required unknown-admission recovery path lacked wire-level verification.
- **components:** apps/hermes-native/src/gateway.rs, docs/CHANGELOG.agent.md, docs/EXECUTION_LOGS.md.
- **type:** test
- **validation:** cargo fmt passed; the focused fixture passed.
- **performance:** Not measured.
- **risks:** Fixture does not verify an authenticated running Hermes gateway.
- **follow_up:** Verify recovery against the live gateway and add the remaining source-paired chat controls.

## HUI-014

- **id:** HUI-014
- **timestamp:** 2026-11-15T04:14:03Z
- **what:** Added an initial native Markdown renderer for transcript headings, unordered lists and fenced code blocks with language labels and copy actions; captured the current isolated UI screenshot.
- **why:** Transcript messages were previously plain text, while the fixed Hermes source renders structured Markdown and code.
- **components:** apps/hermes-native/src/app.rs, README.md, docs/ARCHITECTURE.md, docs/BACKLOG.md, docs/CHANGELOG.agent.md, docs/EXECUTION_LOGS.md, apps/hermes-native/artifacts/agent-ui/hermes-native-current-20261115.png.
- **type:** feat
- **validation:** cargo fmt --all -- --check passed; cargo check --all-targets --features agent-automation passed; cargo test --all-targets --features agent-automation passed (56 library tests, 5 automation tests). The headless UI runner emitted an accessible-tree snapshot and a visually inspected 1440x900 PNG. Screenshot has no active session, so it confirms the current shell and fonts but does not display a Markdown transcript.
- **performance:** Not measured; no performance claim.
- **risks:** This is a deliberately limited renderer, not full Markdown or source parity. Inline emphasis, links, tables, syntax highlighting, and rich card rendering remain incomplete. The headless screenshot is gateway isolated.
- **follow_up:** Implement and validate richer transcript rendering against fixtures derived from the fixed snapshot only. Keep the parity target pinned to 8783054ef6cbafd86f10892241436862c1171a66; do not chase subsequent upstream updates.

## HUI-015

- **id:** HUI-015
- **timestamp:** 2026-11-15T04:35:56Z
- **what:** Replaced the hand-written line parser with egui_commonmark 0.26, pulldown-cmark and syntect; added an isolated transcript fixture and visible screenshot; replaced the unsupported copy emoji with a sized Copy/Copied control; made the native app the Cargo workspace default while retaining old prototypes; aligned active acceptance docs to the fixed 565-item package.
- **why:** The fixed Hermes snapshot uses a substantially richer Markdown pipeline, the selected fonts lack the upstream code-copy glyph, and the new repository needs to launch the active app without losing the older prototype.
- **components:** Cargo.toml, Cargo.lock, .gitignore, README.md, docs/ARCHITECTURE.md, docs/BACKLOG.md, docs/CHANGELOG.agent.md, docs/EXECUTION_LOGS.md, apps/hermes-native/Cargo.toml, apps/hermes-native/Cargo.lock, apps/hermes-native/src/app.rs, apps/hermes-native/src/bin/hermes-automation.rs, apps/hermes-native/vendor/egui_commonmark_backend, apps/hermes-native/docs/ARCHITECTURE.md, apps/hermes-native/docs/BACKLOG.md, apps/hermes-native/docs/PLAN.md, apps/hermes-native/docs/CHANGELOG.agent.md, apps/hermes-native/docs/EXECUTION_LOGS.md, apps/hermes-native/README.md, apps/hermes-native/artifacts/agent-ui/markdown-fixture-20261115.png.
- **type:** transcript rendering; agent visual fixture; Cargo workspace wiring; acceptance-scope correction.
- **validation:** cargo fmt --all -- --check passed; root cargo check --all-targets passed for the default Hermes Native member; cargo check -p hermes-doofusdock passed for the preserved root prototype; cargo check -p hermes-native --all-targets --features agent-automation passed; full feature suite passed (57 library, 5 protocol). The root runner produced and the agent visually inspected the 1440x900 Markdown fixture PNG; AccessKit exposed the table cell and Copy button. Default member metadata resolves to hermes-native.
- **performance:** Not benchmarked; the native parser cache is retained on the app instance. No performance improvement is claimed.
- **risks:** This is not full Streamdown parity. Tail streaming repairs, math/media plugins, transcript directives, artifact promotion and rich cards remain open. The screenshot is headless and gateway isolated. A small vendored backend patch must be reconciled deliberately if changing egui_commonmark versions.
- **follow_up:** Continue the 565 fixed-snapshot requirements and matched Windows visual/interaction evidence. Never expand the target because Hermes upstream changes.

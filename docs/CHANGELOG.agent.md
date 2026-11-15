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

## HUI-016

- **id:** HUI-016
- **timestamp:** 2026-11-15T04:37:44Z
- **what:** Initialized the user-provided repository as the canonical project root, published the reviewed workspace to origin/main, and preserved the active app plus the two older prototype trees. Build outputs, the local downloaded-source staging tree, env files and interrupted-write scratch files are excluded.
- **why:** The user created robertheadley/Doofus-Dock-Hermes for ongoing implementation; the prior workspace had no Git metadata.
- **components:** .gitignore, .gitattributes, Cargo.toml, Cargo.lock, README.md, apps/hermes-native, apps/hermes-native-rev1, src, docs, docs/EXECUTION_LOGS.md, docs/CHANGELOG.agent.md.
- **type:** repository bootstrap and publication.
- **validation:** Reviewed 230 staged paths; zero staged env/build/staging/temp paths; git diff --cached --check passed; initial commit 3c4f905de63f5d2fae2fe3c1e35645abe539aba9 pushed to origin/main; remote ref resolved to the same SHA and working tree was clean.
- **performance:** Not applicable.
- **risks:** The GitHub repository is public as created by the user. It contains the fixed acceptance package and historical prototype/reference files; the downloaded Hermes source checkout remains in ignored local staging and is not published.
- **follow_up:** Continue from this canonical main checkout and keep the parity target fixed at 8783054ef6cbafd86f10892241436862c1171a66.


## HUI-017

- **id:** HUI-017
- **timestamp:** 2026-11-15T04:44:53Z
- **what:** Reconciled acceptance target metadata with the fixed Hermes snapshot and classified the preserved inventory: 499 Hermes Desktop rows are in scope; 66 Hermes-Doofus rows remain excluded and pending.
- **why:** The 565-row package had mixed source SHAs, while the approved replacement scope is stock Hermes Desktop and the target is the downloaded `8783054ef6cbafd86f10892241436862c1171a66` snapshot.
- **components:** fixed 565-row reference package, acceptance matrix, source audit, active plan/backlog, root README, architecture and backlog.
- **type:** scope accounting and snapshot metadata.
- **validation:** Package validator passed with 565 consistent behaviors and all 565 statuses pending. A dedicated reconciliation note documents original research pins and clarifies that changing the matrix test target is not per-row source verification.
- **performance:** Not measured; no runtime behavior changed.
- **risks:** Individual requirement/source pairings and matched UI evidence remain incomplete. The 66 excluded records stay in the research package and may be mistaken for active requirements if callers ignore the scope note.
- **follow_up:** Continue source review and implementation for the 499 fixed-snapshot Hermes Desktop requirements; do not query newer upstream commits.



## HUI-018

- **id:** HUI-018
- **timestamp:** 2026-11-15T04:57:17Z
- **what:** Added bounded older-transcript pagination, profile/session/order validation, stale-response fencing, overlap-safe prepend, an accessible history action, and a headless visual fixture with stable agent locator.
- **why:** Hermes snapshot `8783054ef6cbafd86f10892241436862c1171a66` hydrates a 120-message tail and fetches earlier messages on demand.
- **components:** apps/hermes-native/src/gateway.rs, app.rs, automation runner/protocol, long-session screenshot, active plan/backlog/source audit, README, architecture, execution log.
- **type:** transcript history feature; local protocol and visual evidence.
- **validation:** `cargo fmt --all`; `cargo check -p hermes-native --all-targets --features agent-automation`; `cargo test -p hermes-native --all-targets --features agent-automation` passed (62 library + 5 automation tests). A headless 1440×900 screenshot was rendered and visually inspected.
- **performance:** No benchmark; each page is capped at 120 messages and the existing HTTP response limit is 512 KiB.
- **risks:** Live gateway acceptance is open. Transcript virtualization, source scroll-window and scroll-anchor behavior, and retention remain incomplete.
- **follow_up:** Implement the source-equivalent transcript window and scroll-driven “Show earlier” behavior, then validate against isolated fixed-snapshot gateway data.



## HUI-019

- **id:** HUI-019
- **timestamp:** 2026-11-15T05:01:08Z
- **what:** Normalized the isolated long-session screenshot from RGBA to RGB after Codex preview showed black; retained the correctly rendered artifact at `apps/hermes-native/artifacts/agent-ui/long-session-history-20261115-visible.png`.
- **why:** Keep visual evidence inspectable and prevent an invalid black preview from being treated as the UI result.
- **components:** long-session agent UI screenshot and execution/change journals.
- **type:** artifact verification.
- **validation:** `ffprobe` reports 1440×900 RGB24; the image was visually inspected and shows the app shell, fixture transcript and disabled offline history action.
- **performance:** Not applicable.
- **risks:** Headless fixture only; it does not prove live gateway behavior.
- **follow_up:** Continue source-driven implementation and matched fixed-snapshot visual checks.



## HUI-020

- **id:** HUI-020
- **timestamp:** 2026-11-15T05:04:36Z
- **what:** Adjusted transcript history compatibility so the 120-row cap applies only to responses that confirm pagination; legacy complete transcripts may exceed one page under explicit payload and row limits.
- **why:** The pinned Hermes Desktop supports legacy backends that return the entire transcript without pagination metadata.
- **components:** apps/hermes-native/src/gateway.rs, active plan/backlog/source audit, execution log.
- **type:** compatibility fix and negative-path fixture.
- **validation:** `cargo fmt --all`; feature test suite passed (63 library + 5 automation tests); a local HTTP fixture returns 121 complete legacy rows and confirms whole-transcript replacement.
- **performance:** Complete response bounded by 512 KiB and 10,000 rows; normal paginated reads remain 120 rows.
- **risks:** Live source gateway compatibility remains unverified.
- **follow_up:** Exercise current and legacy page shapes against an isolated gateway from the pinned snapshot.
## HUI-021

- **id:** HUI-021
- **timestamp:** 2026-11-15T05:34:17Z
- **what:** Added profile-scoped in-memory chat view caching, draft/transcript restoration across routes, and background message/tool stream routing with ambiguity rejection.
- **why:** Establish independent session state before exposing multiple dynamic Hermes session panes.
- **components:** Hermes Native app route state, gateway event projection, diagnostics, app tests.
- **type:** session-state foundation.
- **validation:** Formatting and all-target feature check passed; test suite passed (68 library + 5 automation tests), including profile collision, background routing and cache-bound coverage.
- **performance:** Cache targets 32 idle session views; active/in-flight work is protected.
- **risks:** State is memory-only and not rendered as separate dynamic panes; background approval/clarify request ownership remains open.
- **follow_up:** Bind each DoofusDock session descriptor to its own visible chat view and request lifecycle.

## HUI-022

- **id:** HUI-022
- **timestamp:** 2026-11-15T06:06:37Z
- **what:** Render independently keyed Hermes session panes in DoofusDock, front stored sessions in canonical tabs, support blank-session creation and first-message sidebar listing, and confirm close of active tabs without stopping Hermes.
- **why:** The previous implementation had persisted pane descriptors and cached route state but no visible dynamic session workspace.
- **components:** Hermes Native app/layout, session gateway lifecycle, headless agent fixture and screenshot capture.
- **type:** session UI and interaction parity increment.
- **validation:** Formatting, all-target feature check, and test suite passed (74 library + 5 automation tests); the 1440x900 isolated session pane rendered through the shared headless UI harness. Screenshot evidence is `artifacts/agent-ui/session-pane-current-20261115.png`.
- **performance:** Session content reuses the existing view cache and shared transcript/composer render path without per-frame network calls.
- **risks:** Source-parity remains incomplete for hidden server requests, primary-pane promotion/reset, reuse of edited blank drafts, error correlation, persistent scroll, close commands/keyboard/focus/overflow and live acceptance.
- **follow_up:** Complete remaining tab lifecycle and hidden-request routing, then continue source-paired parity against the pinned downloaded snapshot.

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

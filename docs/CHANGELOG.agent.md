# Agent change journal

## HUR-001-176/177 — 2026-11-16

- **id:** HUR-001-176-177
- **timestamp:** 2026-11-16T22:40:11Z
- **what:** Implemented Scheduled Job Run now/Stop with bounded isolated trigger work and added a workstream-level partial-completion overview.
- **why:** Continue the fixed Hermes snapshot replacement while making partial engineering progress visible separately from source acceptance.
- **components:** `apps/hermes-native/src/app.rs`, `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/docs/COMPLETION_OVERVIEW.md` and parity docs.
- **type:** feature and reporting documentation
- **validation:** Full feature suite passed (208 library + 15 protocol tests); all-target feature check passed. Final formatter, diff and reference-package validation are recorded in execution logs. Strict parity remains 0/499 accepted.
- **performance:** Trigger has a separate bounded worker (capacity 8); list/history refresh is bounded and state-driven.
- **risks:** Live authenticated behavior and source-paired acceptance remain open. The 491 supplemental rows are not reconciled with the fixed 499 in-scope rows.
- **follow_up:** Finish evidence crosswalk and row-level partial status; continue remaining Hermes UI behaviors.

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

## HUI-033 — Native Hermes public plugin catalog slice

The native Capabilities → Plugins route now fetches the same public `plugins.json` feed as the pinned Desktop through a bounded background worker. Source-shaped entries support local search, selection/details and repository navigation; catalog refresh is registered through the shared UI/agent action path. Added parser coverage and a synthetic isolated screenshot fixture. Installed package management, catalog filters/cards/install workflows, Connectors and paired Windows/source verification remain incomplete.

HUI-033 visual follow-up: corrected the four-pixel left clipping visible in the first fixture, retained loaded catalog rows during refresh/error, and reset selected details when search changes. Rebuilt and visually inspected the updated fixture; the final complete validation run passed.

HUI-033 live-feed follow-up: the isolated Rust UI agent loaded the actual 2,378-row public feed and exposed the first 60 rows. Added Windows Unicode fallback faces plus tofu for unsupported glyphs, 60-row rendering pages, and bounded diagnostics after live CJK/Thai content revealed panic and oversized-state cases. The live UI wait completed in 604 ms and `state.get` confirmed all rows loaded, 60-row display limit, no catalog error, and gateway isolation. Saved a distinct live catalog screenshot and kept the synthetic fixture separate.

HUI-033 screenshot evidence follow-up: identified that fresh headless processes reset to New session, so captures without route navigation/fixtures repeated the default view. Added an explicit route-or-fixture plus wait requirement, and recorded a visually distinct live public catalog capture separately from the synthetic fixture, with route/state/hash-bound sidecars.

HUI-034: added the pinned-source Connectors catalog/list/accounts read path, searchable directory and expanded account status using the profile-bound gateway actor. Local WebSocket and AccessKit fixtures pass; a distinct synthetic directory screenshot is saved. Connector authentication, local MCP, policy/tools management, hosted account operations and live/source-paired acceptance remain open.

HUI-034 publication record: committed the verified Connectors read slice as `aab2176` and pushed it to `origin/main` after confirming the remote parent matched; local/remote main agree and the worktree is clean.

HUI-034 follow-up: matched the pinned Desktop's catalog/list union so connected rows missing from catalog remain visible, with source slug-title and account-status fallbacks. The isolated fixture now proves a list-only connected row and has a distinct 4-row screenshot. This does not implement connector sign-in, MCP, policy/tools, or hosted account operations.

HUI-034 source-join validation: full test/build/source-inventory validation passed; the revised 4-row synthetic screenshot was visually checked. No live profile was used.


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

### HUI-039 — inspected, commit-pinned Agent Plugin install

Added catalog and agent-action entry points to a profile-scoped Hermes inspection and explicit commit review flow. Installs submit the catalog name and inspected SHA; mismatched pins, insecure sources and non-Agent entries fail closed. Receipt correlation refreshes installed inventory and reports missing environment/warnings; disconnects leave an unknown outcome, disable resubmission and block a new dialog until the profile inventory reconciles it. The isolated RPC fixture and review UI were validated without touching the installed Hermes gateway/profile. Desktop components, scan finding consent/force, updates, custom repositories and live parity remain incomplete.

Validation: 109 library and 7 automation tests passed; formatting, builds and fixed-source package validator passed. All 565 inventory rows remain pending, with optional PyYAML parsing skipped. The 1440×900 synthetic review dialog has a visually inspected screenshot. No runtime performance benchmark was taken. See the app-level HUI-039 entry and execution record for details.

HUI-039 publication: implementation commit `c9cd13d` is published on `main` after confirming parent `6ff7dc0` against `origin/main`.

### HUI-040 — Agent Plugin update consent and scan review

Add profile-scoped Agent Plugin updates with capability-consent and malware-scan review. Preserve accepted options through backend gates; only caution findings permit explicit force, while dangerous findings remain blocked. Isolated gateway and AccessKit tests validate the flow. Desktop-half reconciliation, uncertain live result handling, update-all and live/source-paired acceptance remain open.

Validation: 111 library and 7 automation tests passed; formatting, default/all-target checks, package validation and diff check passed. The 565-row inventory remains pending and optional PyYAML was skipped. The synthetic screenshot was inspected without connecting to Hermes. No runtime benchmark was taken. See app-level HUI-040 and execution records.

HUI-040 publication: implementation commit `b6e3686` is published on `main` after confirming parent `2be5378` against `origin/main`.

### HUI-041 — profile-scoped automatic Agent Plugin updates

Add the pinned Desktop plugins.manage auto_update preference to the Plugins inventory, scoped to the selected profile. The control is hidden when unsupported, preserves the backend check interval, and shares one receipt-fenced handler between the visible checkbox and agent action. Offline, stale-profile and in-flight writes fail closed. A synthetic UI fixture and isolated WebSocket contract test cover the new surface.

Validation: 112 library and 7 automation tests passed; formatting, default/all-target checks, fixed-source package validation and diff check passed. The 565-row research inventory remains pending; optional PyYAML was skipped. The screenshot was visually inspected without connecting to Hermes. No runtime benchmark was taken. See app-level HUI-041 and execution records.

Validation correction for HUI-041: the final complete suite passed 113 library tests and 7 automation tests after adding backend interval parsing coverage. All compile/package checks and the screenshot sidecar check passed.

HUI-041 publication: implementation commit 88206d5 is published on main after confirming parent a0cf840 against origin/main.

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

## HUI-048 — 2026-11-15

- **id:** HUI-048
- **timestamp:** 2026-11-15
- **what:** Added optional local transcript timestamps, window-scoped slash controls, lifecycle capture, and an isolated visual fixture.
- **why:** Advance native Hermes P0 chat parity using the pinned Desktop behavior.
- **components:** Hermes Native transcript renderer, prompt command handling, automation fixture, source parity map.
- **type:** feature / source-parity slice.
- **validation:** 121 native library tests and 8 automation tests passed; all-target feature check passed; 1440×900 screenshot rendered and visually inspected.
- **performance:** Formatting is local and conditional; no backend calls added.
- **risks:** Config initialization and source-paired acceptance remain open; PAR-0065 remains pending.
- **follow_up:** Complete config-backed default, source tooltip/tool timing, and paired Windows acceptance.

## HUI-049 — 2026-11-15

- **id:** HUI-049
- **timestamp:** 2026-11-15
- **what:** Recorded the transcript timestamp capability as partial with implementation evidence and extended the fixed-inventory validator for partial capability states.
- **why:** Keep implementation progress visible while retaining pending acceptance until source-paired validation is complete.
- **components:** Hermes parity inventory/crosswalk/UCM, EV-0016 evidence, package validator.
- **type:** parity evidence and validation repair.
- **validation:** Package validator passed all 565 targets, inventory rows, and pending acceptance rows; it counted 14 source inspection receipts and 1 implementation receipt.
- **performance:** No runtime behavior changed.
- **risks:** Optional YAML parser was unavailable in the environment; PAR-0065 remains pending.
- **follow_up:** Continue the fixed-source parity work and close only requirements with source-paired evidence.

HUI-048/HUI-049 publication: commits `4d247d7ccbee6f31604923e52fc37a799100aa3d` (implementation) and `27504e9c55f31a6cdcc8e47ee075ba712ca87dac` (partial evidence + validator) were pushed to `origin/main`. PAR-0065 remains pending.

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

- **id:** HUI-056
- **timestamp:** 2026-11-15T14:05:55Z
- **what:** Added language-highlighted Rust source previews and changed the isolated Files fixture to open `main.rs`.
- **why:** The pinned Hermes source renders code with language-aware highlighting; the native preview should display that source content readably.
- **components:** Hermes Native Files preview, screenshot fixture, feature documentation and validation log.
- **type:** feature slice and validation.
- **validation:** `cargo fmt --all -- --check`; `cargo test -p hermes-native --all-targets --features agent-automation` passed 129 library + 11 automation tests; `cargo check -p hermes-native --all-targets --features agent-automation`; `git diff --check`; 1440×900 synthetic target capture visually inspected.
- **performance:** Reuses the existing preview worker and CommonMark renderer; no additional I/O path.
- **risks:** Preview formats remain partial; target-only capture is not source-paired Hermes parity evidence; PAR-0104 remains pending.
- **follow_up:** Publish this slice, bind a distinct code-preview capture to the published commit, then continue the next pinned Files requirement.
- **id:** HUI-057
- **timestamp:** 2026-11-15T14:06:41Z
- **what:** Refreshed the isolated Rust code-preview screenshot and bound its metadata to published commit `259f7a6e`.
- **why:** Provide a visibly distinct, reproducible capture of the current Files-preview state.
- **components:** Agent automation fixture, screenshot artifact and sidecar metadata.
- **type:** capture and verification.
- **validation:** AccessKit inspection confirms Files-preview fixture state; PNG dimensions are 1440×900; screenshot and binary SHA-256 values recorded; image visually inspected.
- **performance:** Fixture setup and capture complete in about three seconds.
- **risks:** This remains synthetic target-only evidence and does not establish source-paired or native-host parity.
- **follow_up:** Continue pinned-source implementation; keep formal acceptance rows pending until corresponding paired evidence exists.
- **id:** HUI-058
- **timestamp:** 2026-11-15T14:14:22Z
- **what:** Added read-only BMP, GIF, JPEG, PNG and WebP preview decoding on the file worker, cached egui textures, and an isolated image-preview capture fixture.
- **why:** The pinned Hermes Desktop routes recognized image extensions into a visual preview pane; the native tree previously treated them as text.
- **components:** Hermes Native Files preview worker, image dependencies, UI fixture, automation runner and documentation.
- **type:** feature slice and validation.
- **validation:** `cargo fmt --all -- --check`; `cargo test -p hermes-native --all-targets --features agent-automation` passed 130 library + 11 automation tests; all-target agent-automation check and `git diff --check` passed; 1440×900 image fixture captured and visually inspected.
- **performance:** Image I/O and decode stay off the UI thread; encoded size is capped at 16 MiB and dimensions at 64 megapixels; decoded textures are reused by path.
- **risks:** SVG remains unsupported along with PDF/HTML, remote file reads and exact source-paired appearance; formal PAR-0105 remains pending.
- **follow_up:** Add SVG/PDF/HTML paths and source-paired interaction/visual evidence against the pinned snapshot.
- **id:** HUI-059
- **timestamp:** 2026-11-15T14:14:55Z
- **what:** Added a distinct PNG preview screenshot and hash-bound metadata for published image-preview commit `3b203ea014c7eabd66cae436c201a3b91ffd196e`.
- **why:** Keep visual evidence tied to the exact pinned source and target binary while making screenshot routes reproducible.
- **components:** Agent UI image fixture and screenshot artifact.
- **type:** capture and verification.
- **validation:** AccessKit inspection confirmed the image fixture route; PNG dimensions and SHA-256 hashes were parsed and validated; capture is 1440×900 and visually inspected.
- **performance:** Fixture capture completed in under ten seconds including incremental build.
- **risks:** Synthetic target-only capture does not establish source parity or live Windows behavior.
- **follow_up:** Continue remaining pinned Files behaviors and source-paired acceptance.
- **id:** HUI-060
- **timestamp:** 2026-11-15T14:20:52Z
- **what:** Added bounded SVG parsing and vector rasterization to the native image preview, plus a distinct offline SVG fixture.
- **why:** The pinned Hermes Desktop treats SVG as an image preview; the raster-only slice left a recognized source format incomplete.
- **components:** Hermes Native preview worker, `resvg`/`usvg`, automation runner, documentation and screenshot artifact.
- **type:** feature slice and validation.
- **validation:** `cargo fmt --all -- --check`; `cargo test -p hermes-native --all-targets --features agent-automation` passed 131 library + 11 automation tests; all-target feature check and `git diff --check` passed; SVG capture visually inspected at 1440×900.
- **performance:** SVG parsing/rendering runs on the existing background worker; file bytes, edge length and output pixels are bounded.
- **risks:** PDF/HTML and source-paired/native-host acceptance remain open; unusual SVG features follow usvg support limits; PAR-0105 remains pending.
- **follow_up:** Publish the capture with hash metadata; continue PDF/HTML preview and remaining Files semantics.
- **id:** HUI-061
- **timestamp:** 2026-11-15T14:21:20Z
- **what:** Added a distinct vector preview screenshot and hash-bound metadata tied to published SVG-renderer commit `effc98090a10c4fef7bd6e038981549167ab1cc8`.
- **why:** Make SVG preview behavior reproducible and distinguishable from the default route and raster image fixture.
- **components:** Agent UI SVG fixture, screenshot and sidecar.
- **type:** capture and verification.
- **validation:** AccessKit inspection identified the SVG fixture; image is 1440×900 and its SHA-256 plus exact runner binary SHA-256 are recorded and validated; image visually inspected.
- **performance:** Isolated capture completed in under ten seconds after build.
- **risks:** Synthetic target-only screenshot does not establish source-matched behavior or live Windows acceptance.
- **follow_up:** Continue PDF/HTML and other pinned Files behaviors, then gather source-paired evidence.
- **id:** HUI-062
- **timestamp:** 2026-11-15T14:36:35Z
- **what:** Added PDF preview with worker-bound PDFium 8125, selected-page rendering, Previous/Next controls, Windows x64 DLL staging, two-page offline fixture and release-license provenance.
- **why:** The pinned Hermes preview uses a built-in Chromium PDF viewer; plain text decoding could not preview PDFs in the native UI.
- **components:** Hermes Native Files preview, PDFium runtime package, build script, automation fixture and documentation.
- **type:** feature slice and validation.
- **validation:** `cargo fmt --all -- --check`; `cargo test -p hermes-native --all-targets --features agent-automation` passed 132 library + 11 automation tests; all-target feature check and `git diff --check` passed; the PDFium DLL copied beside target output has the bundled SHA-256; headless UI captured page 1 and clicked Next page to verify distinct page 2 content.
- **performance:** PDF input capped at 16 MiB; output is constrained to 1280×1800; only the selected page is rendered and cached; PDFium work stays on the dedicated preview worker.
- **risks:** PDF experience is page-at-a-time instead of Chromium's continuous-scroll/zoom UI; bundled binary is Windows x64 only; HTML and source-paired acceptance remain open; PAR-0107 remains pending.
- **follow_up:** Add continuous page scrolling/zoom or a faithful native document canvas, additional platform runtimes, HTML preview, and source-paired comparisons.
- **id:** HUI-063
- **timestamp:** 2026-11-15T14:37:16Z
- **what:** Added separate, hash-bound PDF page 1 and page 2 screenshot artifacts for published commit `b84c3e69aac2cdfea0cdff8a4f502d4f9e191f5e`.
- **why:** Capture both initial and navigated PDF states instead of repeating the default page.
- **components:** Offline PDF fixture, accessible page action and screenshot metadata.
- **type:** capture and verification.
- **validation:** Sidecars assert exact source/binary/image digests and 1440×900 dimensions; AccessKit click navigates through `Next page`; both images visually inspected.
- **performance:** Two fixture captures run in seconds from the incremental build.
- **risks:** Captures are synthetic target-only proof, not source-matched or native-installed acceptance.
- **follow_up:** Continue pinned-source PDF toolbar/continuous-scroll behavior and paired source evidence.

- **id:** HUI-064
- **timestamp:** 2026-11-15T18:15:20Z
- **what:** Extended stdio screenshot capture to publish a hash-bound JSON provenance sidecar beside each PNG, using the existing artifact naming convention and no-clobber publication.
- **why:** Make agent captures independently reviewable with pinned source/build, route/state, viewport, fixture/profile, timestamp and interaction-receipt evidence.
- **components:** `hermes-automation` protocol capture, capture regression coverage, automation and plan documentation.
- **type:** feature and validation.
- **validation:** `cargo fmt --all -- --check`; `cargo test -p hermes-native --all-targets --features agent-automation` passed 139 library and 12 automation tests; `cargo check -p hermes-native --all-targets --features agent-automation`; `git diff --check`; real `--stdio` child capture returned success and the independently recomputed PNG SHA-256 matched its sidecar. The regression also confirms duplicate capture names fail without altering the original image.
- **performance:** Reads the running executable once per capture to record its SHA-256; output remains bounded by the protocol response limit.
- **risks:** Headless capture has no physical OS DPI or native-window pixels; native window and source-paired visual acceptance stay open.
- **follow_up:** Keep headless and native-window evidence separate; proceed with isolated background window and bridge/supervisor work.

- **id:** HUI-065
- **timestamp:** 2026-11-15T18:26:52Z
- **what:** Added a dependency-free Node-compatible `HermesPage` client with Playwright-shaped semantic locators and actions over the Hermes Rust stdio protocol.
- **why:** Let an agent write readable user-like UI scripts, inspect state, capture evidence and drive the isolated runner without hand-building NDJSON requests.
- **components:** JavaScript client, fake-child tests, Node documentation and plan.
- **type:** feature and validation.
- **validation:** `node --test apps/hermes-native/agent/hermes-page.test.mjs` and `node --check` passed; 3/3 client tests cover semantic queries, generation tracking, stale-state refusal and no replay after an unknown screenshot-write timeout. A real Node-launched Rust child navigated to the Toolsets tab, queried state, captured a hash-bound screenshot and exited cleanly.
- **performance:** No runtime dependencies; one child process per page; mutations are serialized.
- **risks:** Bun execution is unverified because Bun is not installed here; this client is headless and does not control a native window.
- **follow_up:** Run the same suite under Bun when available, then integrate the client with an isolated native background host.

- **id:** HUI-066
- **timestamp:** 2026-11-15T18:59:20Z
- **what:** Changed the Hermes HTML preview lifecycle smoke to create a visible, off-screen, non-activating window on the normal display desktop. It records the Windows foreground owner PID before and after, fails if the smoke process becomes foreground, rejects existing evidence paths, and writes a source/build/image-bound PNG sidecar.
- **why:** Validate actual native WebView2 and Dock lifecycle behavior without taking focus from the operator, and make the result independently reviewable.
- **components:** Hermes Native HTML preview smoke; plan, architecture, backlog and operator docs; native screenshot, JSON receipt and provenance sidecar.
- **type:** native automation evidence and safety improvement.
- **validation:** `cargo fmt --all -- --check`; `cargo test -p hermes-native --all-targets --features agent-automation` passed 139 library and 12 automation tests; all-target feature check passed; actual Windows smoke returned `ok=true`, HTML title `asset-loaded:16`, visible non-zero child bounds, successful hide/reopen, unchanged foreground PID and matching PNG/sidecar SHA-256. The egui screenshot digest is byte-identical to the earlier fixed-fixture capture; the existing labeled WebView2 child image contains the visible document pixels.
- **performance:** One isolated UI launch and renderer capture; no app runtime path or normal launch cost changed.
- **risks:** The isolated Win32 desktop route could not create an OpenGL window config. Off-screen native window actions are not exposed through the agent protocol; live Hermes and source-paired acceptance remain open. The shell screenshot omits the WebView2 child.
- **follow_up:** Integrate the agent protocol with a persistent isolated native host and capture distinct user-selected native states; keep all 499 in-scope requirements pending until source-paired acceptance.

- **id:** HUI-067
- **timestamp:** 2026-11-15T19:17:36Z
- **what:** Added `hermes-automation --native-window`, a Windows-only isolated native app launch that uses a temporary profile, disables Hermes gateway startup, positions the app off-screen and applies `WS_EX_NOACTIVATE`.
- **why:** Let Windows UI Automation inspect the real HermesNativeApp accessibility tree while leaving the user's foreground window unchanged.
- **components:** Hermes automation launcher, native Windows accessibility behavior, plan/backlog/architecture and automation documentation.
- **type:** Windows native accessibility inspection.
- **validation:** `cargo fmt --all -- --check`; `cargo test -p hermes-native --all-targets --features agent-automation` passed 139 library and 12 automation tests; feature check/build passed; Node syntax/tests passed 3/3; launched the actual app, found its HWND and queried 32 UIA descendants. Read-only inspection preserved foreground PID 25840 and confirmed `WS_EX_NOACTIVATE` is set.
- **performance:** No change to the normal app path; one native window is started only when the explicit option is used.
- **risks:** UIA TogglePattern action activated the window despite the no-activate style; direct pointer messages kept focus but did not change the route. This mode is inspection-only; no native action or screenshot bridge exists yet.
- **follow_up:** Implement generation-fenced process-local input and renderer screenshot requests, then repeat UIA/native tests without focus changes before exposing actions to the JS client.
- **id:** HUI-068
- **timestamp:** 2026-11-15T20:02:45Z
- **what:** Added a generation-fenced native background channel for route state, shared typed app actions, renderer screenshots and clean close. Added a native route-specific capture with source/build/image/foreground metadata.
- **why:** Let unattended agents navigate a real HermesNativeApp instance and inspect distinct native UI states without stealing desktop focus, using the installed Hermes profile, or repeating the default New session screenshot as progress evidence.
- **components:** Windows native host, HermesNativeApp route summary, screenshots and sidecar, plan, README, agent automation, architecture, backlog and execution records.
- **type:** native agent automation and evidence.
- **validation:** `cargo fmt --all -- --check`; `cargo check -p hermes-native --all-targets --features agent-automation`; actual `--native-window --stdio` process verified `/` → `/capabilities` via shared `RegisteredUiAction`, captured 2160×1350 PNG at 1.5 pixels/point, confirmed SHA-256 and sidecar agreement, acknowledged close with exit code 0, and preserved foreground PID 5220.
- **performance:** Feature-gated only; screenshots encode at most 12 MiB PNG to base64. No normal application path changes.
- **risks:** Evidence is offline and target-only. Native semantic locator input, reliable user-like click/type/key, live Hermes gateway validation, physical DPI verification and source-paired parity remain open. Synthetic pointer dispatch does not confirm its target effect.
- **follow_up:** Connect the Node semantic client to a trustworthy native tree/action adapter; verify each control's visible effect, capture non-repeated route states, then run source-paired acceptance. Keep the 499-row accepted count unchanged.
- **id:** HUI-069
- **timestamp:** 2026-11-15T20:07:57Z
- **what:** Clarified that identical default screenshots come from fresh offline profiles at the same empty `/` route, and documented the native-window protocol and its evidence limits.
- **why:** Stop treating repeated renderer smoke captures as feature progress or parity evidence.
- **components:** Native automation protocol docs and agent capture guidance.
- **type:** documentation and evidence quality.
- **validation:** Cross-checked protocol method/version/request limits against implementation and confirmed the existing `/` to `/capabilities` capture is distinct route-only evidence.
- **performance:** Documentation only.
- **risks:** Native semantic locators, verified pointer effects, live gateway workflows, and source-paired parity remain incomplete.
- **follow_up:** Require each future visual claim to name a route/fixture, action receipt, and evidence scope.
- **id:** HUI-070
- **timestamp:** 2026-11-15T20:17:26Z
- **what:** Added diagnostic snapshots to the off-screen native UI channel and returned before/after app state with typed action receipts.
- **why:** Give unattended agents visibility into layout/profile and lifecycle state alongside unique route screenshots.
- **components:** Native control protocol, HermesNativeApp diagnostic snapshot, agent protocol/plan/backlog/architecture docs.
- **type:** native UI agent debugging.
- **validation:** Pending direct execution of the built native host and protocol assertions; formatting and compile validation are still required.
- **performance:** Opt-in request-only serialization in the feature-gated host.
- **risks:** Does not complete native semantic user-like input or source-paired visual acceptance.
- **follow_up:** Directly verify route/state changes and use diagnostics to debug the next source-paired workflow.
- **id:** HUI-071
- **timestamp:** 2026-11-15T20:21:28Z
- **what:** Validated native-window diagnostics and before/after state receipts through a real off-screen app process.
- **why:** Make headless debugging evidence current and queryable without repeating the default screenshot.
- **components:** Hermes Native background channel, diagnostic state, protocol documentation and plan.
- **type:** native agent debugging validation.
- **validation:** Formatting, feature check/build, state and diagnostics protocol calls, typed route navigation, post-action generation, acknowledged close and foreground-process preservation passed. One verifier assertion initially omitted the selected-tab query and was corrected; app behavior was unchanged.
- **performance:** Request-only state serialization; no baseline application overhead.
- **risks:** Source-paired parity remains 0/499 accepted; generic native semantic interaction is still unavailable.
- **follow_up:** Use the state receipts for the next source-paired workflow and finish the native locator/input adapter.
- **id:** HUI-072
- **timestamp:** 2026-11-15T20:28:40Z
- **what:** Removed a clipped development-only label from the visible pane bar.
- **why:** Keep the Rust shell aligned with the pinned Hermes UI and eliminate text overflow visible in the supplied capture.
- **components:** Hermes Native pane toolbar and parity plan.
- **type:** UI fidelity correction.
- **validation:** Pending build and headless semantic-tree inspection; no screenshot was taken because the default shell state would repeat prior captures.
- **performance:** No expected runtime change.
- **risks:** This small visual correction does not establish broader source parity.
- **follow_up:** Inspect source-paired toolbar and route states at the fixed source viewport.
- **id:** HUI-073
- **timestamp:** 2026-11-15T20:29:59Z
- **what:** Validated the source-visible pane buttons after removing the prototype-only label.
- **why:** Ensure the correction removes only development text while leaving Review, Files and Terminal available.
- **components:** Hermes Native pane toolbar and inherited-stdio semantic UI tree.
- **type:** UI fidelity validation.
- **validation:** Feature build and headless `ui.tree` assertions passed; the three pane buttons are visible, the development label is absent, and the protocol closed cleanly without stderr.
- **performance:** No expected regression.
- **risks:** Overall source-paired acceptance remains 0/499.
- **follow_up:** Continue matched source/target review of the toolbar, route content and other visible states.
- **id:** HUI-074
- **timestamp:** 2026-11-15T20:33:54Z
- **what:** Made native coordinate-click receipts fail closed when the target effect is not observed.
- **why:** Stop agents from treating a synthetic input event as successful navigation and repeatedly capturing the same route.
- **components:** Native-window agent protocol and automation evidence guidance.
- **type:** agent action correctness.
- **validation:** Pending direct protocol run confirming failed/unverified status, explicit input-dispatched context, state reread and no focus theft.
- **performance:** No meaningful runtime overhead.
- **risks:** Human-equivalent native semantic interactions are still missing.
- **follow_up:** Use verified typed actions until the native UIA/semantic adapter can prove control activation.
- **id:** HUI-075
- **timestamp:** 2026-11-15T20:35:02Z
- **what:** Confirmed that uncertain native pointer input cannot be reported as successful navigation.
- **why:** Prevent false progress captures and unsafe retries after an unverified click.
- **components:** Off-screen Hermes Native automation protocol.
- **type:** native UI action verification.
- **validation:** Native process returned the expected failure code, exposed dispatch/generation context, retained queryable `/` state, exited cleanly, and left foreground PID 5220 unchanged.
- **performance:** No added work beyond existing event dispatch.
- **risks:** Generic semantic native input remains a major acceptance gap.
- **follow_up:** Connect actual native locators/action effects, then exercise matched pinned-source workflows.
- **id:** HUI-076
- **timestamp:** 2026-11-15T20:36:01Z
- **what:** Synchronized the app backlog with fail-closed native pointer receipt semantics.
- **why:** Prevent stale backlog wording from implying native coordinate clicks are usable actions.
- **components:** Hermes Native feature backlog.
- **type:** documentation.
- **validation:** Backlog names the unverified receipt, possible dispatch, generation change and required state reread; native semantic action item remains unchecked.
- **performance:** Documentation only.
- **risks:** Native user-equivalent input still requires an effect-verifying adapter.
- **follow_up:** Keep the semantic native input acceptance item open.

- **id:** HUI-077
- **timestamp:** 2026-11-15T20:41:20Z
- **what:** Added an asynchronous, read-only native UI Automation tree method to the off-screen Hermes channel.
- **why:** Give headless agents semantic visibility into the actual native window while preserving desktop focus.
- **components:** Hermes Native window protocol and native automation docs.
- **type:** agent capability.
- **validation:** Pending actual process probe and route-differentiated tree verification.
- **performance:** Helper process has a 3-second timeout and tree caps of 256 nodes and 16 levels.
- **risks:** No native click/type/select/key action is enabled; UIA mutation can foreground the window.
- **follow_up:** Explore verified non-activating semantic actions and connect the Playwright-shaped client.
- **id:** HUI-078
- **timestamp:** 2026-11-15T20:47:30Z
- **what:** Verified native UI Automation trees distinguish New session and Capabilities routes.
- **why:** Establish a headless semantic inspection result without generating another screenshot.
- **components:** Off-screen native process, UIA reader, shared typed route action.
- **type:** native protocol verification.
- **validation:** Actual process returned 26 and 30 controls respectively; route action receipt reached `/capabilities`, protocol closed cleanly, and foreground PID 5220 was unchanged.
- **performance:** Completed within the 8-second request bound; UIA worker itself is bounded to 3 seconds.
- **risks:** Read-only inspection is not user-equivalent interaction or source parity.
- **follow_up:** Add an action adapter that never activates the operator-facing desktop and reports target effects.
- **id:** HUR-001-132
- **timestamp:** 2026-11-15T21:40:00Z
- **what:** Added a native Ctrl/Cmd+K/P command-palette foundation with registered actions, query filtering, keyboard selection and automation state.
- **why:** Advance the fixed-snapshot Hermes UI replacement with a distinct agent-operable interaction surface.
- **components:** pps/hermes-native/src/app.rs, native README/architecture/backlog/plan.
- **type:** UI capability.
- **validation:** Formatting, focused automation test and all-target feature check passed. Full source parity remains open.
- **performance:** No performance claim; bounded in-memory filtering.
- **risks:** Source command contributions/ranking/context and matched visual acceptance remain incomplete.
- **follow_up:** Complete command contributions, source-paired visuals and native keyboard validation.

- **id:** HUR-001-133
- **timestamp:** 2026-11-15T21:43:59Z
- **what:** Matched native command-palette search ranking to the fixed source score tiers and AND semantics.
- **why:** Improve interaction parity for the fixed downloaded Hermes snapshot.
- **components:** pps/hermes-native/src/app.rs, native backlog/architecture/plan.
- **type:** Interaction fidelity.
- **validation:** Ranking and open/search/navigation focused tests passed; formatting and all-target feature check passed.
- **performance:** No benchmark claim.
- **risks:** Contributions/settings-context/visual acceptance remain incomplete.
- **follow_up:** Add complete source groups and validate paired source behavior.

- **id:** HUR-001-134
- **timestamp:** 2026-11-15T21:48:08Z
- **what:** Added Profiles, Agents, and Star map to the native palette and aligned the supported Go to order with pinned Hermes.
- **why:** Improve fixed-source navigation behavior.
- **components:** pps/hermes-native/src/app.rs and its plan/backlog/architecture docs.
- **type:** Navigation parity.
- **validation:** Order and each registered route were asserted; formatting, focused tests, feature check and diff check passed.
- **performance:** No performance claim.
- **risks:** Full source groups and paired visuals remain open.
- **follow_up:** Implement source dynamic/contributed entries and complete destination screens.

- **id:** HUR-001-135
- **timestamp:** 2026-11-15T21:51:40Z
- **what:** Added session-aware command-palette search for cached pinned/active/archived sessions and direct Hermes session-ID jumps.
- **why:** Improve user and agent navigation to existing session work.
- **components:** pps/hermes-native/src/app.rs and native plan/backlog/README/architecture.
- **type:** Session navigation.
- **validation:** Session search and actual route-action test passed; all-target feature check passed.
- **performance:** In-memory scan only.
- **risks:** No fresh source-style all-profile session fetch or canonical pin order.
- **follow_up:** Implement correlated session reads and complete source-paired acceptance.

- **id:** HUR-001-136
- **timestamp:** 2026-11-15T22:01:38Z
- **what:** Added source-shaped all-profile session refresh and request fencing for command-palette results, plus refresh diagnostics.
- **why:** Let the palette discover sessions across profiles as Hermes Desktop does.
- **components:** pps/hermes-native/src/app.rs, pps/hermes-native/src/gateway.rs, native docs.
- **type:** Authenticated read/navigation.
- **validation:** Five focused palette tests, all-target feature check and diff check passed.
- **performance:** Active results bounded to 200; archived palette rows bounded to first 200.
- **risks:** Pin ordering and live bridge/source-paired proof remain open.
- **follow_up:** Verify live gateway refresh races and exact result ordering.

- **id:** HUR-001-137
- **timestamp:** 2026-11-15T22:12:10Z
- **what:** Added gateway-scoped persistent pin order and applied it to command-palette session results.
- **why:** Preserve the user's pinned navigation order without mixing independent gateway instances.
- **components:** pps/hermes-native/src/layout.rs, pps/hermes-native/src/app.rs, native docs.
- **type:** Persistent session navigation.
- **validation:** Persistence/scope and palette-order tests passed; all-target feature check passed.
- **performance:** Storage bounded to 64 gateway scopes and 20,000 IDs per scope.
- **risks:** Manual reorder and exact source connection-key mapping remain open.
- **follow_up:** Implement manual reorder and verify live connection scope.

- **id:** HUR-001-138
- **timestamp:** 2026-11-15T22:30:00Z
- **what:** Added saved-order pinned-session sidebar grouping and accessible pointer reordering with unresolved-pin preservation.
- **why:** Match the fixed Hermes Desktop snapshot's hand-reorderable Pinned list without dropping unloaded IDs.
- **components:** apps/hermes-native/src/app.rs, apps/hermes-native/src/layout.rs, automation protocol, native docs.
- **type:** Session navigation and persistent preferences.
- **validation:** Headless UI drag persisted the reordered gateway-scoped list and preserved an unloaded pin; full feature suite/build pending final run.
- **performance:** Bounded to visible loaded pins plus the persisted 20,000-ID scope limit; no benchmark claim.
- **risks:** Source-paired Windows visual/physical interaction and authenticated live gateway behavior remain unverified; PAR-0075 stays pending.
- **follow_up:** Complete matched source/target acceptance and continue remaining parity work.
- **id:** HUR-001-139
- **timestamp:** 2026-11-15T22:33:42Z
- **what:** Full feature validation passed for native pinned-session sidebar reorder.
- **why:** Confirm the approved parity slice in current source state before reporting progress.
- **components:** apps/hermes-native/src/app.rs, apps/hermes-native/src/layout.rs, apps/hermes-native/src/bin/hermes_automation/automation_protocol.rs, native docs.
- **type:** Validation record.
- **validation:** Formatting, 147 library tests, 13 automation tests, all-target feature check, headless runner build and diff check passed. The real egui pointer path persisted row reorder while retaining an unresolved pin ID.
- **performance:** No benchmark claim.
- **risks:** Windows physical/source-paired and authenticated-live evidence remain open.
- **follow_up:** Continue source-paired acceptance and remaining behaviors; PAR-0075 remains pending.
- **id:** HUR-001-140
- **timestamp:** 2026-11-15T22:37:36Z
- **what:** Fenced pinned-session row drags against gateway and persisted-order changes during the gesture.
- **why:** A concurrent pin/unpin result must not be overwritten by stale pointer state.
- **components:** apps/hermes-native/src/app.rs and native validation/docs.
- **type:** Session ordering safety and validation.
- **validation:** Formatting, 148 library tests, 13 automation tests, all-target feature check, runner build and diff check passed. Stale-order cancellation and successful persisted drag both have headless tests.
- **performance:** No benchmark claim.
- **risks:** Physical Windows/source-paired and live authenticated acceptance remain open.
- **follow_up:** Continue source-paired acceptance and remaining behaviors; keep PAR-0075 pending.
- **id:** HUR-001-141
- **timestamp:** 2026-11-15T23:07:58Z
- **what:** Ported fixed-source recent-session date grouping and manual within-group ordering into Hermes Native.
- **why:** Keep recency groups chronological while supporting manual order in the downloaded Desktop scope.
- **components:** `apps/hermes-native/src/app.rs`, `layout.rs`, automation fixture/protocol, parity docs and `artifacts/agent-ui/recent-session-groups-20261115-v3.png` with provenance.
- **type:** Native UI parity increment.
- **validation:** 154 library tests and 14 automation protocol tests passed; pointer drag persistence and cross-group rejection verified; all-target feature check, runner build, rustfmt and diff check passed.
- **performance:** No benchmark claim.
- **risks:** Source-paired Windows visual/physical and authenticated gateway checks remain open; PAR-0076 is still pending.
- **follow_up:** Continue the 499-row fixed-source acceptance audit; do not infer overall parity from this slice.
- **id:** HUR-001-142
- **timestamp:** 2026-11-15T23:19:14Z
- **what:** Added per-session color selection and persistence to Hermes Native's session rows.
- **why:** Advance fixed-snapshot PAR-0078 with the source palette and reachable row action.
- **components:** `apps/hermes-native/src/layout.rs`, `src/app.rs`, automation protocol, and Hermes Native parity documentation.
- **type:** Native UI parity increment.
- **validation:** 155 library and 15 automation protocol tests passed. Headless semantic UI verified selecting a palette color, reading it from saved preferences, then clearing it. All-target agent-automation check, binary build, rustfmt and diff check passed.
- **performance:** No benchmark claim.
- **risks:** Compression-lineage IDs and project color inheritance are absent; source-paired Windows/live acceptance remains pending. PAR-0078 stays pending.
- **follow_up:** Complete durable lineage mapping and project fallback, then execute matched source/target acceptance.
- **id:** HUR-001-143
- **timestamp:** 2026-11-15T23:28:57Z
- **what:** Updated the native color picker to a source-shaped six-column swatch grid and captured its populated state.
- **why:** Provide concrete, distinct visual evidence for the new per-session Appearance behavior.
- **components:** `apps/hermes-native/src/app.rs`, runner fixture, README/automation docs, and the session Appearance screenshot with provenance.
- **type:** Visual fixture and evidence.
- **validation:** 155 library tests and 15 automation protocol tests passed; all-target feature check, runner build, rustfmt and diff check passed. The menu and grid were captured and visually reviewed at 1440×900.
- **performance:** No benchmark claim.
- **risks:** Synthetic offline target-only evidence; compression-lineage/project fallback and source-paired Windows/live checks remain open. PAR-0078 remains pending.
- **follow_up:** Close remaining source behavior gaps and gather matched source/native Windows evidence before acceptance.
- **id:** HUR-001-144
- **timestamp:** 2026-11-15T23:32:00Z
- **what:** Corrected session swatch rendering from HSV-like conversion to exact source HSL and refreshed the capture.
- **why:** Match the CSS palette values from the fixed downloaded Hermes source.
- **components:** `apps/hermes-native/src/app.rs`, palette conversion test, session Appearance screenshot and sidecar.
- **type:** Color fidelity correction.
- **validation:** HSL RGB checks passed; 156 library and 15 protocol tests, feature check/build, rustfmt and diff checks passed. Capture was regenerated and visually reviewed.
- **performance:** No benchmark claim.
- **risks:** Compression lineage and inherited project colors are still missing; matched Windows/live acceptance remains open.
- **follow_up:** Continue PAR-0078 source behavior parity and acceptance evidence.
- **id:** HUR-001-145
- **timestamp:** 2026-11-15T23:40:39Z
- **what:** Added upstream compression-lineage transport to native session rows and used the lineage root as the durable color key.
- **why:** Keep a user's chosen session color attached through Hermes compression ID rotation.
- **components:** Hermes Native `gateway.rs`/`app.rs`, semantic automation coverage, parity plan/docs and refreshed screenshot provenance.
- **type:** Session identity parity.
- **validation:** REST fixture proves `_lineage_root_id` mapping; actual menu actions persist and clear via durable root key; 156 library and 15 protocol tests plus full feature check/build/fmt/diff passed.
- **performance:** No benchmark claim.
- **risks:** Project color inheritance and source-paired Windows/live validation remain incomplete; PAR-0078 remains pending.
- **follow_up:** Add project fallback and gather matched source/target evidence.
- **id:** HUR-001-146
- **timestamp:** 2026-11-15T23:52:34Z
- **what:** Implemented native session-dot inheritance from profile-scoped Hermes project-tree ownership and colors.
- **why:** Match the downloaded Desktop source fallback while retaining an explicit session color as the preferred value.
- **components:** Hermes Native gateway/app, focused fixtures and tests, plan/backlog/architecture/README.
- **type:** Session appearance parity.
- **validation:** `projects.tree` mock verified profile-bound RPC and owner color; parser/color tests cover source HSL palette, hex compatibility, override precedence and neutral fallback. 158 library and 15 protocol tests passed with all-target check, runner build, rustfmt and diff checks.
- **performance:** No benchmark claim; one profile-scoped tree read per visible profile per gateway connection.
- **risks:** Live authenticated behavior and matched Windows visual evidence remain open, so PAR-0078 is still pending.
- **follow_up:** Run source-paired Windows visuals and authenticated project-color workflow before acceptance.
- **id:** HUR-001-147
- **timestamp:** 2026-11-16T00:10:00Z
- **what:** Added stale response fencing and cache preservation for project color refreshes.
- **why:** Keep late or denied gateway reads from clobbering valid session appearance or a different profile.
- **components:** Hermes Native app receipt reducer, app regression test, plan and architecture docs.
- **type:** Async gateway state correctness.
- **validation:** Focused test covers stale request, matching error, last known good color, profile isolation and fresh successful replacement.
- **performance:** No benchmark claim.
- **risks:** Source-paired Windows visuals and authenticated live gateway proof remain open; PAR-0078 is pending.
- **follow_up:** Complete full feature validation, then source-paired and live tests.
- **id:** HUR-001-148
- **timestamp:** 2026-11-16T00:16:35Z
- **what:** Added native off-screen keypress dispatch with generation fencing and fail-closed effect receipts.
- **why:** Allow background agents to drive keyboard workflows in the actual Hermes Native window without foreground activation.
- **components:** Hermes Native agent protocol, egui input path, command-palette smoke, and parity/control documentation.
- **type:** Native agent control.
- **validation:** Actual native-window process dispatched Ctrl+P and Escape; state reads confirmed command-palette open then closed. 159 library tests and 15 protocol tests plus feature check/build/fmt/diff passed; child exited 0 with empty stderr.
- **performance:** No benchmark claim.
- **risks:** Native selector/menu, drag/drop, client integration, live gateway, and source-paired acceptance remain open.
- **follow_up:** Continue full native-window agent control and pinned-source parity verification.
- **id:** HUR-001-149
- **timestamp:** 2026-11-16T00:28:17Z
- **what:** Connected the HermesPage client to the actual off-screen native window through `HermesPage.launchNative()`.
- **why:** Make native UI navigation, state reads, keyboard control and screenshots available through the same Playwright-shaped interface.
- **components:** Hermes Native JavaScript client, client protocol tests, native host docs and architecture.
- **type:** Native agent control.
- **validation:** Four Node client tests passed; actual Windows process navigation, state-verified Ctrl+P/Escape, PNG-signature capture and clean close passed.
- **performance:** No benchmark claim.
- **risks:** Native selector, scroll and drag gestures, authenticated live gateway behavior and source-paired acceptance remain open.
- **follow_up:** Extend full user-equivalent native gestures and continue source-parity implementation.
- **id:** HUR-001-150
- **timestamp:** 2026-11-16T00:42:19Z
- **what:** Added actual native ComboBox option selection to the Playwright-shaped HermesPage client and hardened UIA popup geometry serialization.
- **why:** Allow agents to operate visible selectors in the real off-screen Hermes window, not only in the headless fixture runner.
- **components:** Hermes Native Windows UIA tree, HermesPage locator client, protocol fixtures and parity/control docs.
- **type:** Native agent control.
- **validation:** Actual native process selected Simple/Advanced Interface modes and Basic/Default Layout profiles with state readback; screenshot capture and clean close passed. Rust suites passed (159 + 15), all-target check/build and formatting/diff validation passed, and all 4 Node client tests passed.
- **performance:** No benchmark claim.
- **risks:** Native scrolling/drag, authenticated live gateway behavior and source-paired acceptance remain open.
- **follow_up:** Implement and verify native scroll/drag actions before treating the native agent surface as user-complete.
- **id:** HUR-001-151
- **timestamp:** 2026-11-16T01:10:40Z
- **what:** Added native coordinate wheel input through the Playwright-shaped HermesPage client.
- **why:** Enable background scroll interaction on populated native routes when a full UIA locator tree is too expensive.
- **components:** Hermes Native automation process/client, client tests, architecture and backlog docs.
- **type:** Native agent control.
- **validation:** 159 Rust library tests, 15 automation protocol tests and 4 Node tests passed; all-target checks/build, formatting and diff checks passed. On the actual off-screen 2,379-entry Plugins route, a wheel input changed the 2160×1350 screenshot digest and visibly moved different cards into view; the process remained gateway-disabled and used a temporary profile. Exact-locator tree reads still time out on that dense route.
- **performance:** No speedup claimed; tree-query bound remains eight seconds.
- **risks:** Coordinate wheel target effect must be verified from a screenshot or state read; native drag, live gateway and source-paired parity remain open.
- **follow_up:** Continue native gesture support and dense-tree UIA work without counting this as a source-parity acceptance.
- **id:** HUI-2026-11-16-progress-count-wording
  **timestamp:** 2026-11-16T01:25:51Z
  **what:** Reworded the parity status snapshot to report accepted requirements as a count and removed the ambiguous 0% label. Corrected the plan header's working revision to match its appended revision history.
  **why:** Keep source-parity acceptance distinct from partial engineering work; no overall implementation percentage has a defensible denominator.
  **components:** `docs/BACKLOG.md`, `apps/hermes-native/docs/BACKLOG.md`, `apps/hermes-native/docs/PLAN.md`.
  **type:** documentation / progress reporting.
  **validation:** Exact legacy lines were checked before replacement. The acceptance scope remains 499 in-scope requirements plus 66 excluded research rows. No feature or acceptance row changed.
  **performance:** No runtime change or performance claim.
  **risks:** Accepted parity count remains 0 of 499; this edit does not change implementation or acceptance evidence.
  **follow_up:** Report accepted parity counts separately from implemented features; do not give an aggregate percentage without a defined, validated measure.
- **id:** HUI-2026-11-16-first-run-remote-probe
  **timestamp:** 2026-11-16T01:42:09Z
  **what:** Added credential-free, asynchronous first-run remote gateway discovery for public status and optional OAuth-provider endpoints, with URL debounce, timeout/size/redirect bounds, stale-result fencing, and visible auth/provider/version state.
  **why:** Replace a dead first-run discovery surface with the pinned Desktop's real public auth-mode probe while keeping credentials out of discovery requests.
  **components:** `apps/hermes-native/src/remote_probe.rs`, `apps/hermes-native/src/app.rs`, `apps/hermes-native/Cargo.toml`, `apps/hermes-native/docs/`, `docs/ARCHITECTURE.md`, `docs/BACKLOG.md`.
  **type:** feat / parity workflow.
  **validation:** 163 Rust library tests and 15 automation protocol tests passed; all-target feature check and automation binary build passed; rustfmt passed. A local HTTP fixture confirmed `/api/status` and `/api/auth/providers` discovery, OAuth provider/version rendering, and absence of Authorization or X-Hermes-Session-Token headers. An egui harness verified the visible OAuth provider status.
  **performance:** Network probing is off the UI thread, starts after a 500 ms idle debounce, and uses an eight-second request timeout; no benchmark claim.
  **risks:** Probe discovery does not authenticate or establish the live gateway. OAuth/token login, authenticated HTTP+WebSocket test, apply/config persistence, and local bootstrap remain incomplete; PAR-0001 stays pending.
  **follow_up:** Implement source-compatible credential acquisition, exact tested-payload receipt, authenticated HTTP+WebSocket check, remote config persistence/apply, then local bootstrap lifecycle.
- **id:** HUI-2026-11-16-token-connection-test
  **timestamp:** 2026-11-16T16:09:56Z
  **what:** Added first-run token-authenticated Hermes `/api/status` and `/api/ws` checks, masked zeroizing UI input, background execution, stale-result fencing and bounded diagnostics.
  **why:** Verify both authenticated transports required by the pinned Desktop before remote setup can advance.
  **components:** `apps/hermes-native/src/remote_probe.rs`, `apps/hermes-native/src/app.rs`, `apps/hermes-native/README.md`, native/root architecture and backlog docs, plan and journals.
  **type:** feat / parity workflow.
  **validation:** 165 Rust library tests and 15 automation protocol tests passed; all-target feature check, formatting and diff checks passed. Local transport fixtures and an egui UI harness cover auth header/query and credential gating.
  **performance:** Network operations execute off the UI thread and HTTP requests are bounded to eight seconds; no benchmark claim.
  **risks:** OAuth login, exact-payload test receipt, Apply/config persistence, remote attachment and local bootstrap remain incomplete; no parity rows were accepted.
  **follow_up:** Implement OAuth login, tested-payload binding, isolated remote configuration/apply and native remote gateway attachment.
- **id:** HUI-2026-11-16-native-oauth-pkce
  **timestamp:** 2026-11-16T16:27:55Z
  **what:** Added first-run native OAuth PKCE login for gateways advertising `native_pkce`, provider-aware UI, loopback callback/state validation, token redemption, and bearer plus single-use-ticket transport testing.
  **why:** Implement the pinned Desktop's supported system-browser OAuth path and verify the distinct HTTP and WebSocket credentials.
  **components:** `apps/hermes-native/src/native_oauth.rs`, `src/remote_probe.rs`, `src/app.rs`, Cargo dependencies and project docs/journals.
  **type:** feat / parity workflow.
  **validation:** 167 Rust library tests and 15 protocol tests passed; all-target check, formatting and diff checks passed. Local loopback, token exchange, HTTP, WebSocket and egui fixtures passed.
  **performance:** Login and transport work run on worker threads with five-minute login, 15-second exchange and 512 KiB response bounds.
  **risks:** Browser opens only after Sign in is clicked. OAuth tokens are not persisted or refreshed; older/password-provider fallback login, exact-payload test receipt, Apply, remote runtime attachment and local bootstrap remain open. No parity row was accepted.
  **follow_up:** Implement token refresh/secure storage and fallback login, then remote configuration/apply and authenticated gateway attachment.
- **id:** HUI-2026-11-16-oauth-pkce-hardening
  **timestamp:** 2026-11-16T16:32:51Z
  **what:** Aligned mixed-provider OAuth eligibility with the pinned source, normalized URLs at the login worker boundary, redacted tokens from debug output, and added wrong-state rejection coverage.
  **why:** Close security and provider-strategy gaps in the newly implemented PKCE sign-in path.
  **components:** `apps/hermes-native/src/native_oauth.rs`, `src/app.rs`, implementation plan and journals.
  **type:** fix / security hardening.
  **validation:** 169 library tests and 15 protocol tests passed; all-target check, format and diff validation passed.
  **performance:** No material runtime change.
  **risks:** Secure persistence/refresh, legacy login, Apply and remote attach remain open; no source-parity rows accepted.
  **follow_up:** Add secure credential storage and refresh, then finish remote Apply/attachment.
- **id:** HUI-2026-11-16-oauth-ticket-redaction
  **timestamp:** 2026-11-16T16:34:14Z
  **what:** Redacted raw WebSocket client errors that could contain a single-use OAuth ticket from the connection URL.
  **why:** Prevent an ephemeral credential from appearing in user-facing errors or automation diagnostics.
  **components:** `apps/hermes-native/src/remote_probe.rs`, plan and journals.
  **type:** fix / secret redaction.
  **validation:** Full Rust feature test suite, all-target feature check, formatting and diff validation rerun.
  **performance:** No material runtime change.
  **risks:** Socket handshake error detail is intentionally generic; no parity rows accepted.
  **follow_up:** Continue the remote connection path without exposing credential-bearing URLs.
- **id:** HUI-2026-11-16-exact-remote-test-payload
  **timestamp:** 2026-11-16T16:37:36Z
  **what:** Bound Apply to a private SHA-256 receipt for the exact tested first-run URL, auth mode and token/bearer, and revoke eligibility after credential edits.
  **why:** Match the source UI's exact-payload pass requirement and prevent an earlier success from authorizing changed connection data.
  **components:** `apps/hermes-native/src/app.rs`, README, architecture, backlog, plan and journals.
  **type:** feat / parity workflow.
  **validation:** 169 Rust library tests and 15 protocol tests passed; all-target check/format/diff passed. egui test covered disabled, enabled for matching receipt, then disabled after token edit.
  **performance:** Local hash only; no network overhead.
  **risks:** Apply storage/attachment remains incomplete; no parity row accepted.
  **follow_up:** Implement the remote configuration write and gateway attach path.
- **id:** HUI-2026-11-16-native-oauth-refresh-dpapi
  **timestamp:** 2026-11-16T17:05:02Z
  **what:** Added pinned-source near-expiry OAuth refresh before remote Test and Windows DPAPI persistence for rotated native OAuth sessions under the isolated Hermes Native profile.
  **why:** Keep native PKCE sign-in usable across expiry and restart without writing secrets to preferences or installed Hermes data.
  **components:** `apps/hermes-native/src/native_oauth.rs`, `remote_probe.rs`, `native_token_store.rs`, `app.rs`, `layout.rs`, crate docs and Cargo metadata.
  **type:** feat / security / parity.
  **validation:** All-target feature tests (173 library, 15 protocol), check, fmt and diff passed; local gateway fixture verified refresh ordering and new bearer; Windows DPAPI round-trip verified ciphertext-only disk state. Package validator passed. UCM checker exposed existing EV-0016/17 schema errors and one reconciliation item; historical evidence remained unchanged.
  **performance:** Refresh occurs only inside the 60-second expiry margin; protected persistence is local I/O.
  **risks:** Startup restore, token-auth persistence, config apply, remote runtime attach, legacy password login and local bootstrap remain open.
  **follow_up:** Continue first-run connection persistence and remote attach; preserve the pinned snapshot target.

## HUI-2026-11-16-remote-setup-preferences

**id:** HUI-2026-11-16-remote-setup-preferences
**timestamp:** 2026-11-16T17:17:16Z
**what:** Persist the normalized gateway URL after a successful exact-payload Test and restore it, plus any matching DPAPI OAuth session, when Connect setup is reopened.
**why:** Keep the pinned Desktop setup flow useful across retries and app restarts without writing credentials to UI preferences.
**components:** `apps/hermes-native/src/app.rs`, `layout.rs`, README, backlog, architecture and plan docs.
**type:** feat / parity.
**validation:** Added workspace reload and unsafe-URL rejection test; targeted test passed. Full suite pending. No parity row accepted.
**performance:** Preference persistence is a single small atomic local write; OAuth decryption occurs after the credential-free status probe on setup navigation.
**risks:** Saved settings do not auto-attach or persist token-mode credentials; remote runtime support remains incomplete.
**follow_up:** Add remote authenticated gateway actor and startup connection lifecycle against the pinned API contract.

Validation update for HUI-2026-11-16-remote-setup-preferences (2026-11-16T17:21:50Z): full feature suite passed (174 library tests, 15 protocol tests), plus cargo check, rustfmt, diff check and package validator. Headless egui coverage now verifies exact-payload Apply persistence and URL restoration on reopening Connect. No parity row was accepted.

## HUI-2026-11-16-secure-token-mode-setup

**id:** HUI-2026-11-16-secure-token-mode-setup
**timestamp:** 2026-11-16T17:21:50Z
**what:** Save token-mode gateway credentials in the Windows DPAPI store on Apply and restore a URL/auth-mode-matched credential after discovery.
**why:** Retain the tested remote setup without writing static tokens into UI preferences.
**components:** apps/hermes-native/src/app.rs, native_token_store.rs, crate/root README, architecture, backlog and plan.
**type:** feat / security / parity.
**validation:** DPAPI type-separation and headless exact-payload Apply tests pass; full feature suite pending.
**performance:** A small local credential envelope is protected on Apply and read after the public gateway probe.
**risks:** This does not connect the remote runtime or auto-attach during startup.
**follow_up:** Implement and verify the pinned remote WebSocket and REST connection path.

Validation update for HUI-2026-11-16-secure-token-mode-setup (2026-11-16T17:35:35Z): 175 library tests and 15 protocol tests pass; cargo check, rustfmt, git diff --check and package validation pass. The UCM history map was not rewritten; source-paired acceptance remains 0/499. Timestamp note: the HUI-2026-11-16-secure-token-mode-setup timestamp field reused the prior entry time; this addendum gives the current verification time.
### HUI-2026-11-16-remote-websocket-attach

- **id:** HUI-2026-11-16-remote-websocket-attach
- **timestamp:** 2026-11-16T17:45:31Z
- **what:** First-run Apply launches authenticated remote WebSocket attachment for token and OAuth credentials; runtime identity and initial sessions are loaded. Saved URL-scoped credentials are restored for startup attach.
- **why:** Previously Apply only persisted the tested configuration, leaving the native app disconnected from the selected remote gateway.
- **components:** First-run UI, GatewayDiscovery, remote WebSocket attach, encrypted credential store, architecture/backlog/plan docs.
- **type:** feature
- **validation:** cargo check passed; all-target tests passed at 175 library and 15 protocol tests before two additional auth tests. One stale UI assertion failed and was updated. Final rerun pending; no live remote server was contacted.
- **performance:** Network work stays on the background gateway worker.
- **risks:** Remote HTTP operations remain loopback-only; runtime OAuth refresh persistence is incomplete; attach lacks a dedicated server fixture.
- **follow_up:** Add remote HTTP authentication and a runtime WebSocket fixture, then verify live attach and continue source-paired parity review.
Validation update for HUI-2026-11-16-remote-websocket-attach (2026-11-16T17:45:31Z): 177 library tests and 15 protocol tests pass; cargo check, rustfmt check, git diff check, and the 2026-11-13 package validator pass. The validator reports 491 pending capabilities; source-paired acceptance remains 0/499. Remote live attach and authenticated remote HTTP remain unverified.
### HUI-2026-11-16-remote-http-parity

- **id:** HUI-2026-11-16-remote-http-parity
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
- **what:** Added per-user Telegram QR allowlist chips with Add/Enter validation, owner indication, source-style empty-list apply validation and an existing-bot replacement warning.
- **why:** Improve native interaction parity with the pinned Hermes Desktop Messaging route.
- **components:** Hermes Native Messaging UI and source parity records.
- **type:** feature / UI parity
- **validation:** Focused allowlist mutation and headless interaction/accessibility checks passed; full suite 203+15, all-target feature check, rustfmt and diff check passed; package validator reports 491 capabilities pending.
- **performance:** No additional polling or network calls.
- **risks:** Bot credential lifecycle, localization, live authentication and paired Windows evidence remain incomplete.
- **follow_up:** Continue pinned-source parity work; no requirement acceptance row advanced.

- **id:** HUR-001-172
- **timestamp:** 2026-11-16T21:41:36Z
- **what:** Aligned Telegram quick setup with the source two-column order and ready/waiting action placement.
- **why:** Correct a visible Messaging interaction layout mismatch.
- **components:** Hermes Native Telegram setup and parity docs.
- **type:** feature / UI parity
- **validation:** Headless Messaging fixture and full suite 203+15 passed; all-target feature check, rustfmt, diff check and package validator passed (491 capability rows pending).
- **performance:** No polling or network behavior changed.
- **risks:** Visual comparison against the pinned Windows desktop remains open.
- **follow_up:** Continue pinned-source behavior and visual parity; no requirement row was accepted.

- **id:** HUR-001-173
- **timestamp:** 2026-11-16T21:51:24Z
- **what:** Added a read-only, profile-scoped Scheduled Jobs list/detail route with search and async error/retry states.
- **why:** Replace a native unavailable-route placeholder with source-backed content.
- **components:** Hermes Native gateway and Scheduled Jobs UI.
- **type:** feature / route parity
- **validation:** Remote transport and headless UI fixtures pass; final full suite 205+15, all-target check, rustfmt and diff passed; validator reports 491 capabilities pending.
- **performance:** Uses the bounded HTTP worker; no blocking UI calls.
- **risks:** Create/edit/run/control/history remain incomplete.
- **follow_up:** Continue source-matched Scheduled Jobs interactions; no parity row advanced.

- **id:** HUR-001-174
- **timestamp:** 2026-11-16T21:57:42Z
- **what:** Added selected Scheduled Job Pause/Resume with authenticated, profile-bound requests and receipt checks.
- **why:** Extend the new route from read-only inventory to a source-supported action.
- **components:** Hermes Native gateway and Scheduled Jobs detail UI.
- **type:** feature / lifecycle parity
- **validation:** Focused transport and UI fixtures pass; full suite 206+15, all-target check, rustfmt and diff passed; validator reports 491 capabilities pending.
- **performance:** No additional UI-thread networking or polling.
- **risks:** Other job management paths and source-paired acceptance remain incomplete.
- **follow_up:** Continue Scheduled Jobs source actions; no parity acceptance row advanced.

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

## HUR-001-178 — 2026-11-16

- **id:** HUR-001-178
- **timestamp:** 2026-11-16T22:57:19Z
- **what:** Added reproducible requirement crosswalk and partial implementation ledger; corrected active Scheduled Jobs IDs to PAR-0300–0309 and identified PAR-0282 as Messaging.
- **why:** The supplemental 491-row package omits 42 fixed-scope rows and adds 34 new rows; implementation progress requires explicit mapping and evidence.
- **components:** Hermes Native status generators, 599-row crosswalk, 499-row implementation overlay, completion overview and active backlog.
- **type:** evidence and documentation
- **validation:** Generators validated IDs, fixed source pins, matrix joins, scope counts and every seeded evidence locator. Status counts are 5 implemented locally, 6 partial, 1 not implemented and 487 unknown; strict source acceptance stays 0/499.
- **performance:** Offline deterministic generation only.
- **risks:** Structural ID matches do not transfer evidence. All 12 implementation rows remain live-unverified and source-pair pending.
- **follow_up:** Review the remaining 487 fixed-scope requirements and continue UI implementation.

- **id:** HUR-001-179
- **timestamp:** 2026-11-16T23:42:35Z
- **what:** Added profile-scoped Scheduled Jobs create/edit/delete, delivery destinations, model catalog, typed blueprints, script-only-safe updates and delete confirmation; refreshed partial-completion counts from validated evidence.
- **why:** Continue the pinned Hermes Desktop replacement while reporting locally implemented and partial behavior separately from source acceptance.
- **components:** Hermes Native gateway/app, completion overview/status ledger, plan, backlog, architecture and READMEs.
- **type:** feature and progress evidence
- **validation:** Full feature suite passed (217 library + 15 protocol tests); all-target feature check and rustfmt check passed; focused Cron transport (9 tests) and headless Scheduled Jobs tests passed. Both source package validators passed; status generators were deterministic; diff check passed. Current reviewed rows: 6 implemented, 9 partial, 484 unknown; strict source acceptance remains 0/499.
- **performance:** No benchmark performed or claimed.
- **risks:** Live authenticated behavior and matched Windows visuals remain unverified. The pinned Desktop lacks the timezone editor named by PAR-0302. UCM YAML parse was skipped because PyYAML is unavailable.
- **follow_up:** Validate the CRUD forms against an isolated live Hermes profile, capture paired source/native visuals, clarify PAR-0302, and continue the remaining pinned requirements.

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
- **components:** `apps/hermes-native/src/session_peer_sync.rs`, `src/lib.rs`, `src/app.rs`, source audit, plan, progress docs, READMEs, and project journals.
- **type:** parity behavior and local synchronization
- **validation:** Full Rust feature suite passed (243 library, 15 automation protocol, 3 launcher tests); a separate-process receipt test passed; 14 continuity tests passed; all-target cargo check, rustfmt and diff checks passed; progress generator passed (23/499 reviewed, 0/499 accepted); `.doofus` state validated. The optional fixed-source package validator was not present in this checkout and was not run. PAR-0011 remains partial.
- **performance:** Native windows rescan a capped (512 final receipts) local directory every 250 ms while open; no CPU/latency benchmark or physical window interaction test was run.
- **risks:** The existing `%APPDATA%\HermesNative` parent ACL grants the current user, SYSTEM and Administrators full control and the Codex sandbox group read/execute only. Physical parent/child window delivery remains unverified. Process suspension longer than bounded retention can miss individual pings; initial/periodic canonical refresh must close that gap. Other mutation callers, transcript/tile/project sync and source-paired acceptance remain open.
- **follow_up:** Inspect remaining source mutation callers, implement transcript peer/tile reconciliation, then verify real Windows parent/child interaction.

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
- **what:** Added a typed, bounded single-profile `projects.tree` read, a collapsible nested Projects sidebar, and accessible session-opening rows. Added profile/connection/generation fencing and last-good-tree preservation tests; updated implementation evidence locators and the partial status overview.
- **why:** Turn PAR-0011's project-change color-only refresh into a measurable first structural project-tree slice.
- **components:** Hermes Native gateway/app, progress generator and CSV overlay, parity plan/source audit/architecture/backlog/overview, root architecture/backlog.
- **type:** parity behavior and evidence reporting
- **validation:** Focused tree RPC and stale-generation/cache-preservation tests pass. Full feature suite and portable continuity suite are pending in this record until rerun.
- **performance:** Tree request remains bounded to three preview sessions per project and 60-second gateway timeout; no runtime benchmark recorded.
- **risks:** All-profile endpoint, project-list parity, repo discovery, tombstones, live gateway, physical Windows behavior and source-paired acceptance remain open.
- **follow_up:** Finish full validation and checkpoint; continue source-backed PAR-0011 reconciliation without claiming the requirement accepted.

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
- **what:** Added explicit project deletion with confirmation, concrete-profile `projects.delete`, optimistic list/tree removal, separate active-pointer clearing, profile-keyed stale-read tombstones, authoritative-tree reconciliation, and rollback of caches and project drill-in on failure.
- **why:** Continue parity against the downloaded Hermes Desktop snapshot while retaining project files/session data and rejecting synthetic all-profile mutation.
- **components:** Hermes Native app/gateway, PAR-0011 evidence and completion docs, progress overlay, local task checkpoint, execution journals.
- **type:** parity behavior and evidence reporting
- **validation:** 273 Rust library + 15 automation-protocol + 3 launcher tests passed; provider continuity 15/15; all-target check and formatter passed; progress overlay builder passed (23/499 reviewed, 6 implemented, 17 partial, 476 unknown, 0/499 accepted); pinned package validator passed 565 behavior/UI/test rows, 14 source receipts and 2 implementation receipts (optional PyYAML unavailable); task validator passed at revision 44 before checkpoint; `git diff --check` passed with existing LF-to-CRLF notices.
- **performance:** One bounded delete RPC and one authoritative tree refresh on confirmed success; no live-service benchmark.
- **risks:** Headless evidence only; auto-project dismiss/Undo, repo discovery, live service, Windows behavior and source-paired acceptance remain open. A focused test first failed on incorrect fixture casing (`is_auto`), then passed after matching `isAuto`.
- **follow_up:** Continue local repository discovery and auto-project dismiss/Undo, then live and source-paired validation against the pinned snapshot.

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

- **id:** hermes-progress-overview-reconcile-20261117
- **timestamp:** 2026-11-17T21:40:42Z
- **what:** Updated the handoff-facing completion overview, PAR-0011 evidence row, architecture, READMEs and backlog to reflect headless repository-discovery integration and its remaining live/source-paired gaps.
- **why:** Keep progress reporting current and distinguish partial implementation from runtime verification, visual pairing and formal parity acceptance.
- **components:** Native app/gateway/scanner, progress CSV, completion overview, plan, source audit, architecture, backlog, READMEs and handoff ledger.
- **type:** implementation status and evidence documentation
- **validation:** Rust 294 + 15 + 3, continuity 16/16, focused repo scan 7/7, repo config/local scan 4/4, settings 2/2, cargo check, fmt, package validation, progress generation and diff check passed. No live gateway or source-paired acceptance claimed.
- **performance:** No benchmark recorded.
- **risks:** Remote live behavior, physical Windows behavior, visual pairing, and remaining PAR-0011 work remain open; ETA/critical path unavailable.
- **follow_up:** Continue only against pinned source and update each evidence dimension from verified receipts.


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
- components: apps/hermes-native/tools/doofus_task.py, apps/hermes-native/tools/test_doofus_task.py, docs/AGENT_HANDOFF.md, docs/architecture/AGENT_TASK_CONTRACTS.md, apps/hermes-native/docs/COMPLETION_OVERVIEW.md.
- type: progress reporting and continuity validation
- validation: Python compilation passed; provider continuity suite passed 16/16.
- performance: No measured performance impact; benchmark not applicable.
- risks: Confidence is qualitative and is not a calibrated prediction interval.
- follow_up: Record phase-sized remaining work and measured durations; calibrate EWMA error before raising confidence.

- id: hermes-positional-dock-group-fold-20261117
- timestamp: 2026-11-17T23:20:12Z
- what: Vendored the pinned DoofusDock group-minimize extension and wired Hermes root-edge folding, restore rails and tab context-menu action.
- why: Preserve whole-group/tab identity in canonical layout state for the pinned positional fold behavior.
- components: apps/hermes-native/Cargo.toml, Cargo.toml, Cargo.lock, apps/hermes-native/vendor/doofusdock-core, apps/hermes-native/vendor/doofusdock-presentation, apps/hermes-native/src/app.rs, apps/hermes-native/src/layout.rs, apps/hermes-native/docs/PLAN.md Revision 202.
- type: Native UI parity implementation.
- validation: cargo check --workspace --all-targets --locked passed; Hermes Native feature suite passed 302 library + 15 automation + 3 launcher tests.
- performance: No benchmark; edge lookup runs only on a keyboard shortcut and walks the root tree.
- risks: Overall parity remains partial; titlebar controls, owner-aware restoration, arbitrary/narrow layout and Windows source-paired validation remain open.
- follow_up: Validate visible rail interactions through the headless UI protocol and implement titlebar edge controls.

- id: hermes-process-records-20261117
- timestamp: 2026-11-17T23:05:22Z
- what: Added lease-checked process and external-operation record CLI commands, typed schema constraints, reconciliation visibility in status, and handoff retention tests.
- why: Ensure unknown/running provider work is not silently treated as stopped when a conversation or owner changes.
- components: apps/hermes-native/tools/doofus_task.py, apps/hermes-native/tools/test_doofus_task.py, .doofus/schemas/handoff.schema.json, docs/AGENT_HANDOFF.md, docs/architecture/AGENT_TASK_CONTRACTS.md, apps/hermes-native/docs/PLAN.md Revision 201, BACKLOG.md, COMPLETION_OVERVIEW.md.
- type: handoff protocol and recovery implementation
- validation: Python compilation passed; continuity suite passed 18/18; persisted schemas/ledger validate at revision 69. Independent DoofusDock verification in the isolated pinned worktree passed 427 tests in 42 suites.
- performance: No performance change; no benchmark.
- risks: Process discovery remains explicit and manual; provider-supplied state can become stale and must be re-observed by the incoming owner.
- follow_up: Integrate canonical DoofusDock group minimize in Hermes Native; keep the immutable dependency source and acceptance state explicit.

- id: hermes-titlebar-edge-controls-20261117
- timestamp: 2026-11-17T23:33:53Z
- what: Exposed accessible positional left/right side controls in Hermes Native and validated the semantic fallback path.
- why: Keep the pinned titlebar side action reachable to people and agents through the native UI.
- components: apps/hermes-native/src/app.rs, apps/hermes-native/src/layout.rs, apps/hermes-native/docs/PLAN.md Revision 203.
- type: Native UI parity implementation.
- validation: All-target Cargo check; 304 library + 15 automation + 3 launcher tests; continuity 18/18; 36/499 review coverage remains unchanged.
- performance: No measured change.
- risks: Source acceptance remains pending; narrow-window and live Windows behavior remain unverified.
- follow_up: Add side-owner restoration and further source-paired coverage.

- id: hermes-titlebar-edge-evidence-sync-20261117
- timestamp: 2026-11-17T23:37:12Z
- what: Refreshed the native Hermes partial-progress evidence for positional side controls.
- why: Cite current UI behavior while preserving fixed scope and pending acceptance.
- components: apps/hermes-native/tools/build_implementation_progress.py, apps/hermes-native/docs/status/IMPLEMENTATION_PROGRESS.csv, apps/hermes-native/docs/PLAN.md Revision 203.
- type: Evidence documentation.
- validation: Generator passed; 36/499 reviewed and 0/499 accepted; continuity 18/18.
- performance: No runtime change.
- risks: Headless evidence does not prove live or visual parity.
- follow_up: Continue toward complete pinned-source behavior and acceptance.


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
- what: Extended Hermes Native narrow transient overlays to the pinned built-in Review pane, updated zone tabs and 237px/85vw sizing, and synchronized root project docs/status.
- why: The fixed Hermes source marks Review collapsible; its omission left a source-required pane inaccessible in the previous narrow Sessions/Files-only implementation.
- components: apps/hermes-native/src/app.rs; apps/hermes-native/src/layout.rs; progress seed and generated overlay; Hermes Native plan, audit, architecture, backlog, completion overview and README; root architecture, backlog and README.
- type: Native UI parity implementation and validation.
- validation: 311 + 15 + 3 feature tests; continuity 18/18; all-target check; fixed-scope progress 38/499 (6 implemented, 32 partial, 461 unknown; 0 accepted); task state validator revision 79 / ledger 9; supplemental validator 491 pending; Python compilation, rustfmt and git diff check passed.
- performance: No performance change measured.
- risks: Dynamic pane/plugin contributions, reveal aliases, live zone width overrides, physical Windows and source-paired runtime/visual evidence remain open; strict source acceptance is 0/499.
- follow_up: Continue fixed-snapshot source gap implementation and maintain separate evidence for local behavior, live runtime and source-paired acceptance.


- id: hermes-pane-reveal-aliases-20261118
- timestamp: 2026-11-18T01:11:18Z
- what: Exposed pinned Sessions and Files reveal aliases through the shared typed human/agent pane action and added UI/protocol tests.
- why: Agent-controlled narrow pane visibility must resolve the same source IDs as the human-facing control path.
- components: Hermes Native app/action protocol; progress evidence; app/source/agent documentation.
- type: Agent-control parity implementation and validation.
- validation: 311 + 15 + 3 feature tests; continuity 18/18; all-target check; 38/499 reviewed (6 implemented, 32 partial, 461 unknown, 0 accepted); task validator revision 81 / ledger 9; package validator 491 pending; compile, format and diff checks passed.
- performance: No performance change measured.
- risks: Generic plugin contributions, explicit reveal modes, Bot Mode roster UI, live Windows and matched source acceptance remain open.
- follow_up: Implement the real Bot Mode roster vertical slice and keep the fixed pin and strict acceptance state unchanged.

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


### Hermes Command Center System slice — 2026-11-18T16:00:51Z

- id: hermes-command-center-system-20261118
- timestamp: 2026-11-18T16:00:51Z
- what: Added bounded, authenticated, profile-owned gateway status/log transport and a headless-tested Command Center System overlay.
- why: Continue the fixed downloaded Hermes snapshot parity in the missing Command Center sections.
- components: `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, native parity and architecture docs.
- type: Read-only source-matched feature slice.
- validation: Focused authenticated HTTP and rendered-overlay tests passed; full validation follows.
- performance: No performance change measured.
- risks: Runtime, physical Windows and source-paired visual acceptance remain pending.
- follow_up: Run full regression and continuity checks; Usage and Maintenance remain next.
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


### Hermes Native Command Center Usage — 2026-11-18T16:45:14Z

- id: hermes-command-center-usage-20261118
- timestamp: 2026-11-18T16:45:14Z
- what: Added typed, bounded active-profile Usage analytics transport and a native 7/30/90-day view with summary stats, stacked daily token chart, top models/skills, refresh and loading/error states.
- why: Continue the fixed Hermes snapshot replacement with a source-backed Command Center section.
- components: Hermes Native gateway transport, Command Center overlay, source/architecture/backlog/completion records.
- type: Source-matched read-only feature slice.
- validation: Focused HTTP/UI fixtures and full local feature, workspace, continuity, progress, package, task and formatting checks passed.
- performance: No benchmark-relevant runtime change measured.
- risks: Live gateway, Windows and source-paired screenshot acceptance remain pending.
- follow_up: Continue the Command Center Maintenance contract and remaining Sessions actions; keep acceptance separate from local behavior tests.


### Hermes Native Usage refresh cache parity — 2026-11-18T16:54:08Z

- id: hermes-command-center-usage-cache-parity-20261118
- timestamp: 2026-11-18T16:54:08Z
- what: Kept the active-profile last-good analytics response visible while a changed Usage period loads or fails.
- why: Match the pinned Hermes Usage component's stale-while-refresh behavior while retaining strict request/period/owner fencing for replacements.
- components: Hermes Native Command Center Usage state and headless overlay regression.
- type: Source-fidelity correction.
- validation: Focused regression and full feature/workspace/continuity/progress/package/format validators pass.
- performance: No measurable change.
- risks: Visible range selection can lead the displayed cached range until refresh settles, as in the fixed source.
- follow_up: Continue Maintenance and remaining Sessions behavior.


### Hermes Native Command Center Maintenance — 2026-11-18T18:10:37Z

- id: hermes-command-center-maintenance-20261118
- timestamp: 2026-11-18T18:10:37Z
- what: Completed and validated the native Command Center Maintenance section with profile-bound Curator/Memory status reads, background action status polling, Curator pause toggle, pre-confirmation profile owner capture for Debug Share, and confirmed `MEMORY.md` / `USER.md` resets.
- why: Finish the in-flight Command Center Maintenance slice against the pinned Hermes snapshot after resuming from Codex.
- components: `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, remote-authenticated HTTP and headless egui fixtures, parity docs.
- type: Source-matched feature slice.
- validation: PASS: focused HTTP/UI fixtures; 325 library + 15 automation protocol + 3 launcher tests; locked workspace check; continuity 18/18; progress 38/499 reviewed (0/499 accepted); package validator 491 pending; compileall, rustfmt and diff check pass.
- performance: No measurable runtime regression.
- risks: Live gateway, physical Windows and source-paired visual acceptance remain pending.
- follow_up: Implement remaining Command Center Sessions behavior (confirmed delete, export, cross-profile pagination).


### Hermes Native Command Center Sessions confirmed deletion — 2026-11-18T19:45:00Z

- id: hermes-command-center-session-delete-20261118
- timestamp: 2026-11-18T19:45:00Z
- what: Added confirmation-gated session deletion to Command Center session rows, snapshot-fenced `DELETE /api/sessions/{id}` transport, multi-cache eviction, project-session tombstone recording, and peer session-list change broadcast.
- why: Match the pinned Command Center Sessions delete confirmation (#99410) and fenced HTTP delete contract.
- components: `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, remote-authenticated HTTP and headless egui fixtures, parity docs.
- type: Source-matched feature slice.
- validation: PASS: focused HTTP/UI fixtures; 327 library + 15 automation protocol + 3 launcher tests; locked workspace check; continuity 18/18; progress 38/499 reviewed (0/499 accepted); package validator 491 pending; compileall, rustfmt and diff check pass.
- performance: No measurable runtime regression.
- risks: Live gateway, physical Windows and source-paired visual acceptance remain pending.
- follow_up: Implement Command Center session JSON export and cross-profile truncated pagination.


### Hermes Native Command Center Sessions JSON export + Load more & System restart/update actions — 2026-11-18T21:25:00Z

- id: hermes-command-center-export-system-actions-20261118
- timestamp: 2026-11-18T21:25:00Z
- what: Added Command Center Sessions JSON export (`Export`, paging `GET /api/sessions/{id}/messages?limit=500&offset=N&order=oldest&include_compacted=true` up to 32M JSON chars and writing `{title}-{id}.json`), `"Load more (+200)"` pagination, and Command Center System `"Restart gateway"` (`POST /api/gateway/restart`) / `"Update Hermes"` (`POST /api/hermes/update`) actions with 18-poll / 1.2s status polling (`?lines=180`) and post-action System refresh.
- why: Match the pinned Command Center Sessions export/load-more and System action contracts.
- components: `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, remote-authenticated HTTP and headless egui fixtures, parity docs.
- type: Source-matched feature slice.
- validation: PASS: focused HTTP/UI fixtures; 331 library + 15 automation protocol + 3 launcher tests; locked workspace check; continuity 18/18; progress 38/499 reviewed (0/499 accepted); package validator 491 pending; compileall, rustfmt and diff check pass.
- performance: No measurable runtime regression.
- risks: Live gateway, physical Windows and source-paired visual acceptance remain pending.
- follow_up: Continue next pinned-source parity slices.
### Hermes Native main publication — 2026-11-18T21:42:43Z

- id: `codex-main-publish-20261118`
- timestamp: `2026-11-18T21:42:43Z`
- what: Published the handoff's accumulated 130-file Hermes native parity change set as commit `95198cda1b92ae4c56eb795a8586b138c88801d5` on `main` and pushed it to `origin/main`.
- why: The operator requested the repository stay current and specified `main` only.
- components: Hermes Native implementation, vendored DoofusDock crates, source audit and progress artifacts, agent tooling, execution evidence, and durable `.doofus` task state.
- type: Repository publication.
- validation: 331 library + 15 protocol + 3 launcher tests; locked all-target workspace check; 18 task-ledger tests; pinned capability validator; Python compile, format and diff checks; post-push fetch shows `HEAD == origin/main`, zero ahead/behind, clean checkout.
- performance: No performance measurement was part of this synchronization.
- risks: The published project is still partial: 38/499 source requirements reviewed, 0/499 accepted, live runtime and source-paired visual evidence remain unverified.
- follow_up: Continue with the next fixed-source parity slice on `main` only.


### Hermes Native Command Center profile-union pagination — 2026-11-18T22:08:45Z

- id: `hermes-command-center-profile-pagination-20261118`
- timestamp: `2026-11-18T22:08:45Z`
- what: Replaced the ineffective fixed-200 refresh with the pinned 50-row `/api/profiles/sessions/sidebar` window, profile-specific truncation state, a 404 legacy route fallback, partial-failure row/flag retention, a visible partial error, and `+50` Load more that issues the larger next request.
- why: Match the downloaded Hermes snapshot; its Command Center uses `SIDEBAR_SESSIONS_PAGE_SIZE=50` and server truncation metadata.
- components: `apps/hermes-native/src/app.rs`, `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/docs/PLAN.md`, `apps/hermes-native/docs/SOURCE_AUDIT.md`, `apps/hermes-native/docs/backlog.md`.
- type: Fixed-source parity behavior slice.
- validation: PASS focused batched/legacy HTTP, cross-profile carry-forward, refresh ownership and headless Load more tests; full suite 334 library + 15 automation protocol + 3 launcher tests; locked workspace check (pre-existing dead-code warnings only); continuity 18/18; pinned package validation 491 pending; progress 38/499 reviewed and 0/499 accepted; Python compileall, rustfmt and diff checks pass.
- performance: No benchmark change; request cap remains bounded to 2,000 and response body remains bounded to 512 KiB.
- risks: 180 ms query debounce, live Hermes runtime, Windows and source-matched visual evidence remain open; no strict source acceptance changed.
- follow_up: Continue exact source behavior gaps (including the 180 ms search debounce), then live/runtime and source-paired validation; publish this verified slice on `main`.


### Hermes Native Command Center publication — 2026-11-18T22:14:35Z

- id: `hermes-command-center-pagination-publish-20261118`
- timestamp: `2026-11-18T22:14:35Z`
- what: Published `fa9b2f0a5e64ad4dbf096beec463bd5559673d55` to `origin/main`.
- why: The operator specified main-only work and requested the repository remain current.
- components: Hermes Native Command Center implementation and parity documentation.
- type: Repository publication.
- validation: Push succeeded; a subsequent fetch confirmed `HEAD == origin/main == fa9b2f0a5e64ad4dbf096beec463bd5559673d55`; checkout was clean.
- performance: No performance measurement was part of publication.
- risks: The change is partial fixed-source parity; 38/499 rows reviewed, 0/499 formally accepted, live runtime and visual source pairs unverified.
- follow_up: Continue the next pinned-source gap on `main` only.


### Hermes Native Command Center search debounce — 2026-11-18T22:26Z

- id: `hermes-command-center-search-debounce-20261118`
- timestamp: `2026-11-18T22:26:00Z`
- what: Added the pinned 180 ms debounced Sessions search, title/preview/Untitled fallback matching, session ID matching, delayed empty-state/Load more transitions, and deterministic headless regressions for typing and clearing.
- why: Match the downloaded fixed Hermes source snapshot and avoid presenting unsettled query results.
- components: `apps/hermes-native/src/app.rs`, `apps/hermes-native/docs/PLAN.md`, `apps/hermes-native/docs/SOURCE_AUDIT.md`, `apps/hermes-native/docs/backlog.md`, `apps/hermes-native/docs/COMPLETION_OVERVIEW.md`, `README.md`.
- type: Fixed-source UI parity behavior.
- validation: PASS focused debounce tests (2); PASS full native feature suite (336 library + 15 automation protocol + 3 launcher); PASS locked workspace/all-target check; PASS continuity 18/18; PASS pinned package validator (491 pending); PASS progress generation (38/499 reviewed, 0/499 accepted), Python compileall, rustfmt and diff checks.
- performance: No performance benchmark change; one repaint is scheduled for the remaining debounce duration.
- risks: Live Hermes runtime, Windows behavior, all 499 source-paired visual captures and formal source acceptance remain unverified.
- follow_up: Continue the next incomplete pinned Command Center/session state on `main` only.


### Hermes Native Command Center section route state — 2026-11-18T22:35Z

- id: `hermes-command-center-section-route-state-20261118`
- timestamp: `2026-11-18T22:35:00Z`
- what: Reflected the selected Command Center section in the agent-visible canonical route query, matching replace-style section routing: nondefault sections include `?section=...`; Sessions removes the query.
- why: The pinned UI encodes active section in the route so state remains externally observable and reproducible.
- components: `apps/hermes-native/src/app.rs`, `apps/hermes-native/docs/PLAN.md`, `apps/hermes-native/docs/SOURCE_AUDIT.md`, `apps/hermes-native/docs/backlog.md`, `apps/hermes-native/docs/COMPLETION_OVERVIEW.md`, `apps/hermes-native/README.md`.
- type: Fixed-source route-state parity.
- validation: PASS headless real-overlay click regression verifies System and default Sessions route readback; PASS full suite (337 library + 15 automation protocol + 3 launcher), locked workspace check, continuity 18/18, pinned package/progress validators, Python compileall, rustfmt and diff checks.
- performance: No material runtime or performance change.
- risks: This validates native application route readback, not browser URL/history; formal acceptance and source-paired visuals remain pending.
- follow_up: Run the complete validators and continue the next pinned Command Center state on `main` only.


### Hermes Native main publication — 2026-11-18T22:32Z

- id: `hermes-main-publish-8daf857-20261118`
- timestamp: `2026-11-18T22:32:00Z`
- what: Published tested search-debounce commit `8daf857946add06c4ea2131d1c5e2205ed407566` to `origin/main`.
- why: The operator specified main-only work and requested that the repository stay current.
- components: Hermes Native Command Center Sessions search and parity documentation.
- type: Repository publication.
- validation: Push succeeded; fetch verified local `HEAD` equals `origin/main`; worktree was clean at verification.
- performance: No performance benchmark was part of publication.
- risks: This is partial fixed-source parity; formal acceptance remains 0/499 and runtime/visual evidence remains unverified.
- follow_up: Continue native parity on `main` only.


### Hermes Native Command Center Sessions row presentation — 2026-11-18T23:00Z

- id: `hermes-command-center-session-row-presentation-20261118`
- timestamp: `2026-11-18T23:00:00Z`
- what: Matched the compact title/activity-time row, source recency fallback, hover/focus-revealed compact action glyphs, removed native-only row metadata, and sized scrolling to the available pane. Added an offline eight-row CLI fixture and captured a distinct 1440×900 screenshot.
- why: The native session rows had conspicuous screenshot-level mismatches with the pinned Command Center.
- components: `apps/hermes-native/src/app.rs`, `apps/hermes-native/src/bin/hermes-automation.rs`, `apps/hermes-native/README.md`, `apps/hermes-native/docs/PLAN.md`, `apps/hermes-native/docs/SOURCE_AUDIT.md`, `apps/hermes-native/docs/BACKLOG.md`, `apps/hermes-native/docs/COMPLETION_OVERVIEW.md`, `artifacts/agent-ui/command-center-sessions.png`.
- type: Fixed-source visual/interaction parity and headless fixture.
- validation: PASS focused timestamp, actual overlay row/open and offline fixture tests; full suite (339 library + 15 protocol + 3 launcher); locked workspace/all-target check (existing dead-code warnings only); task continuity 18/18; pinned package validator (491 pending); progress 38/499 reviewed, 6 implemented, 32 partial, 461 unknown, 0/499 accepted; Python compileall, rustfmt and diff check; headless runner inspect and 1440x900 PNG capture. No live gateway or installed UI validation.
- performance: No benchmark was requested or run; list height is bounded to available content.
- risks: Icons are glyph approximations; date formatting is English medium/short style rather than locale-specific Intl; capture is target-only, offline evidence.
- follow_up: Complete all validators and continue remaining pinned Command Center visual/state gaps on `main` only.


### Hermes Native Command Center navigation and overlay framing — 2026-11-18T23:45:00Z

- id: `hermes-command-center-navigation-frame-20261118`
- timestamp: `2026-11-18T23:45:00Z`
- what: Added pinned wide left-rail and narrow dropdown navigation, breadcrumb/search header placement, near-full viewport Command Center overlay and floating close control. Fixed clipped session action hit targets and removed duplicate Maintenance heading. Added wide and compact target-only screenshots.
- why: Existing Command Center screenshots and layout diverged visibly from the pinned source structure.
- components: `apps/hermes-native/src/app.rs`, `apps/hermes-native/docs/PLAN.md`, `apps/hermes-native/docs/SOURCE_AUDIT.md`, `apps/hermes-native/docs/BACKLOG.md`, `apps/hermes-native/docs/COMPLETION_OVERVIEW.md`, `apps/hermes-native/README.md`, `artifacts/agent-ui/command-center-sessions-navigation.png`, `artifacts/agent-ui/command-center-sessions-compact.png`.
- type: Fixed-source UI parity and responsive interaction.
- validation: PASS 340 library + 15 automation protocol + 3 launcher tests; headless wide/compact runner captures and route/delete/maintenance regressions. Remaining validators and publication are recorded in execution log after completion. No live gateway or installed UI was opened.
- performance: No benchmark requested; session actions remain bounded by the existing page contract.
- risks: 700px global toolbar still overlaps behind the overlay; vector icon artwork is approximate; captures are synthetic target-only; formal acceptance remains 0/499.
- follow_up: Fix narrow global toolbar and continue pinned Command Center source review on `main` only.


### Hermes Command Center session row gestures — 2026-11-19T00:13:00Z

- **id:** hermes-session-row-gestures-20261119
- **timestamp:** 2026-11-19T00:13:00Z
- **what:** Implemented pinned Alt+Shift archive, Ctrl/Cmd+Shift new window, Ctrl/Cmd new tab, Shift pin/unpin, plain in-place open and middle-click tab behavior on the real native session row.
- **why:** Close a source-confirmed interaction gap discovered while verifying whether prior parity slices were already published to `main`.
- **components:** `apps/hermes-native/src/app.rs`, native session-row routing and gesture tests, parity plan/audit/backlog/completion/readme.
- **type:** parity implementation and headless validation.
- **validation:** 342 library + 15 protocol + 3 launcher tests pass; workspace check passes with only existing dead-code warnings; task continuity 18/18; pinned package validator reports 491 pending; review 38/499 and acceptance 0/499; compileall, rustfmt, diff checks pass.
- **performance:** no performance change measured or expected.
- **risks:** physical Windows modifier behavior, live gateway behavior and source-paired visuals remain unverified; the explicit active-session resume request is still open.
- **follow_up:** verify active-session `requestSessionResume` parity, then continue runtime and source-paired acceptance.


### Hermes active session reselect reattach — 2026-11-19T00:32:25Z

- **id:** hermes-active-session-reattach-20261119
- **timestamp:** 2026-11-19T00:32:25Z
- **what:** Added explicit profile-scoped session resume on plain sidebar row reselect, including the currently active session; replaces stale request ownership, keeps draft/runtime/transcript visible while pending, and rejects reselect during archive/delete mutations.
- **why:** Match pinned `openStoredSession` plus route-resume behavior and close the warm-session no-op in native.
- **components:** `apps/hermes-native/src/app.rs`, test-only gateway command observation in `gateway.rs`, progress generator and overlay, parity plan/source audit/backlog/completion/readme.
- **type:** parity implementation and headless protocol validation.
- **validation:** Full feature suite 344 library + 15 protocol + 3 launcher; workspace/all-target check passes with existing dead-code warnings only; task continuity 18/18; pinned package validator 491 pending; review 39/499 (6 implemented, 33 partial, 460 unknown) / acceptance 0/499; Python compileall, rustfmt and diff checks pass.
- **performance:** no performance change measured or expected.
- **risks:** No live gateway/in-flight turn or Windows UI was exercised; native owner routing has profile identity but not source connection identity.
- **follow_up:** Exercise live in-flight reattach and failure/reconnect cases, then collect same-environment source/target evidence for PAR-0510.


### Hermes UCM partial evidence for PAR-0510 — 2026-11-19T00:40:50Z

- **id:** hermes-ucm-par-0510-ev0018-20261119
- **timestamp:** 2026-11-19T00:40:50Z
- **what:** Added EV-0018 and changed the supplemental UCM capability to `partial`, with explicit headless, live-in-flight, and failure/recovery scenarios.
- **why:** Keep the product capability map current without implying live gateway verification or formal PAR acceptance.
- **components:** `docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/maps/capabilities.yaml`, `maps/capabilities/evidence/EV-0018.json`, parity plan.
- **type:** evidence-backed capability status update.
- **validation:** Targeted UCM report shows PAR-0510 as partial, with the headless scenario covered and live/failure scenarios open. Global checker still reports historical EV-0016/17 schema errors and EV-0017 timestamp reconciliation.
- **performance:** no performance change.
- **risks:** This evidence is headless only; no live backend, Windows installed UI, or matched screenshots.
- **follow_up:** Verify the in-flight and failure/recovery scenarios against a live target and preserve the current source pin.


### Hermes package validator accepts typed test evidence — 2026-11-19T00:46:33Z

- **id:** hermes-ucm-test-result-validator-20261119
- **timestamp:** 2026-11-19T00:46:33Z
- **what:** Extended the pinned package validator to recognize `test_result` evidence with a passing or partial outcome, pinned commit, mapped capability, scenario, and locator.
- **why:** EV-0018 uses the UCM skill's `test_result` evidence kind; the prior validator rejected it before validating the map.
- **components:** `docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/tools/validate_package.py`
- **type:** validation compatibility fix.
- **validation:** Pinned package validator, targeted UCM capability report, task continuity tests, and `git diff --check`.
- **performance:** no runtime impact; validator-only.
- **risks:** Existing historical UCM evidence inconsistencies remain separate and unchanged.
- **follow_up:** Keep evidence-kind validation aligned with the installed UCM schema as new evidence types are added.


### Publish the PAR-0510 evidence batch on main — 2026-11-19T00:49:50Z

- **id:** hermes-par-0510-evidence-publish-20261119
- **timestamp:** 2026-11-19T00:49:50Z
- **what:** Published the UCM partial record, validator support, and task/docs checkpoint as `08eb30f` on `main`.
- **why:** Keep finished evidence and validation tooling synchronized with the implementation already published on `main`.
- **components:** UCM capability map/evidence, pinned package validator, parity plan, execution log, Doofus handoff state.
- **type:** evidence and validation publication.
- **validation:** Pushed to `origin/main`, fetched, and confirmed `HEAD == origin/main == 08eb30f73393a39de4e07b924f12603d7fe0a4d8`; package validator and 18/18 task continuity tests pass; targeted UCM report keeps PAR-0510 partial with two scenarios open.
- **performance:** no runtime impact.
- **risks:** Global UCM report continues to show historical EV-0016/17 schema errors and reconciliation warning; no live gateway evidence was added.
- **follow_up:** Validate the live in-flight reattach and owner-routing/recovery cases when a safe test gateway is available.


### Hermes loopback reattach stream and retry coverage — 2026-11-19T00:59:38Z

- **id:** hermes-par-0510-loopback-stream-retry-20261119
- **timestamp:** 2026-11-19T00:59:38Z
- **what:** Added a `gateway_actor` WebSocket fixture that handles a refused resume, a successful retry, in-flight stream events around a returned snapshot, and completion ordering; updated progress/docs to preserve the partial status and exact remaining owner/live gaps.
- **why:** Prove native transport behavior while a resume is pending and verify recovery without contacting a user backend.
- **components:** `apps/hermes-native/src/gateway.rs`, progress generator/CSV, parity plan, source audit, backlog, completion overview, README.
- **type:** gateway protocol integration test and evidence update.
- **validation:** Focused loopback regression passes; full feature suite 345 library + 15 protocol + 3 launcher passes; locked workspace/all-target check passes with existing dead-code warnings; continuity 18/18, pinned package validator, progress generator, rustfmt and diff checks pass.
- **performance:** test-only code and documentation; no runtime performance change.
- **risks:** This uses a local fixture rather than a real Hermes backend. Native session rows still lack the pinned source's exact `(connectionId, profile)` owner route; Windows and source-paired evidence remain open.
- **follow_up:** Implement and validate exact session owner routing, then exercise production reconnect/in-flight behavior and source/target Windows parity.


### UCM evidence for loopback in-flight reattach — 2026-11-19T01:02:00Z

- **id:** hermes-ucm-par-0510-ev0019-20261119
- **timestamp:** 2026-11-19T01:02:00Z
- **what:** Added EV-0019 for the local gateway loopback resume/retry/stream scenario and declared its coverage in the UCM map.
- **why:** Keep protocol integration proof distinct from live-backend, owner-route, Windows, and visual acceptance.
- **components:** UCM capability map/evidence and Hermes parity plan.
- **type:** evidence-backed status refinement; PAR-0510 remains partial.
- **validation:** Targeted UCM report shows `gateway-loopback-inflight` and `headless-trigger` covered, with `live-inflight` and `failure-recovery` open; package validator passes. The global checker still reports the pre-existing EV-0016/17 schema errors and timestamp reconciliation.
- **performance:** no runtime change.
- **risks:** Loopback behavior does not prove production Hermes behavior or exact `(connectionId, profile)` routing.
- **follow_up:** Implement the source owner route and obtain controlled live gateway evidence.


### Publish EV-0019 loopback evidence on main — 2026-11-19T01:04:04Z

- **id:** hermes-par-0510-ev0019-publish-20261119
- **timestamp:** 2026-11-19T01:04:04Z
- **what:** Published EV-0019, the loopback scenario declaration, and validation notes as `31c5c43` on `main`.
- **why:** Keep the capability map synchronized with the tested implementation commit.
- **components:** UCM map/evidence, parity plan, execution log.
- **type:** evidence publication.
- **validation:** Targeted UCM report confirms headless and loopback scenarios covered, live/failure scenarios open; package validator passes; fetched `origin/main` equals `31c5c43`.
- **performance:** no runtime impact.
- **risks:** Historical UCM global errors remain; no real backend or Windows visual proof was added.
- **follow_up:** Implement source owner routing and pursue live-runtime parity.


### Capability evidence and parity/backport sequence — 2026-11-19

- **id:** hermes-ucm-layout-keybinding-evidence-20261119
- **timestamp:** 2026-11-19T01:58:30Z
- **what:** Added UCM evidence EV-0020 through EV-0022 for layout preset persistence, native keybinding dispatch and shortcut conflict warnings; recorded source action-registry coverage as still open. Added the user's backport sequencing clarification to the parity docs.
- **why:** Keep the capability map evidence-backed and align implementation order with the user's plan to backport Doofus Hermes only after native UI functional parity.
- **components:** Native parity plan/backlog; UCM capability map/evidence; execution and change journals.
- **type:** evidence and sequencing documentation.
- **validation:** Targeted UCM reports show the three implementation scenarios covered. Global checker still reports historical EV-0016/17 invalid kind/outcome plus EV-0017 timestamp reconciliation; historical evidence was preserved. Pending post-edit package and task validators.
- **performance:** no runtime impact.
- **risks:** No live gateway, physical Windows or source-paired acceptance was added; keybinding action registry remains incomplete.
- **follow_up:** Publish evidence/docs, then implement exact session connection/profile routing through the existing Hermes Gateway path.


### Exact session-owner registry foundation — 2026-11-19T02:15:20Z

- **id:** hermes-session-owner-registry-foundation-20261119
- **timestamp:** 2026-11-19T02:15:20Z
- **what:** Added generic stable-ID connection registration, duplicate rejection, exact tagged owner resolution, one explicit legacy route and fail-closed unknown-owner behavior in the session-owner module.
- **why:** Provide a safe routing primitive before connecting multiple Hermes Gateway instances to session UI state.
- **components:** Hermes Native session-owner identity and registry.
- **type:** feature foundation.
- **validation:** Focused registry tests 10/10; full workspace tests 371 native + 15 automation protocol + 3 launcher; all-target workspace check; rustfmt; diff; continuity tests 18/18; pinned package validator passes. Target UCM reports verify new evidence; global UCM remains red only on historical EV-0016/17 errors and EV-0017 reconciliation.
- **performance:** no runtime call sites added.
- **risks:** Registry is not wired into the app or Gateway yet; no multi-connection product behavior is claimed.
- **follow_up:** Carry the exact owner through endpoint descriptors, session identity caches, Gateway requests and correlated results; add same-ID cross-connection isolation tests.

Validation addendum: The targeted UCM report marks named layout preset persistence verified from the real picker/persistence tests; keybinding settings/conflict capabilities remain partial with the full pinned action registry scenario open. Evidence EV-0020/21/22 is bound to the published implementation commit c0e1713. Backporting Doofus Hermes remains sequenced after full native functional parity.\n

### Exact owner registry main publication — 2026-11-19T02:18:10Z

- **id:** hermes-session-owner-foundation-main-20261119
- **timestamp:** 2026-11-19T02:18:10Z
- **what:** Published exact owner registry foundation, UCM EV-0020/21/22 and parity/backport sequencing as commit 21e349e84827a90de3c6bb573da916e930e7438c on main.
- **why:** Keep the shared repository current with tested parity work and persistent task/evidence records.
- **components:** Hermes Native owner identity, UCM map/evidence, parity plan/backlog, task ledger.
- **type:** main publication.
- **validation:** Fetched origin/main and confirmed exact equality with HEAD at 21e349e84827a90de3c6bb573da916e930e7438c.
- **performance:** no runtime performance change.
- **risks:** The registry remains a foundation; app and Gateway routing are not integrated.
- **follow_up:** Implement connection descriptors and carry exact ownership through UI and Gateway actions.\n

### Persist Gateway connection metadata — 2026-11-19T02:25:50Z

- **id:** hermes-gateway-connection-descriptors-20261119
- **timestamp:** 2026-11-19T02:25:50Z
- **what:** Added local/remote stable Gateway connection descriptors, selected connection metadata, URL-keyed remote IDs, and backward-compatible migration from the legacy saved Gateway URL.
- **why:** Establish persistent, non-secret connection identity before exact session owner routing is wired into the Hermes Gateway path.
- **components:** Hermes Native workspace preferences and connection descriptor model.
- **type:** configuration foundation.
- **validation:** Full workspace suite 375 native + 15 automation protocol + 3 launcher; workspace all-target check; rustfmt and diff check pass. Four descriptor/migration focused tests are included in the full suite.
- **performance:** no Gateway connection runtime or UI behavior changed yet.
- **risks:** Descriptors are not consumed by app startup or a connection picker; no multi-connection session routing is claimed.
- **follow_up:** Drive Gateway discovery from selected descriptors and preserve connection IDs across all session request/result/cache paths.\n

### Gateway descriptor main publication — 2026-11-19T02:29:30Z

- **id:** hermes-gateway-connection-descriptors-main-20261119
- **timestamp:** 2026-11-19T02:29:30Z
- **what:** Published stable Gateway descriptors and migration as commit 0c1e27271191e8e5eef2d3d23947af0136507f22 on main.
- **why:** Keep the shared repo and canonical handoff current with tested owner-routing foundations.
- **components:** Hermes Native preferences, parity plan/backlog and task ledger.
- **type:** main publication.
- **validation:** Fetched origin/main and confirmed exact SHA equality with local HEAD.
- **performance:** no runtime connection behavior changed.
- **risks:** Descriptor metadata is not yet consumed by Gateway startup or session routing.
- **follow_up:** Wire selected descriptor and exact owner identity into app session state and Gateway clients.\n

### Selected Gateway startup — 2026-11-19T02:34:30Z

- **id:** hermes-selected-gateway-startup-20261119
- **timestamp:** 2026-11-19T02:34:30Z
- **what:** Make startup attach to the selected stable Gateway descriptor; add persisted local/remote selection API with unknown-ID rejection and preserve the remote URL compatibility pointer.
- **why:** Make the new descriptor selection state drive actual Gateway startup while retaining the existing Hermes discovery and secure credential path.
- **components:** Hermes Native workspace preferences and app Gateway startup.
- **type:** routing foundation.
- **validation:** Full workspace tests pass (376 native + 15 automation protocol + 3 launcher); all-target workspace check passes with existing dead-code warnings; rustfmt and diff checks pass. Layout tests cover remote/local persistence and unknown-ID refusal.
- **performance:** no additional background Gateway actor; the app still holds one discovery.
- **risks:** No UI picker or concurrent per-connection discovery exists; session owner routing remains incomplete.
- **follow_up:** Expose saved connection management in Settings, then route owner-tagged session data through per-connection Gateway actors.\n

### Selected Gateway startup main publication — 2026-11-19T02:36:40Z

- **id:** hermes-selected-gateway-startup-main-20261119
- **timestamp:** 2026-11-19T02:36:40Z
- **what:** Published selected-descriptor startup routing as commit d4c73d7c0afd721b8293d360aca212e22a8dcc9a on main.
- **why:** Make the saved connection selection control startup while preserving established Hermes auth and execution paths.
- **components:** Native app startup, workspace descriptors, parity plan/backlog and task ledger.
- **type:** main publication.
- **validation:** Fetched origin/main and confirmed exact equality with local HEAD at d4c73d7c0afd721b8293d360aca212e22a8dcc9a.
- **performance:** no second Gateway actor is created; one selected discovery remains active.
- **risks:** No user-facing saved-connection picker or session owner routing yet.
- **follow_up:** Implement the picker/safe-switch flow and exact owner routing across caches, Gateway messages and outcomes.

### PAR-0510 session-owner boundary and Gateway retirement — 2026-11-19T15:23Z

- **id:** hermes-par-0510-owner-boundary-20261119
- **timestamp:** 2026-11-19T15:23:40Z
- **what:** Added application-level saved-owner resolution for sidebar session open/window/mutation actions and a consuming GatewayDiscovery shutdown API that drops result receivers before sender channels.
- **why:** Prevent owner-tagged session rows from dispatching through the currently selected but different Hermes connection, and establish a bounded actor-retirement primitive for future connection switching.
- **components:** Native app session actions, GatewayDiscovery actor lifecycle, native README/architecture, PAR-0510 plan/backlog.
- **type:** owner routing and lifecycle foundation.
- **validation:** PASS focused owner guard (1/1); PASS full suite (378 library + 15 automation protocol + 3 launcher); PASS workspace all-target check; PASS rustfmt check. A prior diff check noted the pending publication changelog's blank EOF; rerun after this append.
- **performance:** registry resolution scans the small saved descriptor list per session action; shutdown has no wait/join and closes receiver/sender channels synchronously.
- **risks:** No connection picker or multi-actor route exists yet. An already-sent HTTP mutation or network attach is not canceled by shutdown. Session cache keys and Gateway results still need exact owner tags.
- **follow_up:** Add connection-aware session cache/request/result ownership and prove same-ID cross-connection isolation before exposing live connection switching.

### PAR-0510 main publication receipt — 2026-11-19T15:30Z

- **id:** hermes-par-0510-owner-boundary-main-publication-20261119
- **timestamp:** 2026-11-19T15:30:06Z
- **what:** Published the validated owner-boundary and Gateway retirement slice as commit d930ad3a05b33a015fec8254bae2fe47ab90dd18 on main.
- **why:** Keep the repository current on the user-requested main-only delivery path and preserve exact publication evidence.
- **components:** Doofus Hermes main branch and publication audit.
- **type:** main publication.
- **validation:** Fetched origin/main after push and verified exact SHA equality with local HEAD.
- **performance:** no runtime starts or new Gateway connections were initiated by publication.
- **risks:** Functional parity remains incomplete; published change is a fail-closed owner and lifecycle foundation.
- **follow_up:** Continue connection-aware session keys and Gateway request/result correlation before adding the saved connection picker.

### Chat and tool goal scope revision — 2026-11-19T15:41Z

- **id:** hermes-chat-tool-scope-revision-20261119
- **timestamp:** 2026-11-19T15:41:34Z
- **what:** Reframed the active acceptance target from whole-desktop parity to 138 pinned-source chat/tool requirements; added an explicit selector CSV and an atomic progress-overlay generator.
- **why:** Apply the user's narrowed goal while retaining the 499-row source inventory as a separate reference.
- **components:** Chat/tool scope matrix, scoped progress generator/output, native plan/backlog/README/architecture/completion overview.
- **type:** goal scope and progress tracking.
- **validation:** PASS generator output: 138 rows; 1 implemented, 8 partial, 129 unknown; 9 with mapped evidence; all 138 live runtime, source pair and visual pair states remain unverified/pending. PASS Python bytecode compilation. Diff and task validation pending final checkpoint.
- **performance:** generator completes in under one second on the current 499-row overlay.
- **risks:** Boundary is a deliberate working interpretation; standalone desktop panes and administration are out unless a scoped chat/tool requirement depends on them.
- **follow_up:** Continue in-scope chat/session/Gateway and tool lifecycle work, maintaining the scoped denominator separately from all-desktop source review.

Validation addendum (2026-11-19T15:47:28Z): PASS scoped progress generator (138 unique IDs), Python compilation, `.doofus` schema validation (revision 185 / ledger revision 20 / 439 events), and `git diff --check`. The project plan, README, backlog, architecture and completion overview now state the narrowed chat/tool goal.


### Chat/tool scope expanded for skills and plugins — 2026-11-19T15:52:00Z

- **id:** hermes-chat-tool-scope-skills-plugins-20261119
- **timestamp:** 2026-11-19T15:52:00Z
- **what:** Expanded the explicit chat/tool scope from 138 to 173 pinned-source requirements, including profile skills, skill/toolset management, MCP integrations, and native desktop plugin permissions, contributions, lifecycle, routing, diagnostics, and provenance.
- **why:** The user clarified that skills and plugins are part of the desired chat/tool feature set.
- **components:** Scope selector CSV, generated progress overlay, plan, README, backlog, completion overview, architecture, execution log, and durable task state.
- **type:** goal scope and progress tracking.
- **validation:** Generator and schema/build validation pending after this expansion. Previous scope commit `e6c3dfe09fea9b49c690694e62b18bbab25e1bb4` is pushed to `main`; fresh fetch confirmed local and remote SHA equality.
- **performance:** Scope generation is local and completes in under one second.
- **risks:** The 173-row boundary includes plugin/MCP capability requirements that support using those tools and extensions in chat; standalone unrelated administration remains excluded.
- **follow_up:** Continue implementation against the expanded selector and keep live runtime, source acceptance, visuals, and implementation state separate.

Validation/publication addendum (2026-11-19T15:53:08Z): PASS the 173-row generator, Python compilation, `.doofus` state validation and `git diff --check`. Pushed scope commit `79635cad1b6152e690b088e1c06aa0791c0c7c93` to `main`; a fresh fetch confirmed exact SHA equality. The first operation-receipt call had an invalid locator and made no state change; the retry with a validated file locator succeeded.


### Connection-owned Hermes session resume — 2026-11-19T16:41:18Z

- **id:** hermes-par-0510-connection-owned-resume-20261119
- **timestamp:** 2026-11-19T16:41:18Z
- **what:** Added connection identity to session-pane and view ownership, preserved it through close/restore, attributed resume events to their Gateway discovery actor, and fenced resume and peer transcript results by owner. Recorded four focused implementation tests against PAR-0510.
- **why:** Avoid cross-connection state collisions when separate Hermes Gateways expose the same profile and session IDs.
- **components:** Native app session routing, Gateway discovery events, DoofusDock session pane metadata, scoped progress evidence and project docs.
- **type:** Functional implementation and progress evidence.
- **validation:** Focused and complete native test suite passed in the preceding continuation; workspace check, rustfmt, task validator and progress generator are being rerun for this recorded slice. PAR-0510 remains partial; no live Gateway, Windows installed-app, source-paired visual, or formal acceptance is claimed.
- **performance:** No benchmark scenario changed; no live Gateway was started.
- **risks:** Saved-connection picker/concurrent discovery and owner routing through all other session operations are still missing.
- **follow_up:** Continue exact-owner routing for remaining session APIs, then add multi-connection lifecycle and live reconnect coverage.

Validation addendum (2026-11-19T16:44:23Z): PASS `cargo test --manifest-path apps/hermes-native/Cargo.toml --features agent-automation --locked` (382 library + 15 automation protocol + 3 launcher); PASS `cargo check --workspace --all-targets --locked` (existing dead-code warnings only); PASS `cargo fmt --manifest-path apps/hermes-native/Cargo.toml --all -- --check`; PASS task protocol tests (18/18); PASS task schema validation at revision 192; PASS 173-row scoped progress generator (1 implemented, 8 partial, 164 unknown; 9 rows mapped); PASS `git diff --check`. Live Hermes Gateway behavior, installed Windows behavior, source-paired visuals, and formal acceptance remain unverified.


Publication addendum (2026-11-19T16:47:12Z): Commit `b349bf96378e250c8ec10f2dae6c9e28da1659b2` was pushed to `origin/main`; a fresh fetch verified exact SHA equality and a clean worktree.


### UCM connection-owner evidence — 2026-11-19T16:51:05Z

- **id:** hermes-par-0510-ucm-connection-owner-20261119
- **timestamp:** 2026-11-19T16:51:05Z
- **what:** Appended EV-0023, declared the connection-owner isolation scenario, and linked it to the existing partial session reattach capability.
- **why:** Keep the user-requested partial completion view tied to reviewed, current test evidence.
- **components:** UCM capability map and evidence.
- **type:** Evidence tracking.
- **validation:** PASS package validator (565 capabilities/inventory/tests; 14 inspection and 8 implementation receipts); optional YAML parse skipped because PyYAML is not installed.
- **performance:** No runtime behavior changed.
- **risks:** The UCM checker remains unavailable in this environment without PyYAML; package validator confirms its own crosswalk and receipt constraints.
- **follow_up:** Keep UCM evidence and the 173-row implementation view synchronized as requirements are implemented.


### Gateway replay-gap transcript recovery — 2026-11-19T16:58:47Z

- **id:** hermes-par-0511-replay-gap-implementation-20261119
- **timestamp:** 2026-11-19T16:58:47Z
- **what:** Added `session.replay_gap` handling that resolves one exact active/open session owner and requests an authoritative resume; preserves transcript and draft; coalesces repeated events while resume is pending. Added active-route and pane tests.
- **why:** Recover missed transcript events according to the pinned Hermes replay-gap behavior without duplicating or misrouting session state.
- **components:** Gateway event routing, session-owner cache lookup, transcript recovery, PAR-0511 progress evidence.
- **type:** Functional implementation and headless regression coverage.
- **validation:** Focused replay-gap tests pass (2/2); app package check passes. Full suite and progress/task validators pending. PAR-0511 remains partial.
- **performance:** No runtime benchmark or live Gateway was run.
- **risks:** No automatic reconnect loop; live replay behavior and ambiguous-owner failure path remain unverified.
- **follow_up:** Add live/automatic reconnect and formal source-paired evidence; prioritize source-defined attachment and session-branch behavior after this protocol recovery slice.


Validation addendum (2026-11-19T17:01:09Z): PASS complete Hermes Native feature suite (384 library + 15 automation protocol + 3 launcher); PASS `cargo check --workspace --all-targets --locked` with existing dead-code warnings; PASS task protocol tests (18/18); PASS PAR-0511 task update and schema validation at revision 200; PASS scoped progress generator (1 implemented, 9 partial, 163 unknown; 10/173 mapped evidence); PASS UCM package validator (565 target behaviors; optional PyYAML parse skipped); PASS rustfmt and diff check. Automatic Gateway reconnect, live replay behavior, Windows acceptance and source-paired visuals remain open.


### UCM replay-gap evidence — 2026-11-19T17:02:14Z

- **id:** hermes-par-0511-ucm-replay-gap-20261119
- **timestamp:** 2026-11-19T17:02:14Z
- **what:** Added EV-0024 and EV-0025 for active-route and mounted-tile replay-gap recovery; declared the corresponding verification scenarios and kept the live reconnect scenario open.
- **why:** Preserve accurate partial status for the new pinned chat recovery behavior.
- **components:** Universal Capability Map replay-gap capability.
- **type:** Evidence tracking.
- **validation:** UCM package validator pending; schema keeps the capability partial.
- **performance:** No runtime behavior changed.
- **risks:** Unit coverage does not establish automatic or live Gateway reconnection.
- **follow_up:** Add live reconnect evidence only after the reconnect actor exists and can be exercised.

Publication addendum (2026-11-19T17:02:58Z): Published `a2f4ed924307df440b1028494743ad0d6aa8e5d8` to `origin/main`; fresh fetch confirmed exact local/remote SHA equality and a clean worktree.

### Native Hermes composer attachments — 2026-11-19T17:26:31Z

- **id:** hermes-par-0062-composer-attachments-20261119
- **timestamp:** 2026-11-19T17:26:31Z
- **what:** Added multi-file picker and session/profile-scoped native attachment staging via pinned Hermes image/file RPCs; included staged refs before prompt text and added 16 MiB validation.
- **why:** PAR-0062 was a P0 chat gap blocking image/file use in the native composer.
- **components:** Hermes Native composer, Gateway command/event protocol, pinned progress overlay and UCM crosswalk.
- **type:** feature
- **validation:** Full hermes-native tests passed: 387 library, 15 automation protocol, 3 launcher; focused tests cover byte/data-URL payloads, size guard, reference ordering and wrong-session refusal. Live Gateway and Windows picker were not exercised.
- **performance:** File reading and base64 encoding run on the Gateway actor, not in the egui frame; uploads are capped at 16 MiB. No benchmark claimed.
- **risks:** Restart-safe image draft restore, stale-session reattach, preview/paste/drop, and source path-vs-bytes policy remain incomplete.
- **follow_up:** Implement explicit no-send prepared image recovery, validate RPC error paths against a local/pinned Gateway, then complete Windows and source-paired acceptance.

### UCM evidence for native composer attachments — 2026-11-19T17:35:00Z

- **id:** hermes-par-0062-ucm-evidence-20261119
- **timestamp:** 2026-11-19T17:35:00Z
- **what:** Added EV-0026 for native attachment payload and composer behavior, bound to implementation commit `3dc32577292e9a9596d1e93cfc772a64d59739da`; kept capability status partial.
- **why:** Keep the capability map synchronized with the tested implementation and its unresolved runtime/source acceptance gates.
- **components:** UCM capability evidence, Hermes attachment progress overlay.
- **type:** evidence
- **validation:** Package validator passes 565 target behaviors, 565 inventory rows, 565 pending tests, 14 source receipts and 10 implementation receipts. PyYAML is absent, so optional UCM YAML parsing was skipped.
- **performance:** No runtime performance claim; payload size remains capped at 16 MiB.
- **risks:** Headless payload tests do not prove a live Gateway accepts the upload or that Windows picker behavior matches source.
- **follow_up:** Implement PAR-0063 restore, then verify live attachment RPC results and source-paired Windows behavior.


### PAR-0063 prepared image draft recovery — 2026-11-19T17:54:06Z

- **id:** hermes-par-0063-prepared-image-draft-recovery-20261119
- **timestamp:** 2026-11-19T17:54:06Z
- **what:** Added a bounded versioned image-draft journal, owner-filtered Restore control, empty-composer guard, text/update persistence, and admission-receipt retirement. Restore is local-only and never submits to Hermes.
- **why:** Preserve interrupted image composition across restart while matching the pinned explicit no-send recovery behavior.
- **components:** Hermes Native composer, prepared image draft store, 173-row chat/tool progress ledger.
- **type:** feature
- **validation:** Focused test passes for exact connection/profile/session filtering, restore without Gateway, occupied composer no-op, and journal retention; store tests pass for write/update/read/remove and bounds; `cargo fmt` and scoped progress generator pass. Full suite, locked workspace check, task protocol, package validation, and diff checks pending.
- **performance:** Store writes are bounded to 4 MiB per entry, 128 entries, and atomic local-file replacement; no benchmark claim.
- **risks:** Persisted staged Gateway references may expire or be unavailable after restart; live Gateway, packaged Windows and source-paired acceptance remain open.
- **follow_up:** Run full validation, publish to main, then continue with the next dependency-ready chat/tool capability.


### UCM evidence for prepared image draft recovery — 2026-11-19T17:59:00Z

- **id:** hermes-par-0063-ucm-evidence-20261119
- **timestamp:** 2026-11-19T17:59:00Z
- **what:** Added EV-0027 for no-send recovery and bound it to code commit `981609e38e6d08afad19db09cb707d68ce878b46`; marked the UCM capability partial with its headless scenario.
- **why:** Preserve exact-SHA evidence without implying live Gateway or source acceptance.
- **components:** UCM capability and evidence package.
- **type:** evidence
- **validation:** Package validator passes 565 behaviors, inventory rows and pending tests, 14 source receipts and 12 implementation receipts; optional PyYAML parse skipped because PyYAML is absent.
- **performance:** No runtime performance claim.
- **risks:** Live staged-reference validity and source-paired acceptance remain open.
- **follow_up:** Continue the next dependency-ready chat/tool requirement.


### PAR-0064 failed-turn transcript projection — 2026-11-19T18:09:01Z

- **id:** hermes-par-0064-failed-turn-transcript-20261119
- **timestamp:** 2026-11-19T18:09:01Z
- **what:** Projected Gateway error events and terminal failed completion frames into one assistant transcript error row; preserved user/partial output, structured error surface, peer notification, and matching busy-state release.
- **why:** Avoid a blank/forever-busy chat after Hermes initialization/runtime failures and prevent duplicate cards when Gateway event order varies.
- **components:** Hermes Native transcript reducer, active/cached session state, 173-row progress overlay.
- **type:** feature
- **validation:** Focused headless regression passes for both event orders and transcript rendering; full suite, workspace check, task protocol and package checks pending.
- **performance:** Reducer remains in-memory and bounded by existing transcript limits; no benchmark claim.
- **risks:** Retry/edit controls and code-specific error policy are not implemented; live Gateway and source-paired acceptance remain open.
- **follow_up:** Validate the full suite, record exact-SHA UCM evidence, then implement safe retry/edit behavior.


### UCM evidence for failed-turn transcript behavior — 2026-11-19T18:13:00Z

- **id:** hermes-par-0064-ucm-evidence-20261119
- **timestamp:** 2026-11-19T18:13:00Z
- **what:** Added EV-0028 for event-order-independent failed-turn transcript projection and bound it to code commit `98cfb4a3235451ce274f2e9bb8bbdd2a982ec9bd`; marked the UCM capability partial.
- **why:** Keep evidence tied to the tested behavior and avoid claiming Retry/Edit parity.
- **components:** UCM capability and evidence package.
- **type:** evidence
- **validation:** Package validator passes 565 behaviors, inventory rows and pending tests, 14 source receipts and 13 implementation receipts; optional PyYAML parse skipped because PyYAML is absent.
- **performance:** No runtime performance claim.
- **risks:** Retry/Edit actions, full error taxonomy and live source-paired behavior remain open.
- **follow_up:** Implement error-code-aware retry/edit controls.


### PAR-0064 same-ID failed-turn retry — 2026-11-19T18:24:02Z

- **id:** hermes-par-0064-same-id-retry-20261119
- **timestamp:** 2026-11-19T18:24:02Z
- **what:** Added accessible retry action for retryable/generic failed turns; dispatches the original submission ID, composed text and image attachment metadata without creating another optimistic user row. Blocks explicitly non-retryable errors, busy sends and occupied composers.
- **why:** Match Hermes retry idempotency and let users recover from transient failed turns without duplicate user messages.
- **components:** Hermes Native transcript UI, prompt retry transport, Gateway headless command harness and progress ledger.
- **type:** feature
- **validation:** PASS focused headless UI+Gateway command test and guards; PASS full feature suite (397 library + 15 automation protocol + 3 launcher); PASS workspace all-target check; PASS task protocol 18/18; PASS UCM package validator; PASS formatting, Python compilation and diff checks. Existing dead-code warnings remain; optional YAML parse skipped because PyYAML is absent.
- **performance:** No new background work; retry remains asynchronous through the existing Gateway actor.
- **risks:** Code-specific Edit/model/auth/new-session alternatives, complete error taxonomy, live Gateway and source-paired behavior remain open.
- **follow_up:** Bind UCM evidence to the implementation commit, then continue error action parity.


### PAR-0064 retry evidence publication — 2026-11-19T18:30Z

- **id:** hermes-par-0064-retry-evidence-20261119
- **timestamp:** 2026-11-19T18:30:00Z
- **what:** Added UCM EV-0029 bound to `c8d6d4ad57d03574b59d34b3c32d57e359c6c78e`; recorded same-ID retry scenario and retained partial status.
- **why:** Make retry behavior traceable to the exact published implementation while preserving the remaining parity gaps.
- **components:** UCM capability map, evidence package, crosswalk and PAR-0064 plan record.
- **type:** evidence
- **validation:** PASS package validator: 565 target behaviors, 565 UI inventory items, 565 pending tests, 14 source inspection receipts and 14 implementation receipts; fetched `origin/main` matches retry implementation commit. Optional YAML parse skipped because PyYAML is absent.
- **performance:** No runtime performance claim.
- **risks:** Edit/model/auth/new-session alternatives, full error policy and live/source pairing remain open.
- **follow_up:** Continue the next chat and tool capability slice.

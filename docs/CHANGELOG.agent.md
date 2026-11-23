# Agent change journal

## PAR-0256 — Skills hub browse slice — 2026-11-20

- **id:** PAR-0256
- **timestamp:** 2026-11-20T00:39:46Z
- **what:** Added a bounded background loader/parser for the pinned public Skills catalog and a native Skills hub screen with retry/error/loading states, local search, 60-result paging, selectable detail, tags and repository links. Kept the installed Skills screen separate and fixed result selection resetting on each frame.
- **why:** Add real capability browsing against the downloaded Hermes snapshot so the user can move toward functional parity before visual customization.
- **components:** apps/hermes-native/src/skill_catalog.rs, src/app.rs, src/lib.rs, progress generator and implementation ledger, README, architecture, backlog, completion overview, execution log.
- **type:** feature, headless regression tests, progress evidence and documentation.
- **validation:** Full cargo test -p hermes-native --features agent-automation --locked passed (442 library, 15 automation protocol, 3 launcher tests); cargo check --workspace --all-targets --locked, rustfmt, diff check, task validation, both progress generators, and pinned capability-package validator passed. Strict source acceptance stays pending.
- **performance:** Network fetch runs off the UI thread, bounds response size to 128 MiB and rows to 100,000, uses a 60-second timeout, disables redirects, and caches successful data for 30 minutes.
- **risks:** PAR-0256 remains partial. Installed/catalog merging, discovery shelves, facets, sort modes, card/list preference, live feed/Gateway verification, physical Windows interaction, and source-paired review remain open.
- **follow_up:** Continue scoped Skills workflows with PAR-0257 installation and return to missing hub-browser behavior before claiming feature parity.

### PAR-0256 publication record

- Behavior commit bbc4c3770ca75b9125512295ef25fc738b83da92 and partial evidence receipt EV-0052 were pushed to main.
- Fresh fetch confirmed local HEAD equals origin/main at 4159dcea2d3c3aa9a7cdab9c3190be98e1e632b7.
- Strict source acceptance remains pending; this release is functional progress for the user to evaluate before visual customization, not a feature-parity completion claim.

## PAR-0257 — Profile-scoped skill install with live log — 2026-11-20

- **id:** PAR-0257
- **timestamp:** 2026-11-20T01:07:41Z
- **what:** Added pinned install-identifier mapping, a profile and connection-fenced Skills Hub install request, Gateway action polling, and an accessible live log in the selected skill detail. Successful installs refresh the installed Skills inventory; failure text remains visible and retry is available.
- **why:** Skills are in the user's required chat/tool functionality scope; the pinned Desktop installation flow must be usable before visual customization.
- **components:** apps/hermes-native/src/skill_catalog.rs, src/gateway.rs, src/app.rs, implementation progress generator, capability maps, README, architecture, backlog, completion overview, PLAN, test evidence.
- **type:** feature, transport, headless tests, parity evidence and documentation.
- **validation:** Full locked suite passed (445 library + 15 automation protocol + 3 launcher); locked workspace/all-target check, rustfmt, progress generators, task validator and pinned package validator passed. Source acceptance and live Gateway verification remain pending.
- **performance:** Uses bounded Gateway HTTP responses and 1.2-second action status polling; action logs are capped to 200 lines with each display line bounded to 2048 bytes.
- **risks:** Security-scan refusal currently surfaces the returned bounded error/log but does not navigate to the pinned View scan screen; retry after scan refusal still needs source-matched behavior.
- **follow_up:** Validate profile-switch and failure/retry behavior, add View scan handling, then return to missing Browse hub discovery, merging, facets, sort and card/list behavior.

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


### PAR-0065 precise local timestamp details — 2026-11-19T18:38:00Z

- **id:** hermes-par-0065-precise-local-timestamp-20261119
- **timestamp:** 2026-11-19T18:38:00Z
- **what:** Added hover details with full local date, milliseconds and numeric UTC offset; split epoch seconds/fraction before conversion; marked PAR-0065 partial with pinned/native evidence.
- **why:** Match the pinned timestamp control precise date-time affordance and avoid losing a millisecond to floating-point conversion.
- **components:** Hermes Native transcript timestamp rendering, accessibility tooltip, chat/tool progress ledger.
- **type:** feature
- **validation:** PASS focused timestamp formatting test; PASS full feature suite (397 library + 15 automation protocol + 3 launcher); PASS all-target workspace check; PASS task continuity 18/18; PASS UCM package validator; PASS rustfmt and diff check. Existing dead-code warnings remain; optional YAML parse skipped because PyYAML is absent.
- **performance:** Pure formatting on pointer hover; no Gateway traffic or persistent state changes.
- **risks:** Tool-call nested lifecycle mapping and live/source-paired behavior remain unverified.
- **follow_up:** Validate, attach exact-SHA UCM evidence and continue chat/tool parity.


### PAR-0065 timestamp evidence publication — 2026-11-19T18:44Z

- **id:** hermes-par-0065-timestamp-evidence-20261119
- **timestamp:** 2026-11-19T18:44:00Z
- **what:** Added EV-0030 bound to `2db85e2a3b3d853411fac0a57d554a9c14e29e95`, the pinned timestamp hover test scenario and a partial crosswalk note.
- **why:** Tie the tested timestamp detail to the exact published implementation.
- **components:** UCM timestamp capability and evidence package.
- **type:** evidence
- **validation:** PASS package validator (565 target behaviors, inventory rows and pending tests; 14 source and 15 implementation receipts); pushed implementation `2db85e2a3b3d853411fac0a57d554a9c14e29e95` and fresh-fetch verified local/remote equality. Optional YAML parse skipped because PyYAML is absent.
- **performance:** No runtime performance claim.
- **risks:** Nested tool-run lifecycle timestamp and live/source-paired behavior remain open.
- **follow_up:** Validate, publish and continue chat/tool parity.


### PAR-0066/0067 dependency audit — 2026-11-19T18:48:00Z

- **id:** hermes-par-0066-0067-source-audit-20261119
- **timestamp:** 2026-11-19T18:48:00Z
- **what:** Marked terminal-selection-to-draft blocked by the missing native terminal pane and selected Gateway-backed slash completion as the next independent chat item.
- **why:** Keep progress states tied to observed user-facing functionality and explicit prerequisites.
- **components:** Hermes Native terminal panel, composer, task ledger and parity plan.
- **type:** planning
- **validation:** Pinned source checkout hash verified; task protocol state validation passed.
- **performance:** No runtime changes.
- **risks:** Terminal implementation remains a substantial prerequisite for terminal-derived chat context.
- **follow_up:** Implement accessible slash completion using the active session/profile Gateway owner.

## Hermes PAR-0067 slash completion

- **id:** hermes-par-0067-slash-completion-20261119
- **timestamp:** 2026-11-19T19:08:00Z
- **what:** Added debounced, Gateway-backed slash catalog and query requests; owner-fenced results; pinned Desktop registry/interface-mode filtering; accessible suggestions that insert into the current draft without sending; and visible disconnected/unavailable states. Updated the evidence generator to preserve prior PAR-0062..0065 and PAR-0511 partial rows rather than silently dropping them.
- **why:** Provide a useful first native slash-command flow while keeping status conservative and chat/tool progress reproducible.
- **components:** Hermes Native composer, Gateway discovery actor, pinned slash registry, 499-row implementation overlay and 173-row chat/tool view.
- **type:** feature and progress-evidence maintenance
- **validation:** PASS 7 focused slash tests; PASS full feature suite (404 library, 15 protocol, 3 launcher); PASS workspace all-target check; PASS Doofus task continuity (18/18); PASS UCM package validator (565 behaviors/inventory/tests, 14 source and 15 implementation receipts); PASS progress generators (47 reviewed: 6 implemented, 41 partial, 452 unknown; 173 chat/tool: 1 implemented, 14 partial, 158 unknown); PASS rustfmt, Python compilation and diff check.
- **performance:** 60 ms input debounce; no benchmark run.
- **risks:** Arrow navigation, directive chips, mid-message triggers, owner-scoped caching, special local commands, live Gateway behavior, installed Windows interaction and source-paired acceptance remain open.
- **follow_up:** Continue the next dependency-ready pinned chat/tool feature; keep UI styling deferred until scoped feature support is complete.

## UCM receipt for PAR-0067

- **id:** hermes-par-0067-ucm-evidence-20261119
- **timestamp:** 2026-11-19T19:10:00Z
- **what:** Added EV-0031 for slash completion, bound to exact implementation SHA `de719b6f8f6bf9c3cba4ba04e6202df11d5d6e20`; marked the UCM capability and crosswalk partial.
- **why:** Keep the machine-readable capability map aligned with tested implementation while retaining live and source-paired gates.
- **components:** UCM capability map, evidence receipt and crosswalk.
- **type:** evidence
- **validation:** PASS package validator (565 target behaviors, 565 inventory items, 565 pending tests, 14 source and 16 implementation receipts); optional YAML parse skipped because PyYAML is unavailable.
- **performance:** No runtime change.
- **risks:** Evidence is local/headless only; it does not demonstrate authenticated Gateway or installed Windows behavior.
- **follow_up:** Bind future evidence to exact implementation commits and preserve partial status until all acceptance gates pass.

## PAR-0040 fenced-code clipboard verification

- **id:** hermes-par-0040-code-copy-payload-20261119
- **timestamp:** 2026-11-19T19:20:00Z
- **what:** Extended the headless Markdown UI regression to activate the accessible Copy control and verify an exact full fenced-code payload is emitted to the egui clipboard output.
- **why:** The previous fixture proved only that a button existed; this verifies its actual clipboard behavior.
- **components:** Markdown transcript renderer, vendored egui_commonmark copy button, progress evidence generator.
- **type:** test and partial evidence
- **validation:** PASS focused clipboard test; PASS full Hermes Native suite (404 library + 15 automation protocol + 3 launcher); PASS progress generators (48 reviewed: 6 implemented, 42 partial, 451 unknown; chat/tool 1 implemented, 15 partial, 157 unknown).
- **performance:** No runtime behavior change.
- **risks:** Pinned Hermes also unwraps valid `@url:` directives before copying; native still copies that encoded syntax. Live Windows clipboard and source-paired acceptance remain open. PAR-0068 directive-hover actions remain deferred because the native composer lacks a rich chip model.
- **follow_up:** Add URL directive normalization or keep PAR-0040 partial; choose next dependency-ready chat/tool behavior.

## UCM receipt for PAR-0040

- **id:** hermes-par-0040-ucm-evidence-20261119
- **timestamp:** 2026-11-19T19:22:00Z
- **what:** Added EV-0032 bound to `bc3555f62308ced1c87924ffe0f53ac830441cc7`; marked the code-copy capability and crosswalk partial.
- **why:** Record exact tested behavior without claiming full clipboard parity.
- **components:** UCM capability map, evidence receipt and crosswalk.
- **type:** evidence
- **validation:** PASS package validator (565 behaviors, 565 inventory items, 565 pending tests, 14 source and 17 implementation receipts); optional YAML parse skipped because PyYAML is unavailable.
- **performance:** No runtime change.
- **risks:** URL directive normalization and physical/source-paired clipboard acceptance remain unverified.
- **follow_up:** Normalize valid URL directives and test them, then retain live and source-paired acceptance gates.

## PAR-0058 transcript-find evidence

- **id:** hermes-par-0058-find-in-page-evidence-20261119
- **timestamp:** 2026-11-19T19:25:00Z
- **what:** Added a partial progress seed and EV-0033 for the existing native transcript-find slice; updated the capability map, crosswalk, plan, architecture, README and backlog with the loaded-row/whole-card limit.
- **why:** Replace an unknown status with evidence-backed partial coverage while preserving unimplemented source and live acceptance requirements.
- **components:** Native find bar, transcript search/highlight, 499-row progress overlay, UCM evidence.
- **type:** evidence mapping and documentation
- **validation:** PASS progress generators (49 reviewed: 6 implemented, 43 partial, 450 unknown; chat/tool 1 implemented, 16 partial, 156 unknown); PASS UCM validator (565 items, 14 source and 18 implementation receipts); PASS Doofus task state validation; referenced native test passed in the 404+15+3 feature suite.
- **performance:** No runtime change.
- **risks:** Current-view substring highlighting across transcript Markdown and physical/source-paired behavior remain open; PyYAML schema parse was skipped because it is unavailable.
- **follow_up:** Continue partial parity work and keep source acceptance pending until rendered-surface behavior is covered.

## PAR-0057 older transcript pagination evidence

- **id:** hermes-par-0057-pagination-evidence-20261119
- **timestamp:** 2026-11-19T19:39:24Z
- **what:** Mapped the existing native offset-page pagination to partial evidence and recorded pinned around-row navigation gaps.
- **why:** Distinguish working older-page retrieval from the different pinned selected-prompt history-window behavior.
- **components:** Transcript UI, Gateway page decoding, progress generators and UCM crosswalk.
- **type:** evidence mapping and documentation
- **validation:** PASS focused older-transcript filter (5 tests); progress generators and UCM package validation pending.
- **performance:** No runtime change.
- **risks:** Live Gateway, stale UI request flow, Windows interaction and fixed-source acceptance remain open.
- **follow_up:** Implement around-row predecessor navigation and contiguity handling; then validate live and source-paired behavior.


Validation addendum (2026-11-19T19:42Z): PASS focused pagination filter (5 tests); PASS progress generators (50 reviewed; 6 implemented, 44 partial, 449 unknown; chat/tool 18 mapped with 1 implemented, 17 partial, 155 unknown); PASS UCM validator (565 behaviors/inventory/pending tests, 14 source and 18 implementation receipts); PASS task validation and `git diff --check`. The first package validation caught malformed CSV column alignment from an unquoted comma; the PAR-0057 row was reconstructed through the schema-aware CSV writer, and the rerun passed. Optional YAML validation skipped because PyYAML is unavailable. EV-0034 will be bound to the implementation/documentation commit after commit creation.

UCM evidence addendum (2026-11-19T19:44Z): EV-0034 records the five focused pagination tests and is bound to implementation/documentation commit `b3d3e7189df7fd76eac058bf27ca0f2453f4d61c`; capability and crosswalk remain partial.


## PAR-0041 inline tool card evidence

- **id:** hermes-par-0041-tool-row-evidence-20261119
- **timestamp:** 2026-11-19T19:47:00Z
- **what:** Mapped existing generation-scoped accessible tool cards as partial and documented pinned run-group behavior gaps.
- **why:** Capture working tool event rendering without overstating run grouping parity.
- **components:** Gateway event reducer, accessible transcript tool cards, progress overlay, UCM crosswalk.
- **type:** evidence mapping and documentation
- **validation:** PASS focused generation-scoped tool-card UI test; PASS progress generators, UCM package validator and task-state validator.
- **performance:** No runtime change.
- **risks:** Live, Windows and source-paired behavior are unverified; run summary/ticker/disclosure remain absent.
- **follow_up:** Implement pinned run grouping and live disclosure, then validate against live Gateway and fixed-source UI behavior.

UCM evidence addendum (2026-11-19T19:49Z): EV-0035 records the focused generation-scoped tool-card test and is bound to commit `077677f0d08c07cf7a590fc104986e7c3182611d`; capability and crosswalk remain partial.

Correction (2026-11-19T19:50Z): EV-0035 implementation commit is `077677f1c8e184b1a3b614024626b0ed41061692`; the initially typed abbreviated hash expanded incorrectly and was corrected before receipt commit.


## PAR-0251 installed Skills evidence

- **id:** hermes-par-0251-installed-skills-evidence-20261119
- **timestamp:** 2026-11-19T19:51:00Z
- **what:** Extracted the native installed Skills filter helper, added coverage for name/category/description and blank-query listing behavior, and mapped PAR-0251 partial.
- **why:** Verify an existing skills surface without conflating listing/search with skill lifecycle features.
- **components:** Capabilities view, Gateway skill model, generated status and UCM crosswalk.
- **type:** behavior test and evidence mapping
- **validation:** Focused test passes; progress generators and UCM/task validators are run before publication.
- **performance:** No material runtime change; filtering remains a small linear scan.
- **risks:** No dedicated rendered list test, usage ordering, live Gateway, physical Windows or source-paired acceptance yet.
- **follow_up:** Add rendered Skills list/owner-switch acceptance and continue separate skill lifecycle rows.


Validation addendum (2026-11-19T19:53Z): PASS focused `installed_skills_search_matches_name_category_and_description` test; PASS cargo fmt check; PASS progress generators (52 reviewed: 6 implemented, 46 partial, 447 unknown; chat/tool 20 mapped: 1 implemented, 19 partial, 153 unknown); PASS UCM validator (565 inventory/behavior/test entries, 14 source and 20 implementation receipts); PASS task validation and `git diff --check`. The first atomic documentation pass stopped because the YAML row label used a different `area` than the expected patch; no later documentation was written in that attempt. The exact row was then updated and all validators passed. Optional YAML parsing skipped because PyYAML is unavailable.

UCM evidence addendum (2026-11-19T19:55Z): EV-0036 records installed Skills search field coverage and is bound to commit `27a76e5e671139ce10fa635f34a7c93107adcf81`; capability remains partial.


## PAR-0252 installed Skills search evidence

- **id:** hermes-par-0252-installed-skills-search-evidence-20261119
- **timestamp:** 2026-11-19T19:56:00Z
- **what:** Mapped the native tested Skills query across name, category, and description as partial against the pinned search filter.
- **why:** Track search independently from list loading and skill lifecycle controls.
- **components:** Skills filter helper, native unit test, implementation overlay and UCM crosswalk.
- **type:** evidence mapping and documentation
- **validation:** PASS focused Skills search test; progress and UCM validation run before publication.
- **performance:** No material runtime change.
- **risks:** Usage sort order, rendered interaction, profile switching, live Gateway and source-paired acceptance remain open.
- **follow_up:** Match pinned usage ordering and add UI interaction/source-paired coverage.

UCM evidence addendum (2026-11-19T19:58Z): EV-0037 records the Skills search field test and is bound to code/docs commit `a2d1608a36a9efc834cc24930671e928f6aa1172`; capability remains partial.


## PAR-0252 Skills result ordering

- **id:** hermes-par-0252-skills-ordering-20261119
- **timestamp:** 2026-11-19T19:59:00Z
- **what:** Sort native matching skills by usage descending then lowercase name; extend the focused test to verify both priority and alphabetical tie order.
- **why:** Close the known ordering mismatch against pinned Desktop search results.
- **components:** Installed Skills filter, search result ordering and regression test.
- **type:** behavior correction
- **validation:** PASS focused Skills filter test and formatting; package/task validators run before publication.
- **performance:** Sorting adds O(n log n) work after the existing O(n) filter.
- **risks:** Lowercase ordinal tie-breaking can differ from locale-aware Desktop ordering for non-English names.
- **follow_up:** Add rendered search interaction and locale-aware ordering acceptance.

UCM evidence addendum (2026-11-19T20:00Z): EV-0038 records Skills search filtering and usage/name ordering, bound to behavior commit `073befedd710bfd619d9007e615f1181ad5f1460`; capability remains partial.


## PAR-0254 per-skill enable toggle

- **id:** hermes-par-0254-skill-toggle-20261119
- **timestamp:** 2026-11-19T20:07:00Z
- **what:** Added the accessible native per-skill toggle, pinned Gateway PUT payload, optimistic update, pending guard, owner-fenced receipt and denial rollback.
- **why:** Implement the missing individual enable/disable behavior from the fixed Hermes source.
- **components:** Skills capabilities list, Gateway HTTP actor, mutation reducer and automation action schema.
- **type:** behavior implementation
- **validation:** Focused Gateway HTTP contract and app mutation tests pass; progress and UCM validators run before publication.
- **performance:** One bounded 5-second HTTP mutation; no polling or repeated reads.
- **risks:** Bulk controls, profile-switch interaction, live runtime, Windows rendering and source-paired acceptance remain open.
- **follow_up:** Validate the rendered checkbox on Windows/headless accessibility tree and implement remaining skill management controls.

UCM evidence addendum (2026-11-19T20:17Z): EV-0039 records the PAR-0254 per-skill enable/disable Gateway mutation and focused tests, bound to behavior commit 0d6207564b8b5a753c5b5b2b519d1c5c05a99539. The capability remains partial pending live and rendered UI verification.
### PAR-0262 — Profile-scoped MCP server inventory (2026-11-19T20:30Z)

- **id:** hermes-par0262-mcp-inventory-20261119
- **timestamp:** 2026-11-19T20:30:52Z
- **what:** Added typed MCP server inventory loading through the profile-bound Gateway HTTP worker and visible server rows in the Connectors capability view.
- **why:** Start porting pinned Hermes MCP server management with a bounded, read-only inventory slice.
- **components:** Gateway HTTP commands and events, local/remote authenticated reads, Connectors UI/search, headless automation fixture, progress/UCM evidence projection.
- **type:** feature implementation
- **validation:** PASS cargo check, cargo fmt --check, focused ticketed profile GET test, focused AccessKit row test, progress generators, UCM package validator and task validator.
- **performance:** Single bounded HTTP read on Connectors entry/refresh; existing worker and response limits apply.
- **risks:** Add/edit/test/toggle/remove/catalog install are not implemented; live Gateway, remote auth and packaged Windows UI remain unverified. PyYAML is unavailable, so package validator skipped optional YAML parse.
- **follow_up:** Implement MCP server enable/test mutations and catalog flows as separate reviewable slices.
UCM evidence addendum (2026-11-19T20:33Z): EV-0040 binds the PAR-0262 MCP server inventory and focused HTTP/UI tests to behavior commit f0836b5ae9d21b33d1b5453a0c9b58b6526b6a8c. The capability remains partial pending mutation flows and live/source-paired acceptance.
EV-0040 receipt write recovery (2026-11-19T20:34Z): the first atomic-write attempt failed before creating the file because its destination did not yet exist for Resolve-Path. The receipt was then written to a temp file in the verified evidence folder and atomically moved into place; evidence validator confirms it.

### PAR-0265 — Profile-scoped MCP server toggle (2026-11-19T20:40Z)

- **id:** hermes-par0265-mcp-toggle-20261119
- **timestamp:** 2026-11-19T20:40:10Z
- **what:** Added accessible per-server enable/disable with the pinned PUT route, optimistic state, bounded receipt handling and denial rollback.
- **why:** Complete a basic MCP management action in the chat/tool integration scope.
- **components:** Gateway HTTP worker, typed MCP mutation result, registered UI action, Connectors server rows, task/progress evidence.
- **type:** feature implementation
- **validation:** PASS focused Gateway HTTP route/payload/receipt test, app optimistic-denial-retry test, cargo check and formatting.
- **performance:** One bounded HTTP mutation per user action; existing worker FIFO and response cap apply.
- **risks:** Pinned response only confirms ok; live Gateway state reconciliation and remote authenticated execution remain unverified.
- **follow_up:** Implement MCP test/add/catalog operations and verify real Gateway behavior.
UCM evidence addendum (2026-11-19T20:42Z): EV-0041 binds the PAR-0265 MCP toggle and focused HTTP/app tests to behavior commit caa45df42f235fc26ecd934e550a611d855c2490. The capability remains partial pending live and source-paired acceptance.

### PAR-0264 — Profile-scoped MCP connection test (2026-11-19T20:53Z)

- **id:** hermes-par0264-mcp-test-20261119
- **timestamp:** 2026-11-19T20:53:54Z
- **what:** Added manual MCP connection test with bounded Gateway requests, typed tool metadata, visible result/error state and stale-profile fencing.
- **why:** Move chat/tool integration management toward usable feature coverage so visual iteration can begin sooner.
- **components:** Gateway HTTP worker, MCP test model, Connectors UI, registered action, scoped progress and UCM maps.
- **type:** feature implementation
- **validation:** PASS focused MCP HTTP request/result contract test, app result and stale-profile test, cargo check, rustfmt and diff check.
- **performance:** One bounded background request per server test, maximum four concurrent tests, 60-second request timeout and 512 KiB response cap.
- **risks:** Automatic probe/cache behavior, live/remote Gateway behavior, packaged Windows and source-paired acceptance remain open.
- **follow_up:** Continue through MCP add/catalog lifecycle and scoped chat/tool features; keep visual redesign as a separate later stage.
UCM evidence addendum (2026-11-19T20:53Z): EV-0042 binds the PAR-0264 manual MCP test and its focused HTTP/app tests to behavior commit c44a520f3f936ab444ef189ca7a3d9aec4c044ea. The capability remains partial pending automatic probe/cache, live Gateway and source-paired acceptance.

### PAR-0263 — Add profile-scoped MCP servers (2026-11-19T21:16Z)

- **id:** hermes-par0263-mcp-add-20261119
- **timestamp:** 2026-11-19T21:16:26Z
- **what:** Added accessible stdio and HTTP server setup with args, environment, working directory, headers, OAuth and bearer-token authentication; results update the MCP inventory only for the owning profile/connection epoch.
- **why:** Provide the next core MCP integration feature so the native UI can configure servers instead of only listing, toggling and testing them.
- **components:** Registered agent action, Connectors form, Gateway JSON-RPC actor, MCP response validation, progress and UCM maps.
- **type:** feature implementation
- **validation:** PASS four focused MCP add tests: exact profile-scoped `mcp.servers.add` RPC and secret field, app success/stale fencing, stdio/HTTP payload construction and input validation, and accessible rendered controls for both transports. `cargo check` and `git diff --check` also pass.
- **performance:** One foreground bounded RPC per add; app prevents duplicate in-flight adds and server names already in the loaded inventory.
- **risks:** Live secret-store behavior, OAuth handoff, server startup and installed Windows behavior remain unverified. Bearer token is masked in the form and sent separately from MCP config.
- **follow_up:** Implement pinned MCP import/paste and passthrough environment rows; continue remove/catalog operations and verify against a running Gateway.
UCM evidence addendum (2026-11-19T21:18Z): EV-0043 binds the PAR-0263 manual MCP add, safe bearer secret transport and focused RPC/app/accessibility tests to behavior commit e72ab8590eeaf95b21ea537ba514d9622d9e2fbf. The capability remains partial pending import/passthrough, live secret/OAuth/runtime and source-paired acceptance.

### PAR-0266 — Confirmed MCP server removal (2026-11-19T21:20Z)

- **id:** hermes-par0266-mcp-remove-20261119
- **timestamp:** 2026-11-19T21:20:00Z
- **what:** Added explicit removal confirmation, profile/connection/epoch-bound `mcp.servers.remove` RPC, strict `{ok, removed}` receipt handling, and inventory cleanup on confirmed success.
- **why:** Complete another core MCP lifecycle action needed to manage Hermes tool integrations.
- **components:** Registered agent actions, Connectors server rows, confirmation dialog, Gateway actor/event, MCP inventory state and UCM progress.
- **type:** feature implementation
- **validation:** PASS Gateway RPC/profile/receipt test, app confirmation and stale-profile/result-fencing test with headless confirmation controls, cargo check and formatting.
- **performance:** One foreground RPC after user confirmation; duplicate pending removals are blocked.
- **risks:** Live config/credential cleanup, worker shutdown/restart and packaged Windows behavior remain unverified.
- **follow_up:** Continue MCP catalog/install and other scoped chat/tool gaps, then verify live Gateway behavior.

### PAR-0266 evidence — MCP server removal (2026-11-19T21:30Z)

- **id:** hermes-par0266-ev0044-20261119
- **timestamp:** 2026-11-19T21:30:00Z
- **what:** Added EV-0044 for confirmation-gated MCP removal at commit `2c30e5956bc0ad439fe42f439eff7f83219bf0a4`; task ledger records the slice as tested.
- **why:** Preserve traceable test evidence while accurately retaining partial capability status.
- **components:** UCM evidence, Doofus task ledger and handoff, append-only audit docs.
- **type:** evidence and task-state update
- **validation:** Task ledger revision 309 valid; focused tests and cargo check recorded in EV-0044; package validator run before publication.
- **performance:** No runtime behavior change.
- **risks:** Live config/credential cleanup, MCP worker shutdown/restart and packaged UI remain unverified.
- **follow_up:** Continue pinned-source chat/tool feature coverage, prioritizing end-to-end user value.

### PAR-0267 — MCP catalog browse contract selected (2026-11-19T21:35:19Z)

- **id:** hermes-par0267-contract-audit-20261119
- **timestamp:** 2026-11-19T21:35:19Z
- **what:** Selected the next accepted chat/tool parity slice and checkpointed the pinned `mcp.catalog` read contract: profile-scoped catalog rows expose status, description, required environment key names and transport.
- **why:** Continue connector feature coverage so the user can reach visual customization sooner.
- **components:** Doofus task ledger, Hermes native Gateway and Connectors view plan.
- **type:** task selection and source-contract audit
- **validation:** Task state schema validated at revision 313; pinned source and Desktop usage inspected; a second read-only agent audit matched the contract. No implementation tests run for this slice yet.
- **performance:** No runtime changes.
- **risks:** Native catalog UI, profile fencing, failure states and live Gateway behavior remain unimplemented/unverified. Catalog installation is a separate task.
- **follow_up:** Implement typed `mcp.catalog` browsing and focused tests.

### PAR-0267 — Profile-scoped MCP catalog browsing (2026-11-19T21:55Z)

- **id:** hermes-par0267-catalog-browse-20261119
- **timestamp:** 2026-11-19T21:55:00Z
- **what:** Added a profile-scoped `mcp.catalog` Gateway request, bounded typed response validation, request/profile/instance/authority-epoch fencing, five-minute in-memory freshness, shared search, and installed/available rows with descriptions and required environment-key names.
- **why:** Give people and the coding agent the same read-only MCP catalog visibility while advancing chat/tool/skills/plugin feature coverage.
- **components:** Gateway actor and DTO, Connectors state/rendering, UCM capability projections, scoped progress generation and docs.
- **type:** feature implementation
- **validation:** PASS 3 focused catalog tests; PASS full Hermes Native `agent-automation` feature suite (420 library tests plus protocol and launcher targets); PASS locked cargo check, rustfmt, progress generators, UCM package validator, Doofus task validator and diff check. Progress is 60/499 reviewed (6 implemented, 54 partial, 439 unknown); scoped progress is 28/173 mapped (1 implemented, 27 partial, 145 unknown).
- **performance:** Catalog fetch is one Gateway RPC; local cache freshness is 5 minutes; entry count, text fields and environment-key lists are bounded.
- **risks:** Catalog install, live/remote Gateway behavior, installed Windows behavior and source-paired acceptance remain open.
- **follow_up:** Implement approved catalog installation as a separate slice; keep feature work ahead of visual restyling.

### PAR-0267 validation correction — automation action registry (2026-11-19T22:03Z)

- **id:** hermes-par0267-registry-test-fix-20261119
- **timestamp:** 2026-11-19T22:03:00Z
- **what:** Replaced a brittle exact global widget/action count assertion with a minimum legacy-action count; the test continues to assert every required existing action type.
- **why:** Adding accessible catalog controls legitimately grows the UI action inventory; registry growth must not break a behavior test unrelated to exact total size.
- **components:** Hermes automation protocol integration test and native MCP catalog widgets.
- **type:** regression-test robustness
- **validation:** Full locked `agent-automation` suite passes: 421 library, 15 protocol and 3 launcher tests; locked cargo check, formatting, package/task validators and diff check pass.
- **performance:** No runtime behavior change.
- **risks:** Additional widgets remain visible actions by design; focused and full suites now cover the expanded registry.
- **follow_up:** Keep automation tests scoped to action identities and behavior as the UI grows.

### PAR-0267 evidence — MCP catalog browsing (2026-11-19T21:59Z)

- **id:** hermes-par0267-ev0045-20261119
- **timestamp:** 2026-11-19T21:59:00Z
- **what:** Added EV-0045 bound to behavior commit `a95595723e28cc4999c256f2da0e54b0b6d82487` and marked task PAR-0267 tested with partial feature evidence.
- **why:** Bind the reviewed Gateway, fencing, cache and headless UI evidence to the exact published revision.
- **components:** UCM receipt and portable Doofus task ledger.
- **type:** evidence and task-state update
- **validation:** Receipt JSON parsed; package validator passed with catalog capability still partial; task ledger revision 48 valid.
- **performance:** No runtime change.
- **risks:** Live Gateway, remote profile, installed Windows behavior and source-paired acceptance remain unverified.
- **follow_up:** Continue with separate MCP catalog installation and remaining chat/tool/skills/plugins gaps.

### PAR-0268 — MCP catalog install and Desktop REST contract (2026-11-19T22:28Z)

- **id:** hermes-par0268-mcp-catalog-install-20261119
- **timestamp:** 2026-11-19T22:28:32Z
- **what:** Switched MCP catalog loading from the bot-oriented `mcp.catalog` RPC to the pinned Desktop `GET /api/mcp/catalog` response, preserving required prompts, install/bootstrap metadata, default tools, post-install text and diagnostics. Added a Connectors install form with masked environment fields, required-value validation, automation registry actions, exact profile-scoped `POST /api/mcp/catalog/install` payload, synchronous refresh, and background action polling through the pinned status endpoint.
- **why:** The Desktop install workflow needs the richer REST catalog contract and waits for bootstrap actions before reporting completion; the previous RPC omitted required flags and user-facing prompts.
- **components:** Native MCP catalog DTO and parser, HTTP transport, Connectors form, registered human/agent actions, Gateway install/status fencing, progress and parity documentation.
- **type:** feature parity implementation
- **validation:** PASS six focused MCP catalog tests; PASS full locked feature suite (424 library, 15 automation protocol, 3 launcher tests, and doc tests). Focused coverage includes the exact REST profile query and payload, background receipt, credential error non-echo, masked accessible field, required-value rejection, synchronous completion, and successful background status. Workspace check, format, task/UCM/progress validators and diff check are recorded after this entry.
- **performance:** No benchmark was needed; synchronous HTTP uses the pinned 60-second bound and background bootstrap status is checked every 1.5 seconds for at most ten minutes.
- **risks:** No real local/remote Gateway or physical Windows run was performed. Profile secret persistence, background reconnect reconciliation and formal pinned-source acceptance remain unverified; both PAR-0267 and PAR-0268 remain partial.
- **follow_up:** Run final validators, append evidence receipts, publish to `main`, then continue the remaining chat/tool/skills/plugin gaps.

### PAR-0268 disconnect safety and progress validation correction (2026-11-19T22:38Z)

- **id:** hermes-par0268-disconnect-safety-20261119
- **timestamp:** 2026-11-19T22:38:36Z
- **what:** Clear credential drafts on Gateway disconnect, mark an in-flight synchronous catalog install as outcome-unknown, resume background polling only for the same Gateway owner, and stop polling after the source deadline. Correct the PAR-0267 progress locator to the pinned REST API and current Gateway test.
- **why:** Prevent stale install state, credential retention, misleading retry affordances and indefinite background polling; keep generated progress tied to the actual pinned contract.
- **components:** MCP catalog install lifecycle, Gateway event handling, implementation-progress generator, README, architecture, backlog and parity plan.
- **type:** reliability and evidence correction
- **validation:** PASS seven focused catalog tests; PASS full locked suite (425 library, 15 protocol, 3 launcher tests, doc tests); PASS locked workspace/all-target check with pre-existing dead-code warnings only; PASS `cargo fmt --check`, UCM package validator, Doofus task validator, both progress generators and `git diff --check`.
- **performance:** No runtime benchmark change; background status cadence and ten-minute bound follow the pinned Desktop behavior.
- **risks:** No real local/remote Gateway, credential persistence, installed Windows UI, or source-paired acceptance was exercised. PAR-0267 and PAR-0268 remain partial.
- **follow_up:** Add exact UCM receipts, validate evidence references, and publish the tested changes to `main`.

### PAR-0268 UCM evidence receipts (2026-11-19T22:44Z)

- **id:** hermes-par0268-ev0046-ev0047-20261119
- **timestamp:** 2026-11-19T22:44:53Z
- **what:** Add EV-0046 for the partial MCP catalog install flow and EV-0047 superseding the old RPC browse evidence with the pinned Desktop REST contract. Update UCM capability dimensions and status, crosswalk, UI inventory, full capability map and generated progress.
- **why:** Keep the machine-readable implementation status aligned with validated behavior and preserve the historical EV-0045 receipt without claiming live acceptance.
- **components:** UCM capability package, implementation progress generator and chat/tool progress ledger.
- **type:** evidence and capability-status update
- **validation:** PASS package validator (565 behaviors, 565 UI inventory rows, 32 implementation receipts); both EV JSON files and full capability map parse; implementation progress 61/499 reviewed; chat/tool progress 29/173 with mapped evidence; task schema validation passes. Optional PyYAML parse skipped because PyYAML is absent.
- **performance:** No runtime behavior change.
- **risks:** Evidence is local/headless only. Live local/remote Gateway, credential persistence, installed Windows interaction and source-paired acceptance remain open; both capabilities remain partial.
- **follow_up:** Mark PAR-0268 tested with EV-0046, publish the evidence revision to `main`, verify remote SHA, then continue the next functional parity slice.

### PAR-0268 main publication (2026-11-19T22:47Z)

- **id:** hermes-par0268-main-publish-20261119
- **timestamp:** 2026-11-19T22:47:38Z
- **what:** Published validated implementation and evidence revisions directly to `main`; fresh fetch confirmed local HEAD equals `origin/main` at `2c8e5150e54ce1b027a0e75ad0fede99e0fd1ae2`.
- **why:** Keep the user's main-only repository current with tested functional progress and its partial evidence.
- **components:** MCP catalog install, UCM receipts EV-0046/EV-0047, implementation progress, task ledger.
- **type:** main publication
- **validation:** PASS full Rust feature suite 425 library + 15 automation protocol + 3 launcher tests and doc tests; PASS workspace/all-target check, formatting, task/package/progress validators, JSON parsing, and diff checks; PASS exact remote SHA equality.
- **performance:** No benchmark change.
- **risks:** The capability is partial and remains unverified against a live Hermes Gateway or installed Windows UI; overall source acceptance remains 0/499.
- **follow_up:** Select the next high-value chat/tool/skills/plugin behavior gap; keep functional parity ahead of the user's planned visual restyling.

- **id:** hermes-par0043-tool-progress
- **timestamp:** 2026-11-19T23:05:00Z
- **what:** Projected Gateway `tool.progress` events onto matching running tool rows with accessible percentage/status, profile/session-scoped routing, MCP batch fanout, and terminal cleanup.
- **why:** Let the operator and coding agent see what a running tool is doing during a chat turn.
- **components:** Hermes native transcript reducer/rendering, profile-scoped background chat handling, implementation progress report.
- **type:** functional parity
- **validation:** PASS full locked feature suite (428 library + 15 automation protocol + 3 launcher tests + doc tests), workspace/all-target check, rustfmt, both progress generators, task validator, and diff checks. Existing dead-code warnings only.
- **performance:** No benchmark change; event updates mutate existing rows and add no polling.
- **risks:** Headless evidence only; live Gateway notifications, interrupted-turn source parity, installed Windows rendering, and source-paired acceptance remain open.
- **follow_up:** Bind EV-0048 to the behavior commit, mark PAR-0043 tested partial, and publish directly to main.

- **id:** hermes-par0043-ev0048
- **timestamp:** 2026-11-19T23:06:00Z
- **what:** Added EV-0048 for the tested partial live tool progress capability and updated all UCM capability projections to partial.
- **why:** Tie the implementation claim to reproducible evidence while preserving the live-runtime and source-paired acceptance gaps.
- **components:** UCM capability map, crosswalk, UI inventory, full map, evidence receipt, Doofus task ledger.
- **type:** evidence and progress tracking
- **validation:** PASS UCM package validator (565 target behaviors, 565 inventory rows, 14 source receipts, 33 implementation receipts); JSON receipt parse; task ledger validation at revision 330.
- **performance:** No benchmark change.
- **risks:** PyYAML is not installed, so optional YAML parsing was skipped; package validator passed. Live Gateway and installed Windows behavior remain unverified.
- **follow_up:** Publish both commits on main and verify origin/main equals local HEAD.

- **id:** hermes-par0043-main-publish
- **timestamp:** 2026-11-19T23:08:00Z
- **what:** Published PAR-0043 behavior commit `ed4281c07e9a16098b548578467ba9acfb387eae` and EV-0048 evidence commit `10dc5abe5ffa8ed3f8e9f4a3105df1c892c6fa34` directly to `main`; fresh fetch confirmed exact local/remote SHA equality.
- **why:** Keep the user's main-only repository current and make the tested partial slice reviewable.
- **components:** Hermes native tool-progress behavior, UCM receipt and maps, task ledger and progress report.
- **type:** main publication
- **validation:** PASS exact `HEAD == origin/main` at `10dc5abe5ffa8ed3f8e9f4a3105df1c892c6fa34`; full locked suite, workspace check, formatting, task/package/progress validators passed.
- **performance:** No benchmark change.
- **risks:** PAR-0043 remains partial pending live Gateway, installed Windows rendering, interrupted-turn parity and source-paired acceptance.
- **follow_up:** Continue with pinned PAR-0042 tool argument/output disclosure behavior.

- **id:** hermes-par0042-tool-payload-disclosure
- **timestamp:** 2026-11-19T23:28:00Z
- **what:** Added an accessible, collapsed-by-default Tool payload disclosure to tool transcript rows in Advanced mode, showing pretty-printed Arguments and Result with a 20,000-character display bound. Simple mode continues to show the normal tool row without raw payload.
- **why:** Give users and UI agents a way to inspect exact tool inputs and outputs while keeping ordinary chat readable.
- **components:** Native transcript renderer and tool payload formatter, Advanced/Simple mode behavior, implementation progress and current completion overview.
- **type:** functional parity
- **validation:** PASS focused disclosure/formatter tests; full locked feature suite (431 library + 15 automation protocol + 3 launcher tests + doc tests); workspace/all-target check, rustfmt, implementation/chat-tool progress generators, Doofus task validator, UCM package validator (565 target behaviors, 565 UI items, 33 implementation receipts), and diff check.
- **performance:** Payload serialization is deferred until the disclosure opens; displayed payload is capped at 20,000 characters and nested view height at 180 points.
- **risks:** Native Advanced mode currently gates payload visibility; the pinned source has a separate persisted product/technical tool-view preference. Live Gateway, installed Windows, and source-paired acceptance remain unverified.
- **follow_up:** Bind EV-0049 to the implementation commit, mark PAR-0042 tested partial, and publish directly to main.

- **id:** hermes-par0042-ev0049
- **timestamp:** 2026-11-19T23:30:00Z
- **what:** Added EV-0049 for the tested partial Arguments/Result disclosure and recorded PAR-0042 as tested in the task ledger.
- **why:** Bind the feature claim to the exact implementation revision while preserving the separate tool-view setting and runtime/Windows acceptance gaps.
- **components:** UCM capability map, crosswalk, UI inventory, full map, evidence receipt, Doofus task ledger.
- **type:** evidence and progress tracking
- **validation:** PASS UCM package validator (565 target behaviors, 565 inventory rows, 14 source inspection receipts, 34 implementation receipts); JSON receipt parse; task ledger validation at revision 337.
- **performance:** No benchmark change.
- **risks:** PyYAML is not installed, so optional YAML parsing was skipped; package validator passes. Live Gateway and installed Windows behavior remain unverified.
- **follow_up:** Publish behavior and evidence commits on main and verify exact remote SHA equality.

- **id:** hermes-par0042-main-publish
- **timestamp:** 2026-11-19T23:32:00Z
- **what:** Published PAR-0042 behavior commit `fc53c4dafb7d59fd157ce17b5447f5b0c709edca` and EV-0049 evidence commit `a677b049510bc81077fa6f7c472a6048749c6e65` directly to `main`; fresh fetch confirmed exact local/remote SHA equality.
- **why:** Keep the main-only repository current and make the tested partial tool payload disclosure reviewable.
- **components:** Hermes Native transcript tool rows, UCM receipt and projections, task ledger and implementation progress.
- **type:** main publication
- **validation:** PASS exact `HEAD == origin/main` at `a677b049510bc81077fa6f7c472a6048749c6e65`; full locked suite, workspace check, formatting, task/progress/UCM validators passed.
- **performance:** Payload serialization is deferred until disclosure; 20,000-character rendering cap.
- **risks:** Separate persisted tool-view preference, live Gateway, installed Windows behavior, and source-paired acceptance remain open.
- **follow_up:** Continue with PAR-0044 generated tool-image rendering and enlarge behavior.

- **id:** hermes-par0044-tool-image-preview-and-enlarge
- **timestamp:** 2026-11-19T23:49:00Z
- **what:** PAR-0044 now recognizes tool image candidates and renders local/authenticated-remote previews with an accessible resizable enlarge window. Progress overlays and UCM capability projections now record the feature as partial.
- **why:** Let people inspect images produced or referenced by Hermes tools directly in chat.
- **components:** Hermes Native transcript renderer, file preview store, authenticated Gateway image preview, implementation progress, UCM package, completion overview, architecture and backlog docs.
- **type:** native feature implementation and partial progress evidence.
- **validation:** PASS two focused `tool_image_*` tests; PASS `cargo check -p hermes-native --locked --features agent-automation`; implementation and chat/tool progress generation pass. Full suite, task/package validators and publication remain pending.
- **performance:** Remote previews retain their existing 512×384 decode bounds; local image decoding remains on the existing preview worker and limits.
- **risks:** Data URLs, full-resolution remote enlargement, zoom/pan and retry remain unsupported. Live Gateway, installed Windows and source-paired acceptance remain open.
- **follow_up:** Run full validation, bind EV-0050 to the behavior commit, publish to main, then continue with the next dependency-ready feature.

- **id:** hermes-par0044-validation-addendum
- **timestamp:** 2026-11-19T23:53:00Z
- **what:** Completed the full PAR-0044 validation after the initial implementation record: all 433 library, 15 automation-protocol, and 3 launcher tests passed; workspace check, formatting, progress generation, task validation, package validation, and diff checks passed.
- **why:** Close the implementation validation step and record the corrected package-validator location.
- **components:** Hermes Native test/build suite, Doofus task ledger, UCM package validator.
- **type:** validation addendum.
- **validation:** PASS `cargo test -p hermes-native --features agent-automation --locked`; PASS `cargo check --workspace --all-targets --locked` (existing dead-code warnings only); PASS `cargo fmt --all -- --check`; PASS both progress generators; PASS `python docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/tools/validate_package.py`; PASS task validator at revision 343 and `git diff --check`.
- **performance:** No benchmark change; local/remote preview limits remain unchanged.
- **risks:** Live Gateway, installed Windows, remote full-resolution, data URL, zoom/pan, and source-paired behavior remain unverified or incomplete.
- **follow_up:** Create EV-0050 for the behavior commit, publish on main and verify exact remote SHA equality.

- **id:** hermes-par0044-ev0050-evidence
- **timestamp:** 2026-11-19T23:55:00Z
- **what:** Added EV-0050 for the tested partial image preview/enlarge behavior at commit `f69925d2bd47c01f7f9a0aa9f1135576600e2cfc`, updated the UCM implementation receipt set, and checkpointed PAR-0044 as tested.
- **why:** Keep the implementation claim bound to its exact code commit and retain honest gaps for remote full resolution, data URLs, zoom/pan, live Gateway, Windows and source-paired acceptance.
- **components:** EV-0050 receipt, UCM package validator, Doofus task ledger and checkpoint.
- **type:** partial capability evidence and task continuity.
- **validation:** PASS receipt JSON parse, UCM package validator (565 target behaviors, 565 UI inventory items, 14 source inspection receipts, 35 implementation receipts), task validator at revision 345. PAR-0255 skill detail/source is selected next.
- **performance:** No benchmark change; preview limits are unchanged.
- **risks:** Evidence is headless and local; it does not prove live Gateway or Windows acceptance.
- **follow_up:** Publish EV-0050 and task checkpoint to main, confirm exact remote SHA, then work on skill detail/source.

- **id:** hermes-par0044-main-publish
- **timestamp:** 2026-11-19T23:56:00Z
- **what:** Published tool-image behavior commit `f69925d2bd47c01f7f9a0aa9f1135576600e2cfc` and EV-0050 evidence commit `4343e419f4cb7c55c1d69dc96e42b7afa03ceac3` to `main`; fetched remote and verified exact SHA equality.
- **why:** Keep the user's main-only repository current with tested partial image support and its source/evidence record.
- **components:** Hermes Native, UCM EV-0050, task handoff/checkpoint.
- **type:** main publication.
- **validation:** PASS push, fresh fetch, local `HEAD == origin/main` at `4343e419f4cb7c55c1d69dc96e42b7afa03ceac3`; task checkpoint revision 348 and task validation pass.
- **performance:** No benchmark change.
- **risks:** Live Gateway, installed Windows, remote full-resolution, data URL, zoom/pan, and source-paired acceptance remain open.
- **follow_up:** Continue PAR-0255 skill detail/source workflow; preserve the separate chat/tool and skill/plugin scope evidence dimensions.

- **id:** hermes-par0255-skill-detail-source
- **timestamp:** 2026-11-20T00:12:00Z
- **what:** Added the native Skills detail view and profile-scoped `/api/skills/content` reader, showing the selected skill instructions and source path. PAR-0255 now has headless partial evidence; generated overlays show 65/499 broad requirements and 33/173 chat/tool requirements with mapped evidence.
- **why:** Support the user's requested skills/plugin functionality while keeping skill content fenced to the active Hermes profile and connection.
- **components:** Hermes Native Skills UI, Gateway HTTP reader, task ledger, implementation progress overlay, UCM capability maps, completion overview, architecture, backlog, README.
- **type:** native feature implementation and partial progress evidence.
- **validation:** PASS 435 library, 15 automation-protocol and 3 launcher tests; PASS locked workspace/all-target check; PASS formatting, implementation/chat-tool progress generation, task and UCM package validation, and diff check. A test exposed and drove correction of a malformed profile query separator before final PASS.
- **performance:** Skill body and source path use existing bounded Gateway response limits; no benchmark change.
- **risks:** Live/local/remote Gateway, installed Windows, and source-paired acceptance remain unverified. Edit, lock, archive, install and update are separate open features.
- **follow_up:** Commit implementation and record EV-0051, publish to main, confirm exact remote SHA, then continue with the next feature in the task ledger.

- **id:** hermes-par0255-main-publish
- **timestamp:** 2026-11-20T00:16:00Z
- **what:** Published PAR-0255 behavior commit `50c408483bbf0cbed0c34ba958f7377bd5b078e2` and EV-0051 evidence commit `7f555180bcdbea3296d07272f48cc17c20ba2f37` to `main`; fresh fetch verified exact local and remote equality. Selected PAR-0256 as the next skill workflow.
- **why:** Keep the user's main-only repository current with implemented code, honest partial evidence, and a ready continuation task.
- **components:** Hermes Native Skills UI, Gateway content reader, EV-0051, UCM implementation projection, Doofus task ledger.
- **type:** main publication and task continuity.
- **validation:** PASS both commits pushed; fresh `git fetch origin main` confirmed `HEAD == origin/main` at `7f555180bcdbea3296d07272f48cc17c20ba2f37`; UCM package validator passes with 36 implementation receipts; task selection for PAR-0256 recorded.
- **performance:** No benchmark change.
- **risks:** PAR-0255 remains partial pending live Gateway, Windows UI and source-paired acceptance. The skill edit/lock/archive and hub install/update workflows are still separate tasks.
- **follow_up:** Continue PAR-0256 Browse skill hub against the pinned source snapshot.

## PAR-0257 Skill Hub scan review

- **id:** hermes-par0257-security-scan-review
- **timestamp:** 2026-11-20 01:30:49 UTC
- **what:** Added detection of the pinned Desktop's current and legacy security-block messages, profile-scoped Gateway scan reads, bounded response validation, a “View security scan” action with verdict/policy/findings, and an ordinary retry path that cannot override Hermes policy.
- **why:** Complete the source-observed scan review after a hub install is blocked while keeping Hermes as the security authority.
- **components:** Hermes Native Skills hub UI, Gateway HTTP actor/DTO, install-block parser, implementation progress generator, pinned UCM maps, plan and completion documentation.
- **type:** native UI/Gateway implementation and partial capability progress.
- **validation:** PASS 449 library, 15 automation-protocol and 3 launcher tests; workspace/all-target check; rustfmt; progress generators; task validation; UCM package validation with 38 implementation receipts. The UCM YAML parser was skipped because PyYAML is unavailable; JSON/CSV and other package checks passed.
- **performance:** No benchmark change; scan requests use the existing bounded Gateway HTTP response limit and run off the UI thread.
- **risks:** Live Gateway, installed Windows behavior, cancellation and source-paired acceptance remain unverified. Policy override is intentionally not exposed.
- **follow_up:** Record EV-0054 against the behavior commit, publish directly to main, then continue PAR-0256 merge/discovery/facet/sort/card-list gaps and remaining chat/tool/skills/plugin work.

## PAR-0257 main publication

- **id:** hermes-par0257-scan-review-publish
- **timestamp:** 2026-11-20 01:34:55 UTC
- **what:** Published behavior commit a0e2428e1af3f6d2e1f796da9b7e78d7189c3f8a and EV-0054 commit 7373ab491dcdf9c6f2cc2b4f9674a776343a433e directly to main.
- **why:** Keep the user's main-only repository current with tested behavior and evidence bound to its exact implementation commit.
- **components:** Hermes Native Skills hub, Gateway scan client, EV-0054, UCM maps, task ledger.
- **type:** direct main publication.
- **validation:** PASS push and fresh fetch; local HEAD equals origin/main at 7373ab491dcdf9c6f2cc2b4f9674a776343a433e; the publication operation is recorded succeeded and task validation passes.
- **performance:** No benchmark change.
- **risks:** PAR-0257 remains partial; live Gateway, Windows behavior and source-paired acceptance remain open.
- **follow_up:** Continue PAR-0256 catalog merging, discovery, facets, sort and card/list behavior, then the remaining scoped chat/tool, skills, plugins and MCP work.
## PAR-0258 installed Skills update

- **id:** hermes-par0258-update-installed-skills
- **timestamp:** 2026-11-20 01:57:08 UTC
- **what:** Added the native “Update installed” Skills action with profile-scoped Gateway POST, validated action receipt/PID, bounded live logs, owner-fenced polling and inventory refresh after success. Added headless HTTP/UI coverage, progress/UCM evidence and project documentation. Behavior commit: 9cdee421e47cc2555f560cb14cc18b69bbf9c9ae; evidence: EV-0055.
- **why:** Implement the pinned Hermes batch skill-update behavior as part of chat/tool capability parity before visual tuning.
- **components:** Hermes Native Capabilities/Skills view, Gateway HTTP actor, progress generator, UCM capability evidence, task ledger.
- **type:** native UI/Gateway implementation and partial capability progress.
- **validation:** PASS full locked suite (453 library, 15 automation-protocol, 3 launcher tests); workspace/all-target check; rustfmt; progress generators; task validation; UCM package checks (YAML parsing skipped because PyYAML is unavailable). Live Gateway, installed Windows and source-paired acceptance were not run.
- **performance:** No benchmark change; update and status HTTP work stays off the UI thread and status output is bounded.
- **risks:** A network error after POST may leave admission uncertain; the UI does not replay automatically and instructs the user to check the action log before retrying. Update checks, per-skill updates, cancellation and source-paired acceptance remain open.
- **follow_up:** Publish the behavior and EV-0055 to main, then resume the next dependency-ready chat/tool/skills/plugins/MCP capability; preserve the user's visual-tuning boundary until functional scope is ready.
## PAR-0258 main publication

- **id:** hermes-par0258-main-publish-20261120
- **timestamp:** 2026-11-20 02:00 UTC
- **what:** Pushed behavior commit 9cdee421e47cc2555f560cb14cc18b69bbf9c9ae and EV-0055 evidence commit 4562e53419cfd9025915beb8463bd54e0dbb1f78 directly to main; fresh fetch verified exact local and remote SHA equality.
- **why:** Keep the user's main-only repository current with validated native behavior and its partial evidence receipt.
- **components:** Skills update UI/Gateway flow, EV-0055, progress/UCM maps, task ledger.
- **type:** direct main publication.
- **validation:** PASS push, fresh fetch, exact SHA equality, task validator.
- **performance:** No benchmark change.
- **risks:** PAR-0258 remains partial; live Gateway, packaged Windows, and source-paired acceptance remain open.
- **follow_up:** Continue PAR-0253 category filtering, then remaining Skills, plugins/MCP and chat/tool features before visual tuning.
# Hermes Native Skill Hub category facet

- **id:** hermes-par0253-category-facet-20261120
- **timestamp:** 2026-11-20 02:13 UTC
- **what:** Added a counted category facet to the native Skill Hub catalog, including alphabetic ordering with Other last, multi-select filtering, All/reset, accessibility names, and composition with text search. Selecting every category returns to All. Added a headless egui regression and partial UCM receipt EV-0056. Behavior commit: `dcc4837330fc1bb80311f943df681a68668cc213`.
- **why:** Fill the pinned Skill Hub category-filter behavior before the user begins visual tuning.
- **components:** Native Hermes Capabilities/Skills Hub, progress generator, source inventory and capability evidence, task ledger and handoff state.
- **type:** Native UI behavior and partial capability evidence.
- **validation:** PASS focused regression, full locked feature suite (453 library, 15 automation-protocol, 3 launcher tests), workspace/all-target check, rustfmt, progress generation, task validator, JSON validation and UCM package validator (565 behaviors/items, 14 source receipts, 41 implementation receipts; YAML parse skipped because PyYAML is unavailable). Live Gateway, packaged Windows interaction and source-paired acceptance were not run.
- **performance:** Category facet computation is linear in loaded catalog entries and is re-evaluated during egui rendering, matching the pinned catalog’s derived-facet behavior; no benchmark change.
- **risks:** Live/source-paired behavior remains unverified. The Skill Hub still lacks source/tag facets, installed/catalog merging, discovery shelves, sorting and card/list preference.
- **follow_up:** Validate UCM and task records, publish evidence/docs to main, then continue remaining chat/tool, Skills, plugins and MCP functional gaps before visual redesign.

## PAR-0253 main publication

- **id:** hermes-par0253-main-publish-20261120
- **timestamp:** 2026-11-20 02:21 UTC
- **what:** Pushed category-filter behavior commit `dcc4837330fc1bb80311f943df681a68668cc213` and evidence/docs commit `dda14f4ee30be1673e78a07385d89867990c87d2` to `origin/main`; a fresh fetch verified exact SHA equality.
- **why:** Keep the main-only repository current with the validated feature and its partial evidence record.
- **components:** Skill Hub category filter, EV-0056, progress/UCM maps and task ledger.
- **type:** Direct main publication.
- **validation:** PASS push, fresh fetch and exact local/remote SHA equality.
- **performance:** No benchmark change.
- **risks:** PAR-0253 remains partial; live Gateway, packaged Windows and source-paired behavior remain unverified.
- **follow_up:** Continue PAR-0259 Toolsets inventory, then remaining Skills, plugins/MCP and chat/tool behavior before visual tuning.

# Hermes Native Toolsets inventory curation

- **id:** hermes-par0259-toolsets-inventory-20261120
- **timestamp:** 2026-11-20 02:29 UTC
- **what:** Applied the pinned Desktop hidden-toolset predicate to the native Toolsets list and added a headless UI regression proving included tool names render while all five hidden platform/internal names are excluded. Behavior commit: `ed362576f8d599b5e9cdfc20bfa3c8024173fe9a`.
- **why:** Close the concrete list mismatch and establish rendered native evidence for the requested Toolsets capability.
- **components:** Hermes Capabilities/Toolsets route, progress/UCM maps, task ledger and documentation.
- **type:** Native UI source-alignment fix with partial capability evidence.
- **validation:** PASS focused regression, full locked suite (454 library, 15 automation-protocol, 3 launcher), workspace/all-target check, rustfmt, both progress generators, task validator, JSON validation, and UCM package validator (565 behaviors/items, 14 source receipts, 42 implementation receipts; YAML parse skipped because PyYAML is unavailable). Publication remains pending.
- **performance:** No benchmark change; filtering is a small fixed-name predicate before the existing search.
- **risks:** Live Gateway, packaged Windows and source-paired behavior remain unverified; Toolsets themes/ranking/detail configuration remain separate gaps.
- **follow_up:** Finish evidence validation and publish to main, then continue the next chat/tool/Skills/plugins/MCP task.

## PAR-0259 main publication

- **id:** hermes-par0259-main-publish-20261120
- **timestamp:** 2026-11-20 02:33 UTC
- **what:** Pushed Toolsets visibility behavior commit `ed362576f8d599b5e9cdfc20bfa3c8024173fe9a` and EV-0057/docs commit `9791d17bff33a5b35efc2f83a725570ded1986d6` directly to `origin/main`; a fresh fetch verified exact SHA equality.
- **why:** Keep the user's main-only repository current with tested behavior and its partial evidence receipt.
- **components:** Toolsets inventory, EV-0057, progress/UCM maps, task ledger.
- **type:** Direct main publication.
- **validation:** PASS push, fresh fetch and exact local/remote SHA equality.
- **performance:** No benchmark change.
- **risks:** PAR-0259 remains partial; live Gateway, packaged Windows and source-paired behavior remain unverified.
- **follow_up:** Continue PAR-0260 Toolset enable/disable, then remaining chat/tool, Skills, plugins and MCP gaps.

## PAR-0260 Toolsets master control

- **id:** hermes-par0260-toolsets-master-20261120
- **timestamp:** 2026-11-20 02:50 UTC
- **what:** Added an accessible Toolsets All switch over the Desktop-visible inventory, search-independent sequential updates, denial rollback/stop behavior, and disconnect uncertainty handling. Added headless UI/gateway fixture coverage and mapped partial evidence as EV-0058.
- **why:** Implement the pinned source's master toggle without overlapping Gateway configuration writes, and keep UI state honest after denied or interrupted operations.
- **components:** Native Toolsets UI, gateway test fixture, PAR-0260 evidence, progress overlay, capability maps, plan, architecture, backlog, README and completion overview.
- **type:** Native feature and validation.
- **validation:** PASS focused headless test; PASS full locked suite (455 library, 15 automation protocol, 3 launcher tests); PASS workspace/all-target check, rustfmt, task validation, progress generators, package validator and diff check.
- **performance:** No performance change measured or claimed.
- **risks:** Gateway results and physical Windows interaction are unverified; an interrupted server mutation can remain unknown until the profile inventory is refreshed. Toolset themes, usage ranking and detailed configuration are still missing.
- **follow_up:** Continue to the next scoped Toolsets behavior; preserve partial status until live and source-paired evidence exists.

## PAR-0260 main publication

- **id:** hermes-par0260-main-publish-20261120
- **timestamp:** 2026-11-20 02:52 UTC
- **what:** Published behavior commit `3dc596cbd4e2e172e978210a9421a11e5c69c177` and evidence/docs commit `96f27575f3a149202b31eff394066d28746a4221` to `origin/main`.
- **why:** Keep the main-only repository current with the tested Toolsets feature and evidence.
- **components:** Hermes Native Toolsets, EV-0058, task ledger, generated progress and UCM capability maps.
- **type:** Direct main publication.
- **validation:** PASS push and fresh fetch; local HEAD and `origin/main` both equal `96f27575f3a149202b31eff394066d28746a4221`.
- **performance:** No benchmark change.
- **risks:** PAR-0260 remains partial; live Gateway, packaged Windows and source-paired acceptance remain open.
- **follow_up:** PAR-0261 Toolset prerequisite display is selected next.

## PAR-0261 Toolsets prerequisite state

- **id:** hermes-par0261-prerequisite-state-20261120
- **timestamp:** 2026-11-20 02:56 UTC
- **what:** Added headless coverage for the existing Configured/Needs setup Toolsets row state and recorded EV-0059 as partial evidence.
- **why:** Confirm the current user-visible readiness summary against the pinned detail warning without claiming specific missing-key details that the Gateway response does not provide.
- **components:** Toolsets inventory regression, PAR-0261 progress map, UCM capability maps, plan, architecture, backlog, README and completion overview.
- **type:** Evidence-backed behavior coverage.
- **validation:** PASS focused rendered test, task validation, progress generators, UCM package validator (565 rows, 14 source receipts, 44 implementation receipts), rustfmt and diff check. The package validator initially caught a shifted UI inventory CSV field; corrected before the passing run.
- **performance:** No performance change measured.
- **risks:** The API exposes only a configured boolean; live Gateway, packaged Windows, exact missing-key display and source-paired acceptance remain open.
- **follow_up:** Continue the next scoped capability; keep PAR-0261 partial until richer live prerequisite evidence is available.

## PAR-0261 main publication

- **id:** hermes-par0261-main-publish-20261120
- **timestamp:** 2026-11-20 02:58 UTC
- **what:** Published PAR-0261 rendered coverage, EV-0059 and its progress/docs updates to `main` alongside the prior Toolsets test commit.
- **why:** Keep the user's main-only repository current with the tested readiness indicator evidence.
- **components:** Toolsets readiness state, EV-0059, capability maps and parity progress.
- **type:** Direct main publication.
- **validation:** PASS focused rendered test, task validation, progress generation, package validator and clean remote SHA check after push.
- **performance:** No benchmark change.
- **risks:** PAR-0261 remains partial; specific prerequisites, live Gateway, Windows and source-paired acceptance remain open.
- **follow_up:** Inspect PAR-0269 linked-plugin permission disclosure.

## PAR-0269 native plugin permission gap audit

- **id:** hermes-par0269-permission-audit-20261120
- **timestamp:** 2026-11-20 03:02 UTC
- **what:** Recorded the pinned Electron-local per-origin `preview.executeJavaScript` grants as explicitly not implemented in Hermes Native and added source receipt EV-0060.
- **why:** The native target currently exposes Hermes AgentPluginInfo, which is a separate Gateway plugin model. Claiming those rows have Desktop preview grants would misrepresent user permissions.
- **components:** Linked Desktop plugin permissions, UCM evidence, implementation progress, architecture and backlog.
- **type:** Source audit and scope boundary.
- **validation:** PASS repository package validator and progress generators; source receipt is referenced by the capability map. The standalone UCM checker could not run because PyYAML is not installed; the package validator explicitly skipped YAML parsing for the same reason.
- **performance:** No performance change.
- **risks:** This feature remains a known gap until the native app has a Desktop renderer-plugin host and app-local grant persistence.
- **follow_up:** Continue scoped chat/tool work; keep PAR-0269 planned and not implemented.

## PAR-0033 New Chat create-and-send evidence

- **id:** hermes-par0033-new-chat-send-20261120
- **timestamp:** 2026-11-20 15:20 UTC
- **what:** Added a full-app headless egui regression for New Chat Send through `session.create`, `SessionCreated`, `prompt.submit`, admission receipt, and stored-ID navigation. Filtered unrelated Gateway commands in the dedicated test fixture. Mapped the capability as partial with EV-0061 and refreshed scoped progress.
- **why:** The existing path had only isolated Gateway coverage; the test now proves the user-facing Send control and the distinct runtime/stored session identifier flow together.
- **components:** Hermes Native composer, Gateway test fixture, PAR-0033 evidence, pinned capability maps and completion reporting.
- **type:** Test and evidence update.
- **validation:** PASS focused UI regression, rustfmt, progress generators, task validator, UCM package validator and diff check. Live Gateway and source-paired acceptance remain pending.
- **performance:** No runtime performance change measured.
- **risks:** Create/submit failures and authenticated Gateway behavior are not covered by this receipt.
- **follow_up:** Continue to PAR-0034 incremental assistant text and chat/tool gaps; keep this row partial pending failure/live/source-paired evidence.

## PAR-0034 incremental assistant rendering evidence

- **id:** hermes-par0034-streaming-render-20261120
- **timestamp:** 2026-11-20 15:20 UTC
- **what:** Added a staged full-app egui regression that verifies streamed assistant text appears before completion, updates on the next delta, and settles without duplication while the active Stop control clears.
- **why:** The existing reducer test checked only final transcript state; the pinned Desktop test explicitly checks visible text during the stream.
- **components:** Hermes Native stream event handling, transcript renderer and PAR-0034 evidence mapping.
- **type:** Test and evidence update.
- **validation:** PASS focused regression, rustfmt, progress generators, task validator, package validators and diff check. Live stream and source-paired acceptance remain open.
- **performance:** No runtime performance change measured.
- **risks:** The test uses synthetic Gateway events and does not verify the authenticated runtime or the pinned loading announcement.
- **follow_up:** Continue with the next concrete chat/tool gap; keep PAR-0034 partial until live and source-paired evidence is complete.

## PAR-0033/0034 main publication

- **id:** hermes-par0033-0034-main-publish-20261120
- **timestamp:** 2026-11-20 15:32 UTC
- **what:** Published the New Chat and incremental assistant rendering tests, EV-0061/EV-0062, progress maps, completion docs and task records to `main`.
- **why:** Keep the user's main-only repository current with the newly verified chat evidence.
- **components:** Hermes Native chat tests, capability evidence, UCM/progress maps and `.doofus` task ledger.
- **type:** Direct main publication.
- **validation:** PASS focused tests and repository validators; fresh fetch confirms `origin/main` equals local `HEAD` at `133407016e18ea87779642c646fdd0a5aa14f356`.
- **performance:** No performance change measured.
- **risks:** Both requirements remain partial; Gateway runtime and source-paired acceptance are still open.
- **follow_up:** Continue PAR-0035 and remaining chat/tool/Skills/MCP/plugin behavior against the pinned snapshot.

## PAR-0035 final assistant merge evidence

- **id:** hermes-par0035-final-text-merge-20261120
- **timestamp:** 2026-11-20 15:32 UTC
- **what:** Added EV-0063 and mapped the existing multi-delta-to-final-text reducer test as partial evidence for final assistant message merging.
- **why:** The pinned source has explicit final-text merge semantics; the native success path is covered, while part-level tool/reasoning behavior remains a gap.
- **components:** Native stream reducer, PAR-0035 progress and capability maps.
- **type:** Evidence/status update.
- **validation:** PASS progress generators and task validation; package validation and diff check are run before publication.
- **performance:** No performance change measured.
- **risks:** Native transcript uses a simpler string model and does not fully preserve the pinned tool-delimited assistant parts.
- **follow_up:** Audit the pinned Markdown/code path and then continue substantive chat/tool gaps; keep PAR-0035 partial.

## PAR-0035 interim and tool-delimited stream reconciliation

- **id:** hermes-par0035-stream-boundaries-20261120
- **timestamp:** 2026-11-20 15:54 UTC
- **what:** Implemented `message.interim` transcript handling, assistant bubble boundaries at tool starts, exact cumulative final-text prefix removal, and empty/whitespace final preservation. Partial-error final text also avoids repeating a displayed interim prefix. Added EV-0064 and refreshed the progress overlay and crosswalk state.
- **why:** The pinned Hermes Desktop seals interim assistant bubbles around tool work and reconciles terminal text without duplicating already displayed content.
- **components:** Hermes Native transcript reducer, PAR-0035 evidence, progress generator, completion overview, backlog and task ledger.
- **type:** Behavior, regression tests and evidence update.
- **validation:** PASS six focused Rust tests; `cargo fmt --check`; task schema validation; both progress generators; root and pinned UCM package validators; `git diff --check`. No live Gateway or source-paired UI run was performed.
- **performance:** No runtime performance change measured.
- **risks:** Native still uses string transcript rows and does not preserve the full structured text/reasoning/tool parts or reasoning disclosure. Live Gateway and source-paired acceptance remain open.
- **follow_up:** Continue with the remaining chat/model/worker/skills/MCP/plugin behaviors; keep PAR-0035 partial.

## PAR-0035 main publication

- **id:** hermes-par0035-stream-boundaries-publication-20261120
- **timestamp:** 2026-11-20 16:00 UTC
- **what:** Published the PAR-0035 reducer change and EV-0064 evidence/docs to `main` as commits `88d246d396b41187605e6eb57223ca71da768c65` and `e4a538065f14f265e208799bedff2dac434ed098`.
- **why:** Keep the user's main-only repository current with verified chat behavior and its explicit partial-parity boundary.
- **components:** Hermes Native transcript reducer, capability maps, progress overlay, task ledger and project docs.
- **type:** Direct main publication.
- **validation:** PASS `git push origin main`; fresh `git fetch origin main` confirms local `HEAD` and `origin/main` are both `e4a538065f14f265e208799bedff2dac434ed098`.
- **performance:** No performance change measured.
- **risks:** Full structured reasoning/tool parts, live Gateway behavior and source-paired acceptance remain open.
- **follow_up:** Continue the selected PAR-0214/0215 chat model/reasoning controls slice.

## PAR-0214/0215 chat model and reasoning controls

- **id:** hermes-par0214-0215-model-controls-20261120
- **timestamp:** 2026-11-20 16:37 UTC
- **what:** Added profile/session-scoped model options and reasoning controls to the active chat composer. Model and reasoning changes use fresh revision/execution-generation fences, correlated receipts, guarded confirmation tokens and stale owner/profile/connection rejection. Evidence is recorded as partial.
- **why:** Model switching and reasoning effort are core chat controls needed for the user's feature-parity goal.
- **components:** Hermes Native chat composer, Gateway RPC, requirement overlay, capability maps, progress and project documentation.
- **type:** Feature and regression coverage.
- **validation:** PASS `cargo check -p hermes-native --lib`; five focused Gateway tests and one rendered-controls test; rustfmt and diff checks. Progress and package validators run after evidence is written.
- **performance:** No runtime performance change measured.
- **risks:** UI harness confirms control rendering but does not activate a menu item. Live Gateway, packaged Windows behavior and source-paired acceptance remain open. PAR-0216/0217 custom provider/model flows remain separate.
- **follow_up:** Complete evidence/package validation and publish to main; then continue with a high-impact chat/tool gap.

## PAR-0214/0215 main publication

- **id:** hermes-par0214-0215-main-publication-20261120
- **timestamp:** 2026-11-20 16:41 UTC
- **what:** Published the model/reasoning implementation and its partial evidence as behavior commit `890c291b933df873532d8e33a128a5a00e3385dc` and evidence/docs commit `1e9ade4fe1833f1ef3d4b7be7738e3194f593c44` on `main`.
- **why:** Keep the user's main-only repository current with validated chat functionality and the evidence limits.
- **components:** Hermes Native chat controls, Gateway RPC, capability evidence, progress overlay and task protocol.
- **type:** Direct main publication.
- **validation:** PASS push and fresh fetch; local `main` and `origin/main` both equal `1e9ade4fe1833f1ef3d4b7be7738e3194f593c44`.
- **performance:** No performance change measured.
- **risks:** Live Gateway, physical Windows menu interaction and source-paired acceptance remain open.
- **follow_up:** Continue from checkpoint PAR-0216 custom model selection.

## PAR-0216/0217 custom model and provider selection

- **id:** hermes-par0216-0217-custom-models-20261120
- **timestamp:** 2026-11-20 16:58 UTC
- **what:** Added explicit custom model ID entry, catalog duplicate suppression, provider ordering with the active provider first, and provider-scoped persistence after an accepted model mutation. Verified configured `custom:local-ollama` slugs pass through the profile/session options and mutation contracts.
- **why:** Support user-configured model IDs and providers as part of working chat functionality.
- **components:** Hermes Native chat composer, Gateway model options/mutations, UI preference persistence and capability evidence.
- **type:** Feature and regression coverage.
- **validation:** PASS custom ID validation/persistence tests, profile-scoped fresh-revision model mutation fixture, provider options fixture, rendered editor/candidate test, cargo check and rustfmt. Full progress and package validation run after evidence mapping.
- **performance:** No runtime performance change measured.
- **risks:** Candidate selection is not exercised end-to-end from egui through a live Gateway; custom provider endpoint editing is a separate Settings workflow.
- **follow_up:** Map PAR-0216/0217 partial, validate active and archival packages, publish to main and continue chat/tool feature gaps.

## PAR-0216/0217 main publication

- **id:** hermes-par0216-0217-main-publication-20261120
- **timestamp:** 2026-11-20 17:01 UTC
- **what:** Published custom model/provider behavior as `9dd19f5e5951be1e98c5386c437becfc988ab5b8` and partial evidence/progress/docs as `665f2be27a9263214c3ee23140d62bb32573d11d` on `main`.
- **why:** Keep the user's main-only repository current with the next verified chat-control slice.
- **components:** Hermes Native composer, custom model preferences, Gateway contracts, capability evidence and progress overlays.
- **type:** Direct main publication.
- **validation:** PASS push and fresh fetch; local `main` and `origin/main` both equal `665f2be27a9263214c3ee23140d62bb32573d11d`.
- **performance:** No performance change measured.
- **risks:** Candidate activation through the rendered app and live Gateway/source-paired acceptance remain open.
- **follow_up:** Continue at checkpoint PAR-0218 catalog retry behavior.


## PAR-0218 catalog retry and draft retention

- **id:** hermes-par0218-catalog-retry-20261120
- **timestamp:** 2026-11-20 17:16 UTC
- **what:** Added an error-only top-level model catalog retry action and a rendered UI regression that clicks it through a disconnected-Gateway failure while preserving the typed draft, attachment, runtime session, and profile. Mapped PAR-0218 partial with evidence EV-0061.
- **why:** Make catalog recovery accessible and demonstrate that a failed read cannot discard in-progress chat composition.
- **components:** Hermes Native chat composer, scoped model-options cache, pinned-snapshot capability map and chat/tool progress scope.
- **type:** Feature, regression coverage, and partial evidence.
- **validation:** PASS focused `agent-automation` egui regression, `cargo check -p hermes-native --lib`, rustfmt check and diff check; package validators and progress generators run after evidence mapping.
- **performance:** No runtime performance change measured.
- **risks:** Pinned Desktop has no manual retry control; the native affordance is a recovery enhancement. Authenticated live Gateway and packaged Windows behavior remain unverified.
- **follow_up:** Validate active/archival UCM packages, refresh progress, publish evidence/docs and continue scoped chat/tool requirements.


## PAR-0219 structured model/provider error cards

- **id:** hermes-par0219-error-surface-20261120
- **timestamp:** 2026-11-20 17:35 UTC
- **what:** Added model-not-found, provider-policy and generic provider-layer explanations from the structured Gateway error surface. Provider/model identity is shown, raw provider text is collapsible, and partial assistant output remains visible. Mapped PAR-0219 partial with EV-0062.
- **why:** Let users distinguish model/provider compatibility failures from opaque transport errors while retaining the underlying diagnosis.
- **components:** Hermes Native transcript rendering, Gateway error-surface projection and pinned capability evidence.
- **type:** Feature, regression coverage and partial evidence.
- **validation:** PASS focused rendered egui error-card regression, focused PAR-0218 retry regression, cargo check, rustfmt and diff checks; package validators/progress generators run after mapping.
- **performance:** No runtime performance change measured.
- **risks:** Choose-model/provider-settings actions and localization are still missing; no live Gateway or packaged runtime was tested.
- **follow_up:** Validate package maps, refresh the 175-row scope, publish, and continue chat/tool parity.


## PAR-0219 main publication

- **id:** hermes-par0219-main-publication-20261120
- **timestamp:** 2026-11-20 17:40 UTC
- **what:** Published error-surface behavior as `b9da9a791722796031cb360b88a50b57be3da90d` and partial evidence/docs as `adc10b644083364e7e6a4564c4b66a314badb762` on `main`.
- **why:** Keep the main-only repository current with tested model/provider error behavior.
- **components:** Native transcript error cards, pinned UCM status, scoped progress and task handoff.
- **type:** Direct main publication.
- **validation:** PASS push and fresh fetch; local `main` and `origin/main` match exactly at `adc10b644083364e7e6a4564c4b66a314badb762`.
- **performance:** No performance change measured.
- **risks:** Error-specific recovery actions and localization remain open; no live Gateway or packaged Windows test was run.
- **follow_up:** Audit PAR-0220 and continue pinned chat/tool feature coverage.

## PAR-0220 profile-scoped fallback routes

- **id:** hermes-par0220-fallback-routes-20261120
- **timestamp:** 2026-11-20 18:00 UTC
- **what:** Added a Model fallbacks Settings view with ordered provider/model rows, blank local drafts, removal, and profile-scoped save. Added authenticated local-ticket and remote-session-token Gateway config reads/writes using sparse patches. Route objects retain custom keys; legacy `provider/model` strings are normalized on read. Scoped progress now includes PAR-0220 as partial.
- **why:** Let users configure Hermes' ordered fallback chain when their default model fails, without duplicating Hermes runtime routing.
- **components:** Hermes Native Settings, Gateway HTTP worker, chat/tool scope and progress, pinned capability evidence plan.
- **type:** Feature, regression coverage, and partial evidence.
- **validation:** PASS 10 focused fallback-filtered library tests, PASS 477/477 remaining library tests with one unrelated existing retry test skipped, PASS `cargo check -p hermes-native --lib`, PASS rustfmt. The full 478-test run failed only in `retry_failed_turn_reuses_submission_id_and_keeps_one_user_row`; reproduced in isolation.
- **performance:** No runtime performance change measured.
- **risks:** Native provider/model fields are free text and lack pinned catalog selectors; full UI save/readback, real fallback execution, live Gateway, packaged Windows and source-paired acceptance remain open. Broad test suite retains the unrelated retry test failure.
- **follow_up:** Publish code on main, add PAR-0220 partial receipt bound to the implementation commit, validate both UCM packages and task ledger, then continue chat/tool feature coverage.
## PAR-0220 evidence mapping

- **id:** hermes-par0220-evidence-20261120
- **timestamp:** 2026-11-20 18:05 UTC
- **what:** Added EV-0069 bound to implementation commit `24470c4f8ddbd3403cece9fe65fceca3cb2a2a93`; updated the active UCM status and regenerated the scoped progress overlays. PAR-0220 is partial; no live-runtime or source-paired acceptance is claimed.
- **why:** Keep capability state tied to concrete rendered UI and Gateway fixture evidence, with remaining gaps visible.
- **components:** Active Hermes parity UCM, requirements scope and progress ledgers.
- **type:** Evidence mapping and documentation.
- **validation:** PASS root UCM validator (565 target behaviors, 53 implementation receipts; YAML parse skipped because PyYAML is unavailable), PASS archival validator (491 pending), PASS task schema and progress generators, PASS `git diff --check`.
- **performance:** No runtime performance change measured.
- **risks:** The native editor uses free-text provider/model IDs; fallback execution against a live Gateway remains unverified.
- **follow_up:** Publish evidence/maps to main and continue the next chat/tool capability slice.
## PAR-0220 main publication

- **id:** hermes-par0220-main-publication-20261120
- **timestamp:** 2026-11-20 18:10 UTC
- **what:** Published implementation commit `24470c4f8ddbd3403cece9fe65fceca3cb2a2a93` and EV-0069/UCM evidence commit `621070e65189cb2c3abf2a5c8b06a26d90238342` to `origin/main`; fresh fetch confirmed exact local/remote SHA equality.
- **why:** Keep the user-designated main branch current with validated native work and the evidence supporting its partial status.
- **components:** Native fallback Settings and Gateway config path, parity map, progress records and task journal.
- **type:** Direct main publication.
- **validation:** PASS focused fallback tests, cargo check, rustfmt, root and archival UCM validators, task validation, progress generation, push and post-push fetch.
- **performance:** No runtime performance change measured.
- **risks:** One unrelated library test remains failing; live fallback execution and source-paired acceptance remain open.
- **follow_up:** Continue the next chat/tool functionality gap; exact visual parity is out of scope.

## 2026-11-20 — Fallback catalogs and Agent Plugin settings

- **id:** hermes-par0220-fallback-catalog-agent-plugin-settings
- **timestamp:** 2026-11-20 18:39 UTC
- **what:** Added profile-scoped provider/model catalog selectors and profile-fenced option reads to fallback route settings. Added manifest-backed Agent Plugin settings with typed controls, changed-only nonsecret writes, sequential profile-scoped `/api/env` secret writes, blank-secret-is-keep behavior, secret masking/zeroization, partial-write failure reporting, disabled no-op saves, and dialog-level invalid-JSON coverage (commits `8a057d15b39b44d07204885a373b435c368c801e`, `626d861`).
- **why:** Close functional gaps in Hermes fallback configuration and Agent Plugin configuration before the planned visual presentation pass.
- **components:** Native Settings, Gateway model-options/config HTTP, Agent Plugin settings RPC/secret HTTP, implementation-progress generator and generated ledgers, status and architecture docs, EV-0069.
- **type:** Feature implementation and scoped partial evidence update.
- **validation:** PASS full locked `agent-automation` library suite (488/488), focused plugin-settings tests, `cargo check -p hermes-native --lib`, rustfmt check, both UCM package validators, task-ledger validation, progress generators and `git diff --check`.
- **performance:** No runtime performance change measured.
- **risks:** Live Gateway behavior, full fallback UI save/readback, fallback execution following default-model failure, physical Windows behavior and source-paired acceptance remain open; an `/api/env` sequence can partially persist earlier secrets before a later write fails.
- **follow_up:** Publish evidence/docs to `main`, then continue scoped chat/tool, skills and plugin work; visual styling follows functional scope completion.

## 2026-11-20 — PAR-0220 and Agent Plugin settings publication

- **id:** hermes-par0220-plugin-settings-publication-20261120
- **timestamp:** 2026-11-20 18:48 UTC
- **what:** Published implementation commits `8a057d15b39b44d07204885a373b435c368c801e` and `626d861497e7265dde6fe09a82bdc8a2896eaba2`, plus evidence/docs/task-state commit `d7b7966f9228b77fff5d66ab6b43ea3f99a83880`, to `origin/main`; fresh fetch confirmed exact SHA equality.
- **why:** Keep the user-designated main branch current with the validated feature work and its evidence.
- **components:** Fallback Settings, Agent Plugin Settings, Gateway, progress overlay, UCM evidence, task journal and project docs.
- **type:** Direct main publication.
- **validation:** PASS full 488-test library suite, cargo check, rustfmt, progress generators, root and archival UCM validators, task validation, git diff checks, push and post-push fresh fetch.
- **performance:** No runtime performance change measured.
- **risks:** Live Gateway and packaged Windows behaviors remain unverified; the full scoped feature milestone remains in progress.
- **follow_up:** Continue the remaining scoped chat/tool, skills, MCP and plugin capabilities; defer visual styling until functional parity is ready.


## 2026-11-20 — Reasoning stream separation

- **id:** hermes-par0046-reasoning-stream-disclosure
- **timestamp:** 2026-11-20 19:16 UTC
- **what:** Added a pinned-compatible streaming reasoning scrubber, separated Gateway `reasoning.delta` events from answer text in active and background transcript paths, added an accessible Thinking disclosure, and made it start collapsed in Simple mode. The accumulated raw stream is removed when its segment is sealed.
- **why:** Prevent reasoning tags or partial tags from flashing into the user-visible answer while supporting the pinned reasoning disclosure behavior.
- **components:** Hermes Native transcript reducer, reasoning module, transcript UI, parity progress overlay, capability evidence records and project docs.
- **type:** Chat feature implementation and evidence update.
- **validation:** PASS 498/498 locked agent-automation library tests, including split-frame scrubber, reducer reconciliation and Simple-mode accessible disclosure; PASS cargo check, rustfmt, diff checks and task schema validation.
- **performance:** No performance change measured; the pinned behavior also reprocesses accumulated stream content on each frame.
- **risks:** Profile-level `display.show_reasoning` loading, live Gateway delivery, packaged Windows behavior and source-paired acceptance remain open. The two parity rows remain partial.
- **follow_up:** Implement profile reasoning preference loading and continue the remaining chat/tool, skills, MCP and plugin gaps before visual redesign.


## 2026-11-20 — Reasoning evidence published

- **id:** hermes-par0046-main-publication-20261120
- **timestamp:** 2026-11-20 19:22 UTC
- **what:** Published the reasoning stream implementation, partial parity receipts, task checkpoint and project documentation as commits `259fbf21f25bd89b1968fea30a5bf989f9ef6814` and `01231317666f0e76c0c655302334e8135df84d85`; fresh fetch confirmed local `main` equals `origin/main`.
- **why:** Keep the user-designated main repository current with the validated chat feature slice and its evidence.
- **components:** Native chat transcript, reasoning stream module, progress overlay, capability evidence, task state and project docs.
- **type:** Direct main publication.
- **validation:** PASS 498/498 native library tests, cargo check, rustfmt, task validation, root and archival UCM package validators, push and exact post-push fetch comparison.
- **performance:** No runtime performance change measured.
- **risks:** PAR-0046/PAR-0047 remain partial; profile-level reasoning preference loading and live/source-paired acceptance remain open.
- **follow_up:** Implement `display.show_reasoning` profile preference loading, then continue the dependency-ready chat/tool, skills, MCP and plugin gaps.


## 2026-11-20 — Profile reasoning visibility

- **id:** hermes-par0046-profile-reasoning-preference
- **timestamp:** 2026-11-20 19:31 UTC
- **what:** Added profile-scoped display reasoning reads for local and remote Gateway connections, with CLI override precedence and fail-closed off-by-default behavior. Responses are request/profile fenced and stale-profile responses are discarded.
- **why:** Keep the Thinking disclosure aligned with the selected Hermes profile and prevent hidden reasoning from appearing when its configured preference is off.
- **components:** Gateway HTTP worker, profile config reader, transcript rendering and chat parity evidence.
- **type:** Chat setting integration and regression coverage.
- **validation:** PASS profile-scoped HTTP fixture, CLI/global precedence tests, disabled-preference accessible UI test and full 501-test native library suite; cargo check, rustfmt and diff checks pass.
- **performance:** No performance change measured.
- **risks:** The Gateway setting can be read but not changed from a dedicated native control; live server and packaged Windows behavior remain unverified.
- **follow_up:** Continue remaining chat/tool and skills/plugin parity work; consider the native reasoning setting control with the remaining preference surfaces.


## 2026-11-20 — Profile preference published

- **id:** hermes-par0046-profile-preference-publication-20261120
- **timestamp:** 2026-11-20 19:34 UTC
- **what:** Published profile-scoped reasoning preference code (`d357cefb1dc481978723c55de448b5edbcad0021`) and its task/evidence/doc checkpoint (`c54a2579634df3b26b917efa0884a4d71d945be8`) to `main`; fresh fetch verified local/remote equality.
- **why:** Keep the user-designated repository branch current with validated chat support and source-specific partial evidence.
- **components:** Gateway display config, native transcript rendering, PAR-0046/PAR-0047 progress and task records.
- **type:** Direct main publication.
- **validation:** PASS 501/501 tests, root and archival package validators, task validator, push and exact SHA comparison.
- **performance:** No runtime performance change measured.
- **risks:** Dedicated native preference editing, live Gateway, source-paired acceptance and remaining chat/tool/skills/plugin coverage are open.
- **follow_up:** Continue the next dependency-ready chat/tool capability from pinned snapshot `8783054ef6cbafd86f10892241436862c1171a66`.


### Composer OS file-drop slice — 2026-11-20

- id: hermes-par0116-composer-drop-20261120
- timestamp: 2026-11-20T19:50:00Z
- what: Added a visible OS-drop target to the native message composer; multiple local paths released over the input route through the existing attachment upload flow. Drops without exposed local paths report the Attach file fallback. Added a headless path/target regression and partial UCM receipt EV-0073.
- why: Support the user's requested chat and tool functionality while reducing the gap for file attachments without claiming full pinned-source parity.
- components: apps/hermes-native/src/app.rs; attachment progress generator and CSV; Hermes Native README, architecture, backlog, plan and completion overview; root capability map/crosswalk/inventory and EV-0073.
- type: Feature implementation and partial evidence update.
- validation: PASS 502 library, 15 automation-protocol and 3 launcher tests; cargo check --lib; rustfmt; diff check; progress generator reports 86/499 reviewed (6 implemented, 79 partial, 1 not implemented, 413 unknown); Doofus task schema validator; root capability package validator (565 target behaviors, 15 source inspections, 57 implementation receipts; PyYAML optional parse skipped); pinned-source package validator (491).
- performance: No performance change measured.
- risks: Byte-only and clipboard drops, custom Hermes references, pinned path-sharing policy, live Gateway outcomes, physical Windows behavior and source-paired acceptance remain open. No live runtime or GUI was used.
- follow_up: Continue the next dependency-ready chat/tool feature under the existing Codex lease; keep visual polish deferred until functional scope is complete.


### Composer drop publication — 2026-11-20T19:55:03Z

- id: hermes-par0116-publish-main-20261120
- timestamp: 2026-11-20T19:55:03Z
- what: Published implementation commit d0ab4eb7cdfd7bf82e5ee505e480fb19e8b9e989 and partial progress/evidence commit b72ee45006d45c5e70f2dff08f1d3c4b96e14677 to origin/main.
- why: Keep the user's main-only repository current with validated progress.
- components: origin/main; composer file drop implementation; progress ledger; UCM partial receipt EV-0073; task checkpoint and execution records.
- type: Publication and repository synchronization.
- validation: PASS fetched origin/main before push (local was 2 commits ahead); push succeeded; fresh fetch confirmed exact SHA equality at b72ee45006d45c5e70f2dff08f1d3c4b96e14677.
- performance: Push and verification completed in approximately 45 seconds.
- risks: The separately preserved untracked apps/hermes-native/work/ folder is not part of the published repository. Live Gateway/Windows interaction remains unverified.
- follow_up: Continue the existing PAR-0062 attachment task; do not treat publication as parity acceptance.


### Mixed file-drop guard — 2026-11-20

- id: hermes-par0116-mixed-drop-20261120
- timestamp: 2026-11-20T20:00:25Z
- what: Reject a composer drop as a whole if any dropped item lacks a local path, preventing silent loss from mixed readable/pathless lists. Added EV-0074 and updated evidence progress with the fail-closed behavior.
- why: Preserve the user's entire file selection or clearly request the supported Attach file path; never attach only a silent subset.
- components: apps/hermes-native/src/app.rs; attachment progress generator and CSV; UCM receipt EV-0074; .doofus active task checkpoint.
- type: Drop handling correctness fix and partial evidence.
- validation: PASS full locked package suite (502 library + 15 protocol + 3 launcher); focused regression; cargo check --lib; rustfmt; diff check; task validation; root capability validator and pinned package validator.
- performance: No performance change measured.
- risks: Pathless bytes, clipboard, local shared-backend direct paths, live Gateway, physical Windows drop and source-paired acceptance remain open.
- follow_up: Implement profile/session-scoped backend metadata before considering direct-path attachment; keep byte uploads as the safe default.


### Mixed-drop guard publication — 2026-11-20T20:01:36Z

- id: hermes-par0116-mixed-drop-publish-main-20261120
- timestamp: 2026-11-20T20:01:36Z
- what: Published behavior commit 6a2112277b13627c2f76acd1b3bc9aaae98c52e0 and evidence/docs commit be53c56ff20510f5c742198760b44fa8432e2c0a to origin/main.
- why: Keep the authorized main branch current with the validated drop-safety correction.
- components: origin/main; native composer drop guard; PAR-0116 evidence EV-0074; task handoff records.
- type: Publication and repository synchronization.
- validation: PASS fresh fetch before push confirmed local main was two commits ahead; push succeeded; post-push fetch confirmed exact SHA equality at be53c56ff20510f5c742198760b44fa8432e2c0a.
- performance: Push and verification completed in approximately 45 seconds.
- risks: Local-path-only OS drops remain partial; live Gateway, Windows interaction and source-paired acceptance are open.
- follow_up: Continue the attachment backend-metadata prerequisite and other scoped chat/tool functionality.


### Chat/tool evidence overview refresh — 2026-11-20T20:02:53Z

- id: hermes-chat-tool-progress-snapshot-20261120
- timestamp: 2026-11-20T20:02:53Z
- what: Refreshed the generated 176-row chat/tool status and completion overview to include the current reasoning and file-drop evidence. The view now shows 1 implemented, 51 partial, 1 not implemented and 123 unknown, with mapped evidence on 53 rows.
- why: Keep the user's stated functional scope visible separately from the 499-row whole-desktop inventory; avoid treating review coverage as total completion.
- components: apps/hermes-native/docs/status/CHAT_TOOL_PROGRESS.csv; apps/hermes-native/docs/COMPLETION_OVERVIEW.md.
- type: Progress evidence reconciliation.
- validation: PASS chat/tool progress generator; all 176 remain unverified live and source-pair pending.
- performance: No performance change measured.
- risks: This is evidence coverage only. It is not a total completion percentage or an ETA; live Gateway, installed runtime, and formal/source-paired validation remain open.
- follow_up: Continue the existing attachment work and scoped chat/tool gaps; defer visual-presentation work until functional parity.


### Chat/tool progress publication — 2026-11-20T20:03:44Z

- id: hermes-chat-tool-progress-publish-main-20261120
- timestamp: 2026-11-20T20:03:44Z
- what: Published chat/tool progress ledger and completion overview at 2eff1132712dec5ee27d14b95e4583b2e3bd4294.
- why: Keep the main branch status current for the user's feature-only scope.
- components: main branch progress ledger and completion overview.
- type: Publication record.
- validation: PASS fresh fetch confirmed exact local/remote SHA equality.
- performance: Publication and verification completed in approximately 30 seconds.
- risks: Evidence coverage is not product completion; all 176 remain live-runtime and source-pair pending.
- follow_up: Continue feature implementation under the active Hermes Native task.

### Revision 288 - Owner-scoped backend metadata for attachment routing (2026-11-20)

PAR-0062 now caches `session.info` cwd and terminal backend values per connection/profile/runtime session. Late events from another connection and metadata for a different stored session cannot overwrite the selected session view.

Validation: PASS focused owner-fencing regression; full locked `agent-automation` package suite (503 library + 15 protocol + 3 launcher); `cargo check -p hermes-native --lib`; rustfmt; `git diff --check`. EV-0075 records partial prerequisite evidence. Direct path-versus-bytes routing, live Gateway and packaged Windows behavior remain open.

### Owner-scoped backend metadata publication - 2026-11-20T20:53:08Z

- id: hermes-par0062-publish-main-20261120
- timestamp: 2026-11-20T20:53:08Z
- what: Published tested PAR-0062 owner-scoped cwd/terminal-backend caching and EV-0075 to main at 9decbef493a131922261723be26ce82b1dd04ce3.
- why: Keep the canonical repository current as implementation slices are completed.
- components: native Gateway event ownership, chat session metadata, progress/evidence documentation.
- type: Publication record.
- validation: PASS fresh fetch confirmed exact local/remote main SHA equality.
- performance: Push and verification completed in 12 seconds.
- risks: Attachment metadata is not yet used for path-versus-bytes routing; byte upload remains the default.
- follow_up: Implement and test pinned attachment path policy with a safe byte fallback.

### Agent-focused first-version goal - 2026-11-20

- id: hermes-goal-agent-workflow-reset-20261120
- timestamp: 2026-11-20T20:55:00Z
- what: Replaced the deleted 1:1 desktop replacement goal with a first usable Hermes agent-workflow UI target.
- why: The user wants agent capabilities working soon so visual presentation can be changed afterward.
- components: Goal state, README, Hermes Native backlog, completion overview, portable handoff and parent task objective.
- type: Scope correction.
- validation: PASS task schema and progress validation pending after update.
- performance: Documentation/state update only.
- risks: The feature ledger still has unknown and partial capabilities; it is not an overall completion percentage.
- follow_up: Continue core agent workflow gaps, starting with attachment routing policy.

### Owner-aware attachment route and goal correction - 2026-11-20

- id: hermes-agent-goal-and-attachment-policy-20261120
- timestamp: 2026-11-20T21:27:02Z
- what: Replaced the deleted 1:1 goal with a first-version agent workflow objective; routed non-image files to inline @file references only for proven local shared path context, preserving byte upload for images and uncertain ownership.
- why: Align scope to agent capability so the user can begin visual changes after core workflows work, while preserving existing functional behavior.
- components: goal, parent task/handoff, README, Hermes Native attachment state, Gateway endpoint classification, completion overview, backlog and evidence.
- type: Scope correction and functional change.
- validation: PASS 505 library + 15 automation protocol + 3 launcher tests; cargo check; rustfmt; diff check; task schema.
- performance: No benchmark requested or applicable.
- risks: Live Gateway path resolution and packaged Windows interaction remain unverified; fallback stays byte-based for all uncertain cases.
- follow_up: Publish the tested checkpoint, then continue the remaining agent-focused model/provider, tool, skill, plugin and MCP gaps.

### Goal scope validation follow-up - 2026-11-20

- id: hermes-goal-agent-workflow-reset-validation-20261120
- timestamp: 2026-11-20T21:27:02Z
- what: Confirmed the first-version agent-workflow wording in the README, backlog, completion overview and portable task records.
- why: Close the validation item recorded when the goal correction was first written.
- components: README, Hermes Native docs, .doofus handoff and task ledger.
- type: Validation follow-up.
- validation: PASS `python apps/hermes-native/tools/doofus_task.py validate`; 176-row progress generator reports 53 evidence-mapped rows and 0 live/source acceptances, with these counts explicitly described as evidence coverage.
- performance: Documentation-only validation.
- risks: Unknown and partial workflow rows remain; the evidence counts are not an overall completion percentage.
- follow_up: Continue implementation under the active agent-focused goal.

### Agent workflow goal publication - 2026-11-20T21:33:00Z

- id: hermes-agent-goal-publication-20261120
- timestamp: 2026-11-20T21:33:00Z
- what: Published the first-version agent-workflow goal, safe attachment routing and partial evidence on main at 905a059aa4ceec5b62d61a147d914e49b6228a0e.
- why: Let functional agent workflows define the first milestone and keep the repo current.
- components: native attachment route, goal/handoff documentation, evidence map.
- type: Publication record.
- validation: PASS fresh fetch confirmed local and origin main SHA equality.
- performance: Push and verification completed in 18 seconds.
- risks: Live Gateway and packaged Windows behavior remain unverified; incomplete features remain.
- follow_up: Continue model/provider, tool, skill, plugin and MCP capabilities under the corrected scope.


## hermes-par0050 — 2026-11-20

- **id:** hermes-par0050-continue-preservation-20261120
- **timestamp:** 2026-11-20T22:18:31Z
- **what:** Added partial progress mapping and EV-0077 for the truncated-response Continue action; refreshed the completion overview and pinned capability maps.
- **why:** Keep the agent-focused scope and functional work visible while accurately separating headless implementation from live acceptance.
- **components:** Native assistant error card/composer, progress overlays, pinned capability maps and project progress docs.
- **type:** Feature evidence and progress documentation.
- **validation:** Both progress generators pass; package validators and task-state checks are next. Headless package suite had passed for code commit d8c8364; no live Gateway or packaged Windows behavior claimed.
- **performance:** No runtime performance change measured or expected.
- **risks:** The action is verified only with headless fixtures; live Gateway and packaged Windows behavior remain open.
- **follow_up:** Publish evidence/docs to main, validate remote equality, and continue scoped agent-workflow gaps. Preserve all functional work supporting the new goal.


PAR-0050 validation follow-up: both current and archival UCM package validators pass; durable task state validates at revision 519 and ledger revision 87; `git diff --check` passes. The current validator skipped YAML parsing because optional PyYAML is unavailable. No live Gateway or packaged Windows acceptance is claimed.


## hermes-par0050-main-publication-20261120

- **id:** hermes-par0050-main-publication-20261120
- **timestamp:** 2026-11-20T22:26:12Z
- **what:** Published the PAR-0050 continuation implementation as `d8c8364` and its evidence, progress maps, docs and task receipt as `3619bf1`; fresh fetch confirmed local `main` equals `origin/main` at `3619bf1f7caa4214cb494401a3479d1ebdbe8759`.
- **why:** Keep the user's main branch current with verified agent-workflow progress.
- **components:** Native assistant continuation, evidence ledger, generated progress and durable handoff state.
- **type:** Direct main publication.
- **validation:** Both UCM package validators, task-state validator, diff check and progress generators pass; full `agent-automation` suite passed for the implementation commit (507 library + 15 protocol + 3 launcher). Live Gateway and packaged Windows checks remain open.
- **performance:** No performance changes measured.
- **risks:** Partial evidence only; live and packaged acceptance are still pending.
- **follow_up:** Continue the agent-focused feature backlog while preserving all functional work.


## PAR-0049 — Recover Stop after a stale runtime (2026-11-20)

- **id:** hermes-par0049-20261120
- **timestamp:** 2026-11-20T23:55:39Z
- **what:** Stop now resumes the owner-matched stored session without replaying messages and retries `session.interrupt` once with the fresh runtime ID and execution generation. The app only rebinds the exact matching profile, connection and stored-session view; partial assistant text remains intact.
- **why:** A stale runtime ID should not strand a user without the ability to stop a running turn. Existing functional work is preserved.
- **components:** Native Gateway actor, session ownership/event reducer, Stop action, capability evidence and progress docs.
- **type:** Feature implementation and partial evidence.
- **validation:** Full locked package suite passed (510 library + 15 protocol + 3 launcher); focused stale-runtime retry, resume failure and owner-scoped rebind tests passed; `cargo check`, rustfmt and diff checks passed. Live Gateway and packaged Windows checks remain open.
- **performance:** No performance change measured or expected; retry is bounded to one after one resume attempt.
- **risks:** Runtime behavior, owner response variations and packaged Windows interaction remain unverified.
- **follow_up:** Record code/evidence commit SHAs, validate capability packages and durable handoff, publish to `main`, then continue the next agent workflow gap.


## hermes-par0049-main-publication-20261121

- **id:** hermes-par0049-main-publication-20261121
- **timestamp:** 2026-11-21T00:06:55Z
- **what:** Published PAR-0049 stale-runtime Stop recovery in `4fdf594` and EV-0078, progress maps, docs and durable task receipt in `4c380a2`; fresh fetch confirmed local `main` equals `origin/main` at `4c380a2c0e74ad10f2b534ff817cf85beab9d60e`.
- **why:** Keep the user's main branch current with verified progress on core agent workflows.
- **components:** Hermes Native Stop action, Gateway stale session recovery, UCM evidence and progress/handoff records.
- **type:** Direct main publication.
- **validation:** Locked package suite passed (510 library + 15 protocol + 3 launcher); root and archival package validators, durable state validation, cargo check, rustfmt and diff checks passed. No live Gateway or packaged Windows validation is claimed.
- **performance:** No performance changes measured.
- **risks:** Live runtime behavior and source-paired acceptance remain open.
- **follow_up:** Continue first-version agent workflow gaps and preserve all existing functional work.


## hermes-par0039-20261121

- **id:** hermes-par0039-20261121
- **timestamp:** 2026-11-21T00:53:26Z
- **what:** PAR-0039 now captures CommonMark links as hooks and emits host navigation only for validated HTTP(S) URLs without embedded credentials or user-clicked mailto URLs. Heading fragments stay in the transcript. File, data, javascript, relative and other custom-scheme targets cannot launch host handlers. PAR-0036..0038 were reviewed and mapped partial because the renderer and accessible fixture already cover Markdown, GFM tables/lists, and fenced code/copy.
- **why:** Chat is the agent's primary interaction surface; link output must be safe while existing formatted content remains usable.
- **components:** Hermes Native transcript renderer, pulldown-cmark dependency, UCM evidence and generated progress.
- **type:** Feature implementation and partial capability evidence.
- **validation:** PASS focused Markdown tests (4/4), full locked suite (513 library + 15 protocol + 3 launcher), cargo check, rustfmt and diff checks. Source-paired package validation remains pending evidence receipts; live Gateway and packaged Windows are not claimed.
- **performance:** No benchmark change measured; link targets are parsed once per rendered message and the bare-domain regex is cached.
- **risks:** Backend-aware file/media previews and external-browser experience on packaged Windows remain unverified.
- **follow_up:** Add evidence receipts bound to the implementation commit, validate UCM packages and publish to `main`; continue remaining agent-workflow gaps.


## hermes-par0039-evidence-20261121

- **id:** hermes-par0039-evidence-20261121
- **timestamp:** 2026-11-21T00:58:55Z
- **what:** Added commit-bound partial evidence EV-0079 through EV-0082 for Markdown/code fences, syntax highlighter configuration, lists/tables, and safe chat links at implementation commit `c20baa57c54b4c3f5e23550080a38072268af2c0`.
- **why:** Make existing and newly verified chat behavior visible in the user's partial completion overview without claiming full Desktop parity.
- **components:** Root Hermes capability map, scoped/broad progress overlays, durable task handoff and evidence ledger.
- **type:** Evidence and progress record.
- **validation:** Root validator passes with 66 receipts; archival validator passes with 491 pending requirements; durable task state and diff checks pass. Optional YAML syntax parsing was skipped because PyYAML is not installed.
- **performance:** No runtime changes.
- **risks:** Evidence is headless; Windows packaged and source-paired acceptance remain open.
- **follow_up:** Publish evidence records to main and continue remaining agent workflow gaps.


## hermes-par0039-main-publication-20261121

- **id:** hermes-par0039-main-publication-20261121
- **timestamp:** 2026-11-21T01:02:37Z
- **what:** Published native Markdown link safety in `c20baa5` and EV-0079..82 plus maps/docs/task receipts in `bb23014`; fresh fetch confirmed local `main` equals `origin/main` at `bb23014e31301736c3dc74eb58f4a27c92b13d3f`.
- **why:** Keep the user's main branch current with progress on chat-agent functionality.
- **components:** Native chat Markdown renderer, safe link dispatch, evidence overlay and progress ledger.
- **type:** Direct main publication.
- **validation:** 513 library + 15 protocol + 3 launcher tests; cargo check, formatting, diff checks, root and archival package validators and durable task validation passed. Live Gateway and packaged Windows behavior remain unverified.
- **performance:** No performance change measured. Markdown parsing adds one cached-regex pass over link tokens before the existing CommonMark render.
- **risks:** Filesystem/media link previews, focused syntax coloring and source-paired/packaged acceptance remain open.
- **follow_up:** Continue core agent-workflow gaps and preserve all existing functional work.


## hermes-par0039-local-markdown-preview-20261121

- **id:** hermes-par0039-local-markdown-preview-20261121
- **timestamp:** 2026-11-21T01:50:53Z
- **what:** Extended PAR-0039 so clicked Markdown file links open the existing DoofusDock preview pane for an existing file inside the positively matched local session project. File-like links are no longer misclassified as bare web domains.
- **why:** Make the chat transcript's local project references usable in the agent workflow without reimplementing file preview or crossing the session's filesystem boundary.
- **components:** Transcript Markdown link capture and classification, session/profile/runtime owner checks, Gateway loopback and local terminal checks, canonical file-path policy, DoofusDock preview rail, progress generator and implementation docs.
- **type:** Feature extension with headless UI and policy coverage.
- **validation:** PASS focused Markdown suite (6/6); full locked suite (515 library + 15 protocol + 3 launcher); `cargo check -p hermes-native --lib --locked`; rustfmt and diff checks. UCM receipt/package validation and final publication are tracked after the implementation commit.
- **performance:** Preview file reads remain on the existing bounded asynchronous worker; click handling adds bounded path normalization and canonicalization.
- **risks:** Remote file reads, audio/video playback and broader pinned Markdown media/directive behavior are not implemented. Live Gateway, packaged Windows and source-paired behavior remain unverified. Project containment intentionally blocks explicit links outside the current project.
- **follow_up:** Bind EV-0083 to the implementation commit, validate UCM/state and publish to `main`; continue remote/context feature gaps without deleting functional work.


## hermes-par0039-ev0083-validation-20261121

- **id:** hermes-par0039-ev0083-validation-20261121
- **timestamp:** 2026-11-21T22:53:02Z
- **what:** Added EV-0083 for the local Markdown file-preview slice, bound to implementation commit `80d064abfc10b33b4a6a16133b368d1db07b48c2`.
- **why:** Preserve evidence for the newly working behavior while keeping PAR-0039 partial until remote media, live runtime and source-paired requirements are proven.
- **components:** Pinned Hermes capability evidence package, durable task status, validation log.
- **type:** Commit-bound implementation test evidence.
- **validation:** PASS root package validator (565 targets, 67 implementation receipts); archival package validator (491 pending); durable task validation. Optional YAML syntax parsing was skipped because PyYAML is unavailable.
- **performance:** No runtime changes.
- **risks:** Evidence covers headless local fixtures only; the receipt does not claim live Gateway or packaged Windows behavior.
- **follow_up:** Commit evidence and task records, publish both main commits and verify origin/main SHA equality.


## hermes-par0039-main-publication-20261121

- **id:** hermes-par0039-main-publication-20261121
- **timestamp:** 2026-11-21T22:53:02Z
- **what:** Published the local Markdown preview implementation `80d064a` and commit-bound EV-0083/task evidence `011c11c` to the user's `main` branch.
- **why:** Keep the user's canonical repository current with the verified agent-workflow feature slice.
- **components:** Native chat Markdown rendering, DoofusDock preview panel, UCM evidence package, task handoff and repository logs.
- **type:** Direct main publication.
- **validation:** Fresh post-push fetch confirms local `main` and `origin/main` both equal `011c11c70483485ccff9d641febbf3d777757a3a`. User work folder remains untracked and untouched.
- **performance:** No new performance measurement; preview reads use the existing async store.
- **risks:** Remote/media preview, live Gateway, packaged Windows and source-paired acceptance remain open.
- **follow_up:** Continue the agent-workflow backlog without removing functional features.

## hermes-par0067-slash-keyboard-20261121

- **id:** hermes-par0067-slash-keyboard-20261121
- **timestamp:** 2026-11-21T23:16:55Z
- **what:** Added focused-composer arrow navigation and Enter/Tab insertion for the visible Hermes slash suggestion list; retained mouse selection and no-send semantics.
- **why:** Keyboard users need to select and stage tools/skills commands without leaving the message composer or accidentally submitting.
- **components:** Hermes native egui composer, slash-completion selection state, test coverage and evidence-backed progress maps.
- **type:** Additive accessibility and agent-workflow feature.
- **validation:** PASS rendered keyboard regression and full locked agent-automation suite (516 library + 15 protocol + 3 launcher); rustfmt and diff checks.
- **performance:** Selection is an O(1) index update across at most 24 visible entries.
- **risks:** Live Gateway, packaged Windows keyboard behavior and pinned-source paired acceptance remain unverified.
- **follow_up:** Add EV-0084 bound to the implementation commit, validate progress packages, publish to main, and continue the remaining PAR-0067 gaps.

## hermes-progress-review-date-preservation-20261121

- **id:** hermes-progress-review-date-preservation-20261121
- **timestamp:** 2026-11-21T23:23:04Z
- **what:** The implementation progress generator retains prior review dates for requirements whose evidence records have not changed.
- **why:** Routine regeneration must not make old evidence appear newly reviewed.
- **components:** Progress generator and broad/scoped evidence overlays.
- **type:** Evidence ledger correctness fix.
- **validation:** PASS both progress generators; repeated runs produced identical broad and scoped CSV hashes. PAR-0067 alone receives the current date because its evidence changed.
- **performance:** No runtime impact.
- **risks:** The date comparison is based on the exact evidence state, level, refs and gaps; manual edits to generated output are overwritten by the next generator run.
- **follow_up:** Continue preserving unchanged evidence dates on later progress refreshes.


## hermes-par0067-ev0084-20261121

- **id:** hermes-par0067-ev0084-20261121
- **timestamp:** 2026-11-21T23:23:04Z
- **what:** Added EV-0084 for the rendered slash-suggestion keyboard workflow and updated the capability crosswalk to describe the new interaction.
- **why:** Keep the user-visible P0 capability evidence-backed and distinguish implemented keyboard behavior from the remaining gaps.
- **components:** Native composer test evidence, fixed-source capability map and crosswalk.
- **type:** Partial implementation evidence.
- **validation:** PASS root UCM package validator with 68 implementation receipts and durable task validator at revision 578.
- **performance:** No runtime change.
- **risks:** Evidence is headless only; it does not claim live Gateway or physical Windows keyboard acceptance.
- **follow_up:** Publish the implementation and evidence commits to main, then continue the remaining agent-workflow gaps.

## hermes-par0067-main-publication-20261121

- **id:** hermes-par0067-main-publication-20261121
- **timestamp:** 2026-11-21T23:25:59Z
- **what:** Published implementation `979c970` and evidence `3ef5bb2` to `main`.
- **why:** Keep the verified agent-workflow improvement current in the user's canonical repository.
- **components:** Native composer, progress maps, pinned capability evidence, durable checkpoint.
- **type:** Direct main publication.
- **validation:** Fresh fetch confirmed local and origin main both equal `3ef5bb21222bee9ca37339a3ad57d8588c499648`; prior test, package and task validators passed.
- **performance:** No runtime benchmark needed; this changes only composer key interaction and progress bookkeeping.
- **risks:** PAR-0067 remains partial; live Gateway and physical Windows input are unverified.
- **follow_up:** Continue with another dependency-ready P0/P1 gap while preserving existing functional behavior.

### hermes-remote-startup-fail-closed-20261121

- **id:** hermes-remote-startup-fail-closed-20261121
- **timestamp:** 2026-11-21T23:42:51Z
- **what:** Saved remote Gateway startup now returns an actionable connection error if neither an endpoint-matched token nor OAuth session exists. It no longer starts the local Gateway while claiming the selected remote connection ID. Added EV-0085 and mapped PAR-0240 as partial in the capability and implementation status overlays.
- **why:** A remote connection with missing credentials must fail visibly and preserve connection ownership; silently reaching a different Hermes instance can misroute future UI actions.
- **components:** Hermes Native saved connection discovery, startup status, README, architecture, backlog, implementation plan, UCM evidence and progress overlays.
- **type:** Connection safety fix and regression coverage.
- **validation:** PASS focused regression; full locked `agent-automation` package suite (517 library + 15 protocol + 3 launcher), all-target locked cargo check, rustfmt, `git diff --check`, task validator and both capability-package validators. The root package validates 69 implementation receipts. YAML parsing was skipped because PyYAML is not installed.
- **performance:** Startup avoids creating the wrong local Gateway actor; no benchmark performed.
- **risks:** Live remote authentication and packaged Windows startup remain unverified. The saved-connection picker and complete multi-connection lifecycle are still open.
- **follow_up:** Record partial UCM evidence, publish the validated commits to `main`, then continue owner-safe connection switching and in-flight reattach proof.

### hermes-session-usage-toolbar-20261122

- **id:** hermes-session-usage-toolbar-20261122
- **timestamp:** 2026-11-22T00:11:51Z
- **what:** Added owner-scoped capture and display of Hermes session token/context usage in the active chat toolbar. Nested Desktop usage projections and top-level Gateway counters are parsed, sparse updates merge without erasing omitted values, and stale connection/profile/session events are rejected. Added unit and rendered UI regressions; updated README, architecture, backlog, plan and partial evidence receipt EV-0086.
- **why:** Session usage is an agent workflow capability; users need the active conversation's token/context state without crossing profiles or Gateway connections.
- **components:** Hermes Native event reducer, per-session chat view cache, chat toolbar, headless tests and feature documentation.
- **type:** Additive feature and regression coverage.
- **validation:** PASS focused usage tests (2); full locked `agent-automation` suite (519 library + 15 protocol + 3 launcher); all-target locked cargo check; rustfmt check; `git diff --check`; broad/scoped progress generators; root UCM validator (565 targets, 70 implementation receipts); pinned package validator (491 pending capabilities); task validator (revision 601 / ledger revision 95). Direct YAML parsing was skipped because PyYAML is unavailable.
- **performance:** Event capture is a bounded JSON projection and one unique owner lookup; no additional polling or RPC was added.
- **risks:** Live Gateway payload timing, disconnected-window UX, packaged Windows rendering and pinned-source pair acceptance remain unverified.
- **follow_up:** Continue with the next dependency-ready P0 agent-workflow capability from the pinned ledger; preserve existing functional work and keep live/source-paired proof separate.

### hermes-par0051-edit-safety-audit-20261121

- **id:** hermes-par0051-edit-safety-audit-20261121
- **timestamp:** 2026-11-22T00:22:02Z
- **what:** Audited the pinned prior-message edit workflow and native Gateway path. Recorded a safe implementation boundary in PLAN.md Revision 297 and selected PAR-0051 as the next chat workflow. No functional source changed in this audit.
- **why:** Editing earlier prompts truncates transcript history and must not lose drafts, attachments, or unconfirmed later turns.
- **components:** Fixed-source workflow references, native chat/Gateway task plan and durable handoff state.
- **type:** Read-only design audit and next-task checkpoint.
- **validation:** PASS durable task-state validation and `git diff --check`; no code tests run because no product code changed.
- **performance:** No runtime change.
- **risks:** The prior-message editing implementation and its destructive confirmation behavior remain incomplete.
- **follow_up:** Implement stable-row targeting, preserve drafts/attachments, confirm deep cuts, revalidate after confirmation, and add owner-fenced protocol/render tests.

- **id:** hermes-par0051-prior-message-edit-20261122
- **timestamp:** 2026-11-22T00:40:15Z
- **what:** Added an edit dialog for uniquely identified persisted user rows, safe deep-cut confirmation, post-confirm owner/target revalidation, native prompt.submit truncation parameters, optimistic pending-turn integration, and focused rendered/Gateway tests.
- **why:** Support the pinned Hermes prior-message edit-and-resubmit workflow while preserving the user's current draft and attachments and preventing stale or unconfirmed history truncation.
- **components:** Native session transcript, message edit dialog, Gateway command/RPC payload, focused tests, task plan and backlog.
- **type:** Additive agent chat workflow implementation.
- **validation:** PASS focused protocol payload, rendered dialog, stale-turn revalidation, owner/target submission and admission-gated history update tests; full locked suite (523 library + 15 protocol + 3 launcher); all-target cargo check; rustfmt; diff check; durable task-state validation; root UCM package validation (565 behaviors, 15 source inspections, 71 receipts). Optional YAML parsing was skipped because PyYAML is unavailable. Live Gateway, Windows package and pinned-source acceptance remain open.
- **performance:** No benchmark-sensitive hot path changed; an edit control is rendered only for transcript user rows with a unique positive row_id.
- **risks:** Refusal leaves the existing transcript untouched and uncertain admission keeps the old transcript while awaiting recovery; live recovery, in-flight response interruption and historical attachment semantics need more coverage.
- **follow_up:** Finish full validation, record evidence and task checkpoint, then publish this slice to main with all existing functional work preserved.

- **id:** hermes-par0051-edit-running-turn-20261122
- **timestamp:** 2026-11-22T00:54:01Z
- **what:** Extended prior-message editing to admitted active turns by waiting for the matching owner-scoped interrupt receipt before prompt truncation. Unknown interrupt/edit outcomes preserve history and do not use ordinary-message retry; definite refusal restores edit text while retaining the active composer draft and attachments.
- **why:** Match the pinned edit-during-stream lifecycle without risking a stale owner, dropping local history, or replaying a destructive edit with plain-submit parameters.
- **components:** Transcript edit control, edit/interrupt follow-up state, Gateway interruption receipt handling, prompt admission lifecycle and headless regressions.
- **type:** Additive workflow extension and recovery safety.
- **validation:** PASS focused active-turn interrupt ordering, uncertain no-send, refusal composer preservation, stale-turn revalidation and edit admission tests; full locked suite (526 library + 15 protocol + 3 launcher); all-target cargo check; rustfmt; diff check; progress generators; durable task validation; root package validation (565 behaviors, 15 source inspections, 72 receipts); archival package validation (491 pending capabilities). PyYAML is unavailable, so root YAML parsing was skipped. Live Gateway, Windows package and pinned-source acceptance remain open.
- **performance:** No benchmark-sensitive path changed; only one per-session active pending record and one edit continuation are tracked.
- **risks:** Pinned stop-cooldown and busy 4009 retry behavior, historical attachment semantics, live Gateway, packaged Windows and source-paired acceptance remain open.
- **follow_up:** Complete full validation, add EV-0088 bound to the implementation commit, update progress and publish to main.

- **id:** hermes-par0051-busy-retry-20261122
- **timestamp:** 2026-11-22T01:20:53Z
- **what:** Preserved Gateway JSON-RPC error code 4009 and added one owner-scoped interrupt-and-retry for definitive busy refusals while editing a prior message. A retry flag prevents loops; a second refusal restores the edit and preserves transcript history.
- **why:** Match Hermes' recent-stop cooldown behavior without dropping truncation parameters, replaying uncertain work, or overwriting the user's composer.
- **components:** Gateway JSON-RPC refusal handling, prior-message edit lifecycle, regression tests, completion docs, and progress evidence.
- **type:** Additive agent chat workflow recovery.
- **validation:** PASS focused Gateway and app lifecycle regressions; full locked suite (528 library + 15 protocol + 3 launcher); all-target check; rustfmt check; diff check. Live Gateway, packaged Windows, and pinned-source acceptance remain open.
- **performance:** No polling or additional RPC on successful edits; one interrupt and one retry occur only after a definitive Gateway busy refusal.
- **risks:** Historical attachment behavior during prior-message edits and live Gateway response timing remain unverified.
- **follow_up:** Bind partial evidence to the implementation commit, refresh generated progress, validate the durable task state, and publish to main while preserving all existing work.

- **id:** hermes-par0051-busy-retry-evidence-20261122
- **timestamp:** 2026-11-22T01:23:57Z
- **what:** Added EV-0089 for the bounded Gateway 4009 edit recovery, bound to implementation commit f104e1b4ad20217a3cc96a1b958db106b446935e, and refreshed broad/scoped progress overlays.
- **why:** Keep implementation, automated regression evidence, and partial capability status linked to an immutable code revision.
- **components:** Capability evidence package, progress overlays, and durable task checkpoint.
- **type:** Partial evidence and documentation update.
- **validation:** PASS root package (565 behavior rows, 15 source inspection receipts, 73 implementation receipts); archival package (491 pending capabilities); task-state validation. Evidence coverage remains 95/499 broad and 61/176 chat/tool; source/live acceptance remains pending.
- **performance:** No runtime change.
- **risks:** Receipt validator initially rejected a short SHA; EV-0089 now uses the full SHA and both package validators pass. PyYAML remains unavailable, so the root validator skipped direct YAML parsing.
- **follow_up:** Publish the implementation and evidence commits to main, then continue the attachment-semantics and live-Gateway audit.

- **id:** hermes-par0051-attachment-source-audit-20261122
- **timestamp:** 2026-11-22T01:29:49Z
- **what:** Audited pinned edit/submit code and clarified that prior attachment parts are dropped by the source when an edit replaces a message. Updated the plan and progress gaps: native currently matches replacement semantics; newly dropped files in the native edit dialog remain unsupported.
- **why:** Resolve the historical-attachment question from the fixed source without adding behavior the source does not have, while keeping the real edit-upload gap visible.
- **components:** Pinned source locators, PAR-0051 plan/backlog/completion docs, architecture, README and the generated progress source map.
- **type:** Read-only source audit and status correction.
- **validation:** PASS exact-source inspection at `rewind.ts#planEdit`, `rewind.ts#runRewindSubmit`, `user-edit-composer.tsx#handleDrop`, and `methods_prompt.py#_truncate_history_for_submit`. No product code changed and no tests were run. Process inspection found no running Hermes Gateway, so live validation could not run.
- **performance:** No runtime change.
- **risks:** New file upload from the edit dialog, live Gateway behavior, packaged Windows behavior and formal source acceptance remain open.
- **follow_up:** Continue with edit-dialog file upload or another dependency-ready agent-workflow gap; preserve the existing composer attachment path.

- **id:** hermes-par0051-edit-attachments-20261122
- **timestamp:** 2026-11-22T01:43:14Z
- **what:** Added image/file picker and drop support to the separate prior-message edit dialog. Upload receipts are scoped to the edit's session/profile/Gateway connection, references are composed into the replacement text, and submission remains blocked until uploads succeed. The current main-composer draft and attachments remain isolated.
- **why:** Match the pinned edit composer's ability to stage new files/images while preserving its text-only replacement semantics for old message parts.
- **components:** Native edit dialog, session attachment upload event fencing, prompt composition, Gateway test projections, headless regressions, progress and docs.
- **type:** Additive chat attachment workflow.
- **validation:** PASS full locked suite (530 library + 15 protocol + 3 launcher); all-target check and diff check. `cargo fmt --check` and focused UI validation are being rerun after the final rendered-control assertion; live Gateway and physical Windows picker/drop behavior remain unverified.
- **performance:** Upload requests occur only for files explicitly chosen or dropped in the edit dialog; no new polling was added.
- **risks:** A canceled in-flight upload may leave an unreferenced file in the session's attachment store, as with the main composer. Runtime ownership is fenced, and the prompt is never sent until all retained uploads have receipts.
- **follow_up:** Rerun focused and full validation after the rendered-control assertion, add commit-bound partial evidence, update task state, and publish to main. Preserve all existing functionality.

- **id:** hermes-par0051-edit-attachments-evidence-20261122
- **timestamp:** 2026-11-22T01:45:15Z
- **what:** Added EV-0090 for the edit-dialog attachment workflow, bound to the implementation commit, and regenerated broad/scoped progress.
- **why:** Keep the new upload behavior, headless evidence, and capability map connected to the exact implementation revision.
- **components:** Evidence package, generated progress overlays, and task checkpoint.
- **type:** Partial implementation evidence.
- **validation:** PASS root capability package (565 behaviors, 15 source inspections, 74 implementation receipts), archival package (491 pending), and task-state validation. Progress remains 95/499 broad and 61/176 chat/tool; formal and live acceptance remain pending.
- **performance:** No runtime change.
- **risks:** Optional direct YAML parsing remains skipped because PyYAML is unavailable. Live Gateway and physical Windows interaction remain unverified.
- **follow_up:** Checkpoint the current test/evidence state and publish implementation plus EV-0090 to main.


## PAR-0052 — Regenerate a previous assistant answer — 2026-11-22

- **id:** PAR-0052
- **timestamp:** 2026-11-22T02:28:14Z
- **what:** Added an accessible Regenerate answer action for assistant messages when the nearest preceding user prompt has a unique positive durable row ID. The read-only confirmation dialog requires deep-cut consent, leaves the active composer untouched, submits the owner-scoped Gateway truncation request, and remaps surviving transcript IDs only from the admission receipt; missing replacements remove stale IDs.
- **why:** Complete a core agent chat lifecycle action using the pinned Hermes snapshot while preserving existing chat and edit behavior.
- **components:** `apps/hermes-native/src/app.rs`, `src/gateway.rs`, progress generator/ledgers, `README.md`, architecture, plan, backlog, completion overview, and project journals.
- **type:** feature, Gateway receipt mapping, headless UI/protocol regressions, progress evidence and documentation.
- **validation:** Full locked suite passed (537 library, 15 protocol, 3 launcher); all-target locked check, rustfmt, diff check, both progress generators, Doofus task validator and pinned-package validator passed (74 implementation receipts). The optional YAML parse was skipped because PyYAML is unavailable. PAR-0052 remains partial; live Gateway behavior, packaged Windows interaction and formal pinned-source acceptance are unverified.
- **performance:** No benchmark run; row selection and local survivor rebinding are linear in the loaded transcript length, and one truncating submit RPC is issued after consent.
- **risks:** Hermes rewrites durable rows during truncation. If a receipt lacks a valid replacement row ID, subsequent rewind on that cached row is disabled until history refresh; the feature does not infer IDs.
- **follow_up:** Verify live Gateway admission and transcript reconciliation in an isolated session, then continue the next agent-workflow gap.


### PAR-0052 publication record

- Behavior commit `759b4a49153deddbd56cb4ae5ca05b1e2a0721c9`; partial evidence receipt EV-0091 and generated overlays committed in `c2a05b656bb8e5aeb9fe25d4374f1699b43127cf`.
- Both commits were pushed to `main`; fresh fetch confirmed local `main` and `origin/main` equal `c2a05b656bb8e5aeb9fe25d4374f1699b43127cf`.
- Coverage now records 97/499 broad requirements and 63/176 scoped chat/tool requirements with mapped evidence. All live-runtime and source-paired acceptance remains pending. `apps/hermes-native/work/` was excluded and preserved.


## Doofus task Git status parsing — 2026-11-22

- **id:** HERMES-TASK-GITSTATUS-2026-11-22
- **timestamp:** 2026-11-22T02:38:06Z
- **what:** Preserve the leading status column from `git status --porcelain` so a leading dot in `.doofus/` paths is not stripped; task progress no longer mistakes Doofus state files for ordinary worktree drift.
- **why:** Keep portable handoff checkpoints accurate after publication while retaining clear reporting of operator-owned work.
- **components:** `apps/hermes-native/tools/doofus_task.py`, `apps/hermes-native/tools/test_doofus_task.py`, project journals.
- **type:** continuity tooling fix and regression test.
- **validation:** PASS all 19 isolated task continuity tests; post-fix checkpoint and task validator remain to run.
- **performance:** Adds no repository-scale scans; preserves the existing single porcelain status call.
- **risks:** None beyond platform-specific porcelain parsing, covered by isolated repository regression.
- **follow_up:** Recheckpoint after publishing this correction and confirm `git_drift` is false with the operator work folder preserved.

## PAR-0051 — Recover stale runtime for prior-message edits — 2026-11-22

- **id:** hermes-par0051-stale-runtime-edit-recovery-20261122
- **timestamp:** 2026-11-22T03:02:49Z
- **what:** Prior-message edits now resume the captured stored session after a definitive `session not found` submit refusal, publish a connection-scoped runtime rebind before retry, and retry the same truncating payload at most once. A mismatched stored-session owner fails closed without rebind or retry. The app retargets only the matching connection/profile/session view and pending edit before processing the retry receipt.
- **why:** Match the pinned Hermes shared rewind-submit recovery while preserving edit safety, attachments, truncation consent and transcript admission gating.
- **components:** Native Gateway edit command/actor and typed events; app owner-state reconciliation; WebSocket and app tests; progress overlay; plan, backlog, completion overview, architecture, README and journals.
- **type:** Additive agent-chat lifecycle recovery with fail-closed owner fencing.
- **validation:** PASS focused recovery, wrong-owner, and app owner-rebind tests; PASS full locked suite (540 library + 15 protocol + 3 launcher); PASS all-target locked check, rustfmt, diff check and progress generators. Pinned-package and continuity validators are recorded in execution logs. Live Gateway, packaged Windows picker/drop behavior and pinned-source acceptance remain unverified.
- **performance:** No new polling. Only an explicit stale-runtime refusal adds one `session.resume` and one bounded `prompt.submit` retry.
- **risks:** Live Hermes Gateway versions may differ in resume response fields; a failed or uncertain resume never triggers a second prompt submission. Live runtime remains unverified.
- **follow_up:** Bind local evidence to the implementation commit and continue PAR-0051 toward Gateway and packaged Windows acceptance without touching `apps/hermes-native/work/`.

### PAR-0051 stale-runtime recovery publication

- Implementation commit `3a288f867af28b8826cf63f4f7e0a84a9c4eadae` and EV-0092/progress commit `d9882a5a6e187e62dc20e6f17c8e4c3589a488b5` were pushed to `origin/main`; a fresh fetch confirmed local `main` and `origin/main` match exactly.
- Task `hermes-par-0051` remains in progress under the active Codex lease. Checkpoint revision 640 / task-ledger revision 102 validates with `git_drift: false`, no unresolved operations, and the operator-owned `apps/hermes-native/work/` folder preserved.


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


### PAR-0060 composer draft persistence — 2026-11-22

Added bounded atomic text-draft storage and protected nonempty draft views from LRU eviction. Validation passes: 556 library + 15 protocol + 3 launcher tests, all-target check, rustfmt and diff checks. Attachments remain session-memory-only; live Gateway and packaged Windows acceptance remain open.


PAR-0060 was committed on main as 32d59b04453134876562ab4de8a87f70d0167245; partial evidence EV-0095 is bound to that code commit. Task checkpoint revision 655 records the passed 556 + 15 + 3 suite and remaining packaged/source-paired acceptance.


PAR-0060 is published to `origin/main`; fresh fetch confirms main at 618403c88fd5979659937bd7c4571ba44d5fc4ae. Operator-owned runtime residue remains preserved.


PAR-0061 adds native model-change safety evidence: composer text and attachments survive success/refusal/confirmation outcomes, and stale profile receipts do not consume the active model mutation. Headless full suite passes; live Gateway and packaged Windows acceptance remain open.


PAR-0061 partial receipt EV-0096 is bound to implementation commit 8c19b9cf5ec5f242dd2d31201176b1a230dafc24. The task checkpoint records full headless validation; live and source-paired acceptance remain open.


PAR-0061 implementation and EV-0096/checkpoint are published to `origin/main`; fresh fetch verified `10968d89f501073e1521751e2d07153e3953a633`.

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

### PAR-0055 session-scoped transcript read position — 2026-11-22T06:46:20Z

- **id:** `hermes-par0055-implementation-evidence-20261122`
- **timestamp:** 2026-11-22T06:46:20Z
- **what:** Added bottom-intent or distance-from-bottom scroll state to each existing profile/connection/session-owned chat view. The active view carries it forward when the chat cache is saved. Added headless owner-isolation, threshold, restore and content-extent transformation coverage; mapped PAR-0055 partial with EV-0101.
- **why:** Pinned Hermes keeps transcript reading position per session, distinct from its follow-bottom behavior. Existing work remains intact.
- **components:** `apps/hermes-native/src/app.rs`, progress generator and CSV overlays, `apps/hermes-native/docs/PLAN.md`, `BACKLOG.md`, `COMPLETION_OVERVIEW.md`, capability map/evidence.
- **type:** feature and regression evidence.
- **validation:** focused regression; 564 library + 15 protocol + 3 launcher tests; all-target locked check; rustfmt/diff; 19 task continuity tests; task validation; both progress generators; root capability validator (84 implementation receipts before EV-0101) and archival validator (491 pending).
- **performance:** No benchmarked performance change; state calculations are constant time per rendered transcript.
- **risks:** Partial in-process behavior only; no restart persistence, explicit older-history prepend anchoring, physical Windows input, live Gateway, packaged Windows or formal source-paired acceptance. Optional PyYAML/UCM YAML parse skipped. Existing `handle_server_event` dead-code warning remains.
- **follow_up:** Bind EV-0101 to implementation commit `dee655dbaf82017c2b15eef903800448b37ef6b2`, validate evidence/map, publish to main, then continue the next dependency-ready agent workflow. Preserve all functional work.

### PAR-0055 published to main — 2026-11-22T06:47:57Z

Implementation commit `dee655dbaf82017c2b15eef903800448b37ef6b2` and evidence/task checkpoint commit `2f64e4821668f42fe6f9c9d84e1fadbcf8632655` were pushed to `origin/main`. Fresh fetch confirmed local and remote `main` exactly equal at `2f64e4821668f42fe6f9c9d84e1fadbcf8632655`. PAR-0055 remains partial. The operator-owned `apps/hermes-native/work/` was left untracked and untouched.

### PAR-0056 prompt timeline source audit — 2026-11-22T06:49:48Z

- **id:** `hermes-par0056-source-audit-checkpoint-20261122`
- **timestamp:** 2026-11-22T06:49:48Z
- **what:** Selected PAR-0056 and audited the pinned timeline component, rail, data helpers and viewport position reader. Checkpointed the task in progress with the exact next implementation slice.
- **why:** Prompt timeline navigation is a P0 chat workflow in the pinned source; existing native search and older-page controls do not provide a timeline rail.
- **components:** `.doofus/handoff.json`, `.doofus/task-ledger.json`, `.doofus/events.jsonl`.
- **type:** source audit and task-state transition.
- **validation:** `doofus_task.py validate` passed at state revision 691 / ledger revision 123. No product code changed.
- **performance:** No runtime behavior changed.
- **risks:** Native has no indexed prompt timeline/history window yet; no behavior was claimed as implemented.
- **follow_up:** Add an owner-scoped prompt timeline for loaded messages, then separately account for historical-index loading and installation/runtime validation.

### PAR-0056 checkpoint revision correction — 2026-11-22T06:50:25Z

The prior source-audit changelog entry mistakenly stated ledger revision 123. The successful validator returned state revision 691 and ledger revision 122. The correction is appended here; the original audit record remains unchanged.

### PAR-0056 loaded-prompt timeline — 2026-11-22T07:03:45Z

- **id:** `hermes-par0056-implementation-evidence-20261122`
- **timestamp:** 2026-11-22T07:03:45Z
- **what:** Added a collapsible and keyboard-accessible timeline of real user prompts in the loaded transcript. Entries are virtualized and scroll identity is keyed to the profile/connection/session owner. Selecting a marker scrolls the transcript row and does not submit. Empty prompts and pinned background-process notices are excluded. Added widget and full-app egui interaction tests; recorded PAR-0056 as partial with EV-0102.
- **why:** Prompt navigation is an agent-chat workflow in the pinned Desktop source. It extends existing transcript search, history and bottom-follow behavior without replacing those features.
- **components:** `apps/hermes-native/src/app.rs`, progress generator and overlays, `apps/hermes-native/docs/PLAN.md`, `BACKLOG.md`, `COMPLETION_OVERVIEW.md`, capability map/evidence.
- **type:** feature and regression evidence.
- **validation:** focused widget and app interaction tests; 566 library + 15 protocol + 3 launcher tests; all-target locked check; rustfmt/diff; 19 continuity tests; task validator; both progress generators; root package validator (86 implementation receipts) and archival validator (491 pending).
- **performance:** Timeline rows are virtualized with `show_rows`; entry generation happens only while expanded.
- **risks:** Partial only: unloaded history indexing/around-row fetch and active-marker tracking remain absent. No physical Windows, packaged, live Gateway or formal source-paired acceptance. Optional PyYAML/UCM parse skipped.
- **follow_up:** Publish to main; next add the missing historical-index path while preserving this UI and existing transcript controls.

### PAR-0056 published to main — 2026-11-22T07:05:52Z

Implementation commit `78f1aa008a5f00a7d6dd8c2d1568d72302d31534` and evidence/checkpoint commit `5db2e8d55032819077660f5d6caa20f4407d5738` are on `origin/main`. Fresh fetch confirmed exact local/remote equality at `5db2e8d55032819077660f5d6caa20f4407d5738`. The task remains partial: navigation covers loaded prompts, with indexed older-history navigation and active-marker tracking still open. The operator-owned `apps/hermes-native/work/` remains untracked and untouched.

### PAR-0056 indexed prompt history — 2026-11-22T07:27:17Z

- **id:** `hermes-par0056-indexed-history-20261122`
- **timestamp:** `2026-11-22T07:27:17Z`
- **what:** Added a profile/connection-scoped metadata-only prompt-index API path and chat index UI. Indexed markers can select a loaded durable row or trigger older transcript pagination until an unloaded row is found; the automatic seek is capped at 64 pages. Added response ownership/cursor validation, a 12-session cache cap, remote HTTP fixture, rendered accessible marker test, and evidence EV-0103. PAR-0056 remains partial.
- **why:** The first timeline slice only navigated loaded prompt rows; the pinned source indexes prompt previews independently from full transcript bodies.
- **components:** `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/src/app.rs`, `apps/hermes-native/docs/ARCHITECTURE.md`, `PLAN.md`, `BACKLOG.md`, progress overlays, capability map and evidence.
- **type:** feature and regression evidence.
- **validation:** Full locked suite: 571 library + 15 protocol + 3 launcher tests; all-target check; rustfmt and diff checks; 19 task-continuity tests; task validation; both progress generators; root and archival package validators. The full UCM checker parses the map with Python 3.14/PyYAML but reports 159 pre-existing project-wide errors and 11 reconciliation items; none were added for EV-0103. Python 3.15 lacks PyYAML, so its package-validator YAML parse is skipped.
- **performance:** Timeline API payload is bounded to 500 metadata entries and the UI list is virtualized. Cached session indexes are capped at 12; an unloaded jump performs at most 64 existing 120-row transcript reads.
- **risks:** Automatic history seek is not a substitute for the pinned `/messages/around` window route. Its complete end-to-end path, refreshed cache after new prompts, active-marker tracking, authenticated live Gateway, packaged Windows and formal source-paired acceptance remain open. Existing `handle_server_event` dead-code warning remains.
- **follow_up:** Replace sequential older-page scans with the pinned around-row endpoint; add safe index freshness/refresh and source-matched live behavior checks.

### PAR-0056 indexed history published — 2026-11-22T07:37:18Z

Implementation commit `ea90619cc3b924c80960844f37ba8ecf082fff48` and evidence/documentation commit `041b3604412b1ee48392c49cd699798cccc4b7c0` are published on `origin/main`. Fresh fetch verified local and remote `main` exactly match at `041b3604412b1ee48392c49cd699798cccc4b7c0`. The post-publication task checkpoint is revision 699 / ledger 124, PAR-0056 remains partial, and `apps/hermes-native/work/` remains untracked and untouched.

### PAR-0056 around-row history window — 2026-11-22T07:54:26Z

- **id:** `hermes-par0056-around-window-20261122`
- **timestamp:** `2026-11-22T07:54:26Z`
- **what:** Added the pinned bounded `/messages/around` history read, strict owner/anchor/pagination validation, owner-scoped transcript replacement and explicit prompt-index refresh. Existing chat timeline and older-message controls remain available. Added Gateway and app ownership regressions.
- **why:** Indexed historical prompts need a direct path into unloaded transcript history without scanning up to 64 older pages or risking cross-connection transcript replacement.
- **components:** `apps/hermes-native/src/app.rs`, `apps/hermes-native/src/gateway.rs`, `apps/hermes-native/docs`, `apps/hermes-native/tools/build_implementation_progress.py`.
- **type:** additive feature and regression evidence.
- **validation:** Focused Gateway tests pass (2); app event ownership tests pass (2). Full workspace tests, package validators, generated evidence and publication remain pending.
- **performance:** A selected history jump is capped at 120 messages and does not fetch intervening payloads through sequential pages.
- **risks:** Automatic cache invalidation and active-marker tracking remain open. Authenticated live Gateway, packaged Windows, physical input and formal pinned-source acceptance remain unverified.
- **follow_up:** Run complete validation, append commit-bound capability evidence, publish on `main`, then verify the canonical task state and remote tip.

Validation follow-up — 2026-11-22T07:57:56Z: Full locked test suite passes (575 library + 15 protocol + 3 launcher); all-target cargo check, rustfmt/diff check, 19 task continuity tests, generated implementation/chat progress, and root plus archival package validators pass. PAR-0056 remains partial pending automatic cache invalidation, active-marker tracking and live/package/source-paired validation.

Publication follow-up — 2026-11-22T08:03:48Z: Implementation commit `3071c0984337897630c9551d9129267876d82817` and evidence commit `1bdf82017eb42062c1ae2d0fa008338f9d7c5d29` are published on `origin/main`. Fresh fetch verified exact local/remote equality at `1bdf82017eb42062c1ae2d0fa008338f9d7c5d29`. `EV-0103` is preserved unchanged and `EV-0104` is additive. The operator-owned `apps/hermes-native/work/` remains untouched.


### PAR-0056 prompt history freshness and active marker — 2026-11-22T08:20:46Z

- **id:** `hermes-par0056-history-freshness-active-marker-20261122`
- **timestamp:** `2026-11-22T08:20:46Z`
- **what:** Admitted prompt changes now invalidate the matching metadata index. Refresh preserves existing markers while fetching a new first page, then replaces them. The loaded timeline and indexed history expose a Current marker selected from transcript viewport position and bottom-follow state. Added focused headless regressions and additive EV-0105 capability evidence.
- **why:** Keep historical prompt navigation useful after sends, edits and regenerations while showing which prompt the reader is currently viewing.
- **components:** `apps/hermes-native/src/app.rs`, `apps/hermes-native/docs/PLAN.md`, `apps/hermes-native/docs/BACKLOG.md`, `apps/hermes-native/docs/ARCHITECTURE.md`, `docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/maps/capabilities.yaml`, `docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/maps/capabilities/evidence/EV-0105.json`.
- **type:** additive behavior and regression evidence.
- **validation:** Targeted timeline tests pass (7); active viewport test passes (1); all-target locked cargo check passes. Full suite and package validators are pending.
- **performance:** Refresh is a single metadata first-page request after local admission; existing rows remain visible. Viewport tracking examines the already-rendered transcript frames and adds no Gateway reads.
- **risks:** Time-based expiry and mutations from other clients are not detected. Live authenticated Gateway, packaged Windows, physical scroll input and formal pinned-source acceptance remain open.
- **follow_up:** Run full validation, bind EV-0105 to the tested implementation commit, regenerate coverage, checkpoint task state and publish to `main`.


Validation follow-up — 2026-11-22T08:27:07Z: PAR-0056 freshness/active-marker regressions pass. The full locked Rust suite passes 578 library + 15 protocol + 3 launcher tests; all-target locked check, rustfmt, 19 task-continuity tests, both progress generators, canonical task validation, and the archival package validator (491 pending) pass. The root evidence package validator awaits EV-0105's implementation commit binding. Existing `handle_server_event` dead-code warning remains.


Publication follow-up — 2026-11-22T08:29:04Z: Implementation commit `b65bbc463e521ade6c9fd852dbc34a08aad36837` and evidence/checkpoint commit `9b7e34a13783047bebe273e5048cce2dbf548362` were pushed to `origin/main`. Fresh fetch verified exact local/remote equality at `9b7e34a13783047bebe273e5048cce2dbf548362`. EV-0105 is bound to the implementation commit; EV-0102 through EV-0104 remain unchanged. The operator-owned `apps/hermes-native/work/` remains untracked and untouched.


### PAR-0057 older around-window pagination — 2026-11-22

- **id:** `hermes-par0057-around-pagination-20261122`
- **timestamp:** `2026-11-22T08:51:18Z`
- **what:** “Show earlier” on a prompt-centered history window now finds the nearest predecessor from the shared prompt index, loads further index pages when needed, and requests a bounded around-row window. It merges only contiguous pages and replaces the view for a separated older window.
- **why:** Keep prompt navigation and history paging on one durable row index, and avoid displaying nonadjacent message windows as a continuous transcript.
- **components:** `apps/hermes-native/src/app.rs`, `apps/hermes-native/README.md`, `apps/hermes-native/docs/PLAN.md`, `apps/hermes-native/docs/BACKLOG.md`, `apps/hermes-native/docs/ARCHITECTURE.md`, progress overlays and capability evidence.
- **type:** additive behavior and regression coverage.
- **validation:** PASS focused predecessor/index/gap tests; PASS full all-target locked suite (582 library + 15 protocol + 3 launcher); check, package, task-continuity and publication validation pending.
- **performance:** At most one metadata page is requested per response until the shared index reaches the visible prompt; transcript reads remain bounded to 120 messages.
- **risks:** REST reads cannot be cancelled; request ownership fences stale results but explicit cancel-and-replace and physical scroll-anchor preservation remain open. Live Gateway, packaged Windows and source-paired acceptance are unverified.
- **follow_up:** Run all-target check and continuity/package validators; bind EV-0106 to the implementation commit while keeping the claim partial; checkpoint and publish to `main`.

Validation follow-up — 2026-11-22T08:55:18Z: PASS full `cargo test -p hermes-native --all-targets --features agent-automation --locked` (582 library + 15 protocol + 3 launcher); PASS matching all-target `cargo check`; PASS rustfmt/diff; PASS task-continuity suite (19 tests); PASS both progress generators, task validation and root/archival package validators. The root validator skipped YAML parsing because PyYAML is absent, and EV-0106 remains to be bound to the implementation commit. Live Gateway, packaged Windows and source-paired acceptance are still open.

Evidence follow-up — 2026-11-22T08:57:45Z: EV-0106 is bound to implementation commit `b24bd31861cf749545f7bb2345a2145cc598b80d`, linked from the PAR-0057 capability, and keeps its overall status `partial`. The root evidence package now validates 90 implementation receipts; the archival package remains 491 pending. YAML syntax parsing remains skipped by the root validator because PyYAML is absent.

Publication follow-up — 2026-11-22T08:59:23Z: Implementation commit `b24bd31861cf749545f7bb2345a2145cc598b80d` and EV-0106 evidence commit `cf849723e3b1a631e981a1e081b64bb69dad2ab3` were pushed to `origin/main`. A fresh fetch confirmed exact local/remote main equality at `cf849723e3b1a631e981a1e081b64bb69dad2ab3`. The operator-owned `apps/hermes-native/work/` remains untracked and untouched.

## PAR-0521 — Clear withdrawn Gateway requests — 2026-11-22

- **id:** PAR-0521
- **timestamp:** 2026-11-22T09:08:54Z
- **what:** Added `request.cancel` event handling that removes only the approval or batch-clarification prompt whose server request ID matches the cancellation. When the event includes a profile, the prompt's profile must match too. Missing IDs and mismatches are no-ops.
- **why:** A withdrawn approval or clarification should no longer remain actionable in the native chat.
- **components:** `apps/hermes-native/src/app.rs`, progress generator and overlays, `apps/hermes-native/docs/PLAN.md`, `BACKLOG.md`, `ARCHITECTURE.md`, `COMPLETION_OVERVIEW.md`, `README.md`, root changelog and execution log.
- **type:** additive Gateway event handling, regression coverage, progress evidence and documentation.
- **validation:** PASS focused `request_cancel_clears_only_the_matching_approval_and_clarify_request`; full suite and publication pending.
- **performance:** Bounded map lookup by request ID; no network request or added polling.
- **risks:** Sensitive request card handlers are not implemented; live Gateway, packaged Windows and pinned-source acceptance remain open.
- **follow_up:** Run full locked validation, bind evidence to the implementation commit, and publish to `main`.

Validation follow-up — 2026-11-22T09:08:54Z: PASS focused request-cancel test; PASS full locked all-target suite (583 library + 15 automation protocol + 3 launcher), locked all-target check, rustfmt/diff checks, 19 task-continuity tests, both progress generators, canonical task-state validation and root/archival source package validators. The root validator skipped YAML parsing because PyYAML is absent. Live Gateway, packaged Windows and pinned-source acceptance remain open.

Evidence follow-up — 2026-11-22: EV-0107 binds the headless cancellation regression to implementation commit `d7751abbf377aada711e521305e685279e59ff1a`. The capability remains partial; the root package validates 91 implementation receipts. Live Gateway, packaged Windows and pinned-source acceptance remain open.

Publication follow-up — 2026-11-22T09:14:52Z: Implementation commit `d7751abbf377aada711e521305e685279e59ff1a` and evidence commit `57850ec9a1d3b7649824dfe72d84487753c56e57` were pushed to `origin/main`; fresh fetch confirmed exact local/remote equality at the evidence commit. The operator-owned `apps/hermes-native/work/` remains untracked and untouched.

## PAR-0522 — Preserve unknown Gateway response outcomes — 2026-11-22

- **id:** PAR-0522
- **timestamp:** 2026-11-22T09:35:25Z
- **what:** Added typed distinction between explicit JSON-RPC refusal and ambiguous transport/receipt outcomes for approval, clarification, and clarification-lock responses. Unknown outcomes remain attached to the prompt, survive disconnect, show an accessible warning, and disable duplicate responses. Explicit refusal resets the pending response so a safe retry remains available.
- **why:** A transport failure cannot prove that Hermes rejected a response; presenting it as a normal failure can lead to duplicate tool decisions or answers.
- **components:** `apps/hermes-native/src/gateway.rs`, `src/app.rs`, progress generator and overlays, `PLAN.md`, `BACKLOG.md`, `ARCHITECTURE.md`, `COMPLETION_OVERVIEW.md`, `README.md`, and root validation records.
- **type:** additive protocol outcome model, accessible UI state, regression coverage, evidence and documentation.
- **validation:** PASS full locked all-target suite (585 library + 15 protocol + 3 launcher), locked all-target check, rustfmt, 19 task-continuity tests, task schema validation, both progress generators, root and archival package validators. Root validator skipped YAML parsing because PyYAML is absent.
- **performance:** No added polling or network requests; outcome classification is performed on the existing Gateway request response path.
- **risks:** Uncertainty classification is not yet applied to every mutating Gateway operation. Live Gateway, packaged Windows, and formal pinned-source acceptance remain open.
- **follow_up:** Bind EV-0108 to the implementation commit and continue the next P0 protocol requirement.

Evidence follow-up — 2026-11-22T09:36:37Z: EV-0108 binds the unknown-outcome regression to implementation commit `6a4ad7afdb6fb22d0c206a9fb924beadb2292818`; the capability remains partial. Remaining mutating Gateway operations, live Gateway behavior, packaged Windows behavior and pinned-source acceptance are open.

Publication follow-up — 2026-11-22T09:37:33Z: Implementation commit `6a4ad7afdb6fb22d0c206a9fb924beadb2292818` and evidence commit `f0994d17476b896832ef3ad7f08a015994d6053f` were pushed to `origin/main`. A fresh fetch confirmed local and remote main equal `f0994d17476b896832ef3ad7f08a015994d6053f`.


### PAR-0523 — Scope Gateway prompts by connection and profile — 2026-11-22T10:05:31Z

- **id:** PAR-0523
- **timestamp:** 2026-11-22T10:05:31Z
- **what:** Carried Gateway connection and profile identity through approval, clarification, and batch-clarification prompt events, owner-qualified prompt maps, displayed controls, responses, receipts, and cancellation.
- **why:** The pinned Desktop binds server requests to their source connection and profile; reused IDs must not cross Gateway boundaries.
- **components:** hermes-native Gateway actor/event polling, chat prompt state and UI, progress overlays, architecture/plan/backlog/completion/README.
- **type:** correctness and ownership fencing.
- **validation:** PASS 586 library + 15 protocol + 3 launcher tests; all-target locked check; rustfmt/diff checks; 19 task continuity tests; root and archival package validators; both progress generators.
- **performance:** No benchmarked performance change; prompt maps remain bounded by the existing in-memory lifecycle.
- **risks:** Other mutating Gateway operations still need a full connection/profile audit. Authenticated live Gateway, packaged Windows behavior, and formal pinned-source acceptance remain unverified.
- **follow_up:** Add commit-bound EV-0109 and partial capability state, run the UCM/package/task checks, publish to main, then continue the next P0 workflow gap.

### PAR-0524 — Fence Gateway answer receipts by profile — 2026-11-22T10:22:13Z

- **id:** PAR-0524
- **timestamp:** 2026-11-22T10:22:13Z
- **what:** Added profile ownership to RespondServerRequest and ServerRequestAnswered; receipt application now requires the exact connection/profile/request tuple.
- **why:** Prevent a result receipt for a reused request ID from mutating another profile's approval or clarification prompt.
- **components:** pps/hermes-native/src/gateway.rs, pps/hermes-native/src/app.rs, architecture/plan/backlog/completion docs, implementation progress evidence builder.
- **type:** correctness fix
- **validation:** PASS 587 library + 15 protocol + 3 launcher tests; cargo check --all-targets --locked --features agent-automation; 19 task-continuity tests; package validator; progress builders; rustfmt and diff checks.
- **performance:** No benchmark applicable; only metadata fields and owner comparisons were added.
- **risks:** Other mutating Gateway operations remain unaudited; live authenticated Gateway, packaged Windows and pinned-source acceptance are not verified.
- **follow_up:** Continue the P0 stale-owner audit from the next dependency-ready request/result path.
Publication follow-up — 2026-11-22T10:26:30Z: Implementation 36c2da65a2c20d0049269a7b10cd58e42ec37a4c and evidence commit 31436dd6f964ec8428a205efbea53e03c401393f were pushed to origin/main; fresh fetch confirms local and remote main are equal.

### PAR-0525 — Expose profile secret redaction — 2026-11-22T10:46:10Z

- **id:** PAR-0525
- **timestamp:** 2026-11-22T10:46:10Z
- **what:** Added a native Settings Security tab that reads/saves Hermes' profile-scoped security.redact_secrets preference; unset values default on and explicit opt-out is retained.
- **why:** Secret redaction is an agent safety control and is part of the pinned Hermes profile configuration used by backend log formatting.
- **components:** Gateway config HTTP path, Settings UI, architecture/plan/backlog/completion docs, implementation progress evidence builder.
- **type:** feature and security preference port
- **validation:** PASS 590 library + 15 protocol + 3 launcher tests; all-target check; 19 continuity tests; root and archival package validators; progress generators; rustfmt and diff check.
- **performance:** No benchmark applicable; one small profile-scoped config read/write was added on explicit Settings entry/save.
- **risks:** Pinned pytest could not start because  ruamel.yaml is missing in the available Python environment; live Gateway and packaged Windows behavior remain unverified.
- **follow_up:** Add commit-bound UCM evidence and continue with the next agent-workflow gap.

### PAR-0525 evidence — 2026-11-22

- **id:** PAR-0525-EV-0111
- **timestamp:** 2026-11-22T11:00:22Z
- **what:** Added EV-0111 for the profile-scoped log-redaction preference scenario and linked it to the capability map as partial evidence bound to implementation commit 9d7ffe786e35126c2f2335ce9720fada0d11a0a9.
- **why:** Preserve auditable, commit-specific proof without claiming the live Hermes formatter or packaged runtime was verified.
- **components:** UCM capabilities map and evidence, Doofus task checkpoint, execution log.
- **type:** evidence and progress record
- **validation:** EV-0111 has no evidence-specific schema or reconciliation findings; source package validator passes 565 behaviors, 15 source inspection receipts, 95 implementation receipts; task validator passes revision 754. The UCM checker still reports 162 legacy errors and 11 prior reconciliation items.
- **performance:** No runtime behavior changed.
- **risks:** Pinned Hermes pytest could not execute because ruamel.yaml is missing; live Gateway and packaged Windows behavior remain unverified.
- **follow_up:** Commit evidence and task state, publish PAR-0525 to origin/main, then continue the next dependency-ready agent workflow gap.

### PAR-0525 publication — 2026-11-22

- **id:** PAR-0525-PUBLISH
- **timestamp:** 2026-11-22T11:02:15Z
- **what:** Published implementation commit 9d7ffe786e35126c2f2335ce9720fada0d11a0a9 and evidence commit 2d3631aaf4d20cb99722a329ece5bfd4a54be1e5 to origin/main.
- **why:** Make the completed tested slice and its capability proof available on the requested canonical branch.
- **components:** Hermes native Settings, Gateway config, UCM map, task checkpoint.
- **type:** publication
- **validation:** Fresh fetch confirms local main and origin/main both equal 2d3631aaf4d20cb99722a329ece5bfd4a54be1e5.
- **performance:** No runtime changes in the publication step.
- **risks:** The capability remains partial pending pinned formatter tests and live Gateway/package acceptance.
- **follow_up:** Continue with PAR-0526, distinguishing permission rejection from network failure.

### PAR-0526 — Distinguish permission rejection from network error — 2026-11-22

- **id:** PAR-0526
- **timestamp:** 2026-11-22T11:16:03Z
- **what:** Extended the chat transcript error presenter to use Hermes' structured error layer/code for policy and safety refusals, rejected credentials, provider timeouts, unreachable custom endpoints, and dropped streams. Added a rendered headless regression beside existing model-not-found, partial output, and Continue coverage.
- **why:** Users need a clear explanation of whether Hermes or a provider refused the request or the network failed, so they can choose the right recovery.
- **components:** assistant transcript error presentation, implementation progress generator, README, architecture, backlog, plan, completion overview, generated progress CSVs.
- **type:** feature and headless UI regression
- **validation:** PASS full locked suite (590 library, 15 automation protocol, 3 launcher); focused rendered test; cargo check all targets; 19 task continuity tests; task ledger validator; source package validator (565 behaviors, 15 inspection receipts, 95 implementation receipts); progress generators; rustfmt and diff checks.
- **performance:** No runtime request or rendering work was added; the presenter reads fields already in each transcript row.
- **risks:** Live authenticated error-surface events, packaged Windows interaction and formal pinned-source acceptance remain unverified.
- **follow_up:** Add commit-bound partial UCM evidence, publish PAR-0526 to main, and continue the next dependency-ready agent-workflow gap.

### PAR-0526 evidence — 2026-11-22

- **id:** PAR-0526-EV-0112
- **timestamp:** 2026-11-22T11:19:15Z
- **what:** Added EV-0112 and linked it to the partial `ui-distinguish-permission-rejection-from-network-error` capability for scenario `structured-policy-auth-and-timeout-copy`, bound to implementation SHA 58aa11c445c8624b9128cdf148be7d2e9a8bf180.
- **why:** Keep the permission/auth versus network behavior auditable without overstating live or packaged verification.
- **components:** UCM capability map/evidence, Doofus task checkpoint, plan and execution journal.
- **type:** evidence and progress record
- **validation:** Scenario coverage is pass; EV-0112 has no scoped schema or reconciliation findings. Source package validator passes 565 behaviors, 15 source inspections and 96 implementation receipts; task state validates at revision 758.
- **performance:** No runtime behavior changed in the evidence step.
- **risks:** UCM shows a promotion suggestion based on headless test evidence; status remains partial because live Gateway, packaged Windows and source-paired acceptance are still open. The map-wide checker retains 162 legacy errors and 11 unrelated reconciliation items.
- **follow_up:** Commit evidence and checkpoint, publish PAR-0526 to origin/main, then continue the next P0 agent-workflow gap.

### PAR-0526 publication — 2026-11-22

- **id:** PAR-0526-PUBLISH
- **timestamp:** 2026-11-22T11:20:19Z
- **what:** Published implementation commit 58aa11c445c8624b9128cdf148be7d2e9a8bf180 and evidence commit d97a88aa919f0757ca658b7599f54a6fcc54cca9.
- **why:** Deliver the tested native error distinction and its partial UCM evidence on the requested main branch.
- **components:** Hermes transcript error presentation, UCM, task ledger.
- **type:** publication
- **validation:** Fresh fetch confirmed local main and origin/main both equal d97a88aa919f0757ca658b7599f54a6fcc54cca9.
- **performance:** No runtime behavior changed during publication.
- **risks:** Live Gateway, packaged Windows, and source-paired acceptance are still pending.
- **follow_up:** Continue with PAR-0527, preventing destructive fallback on malformed responses.


### PAR-0527 — Reject malformed successful session refreshes — 2026-11-22

Native rejects a sidebar success response that omits `recents.sessions` without explicit profile failure details. A malformed all-profile refresh leaves cached sessions visible and reports the error; valid empty results and explicit profile failures are unchanged. Focused and full headless tests pass. Live Gateway and packaged Windows acceptance remain open.


### PAR-0527 evidence — 2026-11-22

EV-0113 binds the malformed-sidebar cache-preservation regression to `b1baf05e20c02b8e28638c93db0236a2119b6eb2`. The UCM capability is partial with a passing headless scenario; live Gateway and packaged Windows behavior are unverified. Completion overlays now report 112/499 broad and 78/176 chat/tool evidence coverage.


### PAR-0527 publication — 2026-11-22

Published implementation commit `b1baf05e20c02b8e28638c93db0236a2119b6eb2` and EV-0113/progress commit `4559146774428da8c59eaa41b96afdb394a90740` to main. Fresh fetch confirmed exact local/origin SHA equality at `4559146774428da8c59eaa41b96afdb394a90740`.


### PAR-0528 — New-chat model and reasoning selection — 2026-11-22

- **id:** PAR-0528
- **timestamp:** 2026-11-22T12:04:24Z
- **what:** Added profile-catalog-backed model/provider and reasoning choices to the New Chat composer and included matching manual overrides in `session.create`.
- **why:** The pinned Hermes composer allows model selection before the first prompt while keeping manual session choices separate from profile defaults.
- **components:** Native egui composer, profile model-options request handling, Gateway session creation, headless UI tests, UCM evidence.
- **type:** feature
- **validation:** Four focused new-chat tests pass; full locked suite passes 594 library + 15 protocol + 3 launcher tests; all-target cargo check, 19 task continuity tests, task ledger, rustfmt, diff checks and package validator pass. The UCM checker runs with temporary PyYAML but exits 1 on 164 existing map-wide errors and 11 reconciliation items; capability remains partial.
- **performance:** No benchmark was run; request loading remains one profile-scoped model-options read per new-chat owner.
- **risks:** Profile-global model Settings, authenticated live Gateway behavior, packaged Windows interaction and formal pinned-source acceptance remain open.
- **follow_up:** Implement profile-global main-model policy and preserve the distinct new-chat override behavior.


### PAR-0528 publication — 2026-11-22

- **id:** PAR-0528-PUBLISH
- **timestamp:** 2026-11-22T12:08:23Z
- **what:** Published implementation `e000cd5a0edfe55dd516a4edccecadbc7af0a4cc` and EV-0114/evidence checkpoint commit `efb208069d014293c6ab5f5027439d305d744186` to `main`.
- **why:** Deliver the verified, profile-scoped pre-first-message model/reasoning slice and its honest partial evidence to the canonical repository.
- **components:** GitHub repository `robertheadley/Doofus-Dock-Hermes`, main branch, task ledger and UCM evidence.
- **type:** publication
- **validation:** Push succeeded; fresh fetch confirmed local main and origin/main both equal `efb208069d014293c6ab5f5027439d305d744186`.
- **performance:** No runtime behavior changed during publication.
- **risks:** Profile-global main-model Settings and authenticated live/package/source acceptance remain open; `apps/hermes-native/work/` remains operator-owned and untracked.
- **follow_up:** Continue with profile-global main-model policy and keep it distinct from manual new-chat overrides.


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
- **what:** Added commit-bound partial evidence and regenerated the implementation progress overlay for the existing approval card's supported-choice-to-Gateway behavior.
- **why:** Make the approval interaction traceable without removing or replacing any previously functional code or evidence.
- **components:** PAR-0514, native approval card, Gateway server-request actor, EV-0117, progress overlay, plan/backlog/docs.
- **type:** evidence_and_docs
- **validation:** Full locked suite 599 library + 15 protocol + 3 launcher; all-target check; rustfmt/diff; 19 task-continuity checks; implementation progress generator passed with 114/499 reviewed.
- **performance:** No runtime benchmark was run.
- **risks:** Authenticated live Gateway, packaged Windows interaction, and formal pinned-source acceptance remain open.
- **follow_up:** Keep the task partial; continue the first dependency-ready agent-workflow gap.

- **id:** hermes-par0509-resume-evidence-reconcile-20261122
- **timestamp:** 2026-11-22T18:08:00-06:00
- **what:** Mapped existing exact-profile saved-session resume behavior and headless regressions as partial evidence in EV-0124; updated progress and task lifecycle.
- **why:** The source and tests already exercised profile, stored/runtime identity, owner/request fencing, refusal recovery and no-resubmit behavior, while progress still showed the requirement as unknown.
- **components:** PAR-0509, EV-0124, capability maps, implementation/chat-tool progress, task ledger.
- **type:** evidence_and_status_reconciliation
- **validation:** Existing full locked suite on `1412ed8` passed 617 library + 15 protocol + 3 launcher; all-target check, rustfmt, and 19/19 task-continuity checks passed; package validator reports 565 behaviors and 109 implementation receipts.
- **performance:** No runtime benchmark was run.
- **risks:** Authenticated live Gateway cold-start resume, packaged Windows, and formal source-paired acceptance remain open.
- **follow_up:** Continue the PAR-0062 attachment workflow and preserve partial status until live evidence exists.

- **id:** hermes-par0062-attachment-stale-runtime-recovery-20261122
- **timestamp:** 2026-11-22T18:08:00-06:00
- **what:** Added profile- and stored-session-fenced stale-runtime recovery for image/file staging. A definitive `session not found` refusal resumes once, emits the runtime rebind, and retries the same already-read bytes exactly once. Added loopback actor and app regressions; source commit `3b50cc35a63eac09f08d768b8dcd8f4afc276f76` is published to `main`.
- **why:** Attachments execute before prompt submission, so an otherwise recoverable saved chat could not send image/file context after Hermes rotated its runtime session ID.
- **components:** PAR-0062, Gateway command/actor, composer upload ownership, EV-0125, capability maps, progress ledgers, plan, backlog, README, execution log.
- **type:** feature_and_test
- **validation:** Full locked suite 622 library + 15 protocol + 3 launcher; all-target locked check; rustfmt/diff; task continuity 19/19; package validator 565 behaviors and 109 implementation receipts. Focused attachment actor/app regressions pass.
- **performance:** Attachment bytes are read once and reused for the single retry; no runtime benchmark was run.
- **risks:** Clipboard images/pathless drops, path-vs-bytes proof, authenticated live Gateway outcomes, packaged Windows and source-paired acceptance remain open; ambiguous transport failures are deliberately not retried.
- **follow_up:** Publish EV-0125 and reconciled progress/docs to `main`; continue with clipboard image paste and the remaining attachment interaction gaps.

- **id:** hermes-par0515-clarify-ui-gateway-20261122
- **timestamp:** 2026-11-22T13:40:00Z
- **what:** Added headless app-to-Gateway evidence for ordered clarification answers and Skip, then updated the capability and generated progress records.
- **why:** Verify the existing batch clarification workflow at its user-action boundary without replacing working behavior.
- **components:** PAR-0515, clarification card, `clarify.lock`, EV-0118, capability map, progress overlay, architecture and plan docs.
- **type:** evidence_and_docs
- **validation:** Full locked suite 601 library + 15 protocol + 3 launcher; all-target check; rustfmt/diff; 19 task continuity tests; progress and package validators pass.
- **performance:** No runtime benchmark was run.
- **risks:** Live Gateway, packaged Windows interaction, and formal pinned-source acceptance remain open.
- **follow_up:** Continue the next dependency-ready agent workflow gap.


- **id:** hermes-par0516-sudo-ui-gateway-20261122
- **timestamp:** 2026-11-22T14:08:50Z
- **what:** Added masked, ephemeral Gateway sudo response UI and actor routing, with connection/profile/request ownership and unknown-outcome protection.
- **why:** Preserve the pinned Hermes agent's ability to request administrator credentials without declining valid sudo requests.
- **components:** `apps/hermes-native/src/app.rs`, `apps/hermes-native/src/gateway.rs`, PAR-0516, EV-0119.
- **type:** feature_and_test
- **validation:** PASS 606 library + 15 protocol + 3 launcher tests; all-target locked cargo check; formatting and diff checks.
- **performance:** No runtime benchmark was run.
- **risks:** Live Gateway, packaged Windows and formal pinned-source acceptance remain open; JSON-RPC holds a transient serialized copy in memory.
- **follow_up:** Continue the agent-workflow scope at its next dependency-ready gap.


- **id:** hermes-par0517-vault-unlock-20261122
- **timestamp:** 2026-11-22T22:17:00Z
- **what:** Added masked native vault unlock requests and explicit server-request capability advertisement for local and remote Gateway attachments.
- **why:** Hermes password-manager tool flows must be answerable through the correct profile/session/request without persisting or leaking the master password.
- **components:** PAR-0517, PAR-0520, Gateway actor, vault unlock prompt, EV-0120, EV-0121, generated progress.
- **type:** feature_and_test
- **validation:** PASS 613 library + 15 protocol + 3 launcher tests; all-target locked cargo check; rustfmt/diff; 19 task-continuity tests; progress and package validators.
- **performance:** No runtime benchmark was run.
- **risks:** A normal JSON string copy exists transiently during response serialization/send; live authenticated Gateway, packaged Windows and formal pinned-source acceptance remain open.
- **follow_up:** Verify authenticated local/remote Gateway behavior and continue the next dependency-ready agent-workflow gap.

- **id:** hermes-par0517-publication-20261122
- **timestamp:** 2026-11-22T22:30:20Z
- **what:** Published the vault unlock and server-request capability implementation with its evidence and current progress overview to `main`.
- **why:** Keep the shared repository current and preserve auditable feature status.
- **components:** PAR-0517, PAR-0520, EV-0120, EV-0121, `main`.
- **type:** publication
- **validation:** Fresh fetch confirmed local main and origin/main both equal `4f8eeb7ff81d9963fa7314c64e759a0974306486`.
- **performance:** No runtime benchmark was run.
- **risks:** Live Gateway and packaged Windows acceptance remain open; operator-owned runtime work was preserved.
- **follow_up:** Continue with PAR-0508 bounded Gateway reconnect, replay/resume and duplicate-submit safeguards.

- **id:** hermes-par0508-main-publication-20261122
- **timestamp:** 2026-11-22T23:02:33Z
- **what:** Published the Gateway reconnect source slice on `main`; partial capability evidence and progress documentation follow in a separate commit.
- **why:** Keep the user-selected shared branch current while continuing the feature-parity work.
- **components:** Doofus-Dock-Hermes `main`, PAR-0508, commit `1412ed8`.
- **type:** publication
- **validation:** Full locked suite 617 + 15 + 3, all-target check and 19 continuity tests passed; push/fetch verified source commit on origin/main.
- **performance:** No runtime benchmark was run.
- **risks:** Live Gateway, packaged Windows, endpoint migration and strict source acceptance remain open.
- **follow_up:** Publish evidence/docs and continue the next dependency-ready agent-workflow gap.

- **id:** hermes-par0508-evidence-reconcile-20261122
- **timestamp:** 2026-11-22T23:09:44Z
- **what:** Reconciled reconnect/replay evidence, UCM capability states, generated status ledgers, and durable task lifecycle for PAR-0508/PAR-0511.
- **why:** Keep shared progress grounded in the source commit and distinguish headless implementation evidence from live runtime acceptance.
- **components:** EV-0122, EV-0123, capabilities.yaml, CAPABILITY_CROSSWALK.csv, full_program_capability_map.json, implementation progress, chat/tool progress, .doofus checkpoint.
- **type:** evidence_and_status_reconciliation
- **validation:** Progress builders pass (119/499; 85/176); UCM report confirms declared headless scenario coverage and partial state; package validator 491 pending; task continuity 19/19; doofus state valid; git diff --check passes. Existing map-wide checker errors remain baseline and are documented in EXECUTION_LOGS.
- **performance:** No runtime benchmark was run.
- **risks:** Live authenticated Gateway, packaged Windows, endpoint migration and strict source acceptance remain open.
- **follow_up:** Continue the next dependency-ready P0 agent-workflow gap; preserve partial labels until runtime evidence exists.

- **id:** hermes-par0508-publish-evidence-main-20261122
- **timestamp:** 2026-11-22T23:11:30Z
- **what:** Published PAR-0508/PAR-0511 implementation evidence, progress records and UCM reconciliation to `main`.
- **why:** Keep the user-selected shared branch current and make implementation state auditable against the pinned source.
- **components:** Doofus-Dock-Hermes main, commit `4e6614c`, EV-0122, EV-0123, progress ledgers.
- **type:** publication
- **validation:** `git push` succeeded; fresh `origin/main` fetch exactly matches local main at `4e6614c04eab33d98d2d0e3a8557777043dd3f40`.
- **performance:** No runtime benchmark was run.
- **risks:** Live Gateway and packaged Windows reconnect acceptance remain open; operator-owned runtime data was excluded.
- **follow_up:** Review and implement the selected next P0 Gateway capability, PAR-0503.

- **id:** hermes-par0062-clipboard-image-paste-20261122
- **timestamp:** 2026-11-22T18:28:00-06:00
- **what:** Added focused-composer clipboard image paste with validated, atomic PNG staging and routed it through the existing profile/session-owned attachment and stale-runtime recovery pipeline. Published implementation commit `53e8c62bb729b8a33ddbb9f35ef13a855c9a8547` to `main`.
- **why:** Image context is a core agent prompt input and must be usable from clipboard, not only the file picker or path-based drop.
- **components:** PAR-0062, clipboard-image event routing and staging, app-owned cleanup, Gateway attachment flow, EV-0126, progress maps, architecture, plan, backlog, completion overview, README.
- **type:** feature_and_test
- **validation:** Full locked suite 626 library + 15 automation protocol + 3 launcher tests; all-target locked check, rustfmt and diff check pass. The first full-suite run exposed an existing WebSocket-close race in a fail-closed Gateway test; after correcting the assertion to target only rebind/retry events, the serial full suite passed.
- **performance:** Clipboard PNG conversion is bounded to 16 megapixels and 16 MiB; no runtime benchmark was run.
- **risks:** Actual Windows clipboard capture, authenticated live Gateway upload, pathless drops, thumbnails, interrupted-draft restart validity, packaged Windows and formal source-paired acceptance remain open.
- **follow_up:** Publish EV-0126 and refreshed evidence/progress maps to `main`; then continue remaining attachment paths and live acceptance.

## PAR-0062 — Pathless composer file drops — 2026-11-23

- **id:** PAR-0062-pathless-composer-drop
- **timestamp:** 2026-11-23T00:49:13Z
- **what:** Added bounded byte staging for native dropped-file payloads that lack an absolute local path. Composer-only drops now use sanitized basenames, infer recognized image extensions, verify staged bytes before atomic publication, and clean only files in app-generated operation directories. Published source commit `4392b6ff0267065718ee5006eb76f4f3cfaef4f9` to `main`.
- **why:** A file drop should remain a usable attachment input when the platform supplies bytes and a name but no reusable filesystem path.
- **components:** `apps/hermes-native/src/app.rs`, `src/clipboard_image.rs`, PAR-0062 evidence/progress, attachment architecture, README and backlog.
- **type:** feature, headless regression coverage, parity evidence.
- **validation:** Full locked suite passed (629 library, 15 automation protocol, 3 launcher); workspace/all-target locked check, formatting, diff check, task continuity (19/19), capability package validator (565 target behaviors, 565 inventory items, 111 implementation receipts), and debug build passed.
- **performance:** Payload reads are capped at 16 MiB; writes use create-new temporary files and readback validation. No runtime benchmark was run.
- **risks:** The new helper and staging behavior have headless filesystem coverage; actual Windows OS-drop delivery, live authenticated Gateway outcomes, packaged behavior and source-paired acceptance remain unverified. PAR-0062 remains partial.
- **follow_up:** Continue authenticated attachment outcomes, Windows drop verification, thumbnails and remaining agent-workflow gaps.

### PAR-0062 publication record — 2026-11-23

- Implementation commit `4392b6ff0267065718ee5006eb76f4f3cfaef4f9` and evidence/docs commit `b10fcdc71db091400d12e915cfd5bb9003d82ae1` were pushed to `main`.
- Fresh fetch confirmed local `main` and `origin/main` both equal `b10fcdc71db091400d12e915cfd5bb9003d82ae1`.
- EV-0127 records partial headless evidence for pathless file drops. Windows Gateway acceptance remains open.

### Revision 346 — Profile reasoning visibility control — 2026-11-23

- **id:** PAR-0046-profile-reasoning-visibility
- **timestamp:** 2026-11-23T01:24:22Z
- **what:** Added Settings → Chat · Behavior control for `display.show_reasoning`, profile-aware local/remote Gateway config save, CLI-key precedence, save-receipt draft confirmation, and connection fencing. Source commit `aef5dbd9a07456ca8361f886356d63c31d150619`.
- **why:** Pinned Hermes exposes reasoning visibility as an agent chat behavior preference; the native transcript already separated reasoning but users had no control to enable it.
- **components:** Hermes Native app/settings, Gateway configuration transport, PAR-0046 capability and progress evidence.
- **type:** feature, headless regression coverage, capability evidence.
- **validation:** Full locked suite: 635 library + 15 automation protocol + 3 launcher tests passed. 21 reasoning-focused tests passed; all-target locked check, `cargo fmt --all`, and `git diff --check` passed.
- **performance:** No performance-sensitive path or benchmark changed.
- **risks:** No authenticated live Gateway, packaged Windows UI, or source-paired acceptance was run; PAR-0046 remains partial.
- **follow_up:** Validate against a live authenticated Gateway and pinned UI, then continue the next dependency-ready P0 chat/tool gap.

### PAR-0046 capability evidence reconciliation — 2026-11-23

- **id:** PAR-0046-UCM-evidence-reconciliation
- **timestamp:** 2026-11-23T01:29:25Z
- **what:** Appended EV-0128 through EV-0130, declared their test scenarios, and replaced legacy inline capability locators with stable evidence IDs.
- **why:** Keep the capability map consistent with the current tested profile reasoning setting and distinguish local verification from live acceptance.
- **components:** Pinned-snapshot capability map, evidence files, generated implementation and chat/tool progress.
- **type:** evidence, documentation, progress map.
- **validation:** UCM scoped report finds no capability-specific errors and all three declared scenarios have passing evidence. Package validator passes 565 behavior rows and 112 implementation receipts. Task continuity passed 19/19; task state validator passes.
- **performance:** Not applicable.
- **risks:** Full-map UCM check still reports 160 pre-existing errors and 10 unrelated reconciliation items. Capability stays partial; all 176 chat/tool rows still lack live-runtime verification and pinned-source acceptance.
- **follow_up:** Continue the next dependency-ready P0 agent-workflow gap; resolve UCM-wide legacy errors separately.

### PAR-0046 publication record — 2026-11-23

- **id:** PAR-0046-publication
- **timestamp:** 2026-11-23T01:31:44Z
- **what:** Published reasoning visibility implementation (`aef5dbd9a07456ca8361f886356d63c31d150619`) and capability/docs evidence (`b6ce88fb68a5c27b9c673f07c6ece518d81d4103`) to the user-selected `main` branch. A changelog whitespace-only cleanup follows in `1496ea010630aa3dc7cb45aa09880f50d1fbc645`.
- **why:** Keep the repository's main branch current with validated agent-workflow work and its evidence.
- **components:** `Doofus-Dock-Hermes` main, PAR-0046 source, EV-0128/0129/0130.
- **type:** publication.
- **validation:** Fresh fetch confirmed local `main` and `origin/main` both resolve to `1496ea010630aa3dc7cb45aa09880f50d1fbc645` before this publication record. The record will be committed and pushed separately, then exact ref equality rechecked.
- **performance:** Not applicable.
- **risks:** Live Gateway and packaged Windows acceptance remain open.
- **follow_up:** Select the next dependency-ready P0 gap and continue toward complete chat/tool, skills, plugin/MCP, provider and context/attachment support.

### PAR-0064 — Code-specific failed-turn recovery

- **id:** hermes-par0064-failed-turn-recovery-20261123
- **timestamp:** 2026-11-23T01:48:30Z
- **what:** Added source-code-specific error copy and Edit failed message / Start new session actions, while suppressing Retry for `SESSION_OWNER_UNCONFIRMED`. Existing same-submission-ID retry continues to preserve the sole optimistic user row.
- **why:** Make recovery follow pinned Hermes actions and prevent a prompt from running twice when Gateway admission is uncertain.
- **components:** Hermes Native transcript error cards, retry policy, PAR-0064 evidence and progress maps.
- **type:** feature, safety, documentation.
- **validation:** Focused failed-turn tests pass (8/8). Full locked suite passes (637 library + 15 automation protocol + 3 launcher); all-target locked check, rustfmt and diff check pass. Targeted UCM evidence records the three passing local scenarios; global legacy map findings remain unrelated. Source implementation is commit `2c24649455c8b81fd96d6e367b30155f9791fb84`.
- **performance:** No performance-sensitive path or benchmark changed.
- **risks:** Model/account/provider-settings navigation, context compression, scheduled retry, complete pinned error-code coverage, authenticated live Gateway behavior, and packaged Windows acceptance remain open.
- **follow_up:** Continue the remaining P0 chat/tool work; keep PAR-0064 partial until live Gateway and source acceptance.

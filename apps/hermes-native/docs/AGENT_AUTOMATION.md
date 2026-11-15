# Agent-operated UI and background evidence

## User requirement

A coding agent should be able to do everything a person can do through Hermes UI, and more through debugging and inspection, without repeatedly taking over the operator’s desktop.

## Current semantic runner

`cargo run --features agent-automation --bin hermes-automation -- --inspect --click Files --screenshot artifacts/agent-ui/native-shell.png` runs a headless egui_kittest/AccessKit harness over the shared `HermesNativeApp::draw` function. It can inspect the semantic tree, click a control by accessible name, type into an accessible control, and render a screenshot at `--viewport WIDTHxHEIGHT`. It creates no native window, does not activate the desktop, does not start the Hermes gateway, and uses an isolated temporary layout profile. The runner and egui_kittest dependency require the opt-in `agent-automation` Cargo feature and are not in the default build.

This is a local development executable, not an external service. It has no authenticated Debug Bridge, complete typed capability/action registry, build supervisor, or physical Windows host control. The typed registry covers navigation, pane toggles, interface mode, layout profiles, Capabilities tabs/refresh, Toolsets enable/disable, stopping the active generation, answering visible approval/clarify prompts, and closing the active tab through `close_active_tab`; human controls call the same app-core handlers. Toolsets and session Pin/Unpin/read-state/archive writes use the bounded async gateway HTTP worker and return correlated success/error events; session mutations use the Desktop mutation snapshot fence and carry the owning profile. Archive/Unarchive is available in the cross-profile archived-session view, which the agent can open through `sessions.show-archived`; it can return to active sessions through `sessions.show-active`. Foreign-profile rows are inspectable and restorable but cannot be opened until chat operations support explicit profile scope. Stop uses the active runtime session ID and `session.interrupt`. Approval decisions are limited to the choices offered by Hermes. Single-question clarify answers use source-provided choices and generation-scoped prompt identity; skip sends the contract's empty answer. Batch server-request clarify answers are staged per question and use sequential `clarify.lock` calls; replayed answers remain editable and Skip closes the request. These prompt actions reject stale IDs and disconnected state. The accessibility tree exposes stable IDs for fixed controls and dynamic session rows/actions (`session.row.<profile>.<stored-id>`, `session.actions.<profile>.<stored-id>`, `session.pin.<profile>.<stored-id>`, `session.unread.<profile>.<stored-id>`, `session.archive.<profile>.<stored-id>`, `sessions.show-archived`, `sessions.show-active`); contributed/user controls still need IDs. It validates the shared UI drawing and egui input path; it does not prove Windows window, focus, DPI, or source parity.

## Opt-in child-process protocol

`cargo run --features agent-automation --bin hermes-automation -- --stdio` starts a versioned newline-delimited JSON interface over inherited stdin/stdout. It exposes health, capabilities, state, AccessKit tree, events, a diagnostics bundle, exact role/name click/type/select, keyboard input, wait-for-visible-control, screenshot capture, and close. UI actions require an expected state generation and return a structured before/after receipt. Requests are bounded and idempotent: matching retries replay a response, conflicting IDs fail, and old IDs whose response left the cache are rejected without re-executing. The process uses an isolated temporary profile, no network listener, native window, desktop focus, or Hermes gateway. Screenshot writes are restricted to new `.png` basenames under `artifacts/agent-ui`.

The exact wire contract, bounds and error cases are in `AGENT_AUTOMATION_PROTOCOL.md`. This inherited-pipe interface is a child-process capability for a trusted parent; it is not the supplemental authenticated loopback Debug Bridge or a production service.

## Intended external interface

Expose stable semantic locators backed by native accessibility information and explicit test IDs. A client should be able to inspect the visible control tree and query by role, accessible name, test ID, enabled/visible state, and bounds. Provide Playwright-like actions: click, fill/type, select, modified keypress including tab cycling, bounded pointer drag, wait for a condition, and capture a screenshot. Each action must enter the same application event/handler path used by a human.

The runtime should return structured action receipts with target identity, before/after state, and errors. Locators should be stable across visual restyling and scoped to the running app instance. The native Accessibility tree remains available to ordinary Windows UI Automation clients and screen readers. Coordinate input is a fallback and physical-host verification method, not the primary control architecture.

## More than a person can do

Add a separate local authenticated Debug Bridge for capabilities, bounded state/layout/panels/windows, logs, events, diagnostics, action preview/execute and reload status. Diagnostic commands may inspect richer state; mutations must still follow app permissions, Hermes approvals, destructive confirmations and canonical DoofusDock transactions. The bridge is distinct from Hermes gateway and disabled/absent in production.

## Unattended operation

Run isolated Hermes Native instances with dedicated app data and an isolated Hermes profile or deterministic fixture backend. Do not take foreground focus, send input to the operator’s active window, or write to the installed Hermes profile. Use an isolated live profile for protocol acceptance; do not reuse a user session for mutating scenarios. The headless runner is the current non-disruptive path.

Store screenshots and traces as reviewable evidence with timestamp, source SHA, executable build ID, process generation, route, viewport/DPI, fixture/profile identity, state/layout revision, and action receipts. Keep headless, semantic integration, and physical Windows evidence separate.

## Acceptance

- Agent can discover controls by semantic locator rather than screen coordinates alone.
- Agent can navigate, type, select, right-click context menus, open/close panels and dialogs, and invoke keyboard paths through the same handlers as human input.
- State changes and failures are observable and have structured receipts.
- Screenshots and interaction traces can be captured from a non-foreground isolated instance without disrupting the operator.
- Diagnostics expose additional state but cannot silently execute a user-facing mutation or bypass a confirmation.
- The same interaction succeeds through human input and the automation interface; parity claims require both evidence paths.

## Current status

The headless semantic runner has completed Files-panel reveal and Capabilities-route navigation, each with a screenshot. The opt-in stdio child protocol has also exercised pointer drag to center-tab and left-edge split panels, accessible dock-tab selection, reorder, and right-click context menus, modified Ctrl+Tab session cycling (with semantic source and destination locators), split resize via canonical `ResizeSplit`, stale-state rejection, idempotent replay, diagnostics/events and screenshot capture. The reordered tab screenshot is `artifacts/agent-ui/tab-reorder-v19.png`; its adjacent metadata records the isolated process, canonical revision and binary/screenshot hashes. Fixed first-party controls expose stable IDs and protocol tests click/type/select/wait/drag through them. The typed app-action registry offers Toolsets enable/disable, generation-scoped approval choices, and prompt-ID-scoped clarify answers through the same handlers as the visible controls; protocol coverage confirms registration. Dynamic rows, broader shortcut coverage, the complete typed action registry, supervisor, real window operations and full acceptance remain open. See `docs/AGENT_AUTOMATION_PROTOCOL.md`, `docs/AGENT_CONTROL_CROSSWALK.md` and `docs/PLAN.md`.

Latest evidence: the 1440×900 headless screenshot `artifacts/agent-ui/capabilities-toolsets-v23.png` was captured after semantic navigation to Toolsets, with source/build/route/profile metadata in the adjacent JSON. The expected offline message is shown because the isolated run disables the gateway. This validates route rendering and agent navigation only, not live inventory rendering or source/Windows visual parity.

The latest 1440×900 session-pane render is `artifacts/agent-ui/session-pane-draft-reuse-20261115.png`; its sidecar records the fixed Hermes/DoofusDock SHAs, current source diff, executable and screenshot hashes, and isolated fixture limits. It is a headless capture with no gateway request.

The latest 1440×900 session-pane render is `artifacts/agent-ui/session-pane-primary-close-20261115.png`; its sidecar records the fixed Hermes/DoofusDock SHAs, source diff, executable and image hashes, plus fixture limits. It is a deterministic headless screenshot; no gateway request was sent.

The latest 1440×900 session-pane render is `artifacts/agent-ui/session-pane-background-routing-20261115.png`; its sidecar records the fixed Hermes/DoofusDock SHAs, source diff, executable hash and fixture limits. It is a deterministic headless screenshot; no gateway request was sent. The fixture was recaptured after hidden approval/clarify action routing so the binary and source-diff metadata match.

The current 1440×900 new-chat capture is `artifacts/agent-ui/chat-approval-v27.png`; its metadata records the exact binary and image hashes. It shows the offline welcome route and the chosen fonts. Approval cards are event-driven and therefore are not present in this screenshot; their accessible controls and offline-disabled state are covered by the approval UI fixture. `artifacts/agent-ui/tool-cards-v40.png` shows two completed tool calls emitted by an isolated local event fixture; its sidecar records the build/source/viewport and the limits of that evidence. Transcript rows now expose combined accessible group labels, though dynamic tool rows still have no stable test IDs.

`artifacts/agent-ui/approval-request-v31.png` shows an inbound `approval` server-request fixture rendered through the permission card. Its sidecar records the offline profile and confirms that the offered choices honor `allow_permanent=false`; it is not live-gateway evidence, and server-request negotiation remains disabled.

Typography evidence: `artifacts/agent-ui/font-choice-v24.png` and its JSON sidecar capture the chosen JetBrains Mono UI and Fira Code composer family at 1440×900. This is an isolated, gateway-disabled headless run; it does not establish physical Windows font rasterization or source-paired parity.

The session-pane fixture is artifacts/agent-ui/session-pane-resume-error-20261115.png. Resume-error and Retry accessibility are verified by isolated AccessKit app tests; this screenshot shows the normal pane fixture, not the error card. No authenticated gateway or physical Windows evidence is implied.
## Recent session switcher fixture

Use `cargo run --features agent-automation --bin hermes-automation -- --recent-session-switcher-fixture --inspect --screenshot apps/hermes-native/artifacts/agent-ui/recent-session-switcher-20261115.png` to capture the recent-session HUD over three synthetic profile-scoped rows. The runner always starts a fresh isolated offline profile; without a fixture or navigation action, its screenshot shows the default New session route. The committed capture and JSON sidecar record the source SHA, target build, route, viewport, fixture limits and image hash.

# Hermes Native agent UI protocol

## Scope and transport

`hermes-automation --stdio` is an opt-in, development-only child process enabled by Cargo feature `agent-automation`. It uses newline-delimited JSON over inherited stdin/stdout. The parent process owns the pipes as the capability to control this instance; the runner opens no TCP/HTTP listener, native window, desktop input channel, Hermes gateway, or installed Hermes profile. Stderr is reserved for process errors. One process owns one isolated, temporary app profile and one headless UI instance.

The protocol is `hermes-native-agent-ui/1`. The first output line is a `ready` envelope. Each subsequent nonempty input line is one request and receives one response line. EOF or `protocol.close` ends the process. Requests use `{ "version": 1, "id": "unique-id", "method": "...", "params": {} }`. Responses are `{ "version": 1, "id": ..., "status": "succeeded", "result": ... }` or `{ "version": 1, "id": ..., "status": "failed", "error": { "code": "...", "message": "..." } }`.

## Methods

| Method | Permission | Parameters / result |
| --- | --- | --- |
| `health.get` | Read only | Process, protocol, build, generation, gateway status |
| `capabilities.list` | Read only | Implemented and explicitly unsupported capabilities |
| `app.actions.list` | Read only | Versioned catalog of typed first-party app actions and validation schemas |
| `app.action.execute` | UI interaction | Typed `action` object plus `expected_state_generation`; executes a registered shared app handler |
| `state.get` | Read only | Current route, layout, panels, gateway summary, recent-session switcher open/pending/index/session IDs, generation |
| `ui.tree` | Read only | AccessKit role/name/value/disabled/bounds, nullable stable `test_id`, and child tree |
| `events.list` | Read only | Bounded recent event receipts |
| `diagnostics.bundle` | Read only | State, bounded events, process and protocol health counters |
| `actions.click` | UI interaction | Either `test_id` or `role` + `name`, with `expected_state_generation`; requires one exact accessible match |
| `actions.right_click` | UI interaction | Same locator and generation contract as click; emits secondary-pointer press/release at the exact accessible target center, opening the same egui context menu used by a person |
| `actions.drag` | UI interaction | Source either `test_id` or `role` + `name`; destination either `to_test_id`, `to_role` + `to_name`, or logical `to_x` + `to_y`; optional `steps` (1–32) and `expected_state_generation`; emits egui pointer events |
| `actions.type` | UI interaction | Either `test_id` or `role` + `name`, plus `text` and `expected_state_generation`; requires one exact accessible match |
| `actions.select` | UI interaction | Either `test_id` or `role` + `name`, plus `option` and `expected_state_generation`; opens the accessible selector and clicks the unique visible option |
| `actions.keypress` | UI interaction | `key`, optional `modifiers` string array of up to four names (`ALT`, `CTRL`, `SHIFT`, `COMMAND`, `META`), and `expected_state_generation`; key names include navigation/edit keys and A–Z. `COMMAND` is the platform shortcut modifier; `CTRL` is the physical Control key. The protocol exercises `CTRL+TAB`, `CTRL+SHIFT+TAB`, `CTRL+PAGEDOWN`, `CTRL+PAGEUP`, `CTRL+SHIFT+T` (restore the last closed session tab on Windows/Linux) and `COMMAND+CTRL+SHIFT+T` (restore on macOS) through the application keyboard path. In a session, `CTRL+F` opens transcript search, `CTRL+G` and `CTRL+SHIFT+G` step through matches, and `ESCAPE` closes search; macOS uses `COMMAND` in place of the primary modifier. When no eligible Dock strip exists, these shortcuts use the fixed-source recent-session fallback: quick navigation, delayed/repeated HUD browsing, and Ctrl-release commit. |
| `ui.wait_for` | Read only | Either `test_id` or `role` + `name`, optional `timeout_ms` from 0 to 5000; waits for one visible accessible control |
| `screenshots.capture` | Artifact write | `filename`; single `.png` basename under `artifacts/agent-ui`, never overwrites |
| `protocol.close` | Process control | Empty object; returns closed and exits |

Semantic actions dispatch through egui_kittest/AccessKit input to the shared UI drawing/widget path. `app.actions.list` exposes a versioned typed registry for navigation, pane toggles, interface mode, layout profile selection, Capabilities tabs/refresh, Toolsets enable/disable, `stop_generation`, `respond_approval`, `respond_clarify`, `close_active_tab` and `restore_closed_session_tab`. Human toolbar, pane, route, Toolsets, Stop, approval-card and clarify-card controls call the same app-core handlers; `app.action.execute` validates the tagged action schema and expected state generation before dispatch. Toolsets mutations require a connected gateway and an exact name already present in the loaded inventory; they run on the bounded profile-scoped HTTP worker and report a correlated receipt or error. Stop requires a connected gateway, active runtime session and pending submission, and calls `session.interrupt`; its result is correlated to the pending request. Approval decisions require the currently visible prompt ID and a choice offered by Hermes, then call generation-scoped `approval.respond`. Single-question clarify responses use the active prompt ID and generation-scoped `clarify.respond`; an empty answer means skip. Stale, cross-session, duplicate, unavailable, oversized and disconnected actions fail before dispatch. The registry rejects unregistered pane IDs and unknown fields. This is a partial app-action slice; batch clarify, inbound server requests and other gateway work or DoofusDock operations remain open. The tree includes explicit stable IDs for fixed app controls; dynamic session/toolset rows and user-defined content use role/name locators until their owning feature supplies IDs. Mutating methods accept either a stable `test_id` or an exact role/name pair, never both. Disabled targets and options fail without input. A successful action increments `state_generation` and returns request/correlation/trace IDs, target, before/after state, permission class and outcome. The generation must match exactly; stale mutations fail with `stale_state`. `ui.wait_for` advances egui frames and searches the current accessibility tree, so it can observe an app-local background worker result such as the public plugin catalog. It does not itself issue or wait on a Hermes gateway RPC. Malformed or unsupported modifier names are rejected before input. Native window control remains unsupported.

## Bounds and retry rules

- Request lines are limited to 64 KiB; a request ID is 1–128 bytes; text is limited to 16,384 Unicode characters.
- The accessible tree includes role, name/value, disabled/focused flags, nullable stable `test_id`, children, and physical-pixel bounds when supplied by AccessKit; bounds are `null` for nodes without a bounding box. Stable IDs are assigned to app controls such as `sessions.search`, `chat.composer`, conditional `chat.stop`, `app.layout-profile`, `route.capabilities`, and `panel.sessions`.
- Responses are limited to 256 KiB. The event ring retains 128 events.
- The response cache retains the latest 128 request IDs. Identical retries within that window replay the original response. Reusing an ID with a different payload returns `request_id_conflict`. After cache eviction, an already-seen ID returns `request_replay_expired` and is never executed again. At most 10,000 unique IDs are accepted per process; additional new IDs receive `request_limit_reached`.
- Callers should use a fresh unique ID for each request and retain it for retries. UI mutation requests must also use the generation from the latest state response.
- Screenshot writes validate the PNG signature and atomically rename a temporary file. Path separators, traversal, non-PNG names and existing files are rejected.

The inherited pipe is a local process capability, not an authenticated network API. Only a trusted parent should launch the runner and provide its stdin/stdout. Do not expose these pipes to untrusted clients. The runner is excluded from the default build. It has not been accepted against a physical Windows window or paired visually against upstream Hermes.

`actions.drag` resolves exactly one accessible source (`test_id` or `role`/`name`), uses the source bounds center, and sends primary-button pointer events to the destination. Prefer semantic target locators (`to_test_id` or `to_role`/`to_name`); their accessible bounds determine the destination center. Logical coordinates (`to_x`, `to_y`) remain available for geometry-specific gestures such as placing a panel in a split zone. Provide one destination form only. The destination must be inside the visible UI, `steps` defaults to 8 and is bounded from 1 to 32, and every drag requires `expected_state_generation`. Dock tab labels are prefixed `Dock tab` so they remain uniquely addressable beside workspace route tabs. Dragging a tab over another tab in its current group commits canonical `ReorderPanel` on release; dropping it outside that strip follows the normal panel docking path. Split resize controls are `Button` locators named `Resize split {split_node_id} divider {one_based_index}`; dragging one previews the adjusted pair and commits a canonical `ResizeSplit` on pointer release.

The current session composer appears in the accessibility tree as `MultilineTextInput` / `Message composer` / `chat.composer`. It is disabled while the backend is offline; mutation requests against it fail with `target_disabled` and do not inject text. While an active submission is pending, the composer also exposes enabled `Button` / `Stop response` / `chat.stop`; during the correlated interrupt request it changes to `Stopping response` while retaining the same test ID. The diagnostic state reports `chat.can_stop`, the runtime session ID and pending flags without message text.

## Example

```json
{"version":1,"id":"state-1","method":"state.get","params":{}}
{"version":1,"id":"click-1","method":"actions.click","params":{"role":"Button","name":"Capabilities","expected_state_generation":0}}
{"version":1,"id":"capture-1","method":"screenshots.capture","params":{"filename":"review-001.png"}}
```

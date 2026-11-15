# Hermes DoofusDock architecture

## Current implementation

- `doofusdock-core::LayoutDocument` is the only persisted owner of panel IDs and geometry.
- `DockingService` validates revision-bound panel-center operations.
- `doofusdock-presentation::project` derives the visible panel rectangles from the canonical document.
- `LayoutStore` performs bounded, atomic layout persistence.
- The egui host renders app-owned panel content; the Rust application owns its content and navigation.
- Hermes runtime and Gateway remain authoritative for conversations and capabilities. `gateway.rs` connects over the Hermes gateway protocol and maps typed session, prompt, approval, clarification and capability operations into app events. The authenticated Doofus bridge is not connected yet.
- Session identity is profile plus stored session ID when a profile is known. The active chat carries that profile through resume, prompt submission, interruption, approvals and clarification RPCs. Session IDs that collide across profiles remain separate in the list. The registered `open_session` action accepts both identity fields; path-only navigation rejects ambiguous IDs.
- The isolated agent UI process exposes versioned commands over inherited stdio. Registered actions run through the application handler, advance the egui frame, and then return state; screenshots render that updated frame. This headless process has no Hermes gateway connection, so it verifies the UI control surface rather than live backend behavior.
- Active runtime session renames fetch the current revision with profile-scoped `session.resume`, then send `session.mutate` with `operation: rename`, the runtime ID, revision, and request ID. An uncertain reply retains the exact mutation tuple in the current gateway actor; the dialog can retry that tuple without a fresh snapshot. After reconnect, an exact retry is unavailable and stops before sending a new mutation. Other stored sessions use the gateway's profile-scoped HTTP mutation snapshot and a fresh one-use ticket for the PATCH. A confirmed response title is required before the sidebar, archived list, and search results update. Authenticated live runtime behavior and cross-reconnect recovery remain to be verified.
- Prompt submission carries the UI's stable `submission_id` and validates the canonical receipt's `admission_id`, status, and session destination. Pending sends are keyed by profile plus runtime session, so each open chat can retain its own identity and retry state. An uncertain RPC reply preserves the optimistic transcript row and exposes an agent-accessible retry using the same ID. Completion settles only the matching admission, including when completion arrives before the RPC receipt. An `unknown` receipt triggers profile-scoped `prompt.resolve_unknown` with the admission ID and execution generation. Durable retry state across app restart and replay of background-session transcript events remain incomplete.

The initial tree uses four zones: files/tasks on the left, chat in the center, preview/Doofus on the right, and terminal/tests/logs at the bottom. The layout can form tab groups through panel drag and drop. Floating windows, split resizing, tab selection, hide/close behavior, and profile switching are not implemented in this slice.

## Intended boundaries

The planned app is a Rust + egui client. The Hermes gateway remains responsible for conversations, tools, models, sessions, and approvals. Hermes-Doofus remains responsible for tasks, plans, requirements, evidence, vault data, and routing decisions. DoofusDock owns only workspace composition. Rust reducers and transport adapters will translate typed messages into views; widgets will not make backend decisions.

The app uses the DoofusDock crates at pinned revision `f6eb49b9e488f4e851d8d3abe18e95d296201bba`. The source checkouts were dirty at review time, so they are not path dependencies and their existing edits were left untouched.

## Protocol implementation sequence

1. Continue implementing the fixed Hermes snapshot `8783054ef6cbafd86f10892241436862c1171a66`; do not update the parity denominator when upstream moves.
2. Complete the typed Hermes `hermes-gateway-v1` adapter, including attach/reconnect, replay, event reduction, remaining server-request methods, and no duplicate prompt submissions.
3. Add the authenticated Doofus bridge adapter, including profile-bound scope and event cursor recovery.
4. Build remaining application-owned widgets behind typed reducers and ports, pairing each behavior with the fixed source and acceptance evidence.

## Compatibility and evidence

The Hermes Desktop Plugin SDK is JavaScript/React; DoofusDock extensions are Rust contracts. Existing plugins are not automatically compatible. A sandboxed compatibility path or per-plugin native migration decision remains open. The preserved package contains 565 research rows: 499 stock Hermes Desktop requirements are in scope and 66 Hermes-Doofus requirements are excluded. Capability states are not promoted without current evidence.

- The canonical DoofusDock layout now supports registered profile-scoped session pane descriptors through revision-bound add/select/close operations and profile switching preserves open tiles. The UI still uses its one-item placeholder and the panes do not yet own/render independent chat state; see [Hermes Native PLAN.md](../apps/hermes-native/docs/PLAN.md), revision 46.
- The chat strip is currently a single-item placeholder. Pinned Hermes models session tabs as dynamic session-tile panes in the canonical workspace pane tree, so parity requires pane-registry and DockingService support plus independent per-session view state; an app-owned parallel tab list would split layout authority. The ordered implementation and acceptance plan is in [Hermes Native PLAN.md](../apps/hermes-native/docs/PLAN.md), revision 45.
- The active transcript uses egui_commonmark with GitHub-style tables/task lists and syntect highlighting. A small vendored backend change replaces an unsupported emoji copy glyph with a sized, accessible Copy/Copied button. Stored transcripts can fetch older 120-message pages with profile and latest-order validation; app state rejects stale replies and merges page overlap safely. This is not the source's virtualized scroll window or retention behavior, and live gateway acceptance remains open. Streamdown streaming repair, media, math plugins, transcript directives, rich artifacts/cards, and exact source typography are still incomplete.

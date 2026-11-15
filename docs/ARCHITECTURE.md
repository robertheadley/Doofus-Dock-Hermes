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

- The canonical Dock supports profile-scoped session pane descriptors. Each visible tile temporarily renders its own keyed transcript, composer, paging state and visible approval/clarify cards, then restores the primary chat state. Existing sessions and deep links front canonical tiles; blank sessions create unlisted tiles and first admitted prompts add them to the sidebar. Hidden submission receipts route to the matching cached view. The 32-view cache remains in-memory and persistent scroll remains open. Approval and batch clarification requests route to their unique profile-scoped pane, including semantic response actions while hidden; authenticated live receipts remain unverified. See [Hermes Native PLAN.md](../apps/hermes-native/docs/PLAN.md), revision 50.
- The session tab strip is the canonical DoofusDock tab group and exposes close controls for session tiles. Closing an active tile asks for confirmation and closes the pane without interrupting Hermes. The primary workspace pane remains a separate tab; `Ctrl+W` and agent `close_active_tab` promote the nearest profile-scoped session tile into primary or reset an occupied primary chat to New session. Background server requests also route to their unique profile-scoped pane. Reuse of edited drafts, close-target precedence across tool zones and complete keyboard/focus/overflow behavior remain open.
- The active transcript uses egui_commonmark with GitHub-style tables/task lists and syntect highlighting. A small vendored backend change replaces an unsupported emoji copy glyph with a sized, accessible Copy/Copied button. Stored transcripts can fetch older 120-message pages with profile and latest-order validation; app state rejects stale replies and merges page overlap safely. This is not the source's virtualized scroll window or retention behavior, and live gateway acceptance remains open. Streamdown streaming repair, media, math plugins, transcript directives, rich artifacts/cards, and exact source typography are still incomplete.


### Background prompt ownership

Approval and batch clarification requests are assigned by runtime session ID plus profile to exactly one open native session pane. Prompt state retains that profile, and both human controls and registered semantic actions respond through the same handler using the stored owner scope, even while another pane is selected. Requests with missing/ambiguous ownership or a profile conflicting with the attached gateway endpoint fail closed. Local tests verify routing and UI actions; live authenticated gateway receipts remain unverified.

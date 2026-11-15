# Hermes · DoofusDock

Native Rust and egui desktop shell for Hermes, using DoofusDock's persisted `LayoutDocument` as the workspace geometry authority.

## Current state

The active transcript supports optional local timestamps with millisecond precision. Hermes Native reads the selected profile's `display.timestamps` value from the pinned `/api/config` endpoint; `/timestamps`, `/timestamps on`, `/timestamps off`, and `/timestamps status` provide the same window-only override without submitting a model prompt. A distinct offline review capture is available at `artifacts/agent-ui/transcript-timestamps-20261115.png`; exact locale formatting/tool lifecycle coverage and source-paired visual acceptance remain open.

This is an in-progress replacement, not a one-to-one replacement yet. It launches a native four-zone workspace, persists the canonical Dock layout, and wires pane drags to center-tab or edge-split operations. The Hermes gateway adapter supports session browsing, creation, resume, streaming prompts, stop, approval and clarification responses, capability discovery, and scoped session mutations. Prompt submission carries a stable submission ID and validates the returned admission identity; uncertain acknowledgements can be retried with the original ID. Active runtime rename uses revision-fenced `session.mutate` with an exact retry after an uncertain receipt; stored-session rename uses a fenced HTTP patch. Cross-profile session rows retain their profile through resume and chat RPCs. The registered agent action `open_session` takes both session ID and profile; path navigation rejects IDs shared by multiple profiles. The active transcript now uses a native CommonMark renderer with GitHub-style tables, ordered/task lists, inline emphasis and links, block quotes, syntax-highlighted fenced code, and an accessible copy control. Long sessions expose a profile-scoped, bounded “Show earlier messages” read path with order validation and stale-session guards; virtualized scroll-window parity and live gateway acceptance remain open. Streaming repairs, media, transcript directives, specialized math, artifact promotion, and full source Streamdown styling remain incomplete. The Dock model has a tested foundation for dynamic, profile-scoped session-pane descriptors and canonical add/select/close operations. Route switching now retains each profile+session transcript and composer draft in a bounded in-memory view cache, and uniquely identified background message/tool events update the correct cached session. Each dynamic Dock pane renders its own transcript and composer from a bounded in-memory profile+session view cache; uniquely identified background events and submission receipts update the matching cached session. Existing sessions open in Dock tiles, blank sessions remain unlisted until their first admitted prompt, and closing a busy tab confirms while leaving Hermes running. Approval and batch clarification requests route to their unique profile-scoped pane, and semantic actions retain that scope when responding. Session resume receipts carry request, session and profile identity; stale results are ignored and recoverable errors render with Retry in their owning pane. Authenticated live receipts, Bot Mode owner-route handling, attachment-aware drafts and full close-focus/overflow parity remain open. New session reuses only an empty profile-matching main draft or a known idle, empty, unlisted tile; edited drafts remain in place. Primary-pane close now follows the pinned source: `Ctrl+W` and agent action `close_active_tab` promote the nearest session tile or reset an occupied primary chat to New session. The close target now prefers a session tile in the hovered or last-clicked eligible Dock tab group before falling back to workspace; keyboard-focus traversal and tool-tab handoff remain incomplete. The Doofus bridge and many Hermes screens/workflows are still incomplete. No sample conversation, task, model, terminal, or vault data is fabricated. Live drag/drop persistence has not yet been exercised.

The parity target is the downloaded Hermes source snapshot `8783054ef6cbafd86f10892241436862c1171a66`, documented in [the reviewed research package](docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/README.md). This target stays fixed; later upstream changes do not expand the parity scope. The reference package retains 565 research rows: 499 stock Hermes Desktop requirements are in scope, while 66 Hermes-Doofus requirements are excluded by the approved scope. The 499 in-scope behaviors still require implementation and source-paired acceptance before this can be called a 1:1 replacement.

## Run on Windows

Requirements: Rust 1.95 or later, Cargo, and a Windows desktop session.

```powershell
cargo run
```

The saved layout is stored under `%APPDATA%\HermesDoofusDock\layouts\hermes-main.json` using DoofusDock's atomic `LayoutStore`.

## Agent UI control

`cargo run --features agent-automation --bin hermes-automation -- --stdio` starts an isolated headless UI with versioned JSON requests over inherited stdio. It exposes the accessibility tree, registered actions, state, events, semantic scrolling, and screenshots without opening a native window or connecting to the Hermes gateway. A successful action advances the UI frame before state or screenshot readback. Native window control and authenticated live-gateway automation remain to be implemented.

## Validation

```powershell
cargo check --all-targets
```

See [the implementation backlog](docs/BACKLOG.md) for the remaining bridge, application widgets, support functions, and source-paired parity work.

# Hermes · DoofusDock

Native Rust and egui desktop shell for Hermes, using DoofusDock's persisted `LayoutDocument` as the workspace geometry authority.

## Current state

This is an in-progress replacement, not a one-to-one replacement yet. It launches a native four-zone workspace, persists the canonical Dock layout, and wires pane drags to center-tab or edge-split operations. The Hermes gateway adapter supports session browsing, creation, resume, streaming prompts, stop, approval and clarification responses, capability discovery, and scoped session mutations. Prompt submission carries a stable submission ID and validates the returned admission identity; uncertain acknowledgements can be retried with the original ID. Active runtime rename uses revision-fenced `session.mutate` with an exact retry after an uncertain receipt; stored-session rename uses a fenced HTTP patch. Cross-profile session rows retain their profile through resume and chat RPCs. The registered agent action `open_session` takes both session ID and profile; path navigation rejects IDs shared by multiple profiles. The active transcript now uses a native CommonMark renderer with GitHub-style tables, ordered/task lists, inline emphasis and links, block quotes, syntax-highlighted fenced code, and an accessible copy control. Long sessions expose a profile-scoped, bounded “Show earlier messages” read path with order validation and stale-session guards; virtualized scroll-window parity and live gateway acceptance remain open. Streaming repairs, media, transcript directives, specialized math, artifact promotion, and full source Streamdown styling remain incomplete. The Dock model now has a tested foundation for dynamic, profile-scoped session-pane descriptors and canonical add/select/close operations; Session tabs are still a one-item placeholder; the pinned Desktop uses dynamic panes with per-session state, and the plan now tracks the required DoofusDock-native implementation. The Doofus bridge and many Hermes screens/workflows are still incomplete. No sample conversation, task, model, terminal, or vault data is fabricated. Live drag/drop persistence has not yet been exercised.

The parity target is the downloaded Hermes source snapshot `8783054ef6cbafd86f10892241436862c1171a66`, documented in [the reviewed research package](docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/README.md). This target stays fixed; later upstream changes do not expand the parity scope. The reference package retains 565 research rows: 499 stock Hermes Desktop requirements are in scope, while 66 Hermes-Doofus requirements are excluded by the approved scope. The 499 in-scope behaviors still require implementation and source-paired acceptance before this can be called a 1:1 replacement.

## Run on Windows

Requirements: Rust 1.95 or later, Cargo, and a Windows desktop session.

```powershell
cargo run
```

The saved layout is stored under `%APPDATA%\HermesDoofusDock\layouts\hermes-main.json` using DoofusDock's atomic `LayoutStore`.

## Agent UI control

`cargo run --features agent-automation --bin hermes-automation -- --stdio` starts an isolated headless UI with versioned JSON requests over inherited stdio. It exposes the accessibility tree, registered actions, state, events, and screenshots without opening a native window or connecting to the Hermes gateway. A successful action advances the UI frame before state or screenshot readback. Native window control and authenticated live-gateway automation remain to be implemented.

## Validation

```powershell
cargo check --all-targets
```

See [the implementation backlog](docs/BACKLOG.md) for the remaining bridge, application widgets, support functions, and source-paired parity work.

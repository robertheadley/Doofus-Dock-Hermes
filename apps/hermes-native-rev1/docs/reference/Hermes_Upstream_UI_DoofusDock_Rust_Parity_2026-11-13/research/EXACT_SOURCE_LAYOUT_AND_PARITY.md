# Exact upstream Hermes GUI — research and measurable parity contract

This is a **native Rust/egui UI replacement** for the original, current upstream Hermes **desktop** interface. The agent is the upstream Hermes Python gateway, not Doofus-Hermes. A future Doofus-Hermes plugin port is deliberately excluded. The separate web Dashboard is not a desktop parity target, even when it shares backend APIs.

## Source of visual truth

Pin a specific upstream Hermes SHA and DoofusDock SHA. A source-file feature inventory is NOT sufficient evidence for pixel parity. Capture **source native desktop running on Windows** at 100%, 125%, 150% Windows display scale (where feasible), the same locale, theme, data, font availability and viewport; compare against the Rust app with the same exact fixtures. Publish at least one screenshot for each major route, a screenshot of first launch, popover states, empty/loading/error states and dialog states. Dynamic content must be stabilized or masked with declared exclusion coordinates, not silently ignored. Compare layout rectangles, contrast, icon glyphs, font metrics, baseline alignment, border radii, spacing, scrollbar/overlay behavior, z-order and disabled/hover/focus states. Human review remains necessary even if image metrics pass.

**True 1:1** means: visual, interaction, behavior, accessibility, persistence, security and lifecycle parity. A pixel-perfect static screenshot with inert menus is a failure. A functionally equivalent new skin that changes original Advanced layout is also a failure.

## Verified source-defined Advanced default tree

`apps/desktop/src/app/contrib/layout-presets.ts` defines `DEFAULT_TREE` as a horizontal split of:

- `sessions`: weight **1**, independent left group (`grp-sessions`)
- `workspace`: weight **3.4**, main chat/session route group (`grp-main`)
- right region: weight **1.25**, vertical split (`spl-right`) with top region weight **1.6** and bottom `terminal` weight **1**; top splits horizontal `review` weight **1** and `files` weight **1.2**.

The file, review and terminal panels may be hidden/resting independently; their location in the tree does not mean permanent visibility. A dynamic file preview opens as its own pane next to the file tree, **not** a tab stacked into the file tree. Source session and preview tiles are dynamically registered, and a saved layout template must not persist specific open-session IDs.

The source supports `Basic`, `Focus`, `Terminal Deck`, `Quad`, `Sidebar left`, `Sidebar right` and hidden onboarding `Demo`. The exact trees (normalized, not serialized DoofusDock documents) and source constraints are in `maps/HERMES_SOURCE_LAYOUT_TREES.json`.

The source `store/layout.ts` records a default 237px session rail and max 360px; `app/layout-constants.ts` records a 640px dock floor for responsive collapse. These are source constants, not a claim that a live layout uses those pixel widths at all window sizes.

## Advanced versus Simple policy

Upstream defaults to **Advanced**. The `Simple` mode shadows display preferences, it does NOT overwrite them. Effective visibility follows `sessionReveal ?? policy[mode] ?? userPreference`. This matters when a user toggles a terminal temporarily and switches modes or restarts. Implement both original modes independently of any extra Codex/Antigravity/IDE profile selector.

## Upstream route/page semantics

The `chat` workspace (new or existing session) and full workspace pages (`/capabilities`, `/messaging`, `/artifacts`, contributed routes) remain within the workspace pane; overlay views (`/settings`, `/command-center`, `/cron`, `/profiles`, `/agents`, `/starmap`, `/webhooks`, `/session-import`) are overlays. See `maps/UPSTREAM_DESKTOP_ROUTES.json`. Keep navigation semantics stable while panels move.

## Actual Rust implementation boundary

`doofusdock-core` owns `LayoutDocument`, docking operations, revisions, user profiles, persistence, constraints and audit. `doofusdock-presentation` owns transient Morph projections, retargeting and reduced-motion sampling. `doofusdock-render-egui` and `doofusdock-host-egui` own chrome/viewports. The *Hermes Rust app* must implement real widgets and interaction: transcript, composer, terminal emulator, file tree, Git review, settings, models, plugins, session manager, overlays and attachment previews.

The current renderer reports shader chrome/application effects as **not implemented**; do not use those as prerequisites or claim them present. The Dock public source has synthetic and limited Windows host verification, but not a physical cross-platform parity guarantee; execute Windows live acceptance.

## Morph requirements

Only after Stage 1 is one-to-one in its Hermes preset, introduce additional **named geometry profiles**: Codex-inspired, Antigravity-inspired, IDE-inspired, Doofus-inspired, and user-defined. These are alternate placements of existing instances of identical widgets; they do not change the Hermes backend or implement Doofus-specific features. The user can return to exact Hermes layout at any time.

Morph panel containers by stable ID. The existing transcript, terminal PTY, active agent session, file selection, draft, focus and persistent state must survive. Compile target geometry through the public profile/operation API with policy preflight; commit accepted target arrangement once; sample `Morph` transiently; support mid-transition retarget/reverse, cancel, zero-duration reduced motion, resize and detached viewports. Never persist each animation frame into canonical layout. Preserve focus ownership and clip/z-order when panels cross.

This is a **research-derived specification, not a claim that the Rust application has been built**.

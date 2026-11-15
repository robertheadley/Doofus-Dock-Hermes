# Source audit

## Pins

- Hermes source selected at implementation start: `NousResearch/hermes-agent@8358fc7bb4ce18fbee8d4bf6659dd155c2a03783`.
- DoofusDock: `robertheadley/doofusdock@f6eb49b9e488f4e851d8d3abe18e95d296201bba`.
- The supplied ZIP references Hermes `ffdc7eeb25343df1a7ee84503ba552538db9af72`. The audit also observed `6da444ebda7533429b655aeca2b02d77fd900238`; the upstream branch advanced again before the source clone. Relevant layout, mode, route, sidebar-width and responsive source files were verified unchanged between the ZIP pin and the previous audit pin. They must be rechecked at every source pin refresh.
- Source checkout: `../../.staging/hermes-upstream-8358fc` from this crate. It is isolated from the installed Hermes application.

## Advanced default tree

Source: `apps/desktop/src/app/contrib/layout-presets.ts`.

```text
row [1, 3.4, 1.25]
├── group(sessions)
├── group(workspace)
└── column [1.6, 1]
    ├── row [1, 1.2]
    │   ├── group(review)
    │   └── group(files)
    └── group(terminal)
```

`review`, `files`, and `terminal` have independent resting/visibility behavior. A file preview is a dynamic pane beside the file tree. Session/preview IDs are not written into reusable geometry. Source pane IDs are `sessions`, `workspace`, `review`, `files`, and `terminal`.

## Source presets and mode policy

- Advanced default is the source Default preset; Advanced preserves user display preferences.
- Basic: sessions; workspace above a resting terminal; review/files in their own right column.
- Focus: sessions plus workspace/files/review tabs above the terminal.
- Terminal Deck: sessions/workspace/files+review tabs above a bottom terminal.
- Quad: sessions+files tabs and workspace above terminal/review.
- Sidebar left/right are Simple-tier Basic layouts; Demo is unlisted and chat-only.
- Simple is a preference-shadowing mode, separate from the selected layout. Effective visibility is `sessionReveal ?? policy[mode] ?? userPreference`. Simple rests files, review and terminal closed and applies other source policy values; it does not overwrite Advanced preferences.
- Sidebar width defaults to 237px (max 360px); below a 640px dock width the rails become reveal overlays.

## Routes

Workspace pages: `/`, `/:sessionId`, `/capabilities`, `/messaging`, `/artifacts`, and contributed full-page routes. Overlays: `/session-import`, `/agents`, `/command-center`, `/cron`, `/profiles`, `/settings`, `/starmap`, `/webhooks`. Contributed paths must not be parsed as session IDs.

## Baseline and current gaps

The archive maps 491 in-scope stock capabilities and defers 108 out-of-scope records. It supplies source maps and a test matrix but no source/target screenshots. The previous parent-workspace prototype has four Doofus/Hermes zones, no upstream layout or mode parity, no Hermes Gateway adapter, and an out-of-scope Doofus pane. This crate is separate; no edits were made to that prototype or to either source checkout.

Current DoofusDock APIs selected: `LayoutDocument` + `DockingService` for the one canonical layout; `WorkspaceProfile` / `apply_workspace_profile` / `plan_profile_switch` for source geometry; `Morph` and `render_visual_snapshot` only in the later profile stage; `doofusdock-render-egui` for panel chrome and tabs; `doofusdock-host-egui` for native viewport support. The Hermes app owns all chat, session, terminal, file, Git, settings and other Hermes widgets.

## Source preview setup

The pinned Desktop was installed/launched from the isolated source checkout with Node 26.12.1 and pnpm 12.15.0, a temporary `HERMES_HOME`, `HERMES_DESKTOP_USER_DATA_DIR`, `APPDATA`, and `LOCALAPPDATA`. The initial preview shows the Sessions sidebar, New session workspace and Setup Hermes Desktop overlay with Backend offline. The isolated source checkout needed a temporary declaration-only `.d.mts` shim because its electron TypeScript build did not resolve an imported test `.mjs` helper; no runtime source file was changed. The shim remains untracked in the isolated preview checkout. A cleanup attempt was blocked by local command policy; it does not alter runtime source or affect the installed app.

# Implementation plan — source parity first, morphing second

No time estimates. Plan first, source evidence first, implementation in independently verifiable slices. Do not alter the user's live Hermes/Doofus installations without explicit acceptance and a recoverable migration plan.

## Stage 0: pin and capture current upstream

- Refresh upstream Hermes and DoofusDock HEADs; pin exact commits, dependencies, licences, upstream GUI build flags, Windows theme/fonts and source schema.
- Run upstream Hermes Desktop in a controlled Windows test profile, capture initial GUI, all routes, overlays, modal and destructive-state screens, keyboard map, loading and fail states; record normalized geometry/hover/focus/tab order. Capture same fixture/size/DPI and expected state transitions.
- Compare `maps/HERMES_SOURCE_LAYOUT_TREES.json` to current source and runtime. Update/source-tag feature map. Unknowns remain unknown; prioritize **all stock native Hermes UI** rather than the old Doofus plugin or separate web Dashboard.

Gate: no unverifiable 'pixel perfect' claims, baseline screenshots and source SHAs saved; every feature tied to a source route/component/condition. Do not start new custom layouts.

## Stage 1: native shell and exact source layout

- Use DoofusDock core/canonical operations, profiles, render-egui and egui native host. Implement the upstream `DEFAULT_TREE` exactly as source-defined, and its titlebar, statusbar, session sidebar, center workspace and right rail/resting terminal behavior.
- Implement original Basic, Focus, Terminal Deck, Quad, mirrored Sidebar presets and hidden Demo onboarding profile. Implement original Advanced/Simple mode **preference-shadowing** separately from layout switch.
- Dynamic session, preview and route tiles must not be serialized into reusable geometry presets. Preserve panel identity, profile ownership, and hide/close distinctions.
- Capture and compare native source/Rust screenshots at matching fixtures, dimensions, scale, font and theme. No alternate-inspired profile yet.

Gate: source layout tree and mode contracts pass; same screen geometry and reveal/collapse/resize/drag behavior; screenshot review recorded.

## Stage 2: gateway, sessions, chat and native widgets

- Connect to the **current upstream Hermes** gateway through typed Rust JSON-RPC transport and `client.capabilities`; correct server-request replies, idempotent replay, stored vs live sessions, source-of-truth ownership, missing-capability fail-closed behavior.
- Implement stock chat transcripts, tool/reasoning cards, Markdown, streaming, composer, attachments, model controls, history/rewind, session/sidebar, profile switching, status and command palette.
- Add real files, preview, artifacts, browser/security boundaries, terminal emulator/PTy lifetime, Git review, worktrees, subagents, memory graph, voice, Quick Entry/HUD, cron/webhooks/messaging, settings/onboarding, capability/skills/MCP, plugin install/manage, themes/accessibility/localization and native menus.
- Preserve backend rules and original UX: route overlays vs pages, prompts/tools/request UI, third-party contribution handling documented honestly.

Gate: every stock target P0/P1 behavior has positive and failure-path evidence; no placeholder widgets; upstream remains rollback.

## Stage 3: visual and behavioral parity signoff

- Execute `planning/ACCEPTANCE_MATRIX.csv` with side-by-side Windows runtime source/target, screen captures, keyboard accessibility, mouse/menu actions, protocol receipts, focus state, correct negative paths and persisted layout.
- Record unsupported features explicitly; address all blocking differences. Scripted image diff metrics are aids, not a substitute for human review. Verify source colours/fonts/icons and per-state behavior.

Gate: user accepts original Hermes UI experience as 1:1. Source mode has no forced custom Doofus style.

## Stage 4: morphing and alternate layout profiles

- Expose chooser with `Hermes Advanced`, original simple/basic/focus/etc., `Codex-inspired`, `Antigravity-inspired`, `IDE-inspired`, `Doofus-inspired`, custom.
- Map every profile into **same panel registry**, never new copies of app state. A target profile defines slots, not a replacement transcript or a second PTY. Respect panel visibility/resting and dynamic panes.
- Use public DoofusDock `WorkspaceProfile`, `LayoutDocument`, canonical operation batches, `Morph` and snapshot egui renderer. Sample animations as transient geometry; preserve active resources, focus, scroll position, drafts, clipboard, in-flight agent activity and terminal process.
- Implement interrupt/retarget/reverse, resize mid-flight, reduced-motion immediate final state, escape/cancel and round trip to **source-identical** Hermes preset; allow custom preset save/load.

Gate: exact Hermes screenshot and behavior can be restored after any number of profile changes; no duplicated sessions/processes or lost content; animation results stable under interrupt/reconnect.

## Stage 5: hardening, release, eventual Doofus plugin extension seam

- Run real Windows DPI, multiple monitors, IME, accessibility, reduced motion, low-resource, crash/restore, mixed profile/gateway, session reconnect and long-running terminal acceptance.
- Keep old Hermes executable/working profile as rollback; no forced migration. Verify authorship, third-party notices and Apache/MIT/etc. licences for every imported component.
- Future Doofus-Hermes port is **not part of this delivery**; document clean API boundaries to add it later without changing stock Hermes UX.

Gate: full completion ledger, verified evidence and reversible cutover. No Doofus Hermes plugin changes in this scope.

## Repeatable phase evidence row

`feature_id | source path/commit | native view path/commit | source screenshot | Rust screenshot | same test data | expected behavior | negative path | backend request/event | accessibility receipt | morph/visibility preservation | status | reviewer`

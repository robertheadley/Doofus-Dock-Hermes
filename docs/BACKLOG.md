# Hermes DoofusDock backlog

The complete target is the 565 capability inventory in `docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/`, scoped to the downloaded Hermes source snapshot `8783054ef6cbafd86f10892241436862c1171a66`. That snapshot is the fixed parity target; later upstream updates do not expand scope. The reference package passes its offline consistency check. Completion requires source-paired validation for every in-scope behavior.

## Delivered in this slice

- Native Rust/egui window and four-zone workbench.
- DoofusDock-backed canonical layout, projection, persistence, center-to-tab and edge-split drag operations (wired; live drag/drop not yet verified).
- Hermes gateway session listing and creation, transcript resume, streamed prompt submission, interruption, approval and clarification responses, capability discovery, and scoped session metadata mutations.
- Archived-session browsing and restoration, with profile-aware session identity and profile routing through resume and chat actions. Human row clicks and the registered agent `open_session` action use the same scoped navigation handler.
- Rename dialog, profile-scoped canonical `session.mutate` rename for the active runtime session, and revision-fenced HTTP title patch for other stored sessions, with list updates after the confirmed receipt. Uncertain runtime receipts can retry the retained mutation on the same gateway actor.
- Prompt submission includes stable submission identity, validates the canonical admission receipt, correlates completion by admission ID, and exposes an accessible exact retry after uncertain transport outcomes.
- Explicit disconnected/empty states; no fabricated domain data.

## Remaining, in dependency order

1. Complete profile-aware gateway lifecycle and event isolation, including durable retry identities, background-session event replay, cross-reconnect rename recovery, and HTTP mutation retry identity; then source-paired chat parity: history replay, edit/regenerate/rewind, attachments, Streamdown-specific streaming repair, math/media/directives, rich diff/artifact/tool cards, and reconnect/recovery. CommonMark/GFM structure, links, tables, and syntax-highlighted code are now rendered natively but remain unverified against matched source screenshots.
2. Implement files, previews, artifacts, terminal/PTy, Git and worktree views against the pinned source contracts.
3. Implement models, profile/settings management, skills/plugins/MCP, messaging, scheduling, logs/analytics, voice, onboarding, and dashboard management.
4. Implement the authenticated Doofus Control and Models & Quota views while preserving Python truth/evidence ownership and vault secrecy.
5. Finish DoofusDock parity: tab selection/reorder, resize, float/re-dock, custom profiles, recovery, accessibility and native-window lifecycle.
6. Resolve plugin compatibility and remaining keyboard, focus, accessibility, localization, high contrast, reduced motion, stale/offline/retry, and large-data behaviors.
7. Complete all 565 source-to-target acceptance cases, negative-path and permission probes, Windows visual/interaction checks, rollback and provenance review. Only then consider a cutover.

## Current limitations

- Hermes gateway coverage is partial; many RPCs, events, and screens in the fixed snapshot are not implemented. The authenticated Doofus bridge is not connected.
- The agent UI stdio process is headless and gateway isolated. Native window control and authenticated live-gateway automation still need implementation and verification.
- Models and Doofus views remain incomplete; only the wired gateway/session slices are operational.
- Center/tab and edge-split docking commands are wired, but live drag/drop persistence is unverified. Resize, tab picker, float/detach, and hide/close lifecycle remain unavailable.
- No complete feature crosswalk against current Hermes source, no end-to-end evidence matrix, and no cutover decision.

# Hermes DoofusDock backlog

The preserved research inventory has 565 rows in `docs/reference/Hermes_DoofusDock_Rust_Parity_2026-11-12/`: 499 stock Hermes Desktop requirements are in scope and 66 Hermes-Doofus requirements are excluded by the approved scope. The fixed target is downloaded Hermes snapshot `8783054ef6cbafd86f10892241436862c1171a66`; later upstream updates do not expand scope. The reference package passes its offline consistency check. Completion requires source-paired validation for every in-scope behavior.

**Acceptance progress snapshot (2026-11-15): 0/499 in-scope rows accepted (0%).** All 565 matrix rows currently say `pending`, including the 66 excluded research rows. Many partial implementation slices exist, but the project does not track a defensible weighted implementation percentage.

## Delivered in this slice

- Native Rust/egui window and four-zone workbench.
- DoofusDock-backed canonical layout, projection, persistence, center-to-tab and edge-split drag operations (wired; live drag/drop not yet verified).
- Hermes gateway session listing and creation, transcript resume, streamed prompt submission, interruption, approval and clarification responses, capability discovery, and scoped session metadata mutations.
- Archived-session browsing and restoration, with profile-aware session identity and profile routing through resume and chat actions. Human row clicks and the registered agent `open_session` action use the same scoped navigation handler.
- Session-tab close and restore parity: bounded profile-scoped LIFO restore stack, active-route handoff on close, canonical DoofusDock reopen/focus, native platform shortcut, and shared registered UI-agent action. Local tests cover behavior; source-paired Windows acceptance remains part of the 499-row gate.
- Rename dialog, profile-scoped canonical `session.mutate` rename for the active runtime session, and revision-fenced HTTP title patch for other stored sessions, with list updates after the confirmed receipt. Uncertain runtime receipts can retry the retained mutation on the same gateway actor.
- Prompt submission includes stable submission identity, validates the canonical admission receipt, correlates completion by admission ID, and exposes an accessible exact retry after uncertain transport outcomes.
- Explicit disconnected/empty states; no fabricated domain data.
- Bounded older-transcript pagination with a profile-scoped one-use ticket, exact session/order/offset validation, stale-session response fencing, overlap-safe chronological merge, and an accessible “Show earlier messages” control. Local HTTP and AccessKit fixtures cover success and negative paths; live gateway acceptance and source scroll-window parity remain open.
- Transcript find-in-page: accessible native search bar, case-insensitive matching over loaded transcript rows, match count, previous/next and keyboard stepping, Escape close, message-card highlight and scroll-to-match. The fixed-source PAR-0058 acceptance remains open for exact rendered-text highlights, other rendered panes, and paired Windows evidence.
- Transcript message roles remain visible and active chat now reads `display.timestamps` from the selected profile's authenticated `/api/config`, then renders local millisecond times from stored metadata or stream lifecycle boundaries. `/timestamps [on|off|toggle|status]` does not call the model or alter prompt context and overrides config for this window only. Source locale/time tooltip details, tool-run timestamps, and source-paired PAR-0065 acceptance remain open.

## Remaining, in dependency order

1. Complete profile-aware gateway lifecycle and event isolation, including durable retry identities, background-session event replay, cross-reconnect rename recovery, and HTTP mutation retry identity; then source-paired chat parity: history replay, edit/regenerate/rewind, attachments, Streamdown-specific streaming repair, math/media/directives, rich diff/artifact/tool cards, and reconnect/recovery. CommonMark/GFM structure, links, tables, and syntax-highlighted code are now rendered natively but remain unverified against matched source screenshots.
2. Implement files, previews, artifacts, terminal/PTy, Git and worktree views against the pinned source contracts.
3. Implement models, profile/settings management, skills/plugins/MCP, messaging, scheduling, logs/analytics, voice, onboarding, and dashboard management.
4. Implement the authenticated Doofus Control and Models & Quota views while preserving Python truth/evidence ownership and vault secrecy.
5. Finish DoofusDock parity: tab selection/reorder, resize, float/re-dock, custom profiles, recovery, accessibility and native-window lifecycle.
6. Resolve plugin compatibility and remaining keyboard, focus, accessibility, localization, high contrast, reduced motion, stale/offline/retry, and large-data behaviors.
7. Complete source-to-target acceptance for the 499 in-scope Hermes Desktop requirements, including negative-path and permission probes, Windows visual/interaction checks, rollback and provenance review. Keep the 66 excluded research rows pending and out of the cutover gate.

## Current limitations

- Hermes gateway coverage is partial; many RPCs, events, and screens in the fixed snapshot are not implemented. The authenticated Doofus bridge is not connected.
- The agent UI stdio process is headless and gateway isolated. Native window control and authenticated live-gateway automation still need implementation and verification.
- Models and Doofus views remain incomplete; only the wired gateway/session slices are operational.
- Center/tab and edge-split docking commands are wired, but live drag/drop persistence is unverified. Resize, tab picker, float/detach, and hide/close lifecycle remain unavailable.
- No complete feature crosswalk against current Hermes source, no end-to-end evidence matrix, and no cutover decision.

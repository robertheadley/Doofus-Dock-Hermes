# HERMES RUST NATIVE UI IMPLEMENTATION PROMPT

**Deliver exactly the current upstream Hermes Agent desktop GUI in native Rust (egui + DoofusDock), not an interpretation of it.** Use `NousResearch/hermes-agent` directly, independent of the older Hermes-Doofus project; connect to the upstream Python Hermes gateway. Hermes-Doofus plugin port, its Vault/Control screens and the separate browser Dashboard are **OUT OF SCOPE** for this initial project.

After visual and feature parity are achieved, enable alternate layouts (Codex-inspired, Antigravity-inspired, IDE-inspired, Doofus-inspired, user-defined) that morph the **same** widgets using DoofusDock `Morph` and canonical profile/operation APIs. Returning to the Hermes source preset must recover the original exact GUI.

## Required inputs
- `README.md`
- `research/EXACT_SOURCE_LAYOUT_AND_PARITY.md`
- `maps/HERMES_SOURCE_LAYOUT_TREES.json`
- `maps/LAYOUT_PROFILES.yaml`
- `maps/full_program_capability_map.json` and `maps/capabilities.yaml`
- `planning/IMPLEMENTATION_PLAN.md`, `planning/ARCHITECTURE.md`, `planning/ACCEPTANCE_MATRIX.csv`
- `skills/hermes-upstream-doofusdock/SKILL.md`
- `research/SOURCE_INDEX.csv`

Source repositories and baseline snapshots:
- Upstream Hermes: `NousResearch/hermes-agent@ffdc7eeb25343df1a7ee84503ba552538db9af72` (update if HEAD changed; pin your selected source)
- DoofusDock: `robertheadley/doofusdock@f6eb49b9e488f4e851d8d3abe18e95d296201bba` (refresh before use)
- Universal Capability Map methodology: `robertheadley/skills/Universal Capability Map/SKILL.md`

## PLANNING / MUTATION GATE

First **read and audit** both repositories; produce a versioned implementation plan, gap inventory, exact upstream screen/capability evidence, Rust crate strategy, source/target screenshots plan, test matrix and rollback mechanism. Use supported planning controls, record plan ID/revision, get explicit user acceptance before applying code edits, installing dependencies or altering live projects. Safe read-only inspection is allowed. No time estimates. Do not declare completion from unit tests alone; record physical UI acceptance evidence. Respect existing work in progress and do not modify it.

## Non-negotiable requirements

1. **Hermes source is the UI specification.** Match layout tree/weights, edge rails/resting panes, titlebar, status bar, session sidebar, tabs, chat/composer, Git review, file browser, terminal, route panels/overlays, model menus, fonts, icons, hover/focus and native window behavior. A beautiful *different* layout is a failure for Hermes mode.
2. Implement all source-defined upstream layout presets and `Simple`/`Advanced` preference policy **without conflating mode with preset**. Use `maps/HERMES_SOURCE_LAYOUT_TREES.json` as baseline only; refresh from live source before implementing. The Advanced default is `sessions:workspace:right=1:3.4:1.25`, with right region `review:files=1:1.2` above `terminal`, top:bottom `1.6:1`. Review/files/terminal may rest hidden independently. The original sidebar default is 237px and narrow dock collapse threshold 640px; verify actual live dimensions, don't hardcode false fixed ratios.
3. DoofusDock core owns **one** canonical `LayoutDocument`. egui/DoofusDock host renders native app widgets, not the Electron/React UI or an iframe. Do not invent native widgets in DoofusDock-core; build a Rust Hermes widget host. Use public crates rather than importing demo internals.
4. Hermes gateway Python agent remains the runtime. Implement typed gateway transport (`hermes gateway attach --json`, WebSocket `hermes-gateway-v1`, single-use tickets, `runtime.describe`, `client.capabilities`, session/transcript/prompt/approval/server-request/replay semantics). UI must not double-execute prompts, leak secrets or show stale events as current.
5. Every stock Hermes desktop feature must be real: chat, sessions, files, artifacts, terminal, Git, worktrees, subagents, profiles, models, settings, memory graph, skills/MCP/plugins, voice, Quick Entry/HUD, onboarding, scheduling/cron/webhooks/messaging, status, command palette, notifications, theming, accessibility. Distinguish stock bundled plugin UI from unsupported arbitrary third-party JS contributions. Do not claim automatic React plugin compatibility.
6. Maintain active session/PTY/tool state when a pane hides, rearranges, detaches or morphs. Keep fixed per-panel identity, persistent drafts, scroll/selection, and focus; only explicit close/stop may end the underlying backend resource.
7. Implement `Hermes Advanced` exact preset first, then native parity with source workflows, and **only then** additional Codex/Antigravity/IDE/Doofus-inspired profiles. Build each profile as stable panel slot geometry. Use DoofusDock `Morph`, provider-bound timing, snapshot renderer, reduced-motion and retargeting; canonical source/target commits only, no per-frame layout mutation. Reverse, interrupt and restore exact Hermes behavior.
8. Rebuild all relevant support UI: validation, keyboard, IME, focus, screen reader, search/sort/filter, menus, empty/loading/error/denied/offline/stale states, virtualization, accessible contrast, permissions, responsiveness, clipboard and license credits.
9. Record a source-to-target capability crosswalk and real before/after screenshots for every important UI surface. Run the matrix against same source SHA, fixtures, theme, viewport and display scale. Use `tools/compare_screenshots.py` for an initial objective metric, plus manual review. Mark verified only with relevant visual+functional acceptance evidence.
10. Maintain orthodox engineering constraints: deterministic pure core, idempotent safe retries, atomic mutations, type safety, fail fast, disk source of truth, auditability and zero trust. Preserve previous working Hermes executable as rollback; no forced upgrades or plugin migrations.

## Deliverable stages
1. Source inventory, layout/widget reference capture, accepted plan and Rust architecture.
2. Working native Rust Hermes UI with exact original source default and other original upstream layouts.
3. Full stock GUI behavioral and visual parity with the upstream backend; test/evidence receipts.
4. Alternate layout profiles and seamless panel morph; parity-restoring Hermes mode.
5. Windows hardening/release, user-approved migration only.

## Required first response
Provide a read-only source version report with selected SHAs, exact current upstream layout tree, route/mode behavior, reusable DoofusDock APIs, missing native widgets, proposal for source screenshot capture, a complete plan with risk and verification gates, and an explicit plan-approval request. Do not edit source until approval; do not use time estimates. **Do not port Doofus Hermes yet.**

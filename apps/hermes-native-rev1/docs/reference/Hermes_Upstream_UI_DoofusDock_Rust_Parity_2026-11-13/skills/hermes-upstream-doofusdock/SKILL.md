---
name: hermes-upstream-doofusdock
version: 2.0.0
description: Preserve 1:1 upstream Hermes desktop UI in native Rust/egui via DoofusDock; then morph existing panels between named layouts.
---

# Hermes upstream desktop parity using DoofusDock

## Trigger
Use for an agent working on the Rust native GUI host for current upstream Hermes Agent, with DoofusDock as its layout/presentation library. This is a planning and parity skill draft, **not** an official skill shipped by DoofusDock. Read the real DoofusDock crate API/docs and the repository's current instructions before coding.

## Hard scope
- Stage 1: **exact upstream Hermes desktop** user experience, visuals, routes, preferences and behavior.
- Stage 2: alternate Codex/Antigravity/IDE/Doofus-inspired layouts; same native panels; reversible animated DoofusDock morph.
- Current upstream Hermes Python agent backend remains unchanged; choose its pinned commit independently.
- NO initial Doofus Hermes plugin, bridge, special Vault, Control, Truth Layer or dashboard administration port.

## Research
1. Pin upstream Hermes and Dock SHA. Inspect upstream default layout module and actual current UI; no guessing source appearance from code names.
2. Inspect route classification, mode shadow policy, dynamic tiles, panels, fonts/icons/styles and keyboard shortcuts.
3. Inspect DoofusDock `core`, `presentation`, `render-egui`, `host-egui`, `profiles`, `Morph`, theme, lifecycle and physical platform gaps.
4. Map every visible item and behavior to unique stable requirement/evidence. Use Universal Capability Map status `unknown/not_implemented/partial/implemented/verified` with append-only evidence history. Specs do not count as passing runtime tests.

## Architecture
- `LayoutDocument` is sole authoritative layout; host widgets hold no parallel layout graph.
- Canonical layout/profile operations validate and commit once; morph frames are transient. Realtime session state stays with Hermes gateway.
- Panel identity and widget lifetime are stable across layout changes, including when hidden or detached.
- Agent API transport uses versioned contracts; requests and server prompts are gated. No plugin secret flows via default UI.

## Tests
- Screen: same viewport/theme/fonts/DPI, source vs Rust screenshot; report excluded dynamic regions and metrics.
- Behavior: same user interaction/shortcut, same source result and negative path; capture semantic/agent receipt.
- Morph: Hermes → any profile → Hermes, reverse/retarget/interruption, reduced motion, preserve transcript/draft/PTY, canonical undo, no duplicate requests.
- Platform: Windows real focus, native drag, monitor topology, IME, screen reader, DPI, high contrast, no excessive idle redraw.

## Completion
Complete only when the *original Hermes* UI is functionally and visually equivalent on accepted target platforms. Alternate profile sketches do not excuse any 1:1 Hermes gap. No fabricated verification. Do not start Doofus Hermes port as part of this stage.

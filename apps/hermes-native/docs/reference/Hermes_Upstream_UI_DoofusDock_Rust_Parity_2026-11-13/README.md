# Upstream Hermes GUI 1:1 → Native Rust + DoofusDock

**Revised research + agent implementation package • 2026-11-13 • NOT a built application**

## Exact target

Recreate **current upstream `NousResearch/hermes-agent` DESKTOP GUI** as a fully native Rust/egui application, using `robertheadley/doofusdock` for docking, canonical layout, windows, profiles and Morph animation. **The original Hermes GUI is the default exact visual and behavioral reference**, including original Advanced/Simple mode distinctions and upstream layout presets. After parity is proven, offer alternate Codex-inspired, Antigravity-inspired, IDE-inspired, Doofus-inspired and custom profiles that rearrange the **same live native panels**, animating via DoofusDock morphs. Selecting Hermes restores the exact source arrangement and behavior.

**Backend:** use upstream Hermes Python agent and its current gateway, regardless of whether `Hermes-Doofus` is updated. **Doofus-Hermes plugin port is deferred**; its custom UI, backend/Truth Layer/Vault/Control pane are NOT an implementation prerequisite. The browser-based Hermes Dashboard is separate and also not an initial desktop UI target.

## Research verification

- Inspected upstream source layout trees, route classifier, mode state/policies, renderer/contributions and integration guide via GitHub, pinned to `ffdc7eeb25343df1a7ee84503ba552538db9af72`.
- Inspected DoofusDock's Rust public profile, Morph, native host and renderer sources/docs, pinned to `f6eb49b9e488f4e851d8d3abe18e95d296201bba`.
- Upstream actual UI screenshots and native runtime interactive behavior **have not been collected or compared**. 1:1 pixel/functional parity remains a testable goal, not an achieved result.
- The inherited inventory includes some crosscutting candidate behaviors that need a specific source-component trace before being treated as confirmed upstream functions.

## Contents

- `PROMPT_FOR_HERMES.md` — new complete prompt with source-GUI fidelity first and delayed morph phase.
- `research/EXACT_SOURCE_LAYOUT_AND_PARITY.md` — actual upstream Advanced/default/basic/focus/terminal/quad geometry, mode/route semantics and fidelity criteria.
- `maps/HERMES_SOURCE_LAYOUT_TREES.json` — normalized source layout trees and mode/visibility policy, not DoofusDock serialized schema.
- `maps/LAYOUT_PROFILES.yaml` — original source layout targets plus Codex/Antigravity/IDE/Doofus-inspired arrangements.
- `maps/full_program_capability_map.json`, `maps/capabilities.yaml` — **491** target feature requirements, all pending, mapped to upstream desktop + morph.
- `maps/DEFERRED_OUT_OF_SCOPE.json` — **108** excluded original inventory records with reasons (future plugin and web Dashboard).
- `planning/REQUIREMENTS_LEDGER.csv`, `planning/ACCEPTANCE_MATRIX.csv` — traceable target requirement ID + test ID.
- `maps/UI_ELEMENT_INVENTORY.csv`, `maps/UI_COMPONENT_SPECS.csv`, `maps/CAPABILITY_CROSSWALK.csv`, `maps/STATE_OWNERSHIP.csv` — component and lifecycle design.
- `planning/ARCHITECTURE.md`, `planning/IMPLEMENTATION_PLAN.md` — Rust module boundaries, phase sequence, tests, release gates.
- `research/SOURCE_INDEX.csv` — source file paths and pinned GitHub commits.
- `skills/hermes-upstream-doofusdock/SKILL.md` — project-specific skill draft, not an official published DoofusDock skill.
- `tools/validate_package.py` — dependency-free audit for IDs, map counts and ledger alignment.
- `tools/compare_screenshots.py` — screenshot sizes, pixel RMSE and diff image creation using Pillow, useful for captured source/target images (none supplied here).

## First action in a coding agent

Read the prompt and source-layout research, refresh/pin upstream and Dock commits, conduct read-only source/visual audit, present a plan for explicit approval, and only then make changes. Build Hermes exact UI before touching inspired layouts. Do not introduce Doofus-specific Truth Layer UI in first pass. No time estimates.

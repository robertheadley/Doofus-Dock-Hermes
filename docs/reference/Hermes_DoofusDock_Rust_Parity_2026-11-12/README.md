# Hermes Agent + Hermes-Doofus → Native Rust / DoofusDock

**Research and implementation package · 2026-11-12 · Spec, not completed software**

## Objective
Build a native **Rust + egui + DoofusDock** UI with functional parity with the Hermes desktop experience and the **Hermes-Doofus** Control/Models & Quota/Vault plugin, preserving the existing Hermes Python agent, Doofus truth layer, state, and tool integrations. Cover the separate Hermes dashboard's management surfaces as an explicit parity lane; do not confuse the dashboard's TUI-in-browser with the desktop app.

## Verified architectural conclusion
DoofusDock is the **workspace/layout framework**, not a complete application widget collection. Its public workspace is Rust with `doofusdock-core`, `doofusdock-host-egui`, `doofusdock-presentation`, `doofusdock-render-egui` and other crates. It does not provide a chat engine, terminal emulator, media viewer, Git UI, or a Hermes plugin runtime. Hermes already has a fine-grained gateway with JSON-RPC/WebSocket; **reuse the gateway rather than rewriting the agent**. Hermes-Doofus has an authenticated local bridge and a 5,272-line desktop plugin (`desktop-plugin/plugin.js`), so rebuild those contributed panes in Rust against the existing bridge without weakening authorization.

## Package contents

| Location | Use |
|---|---|
| `PROMPT_FOR_HERMES.md` | Paste into Hermes/Codex; plan-gated, source-sensitive implementation instruction |
| `planning/IMPLEMENTATION_PLAN.md` | Rust architecture milestones, non-negotiable completion gates, dependency sequence |
| `planning/REQUIREMENTS_LEDGER.csv` | **565** numbered requirements with acceptance criteria and test IDs |
| `maps/full_program_capability_map.json` | **565** detailed structured target features with UI primitives, adapters, owners, risk and source trace |
| `maps/capabilities.yaml` | Universal Capability Map-compatible status inventory + baseline inspection evidence |
| `maps/UI_ELEMENT_INVENTORY.csv` | Observable UI element/behavior list suitable for spreadsheet/task import |
| `maps/CAPABILITY_CROSSWALK.csv` | Existing Hermes or Doofus capability → DoofusDock shell → Rust widget/adapter |
| `maps/UI_COMPONENT_SPECS.csv` | Reusable widget/component implementation checklist |
| `maps/NAVIGATION_SCREENS.json` | Screen/route inventory across Hermes Desktop, Dashboard and Doofus |
| `maps/PROTOCOL_SURFACES.json` | Documented RPC/event families and bridge endpoints |
| `maps/LAYOUT_PROFILES.yaml` | Starting workspace profiles; not a replacement for the Dock's canonical layouts |
| `maps/STATE_OWNERSHIP.csv` | Prevents split-brain canonical state and lost hidden surfaces |
| `planning/ACCEPTANCE_MATRIX.csv` | One parity test per feature, initially pending |
| `planning/RISKS_AND_UNRESOLVED.md` | Areas requiring direct repo/live app investigation before claiming one-to-one parity |
| `skills/doofusdock-hermes-parity/SKILL.md` | **New task-specific skill draft**, to be installed/used by the coding agent; not an existing DoofusDock release skill |
| `diagrams/*.mmd` | Architecture and event/state diagrams |
| `tools/validate_package.py` | Offline consistency checks for the capability map, ledgers and files |

## Research confidence and status language
- **Source inspected:** GitHub source files, repo trees, versioned Docs and current desktop plugin were read via a connected GitHub source. Linked snapshots below.
- **Documented behavior:** The upstream documentation describes the UI; not every described behavior was manually replayed or tested.
- **Rust port status:** `not_implemented` for every proposed target item; no Rust Hermes UI exists in this ZIP. DoofusDock library statuses are only *static-inspection-level* `implemented`, not live verification claims.
- **No live QA:** No Windows GUI of Hermes or DoofusDock was executed, and no full repository compilation was performed in this research session. The coding agent must produce real evidence before elevating statuses.
- **No software written to GitHub:** Read-only research. No repo modified.

## Pinned sources
- `hermes_upstream`: [`NousResearch/hermes-agent@415fa335b4d9`](https://github.com/NousResearch/hermes-agent/tree/415fa335b4d98ecf00e29e88625bb478dc63fb67)
- `hermes_doofus`: [`robertheadley/Hermes-Doofus@0278571393b0`](https://github.com/robertheadley/Hermes-Doofus/tree/0278571393b0ae98fd016b67cfd32f66cee986ee)
- `doofusdock`: [`robertheadley/doofusdock@f6eb49b9e488`](https://github.com/robertheadley/doofusdock/tree/f6eb49b9e488f4e851d8d3abe18e95d296201bba)
- `ucm`: [`robertheadley/skills@334f6f56a318`](https://github.com/robertheadley/skills/tree/334f6f56a318051e96c5ed85f2708ba7b550a973)

## Start here
1. Read `RESEARCH_AND_SCOPE.md`, then `planning/IMPLEMENTATION_PLAN.md` and `PROMPT_FOR_HERMES.md`.
2. Give `skills/doofusdock-hermes-parity/SKILL.md` and the Universal Capability Map skill at `robertheadley/skills` to the coding agent.
3. **Refresh all 4 repos before writing**; the pinned SHAs are a research baseline, not permission to ignore new commits.
4. Require a reviewed execution plan and explicit approval before any mutations. Work on isolated branch/worktree; keep current Hermes and Doofus plugin fully functional until cutover parity tests pass.
5. Run `python tools/validate_package.py` on the unpacked folder. For the original UCM checker, run the original `robertheadley/skills/Universal Capability Map/scripts/check_capabilities.py` against `maps/` after installing PyYAML. Do not upgrade a source-inspection state to verified merely because the checker passes.

**The scope is parity, not a cosmetic skin and not a Python agent rewrite.**

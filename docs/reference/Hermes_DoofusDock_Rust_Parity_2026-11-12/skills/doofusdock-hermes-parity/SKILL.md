---
name: doofusdock-hermes-parity
version: 0.1.0
license: MIT
summary: Project-specific agent skill for migrating the Hermes Desktop + Hermes-Doofus UI to native Rust with DoofusDock
---

# DoofusDock Hermes Parity Migration (DRAFT SKILL)

This is a **new, task-specific skill draft included with the research package**, not an official skill already discovered in the DoofusDock repository or in the current installed ChatGPT skill catalog. Review and install it explicitly in your Hermes/Doofus agent configuration as appropriate. It does not certify target implementation.

## Trigger
Use when implementing, auditing, testing or planning native Rust + egui + DoofusDock replacement UI for Hermes, or updating any of its parity/capability maps.

## Read before working
1. `../../README.md`, `../../RESEARCH_AND_SCOPE.md`, `../../RESEARCH_SOURCES.md`.
2. `../../maps/SOURCE_SNAPSHOTS.json`, `../../maps/full_program_capability_map.json`, `../../maps/capabilities.yaml`, and `../../planning/REQUIREMENTS_LEDGER.csv`.
3. `../../planning/ARCHITECTURE.md`, `../../planning/IMPLEMENTATION_PLAN.md`, `../../planning/RISKS_AND_UNRESOLVED.md`.
4. Actual current `doofusdock` `docs/ARCHITECTURE.md`, `docs/PANEL_CHROME.md`, `docs/FRAMEWORK_REQUIREMENTS.csv`, `docs/EXTENSION_API.md`, `docs/URNE_INTEGRATION.md`, public Rust APIs/tests.
5. Actual current upstream `website/docs/developer-guide/programmatic-integration.md`, desktop plugin SDK guide, `apps/desktop/src/`, `tui_gateway/` dispatcher and message contracts.
6. `robertheadley/Hermes-Doofus` `desktop-plugin/plugin.js`, `doofus_hermes/bridge.py`, `docs/doofus-control-2.md`, `docs/ui-journeys.md`, vault docs.
7. Original `robertheadley/skills/Universal Capability Map/SKILL.md` and its checker, not only this draft.

## Non-negotiable rules
- **Plan first, require explicit acceptance for mutations**. Inspect and report before changing files; preserve active agent tasks. No time estimates.
- **Native Rust + egui** for visible application content; **DoofusDock** for canonical layouts, docking, native windows, semantics, workspace persistence, transitions and theme contracts. Do not write a second docking engine.
- Existing Hermes Python runtime and existing Doofus truth/evidence engine remain authoritative. Use typed clients and replayable immutable snapshots.
- Establish source version; discover real APIs from code. Never invent undocumented RPC names, routes or schema fields.
- Handle unsolicited server requests, gateway errors, timeouts, stream gaps, duplicate events and multiple clients. No optimistic claim of final state.
- Separate domain models, pure reducers, ports/adapters, UI render, native OS integration, bridge clients, durable file stores, and test fixtures. No I/O inside pure reducer.
- `LayoutDocument` state is changed only with validated, revision-bound, atomic Dock operations; presentation animation and transient overlays do not mutate persistent geometry.
- A Doofus VERIFIED state requires eligible current evidence; record failed and partial outcomes. Demote with retained history when a regression is observed.
- Permission, vault and destructive Git controls fail closed; tokenize and scope tokens, never cache or log secret values, do not enable production CDP or arbitrary JS eval.
- Keep source→target traceability for EVERY requirement and protect third-party plugin SDK parity from accidental silent loss.
- Keep a recoverable/rollbackable old UI until release acceptance. Maintain OSS provenance/attribution.

## Workflow
A. **Recon**: pin SHAs; inspect source tree, RPC requests, child component architecture, UI SDK, Doofus bridge and Dock consumer APIs; update caps *only with evidence*.
B. **Map**: maintain `capabilities.yaml` as source of truth, 565 initial target behavior IDs, `REQUIREMENTS_LEDGER.csv`, `UI_ELEMENT_INVENTORY.csv`, `UI_COMPONENT_SPECS.csv`, `CAPABILITY_CROSSWALK.csv`; append additional IDs as needed.
C. **Plan**: dependency-order a testable vertical slice; map each requirement to source component/gateway/Doofus endpoint/Dock host + failure and a11y; ask approval.
D. **Implement**: protect pure adapters, snapshot-qualified events, consistent cross-panel selection. No duplicative stores or hidden resource recreation.
E. **Prove**: positive/negative tests, Windows live smoke where required, proof receipt, UCM evidence record per capability with commit/date/environment; update statuses honestly.
F. **Reconcile**: run local `tools/validate_package.py`, original UCM checker, Cargo tests, E2E real backend and regression gates; post changes and unresolved gaps. Regressions preserve old evidence, append new one.

## Native UI subcomponents
Reference `../../maps/UI_COMPONENT_SPECS.csv`; important: virtualized chat, stable terminal PTYs, git diff, tree explorer, composer draft, RPC approval modal, model picker, graph explorer, rich tool cards, Doofus control and vault, keyboard/focus and plugin compatibility. These are **application widgets**, not magically built into DoofusDock.

## Completion criterion
No task or feature may be claimed verified unless observed at the target commit with positive and appropriate negative verification, authentic backend state, and scenario evidence. General release requires all P0/P1 accepted or explicitly excepted by the user, rollback, security/provenance, Windows host/live a11y and profile/cursor recovery. Never replace a real test with a diagram, screenshot mock or stale source assertion.

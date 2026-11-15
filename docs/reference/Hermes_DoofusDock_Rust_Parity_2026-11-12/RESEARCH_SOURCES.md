# Research sources / evidence provenance

All repository links are **snapshot-pinned**; public documentation links below were consulted on 2026-11-12. Source names are not proof of an implemented interaction: exact behavior must be retested at execution time.

## Upstream Hermes Agent
- [Hermes desktop user guide](https://hermes-agent.nousresearch.com/docs/user-guide/desktop) — desktop tabs, windows, file browser, artifacts, terminal, subagents, Git review, profiles, model selection, voice, settings and native integrations.
- [Hermes web dashboard](https://hermes-agent.nousresearch.com/docs/user-guide/features/web-dashboard) — separate administration pages, security and PTY/TUI embedding.
- [Programmatic integration](https://hermes-agent.nousresearch.com/docs/developer-guide/programmatic-integration) — JSON-RPC over stdio/WebSocket, `runtime.describe`, one-use WS ticket, documented method/event/server-request families.
- [Desktop plugin SDK](https://hermes-agent.nousresearch.com/docs/developer-guide/desktop-plugin-sdk) — React desktop contribution registry, host state/actions, scoped plugins, features a Rust implementation would have to emulate for plugin parity.
- [Gateway internals](https://hermes-agent.nousresearch.com/docs/developer-guide/gateway-internals) — local control-socket discovery and auth assumptions.
- [Pinned desktop guide](https://github.com/NousResearch/hermes-agent/blob/415fa335b4d98ecf00e29e88625bb478dc63fb67/website/docs/user-guide/desktop.md)
- [Pinned programmatic integration](https://github.com/NousResearch/hermes-agent/blob/415fa335b4d98ecf00e29e88625bb478dc63fb67/website/docs/developer-guide/programmatic-integration.md)
- [Pinned plugin SDK](https://github.com/NousResearch/hermes-agent/blob/415fa335b4d98ecf00e29e88625bb478dc63fb67/website/docs/developer-guide/desktop-plugin-sdk.md)
- [Desktop `apps/desktop/src/app`](https://github.com/NousResearch/hermes-agent/tree/415fa335b4d98ecf00e29e88625bb478dc63fb67/apps/desktop/src/app), [desktop components](https://github.com/NousResearch/hermes-agent/tree/415fa335b4d98ecf00e29e88625bb478dc63fb67/apps/desktop/src/components), [gateway `tui_gateway/`](https://github.com/NousResearch/hermes-agent/tree/415fa335b4d98ecf00e29e88625bb478dc63fb67/tui_gateway)

## DoofusDock
- [README](https://github.com/robertheadley/doofusdock/blob/f6eb49b9e488f4e851d8d3abe18e95d296201bba/README.md) — crate inventory, host boundary, native evidence, supported operations.
- [Architecture](https://github.com/robertheadley/doofusdock/blob/f6eb49b9e488f4e851d8d3abe18e95d296201bba/docs/ARCHITECTURE.md) — canonical `LayoutDocument`, operations, native events, transient state.
- [UI Composition Catalog](https://github.com/robertheadley/doofusdock/blob/f6eb49b9e488f4e851d8d3abe18e95d296201bba/docs/UI_COMPOSITION_CATALOG.md) — important boundary: **content is application-owned**.
- [Panel Chrome](https://github.com/robertheadley/doofusdock/blob/f6eb49b9e488f4e851d8d3abe18e95d296201bba/docs/PANEL_CHROME.md) — `panel_title_bar`, `tab_strip`, `PanelShell`, host actions.
- [Extension API](https://github.com/robertheadley/doofusdock/blob/f6eb49b9e488f4e851d8d3abe18e95d296201bba/docs/EXTENSION_API.md) — linked Rust providers, registration, schemas and lifecycle.
- [Framework requirements](https://github.com/robertheadley/doofusdock/blob/f6eb49b9e488f4e851d8d3abe18e95d296201bba/docs/FRAMEWORK_REQUIREMENTS.csv) — 52-item acceptance/evidence ledger, with in-progress entries.
- [Backlog](https://github.com/robertheadley/doofusdock/blob/f6eb49b9e488f4e851d8d3abe18e95d296201bba/docs/BACKLOG.md) — unresolved native/cross-host acceptance.
- [Renderer capability source](https://github.com/robertheadley/doofusdock/blob/f6eb49b9e488f4e851d8d3abe18e95d296201bba/crates/doofusdock-render-egui/src/lib.rs) — shader chrome is explicitly `NotImplemented`.

## Hermes-Doofus
- [README](https://github.com/robertheadley/Hermes-Doofus/blob/0278571393b0ae98fd016b67cfd32f66cee986ee/README.md)
- [Current desktop plugin](https://github.com/robertheadley/Hermes-Doofus/blob/0278571393b0ae98fd016b67cfd32f66cee986ee/desktop-plugin/plugin.js) — actual registered routes, UI functions and fetch paths; 5,272 lines at baseline.
- [Control 2.0 documented semantics](https://github.com/robertheadley/Hermes-Doofus/blob/0278571393b0ae98fd016b67cfd32f66cee986ee/docs/doofus-control-2.md)
- [Doofus UI journeys](https://github.com/robertheadley/Hermes-Doofus/blob/0278571393b0ae98fd016b67cfd32f66cee986ee/docs/ui-journeys.md) — dev-only renderer CDP, packaged-app plugin boundary.
- [Doofus bridge](https://github.com/robertheadley/Hermes-Doofus/blob/0278571393b0ae98fd016b67cfd32f66cee986ee/doofus_hermes/bridge.py) — auth and routes.
- [Doofus functionality reference](https://github.com/robertheadley/Hermes-Doofus/blob/0278571393b0ae98fd016b67cfd32f66cee986ee/docs/functionality-reference.md) — truth layer, evidence, audit and hooks.
- [Vault design](https://github.com/robertheadley/Hermes-Doofus/blob/0278571393b0ae98fd016b67cfd32f66cee986ee/docs/secrets.md) — redaction and leased secret boundaries.
- [Desktop pane baseline and subsequent changes](https://github.com/robertheadley/Hermes-Doofus/blob/0278571393b0ae98fd016b67cfd32f66cee986ee/docs/investigations/urne-execution-visibility/desktop-pane-inventory.md) — dated baseline includes later implemented routes; do not treat initial gap list as current.

## Universal Capability Map
- [Skill specification](https://github.com/robertheadley/skills/blob/334f6f56a318051e96c5ed85f2708ba7b550a973/Universal%20Capability%20Map/SKILL.md)
- [Checker](https://github.com/robertheadley/skills/blob/334f6f56a318051e96c5ed85f2708ba7b550a973/Universal%20Capability%20Map/scripts/check_capabilities.py)

## Source-data limitations
GitHub reads were limited to current tree indices and selected files; no full automated static-analysis scan or live GUI session was performed. The generated 565-row map is a **research-based target inventory**. It is not a claim that all existing Desktop interaction variants were exhaustively detected. Its `not_implemented` target statuses must be reconciled by the implementer after the real Rust repo is created.

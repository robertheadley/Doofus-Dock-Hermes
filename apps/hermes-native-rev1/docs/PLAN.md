# Approved implementation plan

- **Plan ID:** `HUR-001`
- **Approved revision:** `1`
- **Source pin refresh:** `1a` (implementation-start re-resolution required by the approved plan)
- **Status:** in progress
- **Scope:** stock upstream Hermes Desktop UI in native Rust/egui with DoofusDock; Hermes Python gateway remains authoritative.
- **Excluded:** Hermes-Doofus plugin UI/backend, Doofus Vault/Control, and the separate browser Dashboard.

## Stages and gates

1. Pin current upstream sources, refresh the capability map, and capture the source Desktop UI in an isolated profile. Gate: pinned source evidence and baseline captures.
2. Implement the original Hermes default and source-defined layout profiles, Advanced/Simple preference policy, source routes, native shell and DoofusDock persistence. Gate: exact layout trees, mode behavior, reveal/hide, resize/dock, save/restore and source/target visual review.
3. Implement typed gateway attach, JSON-RPC/WebSocket, capability negotiation, event replay, sessions, chat, approvals and the stock desktop widgets. Gate: protocol and workflow acceptance matrix with positive and negative cases.
4. Complete Windows visual and functional parity for all in-scope capabilities. Gate: paired captures and acceptance receipts for every mapped behavior.
5. Only after source parity, add alternate layout profiles using the same widgets and DoofusDock Morph. Gate: transition interruption, reverse, reduced motion, state preservation and exact restoration of Hermes layout.
6. Harden and prepare reversible release. Gate: DPI/native-window/accessibility/recovery checks, license review, rollback verification, and separate approval before any live migration.

## Verification approach

Capture upstream and Rust windows using matched dimensions, display scale, locale, theme, fonts and deterministic gateway fixtures. Cover 100%, 125%, and 150% scale where available; first launch, every source preset, Simple/Advanced, workspace pages, overlays, hidden/revealed panes, modal/popover, focus/hover, empty/loading/offline/error/denied states. Use the supplied screenshot comparison utility if its declared dependency is available, plus rectangle checks and manual review. Run the requirement ledger as positive and failure-path cases; compilation alone is not parity evidence.

## Rollback

The current Hermes installation and profile remain untouched. Develop in this independent crate and preserve them as the rollback path. A migration or replacement of the installed executable requires a separate explicit user decision.

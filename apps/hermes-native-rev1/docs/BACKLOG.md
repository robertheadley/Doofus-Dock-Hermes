# Hermes Native parity backlog

The supplied, refreshed reference target contains 491 stock Desktop capabilities; 108 inherited items are explicitly out of scope. All remain pending until tied to current source and accepted with evidence.

## Stage 1 — exact native shell

- [x] Implement source Advanced Default geometry, Basic, Focus, Terminal Deck, Quad and Sidebar left/right trees through `WorkspaceProfile`.
- [x] Implement independent Simple/Advanced selection and shadowed optional-pane visibility policy.
- [ ] Match the source titlebar, sidebar controls, tabs, welcome/setup content, status bar, dialogs, fonts/icons, hover/focus and native window behavior at matched viewport and DPI.
- [ ] Add source routes and overlay classification.
- [x] Persist canonical layout via `LayoutStore`; persist bounded UI preferences separately with a validated temp-file replacement.
- [ ] Verify live docking, resize, tab, save/restore, narrow width and keyboard/accessibility behavior.

The initial shell passed compile and two layout-policy tests and was opened on Windows. A manual Simple → Advanced round trip and Files reveal/hide check passed. These checks cover only the scaffold slice; they do not count as visual parity receipts. The upstream source preview later displayed its backend-start timeout recovery dialog, so its initial setup screen needs to be recaptured in a fresh isolated profile for paired evidence.

## Stage 2 — gateway and core UI

- [ ] Attach using official upstream gateway discovery/ticket/WebSocket contracts.
- [ ] Typed JSON-RPC, `runtime.describe`, `client.capabilities`, server request matrix, replay/reconnect and profile isolation.
- [ ] Session sidebar, history, chat/transcript, streaming, composer, tools/reasoning cards, model picker and attachments.

## Stage 3 — remaining stock widgets

- [ ] Files, dynamic previews, artifacts, terminal/PTY lifetime, Git review/worktrees and subagents.
- [ ] Profiles, models, settings, memory graph, skills/MCP/plugins, voice, Quick Entry/HUD, onboarding, scheduling, messaging, notifications, theming and accessibility.

## Stage 4 — source parity evidence

- [ ] Execute current `ACCEPTANCE_MATRIX.csv` with matched live source/target fixtures, screen captures, protocol receipts and negative paths.
- [ ] Close all in-scope capability gaps and record limitations; no screenshot-only or test-only completion claims.

## Stage 5 — alternate Morph profiles

- [ ] Add alternate layouts only after Hermes source parity. Preserve panel instances/state/focus; sample Morph transiently and commit canonical geometry once.
- [ ] Verify interruption, retarget/reverse, reduced motion, restore-to-Hermes and rollback.

## Known risks

- Upstream changes rapidly; pin and re-audit current source before each parity batch.
- The supplied source map is research evidence, not live visual acceptance; no real-source screenshot has yet been saved to the project.
- Native viewport behavior and multi-DPI acceptance need physical Windows evidence.

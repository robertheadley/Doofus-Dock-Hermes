# Supplemental agent-control architecture crosswalk

Source: user-supplied “Doofus Apps: Agent Control and Development Architecture” attachment, received 2026-11-13. This applies its Hermes-specific requirements to this replacement; generic reusable crates and work in other Doofus applications are not included.

| Layer or requirement | Current evidence | Status | Remaining proof/work |
|---|---|---|---|
| Application core | `src/gateway.rs`, `src/app.rs`; gateway remains upstream Python | Partial | Typed app services and source-complete workflows |
| UI/window host | eframe/egui with AccessKit; titlebar drag center-tabs/edge-splits, same-group tab reorder, and split dividers commit through revision-checked DoofusDock transactions | Partial | Native pointer acceptance; float/cross-window/window lifecycle, stable identities, focus/DPI acceptance |
| Canonical DoofusDock layout | `Workspace` owns `DockingService`, `LayoutStore`, profiles; renderer uses `project` | Partial | All changes through revision-checked transactions; undo/redo, preflight, native host receipts |
| Semantic UI runner | Opt-in `agent-automation` feature builds `hermes-automation`; it inspects AccessKit labels/bounds, clicks and scrolls over controls, performs bounded pointer drags through egui input including split resize and tab reorder, captures 1440×900 PNGs, and cleans per-run temp profiles | Headless prototype implemented | Broader route coverage, dynamic test IDs and physical host evidence |
| Child-process UI protocol | `hermes-automation --stdio`; versioned NDJSON over parent-owned inherited pipes; bounded state/tree/events/diagnostics, semantic click/scroll/type/select/keypress, drag-to-locator or bounded-coordinate, wait, screenshot and close; fixed controls have stable IDs | Implemented and protocol-exercised | Capability/action registry shared with app services; modifiers, dynamic IDs and further workflow contracts |
| Shared semantic control service | Typed versioned registry covers route navigation, pane toggles, interface mode, layout profile selection, Capabilities tab selection/refresh, Toolsets enable/disable, stop generation, offered approval decisions and visible clarify answers; human controls and `app.action.execute` share app-core handlers. Skills/Toolsets reads and Toolsets writes use profile-scoped one-use HTTP tickets. | Partial | Extend to all app/DoofusDock/Hermes operations; add dynamic session/toolset semantic IDs; centralize per-action permission/confirmation metadata |
| Development Debug Bridge | No authenticated loopback listener/protocol | Missing | If the supplemental architecture still needs cross-process network clients: scoped transport, audit/redaction/revocation and production exclusion |
| Development supervisor | No watcher/rebuild/restart/reconnect service | Missing | Debounced lifecycle, build IDs, readiness, last-good recovery and state restore |
| Declarative reload | Not implemented | Missing | Validate, apply, rollback and preserve supported UI state |
| Structured runtime diagnostics | Protocol exposes generation, bounded correlated events and a read-only state/health diagnostic bundle; development journals exist | Partial | App-wide logs/errors, richer diagnostics and persistent redacted audit |
| Hermes operation safety | Ticket attach/list/resume, existing-session prompt.submit, and partial fresh-session session.create → initial prompt.submit implemented; runtime and stored session IDs retain distinct roles; basic message start/delta/complete, approval, and single-question clarify events project into the active runtime session | Partial | Full create options/retry, upstream admission identity/idempotent retry, batch clarify/server requests, tool/reasoning events, replay/reconnect and complete transcript reconciliation; stop and prompt decisions are fixture-tested but lack live-profile acceptance |
| Three-layer verification | Default and automation-feature Cargo checks; headless protocol exercised for center-tab, edge-split, accessible tab selection and screenshot | Partial | Broader core contracts, source-paired semantic integration and separate physical Windows evidence |
| Production security boundary | Bridge is not present yet | Not applicable | Ensure privileged dev control is opt-in and absent/disabled in releases |

## Protocol reconciliation

Attachment endpoint names are proposals. The implemented child process uses inherited stdio instead of a network listener, so parent ownership is the transport capability; it does not expose routes to other processes. A future loopback Debug Bridge is only needed if a concrete cross-process client requires it. App operations remain distinct from Hermes gateway operations, which control actual sessions. Neither surface may bypass gateway approvals or become an unrestricted shell/filesystem proxy.

## Scope boundary

The active user objective authorizes the Hermes Native replacement and non-disruptive agent verification. This supplement applies to Hermes Native and its development tooling only. It does not authorize migration, changes to installed Hermes, or edits to the older prototype or unrelated apps.

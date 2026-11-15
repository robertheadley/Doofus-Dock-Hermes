# Risks, blockers, and open source questions

The repository review establishes a strong design basis but **not live 1:1 parity certification**. These risks must remain visible as unresolved until real integration evidence is recorded.

| Risk | Impact | Required resolution |
|---|---|---|
| **JS React Desktop Plugin SDK is not native Rust ABI** | Existing community plugins and Doofus plugin contributions may not load | Inventory all contribution areas in SDK, decide explicit sandbox WebView/bridge adapter or native plugin migration; never claim equivalent loading without tests |
| **Public JSON-RPC method catalog is selected, not exhaustive** | Missing server-to-client RPC or ephemeral event can break an agent workflow | Read current TUI gateway dispatcher/code and event schema, generate exhaustive conformance fixtures and unknown-method handling |
| **Hermes desktop changes rapidly** | Pinned feature list may have drifted | Refresh SHA/diff before coding; append requirement IDs for additions rather than deleting old ones |
| **Hermes Dashboard != Electron Desktop** | False completeness if management/admin overlooked | Track dashboard pages and native adaptation separately; decide which remote admin/API features must be exposed in Rust without rewriting server |
| **Doofus bridge schema and authentication lifecycle** | Vault leakage/unauthorized task control | Use supported bootstrap and bearer token scoped by profile, confirm required security checks before mutations |
| **CDP developer debugger is not production capability** | Remote code execution surface if reproduced | Do not expose CDP/JS `eval` in a packaged native app; use authorized typed API actions |
| **Window and display acceptance not complete in Dock** | Multi-monitor, mixed DPI, focus and screen-reader regressions | Real Windows dual-monitor/scale/change/unplug; record observations; keep framework evidence distinct from UI widget proof |
| **Dock shader application effects not implemented** | Cosmetic parity promises can be false | Stick to vector/nine-slice fallback; shader implementation is optional explicit follow-up |
| **Terminal ownership can be destroyed accidentally** | Lost sessions and commands on hide, move or detach | Separate PTY id from Dock panel id; lifecycle tests on hide/reparent/reopen/process crash |
| **Transcript replay/reconnect** | Duplicate stream content or lost approvals | Store event ID/cursor, reconcile transcript snapshot after reconnect, do not automatically replay prompts |
| **Vault, model keys and permissions** | Secret exposure or unauthorized action | No plaintext in UI persistence, telemetry, diagnostics or screenshots; fail closed, audit requests |
| **HTML previews and third-party UI** | Injection into trusted native client | Sandboxed renderer, denied filesystem/shell privileges, explicit safe scheme/open external behavior |
| **Performance claims** | UI lags despite Dock microbenchmarks | Measure full native app CPU/GPU/latency/memory at growing chats/panes; keep Dock microbenchmarks as separate evidence |
| **Licensing and provenance** | Missing OSS attribution | Attribute upstream, Dock, engine libraries/assets and bundled code; run license checks |

## Open questions the implementation agent must resolve from code
- What is the exact current gateway RPC and server-request method catalog? Which methods are not publicly documented?
- Which Desktop Plugin SDK panes, routes, composer extensions, tool renderers, notifications and theme contributions are installed and expected to load?
- Which Dashboard admin-only endpoints work with remote gateway and which require local `webui.py` instance?
- What native Rust PTY strategy supports the gateway's remote target semantics?
- Which Dock host data structures safely allow arbitrary app-owned rich egui child content in detached windows?
- What data belongs to Hermes gateway profiles versus Doofus workspaces versus OS settings? Verify actual flows.
- What input, focus, native titlebar, high contrast and multi-monitor behavior is *physically* evidenced on Windows?

The first accepted milestone is a read-only source and protocol reconciliation. The next is an independently runnable Rust shell+gateway client. None is currently delivered by this ZIP.

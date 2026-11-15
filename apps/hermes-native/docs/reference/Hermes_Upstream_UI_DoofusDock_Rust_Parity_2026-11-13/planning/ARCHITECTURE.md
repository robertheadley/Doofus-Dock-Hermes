# Architecture and integration boundaries

## Native workspace target

Rust-only GUI host over **DoofusDock + egui** with the upstream Hermes gateway as runtime. The app is a UI host; it is not a fork of the Hermes Python reasoning/tool engine. Do not fork DoofusDock into a separate layout engine or rebuild its canonical graph inside widget state.

```text
┌──────────────────────────────────────── Native Rust UI ────────────────────────────────────────┐
│ Hermès Original Fidelity Profile     [Advanced/Simple] [Layout: Hermes/Codex/Antigravity/...]  │
│ Toolbar/menu, titlebar and statusbar | UI action router | stable panel content registry       │
│ egui widgets: sessions/chat/composer/files/preview/terminal/review/settings/palette/etc.       │
│ DoofusDock core LayoutDocument + operations + profiles + persistence + theme                  │
│ DoofusDock presentation Morph snapshots; render-egui; native egui host                         │
└───────────────┬────────────────────────────────────────────────────────────┬───────────────────┘
                │ GatewayClient JSON-RPC/WebSocket                          │ native OS adapter
                ▼                                                           ▼
    Upstream Hermes gateway (Python)                                Windows input/IME/PTy/tray
    session/message/tool/approval/scheduler/skills/MCP/plugins      accessibility/monitor/DPI
```

## Rust crate recommendation

- `hermes-dock-app`: desktop boot, window lifecycle, layout picker, injected providers.
- `hermes-gateway-client`: typed protocol, client capabilities, session lifecycle, event ordering, request replies, reconnect and replay.
- `hermes-ui-core`: pure application reducers/action router and per-pane view-models, independent of egui.
- `hermes-ui-egui`: native widgets, accessibility/IME/theming/font implementation and transient overlays.
- `hermes-dock-profiles`: exact upstream source-mode arrangement descriptors plus alternate geometry profiles; reconciles with public DoofusDock operation API.
- `hermes-morph`: source/target visual snapshots, retarget, focus/clip/panel preservation, easing, reduced motion.
- `hermes-native`: terminal host contract, notifications, OS file dialogs, clipboard, global hotkeys and platform accessibility.
- `hermes-parity-harness`: fixture orchestration, capture indexing, screenshot diff and keyboard/mouse interaction traces.

## Gateway protocol must match selected upstream version

Source: `website/docs/developer-guide/programmatic-integration.md` and actual `tui_gateway` schemas. Discovery `hermes gateway attach --json`, single-use WebSocket ticket, `hermes-gateway-v1`, `runtime.describe`, `client.capabilities({server_requests:true})`, session create/activate/list/transcript, prompt/rewind confirmations, tool streaming, `session.events.since`, open requests on resume, request/cancel, model/config commands. Server-initiated approval, clarify, sudo, secret, vault unlock, connection, terminal.read, window.read, preview.act and tour must be answered or explicitly rejected as unsupported. A reconnect must not duplicate a non-idempotent submit. The client must never steal a session from another process or silently migrate profile owner.

## Contributed desktop plugin limitation

Upstream Desktop supports bundled and third-party JS/React contribution surfaces. Pure native Rust/egui cannot directly render arbitrary JS/React components. Distinguish (1) **all stock/bundled UI features** needed for 1:1 acceptance, (2) backend plugin RPC/events that can remain via Hermes, (3) third-party UI contributed React components requiring a deliberate adapter/rewrite or sandboxed compatibility surface. The present goal targets 1:1 **stock upstream UI**. If arbitrary installed third-party plugin GUI is required later, that is a separate explicit compatibility acceptance gate. Never pretend that a native Rust UI is capable of rendering unported JS plugins.

## Doofus-Hermes boundary (future only)

Do not read from or depend on `robertheadley/Hermes-Doofus`, `desktop-plugin/plugin.js`, the Doofus bridge, its Vault, its model-quota widget or Doofus-specific Truth Layer. The Doofus-themed **layout profile** is simply panel geometry. A later plugin port may contribute UI panels via versioned Rust extension APIs without rewriting foundational stock UI.

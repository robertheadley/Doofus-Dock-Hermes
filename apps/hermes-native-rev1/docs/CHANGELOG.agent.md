# Agent change journal

## HUR-001-001 — 2026-11-13

- **what:** Started an independent upstream Hermes native Rust app crate, copied the approved plan/pin and exact source layout/routes into project audit notes, and configured DoofusDock public crates.
- **why:** Begin the approved source-parity implementation without changing the earlier Doofus/Hermes prototype or source checkouts.
- **components:** `Cargo.toml`, `src/`, `README.md`, `docs/PLAN.md`, `docs/SOURCE_AUDIT.md`, `docs/BACKLOG.md`.
- **type:** feat
- **validation:** Source pin confirmed at `8358fc7bb4ce18fbee8d4bf6659dd155c2a03783`; source Desktop first-run shell launched from an isolated checkout/profile. Rust compile verification is pending.
- **performance:** Not measured.
- **risks:** First slice is not feature parity. Upstream UI compilation required an isolated type-only declaration shim; screenshot evidence is still pending.
- **follow_up:** Implement and compile source profile trees and the Advanced/Simple shell, then save source/target evidence and expand the acceptance matrix.

- **id:** HUR-001-002
  **timestamp:** 2026-11-13T21:15:58Z
  **what:** Added the first native Hermes shell slice with pinned DoofusDock integration, source-defined profiles, separate Simple/Advanced mode handling, persistent layout/preferences, Sessions and New session panes, and explicit offline/empty states.
  **why:** Establish a testable native Rust host before gateway and stock widget parity work.
  **components:** `src/main.rs`, `src/app.rs`, `src/layout.rs`, `Cargo.toml`, `Cargo.lock`.
  **type:** feature
  **validation:** `cargo fmt`, `cargo check`, `cargo test` (2 passed), Windows UI inspection, Simple/Advanced round trip, Files reveal/hide round trip.
  **performance:** No performance change measured; no benchmark claimed.
  **risks:** This is a partial shell and is not a 1:1 replacement. Gateway, source routes/overlays, real session/chat workflows, and remaining stock widgets are absent. Upstream visual capture later hit its isolated backend timeout.
  **follow_up:** Implement routes and typed gateway before claiming source feature parity; capture matched paired screenshots and complete the source acceptance matrix.
- **id:** HUR-001-003
  **timestamp:** 2026-11-13T21:20:19Z
  **what:** Published the independent crate at `apps/hermes-native`, revalidated the published copy, and removed Cargo build outputs using `cargo clean`.
  **why:** Make the Rust UI implementation easy to locate and avoid shipping generated build artifacts.
  **components:** `apps/hermes-native/`.
  **type:** packaging
  **validation:** `cargo fmt --check`, `cargo check`, and `cargo test` passed at the candidate package path (2 tests); final-path `cargo check` and `cargo test` passed (2 tests).
  **performance:** No benchmark performed or claimed.
  **risks:** Full upstream parity is not complete; a separate Cargo cache remains in the workspace staging folder because local policy rejected its direct cleanup. An isolated preview declaration shim also remains untracked.
  **follow_up:** Continue gateway, route, stock-widget and parity-matrix implementation; re-capture upstream baseline after resolving source backend timeout.
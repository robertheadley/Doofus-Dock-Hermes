# Hermes Native Rust

Independent native Rust/egui implementation of the stock upstream Hermes Desktop GUI. It uses the Hermes Python gateway as the runtime and DoofusDock for canonical workspace layouts, profiles, docking and presentation.

## Source pins

- Hermes Agent: `8358fc7bb4ce18fbee8d4bf6659dd155c2a03783`
- DoofusDock: `f23ed02a99c7f8c57f83164d98feadb17cc4ed87`

The source pin and source tree evidence are recorded in `docs/SOURCE_AUDIT.md`. The prior Doofus/Hermes prototype remains in the parent workspace and is not part of this crate.

## Current implementation

The first native shell slice is implemented: DoofusDock-backed source layout profiles, independent Simple/Advanced mode selection, resting optional panes, profile persistence, a Sessions sidebar, a New session workspace, panel reveal controls, and truthful offline/empty states. Gateway transport, upstream route/overlay coverage, application workflows, docking gestures, complete source widget parity, acceptance receipts, and alternate Morph profiles are not yet complete. This is not a 1:1 replacement yet.

## Run on Windows

```powershell
cargo run
```

The DoofusDock layout document is saved under `%APPDATA%\HermesNative\layouts`. Set `HERMES_NATIVE_PROFILE_DIR` to an isolated directory for acceptance runs.

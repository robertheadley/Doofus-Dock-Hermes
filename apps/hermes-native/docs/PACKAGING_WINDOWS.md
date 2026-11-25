# Portable Windows package

Build a portable Windows x64 bundle from the current Hermes Native source:

```powershell
pwsh -NoProfile -File apps/hermes-native/tools/package-windows-portable.ps1
```

The script performs a locked release build of `hermes-native.exe` and the `agent-automation` feature binary, verifies the bundled PDFium DLL against the pinned asset, includes the PDFium/Fira Code/JetBrains Mono notices, hashes package files, and publishes the completed folder by an atomic same-volume rename. It refuses to replace an existing output folder. The default result is `artifacts/releases/hermes-native-windows-x64-<source-commit>/`.

The bundle is a portable application folder, not an installer. Keep `hermes-native.exe`, `hermes-automation.exe`, and `pdfium-8125.dll` together. It contains no Hermes profile, credential, Gateway database, session history, or app preferences. Hermes' Python runtime is not bundled: connecting to a local Gateway requires the Hermes CLI installed separately and discoverable as `hermes` on `PATH`. The original Hermes Desktop window is not required. The HTML preview depends on the Microsoft Edge WebView2 Runtime on the host.

For headless agent-operated UI interaction, the bundle includes `hermes-automation.exe` and `agent/hermes-page.mjs`. The native helper can be launched off-screen without the Gateway:

```js
import { HermesPage } from './agent/hermes-page.mjs';
const page = await HermesPage.launchNative({
  command: './hermes-automation.exe',
  cwd: process.cwd(),
});
try {
  console.log(await page.state());
} finally {
  await page.close();
}
```

Node.js is validated for the client; Bun compatibility has not been release-validated. The offline UI helper is not evidence of live Gateway behavior. Full release acceptance still requires cold-launching the package and completing a provider-backed coding task.


## Verification record

On 2026-11-25, the script built source commit `1765d78f01a97c7a48be90bd8414acc1e36d4bca` into `artifacts/releases/hermes-native-windows-x64-1765d78f01a9`. PDFium SHA-256 matched the pinned source asset. The package manifest's paths, lengths, hashes, source pin, and credential-exclusion field were checked. The packaged headless native helper exposed the New session route and Settings control through `HermesPage`, then closed cleanly. The main executable rejected a malformed child-window identity before opening a window. No screenshot was taken.

This is package construction and offline UI smoke evidence only. It does not prove a cold visible launch, Gateway connection from the package, provider-backed tool execution, or completion of the coding workflow. The package carries a notice stating that the Hermes Native Cargo manifest declares MIT while this source checkout lacks a standalone app license text; resolve that repository-level license file gap before external redistribution.


On 2026-11-25, after publishing `e6b2653c351f01fee133dcaf7b6ca5a2bbd019ae`, rebuilt `artifacts/releases/hermes-native-windows-x64-e6b2653c351f`. The new `PACKAGE.json` records that exact Hermes source commit and the pinned Hermes/DoofusDock SHAs; all four listed files passed byte-length and SHA-256 verification, and the package excludes user profiles/credentials. The packaged headless UI reports protocol `hermes-native-window/1`, route `/`, and visible Settings and Terminal controls; it closed cleanly. This remains offline smoke evidence, not a visible cold launch, live Gateway connection or provider-backed coding acceptance.

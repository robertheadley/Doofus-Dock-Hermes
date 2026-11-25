# Hermes Native live coding acceptance

`live-coding-assignment.mjs` uses the semantic Hermes Native control API and the isolated local Hermes Gateway to run one deterministic file-edit assignment. It requires the prepared test profile and fixture under `%TEMP%\HermesNative-Delivery-Test-*`; it refuses paths outside that temp-only profile/project pair. It writes uniquely named screenshots and a JSON result artifact, records file hashes, and runs `python -m unittest -q` only after the agent turn settles and `calc.py` changed.

Build the current automation executable first:

```powershell
cargo build --locked -p hermes-native --bins --features agent-automation
```

Set the four environment values to the already prepared isolated fixture, then launch from the repository root:

```powershell
$env:HERMES_TEST_PROFILE = "$env:TEMP\HermesNative-Delivery-Test-<date>\gateway-profile"
$env:HERMES_TEST_PROJECT = "$env:TEMP\HermesNative-Delivery-Test-<date>\project"
$env:HERMES_TEST_EXE = (Resolve-Path .\target\debug\hermes-automation.exe).Path
$env:HERMES_TEST_SCRIPTS = "$env:TEMP\hermes-native-pinned-test-venv\Scripts"
node apps/hermes-native/agent/acceptance/live-coding-assignment.mjs
```

The runner reports provider/model failures, approval or clarification waits, unknown outcomes, file changes, and test results separately. A provider error is a failed workflow; session creation or prompt submission alone never counts as coding acceptance. It does not auto-approve server requests. A successful release gate requires an actual tool result, changed file, and passing fixture test; the live provider must also be able to execute the request.

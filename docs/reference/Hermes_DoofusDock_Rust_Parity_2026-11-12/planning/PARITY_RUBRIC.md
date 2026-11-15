# Parity decision rubric

**Functionality parity** is the target: same user-triggerable action, gateway scope, safe failure semantics, persisted outcome, tool/progress visibility and recovery, not pixel-perfect Electron copying.

Statuses: `unknown` (uninspected), `not_implemented` (inspected and absent on target), `partial` (only some paths), `implemented` (implementation evidence, insufficient verification), `verified` (current positive behavioral test evidence). Do not claim `verified` from a file path alone.

Requirements must enumerate **success, denied, offline, malformed, reconnect and restart paths** as applicable. Every confirmed supported feature must have keyboard/a11y support and observable feedback. A source-only feature may have a native adaptation if clearly documented and independently approved. Browser-specific community plugin compatibility must not silently disappear.

Promotion to `verified` requires: tested target code at target commit, tested source/version and scenario, applicable negative path, source and target evidence, real platform where interaction matters, and UCM evidence. Regression reopens an item; preserve historical records. `P0/P1` misses block general cutover unless explicitly accepted by user with severity and mitigation. Never quantify completion using files written, lines of code or a green compilation alone.

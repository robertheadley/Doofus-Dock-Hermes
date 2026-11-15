# Research scope, limitations and out-of-scope backlog

Sources were inspected through connected GitHub: current upstream Hermes desktop layout, routes, mode settings, chat, desktop documentation and JSON-RPC integration guide; current DoofusDock Rust profile/morph contracts and framework gaps. Source URLs and pinned SHAs in `research/SOURCE_INDEX.csv`.

**Not done:** running upstream Hermes and DoofusDock side-by-side, collecting pixel screenshots, building any Rust replacement, proving pixel equivalence, implementing native JS/React compatibility, or testing third-party products' visual layouts. No claims of 'complete', 'pixel-perfect' or 'functional' are supported yet.

The incoming 565-capability research inventory was built around both stock Hermes and the custom Hermes-Doofus desktop plugin, and included the separate web Dashboard. This revision filters those out of the initial product and adds source fidelity/morph capabilities. See `maps/DEFERRED_OUT_OF_SCOPE.json`. Some carried-over generic features are **candidates** and must be reconciled against specific upstream GUI components before being mandatory; feature labels do not establish actual source behavior. This avoids promising non-existent source functions while allowing a thorough comparison.

The `Doofus-inspired` layout is a profile of ordinary Hermes panels, not a secret early port of the Doofus Hermes plugin. Codex and Antigravity profiles are labeled **inspired** until their independent external UI appearance has been researched and explicitly chosen.

Selected upstream Git SHA is a point-in-time research baseline. There may be additional commits; an implementing agent must refresh HEAD, choose and record the target source version once, then update the inventory without silently moving the fidelity goalpost mid-implementation.

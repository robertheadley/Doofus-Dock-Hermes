# Local patch: egui_commonmark_backend 0.26.0

Source package: crates.io egui_commonmark_backend 0.26.0, from the egui_commonmark 0.26.0 release. The upstream package notices and licenses are retained in LICENSE-MIT and LICENSE-APACHE.

Local change: the code-copy affordance uses a sized, accessible Copy/Copied text button instead of the U+1F5D0 emoji. The selected JetBrains Mono and Fira Code font set does not contain that glyph; rendering the emoji caused the egui frame to panic. The change preserves copy behavior and avoids adding a third runtime font dependency.
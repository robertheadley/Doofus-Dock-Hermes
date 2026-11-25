# Bundled typography

Both bundled typefaces retain their upstream Open Font License 1.1 text beside the font files. The TTFs are unmodified static files from official releases.

| Typeface | Role in Hermes Native | Source/release | Release archive SHA-256 | License |
|---|---|---|---|---|
| JetBrains Mono | Primary bundled face for monospace content (`Monospace` family) | [JetBrains/JetBrainsMono v2.304](https://github.com/JetBrains/JetBrainsMono/releases/tag/v2.304) | `6f6376c6ed2960ea8a963cd7387ec9d76e3f629125bc33d1fdcd7eb7012f7bbf` | [OFL-1.1](https://github.com/JetBrains/JetBrainsMono/blob/master/OFL.txt), bundled as `JetBrainsMono/OFL.txt` |
| Fira Code | Secondary bundled monospace fallback (`Monospace` family) | [tonsky/FiraCode 6.2](https://github.com/tonsky/FiraCode/releases/tag/6.2) | `0949915ba8eb24d89fd93d10a7ff623f42830d7c5ffc3ecbf960e4ecad3e3e79` | [OFL-1.1](https://github.com/tonsky/FiraCode/blob/master/LICENSE), bundled as `FiraCode/LICENSE.txt` |

The Regular faces are registered by egui, with JetBrains Mono first and Fira Code next in the monospace fallback chain. Hermes Desktop's proportional platform UI face is used for chrome, prose and chat composers; on Windows, Segoe UI is loaded from the operating system at runtime and is not bundled. Additional Bold TTFs from each downloaded release remain available as assets for future weight styling. Installed script-specific Windows faces and egui's bundled fonts provide glyph fallbacks for multilingual text, symbols and emoji. Any glyph still unavailable renders as a tofu box rather than panicking on external catalog, transcript or tool text.

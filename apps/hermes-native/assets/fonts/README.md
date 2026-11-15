# Bundled typography

Both bundled typefaces retain their upstream Open Font License 1.1 text beside the font files. The TTFs are unmodified static files from official releases.

| Typeface | Role in Hermes Native | Source/release | Release archive SHA-256 | License |
|---|---|---|---|---|
| JetBrains Mono | Application UI (`Proportional` family); medium face for headings/buttons | [JetBrains/JetBrainsMono v2.304](https://github.com/JetBrains/JetBrainsMono/releases/tag/v2.304) | `6f6376c6ed2960ea8a963cd7387ec9d76e3f629125bc33d1fdcd7eb7012f7bbf` | [OFL-1.1](https://github.com/JetBrains/JetBrainsMono/blob/master/OFL.txt), bundled as `JetBrainsMono/OFL.txt` |
| Fira Code | Monospaced UI (`Monospace` family), including the Hermes message composer | [tonsky/FiraCode 6.2](https://github.com/tonsky/FiraCode/releases/tag/6.2) | `0949915ba8eb24d89fd93d10a7ff623f42830d7c5ffc3ecbf960e4ecad3e3e79` | [OFL-1.1](https://github.com/tonsky/FiraCode/blob/master/LICENSE), bundled as `FiraCode/LICENSE.txt` |

The Regular and Medium faces are loaded by egui; the composer explicitly uses Fira Code's monospace family. Additional Bold TTFs from each downloaded release are retained as bundled assets for future weight styling. egui's default fonts remain in the fallback chain for symbols, emoji and glyphs these faces do not contain.

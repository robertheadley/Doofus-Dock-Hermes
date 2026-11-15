# Fixed target snapshot reconciliation

- Hermes Desktop acceptance target: `NousResearch/hermes-agent@8783054ef6cbafd86f10892241436862c1171a66`, downloaded 2026-11-14 and retained at local ref `refs/codex/parity/hermes-8783054`.
- Of the 565 acceptance rows, the 499 Hermes Desktop rows were initially researched against `415fa335b4d98ecf00e29e88625bb478dc63fb67`. Their IDs and descriptions are preserved. Their `source_snapshot` fields now identify the fixed implementation target.
- The other 66 rows retain Hermes-Doofus source SHA `0278571393b0ae98fd016b67cfd32f66cee986ee`; they are outside the approved stock-Hermes UI scope and remain historical supplemental requirements, not a basis to expand scope.
- All 565 result values remain `pending`. This metadata update does not claim individual source review, implementation, or visual parity. Source-dependent changes are tracked in `apps/hermes-native/docs/SOURCE_AUDIT.md`; per-capability reconciliation and paired visual acceptance remain open.
- Historical research locators in `RESEARCH_SOURCES.md`, `maps/capabilities.yaml`, and inspection receipts retain their original SHAs as provenance. Do not refresh them to a moving branch.

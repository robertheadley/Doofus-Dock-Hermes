# Benchmarks

## HUI-043 — keep the primary gateway responsive during Agent Plugin Update all

- **Scenario:** A local WebSocket fixture holds `plugins.manage action=update_all` until a profile-scoped `plugins.manage action=list` request has completed. Both requests use separate authenticated-socket worker paths in the after case. This isolates RPC scheduling; it is not a Hermes service throughput benchmark.
- **Method:** Run `cargo test -p hermes-native gateway::tests::long_plugin_update_all_does_not_block_primary_gateway_actor -- --nocapture`. The test timestamps the primary list request, waits for its receipt while update-all remains held, then releases update-all and records its RPC duration.
- **Environment:** Windows 11; local Rust test process; loopback TCP/WebSocket fixture; `hermes-native`; fixture compiled with the repository's pinned Rust dependencies.
- **Startup time:** Not measured; the test connects directly to an already-started local fixture.
- **Primary RPC latency before:** Equal to the remaining update-all duration because the serialized actor could not service the list request until update-all returned; the RPC timeout was 600 seconds.
- **Primary RPC latency after:** 64.323 ms in the recorded run, with update-all intentionally held until the list receipt arrived.
- **Update job duration:** 0.325 ms after the test released the held result. This does not represent actual plugin update duration.
- **Memory:** Not measured; this fixture validates scheduling behavior, not resident memory.
- **CPU:** Not measured; the operation is intentionally I/O-blocked and the test is not a CPU benchmark.
- **Event-loop lag:** Not applicable to this worker-level fixture. UI-frame scheduling is unchanged.
- **Limitations:** One local run proves the main actor can progress independently while the isolated RPC is pending. It does not measure authenticated Hermes latency, throughput under load, plugin job duration, or production resource use.

# Requirement Traceability Matrix (0.1.0)

This matrix maps functional/technical requirements to test coverage in the 0.1.0 release.

| Requirement | Test location | Automated |
| ----------- | ------------- | --------- |
| FR-003, TR-114 | `ocluster-config/src/validate.rs` | yes |
| FR-041, TR-085 | `ocluster-core/src/model_mode.rs` | yes |
| FR-048, TR-083 | `ocluster-core/src/fingerprint.rs` | yes |
| FR-060–FR-063, TR-060–TR-062 | `ocluster-core/src/routing.rs` | yes |
| FR-074–FR-076, TR-053 | `ocluster-core/src/retry.rs` | yes |
| FR-020–FR-021, TR-072 | `ocluster-core/src/state.rs` | yes |
| FR-031–FR-032, TR-092 | `ocluster-core/src/circuit_breaker.rs` | yes |
| FR-114, TR-111 | `ocluster-config/src/loader.rs` | yes |
| FR-151, TR-102 | `ocluster-storage/src/store.rs`, `migrations.rs` | yes |
| TXR-010, TR-212 | `mock-ollama/src/server.rs` | yes |
| TXR-022, TXR-100–TXR-150 | `ocluster/tests/e2e.rs` | yes |
| FR-140, TR-030 | `ocluster-controller/src/serve.rs` | manual |
| FR-120–FR-124, TR-160–TR-163 | `ocluster-controller/src/proxy.rs`, `serve.rs` | partial |
| FR-100–FR-104, TR-141 | `ocluster-tui/src/app.rs`, `ui.rs` | manual |
| Web admin panel | `ocluster-admin/static/`, `serve.rs` | manual |

Deferred to future releases (smoke/stub only):

- FR-142–FR-143 (remote auth)
- FR-049, TR-150–TR-156 (node agent)
- FR-052–FR-053 (model pull/delete)

Generate updated coverage with:

```bash
cargo test --workspace
rg 'Covers:' crates/
```

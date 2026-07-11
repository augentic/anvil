# Composed workflow-core harness

This non-shipped package hosts the built `specify.wasm` workflow guest with model-free echo source and target fixtures. The init seam proves `wasi:cli/run`, both adapter WIT links, dispatch by `target:echo-target`, and writes through the project and `/specify-cache` preopens. The canonical `composed-loop` scenario then runs `init → author → approve → execute` through the same deployment with request-keyed Omnia replay fixtures.

Build the guests before running the host test:

```shell
cd harness
cargo make test-composed
```

`omnia-testkit` supplies the temporary deployment manifest while `omnia_wasi_model::ModelDefault` owns request matching and replay through `WasiModelCtx`; Specify does not fork either mechanism. The checked-in fixture corpus contains only the deterministic plan-reconciliation and synthesis requests required by the echo loop. Broader workflow combinations stay in the cheaper native profile.

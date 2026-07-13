# wasm harness

Hosted-component boundary smoke: hosts the built `specify.wasm` workflow guest with the combined fixture-adapter component bound at both `source:fixture` and `target:fixture`. The same hosting shape as the shipped binary, with the model backend swapped for colocated deterministic answers.

This package owns only facts unique to the WASM/WIT seam — combined-component loading and WIT linking, metadata and operation dispatch on both axes, the WIT error lift, model-host invocation, and writes through the project and `/specify-cache` preopens. A short scripted `author → approve → execute` path is the vehicle that reaches those seams; drained-loop and artifact-completeness behaviour belong to the native suites under `crates/change/tests/`.

Build the guests before running the host test:

```shell
cd harness
cargo make test-wasm
```

Or from the repository root: `cargo make test-wasm`.

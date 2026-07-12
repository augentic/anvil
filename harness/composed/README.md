# composed harness

This non-shipped package holds the single composed WASM smoke: it hosts the built `specify.wasm` workflow guest with the combined fixture-adapter component bound at both `source:fixture` and `target:fixture`, then drives `init → author → approve → execute` through fresh in-process command-mode deployments — the same hosting shape as the shipped binary, with the model backend swapped for colocated deterministic answers.

The one test proves the WASM-only boundary: combined-component loading and WIT linking, metadata and operation dispatch on both axes, model-host invocation, writes through the project and `/specify-cache` preopens, and externally visible drained completion. Workflow behaviour beyond the boundary lives in the native suites under `crates/workflow/tests/`.

Build the guests before running the host test:

```shell
cd harness
cargo make test-wasm
```

Or from the repository root: `cargo make test-wasm`.

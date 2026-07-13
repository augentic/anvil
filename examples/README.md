# Examples

Specify's non-shipped example surfaces follow Omnia's [examples](https://github.com/augentic/omnia/tree/main/examples) shape: one package with single-file targets under `native/` and `wasm/`, driven through `[[example]]` entries in [Cargo.toml](Cargo.toml).

| Path                | Role                                                   |
| ------------------- | ------------------------------------------------------ |
| `native/live.rs`    | Explicit live-model trial                              |
| `wasm/adapter.rs`   | Combined adapter WASI component (`adapter-wasm`)       |
| `wasm/smoke.rs`     | Hosted WASM boundary smoke                             |

The deterministic fixture adapter core shared by `wasm/adapter.rs` and the native suites lives in `crates/testkit` as `testkit::adapter`.

## Commands

Run these from the repository root:

```shell
cargo make test-wasm # build guests, then run the wasm-smoke example
cargo make test-live # run the live-model example (needs cursor-agent credentials)
```

Ordinary native suites stay at the repository root: `cargo make test`.

Target-specific dependencies keep Wasmtime and the cursor backend out of the wasm32 guest build.
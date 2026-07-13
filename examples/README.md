# Examples

Specify's non-shipped example surfaces, shaped like Omnia's `[examples/](https://github.com/augentic/omnia/tree/main/examples)`: one package, single-file targets under `native/` and `wasm/`, driven through `[[example]]` entries in `[Cargo.toml](Cargo.toml)`.


| Path                | Role                                                   |
| ------------------- | ------------------------------------------------------ |
| `native/live.rs`    | Explicit live-model trial                              |
| `wasm/adapter.rs`   | Combined adapter WASI component (`adapter-wasm`)       |
| `wasm/smoke.rs`     | Hosted WASM boundary smoke                             |

The deterministic fixture adapter core both `wasm/adapter.rs` and the native suites map lives in `crates/testkit` (`testkit::adapter`).


## Commands

Run these from `examples/`:

```shell
cargo make --makefile test-wasm # build guests, then run the wasm-smoke example
cargo make --makefile test-live # live-model example (needs cursor-agent credentials)
cargo make --makefile guests    # specify.wasm + examples/adapter_wasm.wasm
cargo make --makefile lint      # focused clippy over the examples package
cargo make --makefile wasm      # wasm32-wasip2 compile check for adapter-wasm
```

Ordinary native suites stay at the repository root: `cargo make test`.

The host runners sit behind the package `host` feature so the default wasm32 guest build never compiles Wasmtime or the cursor backend.
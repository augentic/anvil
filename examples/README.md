# Examples

Specify's non-shipped example surfaces follow Omnia's [examples](https://github.com/augentic/omnia/tree/main/examples) shape: colocated runtime, guest, and deployment files driven through `[[example]]` entries in [Cargo.toml](Cargo.toml).

| Path                     | Role                                                     |
| ------------------------ | -------------------------------------------------------- |
| `prompt-eval/engine.rs`  | Live-model workflow demo: plan → execute → finalize      |
| `greeting/runtime.rs`    | Command-mode `runtime!` host (`greeting`)                 |
| `greeting/scripted.rs`   | Deterministic model backend and judgment answers          |
| `greeting/guest.rs`      | Combined source/target component (`greeting-wasm`)        |
| `greeting/omnia.toml`    | Workflow, adapter bindings, links, and writable preopens |

The greeting example mirrors Omnia's manifest-driven examples: the smoke task stages `omnia.toml` unchanged beside the built workflow and adapter components, then invokes the example runtime through Omnia's `run --config` grammar. The deterministic fixture adapter core shared by `greeting/guest.rs` and the native suites lives in `crates/testkit` as `testkit::adapter`.

## Commands

Run these from the repository root:

```shell
cargo make test-wasm # build guests, then run the manifest-hosted WASM example
cargo make prompt-eval # run the prompt evaluation (needs cursor-agent credentials)
```

Ordinary native suites stay at the repository root: `cargo make test`.

Target-specific dependencies keep Wasmtime and the cursor backend out of the wasm32 guest build.
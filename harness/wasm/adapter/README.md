# WASM adapter

The combined harness adapter component exports the additive `adapter` world — both the `source` and `target` interfaces — plus a compiled-in single-document MCP reference over `wasi:http`. Its source and target shims delegate to the sibling [`../../native/adapter`](../../native/adapter/) core, so hosted WASM deployments and native workflow tests exercise identical adapter behaviour.

It compiles against this repository's [`wit/`](../../../wit/) contract. Build it from the repository root or `harness/` with:

```shell
cargo make guests
```

The artifact lands at `target/wasm32-wasip2/debug/adapter.wasm`.

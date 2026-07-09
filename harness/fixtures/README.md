# Fixtures

Echo WASI guest components for the parked composed-deployment test rig, following the same layout as [omnia's `examples/`](https://github.com/augentic/omnia/tree/main/examples): one `fixtures` package, each guest a `crate-type = ["cdylib"]` example compiled for `wasm32-wasip2`.

## echo

The echo adapter guests — skeleton `specify:adapter` components used as fixtures by the composed runtime tests in [`harness/runtime/tests/`](../runtime/tests/):

- **`echo_source`** ([`echo/source.rs`](echo/source.rs)) exports the `source-adapter` world: `survey` returns one hardcoded lead and `extract` one trivial claim, plus a compiled-in single-document MCP references over `wasi:http`.
- **`echo_target`** ([`echo/target.rs`](echo/target.rs)) exports the `target-adapter` world with trivial, model-free operations; `describe` keys its platforms capability off the routed `adapter-id` so one binary stands in for several capability shapes.

Both compile against this repo's own [`wit/`](../../wit/) — the fixtures that let a contract revision and its seam tests land in one engine PR — and are deliberately model-free: they exercise the runtime seams, not Specify logic. Build them with:

```shell
cargo make build-guests
```

Artifacts land at `target/wasm32-wasip2/debug/examples/echo_{source,target}.wasm` (example targets always land under the target dir's `examples/` subdirectory).

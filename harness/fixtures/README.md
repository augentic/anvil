# Fixtures

The Specify-owned fixture adapter package: a deterministic, model-free **native core** ([`src/lib.rs`](src/lib.rs)) implementing both `specify:adapter` axes for engine tests, plus one WASI guest component following the same layout as [omnia's `examples/`](https://github.com/augentic/omnia/tree/main/examples) — a `crate-type = ["cdylib"]` example compiled for `wasm32-wasip2`.

## Native core

The library target supplies controlled survey/extract data (including a cross-source overlap, an authority disagreement, and an evidence gap), stable guidance, observable build output, and typed failures. The `workflow` crate's integration tests consume it through the test-only provider bridge at `crates/workflow/tests/common/fixture.rs`.

## fixture_adapter

The combined fixture-adapter guest ([`adapter.rs`](adapter.rs)) exports the additive `adapter` world — both the `source` and `target` interfaces from one component — plus a compiled-in single-document MCP references over `wasi:http`. The shim is nothing but generated WIT conversions delegating to the native core, so composed deployments (`harness/composed/`) and the native suites exercise identical adapter behaviour; `metadata` keys its platforms capability off the routed `adapter-id` so one binary stands in for several capability shapes.

It compiles against this repo's own [`wit/`](../../wit/) — the fixture that lets a contract revision and its seam tests land in one engine PR — and is deliberately model-free: it exercises the runtime seams, not Specify logic. Build it from inside `harness/` with:

```shell
cargo make guests
```

The artifact lands at `target/wasm32-wasip2/debug/examples/fixture_adapter.wasm` (example targets always land under the target dir's `examples/` subdirectory).

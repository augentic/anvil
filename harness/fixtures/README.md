# Fixtures

The Specify-owned fixture adapter package: a deterministic, model-free **native core** ([`src/lib.rs`](src/lib.rs)) implementing both `specify:adapter` axes for engine tests, plus one WASI guest component following the same layout as [omnia's `examples/`](https://github.com/augentic/omnia/tree/main/examples) — a `crate-type = ["cdylib"]` example compiled for `wasm32-wasip2`.

## Native core

The library target supplies controlled survey/extract data (including a cross-source overlap, an authority disagreement, and an evidence gap), stable guidance, observable build output, and typed failures. The workflow crates' integration tests consume it through the test-only provider bridge at `crates/change/tests/common/fixture.rs`.

Behaviour keys off the routed adapter id (`source:<name>` / `target:<name>`), so one component artifact bound under several identities supplies every profile:

- a name containing `docs` or `code` selects the adversarial two-source pair: an overlapping `login-flow` lead in both, an authority disagreement on `session.timeout` (documentation says 30 minutes, behaviour says 15), and a deliberate evidence gap behind the docs-only `password-reset` lead;
- a name containing `fail-survey`, `fail-extract`, `fail-guidance`, `fail-build`, or `fail-merge` returns the matching typed failure;
- a name containing `missing-output` reports a successful build whose declared output was never written (the `target-build-output-missing` negative case);
- any other name selects the minimal single-lead `greeting` profile used by the deterministic full-loop tests.

A build additionally honours a per-project marker file (`FAIL_BUILD_MARKER`): when it exists at the project root the build returns a *failed report* (as opposed to a seam error), so interruption tests can park and resume a run without rebinding adapters. The phased merge gates honour the analogous `FAIL_MERGE_PREFLIGHT_MARKER` / `FAIL_MERGE_POSTFLIGHT_MARKER` pair.

## fixture_adapter

The combined fixture-adapter guest ([`adapter.rs`](adapter.rs)) exports the additive `adapter` world — both the `source` and `target` interfaces from one component — plus a compiled-in single-document MCP references over `wasi:http`. The shim is nothing but generated WIT conversions delegating to the native core, so composed deployments (`harness/composed/`) and the native suites exercise identical adapter behaviour; `metadata` keys its platforms capability off the routed `adapter-id` so one binary stands in for several capability shapes.

It compiles against this repo's own [`wit/`](../../wit/) — the fixture that lets a contract revision and its seam tests land in one engine PR — and is deliberately model-free: it exercises the runtime seams, not Specify logic. Build it from inside `harness/` with:

```shell
cargo make guests
```

The artifact lands at `target/wasm32-wasip2/debug/examples/fixture_adapter.wasm` (example targets always land under the target dir's `examples/` subdirectory).

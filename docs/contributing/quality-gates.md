# Quality gates

Specify proves engine correctness from this repository alone: native integration tests are the primary surface and the live `eval` rung covers real model behavior. The placement rules live in [testing standards](../standards/testing.md); the [developer loop](dev-loop.md) maps ordinary changes onto commands. This page is the gate model — what runs when, and what each gate is allowed to prove.

## Gate 1 — repository correctness (every push)

`cargo make ci` owns formatting, lints, schemas, crate and binary integration, the `checks` package (adapter boundary + plugin authoring), and the mdBook links gate (Developer Guide link integrity, `cargo make links`). The native engine suites inside it prove the complete `init → author → approve → execute` loop through the mock adapter over the offline native provider and scripted models.

This gate is model-free and self-contained: no sibling checkout, no adapter component build, no Wasmtime in the test compile path.

## Gate 2 — prompt evaluation (operator-invoked)

`cargo make eval` from the repository root runs the native prompt-evaluation rung (the `lab` binary driving the `eval` library) over the mock adapters and a real configured model. It drives the production operator rhythm and requires a non-empty authored plan, a drained execution with every entry done, and valid requirement provenance. Per-leg repair counts are reported (not asserted) as the early warning that a prompt or schema change degraded the model's first answer.

Cadence is documented convention, not automation: before a release tag, and after judgment-prompt or answer-schema changes. Ordinary CI never calls a live model.

## The WASM seam (operator-invoked)

There is no automated WASM gate. The facts only the component boundary can prove — WIT bindings, dispatch-by-id on both axes, metadata reads, guest-to-host model wiring, preopens, the component cache, and the typed error lift across the seam — are exercised by the operator-invoked wasm example: `cargo make wasm-run` stages the checked-in [`examples/wasm/omnia.toml`](../../examples/wasm/omnia.toml) with `specify.wasm` and the per-axis mock components and runs the full `init → author → approve → execute` loop against a live model. Run it when a change crosses a WIT, dispatch, hosting, or preopen seam, and before a release tag. For a model-free signal, compile-check the guests: `cargo check --lib -p specify --examples --target wasm32-wasip2`.

## Placement decision

When adding coverage:

1. Put a private dense matrix in a kernel unit test only when integration is impractical.
2. Put one-crate public behavior in that crate's integration suite.
3. Put cross-crate workflow behavior in the native engine suites over the mock adapter and scripted answers.
4. Behavior that only a WebAssembly/WIT/runtime seam can prove has no automated home here — it is covered by the operator-run wasm example here and in `specify-adapters`.
5. If no deterministic predicate can decide the result, it has no automated home here — adapter output quality belongs to adapter authors, model transport to omnia.

Do not copy an assertion into another gate for reassurance: each fact has one owning seam.

## Boundaries

- `omnia-testkit` owns reusable model doubles, recording, temporary manifests, and runtime hosting. Specify owns workflow scenario content: mock leads, evidence, scripted answers, and assertions.
- `mock::behaviour` (in `crates/mock`) is the only adapter double; the example components (`examples/wasm/source.rs` / `target.rs`) are its WIT component examples over the SDK export macros. Do not add another mock adapter or mock-adapter copy.
- External adapters prove their own behavior against the published WIT package in `specify-adapters`; no Specify gate resolves that repository, and neither repository gates on the other's HEAD.

## Reader acceptance

- **First-time contributor choosing a command:** stay on `cargo make test`; use `cargo make wasm-run` only for component-boundary changes and `cargo make eval` only when model judgment quality matters.
- **Framework developer placing coverage:** use the placement decision above and name the one seam the assertion owns.
- **Release owner:** require gate 1 green, run gate 2 and the wasm example before tagging, and read rising repair counts as prompt drift even when the run passes.

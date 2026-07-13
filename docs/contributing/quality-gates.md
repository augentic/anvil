# Quality gates

Specify proves engine correctness from this repository alone: native integration tests are the primary surface, one WASM boundary smoke covers the component seam, and one explicit live-model test covers real model behavior. The placement rules live in [testing standards](../standards/testing.md); the [developer loop](dev-loop.md) maps ordinary changes onto commands. This page is the gate model — what runs when, and what each gate is allowed to prove.

## Gate 1 — repository correctness (every push)

`cargo make ci` owns formatting, lints, schemas, crate and binary integration, the `checks` package (adapter boundary + docs/plugin links), and a compile-only `wasm32-wasip2` check covering the workflow guest and the harness-adapter shim. The native workflow suites inside it prove the complete `init → author → approve → execute` loop through the harness adapter and scripted models.

This gate is model-free and self-contained: no sibling checkout, no adapter component build, no Wasmtime in the test compile path.

## Gate 2 — WASM boundary (weekly / path-filtered / manual; required for release)

`cargo make test-wasm` (CI: `.github/workflows/wasm.yaml`) hosts `specify.wasm` with the combined `adapter.wasm` and runs the single WASM smoke. It owns the facts only the component boundary can prove: WIT bindings, dispatch-by-id on both axes, metadata reads, guest-to-host model wiring, preopens, the component cache, and the typed error lift across the seam. A short scripted loop is the vehicle that reaches those seams — not a second workflow matrix. Drained-loop and artifact-completeness outcomes belong to Gate 1.

Cadence: weekly schedule, pull requests that touch `wit/`, `src/`, or the harness guests, and manual dispatch. Required green before a release tag. Ordinary pushes keep only the compile-only `wasm32-wasip2` check.

## Gate 3 — live model (operator-invoked)

`cargo make test-live` runs the one ignored native live-model test: adversarial fixture leads (cross-source overlap, authority disagreement, evidence gap) through the real configured model, accepted only when the deterministic validators are clean — coverage catches an unmerged overlap, provenance catches an invented requirement, tag checks catch a suppressed disagreement. Per-leg repair counts are reported (not asserted) as the early warning that a prompt or schema change degraded the model's first answer.

Cadence is documented convention, not automation: before a release tag, and after judgment-prompt or answer-schema changes. Ordinary CI never calls a live model.

## Placement decision

When adding coverage:

1. Put a private dense matrix in a kernel unit test only when integration is impractical.
2. Put one-crate public behavior in that crate's integration suite.
3. Put cross-crate workflow behavior in the native workflow suites over the harness adapter and scripted answers.
4. Extend the WASM smoke only when the behavior crosses a WebAssembly/WIT/runtime seam.
5. If no deterministic predicate can decide the result, it has no automated home here — adapter output quality belongs to adapter authors, model transport to omnia.

Do not copy an assertion into another gate for reassurance: each fact has one owning seam.

## Boundaries

- `omnia-testkit` owns reusable model doubles, recording, temporary manifests, and runtime hosting. Specify owns workflow scenario content: fixture leads, evidence, scripted answers, and assertions.
- `harness/native/adapter` is the only adapter double; `harness/wasm/adapter` is its WIT shim. Do not add another mock adapter or harness-adapter copy.
- External adapters prove their own behavior against the published WIT package in `specify-adapters`; no Specify gate resolves that repository, and neither repository gates on the other's HEAD.

## Reader acceptance

- **First-time contributor choosing a command:** stay on `cargo make test`; use `cargo make test-wasm` only for component-boundary changes and `cargo make test-live` only when model judgment quality matters.
- **Framework developer placing coverage:** use the placement decision above and name the one seam the assertion owns.
- **Release owner:** require gates 1 and 2 green, run gate 3 before tagging, and read rising repair counts as prompt drift even when the run passes.

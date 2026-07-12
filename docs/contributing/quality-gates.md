# Quality gates

Specify uses one workflow-scenario model across deterministic integration tests, composed WebAssembly conformance, and live model evaluation. The scenario says what to run and prove; its profile selects where it runs, which model backend answers judgment requests, and whether semantic grading applies.

The authoritative placement rules live in [testing standards](../standards/testing.md). The [developer loop](dev-loop.md) maps ordinary changes onto the commands below.

## Vocabulary

- **Scenario** — canonical YAML declaring setup, workflow steps, fixtures, hard assertions, semantic rubrics, expected outputs, and gate tier.
- **Profile** — execution selection over one scenario: runtime, model backend, grading mode, trial count, and report destination.
- **Runtime** — native linked-adapter execution or a composed WebAssembly deployment.
- **Model backend** — scripted answers, canonical request-key replay, or a live model.
- **Hard assertion** — mechanically decidable result such as an exit status, lifecycle state, schema result, journal event, filesystem predicate, or generated-output verifier.
- **Semantic rubric** — evidence-backed judgment of meaning or usefulness, used only by live profiles.
- **Run report** — structured result carrying source revisions, profile, trials, assertion results, rubric scores, model metadata, and retained evidence.
- **Gate** — the cadence and threshold applied to reports.

## Gate 1 — repository correctness

`cargo make ci` in each repository owns formatting, lints, schemas, authoring checks, crate integration, transport contracts, adapter-native operations, and adapter-component conformance. Neither repository gate resolves the other repository: the adapters root workspace carries no Specify dependency at all, and Specify's gates use its own echo WIT fixtures. It is model-free and runs on every commit.

## Gate 2 — engine-pinned workflow

The native profile runs canonical workflow scenarios through `specify-dev` with linked adapter crates. Omnia's `omnia-testkit` supplies request-recording `Harness`, `Scripted`, and canonical `Replay`; Specify supplies only workflow setup, execution, assertions, and reports (the canonical scenarios themselves ship embedded in the pinned `scenario` crate's catalog). This model-free gate lives in `specify-adapters/harness/native` — a standalone workspace pinned to a declared engine revision — and runs in that repository's dedicated `native-harness` CI job against that pin, never against arbitrary Specify HEAD. Compatibility with a newer engine is claimed by advancing the pin, not by a cross-repo HEAD gate.

## Gate 3 — composed WebAssembly

The `replay` profile hosts the workflow guest and adapter components. It owns WIT bindings, component dispatch, links, mount/preopen behavior, HTTP/reference wiring, and one replay-backed `init → author → approve → execute` loop. Checked-in fixtures use Omnia's replay format; Specify owns only scenario inputs and expected answers. Adapter-local composed tests remain responsible for each adapter component; Specify's `replay` profile is responsible for workflow-core orchestration across components. Cadence: the scheduled/manual composed workflow and `cargo make test-replay`, not the per-commit gate — every push still proves the guest crates compile for `wasm32-wasip2`.

## Gate 4 — live quality

Live profiles run selected scenarios with the Cursor-backed model. Each has one owning runner: `wasm-live` is the engine repo's `harness/quality` orchestrator over the in-process composed executor; `native-live` is the adapters repo's `specify-dev quality` runner over the in-process guest loop, against its declared engine pin. Hard assertions must pass mechanically on every trial. Semantic rubrics score decomposition, artifact fidelity, generated implementation, and operator ergonomics with evidence, graded through the shared `Judge` seam on the omnia model backend. Reports record subject and judge model identity, source and prompt digests, tokens, latency, generated-output verification, and retained artifacts.

Live profiles are explicit development or release gates, not ordinary CI. Release blockers run for each release; the full catalog runs per the documented release cadence. Ambiguous rubric results and publication approval remain human decisions.

## Placement decision

When adding coverage:

1. Put a private dense matrix in a kernel unit test only when integration is impractical.
2. Put one-crate public behavior in that crate's integration suite.
3. Put deterministic cross-crate workflow behavior in a canonical scenario with the native scripted profile.
4. Add the `replay` profile only when the behavior crosses a WebAssembly/WIT/runtime seam.
5. Add a semantic rubric only when no deterministic predicate can decide the result.

Do not copy an assertion into another layer for reassurance. Name the seam it owns and let other profiles reuse the same scenario result.

## Test infrastructure boundary

`omnia-testkit` is the reusable infrastructure boundary:

- `model` — scripted responses, request recording, MCP-grant inspection.
- `replay` — `omnia-wasi-model` request-key fixture replay.
- `runtime` — temporary manifests, guest discovery, in-process runtime hosting, and HTTP driving.

`crates/scenario` must not duplicate those facilities. It owns Specify's scenario schema, typed assertion registry, report types, workflow execution vocabulary, and the embedded canonical scenario catalog (`scenario::catalog`). `specify-adapters/harness/native` remains the native linked-adapter runtime and live Cursor adapter, not a second scenario catalog — it loads scenarios through the pinned crate.

## Reader acceptance

- **First-time contributor choosing a command:** start with `cargo make dev -- check`; use `cargo make dev -- live` only for model-sensitive behavior and `cargo make dev -- full` only when the WebAssembly boundary or release quality matters.
- **Framework developer placing coverage:** use the placement decision above, name one primary seam owner, and add canonical YAML only for cross-phase workflow behavior or semantic quality.
- **Release owner interpreting reports:** require every hard assertion in every trial, review semantic scores below the review threshold, retain the structured bundle, and never treat a schema-valid build report alone as generated-output proof.

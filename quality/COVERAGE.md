# Scenario coverage map

This map names the primary owner of each seam so coverage can move without being duplicated.

## Always-on repository gate

- `crates/*/tests/` owns public crate behavior and deterministic edge cases.
- `tests/framework/` owns authoring schemas, links, scenario/catalog consistency, and assertion/run-record drift.
- `specify-adapters/{sources,targets}/*/tests/` owns adapter operation behavior and prompt-envelope assembly.

## Native workflow profiles

- `harness/native/tests/full_loop.rs` executes the `intent-only`, `guest-execute-loop` happy path, and `execute-fail-resume` pilots from canonical YAML with Omnia's recorded scripted model harness.
- `harness/native/tests/replay.rs` owns canonical request-key replay compatibility.
- The remaining canonical YAML files preserve the historical scenario contract. Their `registered` hard probes name profile-specific evaluators still supplied by the legacy runbooks; concrete path/exit/JSON probes move into YAML as each runbook becomes directly executable.

## Composed WebAssembly profiles

- `quality/scenarios/composed-init.yaml` plus `harness/composed/tests/workflow.rs` owns workflow-guest command dispatch, target adapter linking, and project/cache writable preopens in CI.
- `specify-adapters`' `evals` composed target owns each adapter component's WIT exports, model bridge, MCP references, and route isolation.
- A replayed composed full loop requires a public fixture-projection API in `omnia-testkit`; until then, workflow behavior is proved natively and the WebAssembly-only seam is proved by `composed-init`.

## Live quality profiles

- `quality/run-live.sh native-live` runs three independent linked-adapter trials by default.
- `quality/run-live.sh wasm-live` runs three independent composed-deployment trials by default.
- Every trial requires all hard assertions and the `guest-spec-sensible` semantic rubric to pass. Reports retain command logs, rubric evidence, source revisions, component digests, prompt/rubric digest, model identity, duration, and generated-output verification.
- `evals/runs/` remains immutable historical evidence; new reports live under `quality/runs/`.

## Legacy compatibility

- `evals/scenarios/*.md` remains explanatory operator guidance while `quality/scenarios/*.yaml` is executable authority.
- `evals/shared/assertions.md` retains expanded probe instructions; `scenario::AssertionId` is the closed registry.
- `evals/drivers/guest-execute-loop.sh` is a compatibility wrapper. Its implementation and grading now live under `quality/`.
- Other legacy drivers remain only where a registered profile-specific evaluator has not yet become a concrete YAML probe. Do not add new lifecycle logic there.

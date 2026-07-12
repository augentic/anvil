# Scenario coverage map

This map names the primary owner of each seam so coverage can move without being duplicated.

## Always-on repository gate

- `crates/*/tests/` owns public crate behavior and deterministic edge cases.
- `tests/framework/` owns authoring schemas, links, scenario/catalog consistency, and assertion/run-record drift.
- `specify-adapters/{sources,targets}/*/tests/` owns adapter operation behavior and prompt-envelope assembly.

## Native workflow profiles

- `specify-adapters/harness/native/tests/full_loop.rs` executes the `intent-only`, `guest-execute-loop` happy path, and `execute-fail-resume` pilots from the embedded canonical catalog (`scenario::catalog`, at that harness's pinned engine revision) with Omnia's recorded scripted model harness.
- `specify-adapters/harness/native/tests/replay.rs` owns canonical request-key replay compatibility.
- Canonical YAML owns every case and assertion id. `registered` probes deliberately select profile-specific evaluators — `scenario::evaluate::guest` for the guest-loop assertions, `quality/profiles/workflow/` scripts when a check needs controlled interruption or multiple workspace slots.

## Composed WebAssembly profiles

- `quality/scenarios/composed-init.yaml` plus `harness/composed/tests/workflow.rs` owns workflow-guest command dispatch, target adapter linking, and project/cache writable preopens in CI.
- `quality/scenarios/composed-loop.yaml` drives `init → author → approve → execute` through the hosted workflow, echo source, and echo target components with checked-in Omnia replay fixtures.
- `specify-adapters`' `harness` composed target owns each adapter component's WIT exports, model bridge, MCP references, and route isolation.

## Live quality profiles

- `cargo make quality -- run native-live` runs three independent linked-adapter trials by default through `specify-dev guest-loop` (`specify-adapters/harness/native`).
- `cargo make quality -- run wasm-live` runs three independent composed-deployment trials by default through the `harness/live` driver over the shipped `specify` binary.
- Every trial requires all hard assertions and the `guest-spec-sensible` semantic rubric to pass. Reports retain command logs, rubric evidence, source revisions, component digests, prompt/rubric digest, model identity, duration, and generated-output verification.
- `quality/runs/archive/` remains immutable historical evidence; new reports live under `quality/runs/`.

## Runbooks and profile-specific evaluators

- `quality/runbooks/*.md` is explanatory operator guidance while `quality/scenarios/*.yaml` is executable authority.
- `quality/reference/assertions.md` retains expanded probe instructions; `scenario::AssertionId` is the closed registry.
- `quality/profiles/workflow/` owns evaluators that require controlled pauses, resumptions, or multiple workspace slots; the guest-loop drivers and graders are Rust (`specify-dev guest-loop`, `harness/live`, `scenario::evaluate`, `scripts/quality.rs`).
- Profile code executes YAML-owned workflow and assertion contracts; it must not define a second catalog or report shape.

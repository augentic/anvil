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

- `quality/scenarios/composed-init.yaml` plus `harness/replay/tests/workflow.rs` owns workflow-guest command dispatch, target adapter linking, and project/cache writable preopens — run by the scheduled/manual composed workflow and `cargo make test-replay`; per-push CI checks only that the guest crates compile for `wasm32-wasip2`.
- `quality/scenarios/composed-loop.yaml` drives `init → author → approve → execute` through the hosted workflow, echo source, and echo target components with checked-in Omnia replay fixtures.
- `specify-adapters`' `harness` composed target owns each adapter component's WIT exports, model bridge, MCP references, and route isolation.

## Live quality profiles

- `native-live` is owned by the adapters repo: `cargo run --manifest-path harness/native/Cargo.toml -- quality` in `specify-adapters` (or `cargo make dev -- live` here) runs three independent linked-adapter trials by default through the in-process `specify-dev` guest loop, graded through that harness's pinned `scenario` crate.
- `wasm-live` is owned by this repo: `cargo make quality -- run wasm-live` runs three independent composed-deployment trials by default through the in-process `quality::executor::ComposedExecutor` over the workflow guest and the release-built adapter components.
- Every trial requires all hard assertions and the `guest-spec-sensible` semantic rubric to pass. Reports retain command logs, rubric evidence, source revisions, component digests, prompt/rubric digest, subject and judge model identity, duration, and generated-output verification.
- Both runners write the same `scenario::bundle` layout and pass the same report-completeness validation, so reports stay comparable across repositories.
- `quality/runs/archive/` remains immutable historical evidence; new reports live under `quality/runs/`.

## Declared-vs-executed profile matrix

Every `(scenario, profile)` cell declared in canonical YAML has an owning runner and cadence; a cell nothing executes is removed from the YAML rather than left implying coverage. The reconciliation removed `native-replay` everywhere, `replay` outside the composed pair, and `native-live` outside `guest-execute-loop` — none had an executor.

| Scenario | Profile | Owning runner | Cadence |
| -------- | ------- | ------------- | ------- |
| `composed-init` | `replay` | `harness/replay/tests/workflow.rs` + the `binary_smoke` shipped-binary subprocess smoke | scheduled/manual composed workflow; `cargo make test-replay` |
| `composed-loop` | `replay` | `harness/replay/tests/workflow.rs` | scheduled/manual composed workflow; `cargo make test-replay` |
| `guest-execute-loop` | `native-scripted` | `specify-adapters/harness/native/tests/full_loop.rs` | adapters `native-harness` CI job |
| `guest-execute-loop` | `native-live` | `specify-dev quality` (`specify-adapters/harness/native`) | operator — `cargo make dev -- live` |
| `guest-execute-loop` | `wasm-live` | `harness/quality` (this repo) | operator — `cargo make dev -- full` / `cargo make quality -- run wasm-live` |
| `intent-only` | `native-scripted` | `specify-adapters/harness/native/tests/full_loop.rs` | adapters `native-harness` CI job |
| `execute-fail-resume` | `native-scripted` | `specify-adapters/harness/native/tests/full_loop.rs` | adapters `native-harness` CI job |
| `execute-fail-resume` | `wasm-live` | `quality/profiles/workflow/execute-fail-resume.sh` (bash operator evaluator) | operator, per gate tier |
| `execute-pause-resume` | `wasm-live` | `quality/profiles/workflow/execute-pause-resume.sh` (bash operator evaluator) | operator, per gate tier |
| `workspace-two-projects` / `workspace-fail-resume` / `workspace-stale-recovery` | `wasm-live` | `quality/profiles/workflow/workspace.sh` (bash operator evaluator) | operator, per gate tier |
| `contract-lifecycle` | `wasm-live` | `quality/profiles/workflow/contract-lifecycle.sh` (execute leg; plan setup stays agent-driven) | operator, per gate tier |
| `documentation-one-slice` / `documentation-multi-slice` / `lead-reconciliation` / `single-project-plan` / `target-shape` / `typescript-multi-slice` | `wasm-live` | operator/agent per runbook (prompt-driven workflow steps; no Rust runner) | operator, per gate tier; status tracked in the runbook catalog |

## Runbooks and profile-specific evaluators

- `quality/runbooks/*.md` is explanatory operator guidance while `quality/scenarios/*.yaml` is executable authority.
- `quality/reference/assertions.md` retains expanded probe instructions; `scenario::AssertionId` is the closed registry.
- `quality/profiles/workflow/` owns evaluators that require controlled pauses, resumptions, or multiple workspace slots. They are **non-catalog bash operator evaluators**, deliberately outside the two-axis Rust runner architecture (`Executor` × model backend): the ground-up quality refactor did not fold them in, and the matrix above names them explicitly so their cells are not silently attributed to the Rust runners. The uninterrupted guest-loop drivers and graders are Rust (`specify-dev quality`, `harness/quality`, `scenario::evaluate`).
- Profile code executes YAML-owned workflow and assertion contracts; it must not define a second catalog or report shape.

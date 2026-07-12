# Live quality profiles

Live evaluation is a profile of the canonical scenario system, not a separate test framework. Canonical executable cases live under [`quality/scenarios/`](../../quality/scenarios/); the assertion registry and report types live in `crates/scenario`; the complete gate model is [Quality gates](quality-gates.md).

All workflow-quality material shares the `quality/` tree: YAML scenarios are executable authority, runbooks provide operator guidance, profiles select runtimes, and archived Markdown records remain audit evidence. `harness/` implements the native and composed hosts rather than a second scenario catalog.

## What live profiles prove

Deterministic native replay owns workflow lifecycle state, schemas, journals, files, and exit contracts. The composed `wasm-replay` profile drives the canonical full loop through the hosted workflow, source, and target components with checked-in Omnia replay fixtures. Live profiles add only what requires a current model:

- decomposition and reconciliation quality;
- behavioral artifact fidelity;
- usefulness of target-specific design and generated output;
- operator-facing clarity where no structural predicate can decide the result.

Every mechanically decidable condition remains a hard assertion and must pass on every trial. Semantic output is rubric-graded with evidence; it is never byte-golden tested.

## Running workflow quality

```bash
cargo make dev -- live   # guest-execute-loop, native-live, three trials
cargo make dev -- full   # deterministic gates plus wasm-live, three trials

# Direct profile selection and overrides
TRIALS=1 cargo make quality -- run native-live
TRIALS=3 SPECIFY_EVAL_MODEL=<model-id> cargo make quality -- run wasm-live
```

Both profiles require `cursor-agent` on `PATH` with command-mode credentials. `cargo make dev -- doctor --live` verifies that path. The WebAssembly profile also requires release-built adapter components in the sibling `specify-adapters` checkout.

The live orchestrator is the Cargo Script [`scripts/quality.rs`](../../scripts/quality.rs). It creates one isolated workspace per trial, drives the canonical profile through the owning Rust harness — `specify-dev guest-loop` (in-process linked adapters) for `native-live`, `harness/live` (the shipped binary over a composed deployment) for `wasm-live` — settles hard assertions through `scenario::grade` plus the registered guest evaluators in `scenario::evaluate`, verifies generated crates, grades semantic rubrics through the live model, and writes a structured evidence bundle under `quality/runs/`. The report's `runner` field names the orchestrator and profile (`scripts/quality.rs <profile>`).

## Report contract

The top-level `report.json` follows `scenario::ScenarioReport`. It records:

- Specify and adapter revisions;
- adapter/component digests;
- model identity and prompt/rubric digest;
- start/completion times, trial duration, and token counters when the selected backend exposes them;
- every hard-assertion verdict and evidence pointer;
- every semantic rubric score, outcome, explanation, and evidence pointer;
- retained logs and generated-output verification.

Hard assertions must pass in every trial. The shared semantic rubric currently passes at 80 and requests review below 90. A release owner reviews ambiguous scores and makes the publication decision; ordinary successful live trials need no manual re-grading.

## Catalog and cadence

Gate tiers are declared in canonical YAML:

- `release-blocker` scenarios run for every release;
- `full` scenarios run per minor release or monthly, whichever comes first;
- live calls never run in ordinary per-commit CI.

The three native pilots (`intent-only`, the full-loop happy path, and `execute-fail-resume`) are deterministic scenario executions in `specify-adapters/harness/native/tests/full_loop.rs`. The composed init seam and replay-backed full loop run under `harness/composed/` on the scheduled/manual composed workflow (`cargo make test-composed` locally). The live workflow runner currently exercises the complete guest loop through `native-live` and `wasm-live`; additional live cases should reuse the same report and rubric vocabulary.

## Historical records

`quality/runs/archive/` is immutable historical evidence. Its deviations and workarounds describe the binaries used at the time and are not rewritten as current success. New release evidence belongs under `quality/runs/`.

`quality/runbooks/*.md` remains expanded operator guidance corresponding one-to-one with canonical YAML. `crates/scenario/tests/catalog.rs` enforces that mapping and keeps the assertion document aligned with the typed registry.

## Adding coverage

Before adding a semantic rubric, try to express the result as a hard assertion. Before adding a scenario, confirm the behavior crosses multiple workflow phases or depends on semantic judgment; isolated crate/CLI behavior belongs in integration tests.

New runtime execution goes under `quality/profiles/` or the owning Rust harness. Shell may select a profile, but it must not fork lifecycle scheduling, replay semantics, assertion definitions, or report shapes. Generic model and runtime mechanics stay in `omnia-testkit`.

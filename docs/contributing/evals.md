# Live quality profiles

Live evaluation is a profile of the canonical scenario system, not a separate test framework. Canonical executable cases live under [`quality/scenarios/`](../../quality/scenarios/); the assertion registry and report types live in `crates/scenario`; the complete gate model is [Quality gates](quality-gates.md).

Historical scenario prose, operator drivers, and Markdown run records remain under `evals/` as migration guidance and audit evidence. Do not add new executable behavior there.

## What live profiles prove

Deterministic native replay owns workflow lifecycle state, schemas, journals, files, and exit contracts. The model-free composed profile owns WIT links and mount behavior; full-loop composed replay remains blocked on public replay-fixture projection in Omnia. Live profiles add only what requires a current model:

- decomposition and reconciliation quality;
- behavioral artifact fidelity;
- usefulness of target-specific design and generated output;
- operator-facing clarity where no structural predicate can decide the result.

Every mechanically decidable condition remains a hard assertion and must pass on every trial. Semantic output is rubric-graded with evidence; it is never byte-golden tested.

## Running workflow quality

```bash
make dev-live   # guest-execute-loop, native-live, three trials
make dev-full   # deterministic gates plus wasm-live, three trials

# Direct profile selection and overrides
TRIALS=1 quality/run-live.sh native-live
TRIALS=3 SPECIFY_EVAL_MODEL=<model-id> quality/run-live.sh wasm-live
```

Both profiles require `cursor-agent` on `PATH` with command-mode credentials. `make dev-doctor LIVE=1` verifies that path. The WebAssembly profile also requires release-built adapter components in the sibling `specify-adapters` checkout.

`quality/run-live.sh` creates one isolated workspace per trial, executes the canonical profile, runs hard assertions, verifies generated crates, grades semantic rubrics through the live model, and writes a structured evidence bundle under `quality/runs/`.

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

The three native pilots (`intent-only`, the full-loop happy path, and `execute-fail-resume`) are deterministic scenario executions in `harness/native/tests/full_loop.rs`. The model-free composed scenario runs in CI under `harness/composed/`. The live workflow runner currently exercises the complete guest loop through `native-live` and `wasm-live`; additional live cases should reuse the same report and rubric vocabulary.

## Historical records

`evals/runs/` is immutable historical evidence. Its deviations and workarounds describe the binaries used at the time and are not rewritten as current success. New release evidence belongs under `quality/runs/`.

`evals/scenarios/*.md` remains expanded operator guidance corresponding one-to-one with canonical YAML. `crates/scenario/tests/catalog.rs` enforces that mapping and keeps the assertion document aligned with the typed registry.

## Adding coverage

Before adding a semantic rubric, try to express the result as a hard assertion. Before adding a scenario, confirm the behavior crosses multiple workflow phases or depends on semantic judgment; isolated crate/CLI behavior belongs in integration tests.

New runtime execution goes under `quality/profiles/` or the owning Rust harness. Shell may select a profile, but it must not fork lifecycle scheduling, replay semantics, assertion definitions, or report shapes. Generic model and runtime mechanics stay in `omnia-testkit`.

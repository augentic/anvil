# Live quality profiles

Live evaluation is a profile of the canonical scenario system, not a separate test framework. Canonical executable cases live under [`quality/scenarios/`](../../quality/scenarios/); the assertion registry and report types live in `crates/scenario`; the complete gate model is [Quality gates](quality-gates.md).

All workflow-quality material shares the `quality/` tree: YAML scenarios are executable authority, runbooks provide operator guidance, profiles select runtimes, and archived Markdown records remain audit evidence. `harness/` implements the native and composed hosts rather than a second scenario catalog.

## What live profiles prove

Deterministic native replay owns workflow lifecycle state, schemas, journals, files, and exit contracts. The composed `replay` profile drives the canonical full loop through the hosted workflow, source, and target components with checked-in Omnia replay fixtures. Live profiles add only what requires a current model:

- decomposition and reconciliation quality;
- behavioral artifact fidelity;
- usefulness of target-specific design and generated output;
- operator-facing clarity where no structural predicate can decide the result.

Every mechanically decidable condition remains a hard assertion and must pass on every trial. Semantic output is rubric-graded with evidence; it is never byte-golden tested.

## Running workflow quality

```bash
cargo make dev -- live   # guest-execute-loop, native-live, three trials (adapters-repo runner)
cargo make dev -- full   # deterministic gates plus wasm-live, three trials

# Direct profile selection and overrides
TRIALS=3 SPECIFY_EVAL_MODEL=<model-id> cargo make quality -- run wasm-live
TRIALS=1 cargo run --manifest-path harness/native/Cargo.toml -- quality   # native-live, from specify-adapters
```

Both profiles require `cursor-agent` on `PATH` with command-mode credentials. `cargo make dev -- doctor --live` verifies that path. The WebAssembly profile also requires release-built adapter components in the sibling `specify-adapters` checkout.

Each live profile has one owning runner, and both grade through the same pinned pipeline and write the same bundle shape:

- `wasm-live` — the workspace binary [`harness/quality`](../../harness/quality/src/main.rs) in this repo. It creates one isolated workspace per trial and drives the canonical profile through the in-process composed executor (`quality::executor::ComposedExecutor` over the workflow guest and the release-built adapter components).
- `native-live` — the `specify-dev quality` runner in `specify-adapters/harness/native`. It drives the same scenario through the in-process guest loop over the linked adapter crates, against that harness's declared engine pin (no working-tree patch flags; pin green is the contract). `cargo make dev -- live` delegates to it.

After the workflow completes, each runner settles hard assertions through `scenario::grade` plus the registered evaluators, verifies generated crates, checks the scenario's declared `expected-outputs`, grades semantic rubrics through the `Judge` seam on the omnia model backend, and writes a `scenario::bundle`-validated evidence bundle under `quality/runs/`. The report's `runner` field names the orchestrator and profile (`quality wasm-live`, `specify-dev quality native-live`).

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

The three native pilots (`intent-only`, the full-loop happy path, and `execute-fail-resume`) are deterministic scenario executions in `specify-adapters/harness/native/tests/full_loop.rs`. The composed init seam and replay-backed full loop run under `harness/replay/` on the scheduled/manual composed workflow (`cargo make test-replay` locally). The two live runners currently exercise the complete guest loop (`native-live` via `specify-dev quality`, `wasm-live` via `harness/quality`); additional live cases should reuse the same report and rubric vocabulary. The declared-vs-executed matrix in [`quality/COVERAGE.md`](../../quality/COVERAGE.md) records every `(scenario, profile)` cell's owner and cadence.

## Historical records

`quality/runs/archive/` is immutable historical evidence. Its deviations and workarounds describe the binaries used at the time and are not rewritten as current success. New release evidence belongs under `quality/runs/`.

`quality/runbooks/*.md` remains expanded operator guidance corresponding one-to-one with canonical YAML. `crates/scenario/tests/catalog.rs` enforces that mapping and keeps the assertion document aligned with the typed registry.

## Adding coverage

Before adding a semantic rubric, try to express the result as a hard assertion. Before adding a scenario, confirm the behavior crosses multiple workflow phases or depends on semantic judgment; isolated crate/CLI behavior belongs in integration tests.

New runtime execution goes under `quality/profiles/` or the owning Rust harness. Shell may select a profile, but it must not fork lifecycle scheduling, replay semantics, assertion definitions, or report shapes. Generic model and runtime mechanics stay in `omnia-testkit`.

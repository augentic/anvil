---
id: invalid-evidence
owner: lifecycle
kind: suite
backend: fixture
entrypoint: /spec:plan
stages: [plan, refine]
isolation: fresh-project
assertions:
  - plan-exists
  - validation-fails-before-synthesis
  - structured-error
  - slice-stays-refining
expected-artifacts:
  - plan.yaml
---

# Invalid Evidence schema rejection

Scenario ID: `invalid-evidence`

> **Automated (`backend: fixture`).** This scenario's assertions are deterministic and proven by a fixture-driven test in the deterministic surface — no manual sweep run is required. See [Automated coverage](#automated-coverage).

## Intent

Prove the Evidence-validation gate: when an adapter emits `Evidence` that fails `evidence.schema.json`, validation fails before synthesis runs, a structured error is returned, and the slice stays in `refining`.

## Automated coverage

Proven by `finalize_invalid_persists_no_file` in [`augentic/specify-cli` `tests/source_extract.rs`](https://github.com/augentic/specify-cli/blob/main/tests/source_extract.rs), run under `cargo make test` (and `cargo nextest run --test source_extract`).

Assertion → coverage map:

- `plan-exists`: the test seeds a plan with a bound source before extracting.
- `validation-fails-before-synthesis`: schema-invalid Evidence (missing the required `claims` field) is rejected at `source extract --phase finalize` before any synthesis runs.
- `structured-error`: the failure surfaces as `error: evidence-schema` (exit code 2), not a panic or silent skip.
- `slice-stays-refining`: validate-before-visible — no Evidence file lands on the slice path and no cache event is emitted, so the slice never leaves `refining`.

## Reproducing by hand (optional)

The fixture test is the source of truth; the steps below only reproduce it for inspection. Follow the **single-project setup** in [`shared/setup.md`](../shared/setup.md) with `specify init omnia@v1`, bind a source configured to emit schema-invalid Evidence, plan a one-slice change named `bad-evidence`, stamp Gate 1, then `/spec:refine` and capture the validation failure.

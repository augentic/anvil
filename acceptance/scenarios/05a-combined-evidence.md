---
id: combined-evidence
owner: scenarios
kind: suite
backend: fixture
entrypoint: /spec:plan
stages: [plan, refine]
isolation: fresh-project
assertions:
  - plan-exists
  - serial-extract-per-source
  - two-entry-evidence
  - sources-line-carries-both
  - deterministic-reconciliation
  - lifecycle-reaches-refined
expected-artifacts:
  - plan.yaml
  - .specify/slices/inventory-sync/spec.md
---

# Combined evidence (code + documentation), one slice

Scenario ID: `combined-evidence`

> **Automated (`backend: fixture`).** This scenario's assertions are deterministic and proven by fixture-driven tests in the deterministic surface — no manual sweep run is required. See [Automated coverage](#automated-coverage).

## Intent

Prove synthesis end to end when two agreeing sources are bound on one slice: serial `extract` per source, a two-entry `Evidence[]`, a `Sources:` line carrying both keys, deterministic reconciliation by `id` correlation, and a clean transition to `refined`.

## Automated coverage

Proven by `fan_in_twice_fan_out_once` in [`augentic/specify-cli` `tests/plan/fan_in_fan_out.rs`](https://github.com/augentic/specify-cli/blob/main/tests/plan/fan_in_fan_out.rs) (with `tests/slice/synthesize.rs::synthesize_from_is_deterministic`), run under `cargo make test`. The two sources agreeing is a fixture-Evidence input; the reconciliation over that Evidence is deterministic.

Assertion → coverage map:

- `plan-exists`: the fan-in fixture seeds a plan binding two sources (`docs`, `legacy`).
- `serial-extract-per-source`: `source extract` runs once per bound source, persisting `evidence/docs.yaml` and `evidence/legacy.yaml`.
- `two-entry-evidence`: both Evidence files are asserted present on the slice before synthesis.
- `sources-line-carries-both`: synthesis renders the combined ordered `Sources:` list (see also the `Sources: docs, legacy` assertion in `tests/slice/synthesize.rs`).
- `deterministic-reconciliation`: `synthesize_from_is_deterministic` proves byte-identical re-projection; `kernel_projection_deterministic` proves `id`-correlated requirements are stable.
- `lifecycle-reaches-refined`: the slice synthesizes drift-clean and transitions through `refined` to `built` in the fan-in path.

## Reproducing by hand (optional)

The fixture tests are the source of truth; the steps below only reproduce it for inspection. Follow the **single-project setup** in [`shared/setup.md`](../shared/setup.md) with `specify init omnia@v1`, bind a legacy repo path and an agreeing design-notes docs path, plan a one-slice change named `inventory-sync` with both sources, stamp Gate 1, then `/spec:refine` and inspect the slice's `spec.md` and evidence directory.

---
id: extract-failure
owner: lifecycle
kind: suite
backend: fixture
entrypoint: /spec:plan
stages: [plan, refine]
isolation: fresh-project
assertions:
  - plan-exists
  - slice-stays-refining
  - no-synthesis-runs
  - structured-error
expected-artifacts:
  - plan.yaml
---

# Extract failure

Scenario ID: `extract-failure`

> **Automated (`backend: fixture`).** This scenario's assertions are deterministic and proven by a fixture-driven test in the deterministic surface — no manual sweep run is required. See [Automated coverage](#automated-coverage).

## Intent

Prove the extract-failure path: when a bound source's `extract` fails to produce Evidence, the slice stays in `refining`, no synthesis can run, and a structured (not panicking) error is returned.

## Automated coverage

Proven by `finalize_missing_evidence_stays_refining` in [`augentic/specify-cli` `tests/source_extract.rs`](https://github.com/augentic/specify-cli/blob/main/tests/source_extract.rs), run under `cargo make test`. The case models the agent's `extract` brief running but staging nothing in `$SCRATCH_DIR`; the deterministic finalize seam then fails closed.

Assertion → coverage map:

- `plan-exists`: the test seeds a plan with a bound `code-typescript` source before extracting.
- `structured-error`: finalize returns the wire-stable `extract-evidence-missing` diagnostic (exit code 1) naming the missing `evidence.yaml` artifact path — never a panic or silent skip. (The error names the missing artifact, whose path embeds the bound adapter and slice; it does not echo the plan source *key*.)
- `slice-stays-refining`: validate-before-visible — no Evidence file lands on the slice path, so the slice never leaves `refining`.
- `no-synthesis-runs`: with no persisted Evidence and no cache event, synthesis has nothing to consume.

Two adjacent extract-failure modes are covered by sibling tests in the same file: a *present-but-schema-invalid* document (`finalize_invalid_persists_no_file` → `evidence-schema`) and an *out-of-sandbox* document (`sandbox_denies_out_of_scope` → `extract-evidence-missing`).

## Reproducing by hand (optional)

The fixture test is the source of truth; the steps below only reproduce it for inspection. Follow the **single-project setup** in [`shared/setup.md`](../shared/setup.md) with `specify init omnia@v1`, bind a source whose `extract` will fail to produce Evidence, plan a one-slice change named `broken-extract`, stamp Gate 1, then `/spec:refine` and capture the structured failure.

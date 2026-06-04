---
id: same-authority-conflict
owner: lifecycle
kind: suite
backend: fixture
entrypoint: /spec:plan
stages: [plan, refine]
isolation: fresh-project
assertions:
  - plan-exists
  - conflict-tag-written
  - both-values-preserved
  - lifecycle-reaches-refined
  - operator-must-reconcile
expected-artifacts:
  - plan.yaml
  - .specify/slices/retry-policy/spec.md
---

# Conflict from same-authority disagreement

Scenario ID: `same-authority-conflict`

> **Automated (`backend: fixture`).** This scenario's assertions are a deterministic kernel projection and proven by a fixture-driven test in the deterministic surface — no manual sweep run is required. See [Automated coverage](#automated-coverage).

## Intent

Prove that when two sources of the *same* authority class (two `documentation` sources) disagree on one claim, synthesis writes `[conflict]` with both values preserved as inline commentary, the lifecycle still transitions to `refined`, and the operator must reconcile (by editing or amending sources) before the requirement is meaningful.

## Automated coverage

Proven by `synthesize_resolves_same_authority_conflict` in [`augentic/specify-cli` `tests/slice/synthesize.rs`](https://github.com/augentic/specify-cli/blob/main/tests/slice/synthesize.rs), run under `cargo make test`. The authority kernel derives `Conflict` from a tie at the top authority class (`crates/workflow/src/slice/synthesis/authority.rs`); only the upstream `extract` that produces the disagreeing claims involves the live agent.

Assertion → coverage map:

- `plan-exists`: the test seeds a plan binding two `documentation`-authority sources (`docs-a`, `docs-b`).
- `conflict-tag-written`: same-class claims tie with no unique winner, so the requirement derives `status: conflict` and `spec.md` carries the `[conflict]` heading tag and `Status: conflict`.
- `both-values-preserved`: neither claim carries a `winner` marker; both sources survive in the requirement's `sources` list and in `spec.md`.
- `lifecycle-reaches-refined`: synthesis succeeds (exit 0) and persists the model, so the slice transitions to `refined` despite the conflict.
- `operator-must-reconcile`: the requirement is tagged `[conflict]` (not operative) until the operator edits or amends a source — the recovery is the general `specify plan amend` / hand-edit path.

## Reproducing by hand (optional)

The fixture test is the source of truth; the steps below only reproduce it for inspection. Follow the **single-project setup** in [`shared/setup.md`](../../shared/setup.md) with `specify init omnia@v1`, bind two `documentation` sources that disagree on the same claim, plan a one-slice change named `retry-policy`, stamp Gate 1, then `/spec:refine` and inspect `.specify/slices/retry-policy/spec.md`.

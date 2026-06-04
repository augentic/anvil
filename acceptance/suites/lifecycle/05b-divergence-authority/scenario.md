---
id: divergence-authority
owner: lifecycle
kind: suite
backend: fixture
entrypoint: /spec:plan
stages: [plan, refine]
isolation: fresh-project
assertions:
  - plan-exists
  - divergence-tag-written
  - documentation-authority-wins
  - behaviour-preserved-as-commentary
  - lifecycle-reaches-refined
expected-artifacts:
  - plan.yaml
  - .specify/slices/token-expiry/spec.md
---

# Divergence from authority resolution

Scenario ID: `divergence-authority`

> **Automated (`backend: fixture`).** This scenario's assertions are deterministic and proven by fixture-driven tests in the deterministic surface — no manual sweep run is required. See [Automated coverage](#automated-coverage).

## Intent

Prove authority-resolved disagreement: when documentation and observed legacy code disagree at different authority classes (e.g. docs say "30 minutes" expiry while code observed 24 hours), synthesis writes `[divergence]`, the higher-authority `documentation` value wins as the operative requirement, and the behaviour value is preserved as inline commentary. The slice still reaches `refined`.

## Automated coverage

Proven by `synthesize_resolves_per_kind_divergence` (with `synthesize_from_is_deterministic`) in [`augentic/specify-cli` `tests/slice/synthesize.rs`](https://github.com/augentic/specify-cli/blob/main/tests/slice/synthesize.rs), run under `cargo make test`. Authority resolution is a deterministic kernel projection over fixture Evidence; only the upstream `extract` that produces the disagreeing claims involves the live agent.

Assertion → coverage map:

- `plan-exists`: the test seeds a plan binding a `documentation` and a `behaviour` source.
- `divergence-tag-written`: `spec.md` carries the `[divergence]` heading tag and `Status: divergence`.
- `documentation-authority-wins`: `documentation` (docs) outranks `behaviour` (legacy), so the docs claim renders first with `winner: true`.
- `behaviour-preserved-as-commentary`: the legacy claim survives in the requirement with `winner: false` and renders in the ordered `Sources: docs, legacy` list.
- `lifecycle-reaches-refined`: the slice synthesizes cleanly and is drift-clean (`synthesize_then_validate_is_drift_clean`), so it transitions to `refined`.

## Reproducing by hand (optional)

The fixture test is the source of truth; the steps below only reproduce it for inspection. Follow the **single-project setup** in [`shared/setup.md`](../../shared/setup.md) with `specify init omnia@v1`, bind a docs source (`authority: documentation`) and a legacy repo (`authority: behaviour`) that disagree on one value, plan a one-slice change named `token-expiry`, stamp Gate 1, then `/spec:refine` and inspect `.specify/slices/token-expiry/spec.md`.

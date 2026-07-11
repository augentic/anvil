# Workflow quality

This tree is the canonical case and report surface for Specify workflow quality. A scenario is declared once and executed through one or more profiles; runtime-specific test code must not duplicate its lifecycle or assertions.

## Layout

```text
quality/
├── scenarios/  # canonical YAML cases
├── fixtures/   # inputs and replay rows referenced by scenarios
├── rubrics/    # semantic grading definitions for live profiles
└── runs/       # structured live/release reports
```

The Rust types, loader, assertion registry, and report shape live in `crates/scenario`. Generic model and runtime test mechanics come from `omnia-testkit`.

## Profiles

- **native-scripted** — linked adapters with ordered scripted responses; fast deterministic integration.
- **native-replay** — linked adapters with canonical request-key replay; cross-repository CI.
- **wasm-replay** — hosted workflow and adapter components; CI runs both the model-free `composed-init` seam and the replay-backed `composed-loop` scenario.
- **native-live** — linked adapters with the live Cursor backend; prompt and workflow iteration.
- **wasm-live** — composed deployment with the live Cursor backend; explicit release confidence.

Hard assertions execute automatically in every applicable profile. Semantic rubrics execute only in live profiles. Markdown may explain a case, but YAML is the executable authority.

Historical operator explanations and reports remain under `evals/` for audit and compatibility. New executable behavior belongs here; do not add another lifecycle driver to `evals/drivers/`. The exact seam ownership and remaining registered-evaluator boundary is recorded in [`COVERAGE.md`](COVERAGE.md).

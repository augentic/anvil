# Workflow quality

This tree is the canonical case and report surface for Specify workflow quality. A scenario is declared once and executed through one or more profiles; runtime-specific test code must not duplicate its lifecycle or assertions.

## Layout

```text
quality/
├── scenarios/  # canonical executable YAML cases
├── runbooks/   # expanded operator guidance keyed by scenario id
├── profiles/   # runtime/profile drivers; no scenario definitions
├── fixtures/   # executable and reference inputs plus replay rows
├── reference/  # shared assertion, setup, and inspection guidance
├── rubrics/    # semantic grading definitions for live profiles
└── runs/       # structured reports, with immutable records in archive/
```

The Rust types, loader, assertion registry, and report shape live in `crates/scenario`. Generic model and runtime test mechanics come from `omnia-testkit`.

## Profiles

- **native-scripted** — linked adapters with ordered scripted responses; fast deterministic integration.
- **native-replay** — linked adapters with canonical request-key replay; cross-repository CI.
- **wasm-replay** — hosted workflow and adapter components; the scheduled/manual composed workflow (and `cargo make test-composed`) runs both the model-free `composed-init` seam and the replay-backed `composed-loop` scenario.
- **native-live** — linked adapters with the live Cursor backend; prompt and workflow iteration.
- **wasm-live** — composed deployment with the live Cursor backend; explicit release confidence.

Hard assertions execute automatically in every applicable profile. Semantic rubrics execute only in live profiles. Markdown may explain a case, but YAML is the executable authority.

The former workflow eval tree has been fully absorbed here. There is no second scenario framework: YAML is authoritative, runbooks explain it, profiles execute it, and `harness/` supplies native/WASM runtime hosts. Adapter-local hosted quality tests live in the sibling repository's `harness/` package. The exact seam ownership is recorded in [`COVERAGE.md`](COVERAGE.md).

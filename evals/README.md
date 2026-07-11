# Historical eval surface

Executable workflow quality now lives under [`quality/`](../quality/README.md). Canonical YAML defines cases and profiles; `crates/scenario` defines assertions and reports; native/composed harnesses execute deterministic profiles; `quality/run-live.sh` executes repeated live profiles.

This directory remains for migration guidance and audit history:

- [`scenarios/`](scenarios/README.md) — expanded operator explanations corresponding one-to-one with canonical YAML.
- [`shared/`](shared/setup.md) — historical setup, inspection, prompts, assertion detail, and run template.
- [`runs/`](runs/README.md) — immutable Markdown records from earlier operator sweeps.
- [`drivers/`](drivers/README.md) — compatibility/recovery runbooks where a profile-specific registered evaluator remains.
- [`fixtures/`](fixtures/README.md) — classified executable, reference, and obsolete fixture families.

Do not add a new scenario, assertion, report format, or lifecycle driver here. Add canonical YAML under `quality/scenarios/`, typed assertion metadata in `crates/scenario`, and runtime execution under the owning harness or `quality/profiles/`.

## Current commands

```bash
make dev-check  # deterministic native profiles
make dev-live   # repeated native-live profile
make dev-full   # deterministic gates plus repeated wasm-live profile
```

See [Live quality profiles](../docs/contributing/evals.md) and [Quality gates](../docs/contributing/quality-gates.md).

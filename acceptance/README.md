# Acceptance packs

Operator-driven acceptance scenarios and reference corpora for the Specify plugin repo. (The deterministic CLI proof lives in [augentic/specify-cli](https://github.com/augentic/specify-cli) under `cargo make test`.)

A release is green only when **both** surfaces below pass.

## Automated surface — run it yourself

```bash
make acceptance   # build the binary, run lints
```

## Manual sweep — hand it to a Cursor agent

Tell an agent:

```text
Run Specify's acceptance tests and report your findings for me to review.
```

It follows the runbook in `[docs/contributing/acceptance.md](../docs/contributing/acceptance.md#agent-runbook)`: drives each `[suites/lifecycle/](suites/lifecycle/README.md)` scenario in wave order, self-grades, files a record under `[runs/](runs/README.md)`, and hands back at the human seams (forge merges, judgment calls, sign-off). **Wave 0 (`01-pure-intent`) is a hard halt.**

## Layout


| Path                                              | Role                                                                   |
| ------------------------------------------------- | ---------------------------------------------------------------------- |
| `[suites/lifecycle/](suites/lifecycle/README.md)` | Scenario catalog + one self-contained `<id>/scenario.md` per scenario. |
| `[suites/shared/](suites/shared/setup.md)`        | Shared `setup.md`, `meta-prompts.md`, `run-summary-template.md`.       |
| `[runs/](runs/README.md)`                         | Filled run records — the audit trail.                                  |
| `[examples/](examples/)`                          | Reference inputs and expected artifact shapes.                         |
| `[_lint/](_lint/)`                                | Fixtures for framework checks.                                         |


Owner-local adapter scenarios live under `[adapters/targets/<name>/tests/](../adapters/targets/contracts/tests/README.md)`.
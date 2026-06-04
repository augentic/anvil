# Acceptance packs

Operator-driven acceptance scenarios and reference corpora for the Specify plugin repo. (The deterministic CLI proof lives in [augentic/specify-cli](https://github.com/augentic/specify-cli) under `cargo make test`.)

A release is green only when **both** surfaces below pass.

## Automated surface — run it yourself

```bash
# build `specify`, run the automated acceptance tests, and symlink the
# build onto your PATH (~/.local/bin) so the manual sweep can call it
make acceptance
```

`make acceptance` symlinks the freshly built `specify` into `~/.local/bin` (override with `INSTALL_DIR=…`) and warns if that directory is not on your `PATH`. The symlink always points at the latest build, so the bare `specify` command stays current — confirm with `specify --version`.

## Manual sweep — hand it to a Cursor agent

Tell an agent:

```text
Run Specify's acceptance tests and report your findings for me to review.
```

It follows the runbook in `[docs/contributing/acceptance.md](../docs/contributing/acceptance.md#agent-runbook)`: drives each `[lifecycle/](lifecycle/README.md)` scenario in wave order, self-grades, files a record under `[runs/](runs/README.md)`, and hands back at the human seams (forge merges, judgment calls, sign-off). **Wave 0 (`01-pure-intent`) is a hard halt.**

## Layout


| Path                                | Role                                                             |
| ----------------------------------- | ---------------------------------------------------------------- |
| `[lifecycle/](lifecycle/README.md)` | Scenario catalog + one self-contained `<id>.md` per scenario.    |
| `[shared/](shared/setup.md)`        | Shared `setup.md`, `meta-prompts.md`, `run-summary-template.md`. |
| `[runs/](runs/README.md)`           | Filled run records — the audit trail.                            |
| `[examples/](examples/)`            | Reference inputs and expected artifact shapes.                   |
| `[_lint/](_lint/)`                  | Fixtures for framework checks.                                   |


Owner-local adapter scenarios live under `[adapters/targets/<name>/tests/](../adapters/targets/contracts/tests/README.md)`.
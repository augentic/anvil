# Acceptance packs

Operator-driven acceptance scenarios and reference corpora for the Specify plugin repo. (The deterministic CLI proof lives in [augentic/specify-cli](https://github.com/augentic/specify-cli) under `cargo make test`.)

A release is green only when **both** surfaces below pass.

## Run all tests

Tell a Cursor agent:

```text
Run Specify's acceptance tests and report your findings for me to review.
```

The prompt tells the agent to follow the runbook in [docs/contributing/acceptance.md](../docs/contributing/acceptance.md#agent-runbook): 

1. It runs the  non-agent, "mechanical" tests (`make acceptance`)
2. Then drives each agent-based scenario in [lifecycle/](lifecycle/README.md) in wave order
3. The agent self-grades and files a report under [runs/](runs/README.md),
4. Once done, the agent hands back at the human seams (forge merges, judgment calls, sign-off). 

**N.B. Wave 0 (**`01-pure-intent`**) is a hard halt.**

## Running the automated tests

The agent runs this for you as runbook step 1; run it directly only when you want the deterministic surface without the manual sweep.

```bash
# build `specify`, run the automated acceptance tests, and symlink the
# build onto ~/.local/bin
make acceptance
```

`make acceptance` symlinks the freshly built `specify` into `~/.local/bin` (override with `INSTALL_DIR=…`) and warns if that directory is not on your `PATH`. The symlink always points at the latest build, so the bare `specify` command stays current — confirm with `specify --version`.

## Layout


| Path                                | Role                                                             |
| ----------------------------------- | ---------------------------------------------------------------- |
| `[lifecycle/](lifecycle/README.md)` | Scenario catalog + one self-contained `<id>.md` per scenario.    |
| `[shared/](shared/setup.md)`        | Shared `setup.md`, `meta-prompts.md`, `run-summary-template.md`. |
| `[runs/](runs/README.md)`           | Filled run records — the audit trail.                            |
| `[fixtures/](fixtures/)`            | Reference inputs and expected artifact shapes.                   |


Owner-local adapter scenarios live under `[adapters/targets/<name>/tests/](../adapters/targets/contracts/tests/README.md)`.
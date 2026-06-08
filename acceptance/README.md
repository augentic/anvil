# Acceptance packs

Operator-driven acceptance scenarios and reference corpora for the Specify plugin repo. (The deterministic CLI proof lives in [augentic/specify-cli](https://github.com/augentic/specify-cli) under `cargo make test`.)

A release is green only when **both** surfaces below pass.

## Run all tests

Tell a Cursor agent:

```text
Run Specify's acceptance tests and report your findings for me to review.
```

The prompt tells the agent to follow the runbook in [docs/contributing/acceptance.md](../docs/contributing/acceptance.md#agent-runbook): 

1. It runs the non-agent static checks and prepares the build under test (`make install-specify`)
2. Then drives each agent-based scenario in [scenarios/](scenarios/README.md) in group order
3. The agent self-grades and files a report under [runs/](runs/README.md),
4. Once done, the agent hands back at the human seams (forge merges, judgment calls, sign-off). 

**N.B. the N=1 hard halt (**`01-pure-intent`**) is a hard halt.**

## Running the static checks

The agent runs this for you as runbook step 1; run it directly only when you want the static checks and a fresh build under test without the manual sweep.

```bash
# build `specify`, run the static checks (make lint), and symlink the
# build onto ~/.local/bin
make install-specify
```

`make install-specify` materializes the release binary, runs `make lint`, and symlinks `specify` into `cli.path` from [`Specify.toml`](Specify.toml) (default `~/.local/bin`), warning if that directory is not on your `PATH`. The symlink always points at the materialized build, so the bare `specify` command stays current — confirm with `specify --version`. It does not re-run the deterministic acceptance tests; those are owned by `specify-cli` (`cargo make test`).

## Layout


| Path                                | Role                                                                          |
| ----------------------------------- | ----------------------------------------------------------------------------- |
| `[scenarios/](scenarios/README.md)` | Scenario catalog + one self-contained `<id>.md` per scenario.                  |
| `[shared/](shared/setup.md)`        | Shared `setup.md`, `inspect.md`, `meta-prompts.md`, `run-summary-template.md`. |
| `[runs/](runs/README.md)`           | Filled run records — the audit trail.                                          |
| `[fixtures/](fixtures/)`            | Reference inputs and expected artifact shapes.                                 |
| `.sandbox/` (gitignored)            | Stable per-scenario run roots — browsable, inspectable, recreated per run.     |


Owner-local adapter scenarios live under `[adapters/targets/<name>/tests/](../adapters/targets/contracts/tests/README.md)`.
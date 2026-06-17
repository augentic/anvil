# Eval packs

Operator-driven eval scenarios and reference corpora for the Specify plugin repo. The scenarios covered here are agent-based. All deterministic tests live in `specify-cli`.

Before your first run, check [Prerequisites](shared/setup.md#prerequisites) — `Specify.toml` points to a valid `specify-cli` install location and network access for the adapter fetch. The finalize scenarios push to local bare-repo remotes; no forge client (`gh`) is required.

## Run all scenarios

Tell a Cursor agent:

```text
Run Specify's evals. For fail-resume scenarios (execute-fail-resume, workspace-fail-resume, execute-pause-resume, workspace-stale-recovery), follow each scenario's multi-step Invocation exactly — a parked execute is expected, not a failure. After the park, fix/breakout/resync as written, then resume until all-done.
```

The prompt tells the agent to follow the runbook in [docs/contributing/evals.md](../docs/contributing/evals.md#agent-runbook): it installs the build under test (`make install-cli`), drives each agent-based scenario in [scenarios/](scenarios/README.md), and files a report under [runs/](runs/README.md).

**N=1 hard halt:** if `[intent-only](scenarios/intent-only.md)` fails, the sweep stops there — triage and resume once it is green.

## Run a single scenario

Tell a Cursor agent to run one named scenario, e.g.:

```text
Run Specify's eval <scenario>.
```

Same delegation as **Run all scenarios**, but the agent follows the [single-scenario runbook](../docs/contributing/evals.md#running-a-single-scenario).


| Scenario                                                              | What it exercises                                                        |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `[intent-only](scenarios/intent-only.md)`                             | N=1 pure intent → one slice (**release blocker** — hard halt on failure) |
| `[documentation-one-slice](scenarios/documentation-one-slice.md)`     | Documentation source, one slice                                          |
| `[documentation-multi-slice](scenarios/documentation-multi-slice.md)` | Documentation source, multiple slices                                    |
| `[typescript-multi-slice](scenarios/typescript-multi-slice.md)`       | TypeScript code source, multiple slices                                  |
| `[lead-reconciliation](scenarios/lead-reconciliation.md)`             | Cross-source propose-time merge                                          |
| `[single-project-plan](scenarios/single-project-plan.md)`             | Single-project plan generation                                           |
| `[contract-lifecycle](scenarios/contract-lifecycle.md)`               | Cross-repo contract flow (full lifecycle, local bare-repo remotes)       |
| `[target-shape](scenarios/target-shape.md)`                           | Target `shape` injection                                                 |
| `[execute-pause-resume](scenarios/execute-pause-resume.md)`           | Step-through breakout mid-execute                                        |
| `[execute-fail-resume](scenarios/execute-fail-resume.md)`             | `/spec:execute` parks on a build failure                                 |
| `[workspace-two-projects](scenarios/workspace-two-projects.md)`       | Workspace `/spec:execute` across two projects                            |
| `[workspace-fail-resume](scenarios/workspace-fail-resume.md)`         | Workspace breakout after build failure                                   |
| `[workspace-stale-recovery](scenarios/workspace-stale-recovery.md)`   | Stale-workspace recovery                                                 |


## Installing `specify-cli` runtime

The agent runs this for you as runbook step 1; run it directly only when you want the static checks and a fresh build under test without the eval sweep.

```bash
# build `specify` from the pinned cli source and symlink the build onto
# ~/.local/bin (run `make lint` separately for the static checks)
make install-cli
```

`make install-cli` builds the resolved `cli` source from `[Specify.toml](Specify.toml)` (or a gitignored `Specify.local.toml` overlay), materializes `.cli/bin/specify`, and symlinks `specify` into `~/.local/bin` (overridable with `INSTALL_DIR=`), warning if that directory is not on your `PATH`. The symlink always points at the freshly built binary, so the bare `specify` command stays current — confirm with `specify --version`. It does not re-run the deterministic tests; those are owned by `specify-cli` (`cargo make test`).

## Layout


| Path                                | Role                                                                       |
| ----------------------------------- | -------------------------------------------------------------------------- |
| `[scenarios/](scenarios/README.md)` | Scenario catalog + one self-contained `<id>.md` per scenario.              |
| `[shared/](shared/setup.md)`        | Shared `setup.md`, `inspect.md`, `prompts.md`, `run-template.md`.          |
| `[runs/](runs/README.md)`           | Filled run records — the audit trail.                                      |
| `[drivers/](drivers/README.md)`     | Checked-in operator replay scripts (execute / workspace scenarios).        |
| `[fixtures/](fixtures/)`            | Reference inputs and expected artifact shapes.                             |
| `.sandbox/` (gitignored)            | Stable per-scenario run roots — browsable, inspectable, recreated per run. |


`.sandbox/` accumulates full per-scenario project trees (including Cargo target dirs) and is never pruned automatically — it can grow to multiple gigabytes across sweeps. Each scenario recreates its own root on the next run, so it is always safe to reclaim the space between runs with `rm -rf evals/.sandbox`.

Owner-local adapter scenarios live under `[adapters/targets/<name>/tests/](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/contracts/tests/README.md)`.
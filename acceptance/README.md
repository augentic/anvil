# Acceptance packs

Operator-driven acceptance scenarios and reference corpora for the Specify plugin repo. The scenarios covered here are agent-based. All deterministic tests live in `specify-cli`.

Before your first run, check [Prerequisites](shared/setup.md#prerequisites) — a sibling `../specify-cli` checkout, network access for the adapter fetch, and `gh` for the finalize scenarios.

## Run all scenarios

Tell a Cursor agent:

```text
Run Specify's acceptance scenarios and report your findings.
```

The prompt tells the agent to follow the runbook in [docs/contributing/acceptance.md](../docs/contributing/acceptance.md#agent-runbook): it installs the build under test (`make install-cli`), drives each agent-based scenario in [scenarios/](scenarios/README.md), and files a report under [runs/](runs/README.md).

**N=1 hard halt:** if [`01-pure-intent`](scenarios/01-pure-intent.md) fails, the sweep stops there — triage and resume once it is green.

## Run a single scenario

Tell a Cursor agent to run one named scenario, e.g.:

```text
Run Specify's acceptance scenario 01-pure-intent and report your findings.
```

Same delegation as **Run all scenarios**, but the agent follows the [single-scenario runbook](../docs/contributing/acceptance.md#running-a-single-scenario). Only the agent-based (`backend: manual`) scenarios are listed; the `backend: fixture` ones are proven by `cargo make test` in `specify-cli` (see [Automated coverage](scenarios/README.md#automated-coverage)), not by an agent.


| Scenario | What it exercises |
| --- | --- |
| [`01-pure-intent`](scenarios/01-pure-intent.md) | N=1 pure intent → one slice (**release blocker** — hard halt on failure) |
| [`02-documentation-one-slice`](scenarios/02-documentation-one-slice.md) | Documentation source, one slice |
| [`03-documentation-multi-slice`](scenarios/03-documentation-multi-slice.md) | Documentation source, multiple slices |
| [`04-code-multi-slice`](scenarios/04-code-multi-slice.md) | TypeScript code source, multiple slices |
| [`05e-cross-source-merge`](scenarios/05e-cross-source-merge.md) | Cross-source propose-time merge |
| [`plan-single-project`](scenarios/plan-single-project.md) | Single-project plan generation |
| [`cross-repo-contract-flow`](scenarios/cross-repo-contract-flow.md) | Cross-repo contract flow (full lifecycle, live forge) |
| [`05h-target-shape-injection`](scenarios/05h-target-shape-injection.md) | Target `shape` injection |
| [`08-stepthrough-breakout`](scenarios/08-stepthrough-breakout.md) | Step-through breakout mid-execute |
| [`09-execute-build-failure`](scenarios/09-execute-build-failure.md) | `/spec:execute` parks on a build failure |
| [`10-workspace-execute-two-projects`](scenarios/10-workspace-execute-two-projects.md) | Workspace `/spec:execute` across two projects |
| [`11-workspace-breakout`](scenarios/11-workspace-breakout.md) | Workspace breakout after build failure |
| [`12-dual-driving-refused`](scenarios/12-dual-driving-refused.md) | Dual-driving refused |
| [`13-stale-workspace-recovery`](scenarios/13-stale-workspace-recovery.md) | Stale-workspace recovery |


## Installing `specify-cli` runtime

The agent runs this for you as runbook step 1; run it directly only when you want the static checks and a fresh build under test without the manual sweep.

```bash
# build `specify` from the pinned cli source and symlink the build onto
# ~/.local/bin (run `make lint` separately for the static checks)
make install-cli
```

`make install-cli` builds the resolved `cli` source from [`Specify.toml`](Specify.toml) (or a gitignored `Specify.local.toml` overlay), materializes `.bin/bin/specify`, and symlinks `specify` into `~/.local/bin` (overridable with `INSTALL_DIR=`), warning if that directory is not on your `PATH`. The symlink always points at the freshly built binary, so the bare `specify` command stays current — confirm with `specify --version`. It does not re-run the deterministic acceptance tests; those are owned by `specify-cli` (`cargo make test`).

## Layout


| Path | Role |
| --- | --- |
| [`scenarios/`](scenarios/README.md) | Scenario catalog + one self-contained `<id>.md` per scenario. |
| [`shared/`](shared/setup.md) | Shared `setup.md`, `inspect.md`, `meta-prompts.md`, `run-summary-template.md`. |
| [`runs/`](runs/README.md) | Filled run records — the audit trail. |
| [`fixtures/`](fixtures/) | Reference inputs and expected artifact shapes. |
| `.sandbox/` (gitignored) | Stable per-scenario run roots — browsable, inspectable, recreated per run. |

Owner-local adapter scenarios live under [`adapters/targets/<name>/tests/`](../adapters/targets/contracts/tests/README.md).
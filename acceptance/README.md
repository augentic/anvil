# Acceptance packs

Manual acceptance scenarios and reference corpora for the Specify plugin repo. Automated proof lives in `[augentic/specify-cli](https://github.com/augentic/specify-cli)` (`cargo make test`, including `tests/fan_in_fan_out.rs`); this tree is operator-driven acceptance plus worked examples for docs and future replay.

## Quick start

Acceptance has two surfaces; a release is green only when both pass.

### Manual lifecycle sweep — hand it to a Cursor agent

**Step 1:** 

Build a `specify` binary in the sibling `[specify-cli](https://github.com/augentic/specify-cli)` repo and export `SPECIFY_BIN=/abs/path/to/specify`.

```bash
cd ../specify-cli
cargo build --release
export SPECIFY_BIN=$(pwd)/target/release/specify
```

**Step 2:** 

In an agent window say:

```text
Run Specify's acceptance tests and report your findings for me to review.
```

The agent follows the runbook in `[docs/contributing/acceptance.md](../docs/contributing/acceptance.md#agent-runbook)`: it runs the automated surface, then drives each scenario in `[suites/lifecycle/](suites/lifecycle/README.md)` in wave order using the [meta-prompts](suites/shared/meta-prompts.md), self-grades the structural assertions, files a run record under `[runs/](runs/README.md)`, and **stops to hand back** at the three human seams (real forge merges, judgment-only
assertions, `deferred` / `pure-intent` sign-off). It never fakes a forge, runner, or golden compare.

Wave 0 (`01-pure-intent`) is a hard halt: if it fails, nothing else runs until
it's green.

### Automated (deterministic) — run it yourself

```bash
make lint                                   # this repo: skill/adapter/rule/link/scenario checks
cd ../specify-cli && cargo make test        # fan_in_fan_out + CLI proof (sibling repo)
```

## Layout


| Path                                              | Role                                                                                                                                                          |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `[suites/lifecycle/](suites/lifecycle/README.md)` | The unified `lifecycle` scenario pack — the canonical catalog plus one self-contained `<id>/scenario.md` per scenario, validated by `specify lint framework`. |
| `[suites/shared/](suites/shared/setup.md)`        | Shared scenario support: `setup.md` (disposable-env + briefs), `meta-prompts.md` (operator prompts), `run-summary-template.md` (single run-record template).  |
| `[runs/](runs/README.md)`                         | Filled run records, kept separate from the catalog so scenarios stay pristine fixtures.                                                                       |
| `[examples/](examples/)`                          | Reference inputs and expected artifact shapes (`sources/`, `targets/`, `skills/`) — not scenario discovery roots.                                             |
| `[_lint/](_lint/)`                                | Non-scenario fixtures consumed by framework checks (e.g. link-check regression).                                                                              |


Owner-local adapter scenarios stay under `[adapters/targets/<name>/tests/](../adapters/targets/contracts/tests/README.md)`.

## Running acceptance

See the single entry point: `[docs/contributing/acceptance.md](../docs/contributing/acceptance.md)`. It defines the two acceptance surfaces, the agent runbook for "run specify's acceptance tests", the wave ordering and halt gate, and the green-gate signal. The scenario catalog is `[suites/lifecycle/README.md](suites/lifecycle/README.md)`.

## Discovery

`specify lint framework` discovers `acceptance/suites/<pack>/scenario.md` (umbrella) and `acceptance/suites/<pack>/<id>/scenario.md` (per-scenario, depth 3 under `suites/`), plus `adapters/targets/*/tests/*.md` and promoted `plugins/*/skills/*/fixtures/*/scenario.md`. Prose-only files (this README, `suites/shared/*`, `runs/`, catalog READMEs) lack scenario frontmatter and are skipped. See [docs/contributing/checks.md](../docs/contributing/checks.md#11-acceptance-scenario-frontmatter).
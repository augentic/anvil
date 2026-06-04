# Acceptance packs

Manual acceptance scenarios and reference corpora for the Specify plugin repo. Automated proof lives in [`augentic/specify-cli`](https://github.com/augentic/specify-cli) (`cargo make test`, including `tests/fan_in_fan_out.rs`); this tree is operator-driven acceptance plus worked examples for docs and future replay.

## Layout

| Path | Role |
| --- | --- |
| [`suites/`](suites/) | Scenario markdown validated by `specify lint framework` — each scenario is `<pack>/<scenario-id>/scenario.md` or an umbrella `<pack>/scenario.md`. |
| [`examples/`](examples/) | Reference inputs and expected artifact shapes (`sources/`, `targets/`, `skills/`) — not scenario discovery roots. |
| [`_lint/`](_lint/) | Non-scenario fixtures consumed by framework checks (e.g. link-check regression). |

Owner-local adapter scenarios stay under [`adapters/targets/<name>/tests/`](../adapters/targets/contracts/tests/README.md).

## Running acceptance

See [docs/contributing/acceptance.md](../docs/contributing/acceptance.md).

- **Cross-repo change lifecycle:** [`suites/cross-repo-workflow/`](suites/cross-repo-workflow/) — umbrella [`scenario.md`](suites/cross-repo-workflow/scenario.md), operator guide in [`operator/RUNNING.md`](suites/cross-repo-workflow/operator/RUNNING.md), queue stubs in [`queue/`](suites/cross-repo-workflow/queue/).
- **Plan authoring only:** [`suites/plan-authoring/`](suites/plan-authoring/) — [`single-project/scenario.md`](suites/plan-authoring/single-project/scenario.md), [`contract-routing/scenario.md`](suites/plan-authoring/contract-routing/scenario.md).

## Discovery

`specify lint framework` discovers `acceptance/suites/**/scenario.md` (depth 2–3 under `suites/`) plus `adapters/targets/*/tests/*.md` and promoted `plugins/*/skills/*/fixtures/*/scenario.md`. See [docs/contributing/checks.md](../docs/contributing/checks.md#11-acceptance-scenario-frontmatter).

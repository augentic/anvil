# Eval scenarios

The platform scenario pack proves the operator-facing `/spec:*` change lifecycle end-to-end across the full difficulty range — N=1 trivial through multi-repo, happy-path through failure and recovery. Each scenario is a self-contained, schema-validated `<id>.md`: open one file and you have its intent, setup, invocation, and assertions.

This README is the **single catalog** — the canonical list of scenarios, their grouping, release-blocker status, and run status. How to run the sweep (surfaces, the `specify` build on PATH, the agent runbook, the gate signal) lives in [`docs/contributing/evals.md`](../../docs/contributing/evals.md). Common setup is factored into [`shared/setup.md`](../shared/setup.md); reusable operator prompts into [`shared/prompts.md`](../shared/prompts.md); run records into [`evals/runs/`](../runs/README.md).

## Groups

The catalog drains in groups. The N=1 hard halt (`pure-intent`) is a **hard halt**: if `pure-intent` fails, record it, run nothing else, triage, resume once green. Within the remaining groups scenarios are independent and may run in any order.

### N=1 hard halt — release blocker

| Scenario | File | Status |
| --- | --- | --- |
| Pure intent, one slice | [`pure-intent`](pure-intent.md) | passed |

### Core synthesis, planning, and routing

| Scenario | File | Status |
| --- | --- | --- |
| Documentation, one slice | [`documentation-one-slice`](documentation-one-slice.md) | passed |
| Documentation, multi-slice | [`documentation-multi-slice`](documentation-multi-slice.md) | passed |
| Code, multi-slice | [`code-multi-slice`](code-multi-slice.md) | passed |
| Cross-source propose-time merge | [`cross-source-merge`](cross-source-merge.md) | passed |
| Single-project plan generation | [`plan-single-project`](plan-single-project.md) | passed |
| Cross-repo contract flow (full lifecycle) | [`cross-repo-contract-flow`](cross-repo-contract-flow.md) | pending |

### Failure and breakout paths

| Scenario | File | Status |
| --- | --- | --- |
| Target `shape` injection | [`target-shape-injection`](target-shape-injection.md) | passed |
| Step-through breakout mid-execute | [`stepthrough-breakout`](stepthrough-breakout.md) | pending |
| `/spec:execute` parks on a build failure | [`execute-build-failure`](execute-build-failure.md) | pending |
| Workspace `/spec:execute` across two projects | [`workspace-execute-two-projects`](workspace-execute-two-projects.md) | pending |
| Workspace breakout after build failure | [`workspace-breakout`](workspace-breakout.md) | pending |
| Dual-driving refused | [`dual-driving-refused`](dual-driving-refused.md) | pending |
| Stale-workspace recovery | [`stale-workspace-recovery`](stale-workspace-recovery.md) | pending |

14 scenarios, one `<id>.md` file each, all driven by the sweep. Each file is named for its frontmatter `id` (`<id>.md`) — the single identity the scenario schema validates; run order lives in the group tables above, not in the filename. Fully deterministic behavior is never a scenario: it is a named test in [`augentic/specify-cli`](https://github.com/augentic/specify-cli), run under `cargo make test` on every commit, with no catalog entry here.

## Status legend

- **pending** — operator has not run the scenario yet.
- **passed** — run completed; run-summary filled; verdict `pass`.
- **failed** — run-summary verdict `fail`; follow-up issue linked.
- **deferred** — could not run on this binary (capability missing); follow-up issue linked + release-owner sign-off required before the gate counts it.

The **release gate is green** when `cargo make test` is green in `specify-cli` (it runs there on every commit), `pure-intent` is `passed`, and every other non-deferred entry is `passed`. When the whole catalog is `passed` (or `deferred` with sign-off), record the gate as green here and flip RM-05 from *Partial* to *Done* in [`rfcs/roadmap.md`](../../rfcs/roadmap.md).

Owner-local adapter scenarios stay under [`adapters/targets/<name>/tests/`](../../adapters/targets/contracts/tests/README.md).

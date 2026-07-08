# Eval scenarios

The platform scenario pack proves the operator-facing `/spec:*` change lifecycle end-to-end across the full difficulty range — N=1 trivial through multi-repo, happy-path through failure and recovery. Each scenario is a self-contained, schema-validated `<id>.md`: open one file and you have its intent, setup, invocation, and assertions.

This README is the **single catalog** — the canonical list of scenarios, their grouping, gate tier, and run status. How to run the sweep (surfaces, the `specify` build on PATH, the agent runbook, the gate signal) lives in [`docs/contributing/evals.md`](../../docs/contributing/evals.md). Common setup is factored into [`shared/setup.md`](../shared/setup.md); reusable operator prompts into [`shared/prompts.md`](../shared/prompts.md); run records into [`evals/runs/`](../runs/README.md); per-assertion probes into [`shared/assertions.md`](../shared/assertions.md).

## Groups

The catalog drains in groups. The N=1 hard halt (`intent-only`) is a **hard halt**: if `intent-only` fails, record it, run nothing else, triage, resume once green. Within the remaining groups scenarios are independent and may run in any order.

### N=1 hard halt — release blocker

| Scenario | File | Status | Gate |
| --- | --- | --- | --- |
| Pure intent, one slice | [`intent-only`](intent-only.md) | passed | release-blocker |

### Core synthesis, planning, and routing

| Scenario | File | Status | Gate |
| --- | --- | --- | --- |
| Documentation, one slice | [`documentation-one-slice`](documentation-one-slice.md) | passed | full |
| Documentation, multi-slice | [`documentation-multi-slice`](documentation-multi-slice.md) | passed | full |
| TypeScript, multi-slice | [`typescript-multi-slice`](typescript-multi-slice.md) | passed | full |
| Cross-source propose-time merge | [`lead-reconciliation`](lead-reconciliation.md) | passed | full |
| Single-project plan generation | [`single-project-plan`](single-project-plan.md) | passed | full |
| Cross-repo contract flow (full lifecycle) | [`contract-lifecycle`](contract-lifecycle.md) | passed | full |

### Failure and breakout paths

| Scenario | File | Status | Gate |
| --- | --- | --- | --- |
| Target `shape` injection | [`target-shape`](target-shape.md) | passed | full |
| Step-through breakout mid-execute | [`execute-pause-resume`](execute-pause-resume.md) | passed | full |
| `specify plan execute` parks on a build failure | [`execute-fail-resume`](execute-fail-resume.md) | passed | release-blocker |
| Workspace `specify plan execute` across two projects | [`workspace-two-projects`](workspace-two-projects.md) | passed | release-blocker |
| Workspace breakout after build failure | [`workspace-fail-resume`](workspace-fail-resume.md) | passed | full |
| Stale-workspace recovery | [`workspace-stale-recovery`](workspace-stale-recovery.md) | passed | full |

### Composed runtime (inverted loop)

| Scenario | File | Status | Gate |
| --- | --- | --- | --- |
| Composed guest execute loop | [`guest-execute-loop`](guest-execute-loop.md) | failed | full |

14 scenarios, one `<id>.md` file each, all driven by the sweep. Each file is named for its frontmatter `id` (`<id>.md`) — the single identity the scenario schema validates; run order lives in the group tables above, not in the filename. Fully deterministic behavior is never a scenario: it is a named test in the [Rust workspace](https://github.com/augentic/specify), run under `cargo make test` on every commit, with no catalog entry here — dual-driving refusal left the catalog this way when the guest execute marker (the create-exclusive `.specify/guest.lock`, covered by `crates/workflow-lib/tests/execute.rs`) made it deterministic.

## Status legend

- **pending** — operator has not run the scenario yet.
- **parked** — no owner; excluded from the full-tier drain expectation until someone claims it. A parked row has no run record; claiming it flips the row back to `pending`. The catalog does not grow while parked rows exist.
- **passed** — run completed; run-summary filled; verdict `pass`.
- **failed** — run-summary verdict `fail`; follow-up issue linked.
- **deferred** — could not run on this binary (capability missing); follow-up issue linked + release-owner sign-off required before the gate counts it.

## Gate tiers

The **Gate** column tiers the catalog into two signals:

- **release-blocker** — the blocking set, re-proven **per release**: `intent-only` (the N=1 hard halt), `execute-fail-resume`, and `workspace-two-projects`. The **release gate is green** when every `release-blocker` row is `passed`. The `intent-only` hard halt is unchanged: if it fails, record it, run nothing else, triage, resume once green.
- **full** — the remaining scenarios, drained **per minor release or monthly**, whichever comes first. A non-blocking `failed` row is triaged via its linked follow-up issue but does not hold a release on its own. `parked` rows sit outside the drain expectation until they gain an owner.

Groups keep carrying execution order; the Gate tier never moves a scenario between groups. Flipping any row's status requires the matching committed record at [`evals/runs/<id>.<result>.md`](../runs/README.md) — the catalog↔runs check in `tests/framework_quality/scenarios.rs` enforces the agreement. When the whole catalog is `passed` (or `deferred` with sign-off), record the gate as green here.

Owner-local adapter scenarios stay under [`evals/<name>/scenarios/`](https://github.com/augentic/specify-adapters/blob/main/evals/contracts/scenarios/README.md) in `augentic/specify-adapters`.

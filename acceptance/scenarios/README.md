# Acceptance scenarios

The platform scenario pack proves the operator-facing `/spec:*` change lifecycle end-to-end across the full difficulty range — N=1 trivial through multi-repo, happy-path through failure and recovery. Each scenario is a self-contained, schema-validated `<id>.md`: open one file and you have its intent, setup, invocation, and assertions.

This README is the **single catalog** — the canonical list of scenarios, their grouping, release-blocker status, and run status. How to run the sweep (surfaces, the `specify` build on PATH, the agent runbook, the gate signal) lives in [`docs/contributing/acceptance.md`](../../docs/contributing/acceptance.md). Common setup is factored into [`shared/setup.md`](../shared/setup.md); reusable operator prompts into [`shared/meta-prompts.md`](../shared/meta-prompts.md); run records into [`acceptance/runs/`](../runs/README.md).

## Groups

The catalog drains in groups. The N=1 hard halt (`pure-intent`) is a **hard halt**: if `pure-intent` fails, record it, run nothing else, triage, resume once green. Within the remaining groups scenarios are independent and may run in any order.

### N=1 hard halt — release blocker

| Scenario | File | Status |
| --- | --- | --- |
| Pure intent, one slice | [`01-pure-intent`](01-pure-intent.md) | passed |

### Core synthesis, planning, and routing

| Scenario | File | Status |
| --- | --- | --- |
| Documentation, one slice | [`02-documentation-one-slice`](02-documentation-one-slice.md) | pending |
| Documentation, multi-slice | [`03-documentation-multi-slice`](03-documentation-multi-slice.md) | pending |
| Code, multi-slice | [`04-code-multi-slice`](04-code-multi-slice.md) | pending |
| Combined evidence (code + docs) | [`05a-combined-evidence`](05a-combined-evidence.md) | automated |
| `[divergence]` from authority resolution | [`05b-divergence-authority`](05b-divergence-authority.md) | automated |
| `[conflict]` from same-authority disagreement | [`05c-same-authority-conflict`](05c-same-authority-conflict.md) | automated |
| Cross-source propose-time merge | [`05e-cross-source-merge`](05e-cross-source-merge.md) | pending |
| Multi-repo assignment from a workspace | [`06-multi-repo-workspace`](06-multi-repo-workspace.md) | automated |
| Operator amends one-slice plan into two | [`07-amend-into-two`](07-amend-into-two.md) | automated |
| Single-project plan generation | [`plan-single-project`](plan-single-project.md) | pending |
| Contract routing plan generation | [`contract-routing`](contract-routing.md) | automated |
| Cross-repo contract flow (full lifecycle) | [`cross-repo-contract-flow`](cross-repo-contract-flow.md) | pending |

### Failure and breakout paths

| Scenario | File | Status |
| --- | --- | --- |
| Extract failure | [`05f-extract-failure`](05f-extract-failure.md) | automated |
| Invalid Evidence schema rejection | [`05g-invalid-evidence`](05g-invalid-evidence.md) | automated |
| Target `shape` injection | [`05h-target-shape-injection`](05h-target-shape-injection.md) | pending |
| Source-adapter sandbox path-denied | [`05j-source-sandbox-denied`](05j-source-sandbox-denied.md) | automated |
| Step-through breakout mid-execute | [`08-stepthrough-breakout`](08-stepthrough-breakout.md) | pending |
| `/spec:execute` parks on a build failure | [`09-execute-build-failure`](09-execute-build-failure.md) | pending |
| Workspace `/spec:execute` across two projects | [`10-workspace-execute-two-projects`](10-workspace-execute-two-projects.md) | pending |
| Workspace breakout after build failure | [`11-workspace-breakout`](11-workspace-breakout.md) | pending |
| Dual-driving refused | [`12-dual-driving-refused`](12-dual-driving-refused.md) | pending |
| Stale-workspace recovery | [`13-stale-workspace-recovery`](13-stale-workspace-recovery.md) | pending |

23 scenarios. File numbering preserves the historical queue ordering (`5x` ids verbatim) so cross-references stay stable; the frontmatter `id` is the letter-led form the scenario schema requires.

## Status legend

- **pending** — operator has not run the scenario yet.
- **passed** — run completed; run-summary filled; verdict `pass`.
- **failed** — run-summary verdict `fail`; follow-up issue linked.
- **deferred** — could not run on this binary (capability missing); follow-up issue linked + release-owner sign-off required before the gate counts it.
- **automated** — `backend: fixture`; the scenario's structural assertions are proven by a named deterministic test in `augentic/specify-cli` (run under `cargo make test`), not by a manual sweep. The scenario file's **Automated coverage** section names the test. These drop out of the manual sweep.

The **release gate is green** when `tests/plan/fan_in_fan_out.rs` passes under `cargo make test`, `pure-intent` is `passed`, every `automated` entry's named test passes under `cargo make test`, and every other non-deferred entry is `passed`. When the whole catalog is `passed` (or `deferred` with sign-off), record the gate as green here and flip RM-05 from *Partial* to *Done* in [`rfcs/roadmap.md`](../../rfcs/roadmap.md).

Owner-local adapter scenarios stay under [`adapters/targets/<name>/tests/`](../../adapters/targets/contracts/tests/README.md).

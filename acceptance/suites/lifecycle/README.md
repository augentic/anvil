# Lifecycle acceptance scenarios

The `lifecycle` pack proves the operator-facing `/spec:*` change lifecycle end-to-end across the full difficulty range — N=1 trivial through multi-repo, happy-path through failure and recovery. Each scenario is a self-contained, schema-validated `<dir>/scenario.md`: open one file and you have its intent, setup, invocation, and assertions.

This README is the **single catalog** — the canonical list of scenarios, their wave, release-blocker status, and run status. How to run the sweep (surfaces, `SPECIFY_BIN`, the agent runbook, the gate signal) lives in [`docs/contributing/acceptance.md`](../../../docs/contributing/acceptance.md). Common setup is factored into [`shared/setup.md`](../shared/setup.md); reusable operator prompts into [`shared/meta-prompts.md`](../shared/meta-prompts.md); run records into [`acceptance/runs/`](../../runs/README.md).

## Waves

The catalog drains in three waves. Wave 0 is a **hard halt**: if `pure-intent` fails, record it, run nothing else, triage, resume once green. Within Wave 1 and Wave 2 scenarios are independent and may run in any order.

### Wave 0 — release blocker

| Scenario | Directory | Status |
| --- | --- | --- |
| Pure intent, one slice | [`01-pure-intent/`](01-pure-intent/scenario.md) | failed |

### Wave 1 — core synthesis, planning, and routing

| Scenario | Directory | Status |
| --- | --- | --- |
| Documentation, one slice | [`02-documentation-one-slice/`](02-documentation-one-slice/scenario.md) | pending |
| Documentation, multi-slice | [`03-documentation-multi-slice/`](03-documentation-multi-slice/scenario.md) | pending |
| Code, multi-slice | [`04-code-multi-slice/`](04-code-multi-slice/scenario.md) | pending |
| Intra-Evidence `[conflict]` | [`05-intra-evidence-conflict/`](05-intra-evidence-conflict/scenario.md) | pending |
| Combined evidence (code + docs) | [`05a-combined-evidence/`](05a-combined-evidence/scenario.md) | automated |
| `[divergence]` from authority resolution | [`05b-divergence-authority/`](05b-divergence-authority/scenario.md) | automated |
| `[conflict]` from same-authority disagreement | [`05c-same-authority-conflict/`](05c-same-authority-conflict/scenario.md) | automated |
| Cross-source propose-time merge | [`05e-cross-source-merge/`](05e-cross-source-merge/scenario.md) | pending |
| Multi-repo assignment from a workspace | [`06-multi-repo-workspace/`](06-multi-repo-workspace/scenario.md) | automated |
| Operator amends one-slice plan into two | [`07-amend-into-two/`](07-amend-into-two/scenario.md) | automated |
| Single-project plan generation | [`plan-single-project/`](plan-single-project/scenario.md) | pending |
| Contract routing plan generation | [`contract-routing/`](contract-routing/scenario.md) | automated |
| Cross-repo contract flow (full lifecycle) | [`cross-repo-contract-flow/`](cross-repo-contract-flow/scenario.md) | pending |

### Wave 2 — failure and breakout paths

| Scenario | Directory | Status |
| --- | --- | --- |
| Extract failure | [`05f-extract-failure/`](05f-extract-failure/scenario.md) | automated |
| Invalid Evidence schema rejection | [`05g-invalid-evidence/`](05g-invalid-evidence/scenario.md) | automated |
| Target `shape` injection | [`05h-target-shape-injection/`](05h-target-shape-injection/scenario.md) | pending |
| Source-adapter sandbox path-denied | [`05j-source-sandbox-denied/`](05j-source-sandbox-denied/scenario.md) | automated |
| Step-through breakout mid-execute | [`08-stepthrough-breakout/`](08-stepthrough-breakout/scenario.md) | pending |
| `/spec:execute` parks on a build failure | [`09-execute-build-failure/`](09-execute-build-failure/scenario.md) | pending |
| Workspace `/spec:execute` across two projects | [`10-workspace-execute-two-projects/`](10-workspace-execute-two-projects/scenario.md) | pending |
| Workspace breakout after build failure | [`11-workspace-breakout/`](11-workspace-breakout/scenario.md) | pending |
| Dual-driving refused | [`12-dual-driving-refused/`](12-dual-driving-refused/scenario.md) | pending |
| Stale-workspace recovery | [`13-stale-workspace-recovery/`](13-stale-workspace-recovery/scenario.md) | pending |

24 scenarios. Directory numbering preserves the historical queue ordering (`5x` ids verbatim) so cross-references stay stable; the frontmatter `id` is the letter-led form the scenario schema requires.

## Status legend

- **pending** — operator has not run the scenario yet.
- **passed** — run completed; run-summary filled; verdict `pass`.
- **failed** — run-summary verdict `fail`; follow-up issue linked.
- **deferred** — could not run on this binary (capability missing); follow-up issue linked + release-owner sign-off required before the gate counts it.
- **automated** — `backend: fixture`; the scenario's structural assertions are proven by a named deterministic test in `augentic/specify-cli` (run under `cargo make test`), not by a manual sweep. The scenario file's **Automated coverage** section names the test. These drop out of the manual sweep.

The **release gate is green** when `tests/fan_in_fan_out.rs` passes under `cargo make test`, `pure-intent` is `passed`, every `automated` entry's named test passes under `cargo make test`, and every other non-deferred entry is `passed`. When the whole catalog is `passed` (or `deferred` with sign-off), record the gate as green here and flip RM-05 from *Partial* to *Done* in [`rfcs/roadmap.md`](../../../rfcs/roadmap.md).

Owner-local adapter scenarios stay under [`adapters/targets/<name>/tests/`](../../../adapters/targets/contracts/tests/README.md).

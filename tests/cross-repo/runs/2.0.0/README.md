# 2.0.0 acceptance run-summaries

Manual operator runs of the stable scenario IDs documented in [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md#scenario-ids), captured one file per scenario id and held here for the audit trail required by the 2.0.0 release gate.

For the full run flow — acceptance surfaces, wave ordering and halt gate, the meta-prompt-driven run loop, and the green-gate signal — see [`RUNNING.md`](../../RUNNING.md). The reusable operator prompts live in [`meta-prompts.md`](../../meta-prompts.md).

## How to run

1. Build a 2.0 `specify` binary in the [sibling specify-cli repo](https://github.com/augentic/specify-cli) and export `SPECIFY_BIN=/abs/path/to/specify`. The PATH default `specify` is the historical 0.1.0 build and is **not** the 2.0 binary.
2. Open a fresh disposable workspace per scenario.
3. Drive the documented operator script using `cursor-agent` (or your editor's slash-command runner). The skill bodies under `plugins/spec/skills/` orchestrate `/spec:plan` -> Gate 1 -> `/spec:execute` -> `/spec:finalize`; the deterministic-boundary harness in `specify-standards` framework tests does **not** simulate this layer.
4. Fill in the matching `<scenario-id>.md` from the per-scenario stub. The stub embeds the scenario prompt and links the [`run-summary-template.md`](../../run-summary-template.md) field-set; an operator copy-pastes the template body into the stub, fills it, and commits.
5. **Halt rule:** scenario #1 is the release blocker. If it fails, file the failure into `01-pure-intent.md`, do **not** continue to any other scenario (#2-#13), triage instead, then resume from #1 once green.
6. After every run, file follow-up issues (in the augentic/specify repo) for any gap the deterministic harness in `specify-standards` framework does not yet cover. Link the issue back to the run-summary.

## Queue

| Scenario | File | Status | Release blocker |
| --- | --- | --- | --- |
| #1 -- Pure intent, one slice | [`01-pure-intent.md`](01-pure-intent.md) | pending | yes |
| #2 -- Documentation, one slice | [`02-documentation-one-slice.md`](02-documentation-one-slice.md) | pending | no |
| #3 -- Documentation, multi-slice | [`03-documentation-multi-slice.md`](03-documentation-multi-slice.md) | pending | no |
| #4 -- Code, multi-slice | [`04-code-multi-slice.md`](04-code-multi-slice.md) | pending | no |
| #5 -- Intra-Evidence `[conflict]` | [`05-intra-evidence-conflict.md`](05-intra-evidence-conflict.md) | pending | no |
| #5a -- Combined evidence (code + documentation), one slice | [`05a-combined-evidence.md`](05a-combined-evidence.md) | pending | no |
| #5b -- `[divergence]` from authority resolution | [`05b-divergence-authority.md`](05b-divergence-authority.md) | pending | no |
| #5c -- `[conflict]` from same-authority disagreement | [`05c-same-authority-conflict.md`](05c-same-authority-conflict.md) | pending | no |
| #5e -- Cross-source propose-time merge | [`05e-cross-source-merge.md`](05e-cross-source-merge.md) | pending | no |
| #5f -- Extract failure | [`05f-extract-failure.md`](05f-extract-failure.md) | pending | no |
| #5g -- Invalid Evidence schema rejection | [`05g-invalid-evidence.md`](05g-invalid-evidence.md) | pending | no |
| #5h -- Target `shape` injection | [`05h-target-shape-injection.md`](05h-target-shape-injection.md) | pending | no |
| #5j -- Source-adapter sandbox path-denied | [`05j-source-sandbox-denied.md`](05j-source-sandbox-denied.md) | pending | no |
| #6 -- Multi-repo assignment from a workspace | [`06-multi-repo-workspace.md`](06-multi-repo-workspace.md) | pending | no |
| #7 -- Operator amends one-slice plan into two slices at Gate 1 | [`07-amend-into-two.md`](07-amend-into-two.md) | pending | no |
| #8 -- Step-through breakout mid-execute | [`08-stepthrough-breakout.md`](08-stepthrough-breakout.md) | pending | no |
| #9 -- `/spec:execute` parks on a build failure, operator fixes, resumes | [`09-execute-build-failure.md`](09-execute-build-failure.md) | pending | no |
| #10 -- Workspace `/spec:execute` across two projects | [`10-workspace-execute-two-projects.md`](10-workspace-execute-two-projects.md) | pending | no |
| #11 -- Workspace breakout after build failure in a slot | [`11-workspace-breakout.md`](11-workspace-breakout.md) | pending | no |
| #12 -- Dual-driving refused | [`12-dual-driving-refused.md`](12-dual-driving-refused.md) | pending | no |
| #13 -- Stale-workspace recovery | [`13-stale-workspace-recovery.md`](13-stale-workspace-recovery.md) | pending | no |

21 scenarios; the non-dense `5x` ids are preserved verbatim so cross-references stay stable.

## Status legend

- **pending** -- stub written; operator has not run the scenario yet.
- **passed** -- scenario completed; run-summary fields filled; verdict `pass`.
- **failed** -- run-summary verdict `fail`; follow-up issue linked.
- **deferred** -- scenario could not run on this binary (e.g. capability missing); follow-up issue linked.

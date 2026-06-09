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
| Combined evidence (code + docs) | `combined-evidence` | [automated](#automated-coverage) |
| `[divergence]` from authority resolution | `divergence-authority` | [automated](#automated-coverage) |
| `[conflict]` from same-authority disagreement | `same-authority-conflict` | [automated](#automated-coverage) |
| Cross-source propose-time merge | [`05e-cross-source-merge`](05e-cross-source-merge.md) | pending |
| Multi-repo assignment from a workspace | `multi-repo-workspace` | [automated](#automated-coverage) |
| Operator amends one-slice plan into two | `amend-into-two` | [automated](#automated-coverage) |
| Single-project plan generation | [`plan-single-project`](plan-single-project.md) | pending |
| Contract routing plan generation | `contract-routing` | [automated](#automated-coverage) |
| Cross-repo contract flow (full lifecycle) | [`cross-repo-contract-flow`](cross-repo-contract-flow.md) | pending |

### Failure and breakout paths

| Scenario | File | Status |
| --- | --- | --- |
| Extract failure | `extract-failure` | [automated](#automated-coverage) |
| Invalid Evidence schema rejection | `invalid-evidence` | [automated](#automated-coverage) |
| Target `shape` injection | [`05h-target-shape-injection`](05h-target-shape-injection.md) | pending |
| Source-adapter sandbox path-denied | `source-sandbox-denied` | [automated](#automated-coverage) |
| Step-through breakout mid-execute | [`08-stepthrough-breakout`](08-stepthrough-breakout.md) | pending |
| `/spec:execute` parks on a build failure | [`09-execute-build-failure`](09-execute-build-failure.md) | pending |
| Workspace `/spec:execute` across two projects | [`10-workspace-execute-two-projects`](10-workspace-execute-two-projects.md) | pending |
| Workspace breakout after build failure | [`11-workspace-breakout`](11-workspace-breakout.md) | pending |
| Dual-driving refused | [`12-dual-driving-refused`](12-dual-driving-refused.md) | pending |
| Stale-workspace recovery | [`13-stale-workspace-recovery`](13-stale-workspace-recovery.md) | pending |

23 scenarios — 14 manual `<id>.md` files driven by the sweep plus 9 `automated` (`backend: fixture`) entries whose proof lives in [Automated coverage](#automated-coverage), not a scenario file. Manual file numbering preserves the historical queue ordering (`5x` ids verbatim) so cross-references stay stable; the frontmatter `id` is the letter-led form the scenario schema requires.

## Status legend

- **pending** — operator has not run the scenario yet.
- **passed** — run completed; run-summary filled; verdict `pass`.
- **failed** — run-summary verdict `fail`; follow-up issue linked.
- **deferred** — could not run on this binary (capability missing); follow-up issue linked + release-owner sign-off required before the gate counts it.
- **automated** — `backend: fixture`; the scenario's structural assertions are proven by a named deterministic test in `augentic/specify-cli` (run under `cargo make test`), not by a manual sweep. Each one's proof and assertion → test mapping live in [Automated coverage](#automated-coverage). These drop out of the manual sweep.

The **release gate is green** when `tests/plan/end_to_end.rs` passes under `cargo make test`, `pure-intent` is `passed`, every `automated` entry's named test passes under `cargo make test`, and every other non-deferred entry is `passed`. When the whole catalog is `passed` (or `deferred` with sign-off), record the gate as green here and flip RM-05 from *Partial* to *Done* in [`rfcs/roadmap.md`](../../rfcs/roadmap.md).

## Automated coverage

The `automated` (`backend: fixture`) scenarios below are proven by named deterministic tests in [`augentic/specify-cli`](https://github.com/augentic/specify-cli), run under `cargo make test` on every commit — not by the manual sweep. This matrix is their traceability home; it replaces the per-scenario `<id>.md` files those scenarios used to carry. Links are pinned at the test-file (suite) level so a function rename in `specify-cli` cannot rot them; the function names in each row are the current entry points.

| Scenario | What it proves | Covering tests (under `cargo make test`) |
| --- | --- | --- |
| `combined-evidence` | Two agreeing sources on one slice: serial `extract` per source, two-entry Evidence, combined `Sources:` line, deterministic reconciliation, lifecycle → `refined`. | [`tests/plan/end_to_end.rs`](https://github.com/augentic/specify-cli/blob/main/tests/plan/end_to_end.rs) `fan_in_twice_fan_out_once`; [`tests/slice/synthesize.rs`](https://github.com/augentic/specify-cli/blob/main/tests/slice/synthesize.rs) `synthesize_from_is_deterministic`. |
| `divergence-authority` | Authority-resolved disagreement: `[divergence]` written, higher-authority `documentation` wins, `behaviour` preserved as commentary, lifecycle → `refined`. | [`tests/slice/synthesize.rs`](https://github.com/augentic/specify-cli/blob/main/tests/slice/synthesize.rs) `synthesize_resolves_per_kind_divergence`, `synthesize_from_is_deterministic`. |
| `same-authority-conflict` | Two same-class sources tie: `[conflict]` written, both values preserved, lifecycle → `refined`, operator must reconcile. | [`tests/slice/synthesize.rs`](https://github.com/augentic/specify-cli/blob/main/tests/slice/synthesize.rs) `synthesize_resolves_same_authority_conflict`. |
| `extract-failure` | A bound source's `extract` produces no Evidence: slice stays `refining`, no synthesis runs, structured `extract-evidence-missing` (exit 1). | [`tests/source/extract.rs`](https://github.com/augentic/specify-cli/blob/main/tests/source/extract.rs) `finalize_missing_evidence_stays_refining`. |
| `invalid-evidence` | Schema-invalid Evidence rejected before synthesis: structured `evidence-schema` (exit 2), slice stays `refining`. | [`tests/source/extract.rs`](https://github.com/augentic/specify-cli/blob/main/tests/source/extract.rs) `finalize_invalid_persists_no_file`. |
| `source-sandbox-denied` | Source-adapter sandbox holds: out-of-`$SCRATCH_DIR` Evidence denied, `$PROJECT_DIR` never preopened, slice stays `refining`, operator can rebind/drop. | [`tests/source/extract.rs`](https://github.com/augentic/specify-cli/blob/main/tests/source/extract.rs) `sandbox_denies_out_of_scope`. |
| `multi-repo-workspace` | Multi-repo plan authoring from a registry-only workspace: discriminator set, per-candidate `--project` routing, `workspace sync` before propose. | [`tests/workflow/validate.rs`](https://github.com/augentic/specify-cli/blob/main/tests/workflow/validate.rs) `plan_validate_clean_json`; [`tests/workflow/propose.rs`](https://github.com/augentic/specify-cli/blob/main/tests/workflow/propose.rs) `propose_from_fan_out_golden`, `reconcile_project_binding_required`, `propose_reconcile_project_orphan`; [`tests/workspace.rs`](https://github.com/augentic/specify-cli/blob/main/tests/workspace.rs) `planning_sync_two_symlink_peers`. |
| `amend-into-two` | Gate-1 amendment: `plan amend` splits one slice into two, dependencies stay coherent, plan re-enters `pending`. | [`tests/workflow/mutate.rs`](https://github.com/augentic/specify-cli/blob/main/tests/workflow/mutate.rs) `plan_add_appends_pending_entry_json`, `plan_amend_replaces_depends_on`, `plan_remove_refuses_when_depended_on`; [`tests/workflow/transition.rs`](https://github.com/augentic/specify-cli/blob/main/tests/workflow/transition.rs) `transition_rejects_per_entry_in_progress`. |
| `contract-routing` | Plan-only half of the contract-first path: one contract slice plus routed implementation slices with deterministic routing and `depends-on` ordering. The live-forge tail stays manual in [`cross-repo-contract-flow`](cross-repo-contract-flow.md). | [`tests/workflow/propose.rs`](https://github.com/augentic/specify-cli/blob/main/tests/workflow/propose.rs) `propose_from_fan_out_golden`, `propose_dry_run_workspace_request_golden`; [`tests/plan/end_to_end.rs`](https://github.com/augentic/specify-cli/blob/main/tests/plan/end_to_end.rs) (`depends-on` ordering). |

Owner-local adapter scenarios stay under [`adapters/targets/<name>/tests/`](../../adapters/targets/contracts/tests/README.md).

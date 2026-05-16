---
name: change-draft
description: "Author `plan.yaml` for a change via the planning brief pipeline, then stop at the operator review seam. Use when scoping a new change for review before `/change:execute` runs the per-slice loop, not when continuing into execution or finalising a merged change."
argument-hint: <change-name>
---

# Draft skill

> **Author `plan.yaml` by running the capability's planning brief pipeline and stop at the operator review seam.** `/change:draft` is the Layer 2 authoring counterpart to `/change:execute`: one *writes* the plan, the other *runs* it. The skill never writes `plan.yaml` directly; every write goes through `specify change draft` (initial scaffold) or `specify plan {add, amend}` (subsequent edits).

## Critical Path

1. **Pre-flight** — validate `<change-name>` as kebab-case. Require at least one of `from`, `against`, `source`, or a populated `change.md:inputs`. Refuse if `plan.yaml` already exists (unless `extend`).
2. **Brief scaffold** — `specify change draft <change-name> [--source <key>=<path-or-url> ...]`. Writes `change.md` and `plan.yaml` together (atomic refusal if either already exists). Skipped under `extend`.
3. **Registry validate** — `specify registry validate`. Halts on validation failures (description-missing-multi-repo, kebab violations, invalid URL, capability typo) before any brief work.
4. **Plan brief pipeline** from `capability.yaml`:
   - **(a) Discovery** — invoke the discovery brief; runs `/change:analyze` for `documentation` inputs and writes `discovery.md`. May surface a `## Proposed registry topology` block that triggers the **greenfield registry bootstrap** before step 4(b) when no `registry.yaml` exists yet. See [discovery.md](discovery.md).
   - **(b) Sync workspace** (multi-repo only) — discovery-time `specify workspace sync` + author `workspace.md`. Execution-time sync is separate and prepares only the selected entry's project unless the operator asks for more. See [sync-workspace.md](sync-workspace.md).
   - **(c) Source survey** (legacy-code sources only) — invoke `/change:survey` to mechanically decompose legacy code into surfaces and slice-sized candidates. Skip when the change has no `legacy-code` sources. See [`/change:survey` SKILL.md](../survey/SKILL.md).
   - **(d) Propose** — run the propose brief; iterate accept/edit/reject/abort per slice; `specify plan add` for each accepted slice. See [propose.md](propose.md).
   - **(e) Assignment** (multi-repo only) — infer `project` per entry; `specify plan amend --project <project>`. When an unresolved row names a project that does not exist in `registry.yaml`, run the **registry-proposal sub-step** — `specify registry add` + `specify workspace sync` — before continuing. See [assignment.md](assignment.md).
5. **Validate** — `specify plan validate`. Non-zero exit on any `Error`-level finding. Never skip this step.
6. **Hand-off summary** — print the slice count, the target projects, and any `Warning`-level validate findings the operator should be aware of before executing. Then point the operator at `specify plan status` for review, `specify plan amend` for edits, and `/change:execute loop` for the next stage.

## Orientation

`/change:draft` runs a six-step loop driven by the active capability's `capability.yaml`: pre-flight → scaffold → registry-validate → brief-pipeline → plan-validate → hand-off. Every shell-out targets the `specify` CLI; the skill writes nothing to `plan.yaml` directly. A clean `specify plan validate` plus an explicit hand-off summary is the contract this skill owes its caller — execution is a separate skill, invoked by the operator only after they have reviewed the draft.

The brief pipeline varies by input kind and registry shape. Documentation-only changes run two steps for single-repo (discovery → propose) or four for multi-repo (discovery → sync-workspace → propose → assignment). Legacy-code changes add a source survey between sync-workspace and propose: three steps for single-repo (discovery → survey → propose) or five for multi-repo (discovery → sync-workspace → survey → propose → assignment). `/change:survey` owns the legacy-code decomposition; `/change:analyze` handles `documentation` inputs only.

Modes: `extend` (append-only; skip step 2 and reuse discovery) and `dry-run` (read-only preview; suppress every write under `.specify/`). There is no `orchestrate` mode — `/change:draft` ends at hand-off, and the operator decides when to start `/change:execute loop`.

See [`references/runbook.md`](references/runbook.md) for the operational detail (invocation grammar, input kinds, kind defaults, the verbatim six-step loop body, single-writer invariant, working-directory layout, mode deltas, non-goals, and state-mutation surface).

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Invocation grammar, input kinds, kind defaults, verbatim six-step loop, single-writer invariant, working-directory layout, mode deltas, non-goals, state-mutation surface |
| [`discovery.md`](discovery.md) | Discovery brief (step 4a) — `/change:analyze` for `documentation` inputs and greenfield registry bootstrap |
| [`sync-workspace.md`](sync-workspace.md) | Sync-workspace brief (step 4b, multi-repo only) — `specify workspace sync` + `workspace.md` authoring |
| [`../survey/SKILL.md`](../survey/SKILL.md) | Source survey (step 4c, legacy-code sources only) — `/change:survey` mechanical decomposition |
| [`propose.md`](propose.md) | Propose brief (step 4d) — accept/edit/reject loop, `specify plan add` |
| [`assignment.md`](assignment.md) | Assignment brief (step 4e, multi-repo only) — `--project` inference and registry-proposal sub-step |
| [`briefs/`](briefs/) | Bundled per-capability planning briefs (`omnia/`, `vectis/`) |
| [`fixtures/`](fixtures/) | Per-flow regression fixtures (discovery, propose, multi-project, registry-proposal, dry-run, plan-multi-repo) |
| [`../../references/plan-single-writer.md`](../../references/plan-single-writer.md) | Shared single-writer contract for `plan.yaml` writes |
| [`../../references/plan-invocation.md`](../../references/plan-invocation.md) | Positional grammar, kind suffix syntax, input-sufficiency rule |
| [`../../references/plan-modes.md`](../../references/plan-modes.md) | Per-mode deltas, dry-run prohibitions, `extend` collision rules |

## Guardrails

- **Single-writer for `plan.yaml`.** Every write goes through `specify change draft` (initial scaffold, alongside `change.md`) or `specify plan {add, amend}` (subsequent edits); never edit the file by hand. See [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state) and [`../../references/plan-single-writer.md`](../../references/plan-single-writer.md).
- **Never skip `specify plan validate` (step 5).** A plan that ships to `/change:execute` without a clean validate is a regression. Validate `<change-name>` as kebab-case before any filesystem read or CLI shell-out.
- **`dry-run` MUST NOT write under `.specify/`** (no `draft` / `add` / `amend` / `transition`, no `discovery.md`). **`extend` skips step 2 entirely** and only `amend --project` may touch newly added entries — never pre-existing ones. A missing `briefs/<capability>/{discovery,propose}.md` for the active capability is a hard failure: print the resolved capability and expected paths, then exit non-zero.
- **Stop at the hand-off seam.** This skill never invokes `/change:execute` and never pushes branches or finalizes the change. After step 6, the operator decides whether to run `specify plan amend`, hand the plan to a teammate, or proceed to `/change:execute loop`; the post-execute tail is owned by `/change:finalize`.

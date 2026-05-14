---
name: change-plan
description: "Author `plan.yaml` for a change on the change surface via the planning brief pipeline; with the `orchestrate` positional, drive the cross-repo umbrella end to end. Use when scoping a new change or coordinating multi-repo execution from a single command on the `/change` surface."
argument-hint: <change-name>
---

# Plan skill

> **Author `plan.yaml` by running the capability's planning brief pipeline.** `/change:plan` is the Layer 2 authoring counterpart to `/change:execute`: one *writes* the plan, the other *runs* it. The skill never writes `plan.yaml` directly; every write goes through `specify change plan {create, add, amend}`.

## Critical Path

1. **Parse and validate inputs** — validate `<change-name>` as kebab-case. Require at least one of `from`, `against`, `source`, or a populated `change.md:inputs`. Refuse if `plan.yaml` already exists (unless `extend`).
2. **Scaffold the plan** — `specify change plan create <change-name> [--source <key>=<path-or-url> ...]`. Skipped under `extend`.
3. **Run the plan brief pipeline** from `capability.yaml`:
   - **(a) Discovery** — invoke the discovery brief via `/change:analyze`; writes `discovery.md`. May surface a `## Proposed registry topology` block that triggers the **greenfield registry bootstrap** before step 3(b) when no `registry.yaml` exists yet. See [discovery.md](discovery.md).
   - **(b) Sync workspace** (multi-repo only) — discovery-time `specify workspace sync` (may sync all projects) + author `workspace.md`. Execution-time sync is separate and prepares only the selected entry's project unless the operator asks for more. See [sync-workspace.md](sync-workspace.md).
   - **(c) Propose** — run the propose brief; iterate accept/edit/reject/abort per slice; `specify change plan add` for each accepted slice. See [propose.md](propose.md).
   - **(d) Assignment** (multi-repo only) — infer `project` per entry; `specify change plan amend --project <project>`. When an unresolved row names a project that does not exist in `registry.yaml`, run the **registry-proposal sub-step** — `specify registry add` + `specify workspace sync` — before continuing. See [assignment.md](assignment.md).
4. **Validate** — `specify change plan validate`. Non-zero exit on any `Error`-level finding. Never skip this step.
5. **Exit with hand-off summary** — point the operator at `specify change plan status` and `/change:execute loop`.

## Orientation

`/change:plan` runs a five-step loop driven by the active capability's `capability.yaml`: parse → scaffold → brief-pipeline → validate → hand-off. Every shell-out targets the `specify` CLI; the skill writes nothing to `plan.yaml` directly. A clean `specify change plan validate` is the contract this skill owes its caller.

The brief pipeline is two-step for single-repo capabilities (discovery → propose) and four-step for multi-repo (discovery → sync-workspace → propose → assignment). Multi-repo behaviour fires whenever `registry.yaml` declares more than one project; assignment infers `project` per entry and may run a registry-proposal sub-step when a row names a project that does not exist yet.

**Orchestration mode (`orchestrate`).** When `orchestrate` is set, after the five-step loop the skill continues into the cross-repo umbrella sequence (brief → registry → plan → execute → push/PR handoff → finalize after operator merge). The plan-authoring half of orchestration delegates to the same default mode documented above. See [orchestration.md](orchestration.md) for the full sequence, [shapes.md](shapes.md) for shape inference / validation, and [re-entry.md](re-entry.md) for the idempotent re-entry algorithm.

Modes: `extend` (append-only; skip step 2 and reuse discovery), `dry-run` (read-only preview; suppress every write under `.specify/`), and `orchestrate` (above) compose with the default loop. The Layer 2 planning surface is fully landed today.

See [`references/runbook.md`](references/runbook.md) for the operational detail (invocation grammar, input kinds, kind defaults, the verbatim five-step loop body, single-writer invariant, working-directory layout, mode deltas, non-goals, and state-mutation surface).

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Invocation grammar, input kinds, kind defaults, verbatim five-step loop, single-writer invariant, working-directory layout, mode deltas, non-goals, state-mutation surface |
| [`discovery.md`](discovery.md) | Discovery brief (step 3a) — `/change:analyze` integration and greenfield registry bootstrap |
| [`sync-workspace.md`](sync-workspace.md) | Sync-workspace brief (step 3b, multi-repo only) — `specify workspace sync` + `workspace.md` authoring |
| [`propose.md`](propose.md) | Propose brief (step 3c) — accept/edit/reject loop, `specify change plan add` |
| [`assignment.md`](assignment.md) | Assignment brief (step 3d, multi-repo only) — `--project` inference and registry-proposal sub-step |
| [`orchestration.md`](orchestration.md) | `orchestrate` cross-repo umbrella sequence |
| [`shapes.md`](shapes.md) | Shape inference / validation for `migrate-legacy` / `new-feature` / `update-existing` |
| [`re-entry.md`](re-entry.md) | Idempotent re-entry algorithm for `orchestrate` |
| [`briefs/`](briefs/) | Bundled per-capability planning briefs (`omnia/`, `vectis/`) |
| [`fixtures/`](fixtures/) | Per-flow regression fixtures (discovery, propose, multi-project, registry-proposal, dry-run, plan-multi-repo, shape variants) |
| [`../../references/plan-single-writer.md`](../../references/plan-single-writer.md) | Shared single-writer contract for `plan.yaml` writes |
| [`../../references/plan-invocation.md`](../../references/plan-invocation.md) | Positional grammar, kind suffix syntax, input-sufficiency rule |
| [`../../references/plan-modes.md`](../../references/plan-modes.md) | Per-mode deltas, dry-run prohibitions, `extend` collision rules |

## Guardrails

- **Single-writer for `plan.yaml`.** Every write goes through `specify change plan {create, add, amend}`; never edit the file by hand. See [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state) and [`../../references/plan-single-writer.md`](../../references/plan-single-writer.md).
- **Never skip `specify change plan validate` (step 4).** A plan that ships to `/change:execute` without a clean validate is a regression. Validate `<change-name>` as kebab-case before any filesystem read or CLI shell-out.
- **`dry-run` MUST NOT write under `.specify/`** (no `create` / `add` / `amend` / `transition`, no `discovery.md`). **`extend` skips step 2 entirely** and only `amend --project` may touch newly added entries — never pre-existing ones. A missing `briefs/<capability>/{discovery,propose}.md` for the active capability is a hard failure: print the resolved capability and expected paths, then exit non-zero.

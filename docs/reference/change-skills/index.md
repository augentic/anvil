# Change skills

Change skills coordinate one or more slices through `change.md` and `plan.yaml`. They drive the operator rhythm: plan, operator review step (Gate 1), execute, finalize.

## The change rhythm

<div class="pipeline">


![Default workflow poster](../../assets/diagrams/quick-reference/workflow-poster.svg)

<p class="pipeline-caption">plan → Gate 1 → execute → finalize; slice loop runs inside execute.</p>
</div>


Inside `specify plan execute`, each slice runs through the per-slice loop documented in [Slice skills](../slice-skills/index.md): refine → build → merge. Every phase is also reachable as a manual breakout when execute parks or when you want to drive one slice by hand.

Each row below links to a per-phase stub; the skills are ultrathin wrappers, and the orchestration behind each phase lives in the guest-routed `specify` verb the wrapper invokes.

## Skill summary

| Skill | Purpose | Reads | Writes |
| ----- | ------- | ----- | ------ |
| [/spec:plan](plan.md) | Survey sources, propose slices, exit at `pending` | Bound sources, `project.yaml` | `change.md`, `plan.yaml`, `discovery.md` |
| [specify plan execute](execute.md) | Drive approved plan through refine → build → merge (CLI verb, no skill wrapper) | `plan.yaml`, slice metadata | Per-entry `in-progress`; merge writes `done` |
| [/spec:finalize](finalize.md) | Push branches, archive plan | Drained plan, workspace slots | Archived plan; no direct `.specify/` writes |

## How skills delegate

Each skill is an ultrathin invoke-and-relay wrapper over one guest-routed `specify` verb — plan authoring, lifecycle transitions, spec merging, and workspace sync all run inside the CLI. See [AGENTS.md § Skill / CLI responsibility split](../../../AGENTS.md) for the contract.

Per-slice work (`/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`) lives in [Slice skills](../slice-skills/index.md). `specify plan execute` sequences the same orchestrations; the same guest legs run when you invoke a breakout by hand.

## See also

- [Quick reference card](../quick-reference.md) — every skill and CLI verb at a glance
- [Lifecycle](../lifecycle.md) — plan, per-entry, and slice state machines
- [The layered stack](../../explanation/layered-stack.md) — Layer 2 (change) composes on Layer 1 (slice)

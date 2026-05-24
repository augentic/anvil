# Change skills

Change skills coordinate one or more slices through `change.md` and `plan.yaml`. They drive the operator rhythm: plan, operator review step (Gate 1), execute, finalize.

## The change rhythm

<div class="pipeline">


![Default workflow poster](../../assets/diagrams/quick-reference/workflow-poster.svg)

<p class="pipeline-caption">plan → Gate 1 → execute → finalize; slice loop runs inside execute.</p>
</div>


Inside `/spec:execute`, each slice runs through the per-slice loop documented in [Slice skills](../slice-skills/index.md): refine → build → merge. Every phase is also reachable as a manual breakout when execute parks or when you want to drive one slice by hand.

## Skill summary

| Skill | Purpose | Reads | Writes |
| ----- | ------- | ----- | ------ |
| [/spec:plan](plan.md) | Enumerate sources, propose slices, exit at `pending` | Bound sources, `project.yaml` | `change.md`, `plan.yaml`, `discovery.md` |
| [/spec:execute](execute.md) | Drive reviewed plan through refine → build → merge | `plan.yaml`, slice metadata | Per-entry `in-progress`; merge writes `done` |
| [/spec:finalize](finalize.md) | Push branches, observe PRs, archive plan | Drained plan, workspace slots | Archived plan; no direct `.specify/` writes |

## How skills delegate

Each skill is an agent-driven orchestrator. Deterministic operations — plan creation, lifecycle transitions, spec merging, workspace sync — run through the `specify` CLI. Skills never hand-edit `.metadata.yaml`, never create directories under `.specify/` with shell tools, and never move files into `.specify/archive/` directly.

Per-slice work (`/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`) lives in [Slice skills](../slice-skills/index.md). `/spec:execute` sequences those skills; the same bodies run when you invoke a breakout by hand.

## See also

- [Quick reference card](../quick-reference.md) — every skill and CLI verb at a glance
- [Lifecycle](../lifecycle.md) — plan, per-entry, and slice state machines
- [The layered stack](../../explanation/layered-stack.md) — Layer 2 (change) composes on Layer 1 (slice)

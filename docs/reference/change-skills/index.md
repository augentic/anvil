# Change skills

Change skills coordinate one or more slices through `change.md` and `plan.yaml`. They drive the operator rhythm: plan, operator review step (Gate 1), execute, finalize.

## The change rhythm

<div class="pipeline">


![Default workflow poster](../../assets/diagrams/quick-reference/workflow-poster.svg)

<p class="pipeline-caption">plan → Gate 1 → execute → finalize; slice loop runs inside execute.</p>
</div>


Inside `specify plan execute`, each slice runs through the per-slice loop documented in [Slice skills](../slice-skills/index.md): refine → build → merge. Every phase is also reachable as a manual breakout when execute parks or when you want to drive one slice by hand.

Canonical skill bodies live under [`plugins/spec/skills/`](../../../plugins/spec/README.md). Orchestration behind each phase lives in the guest-routed `specify` verb the wrapper invokes.

## Skill summary

| Skill | Purpose | Canonical body | CLI |
| ----- | ------- | -------------- | --- |
| `/spec:plan` | Survey sources, propose slices, exit at `pending` | [`plan/SKILL.md`](../../../plugins/spec/skills/plan/SKILL.md) | [specify plan](../cli/plan.md) |
| `specify plan execute` | Drive approved plan through refine → build → merge | — (CLI verb, no skill wrapper) | [plan execute](../cli/plan.md#specify-plan-execute) |
| `/spec:finalize` | Push branches, archive plan | [`finalize/SKILL.md`](../../../plugins/spec/skills/finalize/SKILL.md) | [specify plan](../cli/plan.md) |

## How skills delegate

Each skill is an ultrathin invoke-and-relay wrapper over one guest-routed `specify` verb — plan authoring, lifecycle transitions, spec merging, and plan archival run inside the CLI. Workspace slot materialization and repository publication are operator-owned outside Specify. See [AGENTS.md § Skill / CLI responsibility split](../../../AGENTS.md) for the contract.

Per-slice work (`/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`) lives in [Slice skills](../slice-skills/index.md).

## See also

- [Amend a plan at Gate 1](../../how-to/amend-plan-at-gate-1.md)
- [Bind multiple sources](../../how-to/bind-multiple-sources.md)
- [Quick reference card](../quick-reference.md)
- [Lifecycle](../lifecycle.md)
- [The layered stack](../../explanation/layered-stack.md)

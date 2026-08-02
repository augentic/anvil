# Change skills

Change skills coordinate one or more slices through `change.md` and `plan.yaml`. They drive the operator rhythm: plan, operator review, execute, finalize.

## The change rhythm

<div class="pipeline">


![Default workflow poster](../../assets/diagrams/quick-reference/workflow-poster.svg)

<p class="pipeline-caption">plan → review → execute → finalize; slice loop runs inside execute.</p>
</div>


Inside `emery plan execute`, each slice runs through the per-slice loop documented in [Slice skills](../slice-skills/index.md): refine → build → merge. Every phase is also reachable as a manual breakout when execute stops or when you want to drive one slice by hand.

Canonical skill bodies live under [`plugins/emery/skills/`](../../../plugins/emery/README.md). Orchestration behind each phase lives in the `emery` verb the wrapper invokes.

## Skill summary

| Skill | Purpose | Canonical body | CLI |
| ----- | ------- | -------------- | --- |
| `/emery:plan` | Survey sources, propose slices, exit at `pending` | [`plan/SKILL.md`](../../../plugins/emery/skills/plan/SKILL.md) | [emery plan](../cli/plan.md) |
| `/emery:execute` | Drive the plan through refine → build → merge (running it is the approval) | [`execute/SKILL.md`](../../../plugins/emery/skills/execute/SKILL.md) | [plan execute](../cli/plan.md#emery-plan-execute) |
| `/emery:status` | Report where the plan stands and the literal next command (read-only) | [`status/SKILL.md`](../../../plugins/emery/skills/status/SKILL.md) | [plan status](../cli/plan.md#emery-plan-status) |
| `/emery:finalize` | Confirm publication is complete, then archive the plan (publication is operator-owned, outside Emery) | [`finalize/SKILL.md`](../../../plugins/emery/skills/finalize/SKILL.md) | [emery plan](../cli/plan.md) |

## How skills delegate

Each skill is an ultrathin invoke-and-relay wrapper over one `emery` verb — plan authoring, lifecycle transitions, spec merging, and plan archival run inside the CLI. Workspace slot materialization and repository publication are operator-owned outside Emery. See [AGENTS.md § Skill / CLI responsibility split](../../../AGENTS.md) for the contract.

Per-slice work (`/emery:refine`, `/emery:build`, `/emery:merge`, `/emery:drop`) lives in [Slice skills](../slice-skills/index.md).

## See also

- [Amend a plan before executing](../../how-to/amend-a-plan.md)
- [Bind multiple sources](../../how-to/bind-multiple-sources.md)
- [Quick reference card](../quick-reference.md)
- [Lifecycle](../lifecycle.md)
- [The layered stack](../../explanation/layered-stack.md)

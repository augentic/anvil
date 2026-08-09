# Skills

The five `/emery:*` skills drive the operator rhythm: one-time init, then plan → operator review → execute → finalize, with a read-only status probe available at any point.

## The change rhythm

<div class="pipeline">


![Default workflow poster](../../assets/diagrams/quick-reference/workflow-poster.svg)

<p class="pipeline-caption">plan → review → execute → finalize; the refine → build → merge loop runs inside execute.</p>
</div>


Inside `emery plan execute`, each slice runs through the fixed refine → build → merge phases. The phases are internal to the loop — when execute stops, the recovery path is to fix the reported problem and re-run `emery plan execute`, which resumes at the parked phase.

Canonical skill bodies live under [`plugins/emery/skills/`](../../../plugins/emery/README.md). Orchestration behind each skill lives in the `emery` verb the wrapper invokes.

## Skill summary

| Skill | Purpose | Canonical body | CLI |
| ----- | ------- | -------------- | --- |
| `/emery:init` | One-time project setup (`.emery/`, `project.yaml`, cache, `AGENTS.md`) | [`init/SKILL.md`](../../../plugins/emery/skills/init/SKILL.md) | [emery init](../cli/init.md) |
| `/emery:plan` | Survey sources, propose slices, exit at `pending` | [`plan/SKILL.md`](../../../plugins/emery/skills/plan/SKILL.md) | [emery plan](../cli/plan.md) |
| `/emery:execute` | Drive the plan through refine → build → merge (opens the authorization epoch) | [`execute/SKILL.md`](../../../plugins/emery/skills/execute/SKILL.md) | [plan execute](../cli/plan.md#emery-plan-execute) |
| `/emery:status` | Report where the plan stands and the literal next command (read-only) | [`status/SKILL.md`](../../../plugins/emery/skills/status/SKILL.md) | [plan status](../cli/plan.md#emery-plan-status) |
| `/emery:finalize` | Confirm publication is complete, then archive the plan (publication is operator-owned, outside Emery) | [`finalize/SKILL.md`](../../../plugins/emery/skills/finalize/SKILL.md) | [emery plan](../cli/plan.md) |

## How skills delegate

Each skill is an ultrathin invoke-and-relay wrapper over one `emery` verb — plan authoring, lifecycle transitions, spec merging, and plan archival run inside the CLI. Repository publication is operator-owned outside Emery. See [AGENTS.md § Skill / CLI responsibility split](../../../AGENTS.md) for the contract.

Slice curation between runs stays on the CLI: `emery plan drop` abandons a slice without merging, `emery plan amend` records divergence stamps, authority overrides, and the composition-replace merge authorization, and `emery slice {list, validate, provenance, model show}` are the read-only projections.

## See also

- [Amend a plan before executing](../../how-to/amend-a-plan.md)
- [Bind multiple sources](../../how-to/bind-multiple-sources.md)
- [Quick reference card](../quick-reference.md)
- [Lifecycle](../lifecycle.md)
- [The layered stack](../../explanation/layered-stack.md)

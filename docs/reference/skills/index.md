# Skills

The delivery `/emery:*` skills drive the operator rhythm: one-time init, then plan → operator review → refine → operator review → execute → finalize, with a read-only status probe available at any point. When an engagement starts from an estate, the RFC-104 `/emery:system-*` skills run upstream and produce the reviewed handoff `plan author --from --wave` consumes.

## The change rhythm

<div class="pipeline">


![Default workflow poster](../../assets/diagrams/quick-reference/workflow-poster.svg)

<p class="pipeline-caption">plan → review → refine → review → execute → finalize; refine writes every specification, execute runs build → merge.</p>
</div>


`emery plan refine` drains specification refinement over the closed plan and stops before any code work; `emery plan execute` then drives each slice through the fixed build → merge phases over the exact refinement manifests. When either stage stops, the recovery path is to fix the reported problem and re-run the same command — refine skips fresh manifests, execute resumes at the parked phase.

Canonical skill bodies live under [`plugins/emery/skills/`](../../../plugins/emery/README.md). Orchestration behind each skill lives in the `emery` verb the wrapper invokes.

## Skill summary

| Skill | Purpose | Canonical body | CLI |
| ----- | ------- | -------------- | --- |
| `/emery:init` | One-time project setup (`.emery/`, `project.yaml`, cache, `AGENTS.md`) | [`init/SKILL.md`](../../../plugins/emery/skills/init/SKILL.md) | [emery init](../cli/init.md) |
| `/emery:plan` | Bind a reviewed handoff (`--from` / `--wave`), import surface leads, decompose, exit at `pending` | [`plan/SKILL.md`](../../../plugins/emery/skills/plan/SKILL.md) | [emery plan](../cli/plan.md) |
| `/emery:correct` | Record a durable operator correction for a parked or authored decomposition domain | [`correct/SKILL.md`](../../../plugins/emery/skills/correct/SKILL.md) | [plan correct](../cli/plan.md#emery-plan-correct) |
| `/emery:refine` | Drain specification refinement over the closed plan (no code work) | [`refine/SKILL.md`](../../../plugins/emery/skills/refine/SKILL.md) | [plan refine](../cli/plan.md#emery-plan-refine) |
| `/emery:execute` | Drive the plan through build → merge (opens the authorization epoch over the refinement digests) | [`execute/SKILL.md`](../../../plugins/emery/skills/execute/SKILL.md) | [plan execute](../cli/plan.md#emery-plan-execute) |
| `/emery:status` | Report where the plan stands and the literal next command (read-only) | [`status/SKILL.md`](../../../plugins/emery/skills/status/SKILL.md) | [plan status](../cli/plan.md#emery-plan-status) |
| `/emery:finalize` | Confirm publication is complete, then archive the plan (publication is operator-owned, outside Emery) | [`finalize/SKILL.md`](../../../plugins/emery/skills/finalize/SKILL.md) | [emery plan](../cli/plan.md) |
| `/emery:system-survey` | Survey a definition home's declared coverage and correlate the `as-is` architecture | [`system-survey/SKILL.md`](../../../plugins/emery/skills/system-survey/SKILL.md) | `emery system survey` |
| `/emery:system-plan` | Propose the initial architecture once, then reproject views and wave handoffs | [`system-plan/SKILL.md`](../../../plugins/emery/skills/system-plan/SKILL.md) | `emery system plan` |
| `/emery:system-review` | Record architectural authority over one exact wave handoff | [`system-review/SKILL.md`](../../../plugins/emery/skills/system-review/SKILL.md) | `emery system review` |

## How skills delegate

Each skill is an ultrathin invoke-and-relay wrapper over one `emery` verb — plan authoring, lifecycle transitions, spec merging, and plan archival run inside the CLI. Repository publication is operator-owned outside Emery. See [AGENTS.md § Skill / CLI responsibility split](../../../AGENTS.md) for the contract.

Slice curation between runs stays on the CLI: `emery plan drop` abandons a slice without merging, `emery plan amend` records divergence stamps, authority overrides, and the composition-replace merge authorization, and `emery slice {list, validate, provenance, model show}` are the read-only projections.

## See also

- [Amend a plan before executing](../../how-to/amend-a-plan.md)
- [Bind multiple sources](../../how-to/bind-multiple-sources.md)
- [Quick reference card](../quick-reference.md)
- [Lifecycle](../lifecycle.md)
- [The layered stack](../../explanation/layered-stack.md)

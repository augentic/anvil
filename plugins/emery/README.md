# Emery Workflow

Emery routes workflow state changes through `/emery:*` wrappers over the `emery` CLI. `/emery:init` scaffolds a project. `/emery:plan` authors `change.md` + `plan.yaml` through the guest-routed `emery plan author`. `/emery:execute` confirms Gate 1 with the operator — running `emery plan execute` only on an explicit confirmation (its first run stamps `approved`) — and drives the drained per-slice loop through that one guest-routed verb. The per-slice breakouts — `/emery:refine`, `/emery:build`, `/emery:merge`, `/emery:drop` — invoke the matching `emery slice` verbs one slice at a time. Repository publication is operator-owned outside Emery; `/emery:finalize` archives only after publication is complete.

Every skill is an ultrathin invoke-and-relay wrapper: it elicits any missing arguments, invokes the corresponding `emery` command, and relays the output verbatim. Workflow sequencing, artifact synthesis, and validation all live in the CLI's guest orchestrations.

## Skills

| Skill | Command | Description |
|-------|---------|-------------|
| [init](skills/init/SKILL.md) | `/emery:init` | Initialize Emery in a project (`emery init`) |
| [plan](skills/plan/SKILL.md) | `/emery:plan` | Author `change.md` + `plan.yaml`, stop at Gate 1 (`emery plan author`) |
| [execute](skills/execute/SKILL.md) | `/emery:execute` | Confirm Gate 1, then drive the drained per-slice loop (`emery plan execute` — its first run stamps `approved`) |
| [refine](skills/refine/SKILL.md) | `/emery:refine` | Extract, synthesize, validate, and transition one slice to `refined` (`emery slice refine`) |
| [build](skills/build/SKILL.md) | `/emery:build` | Build one slice through its target adapter (`emery slice build`) |
| [merge](skills/merge/SKILL.md) | `/emery:merge` | Fold one slice's deltas into the baseline and archive it (`emery slice merge run`) |
| [drop](skills/drop/SKILL.md) | `/emery:drop` | Discard a slice without merging (`emery slice drop`) |
| [finalize](skills/finalize/SKILL.md) | `/emery:finalize` | Push branches, then archive the plan (PRs operator-owned, outside Emery) |

The judgment prose the workflow's model legs consume (lead reconciliation, the synthesis playbook, spec formatting) is embedded in the `emery` binary from `crates/slice/prompts/` and `crates/change/prompts/`; it is not plugin material.

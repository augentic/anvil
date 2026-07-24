# Specify Workflow

Specify routes workflow state changes through `/spec:*` wrappers over the `specify` CLI. `/spec:init` scaffolds a project. `/spec:plan` authors `change.md` + `plan.yaml` through the guest-routed `specify plan author`. `/spec:execute` confirms Gate 1 with the operator — stamping `specify plan transition <name> approved` only on an explicit confirmation — then drives the drained per-slice loop through the guest-routed `specify plan execute`. The per-slice breakouts — `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop` — invoke the matching `specify slice` verbs one slice at a time. Repository publication is operator-owned outside Specify; `/spec:finalize` archives only after publication is complete.

Every skill is an ultrathin invoke-and-relay wrapper: it elicits any missing arguments, invokes the corresponding `specify` command, and relays the output verbatim. Workflow sequencing, artifact synthesis, and validation all live in the CLI's guest orchestrations.

## Skills

| Skill | Command | Description |
|-------|---------|-------------|
| [init](skills/init/SKILL.md) | `/spec:init` | Initialize Specify in a project (`specify init`) |
| [plan](skills/plan/SKILL.md) | `/spec:plan` | Author `change.md` + `plan.yaml`, stop at Gate 1 (`specify plan author`) |
| [execute](skills/execute/SKILL.md) | `/spec:execute` | Confirm Gate 1, then drive the drained per-slice loop (`specify plan transition <name> approved` + `specify plan execute`) |
| [refine](skills/refine/SKILL.md) | `/spec:refine` | Extract, synthesize, validate, and transition one slice to `refined` (`specify slice refine`) |
| [build](skills/build/SKILL.md) | `/spec:build` | Build one slice through its target adapter (`specify slice build`) |
| [merge](skills/merge/SKILL.md) | `/spec:merge` | Fold one slice's deltas into the baseline and archive it (`specify slice merge run`) |
| [drop](skills/drop/SKILL.md) | `/spec:drop` | Discard a slice without merging (`specify slice drop`) |
| [finalize](skills/finalize/SKILL.md) | `/spec:finalize` | Push branches, then archive the plan (PRs operator-owned, outside Specify) |

The judgment prose the workflow's model legs consume (lead reconciliation, the synthesis playbook, spec formatting) is embedded in the `specify` binary from `crates/slice/prompts/` and `crates/change/prompts/`; it is not plugin material.

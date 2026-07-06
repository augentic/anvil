# Specify Workflow

Specify routes every operator action through `/spec:*` wrappers over the `specify` CLI. `/spec:init` scaffolds a project. `/spec:plan` authors `change.md` + `plan.yaml` through the guest-routed `specify plan author`. After the operator stamps Gate 1 (`specify plan transition <name> approved`), the guest-routed `specify plan execute` drives the drained per-slice loop. The per-slice breakouts — `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop` — invoke the matching `specify slice` verbs one slice at a time. `/spec:finalize` pushes branches and archives the plan; opening and merging pull requests is operator-owned and happens outside Specify.

Every skill is an ultrathin invoke-and-relay wrapper: it elicits any missing arguments, invokes the corresponding `specify` command, and relays the output verbatim. Workflow sequencing, artifact synthesis, and validation all live in the CLI's guest orchestrations.

## Skills

| Skill | Command | Description |
|-------|---------|-------------|
| [init](skills/init/SKILL.md) | `/spec:init` | Initialize Specify in a project (`specify init`) |
| [plan](skills/plan/SKILL.md) | `/spec:plan` | Author `change.md` + `plan.yaml`, stop at Gate 1 (`specify plan author`) |
| [refine](skills/refine/SKILL.md) | `/spec:refine` | Extract, synthesize, validate, and transition one slice to `refined` (`specify slice refine`) |
| [build](skills/build/SKILL.md) | `/spec:build` | Build one slice through its target adapter (`specify slice build`) |
| [merge](skills/merge/SKILL.md) | `/spec:merge` | Fold one slice's deltas into the baseline and archive it (`specify slice merge run`) |
| [drop](skills/drop/SKILL.md) | `/spec:drop` | Discard a slice without merging (`specify slice drop`) |
| [finalize](skills/finalize/SKILL.md) | `/spec:finalize` | Push branches, then archive the plan (PRs operator-owned, outside Specify) |

## References

- [Spec Format](references/spec-format.md)
- [Synthesis pipeline](references/synthesis/)

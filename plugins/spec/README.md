# Specify Workflow

Specify 2.0 routes every operator action through `/spec:*`. `/spec:init` scaffolds a project. `/spec:plan` authors `change.md` + `plan.yaml`. After the operator stamps Gate 1 (`specrun plan transition <name> approved`), `/spec:execute` drives the per-slice loop. The per-slice breakouts — `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop` — share the loop's skill bodies and are reached either through `execute` or by an operator inspecting a slice by hand. `/spec:finalize` pushes branches, observes PR state, and archives the plan once every PR is `MERGED`.

## Skills

| Skill | Command | Description |
|-------|---------|-------------|
| [init](skills/init/SKILL.md) | `/spec:init` | Initialize Specify in a project |
| [plan](skills/plan/SKILL.md) | `/spec:plan` | Author `change.md` + `plan.yaml`, stop at Gate 1 |
| [execute](skills/execute/SKILL.md) | `/spec:execute` | Drive `refine → build → merge` for each slice until the plan is drained |
| [refine](skills/refine/SKILL.md) | `/spec:refine` | Per slice: run `extract` per bound source, synthesize artifacts, validate, transition to `refined` |
| [build](skills/build/SKILL.md) | `/spec:build` | Validate artifacts and implement the slice's tasks |
| [merge](skills/merge/SKILL.md) | `/spec:merge` | Fold the slice's deltas into the baseline and archive it |
| [drop](skills/drop/SKILL.md) | `/spec:drop` | Discard a slice without merging |
| [finalize](skills/finalize/SKILL.md) | `/spec:finalize` | Push branches, observe PR state, archive the plan |

## References

- [Spec Format](references/spec-format.md)
- [Synthesis pipeline](references/synthesis/)

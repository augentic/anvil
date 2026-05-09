# Specify Workflow

Per-slice workflow orchestration for spec-driven development: define slices, build them, and merge or drop. Plan authoring and execution live in the [`change` plugin](../change/README.md); use `/change:plan` and `/change:execute` for change-level work.

## Skills

| Skill | Command | Description |
|-------|---------|-------------|
| [init](skills/init/SKILL.md) | `/spec:init` | Initialize Specify in a project |
| [define](skills/define/SKILL.md) | `/spec:define` | Create a slice and generate all artifacts |
| [build](skills/build/SKILL.md) | `/spec:build` | Validate artifacts and implement tasks from a slice |
| [merge](skills/merge/SKILL.md) | `/spec:merge` | Finalize and merge specs into baseline |
| [drop](skills/drop/SKILL.md) | `/spec:drop` | Discard a slice without merging |
| [extract](skills/extract/SKILL.md) | `/spec:extract` | Extract Specify artifacts from existing source code |
| [analyze](skills/analyze/SKILL.md) | `/spec:analyze` | Plan-time capability inference; emits capability summaries into `discovery.md` |

## References

- [Capability Resolution](references/capability-resolution.md)
- [Spec Format](references/spec-format.md)

## See also

- [`change` plugin](../change/README.md) — canonical home of `/change:plan` and `/change:execute`.

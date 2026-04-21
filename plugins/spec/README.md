# Specify Workflow

Core workflow orchestration for spec-driven development: define changes, build them, and merge or drop.

## Skills

| Skill | Command | Description |
|-------|---------|-------------|
| [init](skills/init/SKILL.md) | `/spec:init` | Initialize Specify in a project |
| [define](skills/define/SKILL.md) | `/spec:define` | Create a change and generate all artifacts |
| [build](skills/build/SKILL.md) | `/spec:build` | Validate artifacts and implement tasks from a change |
| [merge](skills/merge/SKILL.md) | `/spec:merge` | Finalize and merge specs into baseline |
| [drop](skills/drop/SKILL.md) | `/spec:drop` | Discard a change without merging |
| [verify](skills/verify/SKILL.md) | `/spec:verify` | Detect drift between code and baseline specs |
| [explore](skills/explore/SKILL.md) | `/spec:explore` | Thinking partner for ideas and requirements |
| [status](skills/status/SKILL.md) | `/spec:status` | Check artifact completion and task progress |
| [extract](skills/extract/SKILL.md) | `/spec:extract` | Extract Specify artifacts from existing source code |
| [analyze](skills/analyze/SKILL.md) | `/spec:analyze` | Plan-time capability inference; emits capability summaries into `discovery.md` |

## References

- [Schema Resolution](references/schema-resolution.md)
- [Spec Format](references/spec-format.md)
- [Entry Point Patterns](references/entry-point-patterns.md)

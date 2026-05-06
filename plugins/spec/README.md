# Specify Workflow

Per-slice workflow orchestration for spec-driven development: define changes, build them, and merge or drop. Plan authoring and execution moved to the [`change` plugin](../change/README.md) in RFC-13 §3.9; the `spec` plugin retains thin deprecation shims at `/change:plan` and `/change:execute` while the rename lands.

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
| [plan (deprecated shim)](skills/plan/SKILL.md) | `/change:plan` | Deprecated — delegates to [`/change:plan`](../change/skills/plan/SKILL.md). Removed before the post-RFC-13 release. |
| [execute (deprecated shim)](skills/execute/SKILL.md) | `/change:execute` | Deprecated — delegates to [`/change:execute`](../change/skills/execute/SKILL.md). Removed before the post-RFC-13 release. |

## References

- [Capability Resolution](references/capability-resolution.md)
- [Spec Format](references/spec-format.md)

## See also

- [`change` plugin](../change/README.md) — canonical home of `/change:plan` and `/change:execute` after RFC-13 §3.9.
- [RFC-13 §Migration](../../rfcs/rfc-13-extensibility.md#migration) — the cut-over plan and timeline for the deprecation shims.

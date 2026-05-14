# Change

Cross-repo change orchestration: author the executable plan that flows through the per-loop unit, drive the loop end-to-end, and surface umbrella status. The skills here sit above the per-loop phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`) and below the platform-level CLI verbs (`specify change *`, `specify registry *`, `specify workspace *`).

The `change` plugin owns RFC-13's umbrella orchestration noun. Per-loop phase skills stay on the `spec` plugin; this plugin owns the multi-slice coordination that flows through `change.md` + `plan.yaml`.

## Skills

| Skill | Command | Description |
|-------|---------|-------------|
| [plan](skills/plan/SKILL.md) | `/change:plan` | Author `plan.yaml` for a change via the planning brief pipeline; with `orchestrate`, drive the cross-repo umbrella through PR creation, operator merge handoff, and finalize verification. |
| [execute](skills/execute/SKILL.md) | `/change:execute` | Drive the authored plan through the per-loop phases (`/spec:define → /spec:build → /spec:merge`); supports supervised, dry-run, and `loop` modes. |
| [analyze](skills/analyze/SKILL.md) | `/change:analyze` | Plan-time capability inference; emits capability summaries into `discovery.md` from legacy code or documentation inputs (invoked internally by `/change:plan`). |

## See also

- [`specify change`](../../docs/reference/cli/change.md) — the CLI surface that owns `plan.yaml` lifecycle, lock acquisition, and umbrella finalization.
- [`specify registry`](../../docs/reference/cli/registry.md) — the registry topology component the change skills consume but never mutate directly.
- [`specify workspace`](../../docs/reference/cli/workspace.md) — workspace materialisation and PR push for multi-repo runs. PR merging is operator-owned; `workspace merge` has been removed.
- [Change skills overview](../../docs/reference/change-skills/index.md) — the layered stack and the change-vs-spec ownership split.

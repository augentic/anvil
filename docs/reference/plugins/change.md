# Change

The Change plugin owns the umbrella orchestration noun. It carries the skills that author and drive a change's `plan.yaml`: `/change:plan` (multi-slice plan authoring plus the cross-repo umbrella under `orchestrate`), `/change:execute` (slice driver), and `/change:analyze` (plan-time capability inference invoked by the planning brief pipeline). The per-loop phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`) stay on the [Specify plugin](../slice-skills/index.md), where they belong.

The plugin separation reflects the dependency direction: *capabilities* own outcome artefacts and their mechanics; *platform components* (`specify change`, `specify registry`, `specify workspace`) coordinate where and when those per-project slices run. The `change` plugin's skills are the agent-side counterpart to the `specify change *` and `specify workspace *` CLI surfaces.

## Skills

| Skill | Purpose |
|-------|---------|
| [`/change:plan`](../change-skills/plan.md) | Author `plan.yaml` from `from` / `against` / `source` inputs; under `orchestrate`, drive the cross-repo umbrella end to end. |
| [`/change:execute`](../change-skills/execute.md) | Drive the authored plan through the per-slice define-build-merge phases; supports supervised, `dry-run`, and `loop` modes with self-heal. |
| [`/change:analyze`](../change-skills/analyze.md) | Plan-time capability inference; emits capability summaries into `discovery.md` from legacy code or documentation inputs (invoked internally by `/change:plan`). |

## CLI counterpart

The matching CLI surface lives under [`specify change`](../cli/change.md). The umbrella verbs (`create`, `show`, `finalize`) and the nested plan family (`specify change plan {add, amend, next, status, doctor, lock, transition, validate, archive}`) are the current operator-facing commands.

## See also

- [Change Skills overview](../change-skills/index.md) — layered stack and the change-vs-slice split.
- [`/change:plan`](../change-skills/plan.md) — authoring skill reference.
- [`/change:execute`](../change-skills/execute.md) — driver skill reference.
- [`/change:analyze`](../change-skills/analyze.md) — plan-time capability inference reference.
- [`/change:plan <name> orchestrate`](../change-skills/change.md) — cross-repo umbrella mode reference.
- [`specify change`](../cli/change.md) — CLI surface that the change skills shell out through.

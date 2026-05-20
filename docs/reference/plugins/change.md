# Change

The Change plugin coordinates a change's `plan.yaml` across three peer skills with an explicit operator review seam between authoring and execution. It carries the skills that author the plan (`/change:draft`), drive it through the per-slice loop (`/change:execute`), close it out after PRs merge (`/change:finalize`), and infer plan-time adapter summaries from inputs (`/change:analyze`). The per-loop phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`) stay on the [Specify plugin](../slice-skills/index.md), where they belong.

The plugin separation reflects the dependency direction: *adapters* own outcome artefacts and their mechanics; *platform components* (`specify change`, `specify registry`, `specify workspace`) coordinate where and when those per-project slices run. The `change` plugin's skills are the agent-side counterpart to the `specify change *` and `specify workspace *` CLI surfaces.

## Skills

| Skill | Purpose |
|-------|---------|
| [`/change:draft`](../change-skills/draft.md) | Author `plan.yaml` from `from` / `against` / `source` inputs via the planning brief pipeline; stop at the operator review seam. |
| [`/change:execute`](../change-skills/execute.md) | Drive the authored plan through the per-slice define-build-merge phases; supports supervised, `dry-run`, and `loop` modes with self-heal. |
| [`/change:finalize`](../change-skills/finalize.md) | Push branches via `specify workspace push`, observe PR state via `gh pr list`, then run `specify change finalize` once every PR is `MERGED`. |
| [`/change:analyze`](../change-skills/analyze.md) | Plan-time adapter inference; emits adapter summaries into `discovery.md` from legacy code or documentation inputs (invoked internally by `/change:draft`). |

## CLI counterpart

The matching CLI surface lives under [`specify change`](../cli/change.md). The change verbs (`draft`, `show`, `finalize`) and the nested plan family (`specify plan {add, amend, next, status, doctor, lock, transition, validate, archive}`) are the current operator-facing commands.

## See also

- [Change Skills overview](../change-skills/index.md) — three-skill lifecycle and the change-vs-slice split.
- [`/change:draft`](../change-skills/draft.md) — authoring skill reference.
- [`/change:execute`](../change-skills/execute.md) — driver skill reference.
- [`/change:finalize`](../change-skills/finalize.md) — close-out skill reference.
- [`/change:analyze`](../change-skills/analyze.md) — plan-time adapter inference reference.
- [`specify change`](../cli/change.md) — CLI surface that the change skills shell out through.

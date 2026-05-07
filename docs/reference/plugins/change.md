# Change

The Change plugin owns RFC-13's umbrella orchestration noun. It carries the two skills that author and drive a change's `plan.yaml`: `/change:plan` (Layer 3 authoring + Layer 4 cross-repo umbrella under `--orchestrate`) and `/change:execute` (Layer 2 driver). The per-loop phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`) stay on the [Specify plugin](../slice-skills/index.md), where they belong.

The plugin separation reflects the RFC-13 dependency direction: *capabilities* own outcome artefacts and their mechanics; *platform components* (`specify change`, `specify registry`, `specify workspace`) coordinate where and when those per-project slices run. The `change` plugin's skills are the agent-side counterpart to the `specify change *` and `specify workspace *` CLI surfaces.

## Skills

| Skill | Layer | Purpose |
|-------|-------|---------|
| [`/change:plan`](../change-skills/plan.md) | 3 (authoring) and 4 (`--orchestrate`) | Author `plan.yaml` from `--from` / `--against` / `--source` inputs; under `--orchestrate`, drive the cross-repo umbrella end to end. |
| [`/change:execute`](../change-skills/execute.md) | 2 (driver) | Drive the authored plan through the per-slice define-build-merge phases; supports supervised, `--dry-run`, and `--loop` modes with self-heal. |

## Migration from the `spec` plugin

These skills used to live on the `spec` plugin as `/change:plan` and `/change:execute`. RFC-13 §3.9 moved them to the new `change` plugin so umbrella orchestration is owned separately from the per-slice phases. The `spec` plugin retains thin deprecation shims at the historical paths; both shims warn and delegate to the canonical commands and are removed before the post-RFC release per [RFC-13 §Migration](../../../rfcs/archive/rfc-13-extensibility.md#migration).

| Pre-3.9 command | Post-3.9 canonical command |
|-----------------|----------------------------|
| `/change:plan <name>` | `/change:plan <name>` |
| `/change:plan --orchestrate <name>` | `/change:plan --orchestrate <name>` |
| `/change:execute --loop` | `/change:execute --loop` |

## CLI counterpart

The matching CLI surface lives under [`specify change`](../cli/change.md). The umbrella verbs (`create`, `show`, `finalize`) and the nested plan family (`specify change plan {add, amend, next, status, doctor, lock, transition, validate, archive}`) replace the v1.x `specify change *` and `specify plan *` groups, which were retired in RFC-13 §3.5.

## See also

- [Change Skills overview](../change-skills/index.md) — layered stack and the change-vs-slice split.
- [`/change:plan`](../change-skills/plan.md) — authoring skill reference.
- [`/change:execute`](../change-skills/execute.md) — driver skill reference.
- [`/change:plan --orchestrate`](../change-skills/change.md) — Layer 4 umbrella mode reference.
- [`specify change`](../cli/change.md) — Layer 1 CLI surface that the change skills shell out through.
- [Migrating CLI v1](../../explanation/migrating-cli-v1.md) — verb rename map covering the v1.x → post-RFC-13 transition.

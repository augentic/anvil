# Plan single-writer contract

`plan.yaml` is a CLI-owned coordination artifact. Skills and humans may request changes through `specify change plan *` verbs, but they never hand-edit the file.

## Writer boundaries

| Write | Owner | Command |
|---|---|---|
| Create the plan shell and top-level `sources` map (alongside the `change.md` brief) | `/change:plan` or a human authoring by hand | `specify change create <change-name> [--source <key>=<path-or-url> ...]` |
| Add plan entries | `/change:plan`, propose briefs, phase skills that discover neighbouring work, or humans | `specify change plan add <name> ...` |
| Amend non-status fields | Assignment step, phase skills, or humans | `specify change plan amend <name> ...` |
| Change entry status | `/change:execute` or operators driving the loop manually | `specify change plan transition <name> <status> [--reason "..."]` |
| Validate, inspect, or archive the plan | Any operator flow | `specify change plan validate/status/doctor/archive` |

The `amend` surface intentionally has no `status` field. Entry status changes are always transitions.

## Authoring rules

- `/change:plan` writes entries one at a time through `specify change plan add`; it never batches YAML or rewrites existing entries in place.
- Propose briefs are the single-writer edge for accepted draft slices. They create entries without `--project`; project assignment runs later through `specify change plan amend --project <project>`.
- `extend` mode is append-only. Existing entries are not modified except for the explicit assignment step on entries created in the same run.
- `dry-run` is read-only: no plan create/add/amend/transition calls and no files under `.specify/` are written.

## Execution rules

- `/change:execute` writes plan status only through `specify change plan transition`.
- The driver transitions `pending -> in-progress` before phase writes begin, then leaves `in-progress` through exactly one terminal transition: `done`, `failed`, or `blocked`.
- Phase skills may add neighbouring entries or amend non-status fields when a run discovers structural work, but they never transition plan status.
- Direct edits to `plan.yaml` are outside the contract except for narrowly documented manual recovery paths. Prefer CLI verbs so validation, audit behavior, and future hosted execution share one implementation.

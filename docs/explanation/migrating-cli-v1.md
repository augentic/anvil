# Migrating to CLI v1

The `specify` CLI was reshaped in the v1 release so per-change operations live under the `change` noun group, the registry is a top-level verb (it is a *platform* artifact, not an initiative one), and `initiative` is a flat noun group covering the operator brief alone. This page is the operator-facing rename map. CI scripts, custom skills, and personal aliases that pinned to pre-v1 verb names need to be retargeted.

The behavioural surface did not change -- every renamed command does exactly the same thing it did before. The reshape is a routing change, not a semantic one.

## Rename map

| Old verb | New verb |
| --- | --- |
| `specify validate <dir>` | `specify change validate <name>` |
| `specify merge <dir>` | `specify change merge run <name>` |
| `specify spec preview <dir>` | `specify change merge preview <name>` |
| `specify spec conflict-check <dir>` | `specify change merge conflict-check <name>` |
| `specify task progress <dir>` | `specify change task progress <name>` |
| `specify task mark <dir> <task>` | `specify change task mark <name> <task>` |
| `specify status [name]` (overloaded) | dashboard: `specify status` (no args); single change: `specify change status <name>` |
| `specify change phase-outcome <name> <phase> <outcome> ...` | `specify change outcome set <name> <phase> <outcome> ...` |
| `specify change outcome <name>` (read) | `specify change outcome show <name>` |
| `specify change journal-append <name> <phase> <kind> ...` | `specify change journal append <name> <phase> <kind> ...` |
| (new verb) | `specify change journal show <name>` -- read `journal.yaml` |
| `specify initiative brief init <name>` | `specify initiative init <name>` |
| `specify initiative brief show` | `specify initiative show` |
| `specify initiative registry show` | `specify registry show` |
| `specify initiative registry validate` | `specify registry validate` |

Every renamed verb takes the change `<name>` (kebab-case) directly. The on-disk `<change-dir>` is resolved internally by the CLI; operators no longer need to type `.specify/changes/<name>/`.

## Why these renames

- **`registry.yaml` is platform-scoped, not initiative-scoped.** A platform's repository catalogue spans every initiative the platform ever runs. Nesting the verbs under `specify initiative ...` implied the wrong lifetime. They now live at the top level: `specify registry {show, validate}`.
- **Per-change operations belong with `change`.** The old `specify validate`, `specify merge`, `specify spec`, and `specify task` groups all took a change directory and operated on a single change. Folding them into `specify change` makes the noun structure of the CLI match the noun structure of `.specify/changes/`.
- **`outcome` and `journal` group their verbs cleanly.** `phase-outcome` was a hyphenated verb pretending to be a sub-noun; `journal-append` was an action attached to a missing noun group. Both now sit under `outcome {set, show}` and `journal {append, show}`, matching how every other group works.
- **`status` is a project dashboard, not an alias of `change status`.** The bare `specify status` now returns `{registry, plan, changes}` -- the operator's overall view of the project. The single-change view moved to `specify change status <name>`.
- **`<change-dir>` becomes `<name>`.** Every per-change verb now takes the kebab-case name. The CLI resolves `.specify/changes/<name>/` (or the most recent matching archive) internally. CI scripts that joined paths together can drop that.

## Mechanical migration

For CI scripts and operator aliases, the following find/replace patterns cover the bulk of the work. Run them in order; the early replacements rely on the old shapes.

```sh
# Phase outcome / journal verbs (must run before any `specify change ` rewrites)
sed -i '' -E 's@specify change phase-outcome@specify change outcome set@g'           file.sh
sed -i '' -E 's@specify change journal-append@specify change journal append@g'       file.sh

# Per-change verbs that take <change-dir>: rewrite to <name>
sed -i '' -E 's@specify validate (\.specify/changes/)?([a-z0-9][a-z0-9-]*)/?@specify change validate \2@g'                file.sh
sed -i '' -E 's@specify merge (\.specify/changes/)?([a-z0-9][a-z0-9-]*)/?@specify change merge run \2@g'                  file.sh
sed -i '' -E 's@specify spec preview (\.specify/changes/)?([a-z0-9][a-z0-9-]*)/?@specify change merge preview \2@g'       file.sh
sed -i '' -E 's@specify spec conflict-check (\.specify/changes/)?([a-z0-9][a-z0-9-]*)/?@specify change merge conflict-check \2@g' file.sh
sed -i '' -E 's@specify task progress (\.specify/changes/)?([a-z0-9][a-z0-9-]*)/?@specify change task progress \2@g'      file.sh
sed -i '' -E 's@specify task mark (\.specify/changes/)?([a-z0-9][a-z0-9-]*)/?@specify change task mark \2@g'              file.sh

# Registry: drop the `initiative` prefix
sed -i '' -E 's@specify initiative registry @specify registry @g'                    file.sh

# Initiative brief: collapse the inner noun
sed -i '' -E 's@specify initiative brief init@specify initiative init@g'             file.sh
sed -i '' -E 's@specify initiative brief show@specify initiative show@g'             file.sh
```

After running the bulk pass:

- Skim any remaining `specify change outcome <name>` calls (with no `set`/`show` after `outcome`). Reads become `specify change outcome show <name>`. Writes become `specify change outcome set <name> <phase> <outcome> ...`. The presence of trailing positional `<phase> <outcome>` arguments distinguishes them.
- Replace any bare `specify status <name>` with `specify change status <name>`. The bare `specify status` (no positional argument) is now the project dashboard -- a different command shape.

## What did not change

These surfaces are untouched. Scripts that use them keep working.

- `specify init ...` -- project scaffold.
- `specify schema {resolve, check, pipeline}` -- schema and brief pipeline queries.
- `specify plan {init, validate, next, status, create, amend, transition, archive, lock {acquire, release, status}}` -- initiative plan CRUD and lifecycle.
- `specify workspace {sync, status, push}` -- multi-repo workspace clones.
- `specify vectis {init, verify, add-shell, update-versions}` -- Vectis project tooling.
- `specify change {create, list, status, transition, touched-specs, overlap, archive, drop}` -- the existing CRUD verbs on `change`. The rename added new verbs alongside; it did not displace these.

## See also

- [specify change](../reference/cli/change.md) -- per-change CRUD, validate, merge, task, outcome, journal.
- [specify initiative](../reference/cli/initiative.md) -- operator brief at `.specify/initiative.md`.
- [specify registry](../reference/cli/registry.md) -- platform registry at `.specify/registry.yaml`.
- [specify status](../reference/cli/status.md) -- project dashboard.
- [CLI Reference index](../reference/cli/index.md) -- top-level command catalogue.

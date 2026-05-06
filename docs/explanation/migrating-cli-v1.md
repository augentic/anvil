# Migrating to CLI v1

The `specify` CLI was reshaped in the v1 release so per-slice operations live under the `change` noun group, the registry is a top-level verb (it is a *platform* artifact, not an initiative one), and `initiative` is a flat noun group covering the operator brief alone. This page is the operator-facing rename map. CI scripts, custom skills, and personal aliases that pinned to pre-v1 verb names need to be retargeted.

The behavioural surface did not change -- every renamed command does exactly the same thing it did before. The reshape is a routing change, not a semantic one.

> **For the additive surface** -- new verbs (`specify registry add`, `specify workspace merge`, `specify change plan doctor`, `specify change finalize`), new flags (`specify init --hub`), the `/change:plan --orchestrate` Layer 4 umbrella mode (formerly the `/spec:initiative` skill), and contracts -- see [What's New Since v0.23](whats-new.md). The two pages compose: this one is **what was renamed**, the other is **what was added**.

## Rename map

| Old verb | New verb |
| --- | --- |
| `specify validate <dir>` | `specify slice validate <name>` |
| `specify merge <dir>` | `specify slice merge run <name>` |
| `specify spec preview <dir>` | `specify slice merge preview <name>` |
| `specify spec conflict-check <dir>` | `specify slice merge conflict-check <name>` |
| `specify task progress <dir>` | `specify slice task progress <name>` |
| `specify task mark <dir> <task>` | `specify slice task mark <name> <task>` |
| `specify status [name]` (overloaded) | dashboard: `specify status` (no args); single change: `specify slice status <name>` |
| `specify change phase-outcome <name> <phase> <outcome> ...` | `specify slice outcome set <name> <phase> <outcome> ...` |
| `specify slice outcome <name>` (read) | `specify slice outcome show <name>` |
| `specify slice journal-append <name> <phase> <kind> ...` | `specify slice journal append <name> <phase> <kind> ...` |
| (new verb) | `specify slice journal show <name>` -- read `journal.yaml` |
| `specify change brief init <name>` | `specify change init <name>` |
| `specify change brief show` | `specify change show` |
| `specify change registry show` | `specify registry show` |
| `specify change registry validate` | `specify registry validate` |

Every renamed verb takes the change `<name>` (kebab-case) directly. The on-disk `<slice-dir>` is resolved internally by the CLI; operators no longer need to type `.specify/changes/<name>/`.

## v1.x renames

The renames below landed after the v1 cleanup, as part of the RFC-9 §1F+§1G consistency pass on the noun-create verbs. Behaviourally they are identical to their predecessors -- only the verb spellings changed -- and they ship together so the `plan` group never spent an interim release with `init` and `create` for the same noun. Operators on v1 cleanup-era CI/aliases need to apply this second hop on top of the v1 row map above.

| Old verb (v1) | New verb (v1.x) |
| --- | --- |
| `specify change init <name>` | `specify change create <name>` |
| `specify change plan init <name>` | `specify change plan create <name>` |
| `specify change plan create <name>` | `specify change plan add <name>` |

Order matters when running these as bulk replacements: do `plan create` -> `plan add` (the entry-append verb) **before** `plan init` -> `plan create` (the file scaffold), or the second pass will rewrite freshly renamed entries.

## Why these renames

- **`registry.yaml` is platform-scoped, not initiative-scoped.** A platform's repository catalogue spans every initiative the platform ever runs. Nesting the verbs under `specify change ...` implied the wrong lifetime. They now live at the top level: `specify registry {show, validate}` (plus `{add, remove}` added later by RFC-9 §2A).
- **Per-change operations belong with `change`.** The old `specify validate`, `specify merge`, `specify spec`, and `specify task` groups all took a change directory and operated on a single change. Folding them into `specify change` makes the noun structure of the CLI match the noun structure of `.specify/changes/`.
- **`outcome` and `journal` group their verbs cleanly.** `phase-outcome` was a hyphenated verb pretending to be a sub-noun; `journal-append` was an action attached to a missing noun group. Both now sit under `outcome {set, show}` and `journal {append, show}`, matching how every other group works.
- **`status` is a project dashboard, not an alias of `change status`.** The bare `specify status` now returns `{registry, plan, changes}` -- the operator's overall view of the project. The single-change view moved to `specify slice status <name>`.
- **`<slice-dir>` becomes `<name>`.** Every per-slice verb now takes the kebab-case name. The CLI resolves `.specify/changes/<name>/` (or the most recent matching archive) internally. CI scripts that joined paths together can drop that.

## Mechanical migration

For CI scripts and operator aliases, the following find/replace patterns cover the bulk of the work. Run them in order; the early replacements rely on the old shapes.

```sh
# Phase outcome / journal verbs (must run before any `specify change ` rewrites)
sed -i '' -E 's@specify change phase-outcome@specify slice outcome set@g'           file.sh
sed -i '' -E 's@specify slice journal-append@specify slice journal append@g'       file.sh

# Per-change verbs that take <slice-dir>: rewrite to <name>
sed -i '' -E 's@specify validate (\.specify/changes/)?([a-z0-9][a-z0-9-]*)/?@specify slice validate \2@g'                file.sh
sed -i '' -E 's@specify merge (\.specify/changes/)?([a-z0-9][a-z0-9-]*)/?@specify slice merge run \2@g'                  file.sh
sed -i '' -E 's@specify spec preview (\.specify/changes/)?([a-z0-9][a-z0-9-]*)/?@specify slice merge preview \2@g'       file.sh
sed -i '' -E 's@specify spec conflict-check (\.specify/changes/)?([a-z0-9][a-z0-9-]*)/?@specify slice merge conflict-check \2@g' file.sh
sed -i '' -E 's@specify task progress (\.specify/changes/)?([a-z0-9][a-z0-9-]*)/?@specify slice task progress \2@g'      file.sh
sed -i '' -E 's@specify task mark (\.specify/changes/)?([a-z0-9][a-z0-9-]*)/?@specify slice task mark \2@g'              file.sh

# Registry: drop the `initiative` prefix
sed -i '' -E 's@specify change registry @specify registry @g'                    file.sh

# Initiative brief: collapse the inner noun
sed -i '' -E 's@specify change brief init@specify change init@g'             file.sh
sed -i '' -E 's@specify change brief show@specify change show@g'             file.sh
```

After running the bulk pass:

- Skim any remaining `specify slice outcome <name>` calls (with no `set`/`show` after `outcome`). Reads become `specify slice outcome show <name>`. Writes become `specify slice outcome set <name> <phase> <outcome> ...`. The presence of trailing positional `<phase> <outcome>` arguments distinguishes them.
- Replace any bare `specify status <name>` with `specify slice status <name>`. The bare `specify status` (no positional argument) is now the project dashboard -- a different command shape.

## What did not change

These surfaces are untouched. Scripts that use them keep working.

- `specify init ...` -- project scaffold (extended with `--hub` in RFC-9 §1D, additive only; positional argument shape was renamed to `<capability>` by RFC-13 §Migration).
- `specify capability {resolve, check, pipeline}` -- capability and brief pipeline queries (renamed from `specify schema {resolve, check, pipeline}` by RFC-13 §Migration).
- `specify change plan {create, validate, doctor, next, status, add, amend, transition, archive, lock {acquire, release, status}}` -- change plan CRUD and lifecycle (folded under `specify change` by RFC-13 §3.5). The v1.x rename rows above renamed `init` -> `create` (file scaffold) and the entry-append `create` -> `add`; `doctor` is a strict superset of `validate` added by RFC-9 §4B.
- `specify workspace {sync, status, push, merge}` -- multi-repo workspace clones; `merge` was added by RFC-9 §4A.
- `specify change {create, show, finalize}` -- operator brief at `change.md`; `finalize` was added by RFC-9 §4C, and `init` was renamed to `create` in v1.x. The umbrella verbs were renamed from `specify initiative *` to `specify change *` by RFC-13 §3.5.
- `specify registry {add, remove, show, validate}` -- platform registry at `registry.yaml`; `add` and `remove` were added by RFC-9 §2A.
- `specify vectis {init, verify, add-shell, update-versions}` -- Vectis project tooling.
- `specify slice {create, list, status, transition, touched-specs, overlap, archive, drop}` -- the per-slice CRUD verbs (renamed from the v1.x `specify change *` group by RFC-13 §3.2). The rename added new verbs alongside; it did not displace these.

## See also

- [specify slice](../reference/cli/slice.md) -- per-slice CRUD, validate, merge, task, outcome, journal (renamed from the v1.x `specify change *` group by RFC-13 §3.2).
- [specify change](../reference/cli/change.md) -- operator brief at `change.md` plus `finalize` (renamed from the v1.x `specify initiative *` group by RFC-13 §3.5).
- [specify change plan](../reference/cli/plan.md) -- plan CRUD and lifecycle (folded under `specify change` from the v1.x `specify plan *` group by RFC-13 §3.5).
- [specify registry](../reference/cli/registry.md) -- platform registry at `registry.yaml`.
- [specify status](../reference/cli/status.md) -- project dashboard.
- [CLI Reference index](../reference/cli/index.md) -- top-level command catalogue.

# emery registry

Manage the platform registry at `registry.yaml` -- the catalogue of repositories that make up a multi-repo platform. Optional for single-repo projects.

The registry was promoted from `emery registry ...` to a top-level noun group in the CLI cleanup: `registry.yaml` is platform-scoped (it spans every change the platform runs), not change-scoped, so the verb shape now reflects that.

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`add`](#emery-registry-add) | Append a new project entry; creates `registry.yaml` with `version: 1` if absent. Validates kebab-case name and URL classification. `--adapter` is optional (a greenfield scaffold seed only). |
| [`remove`](#emery-registry-remove) | Delete a project entry. Warns when `plan.yaml` references the removed project. |
| [`validate`](#emery-registry-validate) | Structural and referential check; on workspaces runs the `workspace-cannot-be-project` invariant. |

## Subcommands

### emery registry validate

Check structural and referential invariants of `registry.yaml`.

```bash
emery registry validate
```

Validates:

- Registry shape (required fields, kebab-case names, well-formed `url:` values).
- When `contracts` blocks are present on entries, the producer / consumer / imports invariants are coherent.

A project's adapter and description for plan-time topology live in its own `project.yaml` and are checked against `.emery/topology.lock` by `emery plan validate` (`topology-cache-stale`); the registry carries membership and location only.

Used by `/emery:plan` after populating contract roles, and by operators who edit `registry.yaml` by hand.

### emery registry add

Append a new project entry to `registry.yaml`. Creates the file with `version: 1` when absent.

```bash
emery registry add <name> --url <url> [--adapter <adapter>] [--description "..."]
```

Behaviour:

- Validates `name` (kebab-case) and `--url` (same shape rules `registry validate` enforces — `.`, repo-relative path, `git@host:path`, `http(s)://`, `ssh://`, or `git+http(s)://` / `git+ssh://`).
- `--adapter` is optional and, when present, is recorded only as a greenfield scaffold seed; a project's authoritative target adapter lives in its own `project.yaml`.
- Refuses to add a project that already exists.
- Runs `Registry::validate_shape` after the write.
- Workspace repos (`project.yaml: workspace: true`) layer on the `workspace-cannot-be-project` invariant: an entry with `url: .` is rejected.

Used by `/emery:plan`'s registry-proposal sub-step and when staging a new peer ahead of `emery plan amend --project <new>`. The validation-ordering invariant is: `emery registry add` before `emery plan {add, amend} --project <name>`, since the plan verbs reject unknown projects.

### emery registry remove

Delete a project entry from `registry.yaml`.

```bash
emery registry remove <name>
```

Behaviour:

- Refuses when the registry is absent or `<name>` is not declared.
- Validates the resulting shape after the write.
- Surfaces a non-fatal warning (on stderr in text mode, in the JSON `warnings` array) when `plan.yaml` exists and any plan entry references the removed project — naming each affected entry so the operator can rewire them via `emery plan amend <change> --project <other>` separately.

## See also

- [emery plan](plan.md) -- the umbrella verbs for the operator brief at `change.md` and `plan.yaml`.
- [emery workspace](workspace.md) -- materialise registry projects under top-level `workspace/<peer>/`.
- [Configuration Files → registry.yaml](../configuration.md#registryyaml) -- file format reference.

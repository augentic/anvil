# specify registry

Manage the platform registry at `.specify/registry.yaml` -- the catalogue of repositories that make up a multi-repo platform. Optional for single-repo projects.

The registry was promoted from `specify initiative registry ...` to a top-level noun group in the CLI cleanup: `registry.yaml` is platform-scoped (it spans every initiative the platform runs), not initiative-scoped, so the verb shape now reflects that.

## Subcommands

### specify registry show

Render the registry content.

```bash
specify registry show [--format json]
```

`--format json` is the canonical shape consumed by `/spec:plan`'s sync-peers step (`projects.length > 1` triggers multi-repo authoring).

### specify registry validate

Check structural and referential invariants of `.specify/registry.yaml`.

```bash
specify registry validate
```

Validates:

- Schema-level shape (required fields, kebab-case names, well-formed `url:` values).
- `description` is required when more than one project is declared (multi-repo invariant).
- Per-project `schema:` URLs are resolvable.
- When `contracts` blocks are present on entries, the producer / consumer / imports invariants are coherent.

Used by `/spec:plan` after populating contract roles, and by operators who edit `registry.yaml` by hand.

### specify registry add

Append a new project entry to `.specify/registry.yaml`. Creates the file with `version: 1` when absent. (RFC-9 §2A.)

```bash
specify registry add <name> --url <url> --schema <schema> [--description "..."]
```

Behaviour:

- Validates `name` (kebab-case), `--url` (same shape rules `registry validate` enforces — `.`, repo-relative path, `git@host:path`, `http(s)://`, `ssh://`, or `git+http(s)://` / `git+ssh://`), and `--schema` (non-empty after trim).
- Refuses to add a project that already exists.
- Runs `Registry::validate_shape` after the write — including the `description-missing-multi-repo` invariant: if the addition produces a multi-project registry and any existing entry lacks a `description`, the verb fails with a diagnostic naming the offending entry.
- Hub repos (`project.yaml: hub: true`) layer on the `hub-cannot-be-project` invariant: an entry with `url: .` is rejected.

Used by `/spec:plan`'s registry-proposal sub-step (RFC-9 §2B) and by operators staging a new peer ahead of `specify plan amend --project <new>`. The validation-ordering invariant is: `specify registry add` before `specify plan {create, amend} --project <name>`, since the plan verbs reject unknown projects.

### specify registry remove

Delete a project entry from `.specify/registry.yaml`. (RFC-9 §2A.)

```bash
specify registry remove <name>
```

Behaviour:

- Refuses when the registry is absent or `<name>` is not declared.
- Validates the resulting shape after the write.
- Surfaces a non-fatal warning (on stderr in text mode, in the JSON `warnings` array) when `.specify/plan.yaml` exists and any plan entry references the removed project — naming each affected entry so the operator can rewire them via `specify plan amend <change> --project <other>` separately.

## See also

- [specify initiative](initiative.md) -- operator brief at `.specify/initiative.md`.
- [specify workspace](workspace.md) -- materialise registry projects under `.specify/workspace/<peer>/`.
- [Configuration Files → registry.yaml](../configuration.md#registryyaml) -- file format reference.
- [Migrating CLI v1](../../explanation/migrating-cli-v1.md) -- rename map for the cleanup.

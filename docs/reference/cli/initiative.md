# specify initiative

Manage the operator-authored initiative brief at `.specify/initiative.md`.

## Subcommands

### specify initiative create

Scaffold `.specify/initiative.md` with the frontmatter template.

```bash
specify initiative create <name>
```

Refuses to overwrite an existing brief — mirrors the `specify plan create` posture for `plan.yaml`. (Renamed from the v1 `init` verb by RFC-9 §1F; see [Migrating CLI v1](../../explanation/migrating-cli-v1.md#v1x-renames).)

### specify initiative show

Render the brief content (frontmatter + prose body).

```bash
specify initiative show [--format json]
```

`--format json` emits the parsed frontmatter alongside the prose body for tooling consumers (e.g. `/spec:plan`).

## See also

- [specify registry](registry.md) -- platform registry (top-level since the CLI cleanup; previously `specify initiative registry`).
- [specify workspace](workspace.md) -- workspace sync, status, and push (moved from `specify initiative workspace` in RFC-3b).
- [Migrating CLI v1](../../explanation/migrating-cli-v1.md) -- rename map for the cleanup.

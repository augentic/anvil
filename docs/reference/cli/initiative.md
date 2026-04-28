# specify initiative

Manage the operator-authored initiative brief at `.specify/initiative.md`.

## Subcommands

### specify initiative init

Scaffold `.specify/initiative.md` with the frontmatter template.

```bash
specify initiative init
```

Idempotent — re-running on an existing brief is a no-op.

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

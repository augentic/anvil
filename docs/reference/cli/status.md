# specify status

Project dashboard summarising registry, plan, and active changes.

## Synopsis

```bash
specify status [--format json|table]
```

## Description

`specify status` (no arguments) is the operator's project dashboard. It rolls up three views into one call:

- **`registry`** -- summary of `.specify/registry.yaml` (project count, multi-repo flag).
- **`plan`** -- progress of `.specify/plan.yaml` (per-status counts, current `in-progress` entry, blocking entries).
- **`changes`** -- active changes under `.specify/changes/` with lifecycle state and task progress.

For the focused single-change view, use [`specify change status <name>`](change.md). The bare `specify status` is no longer overloaded with a positional `<name>` argument -- that responsibility moved to `change status` in the CLI cleanup.

## Options

| Option | Description |
|--------|-------------|
| `--format` | Output format: `json` for structured output, `table` (default) for human-readable. |

## JSON output

```json
{
  "registry": { ... },
  "plan": { ... },
  "changes": [ ... ]
}
```

The top-level keys mirror the noun groups they summarise:

| Key | Description |
|-----|-------------|
| `registry` | Project count, schemas in use, multi-repo flag (or `null` when no `registry.yaml` exists). |
| `plan` | Per-status counts (`pending`, `in-progress`, `done`, `blocked`, `failed`, `skipped`), current entry, blocking entries (or `null` when no `plan.yaml` exists). |
| `changes` | Array of active changes, each with `name`, `status`, `schema`, `tasks`, `artifacts`. |

## See also

- [specify change status](change.md) -- detailed per-change status
- [specify change list](change.md) -- list active changes
- [specify plan status](plan.md) -- per-entry plan view
- [specify registry show](registry.md) -- raw registry contents
- [Migrating CLI v1](../../explanation/migrating-cli-v1.md) -- rename map for the cleanup.

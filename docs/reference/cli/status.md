# specify status

Project dashboard summarising registry, plan, and active slices.

## Synopsis

```bash
specify status [--format json|table]
```

## Description

`specify status` (no arguments) is the operator's project dashboard. It rolls up three views into one call:

- **`registry`** -- summary of `registry.yaml` (project count, multi-repo flag).
- **`plan`** -- progress of `plan.yaml` (per-status counts, current `in-progress` entry, blocking entries).
- **`changes`** -- active slices under `.specify/slices/` with lifecycle state and task progress.

For the focused single-slice view, use [`specify slice status <name>`](change.md). The bare `specify status` is no longer overloaded with a positional `<name>` argument -- that responsibility moved to `change status` in the CLI cleanup.

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
| `changes` | Array of active slices, each with `name`, `status`, `adapter`, `tasks`, `artifacts`. |

## See also

- [specify slice status](change.md) -- detailed per-slice status
- [specify slice status](slice.md#specify-slice-status) -- inspect a single slice
- [specify plan status](plan.md) -- per-entry plan view
- [specify registry show](registry.md) -- raw registry contents

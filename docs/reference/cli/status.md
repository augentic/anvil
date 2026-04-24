# specify status

Show the current state of Specify in the project.

## Synopsis

```bash
specify status [<change-name>] [--format json|table]
```

## Description

A top-level convenience command that lists active changes and their progress. Equivalent to running `specify change list` with per-change detail.

When a `change-name` is provided, shows focused detail for that change (same as `specify change status <name>`).

## Options

| Option | Description |
|--------|-------------|
| `change-name` | Optional. Show detail for a specific change. |
| `--format` | Output format: `json` for structured output, `table` (default) for human-readable. |

## JSON output

Returns a `changes` array where each entry contains:

| Field | Description |
|-------|-------------|
| `name` | Change name |
| `status` | Lifecycle state |
| `schema` | Schema URL |
| `tasks` | `{ total, complete }` or `null` if no tasks file |
| `artifacts` | Map of brief ID to completion boolean |

## See also

- [specify change status](change.md) -- detailed per-change status
- [specify change list](change.md) -- list active changes

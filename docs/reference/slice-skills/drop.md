# /spec:drop

Discard a slice without merging specs into the baseline.

## Synopsis

```text
/spec:drop [change-name?] [reason "<rationale>"]
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `change-name` | No | Name of the slice to drop. If omitted, prompts for selection. |
| `--reason` | No | Rationale for dropping. Skips interactive confirmation when provided. |

## When to use

- A change was exploratory and should not be merged.
- A change has been superseded by a different approach.
- A change is blocked and should be discarded.

## Artifacts produced

| Artifact | Location | Content |
|----------|----------|---------|
| Archived change | `.specify/archive/YYYY-MM-DD-<name>/` | The full slice directory with `dropped` status |

## Behavior

1. Confirms with the user (unless `--reason` is supplied for non-interactive use).
2. Runs `specify slice drop` which:
   - Transitions the slice to `dropped`.
   - Archives the slice directory.
3. Baseline specs remain unchanged.

## Lifecycle transitions

`created|defined|building|complete --> dropped`

Drop is available from any pre-terminal state.

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| Change already terminal | Change is already merged or dropped | No action needed |
| No active slices | No changes exist to drop | Nothing to do |

## Examples

```text
# Drop interactively
/spec:drop

# Drop with a reason (non-interactive)
/spec:drop add-auth reason "Superseded by SSO integration"
```

## See also

- [/spec:merge](merge.md) -- the alternative to dropping
- [Lifecycle](../lifecycle.md) -- the dropped state

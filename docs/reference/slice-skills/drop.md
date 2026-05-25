# /spec:drop

Discard a slice without merging specs into the baseline.

## Synopsis

```text
/spec:drop [slice-name] [reason "<rationale>"]
```

## Arguments

| Argument | Required | Description |
| -------- | -------- | ----------- |
| `slice-name` | No | Name of the slice to drop. Required in non-interactive mode. |
| `reason` | No | Rationale for dropping. Skips interactive confirmation when provided. |

## When to use

- A slice was exploratory and should not be merged.
- A slice has been superseded by a different approach.
- A slice is blocked and should be discarded.

## Artifacts produced

| Artifact | Location | Content |
| -------- | -------- | ------- |
| Archived slice | `.specify/archive/YYYY-MM-DD-<name>/` | Full slice directory with `dropped` status |

Baseline specs remain unchanged.

## Behavior

1. **Select slice** — use argument or prompt when multiple active slices exist.
2. **Check lifecycle** — warn when slice is `built` (ready for merge); refuse when already `merged` or `dropped`.
3. **Confirm** — AskQuestion unless `reason` supplied (non-interactive).
4. **Drop** — `specrun slice transition <name> dropped --reason "..."`; CLI archives the slice directory.

## Lifecycle transitions

`* → dropped` from any pre-terminal slice state (`refining`, `refined`, `built`)

## Error modes

| Error | Cause | Resolution |
| ----- | ----- | ---------- |
| Already terminal | Slice is `merged` or `dropped` | No action needed |
| No active slices | Nothing to drop | Nothing to do |

## Examples

```text
# Drop interactively
/spec:drop

# Drop with a reason (non-interactive)
/spec:drop add-auth reason "Superseded by SSO integration"
```

## See also

- [/spec:merge](merge.md) — the alternative to dropping
- [Lifecycle](../lifecycle.md) — the dropped state

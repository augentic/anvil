# /spec:status

Show the current state of active changes.

## Synopsis

```text
/spec:status [change-name?]
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `change-name` | No | Show detail for a specific change. If omitted, shows all active changes. |

## When to use

- You want to check which changes are active.
- You want to see what artifacts are complete for a change.
- You want to check how many tasks remain.

## Artifacts produced

None. This is a read-only skill.

## Behavior

1. Runs `specify status` (or `specify change status <name>` for a specific change).
2. Renders a summary including:
   - Active changes with lifecycle status.
   - Per-brief artifact completion.
   - Task progress (completed / total).
   - Next-step guidance.

## Lifecycle transitions

None.

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| No `.specify/` directory | Project not initialised | Run `/spec:init` |

## Examples

```text
# Show all active changes
/spec:status

# Show detail for a specific change
/spec:status add-auth
```

## See also

- [Lifecycle](../lifecycle.md) -- the states shown in status output
- [/spec:init](init.md) -- required before status will work

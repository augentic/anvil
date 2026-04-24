# /spec:verify

Detect drift between code and baseline specs.

## Synopsis

```text
/spec:verify [capability-name?]
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `capability-name` | No | Verify a specific capability. If omitted, verifies all baseline specs. |

## When to use

- Before merging a new change, to confirm the baseline is still accurate.
- Periodically, to catch undocumented code changes.
- After a deployment or refactor, to detect behavioral drift.

## Artifacts produced

None. This is a read-only skill. Produces a drift report.

## Behavior

1. Lists all baseline specs in `.specify/specs/`.
2. For each capability, locates the corresponding source code.
3. Compares each behavioral requirement in the spec against the implementation.
4. Classifies each requirement:

| Classification | Meaning |
|---------------|---------|
| `COVERED` | Code implements the requirement as specified |
| `DRIFTED` | Code behavior diverges from the specification |
| `MISSING` | Specified requirement has no corresponding implementation |
| `UNSPECIFIED` | Code behavior exists with no corresponding spec |

5. Reports findings with suggested actions for each drift case.

## Lifecycle transitions

None.

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| No baseline specs | Nothing has been merged yet | Run `/spec:merge` on a completed change first |
| Source not found | Cannot locate implementation for a capability | Check project structure and capability naming |

## Examples

```text
# Verify all capabilities
/spec:verify

# Verify a specific capability
/spec:verify user-auth
```

## See also

- [Lifecycle](../lifecycle.md) -- baseline accumulation
- [/spec:merge](merge.md) -- how specs reach the baseline
- [Directory Layout](../directory-layout.md) -- where baseline specs live

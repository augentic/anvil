# /spec:merge

Merge a built slice into the baseline — apply spec deltas, archive the slice, stamp the plan entry `done`.

## Synopsis

```text
/spec:merge [slice-name]
```

## Arguments

| Argument | Required | Description |
| -------- | -------- | ----------- |
| `slice-name` | No | Name of the slice to merge. When omitted, uses the active `in-progress` entry from `specify plan next`. |

## When to use

- All tasks are complete (slice is `built`) and you want to fold deltas into the baseline.
- `/spec:execute` reached the merge phase or you ran build successfully as a breakout.

Not when the slice is still `refining` or `refined` (use [/spec:build](build.md)).

## Artifacts produced

| Artifact | Location | Content |
| -------- | -------- | ------- |
| Merged baseline specs | `.specify/specs/<unit>/spec.md` | Updated baseline spec files |
| Adapter output files | Project paths (`crates/`, `contracts/`, …) | Code or contracts from the slice |
| Archived slice | `.specify/archive/YYYY-MM-DD-<name>/` | Full slice directory for audit |
| Per-entry `done` | `plan.yaml` | Written only by `specify slice merge` |

## Behavior

1. **Resolve active slice** — `specify plan next`; validate optional `[slice-name]` matches active entry.
2. **Acquire plan lock** when invoked standalone.
3. **Workspace routing** — `chdir` into workspace slot when `project` is set.
4. **Refuse if not `built`** — hint toward `/spec:build` or report already finalised.
5. **Run target merge brief** — pre-merge gates (cargo, clippy, tests, adapter-specific validators).
6. **Apply merge** — `specify slice merge run <slice>` applies deltas, transitions slice to `merged`, archives slice dir, stamps plan entry `done`.
7. **Post-merge hook** — some targets re-validate promoted baseline; failures are observability only (merge already landed).

Use `specify slice merge preview` to preview without writing. Use `specify slice merge conflict-check` to probe baseline drift.

## Lifecycle transitions

`built → merged`; per-entry: `in-progress → done`

## Error modes

| Error | Cause | Resolution |
| ----- | ----- | ---------- |
| Slice not built | Tasks incomplete or still `refined` | Run `/spec:build` |
| Baseline conflict | Baseline changed since refine | Re-refine or resolve conflicts manually |
| Pre-merge gate failure | Target validation failed | Fix and re-run merge |
| Already finalised | Slice `merged` or `dropped` | No action needed |

## Examples

```text
# Merge the active in-progress slice
/spec:merge

# Merge a specific slice
/spec:merge fix-typo
```

## See also

- [/spec:build](build.md) — complete tasks before merging
- [Lifecycle](../lifecycle.md) — merged state and archiving
- [Directory layout](../directory-layout.md) — archive paths

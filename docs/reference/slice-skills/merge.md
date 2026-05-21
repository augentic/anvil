# /spec:merge

Merge a completed slice into the baseline.

## Synopsis

```text
/spec:merge [change-name?]
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `change-name` | No | Name of the slice to merge. If omitted, uses the only active slice or prompts for selection. |

## When to use

- All tasks are complete and you want to finalise the slice.

## Artifacts produced

| Artifact | Location | Content |
|----------|----------|---------|
| Merged baseline specs | `.specify/specs/<adapter>/spec.md` | Updated or new baseline spec files |
| Adapter output files | Adapter-owned paths such as `contracts/` or generated source trees | Updated project outputs produced by the slice |
| Merged baseline composition (Vectis) | `.specify/specs/composition.yaml` | Updated baseline screen layouts |
| Archived slice | `.specify/archive/YYYY-MM-DD-<name>/` | The full slice directory, preserved for audit |

## Behavior

1. Validates that the slice is in `complete` state.
2. Previews the merge via `specify slice merge preview` -- shows what will change in the baseline.
3. Checks for baseline drift via `specify slice merge conflict-check` -- detects whether the baseline has changed since define.
4. Confirms with the user (unless running non-interactively).
5. Runs `specify slice merge run` which:
   - Applies spec deltas to the baseline.
   - Copies contract files into `contracts/` using opaque file replacement -- files that share a path are replaced; files absent from the slice are left untouched.
   - Applies composition deltas to the baseline `composition.yaml` (Vectis only -- screen-level `added`/`modified`/`removed` operations with per-screen checksum conflict detection).
   - Validates coherence of the merged baseline.
   - Transitions the slice to `merged`.
   - Moves the slice directory to the archive.
   - **Workspace clone auto-commit.** When the merge runs inside a workspace clone (CWD under `.specify/workspace/*/` with `.specify/project.yaml`), `specify slice merge` auto-commits only `.specify/specs/` and `.specify/archive/` with message `specify: merge <slice-name>`. Project-output residue outside those two trees is left uncommitted for `/spec:execute`, which commits it as `specify: residue <slice-name>` before marking a routed plan entry `done`. If the merge commit fails, the spec merge still succeeds -- the commit failure is a warning. The operator publishes prepared branches via `specify workspace push`.
6. Writes phase outcome. On the success path, `specify slice merge run` stamps the outcome automatically -- the skill does not call `outcome set` separately. On failure or deferred paths, the skill writes the outcome via `specify slice outcome set`.

## Lifecycle transitions

`complete --> merged`

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| Change not complete | Tasks remain unfinished | Run `/spec:build` to complete remaining tasks |
| Conflict detected | Baseline changed since refine (another slice merged) | Re-run `/spec:refine` (or drop the slice and re-add via `specify plan amend`) to update specs against current baseline, or resolve conflicts manually |
| Coherence failure | Merged baseline has structural issues | Fix spec files and retry |

## Examples

```text
# Merge the only active slice
/spec:merge

# Merge a specific change
/spec:merge add-auth
```

## See also

- [/spec:build](build.md) -- complete tasks before merging
- [Lifecycle](../lifecycle.md) -- the merged state and archiving
- [Directory Layout](../directory-layout.md) -- where archived changes go

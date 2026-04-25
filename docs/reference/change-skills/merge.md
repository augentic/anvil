# /spec:merge

Merge a completed change into the baseline.

## Synopsis

```text
/spec:merge [change-name?]
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `change-name` | No | Name of the change to merge. If omitted, uses the only active change or prompts for selection. |

## When to use

- All tasks are complete and you want to finalise the change.

## Artifacts produced

| Artifact | Location | Content |
|----------|----------|---------|
| Merged baseline specs | `.specify/specs/<capability>/spec.md` | Updated or new baseline spec files |
| Merged baseline composition (Vectis) | `.specify/specs/composition.yaml` | Updated baseline screen layouts |
| Archived change | `.specify/archive/YYYY-MM-DD-<name>/` | The full change directory, preserved for audit |

## Behavior

1. Validates that the change is in `complete` state.
2. Previews the merge via `specify spec preview` -- shows what will change in the baseline.
3. Checks for baseline drift via `specify spec conflict-check` -- detects whether the baseline has changed since define.
4. Confirms with the user (unless running non-interactively).
5. Runs `specify merge` which:
   - Applies spec deltas to the baseline.
   - Applies composition deltas to the baseline `composition.yaml` (Vectis only -- screen-level `added`/`modified`/`removed` operations with per-screen checksum conflict detection).
   - Validates coherence of the merged baseline.
   - Transitions the change to `merged`.
   - Moves the change directory to the archive.
   - **Workspace clone auto-commit (RFC-3b).** When the merge runs inside a workspace clone (CWD under `.specify/workspace/*/` with `project.yaml`), `specify merge` auto-commits the merged baseline and archive with message `"specify: merge <change-name>"`. The commit stages only `.specify/` subtrees. If the commit fails, the spec-merge still succeeds -- the commit failure is a warning. The operator publishes changes via `specify workspace push`.
6. Writes phase outcome. On the success path, `specify merge` stamps the outcome automatically -- the skill does not call `phase-outcome` separately. On failure or deferred paths, the skill writes the outcome via `specify change phase-outcome`.

## Lifecycle transitions

`complete --> merged`

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| Change not complete | Tasks remain unfinished | Run `/spec:build` to complete remaining tasks |
| Conflict detected | Baseline changed since define (another change merged) | Re-run `/spec:define` to update specs against current baseline, or resolve conflicts manually |
| Coherence failure | Merged baseline has structural issues | Fix spec files and retry |

## Examples

```text
# Merge the only active change
/spec:merge

# Merge a specific change
/spec:merge add-auth
```

## See also

- [/spec:build](build.md) -- complete tasks before merging
- [/spec:verify](verify.md) -- check drift after merging
- [Lifecycle](../lifecycle.md) -- the merged state and archiving
- [Directory Layout](../directory-layout.md) -- where archived changes go

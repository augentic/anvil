# specify merge

Commit delta merge and archive a change.

## Synopsis

```bash
specify merge <change-dir>
```

## Description

The terminal merge operation. Performs:

1. Applies spec deltas from the change to the baseline at `.specify/specs/`.
2. Validates coherence of the merged baseline.
3. Transitions the change to `merged`.
4. Moves the change directory to `.specify/archive/YYYY-MM-DD-<name>/`.

This is the CLI command invoked by `/spec:merge` after preview and conflict-check pass. It is a single atomic operation -- if any step fails, no changes are committed.

**Workspace clone auto-commit (RFC-3b).** When `specify merge` runs inside a workspace clone (CWD is under `.specify/workspace/*/` and contains `.specify/project.yaml`), it auto-commits the merged baseline and archived change directory with message `"specify: merge <change-name>"`. Only `.specify/` subtrees are staged. A commit failure is a warning, not an error -- the spec-merge still succeeds. The operator uses `specify workspace push` to publish commits to remotes.

## Preconditions

- Change must be in `complete` state.
- `specify spec preview` and `specify spec conflict-check` should pass (the skill checks these before calling merge).

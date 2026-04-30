# Land an Initiative

Once `/spec:execute --loop` has driven every change to `done` and `specify workspace push` has shipped the local commits as PRs, the remaining work is **landing** -- merging the PRs and archiving the plan. This how-to covers both modes (autonomous and supervised) and the four guards that `specify initiative finalize` runs before it touches anything.

## Prerequisites

- An initiative whose plan is fully driven: every entry in `.specify/plan.yaml` is `done`, `failed`, or `skipped`.
- `specify workspace push` has been run successfully (one PR per workspace clone with local commits ahead of `main`).
- `gh` (the GitHub CLI) installed and authenticated against every registry remote -- both `workspace merge` and `initiative finalize` shell out to `gh`.

## Autonomous: `workspace merge` then `initiative finalize`

The shortest path lands every PR with green CI, then archives.

### 1. Squash-merge the PRs

```bash
specify workspace merge
```

For each project with an open PR on `specify/<initiative-name>`, the verb:

1. Looks up the PR via `gh pr list --head specify/<initiative-name> --state all`.
2. Refuses if `headRefName` is not `specify/<initiative-name>` exactly (the `branch-pattern-mismatch` guard -- never `--admin`, never `--auto`).
3. Inspects checks via `gh pr checks`. Any `pending` -> `pending-checks`. Any `fail`/`cancel` -> `failed-checks`.
4. Otherwise (every check `pass` or `skipping`) runs `gh pr merge <pr> --squash` and reports `merged`.

Best-effort across projects: a single project's failure surfaces in its row without aborting the others. Exit code is `0` only when every project lands on `merged`, `would-merge`, or `no-branch`.

Use `--dry-run` to preview the classification without invoking `gh pr merge`:

```bash
specify workspace merge --dry-run
```

### 2. Confirm and archive

Once every PR is `merged`:

```bash
specify initiative finalize
```

The verb runs four guards in order. **All-or-nothing:** any guard failure refuses the run with a per-project status table and leaves the on-disk state untouched.

| Guard | Refusal code | Recovery |
|-------|--------------|----------|
| Plan-presence (`.specify/plan.yaml` exists) | `plan-not-found` | The initiative is already finalized -- no action needed. |
| Plan terminal-state (every entry `done`/`failed`/`skipped`) | `non-terminal-entries-present` | Drive the offending entry to terminal via `/spec:execute` or `specify plan transition`. |
| Per-project PR-state (every `specify/<name>` PR is `MERGED`) | `unmerged` / `closed` / `branch-pattern-mismatch` / `failed` | Merge the outstanding PR (by hand or `specify workspace merge`); see [Initiative landing issues](../appendices/troubleshooting.md#initiative-landing-issues) for `branch-pattern-mismatch`. |
| Workspace-cleanliness (`git status --porcelain` empty per clone) | `dirty` | Commit or stash uncommitted work in `.specify/workspace/<peer>/`. |

When every guard passes, the verb runs `Plan::archive` programmatically. `plan.yaml`, `.specify/initiative.md`, and `.specify/plans/<name>/` move atomically into `.specify/archive/plans/<YYYYMMDD>-<name>/`. The archive write is preflighted (both destinations) so a collision returns an error before any file is touched.

## Supervised: merge by hand, then `initiative finalize`

When you want a code-review pause before each merge -- or when CI is configured to require manual approval -- skip `workspace merge` entirely and merge each PR on the forge:

```bash
gh pr list --head specify/<initiative-name>
# review each PR in the browser, merge by hand
specify initiative finalize
```

`finalize`'s guards do not care **how** the PRs got merged -- only that every project's PR is `MERGED` on remote. The supervised path is otherwise identical to the autonomous one.

## `--clean`: prune workspace clones

By default, finalize leaves `.specify/workspace/<peer>/` clones on disk. They are cheap to refresh via `specify workspace sync` for the next initiative. To prune them at the same time:

```bash
specify initiative finalize --clean
```

`--clean` removes every `.specify/workspace/<peer>/` for non-symlink registered projects after the archive completes. Symlink-mode projects (`url: .` or relative paths) are skipped -- they point at source trees the operator owns separately.

`--clean` refuses when any clone has a dirty working tree. The dirty-clone diagnostic warns that `--clean` would drop the uncommitted changes; the operator commits or discards before re-running with `--clean`.

## `--dry-run`: preview the guard table

```bash
specify initiative finalize --dry-run
```

Observation-only: classifies every guard, prints the per-project status table, and stops. Never invokes `gh pr merge` (`workspace merge` is a separate verb anyway) and never moves files. Useful for "is this initiative ready to land?" checks before you commit.

## Idempotency

`finalize` is idempotent across the canonical recovery path:

- First run refuses on an unmerged PR -> operator merges manually -> re-run completes the archive.
- After successful finalize, re-running returns `plan-not-found` (the explicit "already finalized" signal). This is the canonical way to confirm an initiative is closed.

The umbrella mode `/spec:plan --orchestrate <name>` honours the same idempotency: re-running the umbrella against a state where every PR is merged and the plan still exists on disk skips straight to `specify initiative finalize`.

## Output reference

Both `workspace merge` and `initiative finalize` emit the same shape of per-project status table in text mode and `--format json`. Refer to:

- [`specify workspace merge` -- output](../reference/cli/workspace.md#specify-workspace-merge) for the merge status vocabulary (`merged`, `would-merge`, `pending-checks`, `failed-checks`, `closed`, `no-branch`, `branch-pattern-mismatch`, `failed`).
- [`specify initiative finalize` -- output](../reference/cli/initiative.md#specify-initiative-finalize) for the finalize status table (`merged`, `unmerged`, `closed`, `no-branch`, `branch-pattern-mismatch`, `dirty`, `failed`) and the JSON v2 envelope.

## See also

- [Cross-Repo Initiatives](../tutorials/cross-repo-initiative.md) -- end-to-end walkthrough that exercises this landing flow.
- [`specify workspace`](../reference/cli/workspace.md) -- CLI reference for `merge` and `push`.
- [`specify initiative`](../reference/cli/initiative.md) -- CLI reference for `finalize`.
- [`/spec:plan --orchestrate`](../reference/initiative-skills/initiative.md) -- the Layer 4 umbrella mode (formerly `/spec:initiative`) that automates this whole sequence with `--auto-merge`.
- [Initiative landing issues](../appendices/troubleshooting.md#initiative-landing-issues) -- `branch-pattern-mismatch`, `plan-not-found`, dirty clones.

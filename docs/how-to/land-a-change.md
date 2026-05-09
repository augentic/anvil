# Land a Change

Once `/change:execute loop` has driven every plan entry to `done` and `specify workspace push` has shipped the local commits as PRs, the remaining work is **landing** -- merging the PRs through the forge and archiving the plan. This how-to covers the operator-owned merge step and the four guards that `specify change finalize` runs before it touches anything.

This is the checklist version. For the full scenario with example project names, expected outputs, and the `/change:plan <name> orchestrate` variants, read the [Landing a Change tutorial](../tutorials/landing-a-change.md).

## Prerequisites

- A change whose plan is fully driven: every entry in `plan.yaml` is `done`, `failed`, or `skipped`.
- `specify workspace push` has been run successfully (one PR per prepared `specify/<change-name>` branch).
- `gh` (the GitHub CLI) installed and authenticated against every registry remote if you want to inspect or merge PRs from the terminal. `specify change finalize` also uses `gh` to confirm remote PR state.

## 1. Merge the PRs

Review each PR and merge it through the forge UI or an explicit `gh pr merge` command:

```bash
gh pr checks <pr> -R <owner/repo>
gh pr merge <pr> -R <owner/repo> --squash
```

Specify does not merge PRs automatically. RFC-14 removed `workspace merge` automation; `specify workspace merge` is no longer an active CLI subcommand. Use forge UI / `gh pr merge` plus `specify change finalize`.

## 2. Confirm and archive

Once every PR is merged:

```bash
specify change finalize
```

The verb runs four guards in order. **All-or-nothing:** any guard failure refuses the run with a per-project status table and leaves the on-disk state untouched.

| Guard | Refusal code | Recovery |
|-------|--------------|----------|
| Plan-presence (`plan.yaml` exists) | `plan-not-found` | The change is already finalized -- no action needed. |
| Plan terminal-state (every entry `done`/`failed`/`skipped`) | `non-terminal-entries-present` | Drive the offending entry to terminal via `/change:execute` or `specify change plan transition`. |
| Per-project PR-state (every `specify/<name>` PR is `MERGED`) | `unmerged` / `closed` / `branch-pattern-mismatch` / `failed` | Merge the outstanding PR through the forge UI or `gh pr merge`; see [change landing issues](../appendices/troubleshooting.md#change-landing-issues) for `branch-pattern-mismatch`. |
| Workspace-cleanliness (`git status --porcelain` empty per clone) | `dirty` | Commit or stash uncommitted work in `.specify/workspace/<peer>/`. |

When every guard passes, the verb runs `Plan::archive` programmatically. `plan.yaml`, `change.md`, and `.specify/plans/<name>/` move atomically into `.specify/archive/plans/<YYYYMMDD>-<name>/`. The archive write is preflighted (both destinations) so a collision returns an error before any file is touched.

`finalize`'s guards do not care **how** the PRs got merged -- only that every project's PR is `MERGED` on remote.

## `--clean`: prune workspace clones

By default, finalize leaves `.specify/workspace/<peer>/` clones on disk. They are cheap to refresh via `specify workspace sync` for the next change. To prune them at the same time:

```bash
specify change finalize --clean
```

`--clean` removes every `.specify/workspace/<peer>/` for non-symlink registered projects after the archive completes. Symlink-mode projects (`url: .` or relative paths) are skipped -- they point at source trees the operator owns separately.

`--clean` refuses when any clone has a dirty working tree. The dirty-clone diagnostic warns that `--clean` would drop the uncommitted changes; the operator commits or discards before re-running with `--clean`.

## `--dry-run`: preview the guard table

```bash
specify change finalize --dry-run
```

Observation-only: classifies every guard, prints the per-project status table, and stops. Never invokes `gh pr merge` and never moves files. Useful for "is this change ready to land?" checks before you commit.

## Idempotency

`finalize` is idempotent across the canonical recovery path:

- First run refuses on an unmerged PR -> operator merges manually -> re-run completes the archive.
- After successful finalize, re-running returns `plan-not-found` (the explicit "already finalized" signal). This is the canonical way to confirm a change is closed.

The umbrella mode `/change:plan <name> orchestrate` honours the same idempotency: re-running the umbrella against a state where every PR is merged and the plan still exists on disk skips straight to `specify change finalize`.

## Output reference

Refer to [`specify change finalize` -- output](../reference/cli/change.md#specify-change-finalize) for the finalize status table (`merged`, `unmerged`, `closed`, `no-branch`, `branch-pattern-mismatch`, `dirty`, `failed`) and the JSON v2 envelope.

## See also

- [Cross-Repo Changes](../tutorials/cross-repo-change.md) -- end-to-end walkthrough that exercises this landing flow.
- [`specify workspace`](../reference/cli/workspace.md) -- CLI reference for `sync`, `status`, and `push`.
- [`specify change`](../reference/cli/change.md) -- CLI reference for `finalize`.
- [`/change:plan <name> orchestrate`](../reference/change-skills/change.md) -- the Layer 4 umbrella mode that automates through PR creation, then finalizes after operator merge.
- [Change landing issues](../appendices/troubleshooting.md#change-landing-issues) -- `branch-pattern-mismatch`, `plan-not-found`, dirty clones.

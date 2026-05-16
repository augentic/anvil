# Land a Change

A change lifecycle reads `/change:draft → /change:execute → /change:finalize`. This how-to is the checklist for each stage and the operator-owned review pause that sits between draft and execute. For the full scenario with example project names and expected outputs, read the [Landing a Change tutorial](../tutorials/landing-a-change.md).

## Prerequisites

- A bootstrapped project (single-repo) or hub (cross-repo). See [Bootstrap a platform hub](bootstrap-a-platform-hub.md).
- For multi-repo changes: `gh` (the GitHub CLI) installed and authenticated against every registry remote. `/change:finalize` uses `gh` to confirm remote PR state.

## 1. Author the plan with `/change:draft`

```text
/change:draft <change-name> [from <docs>] [against <baseline-paths>] [source <key>=<path-or-url>]
```

`/change:draft` mints `change.md` and `plan.yaml` together (via `specify change draft`), runs `specify registry validate`, walks the brief pipeline (discovery → optional sync-workspace → propose → optional assignment), and stops at a hand-off summary that names the slice count, target projects, and any `Warning`-level findings from `specify plan validate`. It refuses to proceed if `plan.yaml` already exists -- re-author with `extend` to append.

The skill does not start any per-slice work; the operator review pause is the design.

## 2. Review the plan

`/change:draft` deliberately stops at the human seam. Inspect what it produced before kicking off execution:

```bash
specify plan status                       # current entries with statuses
specify plan show                         # rendered plan.yaml
specify plan amend <entry> --depends-on a,b   # edit dependencies
specify plan amend <entry> --project api      # rewire the target project
```

When the plan reads correctly, continue to step 3. If the change should be abandoned at this point, delete the change brief and `plan.yaml` (see the [Drop a slice](drop-a-slice.md) guide for slice-level abandons; for a whole change, remove `change.md` and `plan.yaml` and re-author).

## 3. Drive the per-slice loop with `/change:execute loop`

```text
/change:execute loop
```

`/change:execute` picks the next eligible slice, runs `/spec:define → /spec:build → /spec:merge` against it, transitions the plan entry's status, and continues until no eligible slice remains. Halt classifications (`stuck`, `halted`, `driver-interrupted`, `registry-amendment-required`) re-enter the same skill once the cause is fixed.

`/change:execute` is the explicit second peer skill in the change lifecycle.

## 4. Close the change with `/change:finalize`

```text
/change:finalize <change-name>
```

`/change:finalize` runs the post-execute tail in three steps:

1. **Push** -- `specify workspace push` publishes each prepared `specify/<change-name>` branch as a PR.
2. **PR observation** -- `gh pr list` (read-only) checks every PR is `MERGED`. The skill never merges PRs itself; halt with `pr-not-merged` until the operator merges through the forge UI or `gh pr merge`.
3. **Finalize** -- `specify change finalize` runs the four guards below and archives `plan.yaml`, `change.md`, and `.specify/plans/<name>/`.

Re-run `/change:finalize <change-name>` after merging open PRs externally; the skill re-reads plan and PR state on every invocation.

## `specify change finalize` -- the canonical guard

The CLI verb `specify change finalize` runs four guards in order. **All-or-nothing:** any guard failure refuses the run with a per-project status table and leaves the on-disk state untouched.

| Guard | Refusal code | Recovery |
|-------|--------------|----------|
| Plan-presence (`plan.yaml` exists) | `plan-not-found` | The change is already finalized -- no action needed. |
| Plan terminal-state (every entry `done`/`failed`/`skipped`) | `non-terminal-entries-present` | Drive the offending entry to terminal via `/change:execute` or `specify plan transition`. |
| Per-project PR-state (every `specify/<name>` PR is `MERGED`) | `unmerged` / `closed` / `branch-pattern-mismatch` / `failed` | Merge the outstanding PR through the forge UI or `gh pr merge`; see [change landing issues](troubleshooting/change-landing.md) for `branch-pattern-mismatch`. |
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

`/change:finalize` honours the same idempotency at the skill layer: re-run after merging any open PRs and the skill resumes at the first incomplete step.

## Output reference

Refer to [`specify change finalize` -- output](../reference/cli/change.md#specify-change-finalize) for the finalize status table (`merged`, `unmerged`, `closed`, `no-branch`, `branch-pattern-mismatch`, `dirty`, `failed`) and the JSON v2 envelope.

## See also

- [Cross-Repo Changes](../tutorials/cross-repo-change.md) -- end-to-end walkthrough that exercises this lifecycle.
- [`specify workspace`](../reference/cli/workspace.md) -- CLI reference for `sync`, `status`, and `push`.
- [`specify change`](../reference/cli/change.md) -- CLI reference for `draft` and `finalize`.
- [`/change:draft`, `/change:execute`, `/change:finalize`](../reference/change-skills/index.md) -- the three peer skills that own the change lifecycle.
- [Change landing issues](troubleshooting/change-landing.md) -- `branch-pattern-mismatch`, `plan-not-found`, dirty clones.

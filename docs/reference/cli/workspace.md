# specrun workspace

Materialise, prepare, and publish registry-backed workspace slots for multi-repo changes.

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`sync`](#specify-workspace-sync) | Create or refresh workspace slots. With no selectors, syncs every registry project; with selectors, materialises only those slots. |
| [`push`](#specify-workspace-push) | Publish an existing exact `specify/<change-name>` branch to its remote and create or update a PR. |

## Selectors

`sync` and `push` accept optional project selectors:

```bash
specrun workspace sync [<project>...]
specrun workspace push [<project>...]
```

Selectors are registry project names. Unknown selectors fail before filesystem, Git, or forge side effects. When selectors are omitted, `sync` operates on every project declared in `registry.yaml`; `push` classifies every registry project and only performs transport work for branches that need publication.

## Branch preparation

Before `/spec:execute` mutates a remote-backed workspace slot, the executor prepares the slot on the change branch:

1. Fetch `origin`.
2. Resolve `origin/HEAD` as the remote default branch.
3. Create or reuse `specify/<change-name>` from `origin/HEAD`.
4. Fast-forward from `origin/specify/<change-name>` when that branch already exists.
5. Refuse unsafe dirty work before checkout or mutation.

The hidden `workspace prepare` helper owns this pre-mutation step for the executor. Humans normally use the public lifecycle commands: `/spec:execute`, `specrun workspace push`, and `/spec:finalize` (which runs `specrun plan archive` after PR observation). If the remote default cannot be resolved, branch preparation fails with `origin-head-unresolved`.

## Subcommands

### specrun workspace sync

Clone or refresh selected projects declared in `registry.yaml` into `.specify/workspace/<project>/`.

```bash
specrun workspace sync [<project>...]
```

For each selected registry project:

- **Remote URL** (`git@`, `ssh://`, `https://`, `http://`) -- shallow-clones the repo into the workspace slot, or fetches an existing matching clone.
- **Local path** (`.`, `../foo`, `/absolute/path`) -- symlinks the resolved path into the workspace slot.
- **Greenfield** (remote URL, repo does not yet exist) -- creates the local workspace slot, runs `git init`, sets `origin`, and bootstraps `.specify/project.yaml` via `specrun init <adapter>`. Remote repositories are not created during sync; creation happens, when supported, during `workspace push`.

A partially bootstrapped slot (`.git/` present but `.specify/project.yaml` absent) is detected on re-run: `specrun init` is re-attempted without re-running `git init` or `git remote add`.

Selected sync materialises selected slots only. Unselected registry projects are not cloned, fetched, symlinked, or contract-refreshed. Running without selectors syncs all registry projects. Non-zero exit if any selected project fails, with a per-project status summary.

After materialisation succeeds, `sync` regenerates the committed `.specify/topology.lock` from each materialised slot's `project.yaml` (resolved target adapter, description, capabilities, keywords). The lock is the plan-time topology source for hub planning; it is machine-written (write-if-changed) and never hand-edited. `specrun plan validate` reports `topology-cache-stale` when a slot's `project.yaml` has diverged from the lock — the fix is to re-run `specrun workspace sync`.

### specrun workspace push

Publish selected workspace clones that are already on the exact change branch.

```bash
specrun workspace push [<project>...] [--dry-run]
```

The change name is read from `plan.yaml`; the expected branch is exactly `specify/<change-name>`. `workspace push` is transport-only PR publication/update. It never creates the local change branch, never checks out a branch, never commits files, never pushes a default branch, and never merges a PR.

**Per-project algorithm:**

1. **Selector preflight.** Unknown project selectors fail before any side effect.
2. **Worktree and remote.** The slot must be a Git worktree. Missing `origin` reports `local-only`.
3. **Branch guard.** The current branch must be exactly `specify/<change-name>`. Any other branch, including `main`, `master`, or detached `HEAD`, reports `no-branch`.
4. **Dirty guard.** `git status --porcelain` must be empty. Dirty checkouts report `failed`.
5. **Default-branch guard.** The expected change branch must not be the remote default branch.
6. **Remote branch inspection.** Compare local `HEAD` with `origin/specify/<change-name>` when present.
7. **Push.** Push `refs/heads/specify/<change-name>` to `origin` with `--force-with-lease`.
8. **PR.** Create a PR, or update the existing PR base, targeting the remote default branch resolved from `origin/HEAD`.

**Flags:**

| Flag | Description |
|------|-------------|
| `--dry-run` | Classify each selected project's push status without `git push`, `gh repo create`, or `gh pr create`. |
| `--format json` | Machine-readable JSON output. |

**Output (human-readable):**

```text
specify: workspace push - <change-name>

  traffic              pushed         specify/platform-v2 PR #42
  command-centre       up-to-date     specify/platform-v2 PR #7
  mobile               no-branch
  local-lib            local-only

0 created, 1 pushed, 1 up-to-date, 1 local-only, 1 no-branch. 0 failed.
```

**Output (JSON, `--format json`):**

```json
{
  "projects": [
    { "name": "traffic", "status": "pushed", "branch": "specify/platform-v2", "pr": 42 },
    { "name": "command-centre", "status": "up-to-date", "branch": "specify/platform-v2", "pr": 7 },
    { "name": "mobile", "status": "no-branch" },
    { "name": "local-lib", "status": "local-only" }
  ]
}
```

Under `--dry-run`, JSON adds `"dry-run": true` at the top level and human-readable action statuses are prefixed with `would-` for transport actions (for example, `would-pushed` and `would-created`).

**Status vocabulary:**

| Status | Meaning |
|--------|---------|
| `created` | Greenfield GitHub remote was created, then the change branch was pushed and a PR was created. |
| `pushed` | Existing remote branch was updated and a PR was created or updated. |
| `up-to-date` | Remote `specify/<change-name>` already matches local `HEAD`; the PR is created or updated if needed. |
| `local-only` | No `origin` remote is configured for this slot. No push or PR is attempted. |
| `no-branch` | The slot is not currently on exact `specify/<change-name>`, or the expected branch resolves to the remote default branch. |
| `failed` | The slot is dirty, Git or `gh` failed, the remote default could not be resolved for PR creation, or another transport error occurred. |

**Prerequisites:** `gh` (GitHub CLI) is required for GitHub repository creation and PR creation/update. Plain Git remotes can still be pushed when PR creation is not needed.

## PR landing

Automated workspace merge has been removed. There is no active `specrun workspace merge` subcommand. Merge each PR through the forge UI or `gh pr merge`, then run:

```bash
/spec:finalize <name>
```

`/spec:finalize` verifies the operator-merged PR state with `gh pr view` and archives the coordinator state via `specrun plan archive`; it does not merge PRs.

## See also

- [Cross-Repo Changes](../../tutorials/cross-repo-change.md) -- tutorial for multi-repo workflows
- [Configuration Files](../configuration.md) -- `registry.yaml` and `plan.yaml` format
- [/spec:execute](../change-skills/execute.md) -- skill that drives workspace execution
- [`specrun plan archive`](plan.md) -- archive verb used by `/spec:finalize` after PRs are operator-merged

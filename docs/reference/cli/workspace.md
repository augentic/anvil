# specify workspace

Materialise, inspect, and push workspace peer clones for multi-repo initiatives.

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`sync`](#specify-workspace-sync) | Clone or refresh every registry project into `.specify/workspace/<project>/`. Runs automatically during `/spec:plan`'s sync-peers phase; re-run by hand to refresh between initiatives. |
| [`status`](#specify-workspace-status) | Per-project materialisation report (slot path, type, HEAD sha, dirty flag, `.specify/` tree summary). |
| [`push`](#specify-workspace-push) | Ship local commits in each clone to its remote on `specify/<initiative-name>` and create a PR. |
| [`merge`](#specify-workspace-merge) | Squash-merge the open PRs once their CI is green (RFC-9 §4A). Refuses on `branch-pattern-mismatch`, never `--admin`/`--auto`. |

## Subcommands

### specify workspace sync

Clone or refresh every project declared in `.specify/registry.yaml` into `.specify/workspace/<project>/`.

```bash
specify workspace sync
```

For each registry project:

- **Remote URL** (`git@`, `ssh://`, `https://`, `http://`) -- shallow-clones the repo into the workspace slot.
- **Local path** (`.`, `../foo`, `/absolute/path`) -- symlinks the resolved path into the workspace slot.
- **Greenfield** (remote URL, repo does not yet exist) -- creates the workspace slot, runs `git init`, sets the remote, and bootstraps `.specify/project.yaml` via `specify init <schema> --schema-dir <dir>` using the initiating repo's `.specify/.cache/`.

A partially bootstrapped slot (`.git/` present but `.specify/project.yaml` absent) is detected on re-run: `specify init` is re-attempted without re-running `git init` or `git remote add`.

Non-zero exit if any project fails, with a per-project status summary.

### specify workspace status

Report the materialisation state of every registry project's workspace slot.

```bash
specify workspace status
```

Per-project output includes: slot path, materialisation type (`symlink`, `git-clone`, `missing`), HEAD sha, dirty flag, and `.specify/` tree summary.

### specify workspace push

Push workspace clones that have local commits back to their remote repositories.

```bash
specify workspace push [<project>...]
```

Omitting the project argument pushes all dirty clones. The initiative name for branch naming (`specify/<initiative-name>`) is read from `.specify/plan.yaml`.

**Per-project algorithm:**

1. **Remote resolution.** Remote URLs are used directly. Local paths read `git remote get-url origin`; if no remote exists, the project is skipped with `local-only` status.
2. **Branch.** Creates or updates `specify/<initiative-name>` from the clone's current HEAD.
3. **Repo creation (greenfield).** If the remote does not exist and the URL is a GitHub URL, creates the repo via `gh repo create`.
4. **Push.** `git push --force-with-lease -u origin specify/<initiative-name>`.
5. **PR.** Creates a PR via `gh pr create` if none exists for the branch.

**Flags:**

| Flag | Description |
|------|-------------|
| `--dry-run` | Classify each project's push status without performing any writes. No `git push`, `gh repo create`, or `gh pr create`. |
| `--format json` | Machine-readable JSON output. |

**Output (human-readable):**

```text
specify: workspace push — <initiative-name>

  traffic        pushed       specify/platform-v2  PR #42
  command-centre up-to-date
  mobile         created      specify/platform-v2  PR #7

1 created, 1 pushed, 1 up-to-date. 0 failed.
```

**Output (JSON, `--format json`):**

```json
{
  "projects": [
    { "name": "traffic", "status": "pushed", "branch": "specify/platform-v2", "pr": 42 },
    { "name": "command-centre", "status": "up-to-date" },
    { "name": "mobile", "status": "created", "branch": "specify/platform-v2", "pr": 7 },
    { "name": "local-lib", "status": "local-only" }
  ]
}
```

Under `--dry-run`, the JSON output adds `"dry_run": true` at the top level and action statuses are prefixed with `would-` in human-readable output (e.g. `would-push`, `would-create`).

**Status vocabulary:** `created` (remote repo created, greenfield), `pushed` (existing remote updated), `up-to-date` (no local commits ahead), `local-only` (no remote configured), `failed` (error).

**Prerequisites:** `gh` (GitHub CLI) is required only when repo creation or PR creation is needed. Plain `git push` works for any forge.

### specify workspace merge

Squash-merge the open PRs created by `workspace push` once their CI is green (RFC-9 §4A).

```bash
specify workspace merge [<project>...]
```

Omitting the project argument considers every entry in `.specify/registry.yaml`. The initiative name (and therefore the expected PR branch `specify/<initiative-name>`) is read from `.specify/plan.yaml`.

**Per-project algorithm:**

1. **Branch lookup.** `gh pr list --head specify/<initiative-name> --state all --json number --limit 1` followed by `gh pr view ... --json state,merged,headRefName,number,url`. No PR on the branch ⇒ `no-branch`.
2. **Branch-pattern guard.** Refuses to operate on any PR whose `headRefName` does not equal the resolved `specify/<initiative-name>` exactly. Surfaces the literal expected branch in the diagnostic so an operator can see the drift.
3. **Already-landed short-circuit.** `state == MERGED` ⇒ `merged`. `state == CLOSED` (without merge) ⇒ `closed`.
4. **Check inspection.** `gh pr checks --json bucket,name`. Any `fail`/`cancel` ⇒ `failed-checks`. Any `pending` ⇒ `pending-checks`. Otherwise (all `pass`/`skipping`, or empty list) proceed.
5. **Merge.** `--dry-run` ⇒ `would-merge` and stop. Otherwise `gh pr merge <pr> --squash` ⇒ `merged` on success, `failed` on shell error.

Best-effort across projects: a single project's failure surfaces in its row without aborting the others.

**Flags:**

| Flag | Description |
|------|-------------|
| `--dry-run` | Classify each project's mergeability without invoking `gh pr merge`. Mergeable PRs report `would-merge`. |
| `--format json` | Machine-readable JSON output. |

**Output (human-readable):**

```text
specify: workspace merge — platform-v2 (specify/platform-v2)

  traffic              merged                    PR #42     https://github.com/org/traffic/pull/42
  command-centre       pending-checks            PR #7      https://github.com/org/command-centre/pull/7
    pending checks: e2e
  mobile               no-branch
    no open PR on specify/platform-v2; run `specify workspace push` first

1 merged, 0 would-merge, 1 pending-checks, 0 failed-checks, 0 closed, 1 no-branch, 0 branch-pattern-mismatch, 0 failed.
```

**Output (JSON, `--format json`):**

```json
{
  "schema-version": 2,
  "initiative": "platform-v2",
  "expected-branch": "specify/platform-v2",
  "projects": [
    {
      "name": "traffic",
      "status": "merged",
      "pr-number": 42,
      "url": "https://github.com/org/traffic/pull/42",
      "head-ref-name": "specify/platform-v2"
    }
  ],
  "summary": {
    "merged": 1,
    "would-merge": 0,
    "pending-checks": 0,
    "failed-checks": 0,
    "closed": 0,
    "no-branch": 0,
    "branch-pattern-mismatch": 0,
    "failed": 0
  }
}
```

Under `--dry-run`, the JSON output adds `"dry-run": true` at the top level.

**Status vocabulary:**

| Status | Meaning |
|--------|---------|
| `merged` | PR already merged, or successfully squash-merged this run. |
| `would-merge` | Dry-run only: PR is mergeable; no merge attempt was made. |
| `pending-checks` | At least one CI check is still running. Operator action: wait. |
| `failed-checks` | At least one CI check failed or was cancelled. Operator action: fix CI, push, re-run. |
| `closed` | PR was closed without merging. |
| `no-branch` | No PR exists on `specify/<initiative-name>`. Operator action: `specify workspace push`. |
| `branch-pattern-mismatch` | A PR exists but its `headRefName` does not equal the resolved branch. The verb refuses to operate. |
| `failed` | Generic shell-out failure (`gh` missing, network error, merge conflict, …). See `detail`. |

Exit code is `0` only when every project lands on `merged`, `would-merge`, or `no-branch`. Any of `failed`, `failed-checks`, `pending-checks`, `closed`, or `branch-pattern-mismatch` flips the exit code to `1` so CI loops and the 2C umbrella skill can branch on the result.

**Safety guards (non-negotiable):**

- Branch-pattern guard refuses any PR whose `headRefName` ≠ `specify/<initiative-name>` exactly.
- Never `--admin`; never `--auto`; never overrides failing or pending checks.
- Failure on one project never aborts the batch — each project runs to its own classification.

**Prerequisites:** `gh` (GitHub CLI) authenticated against every registry remote.

## See also

- [Cross-Repo Initiatives](../../tutorials/cross-repo-initiative.md) -- tutorial for multi-repo workflows
- [Configuration Files](../configuration.md) -- registry.yaml and plan.yaml format
- [/spec:execute](../initiative-skills/execute.md) -- skill that drives workspace execution

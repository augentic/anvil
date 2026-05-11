# specify change

Manage the operator-authored change brief at `change.md` and close out a change once every per-project PR has merged.

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`create`](#specify-change-create) | Scaffold `change.md` from the canonical template at the start of a change. |
| [`show`](#specify-change-show) | Render the current brief (frontmatter + prose body) for tooling consumers and review. |
| [`finalize`](#specify-change-finalize) | Close out a fully-landed change: confirm every per-project PR merged, archive `plan.yaml` + brief + working dir. |

## Subcommands

### specify change create

Scaffold `change.md` with the frontmatter template.

```bash
specify change create <name>
```

Refuses to overwrite an existing brief — mirrors the `specify change plan create` posture for `plan.yaml`.

### specify change show

Render the brief content (frontmatter + prose body).

```bash
specify change show [--format json]
```

`--format json` emits the parsed frontmatter alongside the prose body for tooling consumers (e.g. `/change:plan`).

### specify change finalize

Close out a change once every plan entry is in a terminal state and every per-project PR has already been merged on its remote. This is the **canonical closure verb** for the platform-first loop: it verifies landing state, archives coordinator artifacts, and optionally removes clean workspace clones. It never merges, force-merges, approves, or otherwise mutates a pull request.

```bash
specify change finalize [--clean] [--dry-run]
```

The verb runs four guards in order. **All-or-nothing:** any guard failure refuses the run with a per-project status table and leaves the on-disk state untouched.

1. **Plan-presence guard.** `plan.yaml` must exist. Absent file refuses with `plan-not-found` — the canonical "change is already finalized" signal (the previous run swept the plan into `.specify/archive/plans/`).
2. **Plan terminal-state guard.** Every entry must be in `done`, `failed`, or `skipped` (the in-`Plan` mapping for `dropped`). Anything `pending`, `in-progress`, or `blocked` refuses with `non-terminal-entries-present`; the diagnostic names the offending entries and points the operator at `specify change plan status`.
3. **Per-project PR-state guard.** For each registry project, `gh pr view --json state,merged,headRefName,number,url` is run against the workspace clone. The PR, when present, must use the exact branch `specify/<change-name>`. Status mapping:

   | Status | Meaning | Passes? |
   |---|---|---|
   | `merged` | PR is `MERGED` on remote | yes |
   | `no-branch` | No PR on `specify/<change-name>` for this project | yes |
   | `unmerged` | PR is `OPEN` and must be operator-merged through the forge UI, `gh pr merge`, or the project's normal merge queue before finalize | no |
   | `closed` | PR was `CLOSED` without merging | no |
   | `branch-pattern-mismatch` | A PR exists but its `headRefName` is not `specify/<change-name>` | no |
   | `failed` | `gh` shell-out failed (network, missing binary, parse error) | no |

4. **Workspace-cleanliness guard.** `git status --porcelain` for each workspace clone must be empty. Dirty clones surface as status `dirty` and refuse — protecting uncommitted work from a subsequent `--clean` run.

When every guard passes, the verb runs `Plan::archive` programmatically: `plan.yaml`, `change.md`, and `.specify/plans/<name>/` move atomically into `.specify/archive/plans/<YYYYMMDD>-<name>/`. The archive write is preflighted (both destinations) so a collision returns an error before any file is touched.

#### `--clean`

Removes clean `.specify/workspace/<peer>/` clones after the archive completes. Symlink-mode projects (`url: .` or relative paths) are skipped — they point at source trees the operator owns separately. Dirty clones refuse finalize before any archive or cleanup happens. Without `--clean`, the clones stay on disk; they are cheap to refresh via `specify workspace sync` for the next change.

#### `--dry-run`

Observation-only: classifies every guard, prints the per-project status table, and stops. Never invokes `gh pr merge` and never moves files. Useful for checking "is this change ready to land?" before committing.

#### Output

Text mode prints the per-project status rows followed by a summary line and a final `Change <name> finalized.` (or `blocked: <reason>`). JSON output:

```json
{
  "envelope-version": 6,
  "change": "oauth-login",
  "finalized": true,
  "expected-branch": "specify/oauth-login",
  "projects": [
    {
      "name": "shop-backend",
      "status": "merged",
      "pr-number": 41,
      "url": "https://github.com/org/shop-backend/pull/41",
      "head-ref-name": "specify/oauth-login",
      "dirty": false
    },
    {
      "name": "shop-mobile",
      "status": "merged",
      "pr-number": 18,
      "url": "https://github.com/org/shop-mobile/pull/18",
      "head-ref-name": "specify/oauth-login",
      "dirty": false
    }
  ],
  "summary": {
    "merged": 2,
    "unmerged": 0,
    "closed": 0,
    "no-branch": 0,
    "branch-pattern-mismatch": 0,
    "dirty": 0,
    "failed": 0
  },
  "archived": "/.../shop-platform/.specify/archive/plans/oauth-login-20260428.yaml",
  "archived-plans-dir": "/.../shop-platform/.specify/archive/plans/oauth-login-20260428",
  "cleaned": ["shop-backend", "shop-mobile"]
}
```

Failure JSON keeps `finalized: false` and reports the blocking statuses in the per-project rows. Refused runs (plan absent, non-terminal entries, any per-project refusal) exit `1`; success exits `0`.

#### Composition with operator merge

`specify workspace push` stops at branch publication and PR creation/update. The operator lands each PR through the forge UI, `gh pr merge`, or the repository's normal merge queue. `specify change finalize` is the read-only confirmation and cleanup gate after those PRs have landed.

The old `specify workspace merge` automation has been removed. Operators land PRs through the forge, then use `specify change finalize` for read-only confirmation and cleanup.

#### Idempotency

`finalize` is idempotent across the canonical recovery path. If the first run refuses on an unmerged PR, the operator merges it outside Specify and re-runs `finalize` — the archive completes. After successful finalize, re-running returns `plan-not-found`, which is the explicit "already finalized" signal.

## See also

- [specify slice](slice.md) -- the per-slice CLI verbs that change-orchestration drives through the slice loop.
- [specify registry](registry.md) -- platform registry.
- [specify workspace](workspace.md) -- sync, status, push.

# specify initiative

Manage the operator-authored initiative brief at `.specify/initiative.md` and close out an initiative once every per-project PR has merged.

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`create`](#specify-initiative-create) | Scaffold `.specify/initiative.md` from the canonical template at the start of an initiative. |
| [`show`](#specify-initiative-show) | Render the current brief (frontmatter + prose body) for tooling consumers and review. |
| [`finalize`](#specify-initiative-finalize) | Close out a fully-landed initiative: confirm every per-project PR merged, archive `plan.yaml` + brief + working dir. |

## Subcommands

### specify initiative create

Scaffold `.specify/initiative.md` with the frontmatter template.

```bash
specify initiative create <name>
```

Refuses to overwrite an existing brief — mirrors the `specify plan create` posture for `plan.yaml`. (Renamed from the v1 `init` verb by RFC-9 §1F; see [Migrating CLI v1](../../explanation/migrating-cli-v1.md#v1x-renames).)

### specify initiative show

Render the brief content (frontmatter + prose body).

```bash
specify initiative show [--format json]
```

`--format json` emits the parsed frontmatter alongside the prose body for tooling consumers (e.g. `/spec:plan`).

### specify initiative finalize

Close out an initiative once every plan entry is in a terminal state and every per-project PR has merged on its remote (RFC-9 §4C). This is the **canonical closure verb** for the platform-first loop — it replaces the manual `specify plan archive` step with a defence-in-depth confirmation that the whole initiative has landed.

```bash
specify initiative finalize [--clean] [--dry-run]
```

The verb runs four guards in order. **All-or-nothing:** any guard failure refuses the run with a per-project status table and leaves the on-disk state untouched.

1. **Plan-presence guard.** `.specify/plan.yaml` must exist. Absent file refuses with `plan-not-found` — the canonical "initiative is already finalized" signal (the previous run swept the plan into `.specify/archive/plans/`).
2. **Plan terminal-state guard.** Every entry must be in `done`, `failed`, or `skipped` (the in-`Plan` mapping for `dropped`). Anything `pending`, `in-progress`, or `blocked` refuses with `non-terminal-entries-present`; the diagnostic names the offending entries and points the operator at `specify plan status`.
3. **Per-project PR-state guard.** For each registry project, `gh pr view --json state,merged,headRefName,number,url` is run against the workspace clone. Status mapping:

   | Status | Meaning | Passes? |
   |---|---|---|
   | `merged` | PR is `MERGED` on remote | yes |
   | `no-branch` | No PR on `specify/<initiative-name>` for this project | yes |
   | `unmerged` | PR is `OPEN` (not yet merged) | no |
   | `closed` | PR was `CLOSED` without merging | no |
   | `branch-pattern-mismatch` | A PR exists but its `headRefName` is not `specify/<initiative-name>` | no |
   | `failed` | `gh` shell-out failed (network, missing binary, parse error) | no |

4. **Workspace-cleanliness guard.** `git status --porcelain` for each workspace clone must be empty. Dirty clones surface as status `dirty` and refuse — protecting uncommitted work from a subsequent `--clean` run.

When every guard passes, the verb runs `Plan::archive` programmatically: `plan.yaml`, `.specify/initiative.md`, and `.specify/plans/<name>/` move atomically into `.specify/archive/plans/<YYYYMMDD>-<name>/`. The archive write is preflighted (both destinations) so a collision returns an error before any file is touched.

#### `--clean`

Removes `.specify/workspace/<peer>/` clones after the archive completes. Symlink-mode projects (`url: .` or relative paths) are skipped — they point at source trees the operator owns separately. Without `--clean`, the clones stay on disk; they are cheap to refresh via `specify workspace sync` for the next initiative.

#### `--dry-run`

Observation-only: classifies every guard, prints the per-project status table, and stops. Never invokes `gh pr merge` and never moves files. Useful for checking "is this initiative ready to land?" before committing.

#### Output

Text mode prints the per-project status rows followed by a summary line and a final `Initiative <name> finalized.` (or `blocked: <reason>`). JSON output:

```json
{
  "schema-version": 2,
  "initiative": "oauth-login",
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

#### Composition with `specify workspace merge`

Two valid operator paths:

- **Autonomous:** `specify workspace merge` (RFC-9 §4A) merges every PR with green CI; `specify initiative finalize` confirms the merges and archives. The 2C umbrella skill drives this path end-to-end.
- **Supervised:** the operator merges PRs by hand on the forge; `specify initiative finalize` confirms and archives.

#### Idempotency

`finalize` is idempotent across the canonical recovery path. If the first run refuses on an unmerged PR, the operator merges it manually (or via `workspace merge`) and re-runs `finalize` — the archive completes. After successful finalize, re-running returns `plan-not-found`, which is the explicit "already finalized" signal.

## See also

- [specify registry](registry.md) -- platform registry (top-level since the CLI cleanup; previously `specify initiative registry`).
- [specify workspace](workspace.md) -- workspace sync, status, push, and merge (moved from `specify initiative workspace` in RFC-3b).
- [Migrating CLI v1](../../explanation/migrating-cli-v1.md) -- rename map for the cleanup.

# Working across repos: executing

You finished [Working across repos: planning](cross-repo-change.md) with `plan.yaml` carrying three changes -- `oauth-login-contract`, `add-oauth-tokens`, `add-oauth-screens` -- all `pending`, ready to drive. The workspace clones are materialised under `.specify/workspace/`, the registry validates, and every entry has its `project:` resolved (or none, for the hub-level contract change). This tutorial drives that plan to PRs.

## Where you are in the cross-repo loop

The full loop is nine steps. This page covers steps **5-7**.

1. Initialise the platform hub (`specify init --hub`)
2. Register code projects (`specify registry add`)
3. Write the change brief (`specify change draft`)
4. Draft the plan (`/change:draft`)
   - *(seam — operator reviews `plan.yaml`; see [Reviewing the plan](reviewing-a-plan.md))*
5. **Inspect the workspace**
6. **Execute the plan** (`/change:execute loop`)
7. **Push branches and open PRs** (`specify workspace push`)
8. Operator merges the PRs
9. Finalize the change (`/change:finalize`)

Steps 1-4 live in [Working across repos: planning](cross-repo-change.md); the review seam in [Reviewing the plan](reviewing-a-plan.md); steps 8-9 in [Working across repos: landing](landing-a-change.md). `/change:finalize <name>` composes steps 7-9 into one operator action; this tutorial still shows step 7 as a discrete `specify workspace push` because the workspace push is the natural end of the executing half of the loop.

**Prerequisites:**

- Completed [Working across repos: planning](cross-repo-change.md) -- the hub is bootstrapped, the registry has both projects, the brief is authored, and `plan.yaml` lists three `pending` changes.
- You have reviewed the drafted plan (see [Reviewing the plan](reviewing-a-plan.md)). If you skipped the review, do it now -- the draft → review → execute seam is the design.
- Same toolchain as the planning tutorial: `specify` CLI on `PATH`, `gh` authenticated against your GitHub org.

## Contents

- [5. Inspect the workspace](#5-inspect-the-workspace)
- [6. Execute the plan](#6-execute-the-plan)
- [/change:execute — oauth-login](#changeexecute--oauth-login)
- [/change:execute — oauth-login — terminated](#changeexecute--oauth-login--terminated)
- [7. Push branches and PRs](#7-push-branches-and-prs)
- [Pause point](#pause-point)
- [Troubleshooting](#troubleshooting)
- [Verification](#verification)
- [Change shapes (preview)](#change-shapes-preview)
- [What you learned](#what-you-learned)
- [Cross-links](#cross-links)
- [Next](#next)

## 5. Inspect the workspace

`/change:draft` already ran `specify workspace sync` during the sync-workspace phase. Verify the resulting clones:

```bash
specify workspace status
```

<details>
<summary>Expected output</summary>

```text
shop-backend     git-clone     <40-char sha>     dirty: no     specify-tree: project.yaml
shop-mobile      git-clone     <40-char sha>     dirty: no     specify-tree: project.yaml
```

</details>

`specify workspace sync` is idempotent — re-run it between changes to refresh clones. Greenfield projects (remote does not yet exist) are bootstrapped in place via `git init` + `specify init`.

> **Tier-2 only.** `.specify/workspace/<peer>/` clones are durable; they outlive any single change. The legacy-source clones under `.specify/plans/<change>/analyze/<key>/` (tier-1) are a separate concern — read-only and ephemeral. See [Workspace tiers](../explanation/workspace-tiers.md) for the full contrast.

## 6. Execute the plan

Drive every change in dependency order:

```text
/change:execute loop
```

The driver:

1. Acquires the plan lock at `.specify/plan.lock` (one driver at a time).
2. Picks the next eligible slice via `specify plan next --format json`.
3. For multi-repo entries, resolves the `project` field against `registry.yaml`, materialises only the selected workspace slot if it is missing, and prepares `specify/oauth-login` before any phase writes. The contract slice has no `project` and runs against the hub itself.
4. Runs `/spec:define` -> `/spec:build` -> `/spec:merge` for the slice.
5. After a routed merge succeeds, verifies the `/spec:merge` baseline commit boundary (`.specify/specs/` plus `.specify/archive/`) and commits non-baseline residue as `specify: residue <slice-name>`.
6. Restores CWD to the hub root and transitions the plan entry to `done`/`failed`/`blocked`.
7. Repeats from step 2 until `specify plan next` reports `all-done` or `stuck`.

After producer contracts change, run `specify compatibility check --change oauth-login --report-only` when you want a classified consumer-impact report against workspace views.

<details>
<summary>Expected loop transcript (abbreviated)</summary>

```text
## /change:execute — oauth-login

### Change: oauth-login
Progress: done 0, in-progress 0, pending 3, blocked 0, failed 0, skipped 0 (total 3)

---

Self-heal: no in-progress entries found.

# specify plan next --format json → { "next": "oauth-login-contract", "project": null, "description": "...", "sources": null }
# specify plan transition oauth-login-contract in-progress

### Processing: oauth-login-contract (greenfield)

Step 1/3: define
  Artifacts: proposal.md, contracts/, design.md, tasks.md ✓
Step 2/3: build
  Tasks: 2/2 complete ✓
Step 3/3: merge
  Baseline updated: contracts/http/oauth-login.yaml ✓
  Status: done

---

# specify plan next --format json → { "next": "add-oauth-tokens", "project": "shop-backend", ... }
# registry selector: shop-backend → git@github.com:org/shop-backend.git
# specify workspace status shop-backend --format json → git-clone, branch=main, dirty=false
# specify workspace prepare-branch shop-backend --change oauth-login --format json
#   → prepared=true branch=specify/oauth-login local-branch=created remote-branch=absent
# CWD saved: /…/shop-platform
# specify plan transition add-oauth-tokens in-progress

Routing: add-oauth-tokens → shop-backend (.specify/workspace/shop-backend/)
Workspace: shop-backend prepared on specify/oauth-login

### Processing: add-oauth-tokens (greenfield)

Step 1/3: define ✓
Step 2/3: build
  Tasks: 5/5 complete ✓
Step 3/3: merge
  specify: merge add-oauth-tokens
  Baseline committed: git add .specify/specs/ .specify/archive/ \
      && git commit -m "specify: merge add-oauth-tokens"
  Baseline updated: .specify/specs/oauth-tokens/spec.md ✓
  Residue committed: specify: residue add-oauth-tokens

# CWD restored: /…/shop-platform
# specify plan transition add-oauth-tokens done
  Status: done

---

# specify plan next --format json → { "next": "add-oauth-screens", "project": "shop-mobile", ... }

Routing: add-oauth-screens → shop-mobile (.specify/workspace/shop-mobile/)

### Processing: add-oauth-screens (greenfield)

Step 1/3: define ✓
Step 2/3: build ✓
Step 3/3: merge ✓
  Status: done

---

## /change:execute — oauth-login — terminated

### Final state
Progress: done 3, in-progress 0, pending 0, blocked 0, failed 0, skipped 0 (total 3)

Completion: all-done

Next action: Change complete. Run specify workspace push to publish prepared specify/oauth-login branches and create or update PRs. Merge those PRs through the forge UI or gh pr merge, then close out via specify change finalize.
```

</details>

Each implementation slice leaves two local commits in its workspace clone: `/spec:merge` commits only `.specify/specs/` and `.specify/archive/` as `specify: merge <slice-name>`, then `/change:execute` commits project-output residue as `specify: residue <slice-name>`. This is what `specify workspace push` ships in Step 7.

> **Failure handling.** If a change fails mid-loop, `/change:execute` invokes `/spec:drop`, transitions the entry to `failed` (verbatim `outcome.summary` as `reason`), and continues. Subsequent changes that depend on the failed one stay `pending` until you `specify plan transition <pred> pending` to retry, or `specify plan transition <entry> skipped reason …` to drop the dependency leaf. See `/change:execute`'s [§Output format → Failure transcript](../../plugins/change/skills/execute/SKILL.md) for the recovery prompt.

## 7. Push branches and PRs

After execution, each workspace clone is already on `specify/oauth-login` with local commits ahead of the remote branch. Publish them:

```bash
specify workspace push
```

Per project, the verb:

1. Verifies the clone is clean and already checked out to `specify/oauth-login`; any other checkout is reported as `no-branch`.
2. Runs `git push --force-with-lease -u origin specify/oauth-login`.
3. For greenfield remotes, creates the repo via `gh repo create`.
4. Creates or updates a PR for the branch via `gh pr create` when needed.

`workspace push` is transport-only: it does not create the change branch on the fly, does not create commits, does not push default branches, and never merges PRs.

<details>
<summary>Expected output</summary>

```text
specify: workspace push — oauth-login

  shop-backend   pushed       specify/oauth-login   PR #41
  shop-mobile    pushed       specify/oauth-login   PR #18

2 pushed, 0 created, 0 up-to-date. 0 failed.
```

</details>

<details>
<summary>JSON output (<code>--format json</code>)</summary>

```json
{
  "projects": [
    { "name": "shop-backend", "status": "pushed", "branch": "specify/oauth-login", "pr": 41 },
    { "name": "shop-mobile",  "status": "pushed", "branch": "specify/oauth-login", "pr": 18 }
  ]
}
```

</details>

For greenfield projects (remote did not exist before this run), the per-project status flips to `created` and `gh repo create` runs first. Use `--dry-run` to classify each clone's push status without performing any writes — the verb adds `would-` prefixes to the action statuses. See [`specify workspace push`](../reference/cli/workspace.md#specify-workspace-push) for the full status vocabulary.

## Pause point

Two PRs are now open against `org/shop-backend` and `org/shop-mobile`, both on the `specify/oauth-login` branch. The `oauth-login` plan still lives at `plan.yaml` with every entry `done`. The hub is in the canonical "ready to land" state.

[**Continue to Working across repos: landing**](landing-a-change.md) for the final stage: merging the PRs (operator action) and running `/change:finalize <name>` to observe PR state and archive the plan. The landing tutorial also covers the three change shapes (migrate-legacy / new-feature / update-existing).

If you stop here, the cross-repo work is shipped but unmerged. The PRs sit on the forge until reviewed; nothing is blocking. You can resume landing at any time -- `/change:finalize` is idempotent and re-reads `plan.yaml` and PR state on every invocation. Merge the PRs through the forge UI or `gh pr merge` first.

## Troubleshooting

If `/change:execute loop` exits with `Completion: stuck` or any single invocation reports `reason: stuck`, the first triage step is `specify plan validate`:

```bash
specify plan validate
```

`doctor` is a strict superset of `specify plan validate` — it runs every check `validate` runs, then layers four health diagnostics on top:

| Code | Severity | Recovery |
|------|----------|----------|
| `cycle-in-depends-on` | error | Break the cycle: `specify plan amend <name> --depends-on …`. |
| `orphan-source-key` | warning | Reference the key from an entry's `sources:` (`specify plan amend <name> --sources …`) or remove it from the top-level map. |
| `stale-workspace-clone` | warning | Refresh: `specify workspace sync`. |
| `unreachable-entry` | error | `specify plan transition <pred> pending` after fixing the predecessor, or `specify plan transition <entry> skipped --reason "…"` to drop the leaf. |

See [`specify plan validate`](../reference/cli/plan.md#specify-plan-validate) for the full diagnostic table and JSON shape.

Other common issues:

- **`Error::DriverBusy { pid }`** — another `/change:execute` is holding `.specify/plan.lock`. If it is dead, `specify plan lock release --pid <pid>` reclaims the stamp; otherwise wait for the live driver.
- **`hub-cannot-be-project`** — a registry entry has `url: .` on a hub. Either remove the entry (`specify registry remove <name>`) or convert the hub to a platform-as-project shape by removing `.specify/` and re-running `specify init <adapter>` without `--hub`.
- **Breaking compatibility findings** — run `specify compatibility check --change <name> --report-only` to inspect producer-to-consumer contract deltas, then see [Resolve Cross-Project Compatibility Findings](../how-to/resolve-cross-project-contract-warnings.md).

## Verification

A reviewer (or an operator stepping through this tutorial as an integration test) can grep these expected outputs at each step. The first four rows recap the planning tutorial's invariants; rows for Steps 5-7 are the executing-tutorial gates.

| After | Command | Expect |
|---|---|---|
| Step 1 | `cat .specify/project.yaml` | A line containing `hub: true` and **no** `adapter:` line. |
| Step 1 | `ls .specify/` | `project.yaml`, `context.lock`. **No** `slices/`, `specs/`, or `.cache/` (phase pipelines disabled). |
| Step 1 | `test -f AGENTS.md && specify context check` | Exit 0. |
| Step 2 | `specify registry validate` | Exit 0; no diagnostics. |
| Step 2 | `specify registry show` | `version: 1` and two `projects[]` entries with descriptions. |
| Step 3 | `head -10 change.md` | Frontmatter `name: oauth-login` and the documentation `inputs:` entry. |
| Step 4 | `specify plan validate` | Exit 0; no error-level findings. |
| Step 4 | `specify plan status` | Three entries; the two implementation entries carry `project: shop-backend` / `project: shop-mobile`. |
| Step 5 | `specify workspace status` | Both projects show `git-clone` materialisation, `dirty: no`. |
| Step 6 | `specify plan status` | All three changes `done`; `Summary: 0 pending, 0 in-progress, 3 done`. |
| Step 7 | `gh pr list -R org/shop-backend --head specify/oauth-login` | Exactly one open PR. |
| Step 7 | `gh pr list -R org/shop-mobile --head specify/oauth-login` | Exactly one open PR. |

Any deviation is a blocker. File the failing transcript against this tutorial; the gap is in the implementation, not the design. The Steps 8-9 verification (PR `MERGED` on remote, plan archived, re-run `plan-not-found`) lives in [Working across repos: landing](landing-a-change.md#verification).

## Change shapes (preview)

The cross-repo loop above is shape-agnostic. The same three-skill sequence (`/change:draft` → review → `/change:execute loop` → `/change:finalize`) drives three change shapes -- `migrate-legacy`, `new-feature`, `update-existing` -- with the only difference being the inputs to `/change:draft`. The walkthrough across the planning and executing tutorials is the **new-feature** shape (sources are documentation only); the other two arrive in [Working across repos: landing](landing-a-change.md#change-shapes).

## What you learned

- `/change:execute loop` `chdir`s into each workspace clone, runs define-build-merge, transitions the plan entry, and routes back. Multi-repo CWD routing is invisible to the phase skills.
- The contract slice (no `project`) runs against the hub itself; routed implementation slices run inside `.specify/workspace/<peer>/` and leave two commits per slice (baseline merge + residue) ready to push.
- `specify workspace push` ships prepared `specify/<change-name>` branches as PRs without creating branches, committing residue, pushing default branches, or merging PRs.
- `specify plan validate` is the first triage step when a loop ends `stuck` -- the base shape rules plus four health diagnostics (cycle, orphan source key, stale clone, unreachable entry).

## Cross-links

- [Platform repo topologies](../explanation/platform-repo.md) -- registry-only hub vs platform-as-project, the validation invariant, and the on-disk shape of each.
- [Workspace tiers](../explanation/workspace-tiers.md) -- the legacy-source vs registered-project clone distinction the loop relies on.
- [`/change:draft`](../reference/change-skills/draft.md) -- plan authoring skill; ends at the operator review seam.
- [`/change:execute`](../reference/change-skills/execute.md) -- plan driver.
- [`specify compatibility`](../reference/cli/compatibility.md) -- consumer-impact contract report.
- [`specify init`](../reference/cli/init.md) -- the `--hub` flag.
- [`specify registry`](../reference/cli/registry.md) -- `add` / `remove` / `show` / `validate`.
- [`specify workspace`](../reference/cli/workspace.md) -- `sync` / `status` / `push`.
- [`specify plan`](../reference/cli/plan.md) -- `create` / `add` / `amend` / `next` / `doctor` / `archive` / `lock`.

## Next

[Working across repos: landing](landing-a-change.md) -- merge the PRs you just pushed, run `/change:finalize <name>` to archive the change, and walk through the three change shapes.

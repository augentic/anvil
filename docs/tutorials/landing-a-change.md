# Working across repos: landing

This tutorial picks up where [Working across repos: planning](cross-repo-change.md) and [Working across repos: executing](cross-repo-execute.md) leave off: two PRs are open against the registered projects, and the `oauth-login` plan has every entry `done`. We now exercise the **landing half** of the cross-repo loop -- merging the PRs through the forge and archiving the plan via `/change:finalize` -- and round it out with the three change shapes (`migrate-legacy`, `new-feature`, `update-existing`).

Use this page when you want the worked scenario and full end-to-end narrative. If you already have PRs open and just need the operator checklist, use [Land a Change](../how-to/land-a-change.md).

## Where you are in the cross-repo loop

The full loop is nine steps. This page covers steps **8-9**.

1. Initialise the platform hub (`specify init --hub`)
2. Register code projects (`specify registry add`)
3. Write the change brief (`specify change draft`)
4. Draft the plan (`/change:draft`)
   - *(seam — operator reviews `plan.yaml`; see [Reviewing the plan](reviewing-a-plan.md))*
5. Inspect the workspace
6. Execute the plan (`/change:execute loop`)
7. Push branches and open PRs (`specify workspace push`)
8. **Operator merges the PRs**
9. **Finalize the change** (`/change:finalize`)

Steps 1-4 live in [Working across repos: planning](cross-repo-change.md); the review seam in [Reviewing the plan](reviewing-a-plan.md); Steps 5-7 in [Working across repos: executing](cross-repo-execute.md). Together, the four pages walk the full loop and can be replayed against the live CLI as an integration test -- any deviation is a blocker.

`/change:finalize <name>` composes steps 7-9 (push + PR observation + archive) into one operator action. This tutorial walks through the underlying verbs first -- `specify workspace push` (already covered upstream), the operator PR merge, and `specify change finalize` -- then shows how `/change:finalize` wraps the lot.

> **Choosing your topology.** This tutorial extends the platform-hub flow from the planning and executing tutorials. The Steps 8-9 verbs work identically against the platform-as-project shape (`url: .` in the registry), but the workspace clones in that case are symlinks to the initiating repo rather than separate clones.

**Prerequisites:**

- Completed [Working across repos: planning](cross-repo-change.md) and [Working across repos: executing](cross-repo-execute.md) up to and including Step 7. Two PRs are open on `specify/oauth-login` against `org/shop-backend` and `org/shop-mobile`.
- [`gh`](https://cli.github.com/) installed and authenticated against your GitHub org. `/change:finalize` and `specify change finalize` shell out to `gh` to confirm PR state; PR merge itself is an operator action through the forge UI or an explicit `gh pr merge`.

## Contents

- [State on entry](#state-on-entry)
- [8. Land the PRs](#8-land-the-prs)
- [9. Finalize the change](#9-finalize-the-change)
- [Verification](#verification)
- [Change shapes](#change-shapes)
- [What you learned](#what-you-learned)
- [Cross-links](#cross-links)
- [Next](#next)

## State on entry

Recap the on-disk state we are starting from:

| Surface | Expected |
|---------|----------|
| `specify plan status` | Three entries; `Summary: 0 pending, 0 in-progress, 3 done` |
| `gh pr list -R org/shop-backend --head specify/oauth-login` | Exactly one open PR |
| `gh pr list -R org/shop-mobile --head specify/oauth-login` | Exactly one open PR |
| `git status` (in each workspace clone) | Clean (the auto-commits from `/change:execute loop` are pushed, not staged) |

## 8. Land the PRs

Once CI is green on each PR, merge them through the forge UI or explicitly with `gh pr merge`:

```bash
gh pr checks 41 -R org/shop-backend
gh pr merge 41 -R org/shop-backend --squash

gh pr checks 18 -R org/shop-mobile
gh pr merge 18 -R org/shop-mobile --squash
```

Specify does not merge PRs automatically. The older `specify workspace merge` command has been removed; operators merge through forge UI / `gh pr merge`, then run `specify change finalize`.

<details>
<summary>Expected verification after merge</summary>

```text
gh pr view 41 -R org/shop-backend --json state,merged
{"state":"MERGED","merged":true}

gh pr view 18 -R org/shop-mobile --json state,merged
{"state":"MERGED","merged":true}
```

</details>

`change finalize` (Step 9) does not care **how** the PRs got merged, only that every project's PR is `MERGED` on remote and every workspace clone is clean.

## 9. Finalize the change

Once every PR is merged, close the change with the canonical closure verb:

```bash
specify change finalize
```

`specify change finalize` confirms the whole change is landed and atomically sweeps local plan state into the archive. It runs four guards in order before any move:

1. **Plan-presence:** `plan.yaml` exists.
2. **Plan terminal-state:** every entry is `done` / `failed` / `skipped`.
3. **Per-project PR-state:** every registered project's PR on `specify/oauth-login` is `MERGED` on its remote (or has no PR at all). Refuses on `unmerged` / `closed` / `branch-pattern-mismatch` / `failed`.
4. **Workspace-cleanliness:** `git status --porcelain` is empty for every workspace clone.

Any guard failure refuses with a per-project status table and leaves the on-disk state untouched. When all guards pass, `plan.yaml`, `change.md`, and `.specify/plans/oauth-login/` move atomically into `.specify/archive/plans/<YYYYMMDD>-oauth-login/`.

<details>
<summary>Expected output (all PRs merged, clean clones)</summary>

```text
specify: change finalize — oauth-login (specify/oauth-login)

  shop-backend         merged                   PR #41     https://github.com/org/shop-backend/pull/41
  shop-mobile          merged                   PR #18     https://github.com/org/shop-mobile/pull/18

2 merged, 0 unmerged, 0 closed, 0 no-branch, 0 branch-pattern-mismatch, 0 dirty, 0 failed.

Change `oauth-login` finalized.
  archived plan: /…/shop-platform/.specify/archive/plans/oauth-login-20260428.yaml
  archived dir:  /…/shop-platform/.specify/archive/plans/oauth-login-20260428
```

</details>

The two workspace clones stay on disk under `.specify/workspace/` -- they are the staging area for the next change. To prune them at the same time:

```bash
specify change finalize --clean
```

`--clean` removes `.specify/workspace/<peer>/` for every non-symlink registered project after the archive completes. Refused when any clone has a dirty working tree; the diagnostic warns that `--clean` would drop the uncommitted changes.

Use `--dry-run` to preview the guard table without writing anything -- useful for verifying readiness before you commit. `specify change finalize` is **idempotent**: re-running it after manually clearing a refused guard (e.g. merging the last PR by hand) completes the archive on the second invocation. Re-running after a successful finalize returns `plan-not-found`, the explicit "already finalized" signal.

> **`/change:finalize <name>` wraps steps 7-9.** Rather than running `specify workspace push`, watching PRs, and then `specify change finalize` by hand, invoke the third skill -- `/change:finalize oauth-login` -- which composes the three CLI verbs. It pushes branches, observes PR state via `gh pr list` (halting on any non-`MERGED` PR with the URL), and runs `specify change finalize` once every PR is merged. The skill is **idempotent**: re-run it after merging an open PR externally and it picks up where it stopped. It never merges PRs itself. See the [`/change:finalize` reference](../reference/change-skills/index.md) for the Critical Path and halt semantics.

## Verification

Continuing from the [Working across repos: executing](cross-repo-execute.md#verification) verification table, Steps 8-9 produce these expected outputs:

| After | Command | Expect |
|---|---|---|
| Step 8 | `gh pr view <pr> -R org/shop-backend --json state,merged` | `{"state":"MERGED","merged":true}`. |
| Step 9 | `ls .specify/archive/plans/` | A `oauth-login-<YYYYMMDD>.yaml` plan file plus a `oauth-login-<YYYYMMDD>/` directory holding `change.md` and the `plans/oauth-login/` authoring trail. |
| Step 9 | `ls plan.yaml` | `No such file or directory` -- the plan moved to the archive. |
| Step 9 | `specify change finalize` (re-run) | Exits `1` with `error: plan-not-found` -- the canonical "already finalized" signal. |

Any deviation is a blocker. File the failing transcript against this tutorial; the gap is in the implementation, not the design.

## Change shapes

The cross-repo loop is shape-agnostic. The same three-skill sequence drives three change shapes -- `migrate-legacy`, `new-feature`, and `update-existing` -- with the only difference being the inputs you pass to `/change:draft`. Each variant below shows the canonical three-skill invocation against the same hub.

The recommended shape of every variant is the same:

```text
/change:draft <name> [shape-specific inputs]
# review plan.yaml — `specify plan status`, edit with `specify plan amend` if needed
/change:execute loop
/change:finalize <name>
```

If `/change:finalize` halts (a PR is not yet `MERGED`, the workspace is dirty, etc.), fix the cause and re-run it; the skill re-reads `plan.yaml` and PR state on every invocation.

### Variant: migrate-legacy

Sources arrive via `--source <key>=<git-url-or-path>`. `/change:analyze` (inside `/change:draft`) clones each source into `.specify/plans/<change>/analyze/<key>/` (the [tier-1 workspace](../explanation/workspace-tiers.md#the-two-tiers)) for shallow capability inventory; deep `/spec:extract` runs at define time per slice. Targets are existing or newly-minted registered projects.

Run against an empty hub:

```text
/change:draft migrate-foo \
    shape migrate-legacy \
    source monolith=git@github.com:org/legacy-foo.git
# review plan.yaml
/change:execute loop
# merge the two PRs through the forge UI or `gh pr merge`
/change:finalize migrate-foo
```

What each skill does:

1. **`/change:draft`.** Scaffolds `change.md` (with the legacy monolith as a `legacy-code` input) and `plan.yaml` via `specify change draft migrate-foo --source monolith=…`. Empty registry + `migrate-legacy` shape -> hand off to the greenfield registry-proposal path; the draft brief proposes a two-project topology (`foo-backend` + `foo-mobile`), shells `specify registry add` x 2 and `specify workspace sync` once, then decomposes into one cross-project contract change plus one implementation slice per project. Assignment routes the implementation slices. Final gate: `specify plan validate`.
2. **Operator review.** Confirm the proposed topology and routing. See [Reviewing the plan](reviewing-a-plan.md).
3. **`/change:execute loop`.** Drives all three slices to `done` (the contract slice runs against the hub; the two implementation slices run inside their workspace clones).
4. **`/change:finalize`.** Pushes branches, halts on the two non-`MERGED` PRs and prints their URLs. After the operator merges both PRs through the forge UI or `gh pr merge`, re-run `/change:finalize migrate-foo`; it observes the merged state and runs `specify change finalize` to archive the plan.

Underlying CLI verb sequence: `specify change draft` -> `specify registry validate` -> `specify registry add` x 2 -> `specify workspace sync` -> `specify plan add` x 3 -> `specify plan amend --project` x 2 -> `specify plan validate` -> `/change:execute loop` -> `specify workspace push` -> operator PR merge -> `specify change finalize`.

### Variant: new-feature

Sources arrive via `--from <docs>` only (or via `change.md:inputs`). Targets are existing registered projects, possibly with new ones spawned at assignment time via the registry-proposal sub-step.

Run against the populated hub from [Working across repos: planning](cross-repo-change.md) Steps 1-3 (or your own equivalent):

```text
/change:draft dark-mode \
    shape new-feature \
    from ./docs/dark-mode-spec.md
# review plan.yaml
/change:execute loop
# merge the two PRs through the forge UI or `gh pr merge`
/change:finalize dark-mode
```

**The walkthrough across the planning, executing, and landing tutorials is this shape.** The three skills together drive the nine-step flow, with two deliberate operator pauses: the plan review between draft and execute, and the PR merge between execute and finalize. `/change:finalize` halts on the second pause, naming each open PR with its URL; re-running after merge resumes at `specify change finalize`.

Underlying CLI verb sequence (draft + execute): `specify change draft` -> `specify registry validate` -> (multi-source decomposition into the draft pipeline) -> `specify workspace sync` -> `specify plan add` x 3 -> `specify plan amend --project` x 2 -> `specify plan validate` -> `/change:execute loop`. No registry mutation -- both projects exist before the run.

Underlying CLI verb sequence (finalize, run 1, halts on open PRs): `specify workspace push` -> `gh pr list` (read-only).

Underlying CLI verb sequence (finalize, run 2, after the operator merges PRs by hand): `specify workspace push` (reports `up-to-date`) -> `gh pr list` -> `specify change finalize`.

### Variant: update-existing

No `--from` and no `--source` -- sources are unused. Targets are existing registered projects; baseline accumulation in `.specify/workspace/<peer>/specs/` is the dominant signal during planning.

Run against the same populated hub:

```text
/change:draft polish-pass shape update-existing
# review plan.yaml
/change:execute loop
# merge the two PRs through the forge UI or `gh pr merge`
/change:finalize polish-pass
```

Pre-flight forbids `--from`, `--against`, and `--source` under this shape; supplying any is a hard exit.

What each skill does:

1. **`/change:draft`.** `change.md` is scaffolded with `inputs: []`; the operator writes one paragraph naming the capabilities being polished. Multi-project registry; descriptions complete, no mutation. Discovery falls back to baseline accumulation in `.specify/workspace/<peer>/specs/` because the input set is empty. Propose surfaces two slices (one per project, **no contract change** -- the polish does not change the API surface). Assignment routes each slice to its existing project.
2. **Operator review.**
3. **`/change:execute loop`.** Both slices drive to `done`.
4. **`/change:finalize`.** Pushes branches, halts on the two open PRs. After the operator merges them, re-run to archive.

Underlying CLI verb sequence: `specify change draft` -> `specify registry validate` -> `specify workspace sync` -> `specify plan add` x 2 -> `specify plan amend --project` x 2 -> `specify plan validate` -> `/change:execute loop` -> `specify workspace push` -> operator PR merge -> `specify change finalize`.

### Dropping down a layer

Each step in every shape above is a shell-out one of the three skills runs verbatim. Operators can invoke the underlying CLI verbs directly at any point -- the skills add idempotent re-entry and a uniform operator-facing rhythm, but no behaviour beyond what the CLI verbs already provide.

## What you learned

- Specify opens PRs but does not merge them. Landing is an explicit operator action through the forge UI or `gh pr merge`; `specify change finalize` only verifies that the PRs are already `MERGED`.
- `specify change finalize` is the canonical closure verb: four guards in order (plan-presence, terminal-state, PR-state, workspace-cleanliness) before atomically archiving `plan.yaml`, `change.md`, and `.specify/plans/<name>/` into `.specify/archive/plans/<YYYYMMDD>-<name>/`.
- `--clean` prunes `.specify/workspace/<peer>/` after the archive completes; `--dry-run` previews the guard table without writing.
- `specify change finalize` is idempotent: re-running after a refused guard completes the archive on the second invocation; re-running after a successful finalize returns `plan-not-found` (the "already finalized" signal).
- `/change:finalize <name>` is the operator-facing wrapper -- one skill invocation that runs `specify workspace push`, observes PR state, then runs `specify change finalize`. It halts on any non-`MERGED` PR and is idempotent on re-entry.
- The same three-skill sequence (`/change:draft → /change:execute → /change:finalize`) closes out all three change shapes (`migrate-legacy`, `new-feature`, `update-existing`); only the inputs to `/change:draft` differ.

## Cross-links

- [`/change:finalize`](../reference/change-skills/index.md) -- closing skill reference; Critical Path, halts, re-entry.
- [Land a change](../how-to/land-a-change.md) -- focused how-to on autonomous vs supervised landing.
- [`specify workspace`](../reference/cli/workspace.md) -- workspace sync, status, and push.
- [`specify change finalize`](../reference/cli/change.md#specify-change-finalize) -- CLI reference, the four guards, JSON v2 envelope.
- [Change landing issues](../how-to/troubleshooting/change-landing.md) -- `branch-pattern-mismatch`, `plan-not-found`, dirty clones.
- [Drop down a layer](../how-to/drop-down-a-layer.md) -- the manual CLI-verb sequence underneath each skill.

## Next

[Legacy Migration at Scale](legacy-migration-at-scale.md) -- decompose a large monolith across multiple target repos using the analyze/extract split.

# Landing a Change

This tutorial picks up where [Cross-Repo Changes](cross-repo-change.md) leaves off: two PRs are open against the registered projects, and the `oauth-login` plan has every entry `done`. We now exercise the **landing half** of the platform-first loop -- merging the PRs through the forge and archiving the plan -- and round it out with the `/change:plan <name> orchestrate` umbrella that drives each of the three change shapes.

Use this page when you want the worked scenario and full end-to-end narrative. If you already have PRs open and just need the operator checklist, use [Land a Change](../how-to/land-a-change.md).

It exercises Steps 8-9 of the RFC-9 §1C critical path:

8. Operator PR merge -- review and merge each PR through the forge UI or `gh pr merge`
9. `specify change finalize` -- confirm landing and archive (RFC-9 §4C)

Together with [Cross-Repo Changes](cross-repo-change.md), the page-pair walks the full Steps 1-9 path. Both halves can be replayed against the live CLI as an integration test (per RFC-9 §1C, any deviation is a blocker).

> **Choosing your topology.** This tutorial extends the platform-hub flow from [Cross-Repo Changes](cross-repo-change.md). The Steps 8-9 verbs work identically against the platform-as-project shape (`url: .` in the registry), but the workspace clones in that case are symlinks to the initiating repo rather than separate clones.

**Prerequisites:**

- Completed [Cross-Repo Changes](cross-repo-change.md) up to and including Step 7. Two PRs are open on `specify/oauth-login` against `org/shop-backend` and `org/shop-mobile`.
- [`gh`](https://cli.github.com/) installed and authenticated against your GitHub org. `specify change finalize` shells out to `gh` to confirm PR state; PR merge itself is an operator action through the forge UI or an explicit `gh pr merge`.

## State on entry

Recap the on-disk state we are starting from:

| Surface | Expected |
|---------|----------|
| `specify change plan status` | Three entries; `Summary: 0 pending, 0 in-progress, 3 done` |
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

Specify does not merge PRs automatically. The pre-RFC-14 `specify workspace merge` command has been removed; operators merge through forge UI / `gh pr merge`, then run `specify change finalize`.

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

`finalize` confirms the whole change is landed and atomically sweeps local plan state into the archive (RFC-9 §4C). It runs four guards in order before any move:

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

Use `--dry-run` to preview the guard table without writing anything -- useful for verifying readiness before you commit. `finalize` is **idempotent**: re-running it after manually clearing a refused guard (e.g. merging the last PR by hand) completes the archive on the second invocation. Re-running after a successful finalize returns `plan-not-found`, the explicit "already finalized" signal.

> **One-shot variant -- `/change:plan <name> orchestrate` (RFC-9 §2C).** The Layer 4 umbrella mode composes the automated half of the loop: brief -> registry validate -> plan -> execute -> push. It then lists the opened PRs and stops. After the operator merges those PRs through the forge UI or `gh pr merge`, re-running the umbrella resumes at `specify change finalize`. The three subsections below show the umbrella driving each of the three change shapes against the same hub. See [`/change:plan <name> orchestrate`](../reference/change-skills/change.md) for the full algorithm, halt semantics, and re-entry rules.

## Verification

Continuing from the [Cross-Repo Changes](cross-repo-change.md#verification) verification table, Steps 8-9 produce these expected outputs:

| After | Command | Expect |
|---|---|---|
| Step 8 | `gh pr view <pr> -R org/shop-backend --json state,merged` | `{"state":"MERGED","merged":true}`. |
| Step 9 | `ls .specify/archive/plans/` | A `oauth-login-<YYYYMMDD>.yaml` plan file plus a `oauth-login-<YYYYMMDD>/` directory holding `change.md` and the `plans/oauth-login/` authoring trail. |
| Step 9 | `ls plan.yaml` | `No such file or directory` -- the plan moved to the archive. |
| Step 9 | `specify change finalize` (re-run) | Exits `1` with `error: plan-not-found` -- the canonical "already finalized" signal. |

Any deviation is a blocker. File the failing transcript against this tutorial; per RFC-9 §1C the gap is in the implementation, not the design.

## Change shapes

The platform-first loop is shape-agnostic. The same Steps 1-9 drive three change shapes: `migrate-legacy`, `new-feature`, and `update-existing`. Only the inputs to Step 4 (Plan) differ. Each shape is also drivable via the Layer 4 umbrella mode `/change:plan <name> orchestrate`. The transcripts below show each shape from the umbrella's perspective; the manual fallback for every step is the same Layer 1 verb the umbrella shells out to (see [Drop down a layer](../how-to/drop-down-a-layer.md#from-layer-4-to-layer-3-skip-the-umbrella) for the exact verb sequence).

### Variant: migrate-legacy

Sources arrive via `--source <key>=<git-url-or-path>`. `/spec:analyze` clones each source into `.specify/plans/<change>/analyze/<key>/` (the [tier-1 workspace](../explanation/workspace-tiers.md#the-two-tiers)) for shallow capability inventory; deep `/spec:extract` runs at define time per slice. Targets are existing or newly-minted registered projects.

Run against an empty hub:

```text
/change:plan <name> orchestrate migrate-foo \
    shape migrate-legacy \
    source monolith=git@github.com:org/legacy-foo.git
```

The umbrella runs through PR creation, stops for operator merge, then finalizes on re-entry:

1. **Brief.** `specify change create migrate-foo` scaffolds `change.md`; the operator confirms a default body listing the legacy monolith as a `legacy-code` input.
2. **Registry.** Empty + `--shape migrate-legacy` -> hand off to the 2B greenfield path inside `/change:plan`.
3. **Plan.** `/change:plan` runs discovery against the cloned monolith, proposes a two-project topology (`foo-backend` + `foo-mobile`), shells `specify registry add` x 2 and `specify workspace sync` once, then propose decomposes into one cross-project contract change plus one implementation slice per project. Assignment routes the implementation slices.
4. **Execute.** `/change:execute loop` drives all three changes to `done` (contract change runs against the hub; the two implementation changes run inside their workspace clones).
5. **Push.** `specify workspace push` opens two PRs.
6. **Land.** The operator reviews and merges both PRs through the forge UI or `gh pr merge`.
7. **Finalize.** Re-run `/change:plan <name> orchestrate migrate-foo`; it observes merged PRs and runs `specify change finalize`.

Verb sequence: `specify change create` -> `specify registry validate` -> `/change:plan` -> `specify change plan create` -> `specify registry add` x 2 -> `specify workspace sync` -> `specify change plan add` x 3 -> `specify change plan amend --project` x 2 -> `specify change plan validate` -> `/change:execute loop` -> `specify workspace push` -> operator PR merge -> `specify change finalize`. Full transcript and on-disk shapes: [`fixtures/migrate-legacy/`](../../plugins/change/skills/plan/fixtures/migrate-legacy/).

### Variant: new-feature

Sources arrive via `--from <docs>` only (or via `change.md:inputs`). Targets are existing registered projects, possibly with new ones spawned at assignment time via the registry-proposal sub-step (RFC-9 §2B).

Run against the populated hub from [Cross-Repo Changes](cross-repo-change.md) Steps 1-3 (or your own equivalent):

```text
/change:plan <name> orchestrate dark-mode \
    shape new-feature \
    from ./docs/dark-mode-spec.md
```

**The walkthrough across [Cross-Repo Changes](cross-repo-change.md) and this tutorial is this shape.** The umbrella drives the same nine-step flow, with one deliberate pause: after `specify workspace push`, it lists the open PRs and **stops**. The operator merges PRs through the forge UI or `gh pr merge`, then re-runs the umbrella to finalize. Re-entry inspects on-disk state -- brief present, plan terminal, every PR `MERGED` on remote -- and skips straight to `specify change finalize`.

Verb sequence (run 1, halts at step 6): `specify change create` -> `specify registry validate` -> `/change:plan <name> from ./docs/dark-mode-spec.md` -> `specify change plan create` -> `specify workspace sync` -> `specify change plan add` x 3 -> `specify change plan amend --project` x 2 -> `specify change plan validate` -> `/change:execute loop` -> `specify workspace push` -> `gh pr list` (read-only). No registry mutation -- both projects exist before the run.

Verb sequence (run 2, after the operator merges PRs by hand): `specify registry validate` -> `specify workspace push` (reports `up-to-date`) -> `gh pr list` -> `specify change finalize`.

Full transcript and on-disk shapes: [`fixtures/new-feature/`](../../plugins/change/skills/plan/fixtures/new-feature/).

### Variant: update-existing

No `--from` and no `--source` -- sources are unused. Targets are existing registered projects; baseline accumulation in `.specify/workspace/<peer>/specs/` is the dominant signal during planning.

Run against the same populated hub:

```text
/change:plan <name> orchestrate polish-pass \
    shape update-existing
```

Pre-flight forbids `--from`, `--against`, and `--source` under this shape; supplying any is a hard exit. The umbrella runs through push, stops for operator PR merge, and finalizes on re-entry:

1. **Brief.** Scaffolded with `inputs: []`; the operator writes one paragraph naming the capabilities being polished.
2. **Registry.** Multi-project; descriptions complete. No mutation.
3. **Plan.** Discovery falls back to baseline accumulation in `.specify/workspace/<peer>/specs/` because the input set is empty. Propose surfaces two slices (one per project, **no contract change** -- the polish does not change the API surface). Assignment routes each slice to its existing project. No registry mutation.
4. **Execute.** Both changes drive to `done`.
5. **Push.** Two PRs opened.
6. **Land.** The operator reviews and merges both PRs through the forge UI or `gh pr merge`.
7. **Finalize.** Re-run the umbrella; archive completes.

Verb sequence: `specify change create` -> `specify registry validate` -> `/change:plan` -> `specify change plan create` -> `specify workspace sync` -> `specify change plan add` x 2 -> `specify change plan amend --project` x 2 -> `specify change plan validate` -> `/change:execute loop` -> `specify workspace push` -> operator PR merge -> `specify change finalize`.

Full transcript and on-disk shapes: [`fixtures/update-existing/`](../../plugins/change/skills/plan/fixtures/update-existing/).

### Manual fallback parity

Each step in every shape above is a shell-out the umbrella runs verbatim. Operators can drop down a layer at any step -- see [Drop down a layer](../how-to/drop-down-a-layer.md#from-layer-4-to-layer-3-skip-the-umbrella) for the canonical command sequence. The umbrella's value is single-command convenience plus idempotent re-entry; it adds no behaviour beyond the underlying skills and CLI verbs.

## What you learned

- Specify opens PRs but does not merge them. Landing is an explicit operator action through the forge UI or `gh pr merge`; `specify change finalize` only verifies that the PRs are already `MERGED`.
- `specify change finalize` is the canonical closure verb (RFC-9 §4C): four guards in order (plan-presence, terminal-state, PR-state, workspace-cleanliness) before atomically archiving `plan.yaml`, `change.md`, and `.specify/plans/<name>/` into `.specify/archive/plans/<YYYYMMDD>-<name>/`.
- `--clean` prunes `.specify/workspace/<peer>/` after the archive completes; `--dry-run` previews the guard table without writing.
- `finalize` is idempotent: re-running after a refused guard completes the archive on the second invocation; re-running after a successful finalize returns `plan-not-found` (the "already finalized" signal).
- The same Steps 8-9 close out all three change shapes (`migrate-legacy`, `new-feature`, `update-existing`); only the inputs to Step 4 (Plan) differ.
- The Layer 4 umbrella `/change:plan <name> orchestrate` composes Steps 1-9 into a single operator action; it is composition only and adds no behaviour beyond the underlying skills and CLI verbs.

## Cross-links

- [`/change:plan <name> orchestrate`](../reference/change-skills/change.md) -- Layer 4 umbrella reference page.
- [Land a change](../how-to/land-a-change.md) -- focused how-to on autonomous vs supervised landing.
- [`specify workspace`](../reference/cli/workspace.md) -- workspace sync, status, and push.
- [`specify change finalize`](../reference/cli/change.md#specify-change-finalize) -- CLI reference, the four guards, JSON v2 envelope.
- [Change landing issues](../appendices/troubleshooting.md#change-landing-issues) -- `branch-pattern-mismatch`, `plan-not-found`, dirty clones.
- [Drop down a layer](../how-to/drop-down-a-layer.md) -- manual-fallback for every umbrella step.

## Next

[Legacy Migration at Scale](legacy-migration-at-scale.md) -- decompose a large monolith across multiple target repos using the analyze/extract split.

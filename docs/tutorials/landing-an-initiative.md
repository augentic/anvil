# Landing an Initiative

This tutorial picks up where [Cross-Repo Initiatives](cross-repo-initiative.md) leaves off: two PRs are open against the registered projects, and the `oauth-login` plan has every entry `done`. We now exercise the **landing half** of the platform-first loop -- merging the PRs and archiving the plan -- and round it out with the `/spec:plan --orchestrate` umbrella (formerly the `/spec:initiative` skill) that drives each of the three initiative shapes as a single operator action.

It exercises Steps 8-9 of the RFC-9 §1C critical path:

8. `specify workspace merge` -- squash-merge PRs once CI is green (RFC-9 §4A)
9. `specify initiative finalize` -- confirm landing and archive (RFC-9 §4C)

Together with [Cross-Repo Initiatives](cross-repo-initiative.md), the page-pair walks the full Steps 1-9 path. Both halves can be replayed against the live CLI as an integration test (per RFC-9 §1C, any deviation is a blocker).

> **Choosing your topology.** This tutorial extends the platform-hub flow from [Cross-Repo Initiatives](cross-repo-initiative.md). The Steps 8-9 verbs work identically against the platform-as-project shape (`url: .` in the registry), but the workspace clones in that case are symlinks to the initiating repo rather than separate clones.

**Prerequisites:**

- Completed [Cross-Repo Initiatives](cross-repo-initiative.md) up to and including Step 7. Two PRs are open on `specify/oauth-login` against `org/shop-backend` and `org/shop-mobile`.
- [`gh`](https://cli.github.com/) installed and authenticated against your GitHub org -- both `workspace merge` and `initiative finalize` shell out to `gh`.

## State on entry

Recap the on-disk state we are starting from:

| Surface | Expected |
|---------|----------|
| `specify plan status` | Three entries; `Summary: 0 pending, 0 in-progress, 3 done` |
| `gh pr list -R org/shop-backend --head specify/oauth-login` | Exactly one open PR |
| `gh pr list -R org/shop-mobile --head specify/oauth-login` | Exactly one open PR |
| `git status` (in each workspace clone) | Clean (the auto-commits from `/spec:execute --loop` are pushed, not staged) |

## 8. Land the PRs (optional)

Once CI is green on each PR, squash-merge them in one shot:

```bash
specify workspace merge
```

Per project, the verb checks `gh pr checks` against the `specify/oauth-login` branch and, if every check is `pass` or `skipping`, runs `gh pr merge --squash`.

<details>
<summary>Expected output (all checks green)</summary>

```text
specify: workspace merge — oauth-login (specify/oauth-login)

  shop-backend     merged                    PR #41     https://github.com/org/shop-backend/pull/41
  shop-mobile      merged                    PR #18     https://github.com/org/shop-mobile/pull/18

2 merged, 0 would-merge, 0 pending-checks, 0 failed-checks, 0 closed, 0 no-branch, 0 branch-pattern-mismatch, 0 failed.
```

</details>

The verb refuses to operate on any PR whose branch is not `specify/oauth-login` exactly (the `branch-pattern-mismatch` guard). It never passes `--admin` or `--auto`, and it never overrides failing or pending checks. Failures on one project surface in their own row without aborting the others.

Use `--dry-run` to see the would-merge classification without invoking `gh pr merge`. See [`specify workspace merge`](../reference/cli/workspace.md#specify-workspace-merge) for the full status table and exit-code contract (any `pending-checks`, `failed-checks`, or `branch-pattern-mismatch` flips the exit code to `1` so CI loops can branch on it).

> **Supervised landing.** If you want a manual review step before each merge -- or if CI requires it -- skip `workspace merge` and merge each PR by hand on the forge. `initiative finalize` (Step 9) does not care **how** the PRs got merged, only that every project's PR is `MERGED` on remote.

## 9. Finalize the initiative

Once every PR is merged, close the initiative with the canonical closure verb:

```bash
specify initiative finalize
```

`finalize` confirms the whole initiative is landed and atomically sweeps local plan state into the archive (RFC-9 §4C). It runs four guards in order before any move:

1. **Plan-presence:** `plan.yaml` exists.
2. **Plan terminal-state:** every entry is `done` / `failed` / `skipped`.
3. **Per-project PR-state:** every registered project's PR on `specify/oauth-login` is `MERGED` on its remote (or has no PR at all). Refuses on `unmerged` / `closed` / `branch-pattern-mismatch` / `failed`.
4. **Workspace-cleanliness:** `git status --porcelain` is empty for every workspace clone.

Any guard failure refuses with a per-project status table and leaves the on-disk state untouched. When all guards pass, `plan.yaml`, `initiative.md`, and `.specify/plans/oauth-login/` move atomically into `.specify/archive/plans/<YYYYMMDD>-oauth-login/`.

<details>
<summary>Expected output (all PRs merged, clean clones)</summary>

```text
specify: initiative finalize — oauth-login (specify/oauth-login)

  shop-backend         merged                   PR #41     https://github.com/org/shop-backend/pull/41
  shop-mobile          merged                   PR #18     https://github.com/org/shop-mobile/pull/18

2 merged, 0 unmerged, 0 closed, 0 no-branch, 0 branch-pattern-mismatch, 0 dirty, 0 failed.

Initiative `oauth-login` finalized.
  archived plan: /…/shop-platform/.specify/archive/plans/oauth-login-20260428.yaml
  archived dir:  /…/shop-platform/.specify/archive/plans/oauth-login-20260428
```

</details>

The two workspace clones stay on disk under `.specify/workspace/` -- they are the staging area for the next initiative. To prune them at the same time:

```bash
specify initiative finalize --clean
```

`--clean` removes `.specify/workspace/<peer>/` for every non-symlink registered project after the archive completes. Refused when any clone has a dirty working tree; the diagnostic warns that `--clean` would drop the uncommitted changes.

Use `--dry-run` to preview the guard table without writing anything -- useful for verifying readiness before you commit. `finalize` is **idempotent**: re-running it after manually clearing a refused guard (e.g. merging the last PR by hand) completes the archive on the second invocation. Re-running after a successful finalize returns `plan-not-found`, the explicit "already finalized" signal.

> **One-shot variant -- `/spec:plan --orchestrate` (RFC-9 §2C).** The Layer 4 umbrella mode (formerly `/spec:initiative`) composes Steps 1-9 into a single operator action: brief -> registry validate -> plan -> execute -> push -> optional merge -> finalize. The three subsections below show the umbrella driving each of the three initiative shapes against the same hub. See [`/spec:plan --orchestrate`](../reference/initiative-skills/initiative.md) for the full algorithm, halt semantics, and re-entry rules.

## Verification

Continuing from the [Cross-Repo Initiatives](cross-repo-initiative.md#verification) verification table, Steps 8-9 produce these expected outputs:

| After | Command | Expect |
|---|---|---|
| Step 8 | `gh pr view <pr> -R org/shop-backend --json state,merged` | `{"state":"MERGED","merged":true}`. |
| Step 9 | `ls .specify/archive/plans/` | A `oauth-login-<YYYYMMDD>.yaml` plan file plus a `oauth-login-<YYYYMMDD>/` directory holding `initiative.md` and the `plans/oauth-login/` authoring trail. |
| Step 9 | `ls plan.yaml` | `No such file or directory` -- the plan moved to the archive. |
| Step 9 | `specify initiative finalize` (re-run) | Exits `1` with `error: plan-not-found` -- the canonical "already finalized" signal. |

Any deviation is a blocker. File the failing transcript against this tutorial; per RFC-9 §1C the gap is in the implementation, not the design.

## Initiative shapes

The platform-first loop is shape-agnostic. The same Steps 1-9 drive three initiative shapes (RFC-9 §Motivation -> *The three initiative shapes*); only the inputs to Step 4 (Plan) differ. Each shape is also drivable as a single command via the Layer 4 umbrella mode `/spec:plan --orchestrate` (RFC-9 §2C, formerly `/spec:initiative`). The transcripts below show each shape from the umbrella's perspective; the manual fallback for every step is the same Layer 1 verb the umbrella shells out to (see [Drop down a layer](../how-to/drop-down-a-layer.md#from-layer-4-to-layer-3-skip-the-umbrella) for the exact verb sequence).

### Variant: migrate-legacy

Sources arrive via `--source <key>=<git-url-or-path>`. `/spec:analyze` clones each source into `.specify/plans/<initiative>/analyze/<key>/` (the [tier-1 workspace](../explanation/workspace-tiers.md#the-two-tiers)) for shallow capability inventory; deep `/spec:extract` runs at define time per change. Targets are existing or newly-minted registered projects.

Run against an empty hub:

```text
/spec:plan --orchestrate migrate-foo \
    --shape migrate-legacy \
    --source monolith=git@github.com:org/legacy-foo.git \
    --auto-merge
```

The umbrella runs all seven steps without halting:

1. **Brief.** `specify initiative create migrate-foo` scaffolds `initiative.md`; the operator confirms a default body listing the legacy monolith as a `legacy-code` input.
2. **Registry.** Empty + `--shape migrate-legacy` -> hand off to the 2B greenfield path inside `/spec:plan`.
3. **Plan.** `/spec:plan` runs discovery against the cloned monolith, proposes a two-project topology (`foo-backend` + `foo-mobile`), shells `specify registry add` x 2 and `specify workspace sync` once, then propose decomposes into one cross-project contract change plus one implementation slice per project. Assignment routes the implementation slices.
4. **Execute.** `/spec:execute --loop` drives all three changes to `done` (contract change runs against the hub; the two implementation changes run inside their workspace clones).
5. **Push.** `specify workspace push` opens two PRs.
6. **Land.** `--auto-merge` -> `specify workspace merge` waits for CI, sees both PRs green, squash-merges them.
7. **Finalize.** `specify initiative finalize` archives the plan and brief.

Verb sequence: `specify initiative create` -> `specify registry validate` -> `/spec:plan` -> `specify plan create` -> `specify registry add` x 2 -> `specify workspace sync` -> `specify plan add` x 3 -> `specify plan amend --project` x 2 -> `specify plan validate` -> `/spec:execute --loop` -> `specify workspace push` -> `specify workspace merge` -> `specify initiative finalize`. Full transcript and on-disk shapes: [`fixtures/migrate-legacy/`](../../plugins/change/skills/plan/fixtures/migrate-legacy/).

### Variant: new-feature

Sources arrive via `--from <docs>` only (or via `initiative.md:inputs`). Targets are existing registered projects, possibly with new ones spawned at assignment time via the registry-proposal sub-step (RFC-9 §2B).

Run against the populated hub from [Cross-Repo Initiatives](cross-repo-initiative.md) Steps 1-3 (or your own equivalent):

```text
/spec:plan --orchestrate dark-mode \
    --shape new-feature \
    --from ./docs/dark-mode-spec.md
```

**The walkthrough across [Cross-Repo Initiatives](cross-repo-initiative.md) and this tutorial is this shape.** The umbrella drives the same nine-step flow, with one wrinkle: without `--auto-merge`, Step 6 lists the open PRs and **stops**. The operator merges PRs by hand on the forge (or runs `specify workspace merge` directly), then re-runs the umbrella to land Step 7. Re-entry inspects on-disk state -- brief present, plan terminal, every PR `MERGED` on remote -- and skips straight to `specify initiative finalize`.

Verb sequence (run 1, halts at step 6): `specify initiative create` -> `specify registry validate` -> `/spec:plan --from ./docs/dark-mode-spec.md` -> `specify plan create` -> `specify workspace sync` -> `specify plan add` x 3 -> `specify plan amend --project` x 2 -> `specify plan validate` -> `/spec:execute --loop` -> `specify workspace push` -> `gh pr list` (read-only). No registry mutation -- both projects exist before the run.

Verb sequence (run 2, after the operator merges PRs by hand): `specify registry validate` -> `specify workspace push` (reports `up-to-date`) -> `gh pr list` -> `specify initiative finalize`.

Full transcript and on-disk shapes: [`fixtures/new-feature/`](../../plugins/change/skills/plan/fixtures/new-feature/).

### Variant: update-existing

No `--from` and no `--source` -- sources are unused. Targets are existing registered projects; baseline accumulation in `.specify/workspace/<peer>/specs/` is the dominant signal during planning.

Run against the same populated hub:

```text
/spec:plan --orchestrate polish-pass \
    --shape update-existing \
    --auto-merge
```

Pre-flight forbids `--from`, `--against`, and `--source` under this shape; supplying any is a hard exit. The umbrella runs all seven steps without halting:

1. **Brief.** Scaffolded with `inputs: []`; the operator writes one paragraph naming the capabilities being polished.
2. **Registry.** Multi-project; descriptions complete. No mutation.
3. **Plan.** Discovery falls back to baseline accumulation in `.specify/workspace/<peer>/specs/` because the input set is empty. Propose surfaces two slices (one per project, **no contract change** -- the polish does not change the API surface). Assignment routes each slice to its existing project. No registry mutation.
4. **Execute.** Both changes drive to `done`.
5. **Push.** Two PRs opened.
6. **Land.** `--auto-merge` -> both PRs squash-merged.
7. **Finalize.** Archive completes.

Verb sequence: `specify initiative create` -> `specify registry validate` -> `/spec:plan` -> `specify plan create` -> `specify workspace sync` -> `specify plan add` x 2 -> `specify plan amend --project` x 2 -> `specify plan validate` -> `/spec:execute --loop` -> `specify workspace push` -> `specify workspace merge` -> `specify initiative finalize`.

Full transcript and on-disk shapes: [`fixtures/update-existing/`](../../plugins/change/skills/plan/fixtures/update-existing/).

### Manual fallback parity

Each step in every shape above is a shell-out the umbrella runs verbatim. Operators can drop down a layer at any step -- see [Drop down a layer](../how-to/drop-down-a-layer.md#from-layer-4-to-layer-3-skip-the-umbrella) for the canonical command sequence. The umbrella's value is single-command convenience plus idempotent re-entry; it adds no behaviour beyond the underlying skills and CLI verbs.

## What you learned

- `specify workspace merge` lands the PRs once CI is green (RFC-9 §4A): per-project `gh pr checks` -> `gh pr merge --squash`, with a `branch-pattern-mismatch` guard that refuses any PR whose branch is not `specify/<initiative-name>` exactly.
- `specify initiative finalize` is the canonical closure verb (RFC-9 §4C): four guards in order (plan-presence, terminal-state, PR-state, workspace-cleanliness) before atomically archiving `plan.yaml`, `initiative.md`, and `.specify/plans/<name>/` into `.specify/archive/plans/<YYYYMMDD>-<name>/`.
- `--clean` prunes `.specify/workspace/<peer>/` after the archive completes; `--dry-run` previews the guard table without writing.
- `finalize` is idempotent: re-running after a refused guard completes the archive on the second invocation; re-running after a successful finalize returns `plan-not-found` (the "already finalized" signal).
- The same Steps 8-9 close out all three initiative shapes (`migrate-legacy`, `new-feature`, `update-existing`); only the inputs to Step 4 (Plan) differ.
- The Layer 4 umbrella `/spec:plan --orchestrate <name>` composes Steps 1-9 into a single operator action; it is composition only and adds no behaviour beyond the underlying skills and CLI verbs.

## Cross-links

- [`/spec:plan --orchestrate`](../reference/initiative-skills/initiative.md) -- Layer 4 umbrella reference page (formerly `/spec:initiative`).
- [Land an initiative](../how-to/land-an-initiative.md) -- focused how-to on autonomous vs supervised landing.
- [`specify workspace merge`](../reference/cli/workspace.md#specify-workspace-merge) -- CLI reference, status vocabulary, exit-code contract.
- [`specify initiative finalize`](../reference/cli/initiative.md#specify-initiative-finalize) -- CLI reference, the four guards, JSON v2 envelope.
- [Initiative landing issues](../appendices/troubleshooting.md#initiative-landing-issues) -- `branch-pattern-mismatch`, `plan-not-found`, dirty clones.
- [Drop down a layer](../how-to/drop-down-a-layer.md) -- manual-fallback for every umbrella step.

## Next

[Legacy Migration at Scale](legacy-migration-at-scale.md) -- decompose a large monolith across multiple target repos using the analyze/extract split.

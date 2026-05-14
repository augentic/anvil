# RFC-14: Registry Workspace

> Status: Implemented · Depends: [RFC-13](rfc-13-extensibility.md), [RFC-1](rfc-1-cli.md), [RFC-9](rfc-9-platform.md)

## Abstract

RFC-14 defines the **registry workspace** lifecycle: a change can clone some or all remote repositories listed in `registry.yaml` into a local derived workspace, run the ordinary Specify slice loop against those local checkouts, then push each changed checkout back to its remote origin as a branch and a pull request.

The coordinator repository remains the owner of platform state:

- `registry.yaml` declares the remote repositories that may participate in a change.
- `change.md`, `plan.yaml`, and `.specify/plan.lock` coordinate which slices run and in what order.
- `.specify/workspace/<project>/` holds temporary local materialisations of registry projects.
- `specify workspace push` ships completed local work back to the projects' remotes as branches and opens pull requests against the remote default branch. RFC-14 never pushes directly to a default branch and never merges pull requests automatically — landing each PR is operator-driven through the forge UI or `gh pr merge`.

RFC-14 focuses solely on making the registry workspace a first-class execution area for remote repositories.

## Motivation

RFC-9 introduced a platform registry so one change can coordinate work across multiple repositories. RFC-13 clarified that the registry is a first-party platform component. The remaining gap is operational: once a plan says "run this slice against `orders` and that slice against `billing`", Specify needs a deterministic way to materialise those remote repositories locally, let the slice loop modify them, and return the work to the original remotes.

Without a first-class registry workspace contract:

- Agents and humans must clone peer repositories by hand, creating inconsistent paths and branch names.
- `/change:plan` can reason about registry projects, but `/change:execute` lacks a stable local project root for each remote.
- Push-back behavior is easy to reimplement inconsistently across skills, scripts, and CI.
- Cleanup and stale-clone safety are unclear.

RFC-14 makes `.specify/workspace/` the canonical local execution area for registry projects. The workspace is derived state: safe to refresh, inspect, push from, and remove without changing the coordinator's durable plan artifacts.

## Current State And Delta

The framework already has the registry workspace substrate:

- `specify workspace sync` materialises all registry projects under `.specify/workspace/<project>/`.
- `specify workspace status` reports basic slot kind, HEAD, and dirty state.
- `specify workspace push [<project>...]` pushes `specify/<change-name>` branches with `--force-with-lease` and creates PRs.
- `specify workspace merge [<project>...]` exists today and squash-merges green PRs. RFC-14 removes automated merge from the workspace verb surface (see §Non-Goals and §Migration).
- `specify change finalize --clean` verifies per-project PR state and workspace cleanliness, archives coordinator state, and can remove clean workspace clones.

RFC-14 is therefore not a greenfield workspace implementation. It tightens the execution contract around the existing primitives:

- selected `sync` and `status` operations;
- early validation for unknown project selectors;
- automatic materialisation of the next plan entry's project during `/change:execute`;
- pre-mutation branch preparation on `specify/<change-name>` from `origin/HEAD`;
- dirty-work guards before execution and push;
- richer status output for branch, remote, project config, and active slices;
- hardening for mismatched slots, path traversal, and unexpected symlinks;
- removal of automated PR merge: `specify workspace push` ends at PR creation, and per-project merge becomes an operator action confirmed by `specify change finalize`.

## Design

### Core Model

A **coordinator root** is the repository where the operator runs the change. It owns:


| Path                  | Purpose                                                       |
| --------------------- | ------------------------------------------------------------- |
| `registry.yaml`       | Declares projects that may participate in cross-repo changes. |
| `change.md`           | Operator brief for the current change.                        |
| `plan.yaml`           | Ordered list of slices, including target registry projects.   |
| `.specify/plan.lock`  | Advisory lock for `/change:execute`.                          |
| `.specify/workspace/` | Derived local materialisations of registry projects.          |
| `.specify/archive/`   | Coordinator-owned change and plan archives.                   |


A **workspace slot** is one local checkout or symlink at:

```text
<coordinator-root>/.specify/workspace/<project-name>/
```

Each slot corresponds to exactly one `registry.yaml:projects[]` entry. For remote URLs, the slot is a Git checkout with `origin` pointing at the registry URL. For local paths, the slot is a symlink to the resolved local repository.

The slot is the project root passed to the slice loop. Capability skills see the materialised repository as their `$PROJECT_DIR`; they do not need to know whether it came from a remote clone, a greenfield bootstrap, or a local symlink.

### Registry Input

Remote repositories are declared in `registry.yaml`:

```yaml
version: 1
projects:
  - name: orders
    url: git@github.com:org/orders.git
    schema: omnia@v1
    description: Customer orders service.
  - name: billing
    url: https://github.com/org/billing.git
    schema: omnia@v1
    description: Billing ledger and invoicing.
```

The registry remains the source of truth for:

- project name;
- clone or symlink target;
- default capability identifier;
- human description used by planning;
- optional contract role declarations.

### Project Selection

Workspace operations may target all registry projects or a selected subset.

Selection rules:

1. Positional project names passed to `specify workspace sync`, `status`, or `push` select exactly those projects.
2. Selector resolution is a shared preflight across all workspace verbs. Unknown project names fail before any clone, fetch, status probe, push, or pull-request lookup occurs.
3. `specify workspace sync` and `status` with no positional project names operate on every project in `registry.yaml`.
4. `/change:plan` may sync every registry project when it needs discovery context.
5. `/change:execute` materialises only the project needed by the selected plan entry unless the operator explicitly asks for a broader sync.
6. `workspace push` with no positional project names retains today's all-project behavior; unaffected projects classify as `up-to-date`, `local-only`, or `no-branch`.

This preserves the existing human-facing defaults while giving drivers a precise way to materialise only the project they are about to mutate.

### Materialisation

`specify workspace sync [<project>...]` creates or refreshes selected slots under `.specify/workspace/`.

For each selected registry project:

1. **Remote URL.** `git@`, `ssh://`, `https://`, `http://`, `git+ssh://`, and `git+https://` URLs materialise as Git checkouts.
2. **Local path.** `.`, repo-relative paths, and absolute paths materialise as symlinks.
3. **Greenfield remote.** A remote URL that cannot be cloned may materialise as a local Git repository with `origin` set and `.specify/project.yaml` bootstrapped from the registry capability. Sync may create the initial local scaffold commit required to leave the slot clean, but it MUST NOT create the remote repository or mutate the forge. Remote repository creation remains part of `workspace push`.

Materialisation is idempotent:

- Existing remote slots fetch updates from `origin`.
- Existing local slots keep pointing at the same resolved target.
- A slot for the wrong project, wrong URL, or wrong materialisation type fails with an actionable diagnostic instead of being overwritten.
- Remote-backed slot roots must be ordinary directories; an existing symlink at a remote-backed slot path is a mismatched slot and fails.
- Local-path slot roots must be symlinks whose canonical target matches the registry entry's resolved path.
- `.specify/workspace/` and `.specify/.cache/` remain ignored by Git.

The workspace is temporary derived state, but it is not required to be deleted after every change. Keeping slots between changes is allowed so future syncs can fetch rather than reclone. Removing `.specify/workspace/<project>/` is always a valid way to force a fresh materialisation.

### Branch Preparation

Before a slice mutates a remote-backed slot, `/change:execute` must ensure the slot is on the change branch:

```text
specify/<change-name>
```

This is the core RFC-14 execution delta. Today branch creation happens late, during `workspace push`; RFC-14 moves the branch guard before file mutation so all slice output is produced on the branch that will be pushed. The change branch is the only branch RFC-14 ever pushes to: `origin/HEAD` (and any other default branch) is read-only from this lifecycle's perspective.

Branch preparation:

1. Refuses to proceed when the slot has **unrelated uncommitted changes** (see definition below).
2. Fetches `origin` if the slot is a remote checkout.
3. Resolves the base branch from the remote default branch (`origin/HEAD`) after fetch. Implementations may refresh `origin/HEAD` with `git remote set-head origin --auto` or an equivalent remote-default discovery step.
4. Creates or checks out `specify/<change-name>` from `origin/HEAD` when the branch does not yet exist locally. The first time a change touches a slot, the branch is always cut from the freshly fetched remote default — never from an existing local branch tip.
5. Reuses the local branch when resuming an in-progress change, fast-forwarding it to `origin/specify/<change-name>` when the remote tip is ahead.
6. Refuses branch names that do not match the exact `specify/<change-name>` pattern.

**Unrelated uncommitted changes** are any tracked-file diffs against the slot's `HEAD` whose paths fall outside both:

- `.specify/slices/<change-name>/`, and
- the union of paths declared on the active plan entry (the slice's `sources` mapping resolved to absolute paths inside the slot, plus baseline writes under `.specify/specs/`, `.specify/archive/`, and capability-owned output directories such as `crates/`, `contracts/`, `apps/`).

Untracked files outside those paths do not block branch preparation, but `workspace push` still refuses dirty checkouts (§Validation And Safety). The intent is to let an in-progress slice resume cleanly while refusing to layer a fresh slice on top of unrelated edits.

RFC-14 does not add a registry-level base-branch field. If `origin/HEAD` cannot be resolved (the remote was never fetched, the symbolic ref is missing, or the forge returned an error), branch preparation fails with the diagnostic key `origin-head-unresolved` and an actionable message. The driver never guesses `main`, `master`, or any other branch name.

Local-path symlink slots are handled conservatively: they may be inspected and modified only when the target repository can provide a usable `origin`. Push reports `local-only` when no remote can be resolved.

#### Interaction with `registry-amendment-required`

When a phase emits `outcome: registry-amendment-required` (per `/change:execute`'s deferred branch, RFC-9 §2B), branch preparation never runs against a non-existent slot: the deferred outcome is recorded before any mutation, the slice is dropped, and the plan entry transitions to `blocked` with the proposed registry payload journalled for the operator. The next `/change:execute` pass after the operator runs the canonical recovery sequence (`registry add` → `workspace sync` → `plan amend --project` → `plan transition pending`) sees a fully materialised slot and prepares the change branch normally.

### Slice Execution

Plan entries continue to target registry projects using `project:`.

```yaml
name: modernise-checkout
changes:
  - name: orders-domain-model
    project: orders
    schema: omnia@v1
    status: pending
    description: Replace the order aggregate and persistence adapter.
  - name: billing-contract
    project: billing
    schema: omnia@v1
    status: pending
    description: Consume the new order-completed event contract.
```

When `/change:execute` selects a plan entry:

1. Resolve `entry.project` through `registry.yaml`.
2. Ensure the corresponding workspace slot exists, materialising it if needed.
3. Prepare the change branch for that slot.
4. Run `/spec:define -> /spec:build -> /spec:merge` with the slot as the project root.
5. Commit any non-baseline residue in the slot before marking the plan entry `done`.
6. Record phase outcomes and plan status in the existing coordinator plan flow.

#### Commit ownership

RFC-14 splits commit responsibility between two owners and assigns each owner a disjoint set of paths so commits never overlap:

| Owner | Trigger | Paths covered | Commit message |
| --- | --- | --- | --- |
| `specify slice merge run` (within `/spec:merge`) | Auto-fires when CWD is inside a workspace clone (slot has `.specify/project.yaml`). | Baseline merge output: `.specify/specs/` and `.specify/archive/`. | `specify: merge <slice-name>` |
| `/change:execute` (post-merge step 5 above) | Runs after `/spec:merge` returns success, before the plan entry transitions to `done`. | Non-baseline residue: every other tracked path the slice produced — capability-owned outputs (e.g. `crates/`, `contracts/`, `apps/`), generated assets, and any other tracked diff outside the merge owner's paths. | `specify: residue <slice-name>` |

The `slice merge run` auto-commit remains the canonical owner of baseline writes; `/change:execute` MUST NOT re-commit `.specify/specs/` or `.specify/archive/` even if those paths are dirty after merge — that condition indicates a bug in the merge owner and surfaces as an execution failure with the diagnostic key `baseline-residue-after-merge`.

If the slot is clean of non-baseline residue after `/spec:merge`, `/change:execute` skips its commit step entirely (no empty commits). If its commit fails, the driver surfaces an execution failure with the diagnostic key `residue-commit-failed` before transitioning the plan entry to `done`.

`workspace push` is transport-only in RFC-14: it never creates commits implicitly and never packages arbitrary dirty work at push time.

Slice artifacts, metadata, journals, and baseline writes live inside the materialised project checkout exactly as they would in a single-repo project:

```text
<coordinator-root>/.specify/workspace/orders/.specify/slices/<slice-name>/
<coordinator-root>/.specify/workspace/orders/.specify/specs/
<coordinator-root>/.specify/workspace/orders/contracts/
```

The coordinator owns the change plan; each workspace slot owns the project-local slice state produced while executing that plan entry.

### Commit And Push

After one or more project slots have local commits, `specify workspace push [<project>...]` pushes selected projects back to their remotes.

Per project:

1. Resolve the push remote. Remote registry URLs use `origin`; local-path slots read `git remote get-url origin` from the target repository.
2. Verify the current branch is exactly `specify/<change-name>`. Refuse to push from `origin/HEAD`, the resolved default branch, or any other branch shape.
3. Refuse dirty checkouts.
4. Compare the local change branch with the remote branch, when one exists.
5. Create the remote repository only when the slot is greenfield, the remote is supported for repository creation, and this is not a dry run.
6. Push with `--force-with-lease` to `origin specify/<change-name>` when the local branch has work to publish.
7. Create or update a pull request for that branch, targeting the remote default branch resolved from `origin/HEAD`. Never merge the pull request — RFC-14 ends at PR creation, and the operator lands the PR through the forge UI or `gh pr merge` separately.

Push is best-effort across projects: one project's failure does not prevent other selected projects from pushing. The final result reports:

- `created` when a greenfield remote repository was created and the branch was pushed successfully.
- `pushed` when the local branch was ahead of the remote branch, or the remote branch was absent, and the push succeeded.
- `up-to-date` when the remote branch exists and its tip already equals local `HEAD`.
- `local-only` when no push remote can be resolved.
- `no-branch` when the expected local branch does not exist or the checkout is not currently on `specify/<change-name>`.
- `failed` when the checkout is dirty, remote creation fails, push is rejected, PR creation fails, or another Git/forge error occurs.

`--dry-run` performs all classification and remote-resolution checks without `git push`, repository creation, or pull-request creation.

### Merge And Finalize

RFC-14 does not automate pull-request merge. After `workspace push` opens a PR per affected project, landing each PR is an explicit operator action — performed through the forge UI, `gh pr merge`, or the project's normal review workflow. The framework's role ends at PR creation; the operator owns the merge decision, including review, CI gating, and any forge-side automation rules they have configured outside Specify.

`specify change finalize` remains the coordinator-level closure command. It is read-only with respect to forges: it confirms all required per-project PRs are merged on their remotes (via `gh pr view --json state,merged,headRefName`), archives the plan and change state, and — under `--clean` — may remove clean workspace slots for the finalized change. `finalize` never merges, force-merges, or otherwise mutates a pull request; an unmerged PR is reported as a blocking status (`unmerged`, `closed`, or `branch-pattern-mismatch`) and the operator must merge or close it before re-running `finalize`.

The pre-RFC-14 `specify workspace merge` verb (which squash-merged green PRs in batch) is removed by this RFC. Operators who need batch merge can script `gh pr merge --squash` over the registry list directly, or invoke their forge's built-in queue. See §Migration for the deprecation path.

### Status And Cleanup

`specify workspace status [<project>...]` reports the materialisation state for selected projects:

- slot path;
- slot type: `git-clone`, `symlink`, `missing`, or `other`;
- configured remote;
- current branch;
- HEAD SHA;
- dirty state;
- whether the current branch matches `specify/<change-name>`;
- presence of `.specify/project.yaml`;
- active slices in the slot.

Cleanup remains part of the existing coordinator closure path:

```bash
specify change finalize --clean
```

`finalize --clean` may remove clean workspace clones only after coordinator archive succeeds and the per-project PR-state and workspace-cleanliness guards pass. RFC-14 does not add a standalone `workspace clean` command; if selective cleanup is needed later, it should be proposed as a separate extension to this lifecycle.

## Validation And Safety

`specify workspace sync`, `/change:execute`, `workspace push`, and `specify change finalize` MUST enforce:

- every selected project exists in `registry.yaml`;
- selector validation completes before any filesystem, Git, forge, or merge side effect;
- registry project names are unique and kebab-case;
- remote URLs are supported clone targets;
- slot paths stay under `.specify/workspace/<project>/`;
- remote-backed slot roots are ordinary directories, never symlinks;
- local-path slot roots are symlinks whose canonical targets match the resolved registry path;
- no workspace slot escapes through path traversal or an unexpected symlink;
- dirty unrelated work (per the §Branch Preparation definition) blocks branch preparation;
- branch preparation uses `origin/HEAD` as the only RFC-14 base branch source, and fails with the diagnostic key `origin-head-unresolved` rather than guessing a default branch name;
- `/change:execute` commits non-baseline residue before marking a project entry `done`, and surfaces `baseline-residue-after-merge` when `.specify/specs/` or `.specify/archive/` are dirty after `/spec:merge` returns success;
- `workspace push` refuses dirty checkouts, never creates commits implicitly, and never pushes to a default branch;
- push operates only on `specify/<change-name>`;
- `--force-with-lease` is the only force-style push allowed;
- pull-request merge is never invoked by Specify — neither `workspace push` nor `change finalize` calls `gh pr merge` or any equivalent forge merge API;
- `finalize --clean` refuses destructive removal when any selected workspace clone is dirty.

The workspace is derived state, but safety still matters because it contains real Git checkouts capable of pushing to remote origins.

## Non-Goals

- Running one slice across multiple repositories.
- Hiding Git or PR failures behind best-effort success.
- **Automating pull-request merge.** RFC-14 ends at PR creation; landing the PR is an operator action through the forge UI or `gh pr merge`. The pre-RFC `specify workspace merge` verb is removed.
- Pushing to a default branch (`main`, `master`, the resolved `origin/HEAD`, or any other non-`specify/<change-name>` branch).
- Making `.specify/workspace/` durable source state.
- Adding a standalone `specify workspace clean` command.
- Adding `registry.yaml` base-branch configuration.
- Creating remote repositories during `workspace sync`.

## Implementation Scope

### Phase 1 - Workspace Selection And Status

1. Add optional project selectors to `specify workspace sync` and `status`.
2. Preserve all-project defaults for human-invoked commands without selectors.
3. Extend `workspace status` output with branch, remote, change-branch match, and active-slice summary.
4. Validate unknown project names before touching the filesystem.

### Phase 2 - Materialisation Hardening

1. Make remote sync idempotent across clone, fetch, and greenfield bootstrap.
2. Keep greenfield bootstrap local during sync; do not create remote repositories until push.
3. Refuse mismatched existing slots rather than overwriting them.
4. Harden symlink and path traversal checks under `.specify/workspace/`.
5. Keep `.specify/workspace/` and `.specify/.cache/` ignored.

### Phase 3 - Execute Against Workspace Slots

1. Teach `/change:execute` to resolve `entry.project` to a workspace slot.
2. Materialise missing selected slots before slice execution.
3. Prepare or resume `specify/<change-name>` from `origin/HEAD` before mutation; surface `origin-head-unresolved` when the remote default cannot be discovered.
4. Refuse unrelated dirty work before mutation, scoping "unrelated" per §Branch Preparation.
5. Run `/spec:define`, `/spec:build`, and `/spec:merge` with the slot as project root.
6. Leave baseline-merge commits to `specify slice merge run`'s existing workspace auto-commit; commit only non-baseline residue from `/change:execute` before transitioning the plan entry to `done`. Surface `baseline-residue-after-merge` if `/spec:merge` returns success while leaving `.specify/specs/` or `.specify/archive/` dirty.
7. Preserve coordinator-level plan locking and status transitions.

### Phase 4 - Push, Finalize, And Cleanup

1. Ensure `workspace push` uses selected projects and reports per-project outcomes.
2. Ensure `workspace push` refuses dirty checkouts, refuses any non-`specify/<change-name>` branch, creates no commits, and reports `created`, `pushed`, `up-to-date`, `local-only`, `no-branch`, or `failed`.
3. Ensure `workspace push` creates or updates PRs from `specify/<change-name>` against the remote default branch resolved from `origin/HEAD`, and never invokes any forge merge API.
4. Remove `specify workspace merge` from the CLI surface (or land it as a one-release deprecation shim that exits non-zero with a pointer to `gh pr merge` and `specify change finalize`).
5. Keep cleanup on `change finalize --clean`, with dirty-clone refusal before removal.
6. Wire `change finalize` to verify operator-merged per-project PRs before archiving; ensure `finalize` itself never calls a forge merge endpoint.

### Repository Updates

1. Update `docs/reference/registry.md` with the RFC-14 workspace lifecycle, including the operator-driven PR merge step and the removal of `workspace merge` automation.
2. Update `docs/reference/cli/workspace.md` for selectors, branch preparation, dirty-checkout push refusal, default-branch push refusal, and the removal of the `merge` subcommand.
3. Update `docs/reference/cli/change.md` for the operator-merge expectation in `change finalize`.
4. Update `/change:plan` sync-workspace guidance to distinguish discovery sync from execution sync.
5. Update `/change:execute` guidance to run project entries inside materialised slots, prepare branches before mutation, and commit non-baseline residue post-merge.
6. Update `/spec:merge` guidance to own the baseline workspace auto-commit explicitly (paths and commit message) so the split with `/change:execute` is unambiguous.
7. Add glossary entries for coordinator root, registry workspace, workspace slot, and change branch.

## Migration

RFC-14 is additive for sync, status, and finalize, but introduces two intentional behavioural breaks for operators who relied on the pre-RFC `workspace push` and `workspace merge` shapes.

Compatibility:

- Single-repo projects without `registry.yaml` continue to run directly in the current repository.
- Existing `registry.yaml` files remain valid.
- Existing workspace slots remain valid when their URL and materialisation type match the registry entry.
- Operators may remove `.specify/workspace/` to force a fresh sync.
- `change finalize --clean` keeps its existing four-guard shape (plan presence, terminal entries, per-project PR state, workspace cleanliness).

Compat breaks:

1. **`workspace push` no longer creates the change branch on the fly.** Pre-RFC, push would create or update `specify/<change-name>` from the slot's current `HEAD` regardless of which branch the slot was on. RFC-14's push verifies the slot is already on `specify/<change-name>` and refuses every other branch (including `main`, `master`, and `origin/HEAD`) with status `no-branch`. Operators who previously ran `workspace push` directly against a feature or default branch must either (a) drive the slot through `/change:execute` so branch preparation runs, or (b) check out `specify/<change-name>` themselves before push. This change is what makes "never push to `main`" enforceable.
2. **`specify workspace merge` is removed.** Pre-RFC, this verb batch-squash-merged every green PR on `specify/<change-name>`. RFC-14 deletes the verb and routes operators to the forge UI or `gh pr merge`. The recommended migration is a thin shell loop over `specify registry show --format json | jq` that invokes `gh pr merge --squash` per project; teams that want batch merge should script it explicitly so the merge decision stays visible. A single-release deprecation shim (exits non-zero with a pointer to the replacement) is acceptable; a silent removal is not.

Both breaks surface in `change finalize`'s output as actionable status rows, so operators discover them at the next finalize attempt rather than mid-loop.

Acceptance criteria:

- A multi-project change can materialise only the project needed by the selected plan entry, while human-invoked `workspace sync` without selectors still syncs all registry projects.
- `/change:execute` can run a plan entry against a remote-backed workspace slot.
- `/change:execute` prepares `specify/<change-name>` from `origin/HEAD` before mutating a remote-backed slot, and surfaces `origin-head-unresolved` when the remote default is missing.
- `/spec:merge`'s workspace auto-commit owns `.specify/specs/` and `.specify/archive/`; `/change:execute` commits only non-baseline residue and surfaces `baseline-residue-after-merge` if those paths are dirty after merge returns success.
- Completed local changes can be pushed to the corresponding remote origin on `specify/<change-name>`, and never to a default branch.
- `workspace push` refuses dirty checkouts and refuses non-`specify/<change-name>` checkouts deterministically (`no-branch`), instead of silently rebranding the slot's current `HEAD`.
- `workspace push` does not create commits and classifies `created`, `pushed`, `up-to-date`, `local-only`, `no-branch`, and `failed` deterministically.
- `workspace push` creates a pull request per affected project but never merges it; `change finalize` confirms operator-merged PRs without invoking any forge merge API.
- The `specify workspace merge` verb is removed (or reduced to a deprecation shim) and no Specify code path calls `gh pr merge` or an equivalent forge merge API.
- Deleting `.specify/workspace/<project>/` and re-running sync recreates a usable slot.

## Future Extensions

RFC-14 resolves the first landing around strict selectors, `origin/HEAD` branch preparation, split-owner commits (baseline → `/spec:merge`, residue → `/change:execute`), transport-only push, push-time greenfield repository creation, and operator-driven PR merge. Later RFCs may add:

1. **Commit customization.** Operator-configurable commit message templates or commit grouping across multiple slices.
2. **Base branch configuration.** An optional registry field for projects that intentionally branch from something other than the remote default branch.
3. **Selective cleanup.** A standalone `specify workspace clean [<project>...]` command with the same dirty-clone guards as `change finalize --clean`.
4. **Optional batch-merge helper.** A scripted helper (separate from RFC-14) that loops `gh pr merge --squash` over registry projects with explicit per-PR confirmation, for teams that want one-command landing without re-introducing automated merge into the framework.

## References

- [RFC-13: Immutable core + capability extensions](rfc-13-extensibility.md) - capability protocol, platform components, and registry-materialised execution.
- [RFC-1: `specify` CLI](rfc-1-cli.md) - CLI conventions and project config.
- [RFC-2: Execution](rfc-2-execution.md) - plan loop and dependency ordering.
- [RFC-3a: Monoliths](rfc-3a-monoliths.md) - plan entry shape.
- [RFC-3b: Platform](rfc-3b-platform.md) - cross-repo routing.
- [RFC-9: Platform](rfc-9-platform.md) - registry/change root placement and workspace push (the PR merge automation introduced in RFC-9 §4A is removed by RFC-14).


# RFC-14: Registry Workspace

> Status: Draft · Depends: [RFC-13](archive/rfc-13-extensibility.md), [RFC-1](archive/rfc-1-cli.md), [RFC-9](archive/rfc-9-platform.md)

## Abstract

RFC-14 defines the **registry workspace** lifecycle: a change can clone some or all remote repositories listed in `registry.yaml` into a local derived workspace, run the ordinary Specify slice loop against those local checkouts, then push each changed checkout back to its remote origin as a branch and pull request.

The coordinator repository remains the owner of platform state:

- `registry.yaml` declares the remote repositories that may participate in a change.
- `change.md`, `plan.yaml`, and `.specify/plan.lock` coordinate which slices run and in what order.
- `.specify/workspace/<project>/` holds temporary local materialisations of registry projects.
- `specify workspace push` and `specify workspace merge` ship completed local work back to the projects' remotes.

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
- `specify workspace merge [<project>...]` refuses branch mismatches, checks CI, and squash-merges green PRs.
- `specify change finalize --clean` verifies per-project PR state and workspace cleanliness, archives coordinator state, and can remove clean workspace clones.

RFC-14 is therefore not a greenfield workspace implementation. It tightens the execution contract around the existing primitives:

- selected `sync` and `status` operations;
- early validation for unknown project selectors;
- automatic materialisation of the next plan entry's project during `/change:execute`;
- pre-mutation branch preparation on `specify/<change-name>`;
- dirty-work guards before execution and push;
- richer status output for branch, remote, project config, and active slices;
- hardening for mismatched slots, path traversal, and unexpected symlinks.

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

1. Positional project names passed to `specify workspace sync`, `status`, `push`, or `merge` select exactly those projects.
2. Unknown project names fail before any clone, fetch, push, or merge occurs.
3. `specify workspace sync` and `status` with no positional project names operate on every project in `registry.yaml`.
4. `/change:plan` may sync every registry project when it needs discovery context.
5. `/change:execute` materialises only the project needed by the selected plan entry unless the operator explicitly asks for a broader sync.
6. `workspace push` and `workspace merge` with no positional project names retain today's all-project behavior; unaffected projects classify as `up-to-date`, `local-only`, or `no-branch`.

This preserves the existing human-facing defaults while giving drivers a precise way to materialise only the project they are about to mutate.

### Materialisation

`specify workspace sync [<project>...]` creates or refreshes selected slots under `.specify/workspace/`.

For each selected registry project:

1. **Remote URL.** `git@`, `ssh://`, `https://`, `http://`, `git+ssh://`, and `git+https://` URLs materialise as Git checkouts.
2. **Local path.** `.`, repo-relative paths, and absolute paths materialise as symlinks.
3. **Greenfield remote.** A remote URL that does not yet exist may materialise as an empty Git repository with `origin` set and `.specify/project.yaml` bootstrapped from the registry capability.

Materialisation is idempotent:

- Existing remote slots fetch updates from `origin`.
- Existing local slots keep pointing at the same resolved target.
- A slot for the wrong project, wrong URL, or wrong materialisation type fails with an actionable diagnostic instead of being overwritten.
- `.specify/workspace/` and `.specify/.cache/` remain ignored by Git.

The workspace is temporary derived state, but it is not required to be deleted after every change. Keeping slots between changes is allowed so future syncs can fetch rather than reclone. Removing `.specify/workspace/<project>/` is always a valid way to force a fresh materialisation.

### Branch Preparation

Before a slice mutates a remote-backed slot, `/change:execute` must ensure the slot is on the change branch:

```text
specify/<change-name>
```

This is the core RFC-14 execution delta. Today branch creation happens late, during `workspace push`; RFC-14 moves the branch guard before file mutation so all slice output is produced on the branch that will be pushed.

Branch preparation:

1. Refuses to proceed when the slot has unrelated uncommitted changes.
2. Fetches `origin` if the slot is a remote checkout.
3. Creates or checks out `specify/<change-name>` from the configured base branch when the branch does not yet exist locally.
4. Reuses the local branch when resuming an in-progress change.
5. Refuses branch names that do not match the exact `specify/<change-name>` pattern.

Local-path symlink slots are handled conservatively: they may be inspected and modified only when the target repository can provide a usable `origin`. Push reports `local-only` when no remote can be resolved.

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
5. Record phase outcomes and plan status in the existing coordinator plan flow.

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
2. Verify the current branch is exactly `specify/<change-name>`.
3. Refuse dirty checkouts. `workspace push` does not create commits implicitly in the RFC-14 first landing; the execution loop or operator must commit completed slice output before push.
4. Push with `--force-with-lease` to `origin specify/<change-name>`.
5. Create or update a pull request for that branch.

Push is best-effort across projects: one project's failure does not prevent other selected projects from pushing. The final result reports `created`, `pushed`, `up-to-date`, `local-only`, or `failed` per project.

`--dry-run` performs all classification and remote-resolution checks without `git push`, repository creation, or pull-request creation.

### Merge And Finalize

`specify workspace merge [<project>...]` merges open PRs created from workspace branches.

Per project:

1. Locate the PR whose head branch is exactly `specify/<change-name>`.
2. Refuse to operate when the branch pattern does not match.
3. Inspect CI status.
4. Squash-merge only when required checks are green.
5. Never use force-merge, admin override, or auto-merge.

`specify change finalize` remains the coordinator-level closure command. It confirms all required per-project PRs are merged, archives the plan/change state, and may offer to remove clean workspace slots for the finalized change.

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

`specify workspace sync`, `/change:execute`, `workspace push`, and `workspace merge` MUST enforce:

- every selected project exists in `registry.yaml`;
- registry project names are unique and kebab-case;
- remote URLs are supported clone targets;
- slot paths stay under `.specify/workspace/<project>/`;
- no workspace slot escapes through path traversal or an unexpected symlink;
- dirty unrelated work blocks branch preparation;
- push and merge operate only on `specify/<change-name>`;
- `--force-with-lease` is the only force-style push allowed;
- merge refuses failing, pending, or missing required checks;
- `finalize --clean` refuses destructive removal when any selected workspace clone is dirty.

The workspace is derived state, but safety still matters because it contains real Git checkouts capable of pushing to remote origins.

## Non-Goals

- Running one slice across multiple repositories.
- Hiding Git or PR failures behind best-effort success.
- Force-merging PRs or bypassing CI.
- Making `.specify/workspace/` durable source state.
- Adding a standalone `specify workspace clean` command.

## Implementation Scope

### Phase 1 - Workspace Selection And Status

1. Add optional project selectors to `specify workspace sync` and `status`.
2. Preserve all-project defaults for human-invoked commands without selectors.
3. Extend `workspace status` output with branch, remote, change-branch match, and active-slice summary.
4. Validate unknown project names before touching the filesystem.

### Phase 2 - Materialisation Hardening

1. Make remote sync idempotent across clone, fetch, and greenfield bootstrap.
2. Refuse mismatched existing slots rather than overwriting them.
3. Harden symlink and path traversal checks under `.specify/workspace/`.
4. Keep `.specify/workspace/` and `.specify/.cache/` ignored.

### Phase 3 - Execute Against Workspace Slots

1. Teach `/change:execute` to resolve `entry.project` to a workspace slot.
2. Materialise missing selected slots before slice execution.
3. Prepare or resume `specify/<change-name>` before mutation.
4. Refuse unrelated dirty work before mutation.
5. Run `/spec:define`, `/spec:build`, and `/spec:merge` with the slot as project root.
6. Preserve coordinator-level plan locking and status transitions.

### Phase 4 - Push, Merge, And Cleanup

1. Ensure `workspace push` uses selected projects and reports per-project outcomes.
2. Ensure `workspace push` refuses dirty checkouts and creates or updates PRs from `specify/<change-name>`.
3. Ensure `workspace merge` refuses branch mismatches and non-green CI.
4. Keep cleanup on `change finalize --clean`, with dirty-clone refusal before removal.
5. Wire `change finalize` to verify merged per-project PRs before archiving.

### Repository Updates

1. Update `docs/reference/registry.md` with the RFC-14 workspace lifecycle.
2. Update `docs/reference/cli/workspace.md` for selectors, branch preparation, dirty-checkout push refusal, and finalize cleanup.
3. Update `/change:plan` sync-peers guidance to distinguish discovery sync from execution sync.
4. Update `/change:execute` guidance to run project entries inside materialised slots.
5. Add glossary entries for coordinator root, registry workspace, workspace slot, and change branch.

## Migration

RFC-14 is additive for existing projects:

- Single-repo projects without `registry.yaml` continue to run directly in the current repository.
- Existing `registry.yaml` files remain valid.
- Existing workspace slots remain valid when their URL and materialisation type match the registry entry.
- Operators may remove `.specify/workspace/` to force a fresh sync.
- Existing `workspace push`, `workspace merge`, and `change finalize --clean` behavior should remain compatible while gaining stricter selection and safety checks.

Acceptance criteria:

- A multi-project change can materialise only the project needed by the selected plan entry, while human-invoked `workspace sync` without selectors still syncs all registry projects.
- `/change:execute` can run a plan entry against a remote-backed workspace slot.
- `/change:execute` prepares `specify/<change-name>` before mutating a remote-backed slot.
- Completed local changes can be pushed to the corresponding remote origin on `specify/<change-name>`.
- `workspace push` refuses dirty checkouts instead of silently omitting uncommitted work.
- `workspace merge` refuses branch mismatches and non-green CI.
- Deleting `.specify/workspace/<project>/` and re-running sync recreates a usable slot.

## Open Questions

1. **Automatic commits.** RFC-14 first landing requires `workspace push` to refuse dirty checkouts. Should a later phase let `/change:execute` commit each completed slice automatically?
2. **Base branch selection.** Should branch preparation always use the remote default branch, or should registry entries allow an optional base branch?
3. **Greenfield remotes.** Should greenfield repository creation stay in `workspace push`, or should sync validate remote existence earlier?

## References

- [RFC-13: Immutable core + capability extensions](archive/rfc-13-extensibility.md) - capability protocol, platform components, and registry-materialised execution.
- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) - CLI conventions and project config.
- [RFC-2: Execution](archive/rfc-2-execution.md) - plan loop and dependency ordering.
- [RFC-3a: Monoliths](archive/rfc-3a-monoliths.md) - plan entry shape.
- [RFC-3b: Platform](archive/rfc-3b-platform.md) - cross-repo routing.
- [RFC-9: Platform](archive/rfc-9-platform.md) - registry/change root placement, workspace push, and PR merge.


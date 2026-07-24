# Cross-repo changes

Coordinate a Specify change across multiple repositories from a workspace. This tutorial assumes you completed the [Quick start](quick-start.md).

## What you will build

A workspace-scoped plan where `change.md`, `plan.yaml`, and `discovery.md` live at the workspace, while slice work runs in materialised project slots under top-level `workspace/<project>/`.

## Prerequisites

- Completed [Quick start](quick-start.md)
- A [registry-only workspace](../reference/configuration.md) or workspace-enabled project (`registry.yaml` describing peer repos)
- Git remotes configured for each registered project

## Step 1 — Initialise a workspace

For a registry-only workspace:

```text
/spec:init workspace
```

For a workspace registered inside a platform project, see [Registry](../reference/registry.md) and [Configuration files](../reference/configuration.md).

## Step 2 — Plan from the workspace

Run `/spec:plan` at the workspace (not from a project slot). Bind sources in the plan invocation. At propose time, route slices to registry slots with `specify plan add --project <name>`.

```text
/spec:plan platform-auth source docs=./design-notes/auth
```

Each slice row in `plan.yaml` may carry a `project:` field naming the registry slot where build and merge run.

## Step 3 — Materialize slots

Before planning or execution, use your normal repository tooling to create each required `workspace/<project>/` checkout or local-path symlink from `registry.yaml`. Prepare the branch, remote, and clean working tree according to the repository's own workflow.

Specify does not clone, refresh, or prepare workspace slots. Plan artifacts stay at the workspace; project-bound phase work runs in the matching materialized slot.

## Step 4 — Execute with workspace routing

```bash
specify plan approve
specify plan execute
```

When a slice targets a project slot, refine/build/merge run inside that checkout. Commits and branch management remain operator-owned.

## Step 5 — Publish and finalize

After execute drains, commit and publish every affected repository through its normal Git and forge workflow. Open and merge pull requests, or satisfy the equivalent publication gate, before finalizing:

```text
/spec:finalize <name>
```

Finalize confirms publication is complete, then runs `specify plan archive`. It does not perform Git or forge operations.

## What you learned

- Workspace plans centralise plan artifacts in the workspace.
- Per-slice `project` routes phase work into registry slots.
- Operators materialize slots and publish repository changes outside Specify.
- Finalize archives only after publication is complete.

## Next steps

- [Registry](../reference/registry.md) — `registry.yaml` format
- [Workspace topology](../reference/cli/workspace.md) — slots, topology lock, and operator-owned publication
- [Drop down a layer](../how-to/drop-down-a-layer.md) — manual CLI when automation fails

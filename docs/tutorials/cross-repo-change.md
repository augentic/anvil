# Cross-repo changes

Coordinate a Specify change across multiple repositories from a workspace hub. This tutorial assumes you completed the [Quick start](quick-start.md).

## What you will build

A workspace-scoped plan where `change.md`, `plan.yaml`, and `discovery.md` live at the workspace root, while slice work runs in materialised project slots under `.specify/workspace/<project>/`.

## Prerequisites

- Completed [Quick start](quick-start.md)
- A [registry-only platform hub](../reference/configuration.md) or workspace-enabled project (`registry.yaml` describing peer repos)
- Git remotes configured for each registered project

## Step 1 — Initialise a workspace hub

For a registry-only hub:

```text
/spec:init hub
```

For a workspace registered inside a platform project, see [Registry](../reference/registry.md) and [Configuration files](../reference/configuration.md).

## Step 2 — Plan from the workspace root

Run `/spec:plan` at the workspace root (not from a project slot). Bind sources in the plan invocation. At propose time, route slices to registry slots with `specify plan add --project <name>`.

```text
/spec:plan platform-auth source docs=./design-notes/auth
```

Each slice row in `plan.yaml` may carry a `project:` field naming the registry slot where build and merge run.

## Step 3 — Sync and prepare slots

Before execute, materialise peer clones:

```bash
specify workspace sync
specify workspace prepare <project> --change <name>
```

`/spec:execute` calls sync and prepare as it routes into each slot. Plan artifacts stay at the workspace root; phase skills `chdir` into `.specify/workspace/<project>/`.

## Step 4 — Execute with workspace routing

```bash
specify plan transition <name> reviewed
/spec:execute
```

When a slice targets a project slot, refine/build/merge run inside that clone. Residue commits and merge commits follow workspace rules documented in [specify workspace](../reference/cli/workspace.md).

## Step 5 — Push and finalize

After execute drains:

```text
/spec:finalize <name>
```

Finalize runs `specify workspace push` to publish `specify/<change-name>` branches as pull requests, observes each PR until `MERGED`, then archives the plan. PR merges remain operator-owned.

## What you learned

- Workspace plans centralise plan artifacts at the hub root.
- Per-slice `project` routes phase work into registry slots.
- Finalize pushes branches and waits for merged PRs before archive.

## Next steps

- [Registry](../reference/registry.md) — `registry.yaml` format
- [specify workspace](../reference/cli/workspace.md) — sync, prepare, push CLI reference
- [Drop down a layer](../how-to/drop-down-a-layer.md) — manual CLI when automation fails

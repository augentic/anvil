# Workspace topology

Emery preserves workspace topology as a planning concept, but it exposes no `emery workspace` command group. Slot materialization, branch preparation, commits, publication, pull requests, and merges are operator-owned repository operations outside Emery.

## Workspace slots

A registry-only workspace declares projects in `registry.yaml`. Each project may be represented locally at top-level `workspace/<project>/`; plan validation and execution use that path when a slice carries the matching `project` binding.

The operator or surrounding automation must create and refresh slots before planning or execution:

- Remote repository URLs normally map to an ordinary checkout or worktree.
- Local paths may map to a symlink.
- The slot's `.emery/project.yaml` and baseline must match the committed `.emery/topology.lock` projection used by plan validation.

Emery does not clone repositories, create worktrees, resolve default branches, create change branches, or repair stale slots.

## Topology lock

`.emery/topology.lock` is the committed plan-time projection of member project metadata and baseline routing identity. It remains part of workspace validation, but regeneration is operator-owned. Never hand-edit it as an ad hoc way to bypass a validation finding; regenerate it through the repository's chosen topology tooling and review the resulting diff.

`emery plan validate` may report stale or mismatched workspace topology when a materialized slot diverges from `registry.yaml` or `.emery/topology.lock`. Resolve the repository state outside Emery, then rerun plan validation.

## Publication and finalization

After `emery plan execute` drains, the operator commits and publishes each affected repository through its normal Git and forge workflow. Open and merge pull requests, or complete any equivalent publication gate, before finalization.

`/emery:finalize` does not publish branches. It verifies the plan is drained and runs `emery plan archive` only after the operator confirms publication is complete.

## See also

- [Cross-Repo Changes](../../tutorials/cross-repo-change.md) -- tutorial for multi-repo workflows
- [Configuration Files](../configuration.md) -- `registry.yaml` and `plan.yaml` format
- [emery plan execute](plan.md#emery-plan-execute) -- the guest-routed driver loop
- [`emery plan archive`](plan.md) -- archive verb used by `/emery:finalize`

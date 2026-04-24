# /spec:plan

Author `.specify/plan.yaml` for a new initiative.

## Synopsis

```text
/spec:plan <initiative-name> \
    [--from <path>...]          # documentation inputs
    [--against <path>]          # existing codebase to delta against
    [--source <key>=<path>...]  # named legacy-code sources
    [--focus <area>]            # scoping hint
    [--extend]                  # add to existing plan
    [--dry-run]                 # preview only, write nothing
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `initiative-name` | Yes | Kebab-case name for the initiative |
| `--from` | No | Documentation files to analyse (PRDs, design docs) |
| `--against` | No | Existing codebase to delta against |
| `--source` | No | Named legacy-code sources (e.g. `monolith=/path/to/legacy`) |
| `--focus` | No | Scoping hint to narrow discovery |
| `--extend` | No | Add changes to an existing plan |
| `--dry-run` | No | Preview the plan without writing anything |

## When to use

- You have a body of work that will span multiple changes (migration, greenfield build, modernisation).
- You want a structured plan with dependency ordering before you start executing.
- You have legacy code or documentation to analyse as input.

## Artifacts produced

| Artifact | Location | Content |
|----------|----------|---------|
| `plan.yaml` | `.specify/plan.yaml` | Ordered change list with dependencies and status |
| `discovery.md` | `.specify/plans/<name>/discovery.md` | Capability inventory from input analysis |
| `proposal.md` | `.specify/plans/<name>/proposal.md` | Audit trail of slice accept/edit/reject decisions |
| `workspace.md` | `.specify/plans/<name>/workspace.md` | Peer inventory for cross-repo planning (multi-repo only) |
| `metadata.json` | `.specify/plans/<name>/analyze/<key>/metadata.json` | Source-tree structural metadata (legacy-code only) |

## Behavior

The skill runs a fixed three-phase internal flow:

1. **Analyse inputs.** Dispatches every input to `/spec:analyze`, which branches on `kind` (`legacy-code` or `documentation`) and emits capability summaries into `discovery.md`.
2. **Sync peers.** *(Runs only when `registry.yaml` declares more than one project.)* Clones every registry project into `.specify/workspace/<project>/` and inventories each repo's baseline specs. Emits `workspace.md`.
3. **Generate plan.** Combines the capability inventory with the peer inventory (when present) into an ordered, dependency-aware list of changes. Presents each proposed change ("slice") for interactive accept / edit / reject. Writes accepted slices via `specify initiative create`.

After the loop, runs `specify initiative validate` to check structural integrity (no cycles, no dangling dependencies).

## Lifecycle transitions

Creates plan entries in `pending` state via `specify initiative create`.

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| No inputs provided | Neither `--from`, `--against`, nor `--source` given | Supply at least one input |
| Source path not found | `--source` references a path that does not exist | Check the path |
| Registry validation failure | `registry.yaml` is malformed or missing required fields | Fix `registry.yaml` |
| Plan validation failure | Generated plan has dependency cycles or dangling references | Review and adjust proposed changes |

## Examples

```text
# Plan a migration from a legacy codebase
/spec:plan migrate-to-v2 --source monolith=/path/to/legacy

# Plan from documentation
/spec:plan new-platform --from ./docs/prd.md --from ./docs/architecture.md

# Plan with a focus area
/spec:plan auth-overhaul --source legacy=./src --focus authentication

# Preview without writing
/spec:plan migrate-to-v2 --source monolith=/path/to/legacy --dry-run

# Extend an existing plan
/spec:plan migrate-to-v2 --source payments=./src/payments --extend
```

## See also

- [/spec:execute](execute.md) -- drive the authored plan
- [/spec:analyze](analyze.md) -- the discovery skill invoked during planning
- [Configuration Files](../configuration.md) -- `plan.yaml` and `registry.yaml` format
- [Tutorial: Multi-Change Initiative](../../tutorials/single-repo-initiative.md) -- walkthrough
- [Tutorial: Cross-Repo Initiatives](../../tutorials/cross-repo-initiative.md) -- multi-repo walkthrough

# /change:plan

Author `plan.yaml` for a change.

> **Renamed.** This skill was previously `/change:plan`. RFC-13 §3.9 moved it to the `change` plugin as `/change:plan`. The historical command remains as a deprecation shim that delegates here and is removed before the post-RFC-13 release; see [RFC-13 §Migration](../../../rfcs/archive/rfc-13-extensibility.md#migration).

## Synopsis

```text
/change:plan <change-name> \
    [from <path>...]          # documentation inputs
    [against <path>]          # existing codebase to delta against
    [source <key>=<path>...]  # named legacy-code sources
    [focus <area>]            # scoping hint
    [extend]                  # add to existing plan
    [dry-run]                 # preview only, write nothing
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `change-name` | Yes | Kebab-case name for the change |
| `--from` | No | Documentation files to analyse (PRDs, design docs) |
| `--against` | No | Existing codebase to delta against |
| `--source` | No | Named legacy-code sources (e.g. `monolith=/path/to/legacy`) |
| `--focus` | No | Scoping hint to narrow discovery |
| `--extend` | No | Add slices to an existing plan |
| `--dry-run` | No | Preview the plan without writing anything |

## When to use

- You have a body of work that will span multiple slices (migration, greenfield build, modernisation).
- You want a structured plan with dependency ordering before you start executing.
- You have legacy code or documentation to analyse as input.

## Artifacts produced

| Artifact | Location | Content |
|----------|----------|---------|
| `plan.yaml` | `plan.yaml` | Ordered slice list with dependencies and status |
| `discovery.md` | `.specify/plans/<name>/discovery.md` | Capability inventory from input analysis |
| `proposal.md` | `.specify/plans/<name>/proposal.md` | Audit trail of slice accept/edit/reject decisions |
| `workspace.md` | `.specify/plans/<name>/workspace.md` | Peer inventory for cross-repo planning (multi-repo only) |
| `metadata.json` | `.specify/plans/<name>/analyze/<key>/metadata.json` | Source-tree structural metadata (legacy-code only) |

## Behavior

The skill runs a fixed flow:

1. **Analyse inputs.** Dispatches every input to `/spec:analyze`, which branches on `kind` (`legacy-code` or `documentation`) and emits capability summaries into `discovery.md`.
2. **Sync peers.** *(Runs only when `registry.yaml` declares more than one project.)* Clones every registry project into `.specify/workspace/<project>/` and inventories each repo's baseline specs. Emits `workspace.md` with per-project `Description` and `Schema` bullets from `registry.yaml`.
3. **Generate plan (propose).** Combines the capability inventory with the peer inventory (when present) into an ordered, dependency-aware list of slices. Presents each proposed slice for interactive accept / edit / reject. Writes accepted slices via `specify change plan add` (without `--project`).
4. **Assignment (multi-repo only).** When `workspace.md` contains more than one project, infers a target project for each new entry using description match, baseline spec affinity, and schema compatibility from `workspace.md`. Presents the full assignment table for operator review and override. Writes each assignment via `specify change plan amend <name> --project <project>`. Appends the assignment rationale to `proposal.md`.

### Contract authorship patterns

`/change:plan` automatically determines how API contracts enter the plan based on registry topology, source declarations, and API boundary detection. Three patterns emerge:

**Contract-first (dedicated contract slice).** When the plan contains slices in multiple projects that share an API boundary, `/change:plan` inserts a dedicated contract slice before the implementation slices on both sides. The contract slice uses `schema: contracts@v1` (no `project`) and defines interface-level behavioral specs. Implementation slices `depends-on` the contract slice and validate alignment:

```yaml
changes:
  - name: user-api-contract
    schema: contracts@v1
    description: "Define the user registration API contract"
    status: pending

  - name: user-api-backend
    project: backend
    depends-on: [user-api-contract]
    status: pending

  - name: registration-screen
    project: mobile
    depends-on: [user-api-contract]
    status: pending
```

**Contract-given (external or legacy contracts).** When a source is flagged as an external system or legacy API, `/change:plan` inserts an import slice before the implementation slices. The operator places the external contract files into the import slice's `contracts/` directory.

**Spec-first (inline derivation).** For single-repo slices with no identified API boundary and no external consumers, no separate contract slice is inserted. The `contracts` brief derives interface shapes inline during the slice's define phase.

After the loop, runs `specify change plan validate` to check structural integrity (no cycles, no dangling dependencies) and the RFC-3b cross-registry checks (`project-not-in-registry`, `project-missing-multi-repo`, `description-missing-multi-repo`, `schema-mismatch-workspace`).

## Lifecycle transitions

Creates plan entries in `pending` state via `specify change plan add`.

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| No inputs provided | Neither `--from`, `--against`, nor `--source` given | Supply at least one input |
| Source path not found | `--source` references a path that does not exist | Check the path |
| Registry validation failure | `registry.yaml` is malformed or missing required fields | Fix `registry.yaml` |
| Plan validation failure | Generated plan has dependency cycles or dangling references | Review and adjust proposed slices |

## Examples

```text
# Plan a migration from a legacy codebase
/change:plan migrate-to-v2 source monolith=/path/to/legacy

# Plan from documentation
/change:plan new-platform from ./docs/prd.md from ./docs/architecture.md

# Plan with a focus area
/change:plan auth-overhaul source legacy=./src focus authentication

# Preview without writing
/change:plan migrate-to-v2 source monolith=/path/to/legacy dry-run

# Extend an existing plan
/change:plan migrate-to-v2 source payments=./src/payments extend
```

## See also

- [/change:execute](execute.md) -- drive the authored plan
- [/spec:analyze](analyze.md) -- the discovery skill invoked during planning
- [Configuration Files](../configuration.md) -- `plan.yaml` and `registry.yaml` format
- [Tutorial: A Multi-Slice Change](../../tutorials/single-repo-change.md) -- walkthrough
- [Tutorial: Cross-Repo Changes](../../tutorials/cross-repo-change.md) -- multi-repo walkthrough

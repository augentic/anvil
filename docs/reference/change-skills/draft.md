# /change:draft

Author `plan.yaml` for a change and stop at the operator review seam.

`/change:draft` is the authoring stage of the three-skill change lifecycle (`/change:draft → operator review → /change:execute loop → /change:finalize`). It runs the planning brief pipeline, produces a validated `plan.yaml`, and hands back to the operator — it never starts execution itself.

The skill lives at [`plugins/change/skills/draft/SKILL.md`](../../../plugins/change/skills/draft/SKILL.md); the runbook with the verbatim step bodies, invocation grammar, and mode deltas lives at [`plugins/change/skills/draft/references/runbook.md`](../../../plugins/change/skills/draft/references/runbook.md).

## Synopsis

```text
/change:draft <change-name> \
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

- You have a body of work that will span multiple slices (migration, greenfield build, modernisation) and want a structured plan with dependency ordering before you start executing.
- You have legacy code or documentation to analyse as input.
- You want a deliberate review seam between authoring the plan and executing it — `/change:draft` ends at hand-off; the operator decides when to start `/change:execute loop`.

## Lifecycle position

```text
/change:draft  →  operator review  →  /change:execute loop  →  /change:finalize
```

There is no umbrella mode and no automatic transition into execution. The pause between draft and execute is the design; the operator decides when to start `/change:execute loop`.

## Critical Path

The skill runs a fixed six-step loop driven by the active capability's `capability.yaml`. See [`plugins/change/skills/draft/references/runbook.md`](../../../plugins/change/skills/draft/references/runbook.md) for the verbatim step bodies.

1. **Pre-flight** — validate `<change-name>` as kebab-case; require at least one of `from`, `against`, `source`, or a populated `change.md:inputs`; refuse if `plan.yaml` already exists (unless `extend`).
2. **Brief scaffold** — `specify change draft <change-name> [--source <key>=<path-or-url> ...]`. Writes `change.md` and `plan.yaml` together (atomic refusal if either already exists). Skipped under `extend`.
3. **Registry validate** — `specify registry validate`. Halts on validation failures (description-missing, kebab violations, etc.) before any brief work.
4. **Plan brief pipeline** — discovery → [sync-workspace, multi-repo only] → propose → [assignment, multi-repo only], with optional survey + synthesise sub-steps when active. Discovery dispatches through `/change:analyze`; propose iterates accept/edit/reject per slice and writes via `specify plan add`; assignment infers the project per entry and may run a registry-proposal sub-step when a row names a project that does not exist yet.
5. **Validate** — `specify plan validate`. Non-zero exit on any `Error`-level finding.
6. **Hand-off summary** — print the slice count, the target projects, and any `Warning`-level validate findings the operator should be aware of before executing. Point the operator at `specify plan status` for review, `specify plan amend` for edits, and `/change:execute loop` for the next stage.

## Artifacts produced

| Artifact | Location | Content |
|----------|----------|---------|
| `plan.yaml` | `plan.yaml` | Ordered slice list with dependencies and status |
| `change.md` | `change.md` | Operator brief written by step 2 alongside `plan.yaml` |
| `discovery.md` | `.specify/plans/<name>/discovery.md` | Capability inventory from input analysis |
| `proposal.md` | `.specify/plans/<name>/proposal.md` | Audit trail of slice accept/edit/reject decisions |
| `workspace.md` | `.specify/plans/<name>/workspace.md` | Peer inventory for cross-repo planning (multi-repo only) |
| `metadata.json` | `.specify/plans/<name>/analyze/<key>/metadata.json` | Source-tree structural metadata (legacy-code only) |

## Modes

- **default** — full six-step loop.
- **`extend`** — append-only; skip step 2 and reuse discovery. Only `specify plan amend --project` may touch newly added entries — never pre-existing ones.
- **`dry-run`** — read-only preview; suppress every write under `.specify/`.

## Contract authorship patterns

`/change:draft` automatically determines how API contracts enter the plan based on registry topology, source declarations, and API boundary detection. Three patterns emerge:

**Contract-first (dedicated contract slice).** When the plan contains slices in multiple projects that share an API boundary, `/change:draft` inserts a dedicated contract slice before the implementation slices on both sides. The contract slice uses `capability: contracts@v1` (no `project`) and defines interface-level behavioral specs. Implementation slices `depends-on` the contract slice and validate alignment:

```yaml
changes:
  - name: user-api-contract
    capability: contracts@v1
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

**Contract-given (external or legacy contracts).** When a source is flagged as an external system or legacy API, `/change:draft` inserts an import slice before the implementation slices. The operator places the external contract files into the import slice's `contracts/` directory.

**Spec-first (inline derivation).** For single-repo slices with no identified API boundary and no external consumers, no separate contract slice is inserted. The `contracts` brief derives interface shapes inline during the slice's define phase.

After the loop, `specify plan validate` checks structural integrity (no cycles, no dangling dependencies) and the cross-registry checks (`project-not-in-registry`, `project-missing-multi-repo`, `description-missing-multi-repo`, `capability-mismatch-workspace`).

## Lifecycle transitions

Creates plan entries in `pending` state via `specify plan add`. Status transitions are owned by `/change:execute` (and `specify plan transition` as the underlying CLI).

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| No inputs provided | Neither `--from`, `--against`, nor `--source` given (and `change.md:inputs` is empty) | Supply at least one input |
| Source path not found | `--source` references a path that does not exist | Check the path |
| Registry validation failure | `registry.yaml` is malformed or missing required fields | Fix `registry.yaml` |
| Plan validation failure | Generated plan has dependency cycles or dangling references | Review and adjust proposed slices |
| `plan.yaml` already exists | A previous draft is still on disk | Use `extend`, or archive the stale plan via `specify plan archive` |

## Examples

```text
# Plan a migration from a legacy codebase
/change:draft migrate-to-v2 source monolith=/path/to/legacy

# Plan from documentation
/change:draft new-platform from ./docs/prd.md from ./docs/architecture.md

# Plan with a focus area
/change:draft auth-overhaul source legacy=./src focus authentication

# Preview without writing
/change:draft migrate-to-v2 source monolith=/path/to/legacy dry-run

# Extend an existing plan
/change:draft migrate-to-v2 source payments=./src/payments extend
```

## See also

- [/change:execute](execute.md) — drive the authored plan through define-build-merge per slice.
- [/change:finalize](finalize.md) — push branches, observe PR state, run `specify change finalize` once every PR is `MERGED`.
- [/change:analyze](analyze.md) — the discovery skill invoked during planning.
- [Configuration Files](../configuration.md) — `plan.yaml` and `registry.yaml` format.
- [Tutorial: A Multi-Slice Change](../../tutorials/single-repo-change.md) — walkthrough.
- [Tutorial: Cross-Repo Changes](../../tutorials/cross-repo-change.md) — multi-repo walkthrough.

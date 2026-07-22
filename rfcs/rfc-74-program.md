# Migration Programs and Durable Progress

> Status: Draft — nothing landed
>
> Owns: a migration-sized umbrella above changes, repository-by-repository scheduling, target selection policy, approved adapter/topology decisions, durable progress, re-entry, migration audit projections.
>
> Depends on: [RFC-71](rfc-71-discovery.md) Stage 1, [RFC-72](rfc-72-migration.md). [RFC-73](rfc-73-materialization.md) optional for the walking skeleton.
>
> Supersedes: [archive/rfc-22-ledger.md](archive/rfc-22-ledger.md).

## Intent

Coordinate work that spans many repositories and many Specify changes. Each work item still uses the existing change → slice refine → build → merge loop; the program schedules batches and records progress.

## First delivery (Stages 1–2)

1. Serial coordinator over an approved program plan
2. Program Gate M1 (operator approval of topology / adapter decisions)
3. Substrate table pointing at RFCs 70–73 for deployment, discovery, intake, and optional materialization

## Deferred

- Rich `progress.yaml` (Stage 3)
- Parallelism across repositories
- Forge / hosted runner integration
- Requiring [RFC-73](rfc-73-materialization.md) before clone friction demands it — operator-prepared slots suffice first

## Non-goals

- Replacing Gate 1 / slice lifecycle with a second lifecycle authority
- Moving publication/merge of PRs into Specify

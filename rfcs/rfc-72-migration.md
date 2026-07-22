# Migration Intake and Source Selection

> Status: Draft — nothing landed
>
> Owns: durable source membership (`sources.yaml`), source materialization, repository profile schema + profiler, source-adapter selection policy, recommendation/approval, lowering approved sources into change plans.
>
> Depends on: [RFC-71](rfc-71-discovery.md) Stage 1. Supersedes: [archive/rfc-21-catalogue.md](archive/rfc-21-catalogue.md).

## Intent

Let an operator provide repositories and supporting inputs once, then reuse them across many Specify changes. Keep source inputs (`sources.yaml`) separate from target projects (`registry.yaml`) and per-change bindings (`plan.yaml`).

## First delivery (Stages 1–3, serial)

1. CLI-owned `sources.yaml` membership
2. Git + docs snapshot materialization into an out-of-tree cache
3. Repository profile + deterministic profiler
4. Recommend → approve exact source bindings
5. Lower approved `@key` bindings into `specify plan author`

## Deferred

- Captures / screenshots intake as first-class membership kinds beyond path binds
- Multi-binding auto-composition
- Auto-approve policy
- `--jobs` parallelism, prune, portal import

## Non-goals

- Replacing `plan.yaml` source bindings for ordinary single-repo work
- Putting regenerable source snapshots under `.specify/cache/`

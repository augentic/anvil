# Adapter Descriptors and Registry Trust

> Status: Draft — nothing landed
>
> Owns: install-time adapter descriptors, closed discovery vocabularies, registry descriptor projection, publisher/namespace trust, deterministic candidate filtering and explanation, recommendation-report currency.
>
> Depends on: [RFC-70](rfc-70-deployment.md). Consumed by: [RFC-72](rfc-72-migration.md), [RFC-74](rfc-74-program.md).

## Intent

Give Specify a typed `AdapterDescriptor` substrate so engine core does not hard-code adapter names for discovery. Source selection ("what can inspect this input?") and target selection ("what should this project become?") share the substrate but keep different owners ([RFC-72](rfc-72-migration.md) / [RFC-74](rfc-74-program.md)).

## First delivery (Stages 1–2)

1. Descriptor shape authored beside each adapter and projected into a static first-party index
2. Closed discovery vocabularies (including workload kinds)
3. Deterministic candidate filtering + structured explanation
4. Immutable recommendation-report currency and invalidation rules

## Deferred

- Model adjudication over candidates
- `describe` WIT operation
- Registry / publisher / namespace trust policy (Stage 3)
- Ranking beyond deterministic filters

## Non-goals

- Auto-executing an arbitrary package from the network
- Treating source implementation shape as desired target architecture

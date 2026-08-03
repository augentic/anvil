# Adapter Descriptors and Registry Trust

> **Status: Superseded (archived).** Source selection ownership moved to [RFC-88 Detached Changes](../rfc-88-detached-changes.md#source-adapter-selection). Do not implement this document; historical prior art only (filename keeps the old number; active [RFC-71](../rfc-71-deployment.md) is deployment).
>
> Owns: install-time adapter descriptors, closed discovery vocabularies, registry descriptor projection, publisher/namespace trust, deterministic candidate filtering and explanation, recommendation-report currency.

## Intent

Give Emery a typed `AdapterDescriptor` substrate so engine core does not hard-code adapter names for discovery. Source selection ("what can inspect this input?") and target selection ("what should this project become?") share the substrate but keep different owners ([RFC-72](rfc-72-migration.md) / [RFC-88](../rfc-88-detached-changes.md)).

Installation is already solved: a pinned identity pulls from the compiled first-party GHCR mapping on miss ([RFC-71](../rfc-71-deployment.md) Stage 3), and the mapping accepts any kebab-case name. What is missing is the fact that decides *which* pin to install. Today a source adapter's whole metadata record is `emery-floor`, so nothing on the wire says a component reads TypeScript or builds mobile shells.

## Two descriptor faces

One substrate, one predicate per axis:

- **source — `inspects`**: which languages, manifest sentinels, and framework idioms the adapter can survey and extract, plus the workload kinds it recognises.
- **target — `produces`**: which workload kinds the adapter can build.

Illustrative shape, authored beside each adapter in `augentic/emery-adapters`:

```yaml
# sources/typescript/descriptor.yaml
axis: source
name: typescript
emery-floor: 0.28.0
inspects:
  languages: [typescript, javascript]
  manifests: [package.json, tsconfig.json]
  frameworks: [express, fastify, nestjs, next, bullmq, node-cron]
  workloads: [service, web-frontend, batch, cli]
```

```yaml
# targets/vectis/descriptor.yaml
axis: target
name: vectis
emery-floor: 0.28.0
produces:
  workloads: [mobile-app]
```

Platforms stay off the descriptor. A target's `PlatformsCapability` is not a selection criterion — it becomes a decision only after the target is approved, at which point the component is installed and its `metadata` export is authoritative. Platform validation already exists at init (`project-platforms-required`).

## Workload kinds

The closed `workload-kind` vocabulary is the join key between a repository profile ([RFC-72](rfc-72-migration.md)) and a target's `produces` ([RFC-88](../rfc-88-detached-changes.md)). Every value below is grounded in existing adapter grammar rather than speculative:

| Kind | Meaning |
| --------------- | -------------------------------------------------------------------------- |
| `service` | Request/response or message-handling backend |
| `web-frontend` | Browser-delivered UI |
| `mobile-app` | iOS / Android application |
| `library` | Reusable code unit with no independent deployment surface |
| `batch` | Scheduled or queue-driven work with no synchronous caller |
| `cli` | Operator-invoked command surface |

Adapters may be workload-neutral. `contracts` produces interface artifacts for any workload, so it declares no `produces.workloads` filter and is always a candidate.

## Decisions

| # | Decision | Consequence |
| - | -------- | ----------- |
| D1 | The descriptor is authored beside its adapter and projected into a static index shipped with the host. | Discovery never downloads a candidate to ask what it is for. Filtering works over adapters the project has never installed. |
| D2 | Keep the `describe` WIT operation deferred. | The component's `metadata` export stays the post-install authority; the index is the pre-install projection. No second runtime dispatch path. |
| D3 | Restated fields (`emery-floor`, and any future field appearing in both) are advisory in the index and authoritative in `metadata`; the adapter publish gate asserts the two agree. | Drift is caught at publish in [RFC-77](../rfc-77-release-process.md), not at an operator's resolve. |
| D4 | Two axis-specific predicates (`inspects` / `produces`) over one shared vocabulary set. | Source and target selection reuse the filter kernel without sharing a policy owner. |
| D5 | Filtering is deterministic and total: every catalogued adapter is either a candidate or excluded with a recorded reason. | The recommendation explains itself. No silent drops, so an operator can tell "no adapter reads Java" from "the Java adapter's floor is above your host". |
| D6 | `emery-floor` participates in filtering. | A candidate the running host cannot dispatch is excluded with that reason up front, instead of being recommended and then failing at the metadata gate. |
| D7 | Discovery emits an immutable recommendation report and stops. It never installs, binds, or writes config. | Approval is the consumer's gate — plan-time Gate 1 for sources ([RFC-72](rfc-72-migration.md)), Gate M1 for targets ([RFC-88](../rfc-88-detached-changes.md)). |
| D8 | The report records the inputs it was computed from (profile digest, index revision, host version) and is invalidated when any of them change. | A stale recommendation cannot be approved after the profile moved underneath it. |
| D9 | The first index covers the `emery:` namespace only. | Third-party descriptors, publisher identity, and namespace trust stay Stage 3, gated on roadmap RM-21 rather than on the migration program. |

## First delivery

Stages 1–2:

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
- Treating source implementation shape as desired target architecture — `inspects` matching says which adapter can *read* a repository, never what the repository should *become*
- Replacing the compiled first-party GHCR mapping with descriptor-supplied locations

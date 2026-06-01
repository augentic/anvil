# RFC-36: Registry as Projection — Single-Authored Project Facets and a Derived Topology Cache

> Status: Draft — Depends: [RFC-29b](rfc-29b-reconciliation.md) (plan-time `projects[]` topology) — Related: [roadmap principle "Treat `registry.yaml` as a projection"](roadmap.md#principles)

Today a project's adapter and description live in **two** authored homes: each project's `.specify/project.yaml`, and the platform hub's `registry.yaml`. The hub copy wins at plan time (`hub_topology` reads `registry.yaml`), so a stale registry silently overrides the project's own truth, and the `capabilities` / `keywords` facets added to `project.yaml` for slice-to-project routing never reach the reconciliation envelope at all. Drift is possible because one fact has two writers.

This RFC removes that drift class by inverting authority and adding a derived cache:

- **Each project owns what it is.** `adapter`, `description`, `capabilities`, and `keywords` are authored only in that project's `.specify/project.yaml`.
- **The registry owns what a project cannot know about itself** — that it is a member of this platform and where it lives: `name` + `url` (plus the cross-project `contracts` wiring, and an optional `adapter` used *only* as a greenfield scaffold seed).
- **A committed, machine-written `.specify/topology.lock`** projects each member project's authored facets into the plan-time topology. `specrun workspace sync` regenerates it; a staleness check verifies it; nobody hand-edits it.

## Decision

| ID | Decision |
| -- | -------- |
| **D36-1 Authority inversion** | Project-describing facets (`adapter`, `description`, `capabilities`, `keywords`) are authored solely in each project's `.specify/project.yaml`. `registry.yaml` is reduced to membership + location (`name`, `url`, optional `contracts`, optional `adapter` seed). The registry no longer authors a project's target adapter or description for topology purposes. |
| **D36-2 Derived topology cache** | `specrun workspace sync` regenerates a committed `.specify/topology.lock` by reading each materialised slot's `project.yaml`, resolving its target adapter to `name@vN`, and recording `{ name, target, description?, capabilities[], keywords[] }` per project. The lock is machine-written (write-if-changed) and is the only writer-owned topology artifact; operators never hand-edit it. |
| **D36-3 Hub topology reads the cache** | `hub_topology` builds the reconciliation `projects[]` from `.specify/topology.lock`, not from `registry.yaml`. A missing cache fails with `topology-cache-missing` directing the operator to run `specrun workspace sync`. A single regular (non-hub) project is unaffected: it reads `project.yaml` live as its own single source of truth. |
| **D36-4 Capabilities reach the envelope** | `capabilities[]` and `keywords[]` flow from `project.yaml` through the cache into the reconciliation request `projects[]`, so the agent binds slices on capability tags, not description prose alone. |
| **D36-5 Staleness, not synchronisation** | A `topology-cache-stale` plan-validate finding compares the committed cache against each slot's current `project.yaml`. Divergence is a stale cache (CI-blockable, one-command fix: `workspace sync`), never a silent override and never a top-down overwrite of an authored file. |

## Why projection, not synchronisation

A top-down sync (registry overwrites `project.yaml`) keeps two authored homes and papers over the duplication with a clobbering writer: it overwrites team edits, needs conflict rules, and forces the platform hub to author facts it does not own. The durable fix is to give every fact exactly one authored home and make any registry-side or cache-side copy *derived*, never authored.

`.specify/topology.lock` follows the lockfile discipline already used by `.specify/context.lock`: a manifest (`project.yaml`) is hand-authored; the resolved snapshot (`topology.lock`) is machine-derived, committed for offline/pre-survey availability, and verified in CI. "Sync" stops being an authoritative overwrite and becomes an idempotent regenerate-and-verify.

## Operator Surface

```bash
specrun workspace sync [<project>...]   # regenerates .specify/topology.lock as a side effect
specrun plan validate                   # surfaces topology-cache-stale / topology-cache-missing
specrun registry add <name> --url <url> [--adapter <seed>] [--description <text>]
```

`registry add --adapter` is now optional and, when present, is a greenfield scaffold seed only — the value written into a brand-new project's `project.yaml` when `workspace sync` clones an empty repo. Once a project's `project.yaml` exists, it is authoritative and the seed is never read again.

## Topology cache shape

`.specify/topology.lock` is a committed YAML document validated against `topology-lock.schema.json`:

```yaml
version: 1
projects:
  - name: identity-contracts
    target: contracts@v1
    description: "Versioned API contracts crate for the identity domain."
    capabilities: [contracts]
  - name: identity-service
    target: omnia@v1
    description: "Omnia identity service implementing auth and password flows."
    capabilities: [auth, password-reset]
    keywords: [identity, session]
```

Each entry is the resolved projection of one member project's `project.yaml`: `name` is the registry slot name (the binding key written to `plan.yaml.slices[].project`), `target` is the project's `adapter` resolved to `name@vN`, and `description` / `capabilities` / `keywords` are copied from the project's own config. Empty `capabilities` / `keywords` stay off the wire.

## Identity stays the registry slot name

The cache keys on the registry slot `name`, not `project.yaml.name`. `name` is what the agent binds, what the kernel writes to `plan.yaml.slices[].project`, and what build-time fan-out resolves against `registry.yaml` ([RFC-29c §"Per-slice fan-out"](rfc-29c-synthesis.md)). Only `target`, `description`, `capabilities`, and `keywords` are sourced from `project.yaml`. The split is "registry = addressing key; project.yaml = what the project is."

## Greenfield seed

When the registry names a project that does not exist yet, `workspace sync` clones an empty repo and scaffolds its `project.yaml`. At that instant there is no `project.yaml` to read an adapter from, so the registry entry's optional `adapter` is used as the scaffold seed. The moment the project's `project.yaml` exists it is authoritative; the seed is never read for topology. This is the only case where the registry legitimately supplies a project-describing value, and it is "author once at creation, derive thereafter".

## Staleness

`specrun plan validate` reads `.specify/topology.lock` and compares each entry against the current `project.yaml` of its materialised slot:

- A cache entry whose `target` / `description` / `capabilities` / `keywords` no longer match the slot's `project.yaml` emits `topology-cache-stale` (warning) with the fix `specrun workspace sync`.
- `propose --dry-run` / `--from` run the same guard, so reconciliation never binds against a stale cache silently.
- An absent cache in a hub fails `hub_topology` with `topology-cache-missing`.

CI verification reuses the exit-2 gate of `specrun plan validate`; regeneration is `specrun workspace sync`. There is no hand-edit path and no `--check` flag — the lockfile generate-if-changed discipline mirrors `.specify/context.lock`.

## Wire contracts

This RFC appends to [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts):

- **Schema:** `topology-lock.schema.json` (`TOPOLOGY_LOCK_JSON_SCHEMA`), validating `.specify/topology.lock`.
- **`proposal.schema.json` `$defs/projectRef`:** gains optional `capabilities[]` and `keywords[]`; the hub source of `projects[]` is restated as "the topology cache" rather than "a `registry.yaml#/projects[]` entry".
- **Validation codes:** `topology-cache-missing` (hub topology with no cache; `propose`), `topology-cache-stale` (cache diverges from slot `project.yaml`; `plan validate`). Both are `Error::Validation` / plan-doctor findings.

## Out of scope

- **Deriving the contracts graph.** `contracts` produce/consume wiring stays registry-authored for now; deriving it from per-project declarations is deferred.
- **Catalog import.** Backstage-style external projections ([RFC-21](future/rfc-21-catalogue.md), RM-12) remain a separate follow-on; this RFC only establishes the local authoritative split and the committed cache they would project into.

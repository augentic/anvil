# RFC-36: Project Facets

> Status: Draft — Depends: [RFC-29b](rfc-29b-reconciliation.md) (plan-time `projects[]` topology) — Related: [roadmap principle "Treat `registry.yaml` as a projection"](roadmap.md#principles)

## Problem

A project's adapter and description currently live in **two** authored homes: `.specify/project.yaml` and the hub's `registry.yaml`. At plan time `hub_topology` reads the registry, so a stale hub entry silently overrides the project's own config. Worse, `capabilities` and `keywords` — added to `project.yaml` for slice-to-project routing — never reach the reconciliation envelope.

## Solution

Give every fact one writer; derive everything else.


| Layer                        | Owns                                                                                                           | Does not own                                                |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `**project.yaml`**           | What the project is: `adapter`, `description`, `capabilities`, `keywords`                                      | Membership or repo location                                 |
| `**registry.yaml**`          | Membership and location: `name`, `url`; optional `contracts`; optional `adapter` seed for greenfield scaffolds | Target adapter, description, or routing facets for topology |
| `**.specify/topology.lock**` | Committed, machine-written snapshot of each slot's projected facets                                            | Anything — operators never hand-edit it                     |


`specrun workspace sync` regenerates the lock from each materialised slot's `project.yaml`. `specrun plan validate` checks staleness. Single-repo projects are unchanged: they read `project.yaml` live.

## Decisions


| ID                                        | Decision                                                                                                                                                                                                            |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **D36-1 Authority inversion**             | Project-describing facets are authored only in each project's `.specify/project.yaml`. `registry.yaml` is membership + location only (plus optional `contracts` and a greenfield `adapter` seed).                   |
| **D36-2 Derived topology cache**          | `specrun workspace sync` writes `.specify/topology.lock` (write-if-changed) by resolving each slot's `adapter` to `name@vN` and recording `{ name, target, description?, capabilities[], keywords[] }` per project. |
| **D36-3 Hub reads the cache**             | `hub_topology` builds reconciliation `projects[]` from `.specify/topology.lock`, not `registry.yaml`. Missing cache → `topology-cache-missing` (run `workspace sync`).                                              |
| **D36-4 Capabilities reach the envelope** | `capabilities[]` and `keywords[]` flow from `project.yaml` through the cache into the reconciliation request so the agent binds slices on tags, not description prose alone.                                        |
| **D36-5 Staleness, not synchronisation**  | `plan validate` emits `topology-cache-stale` when the lock diverges from a slot's current `project.yaml`. Fix: `workspace sync`. No silent override and no top-down clobber of authored files.                      |


## Why projection, not synchronisation

Top-down sync (registry overwrites `project.yaml`) keeps two writers and adds conflict rules. The durable fix is one authored home per fact and derived copies everywhere else.

The lock follows the same discipline as `.specify/context.lock`: manifest hand-authored, snapshot machine-derived, committed for offline/pre-survey use, verified in CI. Sync becomes idempotent regenerate-and-verify.

## How it works

### Cache shape

Validated against `topology-lock.schema.json`:

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

Each entry projects one member's `project.yaml`. `name` is the **registry slot name** (the binding key in `plan.yaml.slices[].project` and build-time fan-out per [RFC-29c](rfc-29c-synthesis.md)); only `target`, `description`, `capabilities`, and `keywords` come from `project.yaml`. Empty `capabilities` / `keywords` stay off the wire.

### Greenfield seed

When `workspace sync` clones a repo with no `project.yaml` yet, the registry entry's optional `adapter` seeds the scaffold. Once `project.yaml` exists it is authoritative; the seed is never read again for topology.

### Staleness

`specrun plan validate` (and `propose --dry-run` / `--from`) compare each lock entry against its slot's current `project.yaml`:

- Divergent `target` / `description` / `capabilities` / `keywords` → `topology-cache-stale` (warning); fix with `workspace sync`.
- No lock in a hub → `topology-cache-missing`.

CI reuses the exit-2 gate of `plan validate`. There is no hand-edit path and no `--check` flag — same generate-if-changed discipline as `context.lock`.

## Operator surface

```bash
specrun workspace sync [<project>...]   # regenerates .specify/topology.lock
specrun plan validate                   # topology-cache-stale / topology-cache-missing
specrun registry add <name> --url <url> [--adapter <seed>] [--description <text>]
```

`registry add --adapter` is optional: a greenfield scaffold seed only, written into a new project's `project.yaml` on first clone.

## Wire contracts

Appends to [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts):

- **Schema:** `topology-lock.schema.json` (`TOPOLOGY_LOCK_JSON_SCHEMA`) for `.specify/topology.lock`.
- `**proposal.schema.json` `$defs/projectRef`:** gains optional `capabilities[]` and `keywords[]`; hub `projects[]` source restated as the topology cache, not `registry.yaml#/projects[]`.
- **Validation codes:** `topology-cache-missing` (hub with no cache; `propose`), `topology-cache-stale` (lock diverges from slot `project.yaml`; `plan validate`). Both are `Error::Validation` / plan-doctor findings.

## Out of scope

- **Deriving the contracts graph.** `contracts` produce/consume wiring stays registry-authored; per-project derivation is deferred.
- **Catalog import.** Backstage-style external projections ([RFC-21](future/rfc-21-catalogue.md), RM-12) are a follow-on; this RFC only establishes the local authoritative split and the cache they would project into.


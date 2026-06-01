# RFC-36: Project Identity

> Status: Draft — Depends: [RFC-29b](rfc-29b-reconciliation.md) (plan-time `projects[]` topology) — Related: [RFC-29c](rfc-29c-synthesis.md) (baseline `model.yaml` sub-trees, the future enrichment path) and the [roadmap principle "Treat `registry.yaml` as a projection"](roadmap.md#principles)

## Problem

A project's adapter and description currently live in **two** authored homes: `.specify/project.yaml` and the hub's `registry.yaml`. At plan time `hub_topology` reads the registry, so a stale hub entry silently overrides the project's own config.

Worse, the reconciliation agent has to bind each slice to a project on `description` prose alone.

The project already produces a deterministic, machine-written record of what it owns:

- **Baseline specs** at `.specify/specs/<unit>/spec.md` — the merged system of record, parseable into unit slugs + requirement titles.
- **The journal outcome ledger** at `.specify/journal.jsonl` — one `slice.archive.created` per merge carrying the slice name, touched specs, and a one-line outcome summary.
- `**project.yaml.description`** — operator-authored intent, present from day one.

Routing identity should be *derived from those*, not re-authored as tags.

## Solution

Give every fact one writer; derive everything else. Routing identity is a deterministic projection of each project's baseline — no new authored fact.


| Layer                                   | Owns                                                                                                                                     | Does not own                                          |
| --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `**project.yaml*`*                      | What the project *intends* to be: `adapter`, `description`                                                                               | Membership, repo location, or derived identity        |
| `**.specify/specs/` + `journal.jsonl*`* | What the project *actually* owns: merged requirements and per-merge outcome summaries (authored via the slice loop / written by the CLI) | —                                                     |
| `**registry.yaml*`*                     | Membership and location: `name`, `url`; optional `contracts`; optional `adapter` seed for greenfield scaffolds                           | Description, identity, or routing signal for topology |
| `**.specify/topology.lock`**            | Committed, machine-written snapshot of each slot's projected identity                                                                    | Anything — operators never hand-edit it               |


`specrun workspace sync` regenerates the lock from each materialised slot's `project.yaml` **plus its baseline**. `specrun plan validate` checks staleness. Single-repo projects are unchanged: they read `project.yaml` and their own baseline live.

`capabilities` and `keywords` are removed. Greenfield projects with no baseline route on `description` alone; identity sharpens automatically as slices merge.

## Decisions


| ID                                              | Decision                                                                                                                                                                                                                                                                                                                                              |
| ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **D36-1 Authority inversion**                   | Project-describing facts are authored only in each project's `.specify/project.yaml` (`adapter`, `description`). `registry.yaml` is membership + location only (plus optional `contracts` and a greenfield `adapter` seed).                                                                                                                           |
| **D36-2 Derived identity cache**                | `specrun workspace sync` writes `.specify/topology.lock` (write-if-changed) by resolving each slot's `adapter` to `name@vN` and recording `{ name, target, description?, surface[], recent[] }` per project, where `surface` / `recent` are a **deterministic structural projection** of that slot's baseline (`.specify/specs/`) and journal ledger. |
| **D36-3 Hub reads the cache**                   | `hub_topology` builds reconciliation `projects[]` from `.specify/topology.lock`, not `registry.yaml`. Missing cache → `topology-cache-missing` (run `workspace sync`).                                                                                                                                                                                |
| **D36-4 Derived identity reaches the envelope** | `surface[]` and `recent[]` flow from each project's baseline through the cache into the reconciliation request so the agent binds slices on *actual owned behaviour*, not description prose alone and not a hand-authored tag. `capabilities` / `keywords` are dropped from `project.yaml` and the wire.                                              |
| **D36-5 Staleness, not synchronisation**        | `plan validate` emits `topology-cache-stale` when the lock diverges from a slot's current `project.yaml` *or baseline projection*. Fix: `workspace sync`. No silent override and no top-down clobber of authored files.                                                                                                                               |
| **D36-6 Deterministic projection only**         | The identity projection is structural (unit slugs, requirement titles, ledger summary lines) and byte-stable. It is never an LLM summary — the lock is committed and verified by regenerate-and-compare, so the derivation must be deterministic.                                                                                                     |


## Why derive, not author

The roadmap rule is "one authored home per fact, derive the rest." Hand-authored facets violate it twice: they add a writer, and they duplicate what the baseline already states. They also rot independently of the project they describe, so a staleness check against them guards nothing meaningful.

Deriving identity from the baseline introduces **no new authored fact**. The baseline is authored through the slice loop; the journal is machine-written; `description` already exists. The projection is, by construction, what the project *currently* owns, so `topology-cache-stale` becomes a meaningful lock-vs-baseline check. Routing quality auto-sharpens: a greenfield project routes on `description`, and its `surface[]` fills in as slices merge — with zero operator tag maintenance.

The lock follows the same discipline as `.specify/context.lock`: snapshot machine-derived, committed for offline/pre-survey use, verified in CI. Sync becomes idempotent regenerate-and-verify.

## How it works

### Identity sources


| Source                                             | Contributes                                   | Priority                                  |
| -------------------------------------------------- | --------------------------------------------- | ----------------------------------------- |
| `.specify/specs/<unit>/spec.md`                    | unit slugs + requirement titles → `surface[]` | Primary (structured spine)                |
| `.specify/journal.jsonl` (`slice.archive.created`) | per-merge one-line summaries → `recent[]`     | Secondary (accumulation signal)           |
| `project.yaml.description`                         | authored intent                               | Cold-start / fallback prose               |
| `AGENTS.md` generated block                        | project guidance prose                        | Fallback only (derivative; not the spine) |
| baseline `model.yaml` `domain` / `apis` sub-trees  | structured domain + API surface               | **Future** (see Out of scope)             |


`design.md` is intentionally **not** an identity source: it is a per-slice artifact with no merged baseline counterpart. Its structured content is what [RFC-29c](rfc-29c-synthesis.md) deferred out of the baseline `model.yaml` (`domain` / `apis` / …); when those sub-trees are earned, identity gains a richer structured source for free, with no bespoke scraper.

### Projection contract

The projection reuses the existing parsers; it introduces no new scraper:

- `**surface[].unit`** is the `<unit>` directory slug under `.specify/specs/`, sorted by slug.
- `**surface[].requirements[]**` are the parsed requirement-block headings of that unit's `spec.md` — the `Requirement.name` field (heading text with any inline `[…]` tag stripped) from the requirement-block parser (`crates/model/src/spec/provenance.rs`), in `Requirement.id` order (`REQ-NNN`, no holes, per [RFC-29c §"ID grammar"](rfc-29c-synthesis.md)). Titles only; bodies, `Sources:`, and `Status:` lines are never projected.
- `**recent[]**` is the `outcome_summary` field of the last `M` `slice.archive.created` journal events (filtered to that event kind, in append order; other event kinds are ignored). The summary text is exactly what the merge engine stamped — `recent[]` never re-summarises, so its richness is bounded by today's `outcome_summary` (terse `"<unit>: N modified"` style) until a future RFC enriches that field. No deduplication: a slice that touches one unit twice across two merges contributes two lines.
- `**description**` is copied verbatim from the slot's `project.yaml`. The `AGENTS.md` generated block is **not** projected into the lock — it is derivative of the same baseline, so projecting it would double-count and add a non-deterministic prose source. It survives in the identity-sources table only as context the reconciliation agent may read directly, never as a lock field.

### Cache shape

Validated against `topology-lock.schema.json`:

```yaml
version: 1
projects:
  - name: identity-contracts
    target: contracts@v1
    description: "Versioned API contracts crate for the identity domain."
    surface:
      - unit: identity-api
        requirements: ["Authenticate user", "Fetch account profile"]
  - name: identity-service
    target: omnia@v1
    description: "Omnia identity service implementing auth and password flows."
    surface:
      - unit: password-reset
        requirements: ["Request password reset", "Reset link expiry"]
      - unit: session
        requirements: ["Issue session token", "Revoke session"]
    recent:
      - "password-reset: added reset-link expiry + email queue"
```

Each entry projects one member's identity. `name` is the **registry slot name** (the binding key in `plan.yaml.slices[].project` and build-time fan-out per [RFC-29c](rfc-29c-synthesis.md)); `target` resolves from the slot's `adapter`; `description` is authored intent; `surface[]` / `recent[]` are the deterministic baseline projection. Empty `surface` / `recent` stay off the wire, so greenfield reconciliation degrades cleanly to `description` only.

### Surface bounds

Identity is bounded on the axis where detail is cheap to lose, not the axis that carries the binding signal. For project binding the load-bearing signal is *which units a project owns* (the discriminator between `identity-service` and `billing-service`); individual requirement titles within a unit are diminishing returns once the unit is identified. The rule:

- **Every unit slug, always.** Units are a bounded, slow-growing set and the primary discriminator — this axis is never capped.
- **Up to `K` requirement titles per unit** (default `K = 8`), in stable declaration order. A unit with more emits a `more:` count of the elided titles, so the agent sees "this project is deep here" without the tail:

```yaml
surface:
  - unit: billing
    requirements: ["Create invoice", "Apply credit", "Void invoice", "..."]   # first K
    more: 14                                                                    # 14 further titles elided
```

- `**recent[]`: the last `M` `slice.archive.created` summaries** (default `M = 10`; the `outcome_summary` tail of `journal.jsonl`, per the projection contract above). Older merges are already reflected in `surface[]`, so a small tail suffices.

Size is bounded by `#units × K` and degrades gracefully — a huge project still shows all its domains with a sample of each, and `more:` bumps by an integer (not a title list) when a capped unit grows, keeping diffs small.

Three mechanics keep this sound:

- **Caps live in the projection, not the wire.** They are applied in `workspace sync`, so the committed `topology.lock` *is* the bounded artifact and the envelope forwards it unchanged.
- **Stable ordering only, never relevance-ranked.** Within a unit, declaration order (which equals `REQ` id order with no holes, [RFC-29c §"ID grammar"](rfc-29c-synthesis.md)); units sorted by slug. Ranking by relevance to a lead would make the lock non-deterministic and break D36-6.
- `**K` / `M` are fixed defaults**, not operator config — `description` already absorbs the cold-start and cryptic-title cases. A project that overflows even at unit granularity is a sign the slot is too coarse for one registry entry; the knob is deferred until a real project demands it.

### Greenfield seed

When `workspace sync` clones a repo with no `project.yaml` yet, the registry entry's optional `adapter` seeds the scaffold. Once `project.yaml` exists it is authoritative; the seed is never read again for topology. A freshly scaffolded project has no baseline, so its lock entry carries `description` with empty `surface` / `recent` until its first slice merges.

### Removing the authored facets

`capabilities` and `keywords` are deleted in one change from every home that carries them today: `ProjectConfig` (`crates/workflow/src/config.rs`), `TopologyProject` + `topology-lock.schema.json`, the `resolve_topology` hub and live paths (`crates/workflow/src/change/plan/core/propose.rs`), and `proposal.schema.json#/$defs/projectRef`, plus their tests and fixtures.

`ProjectConfig` does **not** set `deny_unknown_fields`, so dropping the two struct fields makes a stale `capabilities:` / `keywords:` key in an existing `project.yaml` a silently-ignored unknown field — it loads cleanly and the keys simply stop contributing. No migration script and no operator edit is required.

`TopologyProject`, by contrast, *does* set `deny_unknown_fields`, so a pre-upgrade `topology.lock` still carrying `capabilities` / `keywords` would fail `TopologyLock::load` until regenerated. The lock is machine-written, so the fix is the ordinary one — `workspace sync` rewrites it `surface`-only — but the upgrade note should call this out so a hub operator runs `workspace sync` before the first post-upgrade `plan` reads the cache.

### Staleness

`specrun plan validate` (and `propose --dry-run` / `--from`) compare each lock entry against its slot's current `project.yaml` and baseline projection:

- Divergent `target` / `description` / `surface` / `recent` → `topology-cache-stale` (warning); fix with `workspace sync`.
- No lock in a hub → `topology-cache-missing`.

Because the projection is deterministic (D36-6), staleness is a regenerate-and-compare check — the same generate-if-changed discipline as `context.lock`. CI reuses the exit-2 gate of `plan validate`. There is no hand-edit path and no `--check` flag.

### Why a derived lock, not `registry.yaml`

The lock and `registry.yaml` are separated by **writer**: `registry.yaml` is operator-authored, stable, small (`name` + `url`); the lock is machine-written, churning, and now carries the derived identity surface. Folding the derived identity into `registry.yaml` would make the machine rewrite an operator-authored file on every `workspace sync` — the top-down synchronisation anti-pattern (clobbered comments, noisy diffs, a re-introduced dual-writer file) this RFC exists to remove.

Deriving live at propose time (and dropping the lock) was also rejected: it couples plan-time topology to a synced and, for remotes, reachable workspace whose baselines are all readable. The committed snapshot keeps propose offline and fast. Under baseline-derivation that coupling is stronger, so the lock is more load-bearing, not less. The lock is hub-only — a single-repo project has neither `registry.yaml` nor a lock and reads `project.yaml` + baseline live.

> The lock now projects *identity* rather than topology facets; renaming it `identity.lock` reads truer than `topology.lock`. Cosmetic — kept as `topology.lock` here to avoid churning cross-references; rename in a follow-up if desired.

## Operator surface

```bash
specrun workspace sync [<project>...]   # regenerates .specify/topology.lock from project.yaml + baseline
specrun plan validate                   # topology-cache-stale / topology-cache-missing
specrun registry add <name> --url <url> [--adapter <seed>]
```

`registry add --adapter` is optional: a greenfield scaffold seed only, written into a new project's `project.yaml` on first clone. There is no `--description` flag — description is authored in the project's own `project.yaml`.

## Wire contracts

Appends to [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts):

- **Schema:** `topology-lock.schema.json` (`TOPOLOGY_LOCK_JSON_SCHEMA`) for `.specify/topology.lock`, carrying `surface[]` (`unit`, bounded `requirements[]`, optional `more:` count) and `recent[]` per project.
- `**proposal.schema.json` `$defs/projectRef`:** gains optional `surface[]` and `recent[]`; drops `capabilities[]` / `keywords[]`. Hub `projects[]` source restated as the topology cache, not `registry.yaml#/projects[]`.
- **Validation codes:** `topology-cache-missing` (hub with no cache; `propose`), `topology-cache-stale` (lock diverges from a slot's `project.yaml` or baseline projection; `plan validate`). Both are `Error::Validation` / plan-doctor findings.

## Out of scope

- **Design / model-subtree enrichment.** Richer structured identity from baseline `model.yaml` `domain` / `apis` sub-trees lands automatically once [RFC-29c](rfc-29c-synthesis.md) earns those sub-trees; this RFC ships the spec/journal/description projection only.
- **Deriving the contracts graph.** `contracts` produce/consume wiring stays registry-authored; per-project derivation is deferred.
- **Catalog import.** Backstage-style external projections ([RFC-21](future/rfc-21-catalogue.md), RM-12) are a follow-on; this RFC establishes the local authoritative split and the cache they would project into.


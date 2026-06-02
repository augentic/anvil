# RFC-36: Project Identity

> Status: **Shipped** — archived milestone spec; durable source of truth is [`specify-cli` `DECISIONS.md` §"Registry projection and topology cache (RFC-36)"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#registry-projection-and-topology-cache-rfc-36) and [`docs/standards/workflow.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md). Depends: [From sources to slices §Plan time](../docs/explanation/reconciliation.md#plan-time-leads-become-slices) (plan-time `projects[]` topology) and [`DECISIONS.md` §Slice synthesis engine](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#slice-synthesis-engine-rfc-29-m2b) (synthesis authors `design.md`; the baseline delta-merge model) — Related: [decision-log §"Opaque replacement for contract merge"](../docs/explanation/decision-log.md), [decision-log §"History via git plus an outcome ledger"](../docs/explanation/decision-log.md), and the [roadmap principle "one authored home per fact, derive the rest"](roadmap.md#principles)

## Problem

A project's adapter and description currently live in **two** authored homes: `.specify/project.yaml` and the workspace's `registry.yaml`. At plan time `workspace_topology` reads the registry, so a stale registry entry silently overrides the project's own config.

Worse, the reconciliation agent has to bind each slice to a project on `description` prose alone.

The project already produces a deterministic, machine-written record of what it owns:

- **Baseline specs** at `.specify/specs/<unit>/spec.md` — the merged system of record, parseable into unit slugs + requirement titles.
- **The journal outcome ledger** at `.specify/journal.jsonl` — one `slice.archive.created` per merge carrying the slice name, touched specs, and a one-line outcome summary.
- **`project.yaml.description`** — operator-authored intent, present from day one.

Routing identity should be *derived from those*, not re-authored as tags.

That derivation is incomplete without the design *why*. `/spec:merge` folds each slice's `spec.md` deltas into an authoritative baseline, so the behavioural "what" accumulates into a reviewable system of record. The design "why" has no such home. `design.md` is authored per slice (RFC-29c) and then archived with the slice; the rationale behind a technical choice — and the alternatives that were rejected — survives only in a prunable archive folder no downstream verb reads.

Operators reach for "do for `design.md` what merge does for `spec.md`", but a verbatim copy of that mechanism does not fit:

- **No merge key.** `spec.md` merges because every requirement carries a stable `ID: REQ-XXX` (decision-log §"Stable requirement IDs as merge keys"). `design.md` prose has no equivalent stable granularity, so a prose delta-merge has nothing to key on — the same reason contracts and composition each use a non-`REQ` strategy.
- **State vs decisions.** `design.md` bundles two things with different lifetimes: design *state* (the structural "how" *now* — domain models, API shapes) which is volatile and best derived from ground truth, and design *decisions* (the immutable "why" of a choice) which are append-only. A single master `design.md` would re-author state the baseline/code already states and rot against it — cutting against the archive posture (decision-log §"History via git plus an outcome ledger") and this RFC's "derive, don't author" stance.

The append-only half — design **decisions** — is the part that genuinely deserves a baseline counterpart. It completes the identity projection: `surface[]` answers *what behaviour a project owns*; without a durable *why*, two projects with overlapping behaviour (identical `surface[]`) are indistinguishable when the agent binds a slice about architectural commitment — token rotation, replay protection, storage choice. Decision Records close that gap and feed the same derived-identity machinery as specs and the journal.

## Solution

Give every fact one writer; derive everything else. Routing identity is a deterministic projection of each project's baseline — no new authored fact.


| Layer | Owns | Does not own |
| --- | --- | --- |
| **`project.yaml`** | What the project *intends* to be: `adapter`, `description` | Membership, repo location, or derived identity |
| **`.specify/specs/` + `.specify/decisions/` + `journal.jsonl`** | What the project *actually* owns: merged requirements, append-only design decisions, and per-merge outcome summaries (authored via the slice loop / written by the CLI) | — |
| **`registry.yaml`** | Membership and location: `name`, `url`; optional `contracts`; optional `adapter` seed for greenfield scaffolds | Description, identity, or routing signal for topology |
| **`.specify/topology.lock`** | Committed, machine-written snapshot of each slot's projected identity | Anything — operators never hand-edit it |


`specrun workspace sync` regenerates the lock from each materialised slot's `project.yaml` **plus its baseline**. `specrun plan validate` checks staleness. Single-repo projects are unchanged: they read `project.yaml` and their own baseline live.

**Decision Records** promote slice-authored design decisions into an append-only baseline catalogue at merge time, using the same opaque-add strategy contracts already use (decision-log §"Opaque replacement for contract merge") rather than a second prose delta-merge engine:

- A slice may author zero or more Decision Records under `.specify/slices/<slice>/decisions/<slug>.md`. Each is a YAML front-matter header (schema-validated) plus a Nygard-shaped Markdown body (`Context` / `Decision` / `Consequences`).
- `specrun slice merge` promotes each record into `.specify/decisions/DEC-NNNN-<slug>.md`, assigning the durable, project-global `DEC-NNNN` id; the slice slug is the only key the agent authors.
- The catalogue is **append-only**. The single permitted mutation to an existing record is flipping its status to `superseded` when a newer record names it under `supersedes:`.
- This stores the *why* only. `design.md` stays a per-slice artifact, there is no master `design.md`, and design *state* remains the job of the code plus the future structured `model.yaml` sub-trees (Out of scope).

`AGENTS.md` is **not** the home: it is a derivative, fallback prose source, not a spine. It may link to the catalogue; it never is the catalogue.

`capabilities` and `keywords` are removed. Greenfield projects with no baseline route on `description` alone; identity sharpens automatically as slices merge and decisions accumulate.

## Decisions


| ID | Decision |
| --- | --- |
| **D36-1 Authority inversion** | Project-describing facts are authored only in each project's `.specify/project.yaml` (`adapter`, `description`). `registry.yaml` is membership + location only (plus optional `contracts` and a greenfield `adapter` seed). |
| **D36-2 Derived identity cache** | `specrun workspace sync` writes `.specify/topology.lock` (write-if-changed) by resolving each slot's `adapter` to `name@vN` and recording `{ name, target, description?, surface[], decisions[], recent[] }` per project, where `surface` / `decisions` / `recent` are a **deterministic structural projection** of that slot's baseline (`.specify/specs/`, `.specify/decisions/`, and the journal ledger). |
| **D36-3 Hub reads the cache** | `workspace_topology` builds reconciliation `projects[]` from `.specify/topology.lock`, not `registry.yaml`. Missing cache → `topology-cache-missing` (run `workspace sync`). |
| **D36-4 Derived identity reaches the envelope** | `surface[]`, `decisions[]`, and `recent[]` flow from each project's baseline through the cache into the reconciliation request so the agent binds slices on *actual owned behaviour and architectural commitment*, not description prose alone and not a hand-authored tag. `capabilities` / `keywords` are dropped from `project.yaml` and the wire. |
| **D36-5 Staleness, not synchronisation** | `plan validate` emits `topology-cache-stale` when the lock diverges from a slot's current `project.yaml` *or baseline projection*. Fix: `workspace sync`. No silent override and no top-down clobber of authored files. |
| **D36-6 Deterministic projection only** | The identity projection is structural (unit slugs, requirement titles, decision ids + titles, ledger summary lines) and byte-stable. It is never an LLM summary — the lock is committed and verified by regenerate-and-compare, so the derivation must be deterministic. |
| **D36-7 Decisions, not state** | The baseline catalogue stores design *decisions* (the immutable "why" + rejected alternatives), never design *state*. `design.md` stays per-slice; no master `design.md` is synthesised. |
| **D36-8 Append-only catalogue, opaque-add merge** | Records land at `.specify/decisions/DEC-NNNN-<slug>.md` by whole-file add — never prose delta-merge — mirroring the contracts opaque-replacement decision. The catalogue is one flat, project-global tree (decisions are cross-cutting, not per-`unit`). |
| **D36-9 Front-matter + Nygard body** | A record is schema-validated YAML front-matter (`slug`, `status`, optional `supersedes` / `related`) plus a Markdown body with `## Context` / `## Decision` / `## Consequences`. The slice authors `status: accepted` or `rejected`; the engine never authors those two. |
| **D36-10 CLI assigns durable ids** | `DEC-NNNN` is assigned by `specrun slice merge` as `max(existing) + 1` (zero-padded, monotonic, never reused). The slice carries only `slug`. The single-active-slice invariant + plan lock make sequential numbering race-free. |
| **D36-11 Supersede is the only mutation** | A new record's `supersedes: [DEC-NNNN \| <slug>]` flips each target from `accepted` to `superseded` and stamps `superseded-by: DEC-NNNN`. A target absent from the baseline ∪ this slice raises a blocking `decision-supersede-orphan`. |
| **D36-12 Agent authors prose; kernel untouched** | Records are agent-authored at `/spec:refine` (prose authoring, already the agent's job per RFC-29c) — **not** projected from `model.yaml`. The synthesis kernel, `model.schema.json`, and the `REQ`/`TASK` grammars are unchanged. `validate` checks record shape; `merge` owns id assignment and promotion. |
| **D36-13 Durable record = git + ledger** | The system of record is git history of `.specify/decisions/` plus the merge ledger entry's new `decisions[]` field. The slice's `decisions/` copy is a prunable cache, consistent with the archive posture for slices. |


## Why derive, not author

The roadmap rule is "one authored home per fact, derive the rest." Hand-authored facets violate it twice: they add a writer, and they duplicate what the baseline already states. They also rot independently of the project they describe, so a staleness check against them guards nothing meaningful.

Deriving identity from the baseline introduces **no new authored fact**. The baseline is authored through the slice loop; the journal is machine-written; `description` already exists. The projection is, by construction, what the project *currently* owns and *why it is shaped that way*, so `topology-cache-stale` becomes a meaningful lock-vs-baseline check. Routing quality auto-sharpens: a greenfield project routes on `description`, and its `surface[]` / `decisions[]` fill in as slices merge — with zero operator tag maintenance.

The lock follows the same discipline as `.specify/context.lock`: snapshot machine-derived, committed for offline/pre-survey use, verified in CI. Sync becomes idempotent regenerate-and-verify.

## How it works

### Identity sources


| Source | Contributes | Priority |
| --- | --- | --- |
| `.specify/specs/<unit>/spec.md` | unit slugs + requirement titles → `surface[]` | Primary (what the project does) |
| `.specify/decisions/DEC-NNNN-*.md` | accepted-decision titles → `decisions[]` | Primary (why it is shaped that way) |
| `.specify/journal.jsonl` (`slice.archive.created`) | per-merge one-line summaries → `recent[]` | Secondary (what changed) |
| `project.yaml.description` | authored intent | Cold-start / fallback prose |
| `AGENTS.md` generated block | project guidance prose | Fallback only (derivative; not the spine) |
| baseline `model.yaml` `domain` / `apis` sub-trees | structured domain + API surface | **Future** (see Out of scope) |


`design.md` is intentionally **not** an identity source: it is a per-slice artifact bundling volatile *state* with immutable *decisions*. Its structured content is what [`DECISIONS.md` §Slice synthesis engine](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#slice-synthesis-engine-rfc-29-m2b) deferred out of the baseline `model.yaml` (`domain` / `apis` / …); when those sub-trees are earned, identity gains a richer structured source for free. The *decisions* half is promoted into `.specify/decisions/` at merge and projected as `decisions[]` — the durable, baseline-resident counterpart this identity model requires for the design layer.

### Why decisions improve routing

`surface[]` answers *what behaviour a project owns*; that is the primary discriminator, but it under-separates projects that own overlapping behaviour. Two services can both "Issue session token" (identical `surface[]`) yet differ in architectural commitment — one decided on DPoP sender-constrained tokens, the other on opaque bearer tokens. When the reconciliation agent binds a slice about *token rotation* or *replay protection*, the decision axis is the signal that routes it to the right project. Decisions also carry the constraints a new slice must respect, so surfacing them at plan time lets the agent flag a lead that contradicts an accepted decision before Gate 1, not at build.

### Projection contract

The projection reuses the existing parsers; it introduces no new scraper:

- **`surface[].unit`** is the `<unit>` directory slug under `.specify/specs/`, sorted by slug.
- **`surface[].requirements[]`** are the parsed requirement-block headings of that unit's `spec.md` — the `Requirement.name` field (heading text with any inline `[…]` tag stripped) from the requirement-block parser (`crates/model/src/spec/provenance.rs`), in `Requirement.id` order (`REQ-NNN`, no holes, per [`DECISIONS.md` §Slice synthesis engine](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#slice-synthesis-engine-rfc-29-m2b)). Titles only; bodies, `Sources:`, and `Status:` lines are never projected.
- **`decisions[]`** obeys **D36-6 (deterministic projection only)** — structural and byte-stable, never an LLM summary:
  - **Source filter:** only baseline records with `status: accepted`. `superseded` and `rejected` records describe past or not-taken posture, so they are excluded from *current* identity (the same way `surface[]` reflects current requirements, not removed ones).
  - **`decisions[].id`** is the `DEC-NNNN`; **`decisions[].title`** is the record's H1 heading text. No body, `Context`, or `Consequences` prose is projected.
  - **Order** is `DEC-NNNN` ascending — stable, never relevance-ranked (ranking would break D36-6).
  - **Bounded** the same way `surface[]` is: up to `K` decisions (default `K = 8`, the most recent `DEC` ids), with a `more:` integer counting the elided remainder. Decisions are a slow-growing, project-wide set, so this stays small.
- **`recent[]`** is the `outcome_summary` field of the last `M` `slice.archive.created` journal events (filtered to that event kind, in append order; other event kinds are ignored). The summary text is exactly what the merge engine stamped — `recent[]` never re-summarises, so its richness is bounded by today's `outcome_summary` (terse `"<unit>: N modified"` style) until a future RFC enriches that field. No deduplication: a slice that touches one unit twice across two merges contributes two lines.
- **`description`** is copied verbatim from the slot's `project.yaml`. The `AGENTS.md` generated block is **not** projected into the lock — it is derivative of the same baseline, so projecting it would double-count and add a non-deterministic prose source. It survives in the identity-sources table only as context the reconciliation agent may read directly, never as a lock field.

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
    decisions:
      - { id: DEC-0007, title: "Use PostgreSQL for the identity store" }
      - { id: DEC-0011, title: "DPoP sender-constrained access tokens" }
    recent:
      - "password-reset: added reset-link expiry + email queue"
```

Each entry projects one member's identity. `name` is the **registry slot name** (the binding key in `plan.yaml.slices[].project` and build-time fan-out per [`DECISIONS.md` §Slice synthesis engine](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#slice-synthesis-engine-rfc-29-m2b)); `target` resolves from the slot's `adapter`; `description` is authored intent; `surface[]` / `decisions[]` / `recent[]` are the deterministic baseline projection. Empty `surface` / `decisions` / `recent` stay off the wire, so greenfield reconciliation degrades cleanly to `description` only.

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

- **`recent[]`: the last `M` `slice.archive.created` summaries** (default `M = 10`; the `outcome_summary` tail of `journal.jsonl`, per the projection contract above). Older merges are already reflected in `surface[]`, so a small tail suffices.

Size is bounded by `#units × K` and degrades gracefully — a huge project still shows all its domains with a sample of each, and `more:` bumps by an integer (not a title list) when a capped unit grows, keeping diffs small.

Three mechanics keep this sound:

- **Caps live in the projection, not the wire.** They are applied in `workspace sync`, so the committed `topology.lock` *is* the bounded artifact and the envelope forwards it unchanged.
- **Stable ordering only, never relevance-ranked.** Within a unit, declaration order (which equals `REQ` id order with no holes, [`DECISIONS.md` §Slice synthesis engine](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#slice-synthesis-engine-rfc-29-m2b)); units sorted by slug; decisions sorted by `DEC-NNNN`. Ranking by relevance to a lead would make the lock non-deterministic and break D36-6.
- **`K` / `M` are fixed defaults**, not operator config — `description` already absorbs the cold-start and cryptic-title cases. A project that overflows even at unit granularity is a sign the slot is too coarse for one registry entry; the knob is deferred until a real project demands it.

### Decision Records

Authored in the slice (no `id` — the engine assigns it):

```markdown
---
slug: identity-store-postgres
status: accepted            # accepted | rejected (slice-authored); superseded is engine-only
supersedes: [DEC-0003]      # optional: baseline DEC-NNNN or a slug merged earlier in this slice
related: [REQ-001, REQ-014] # optional: traceability into this slice's requirements
---
# Use PostgreSQL for the identity store

## Context
Why the decision is needed; constraints and forces.

## Decision
What was chosen.

## Consequences
Trade-offs, follow-ups, and what the rejected alternatives cost us.
```

After promotion the baseline file `.specify/decisions/DEC-0007-identity-store-postgres.md` carries the engine-stamped header fields:

```yaml
---
id: DEC-0007
slug: identity-store-postgres
status: accepted
slice: identity-service      # slice that introduced it
date: 2026-06-02             # merge date (injected clock)
supersedes: [DEC-0003]
related: [REQ-001, REQ-014]
---
```

A superseded record is edited in place to `status: superseded` with `superseded-by: DEC-0007` appended; its body is left verbatim so the historical rationale survives.

#### Merge algorithm

`specrun slice merge` gains a decisions pass alongside the existing spec / composition / contracts promotion (it is **core**, not target-specific, so it runs for every target):

1. Read `.specify/slices/<slice>/decisions/*.md`; if none, the pass is a no-op.
2. Resolve the next id from the baseline: `DEC-` + zero-padded `max(existing NNNN) + 1`, incrementing per record in slug order.
3. For each record: re-check every `supersedes:` target resolves (baseline ∪ ids assigned earlier in this same merge); abort with `decision-supersede-orphan` if not.
4. Write each baseline file with the stamped `id` / `slice` / `date`; flip each superseded target to `status: superseded` + `superseded-by`.
5. Record the assigned ids on the `slice.archive.created` ledger entry (`decisions[]`).

Promotion is part of the same atomic merge as the spec deltas. Because adds never collide (ids are engine-assigned) and the only edit is a status flip on a named target, there is no decision-specific baseline-conflict surface beyond the existing `conflict-check` timestamp guard.

#### Validation findings

`specrun slice validate` (refine gate) adds, over `.specify/slices/<slice>/decisions/*.md`:

| Finding | Meaning |
| --- | --- |
| `decision-record-schema` | Front-matter fails `decision.schema.json`. |
| `decision-record-section-missing` | Body is missing a required `## Context` / `## Decision` / `## Consequences` heading. |
| `decision-slug-grammar` | `slug` does not match `^[a-z][a-z0-9-]*$` (≤ 64 chars). |
| `decision-slug-collision` | Two records in the slice share a `slug`. |
| `decision-supersede-orphan` | A `supersedes:` target resolves to neither the baseline nor a sibling slice record. |

Shape, grammar, and intra-slice checks run at `validate` (blocking the `refining → refined` transition at exit 2). `decision-supersede-orphan` is re-checked against the live baseline at `merge` (step 3 above), since the baseline may move between refine and merge.

### Greenfield seed

When `workspace sync` clones a repo with no `project.yaml` yet, the registry entry's optional `adapter` seeds the scaffold. Once `project.yaml` exists it is authoritative; the seed is never read again for topology. A freshly scaffolded project has no baseline, so its lock entry carries `description` with empty `surface` / `decisions` / `recent` until its first slice merges.

### Removing the authored facets

`capabilities` and `keywords` are deleted in one change from every home that carries them today: `ProjectConfig` (`crates/workflow/src/config.rs`), `TopologyProject` + `topology-lock.schema.json`, the `resolve_topology` workspace and live paths (`crates/workflow/src/change/plan/core/propose.rs`), and `proposal.schema.json#/$defs/projectRef`, plus their tests and fixtures.

`ProjectConfig` does **not** set `deny_unknown_fields`, so dropping the two struct fields makes a stale `capabilities:` / `keywords:` key in an existing `project.yaml` a silently-ignored unknown field — it loads cleanly and the keys simply stop contributing. No migration script and no operator edit is required.

`TopologyProject`, by contrast, *does* set `deny_unknown_fields`, so a pre-upgrade `topology.lock` still carrying `capabilities` / `keywords` would fail `TopologyLock::load` until regenerated. The lock is machine-written, so the fix is the ordinary one — `workspace sync` rewrites it — but the upgrade note should call this out so a workspace operator runs `workspace sync` before the first post-upgrade `plan` reads the cache.

### Staleness

`specrun plan validate` (and `propose --dry-run` / `--from`) compare each lock entry against its slot's current `project.yaml` and baseline projection:

- Divergent `target` / `description` / `surface` / `decisions` / `recent` → `topology-cache-stale` (warning); fix with `workspace sync`.
- No lock in a workspace → `topology-cache-missing`.

Because the projection is deterministic (D36-6), staleness is a regenerate-and-compare check — the same generate-if-changed discipline as `context.lock`. CI reuses the exit-2 gate of `plan validate`. There is no hand-edit path and no `--check` flag.

### Why a derived lock, not `registry.yaml`

The lock and `registry.yaml` are separated by **writer**: `registry.yaml` is operator-authored, stable, small (`name` + `url`); the lock is machine-written, churning, and now carries the derived identity surface. Folding the derived identity into `registry.yaml` would make the machine rewrite an operator-authored file on every `workspace sync` — the top-down synchronisation anti-pattern (clobbered comments, noisy diffs, a re-introduced dual-writer file) this RFC exists to remove.

Deriving live at propose time (and dropping the lock) was also rejected: it couples plan-time topology to a synced and, for remotes, reachable workspace whose baselines are all readable. The committed snapshot keeps propose offline and fast. Under baseline-derivation that coupling is stronger, so the lock is more load-bearing, not less. The lock is workspace-only — a single-repo project has neither `registry.yaml` nor a lock and reads `project.yaml` + baseline live.

> The lock now projects *identity* rather than topology facets; renaming it `identity.lock` reads truer than `topology.lock`. Cosmetic — kept as `topology.lock` here to avoid churning cross-references; rename in a follow-up if desired.

## Operator surface

```bash
specrun workspace sync [<project>...]   # regenerates .specify/topology.lock from project.yaml + baseline
specrun plan validate                   # topology-cache-stale / topology-cache-missing
specrun registry add <name> --url <url> [--adapter <seed>]
specrun slice validate <slice>          # decision-record-* findings
specrun slice merge <slice>             # promotes decisions/*.md → .specify/decisions/DEC-NNNN-*.md
```

`registry add --adapter` is optional: a greenfield scaffold seed only, written into a new project's `project.yaml` on first clone. There is no `--description` flag — description is authored in the project's own `project.yaml`.

No new top-level verb for decisions. A read-only `specrun decisions list` projection is a candidate follow-on but is **not** required for this RFC — `ls .specify/decisions/` and git are sufficient at first.

## Wire contracts

Appends to [`DECISIONS.md` §Lead reconciliation](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lead-reconciliation-d2):

- **Schema:** `topology-lock.schema.json` (`TOPOLOGY_LOCK_JSON_SCHEMA`) for `.specify/topology.lock`, carrying `surface[]` (`unit`, bounded `requirements[]`, optional `more:` count), `decisions[]` (`{ id, title }`, plus optional `more:` count), and `recent[]` per project.
- **`proposal.schema.json` `$defs/projectRef`:** gains optional `surface[]`, `decisions[]`, and `recent[]`; drops `capabilities[]` / `keywords[]`. Hub `projects[]` source restated as the topology cache, not `registry.yaml#/projects[]`.
- **Schema:** `decision.schema.json` (`DECISION_JSON_SCHEMA`) — validates the front-matter block (`slug` required; `status` closed enum `accepted | rejected | superseded`; optional `supersedes[]`, `related[]`; engine-stamped `id` / `slice` / `date` / `superseded-by` optional in the slice-authored form, required on the persisted baseline form).
- **Validation codes:** `topology-cache-missing` (workspace with no cache; `propose`), `topology-cache-stale` (lock diverges from a slot's `project.yaml` or baseline projection; `plan validate`); `decision-record-schema`, `decision-record-section-missing`, `decision-slug-grammar`, `decision-slug-collision`, `decision-supersede-orphan` — all `Error::Validation` / slice-doctor findings.
- **Ledger field:** `slice.archive.created` gains an optional `decisions[]` array of the `DEC-NNNN` ids promoted by the merge (no new event kind). This is the durable record alongside git history of `.specify/decisions/`.

## Implementation checklist

The workflow contract spans both repos (parent-repo AGENTS §"Note to the implementing agent").

**`augentic/specify-cli`:**
- Add `crates/schema/src/` `DECISION_JSON_SCHEMA` + `schemas/decision.schema.json`.
- Add a `decisions` parser (front-matter + section check) under `crates/model/src/`.
- Add the five `decision-*` findings to `crates/validate/`.
- Extend the `specrun slice merge` engine (`crates/workflow/src/slice/`) with the decisions promotion pass, id assignment, and supersede flip; thread the injected clock for `date`.
- Add the optional `decisions[]` field to the `slice.archive.created` journal event (`crates/workflow/src/journal.rs`).
- Add `decisions[]` to `topology-lock.schema.json` and `proposal.schema.json#/$defs/projectRef`; extend the baseline projection in `specrun workspace sync` (`crates/workflow/src/change/plan/core/propose.rs` topology path) to read `.specify/decisions/` and emit the bounded `decisions[]`; extend the `topology-cache-stale` comparison to cover it.
- Delete `capabilities` / `keywords` from `ProjectConfig`, `TopologyProject`, and the propose topology paths.
- Tests: golden merge fixtures (fresh add, supersede, multi-record id ordering); validate fixtures for each decision finding; topology-lock projection fixtures asserting accepted-only, `DEC` ordering, and the `K`/`more:` cap; staleness fixtures covering `decisions[]`.

**`augentic/specify` (this repo):**
- `/spec:refine` skill + the refine artifact-conventions reference: document the optional `decisions/<slug>.md` artifact and its format.
- `/spec:merge` SKILL + `references/merge-runbook.md`: add a "Decisions Promoted" line to the summary template.
- `docs/explanation/artifacts.md` and `docs/explanation/augentic-specify-usage.md`: add Decision Records to the artifact lifecycle and the merge-folds-into-baseline description.
- Add a decision-log entry recording this choice (decisions over state; opaque-add over prose delta-merge; `.specify/decisions/` home; decisions as an identity axis).

## Out of scope

- **Design / model-subtree enrichment.** Richer structured identity from baseline `model.yaml` `domain` / `apis` sub-trees lands automatically once [`DECISIONS.md` §Slice synthesis engine](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#slice-synthesis-engine-rfc-29-m2b) earns those sub-trees; this RFC ships the spec/decisions/journal/description projection only.
- **Structured `model.yaml` decisions sub-tree.** A kernel-projected `decisions[]` sub-tree on `model.yaml` (the richer, merge-keyed enrichment path RFC-29c deferred its non-requirements sub-trees toward) is a future RFC. This RFC deliberately ships the lighter opaque-add Markdown catalogue first and does **not** depend on that sub-tree landing; if it lands, these `DEC-NNNN` records become its projection target.
- **Relevance-ranked identity.** The `decisions[]` projection is in scope, but only as a stable, `DEC`-ordered, capped list. Ranking decisions by relevance to a lead, or weighting the binding score by decision overlap, would break **D36-6** determinism and is deferred to whatever future RFC makes binding itself score-based.
- **Design *state* / living architecture docs.** C4 / arc42-style living architecture documentation and any auto-derived design-state surface are not in scope; design state stays the job of the code and the future model sub-trees.
- **Code-to-decision back-links.** Tooling to enforce that code seams reference a `DEC-NNNN` (the `grep -r 'DEC-' src/` discipline) is a standards-layer concern for a later RFC, not part of the merge contract.
- **Deriving the contracts graph.** `contracts` produce/consume wiring stays registry-authored; per-project derivation is deferred.
- **Catalog import.** Backstage-style external projections ([RFC-21](future/rfc-21-catalogue.md), RM-12) are a follow-on; this RFC establishes the local authoritative split and the cache they would project into.

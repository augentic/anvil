# RFC-37: Decision Records

> Status: Accepted — Depends: [RFC-29c](rfc-29c-synthesis.md) (synthesis authors `design.md`; the baseline delta-merge model) — Sequenced-after: [RFC-36](rfc-36-project-identity.md) (implement after RFC-36 lands; **extends** its identity projection — see [§"Decision Records as an identity source"](#decision-records-as-an-identity-source-rfc-36)) — Related: [decision-log §"Opaque replacement for contract merge"](../docs/explanation/decision-log.md), [decision-log §"History via git plus an outcome ledger"](../docs/explanation/decision-log.md), and the [roadmap principle "one authored home per fact, derive the rest"](roadmap.md#principles)

## Problem

`/spec:merge` folds each slice's `spec.md` deltas into an authoritative baseline at `.specify/specs/`, so the *behavioural* "what" of a project accumulates into a reviewable, diffable system of record. The *design* "why" has no such home. `design.md` is authored per slice (RFC-29c) and then archived with the slice; the rationale behind a technical choice — and the alternatives that were rejected — survives only in a prunable archive folder no downstream verb reads.

Operators reach for "do for `design.md` what merge does for `spec.md`", but a verbatim copy of that mechanism does not fit:

- **No merge key.** `spec.md` merges because every requirement carries a stable `ID: REQ-XXX` (decision-log §"Stable requirement IDs as merge keys"). `design.md` prose has no equivalent stable granularity, so a prose delta-merge has nothing to key on — the same reason contracts and composition each use a non-`REQ` strategy.
- **State vs decisions.** `design.md` bundles two things with different lifetimes: design *state* (the structural "how" *now* — domain models, API shapes) which is volatile and best derived from ground truth, and design *decisions* (the immutable "why" of a choice) which are append-only. A single master `design.md` would re-author state the baseline/code already states and rot against it — cutting against the archive posture (decision-log §"History via git plus an outcome ledger") and RFC-36's "derive, don't author" stance.

The append-only half — design **decisions** — is the part that genuinely deserves a baseline counterpart, and it fits Specify's existing merge machinery almost for free.

## Solution

Promote slice-authored **Decision Records** into an append-only baseline catalogue at merge time, using the same opaque-add strategy contracts already use (decision-log §"Opaque replacement for contract merge") rather than a second prose delta-merge engine.

- A slice may author zero or more Decision Records under `.specify/slices/<slice>/decisions/<slug>.md`. Each is a YAML front-matter header (schema-validated) plus a Nygard-shaped Markdown body (`Context` / `Decision` / `Consequences`).
- `specrun slice merge` promotes each record into `.specify/decisions/DEC-NNNN-<slug>.md`, assigning the durable, project-global `DEC-NNNN` id; the slice slug is the only key the agent authors.
- The catalogue is **append-only**. The single permitted mutation to an existing record is flipping its status to `superseded` when a newer record names it under `supersedes:`.
- This stores the *why* only. `design.md` stays a per-slice artifact, there is no master `design.md`, and design *state* remains the job of the code plus the future structured `model.yaml` sub-trees (Out of scope).

`AGENTS.md` is **not** the home: per RFC-36 it is a derivative, fallback prose source, not a spine. It may link to the catalogue; it never is the catalogue.

## Decisions

| ID | Decision |
| -- | -------- |
| **D37-1 Decisions, not state** | The baseline catalogue stores design *decisions* (the immutable "why" + rejected alternatives), never design *state*. `design.md` stays per-slice; no master `design.md` is synthesised. |
| **D37-2 Append-only catalogue, opaque-add merge** | Records land at `.specify/decisions/DEC-NNNN-<slug>.md` by whole-file add — never prose delta-merge — mirroring the contracts opaque-replacement decision. The catalogue is one flat, project-global tree (decisions are cross-cutting, not per-`unit`). |
| **D37-3 Front-matter + Nygard body** | A record is schema-validated YAML front-matter (`slug`, `status`, optional `supersedes` / `related`) plus a Markdown body with `## Context` / `## Decision` / `## Consequences`. The slice authors `status: accepted` or `rejected`; the engine never authors those two. |
| **D37-4 CLI assigns durable ids** | `DEC-NNNN` is assigned by `specrun slice merge` as `max(existing) + 1` (zero-padded, monotonic, never reused). The slice carries only `slug`. The single-active-slice invariant + plan lock make sequential numbering race-free. |
| **D37-5 Supersede is the only mutation** | A new record's `supersedes: [DEC-NNNN \| <slug>]` flips each target from `accepted` to `superseded` and stamps `superseded-by: DEC-NNNN`. A target absent from the baseline ∪ this slice raises a blocking `decision-supersede-orphan`. |
| **D37-6 Agent authors prose; kernel untouched** | Records are agent-authored at `/spec:refine` (prose authoring, already the agent's job per RFC-29c) — **not** projected from `model.yaml`. The synthesis kernel, `model.schema.json`, and the `REQ`/`TASK` grammars are unchanged. `validate` checks record shape; `merge` owns id assignment and promotion. |
| **D37-7 Durable record = git + ledger** | The system of record is git history of `.specify/decisions/` plus the merge ledger entry's new `decisions[]` field. The slice's `decisions/` copy is a prunable cache, consistent with the archive posture for slices. |
| **D37-8 Decisions sharpen identity** | The accepted-decision catalogue projects into RFC-36 routing identity as a third axis beside `surface[]` (what the project does) and `recent[]` (what changed) — *why the project is shaped the way it is*. It reuses RFC-36's deterministic-projection, topology-lock, envelope-forward, and staleness machinery wholesale; it is **not** a new authored fact. See [§"Decision Records as an identity source"](#decision-records-as-an-identity-source-rfc-36). |

## Record shape

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

## Merge algorithm

`specrun slice merge` gains a decisions pass alongside the existing spec / composition / contracts promotion (it is **core**, not target-specific, so it runs for every target):

1. Read `.specify/slices/<slice>/decisions/*.md`; if none, the pass is a no-op.
2. Resolve the next id from the baseline: `DEC-` + zero-padded `max(existing NNNN) + 1`, incrementing per record in slug order.
3. For each record: re-check every `supersedes:` target resolves (baseline ∪ ids assigned earlier in this same merge); abort with `decision-supersede-orphan` if not.
4. Write each baseline file with the stamped `id` / `slice` / `date`; flip each superseded target to `status: superseded` + `superseded-by`.
5. Record the assigned ids on the `slice.archive.created` ledger entry (`decisions[]`).

Promotion is part of the same atomic merge as the spec deltas. Because adds never collide (ids are engine-assigned) and the only edit is a status flip on a named target, there is no decision-specific baseline-conflict surface beyond the existing `conflict-check` timestamp guard.

## Validation findings

`specrun slice validate` (refine gate) adds, over `.specify/slices/<slice>/decisions/*.md`:

| Finding | Meaning |
| ------- | ------- |
| `decision-record-schema` | Front-matter fails `decision.schema.json`. |
| `decision-record-section-missing` | Body is missing a required `## Context` / `## Decision` / `## Consequences` heading. |
| `decision-slug-grammar` | `slug` does not match `^[a-z][a-z0-9-]*$` (≤ 64 chars). |
| `decision-slug-collision` | Two records in the slice share a `slug`. |
| `decision-supersede-orphan` | A `supersedes:` target resolves to neither the baseline nor a sibling slice record. |

Shape, grammar, and intra-slice checks run at `validate` (blocking the `refining → refined` transition at exit 2). `decision-supersede-orphan` is re-checked against the live baseline at `merge` (step 3 above), since the baseline may move between refine and merge.

## Decision Records as an identity source (RFC-36)

RFC-36 derives a project's routing identity from its baseline rather than hand-authored tags, projecting two axes into `.specify/topology.lock` and on into the reconciliation envelope: `surface[]` (unit slugs + requirement titles, from `.specify/specs/`) and `recent[]` (per-merge summaries, from the journal). It explicitly excluded `design.md`:

> `design.md` is intentionally **not** an identity source: it is a per-slice artifact with no merged baseline counterpart.

RFC-37 **creates that missing merged baseline counterpart** for the *why*. The accepted-decision catalogue is exactly the durable, structured, baseline-resident artifact RFC-36 said the design layer lacked — so decisions become a first-class identity source the moment this RFC lands, with no bespoke scraper. It slots into RFC-36's identity-sources table beside the (still-future) `model.yaml` sub-trees:

| Source | Contributes | Priority |
| ------ | ----------- | -------- |
| `.specify/specs/<unit>/spec.md` | unit slugs + requirement titles → `surface[]` | Primary (what the project does) |
| `.specify/decisions/DEC-NNNN-*.md` | accepted-decision titles → `decisions[]` | **New (why it is shaped that way)** |
| `.specify/journal.jsonl` | per-merge summaries → `recent[]` | Secondary (what changed) |
| `project.yaml.description` | authored intent | Cold-start fallback |

### Why decisions improve routing

`surface[]` answers *what behaviour a project owns*; that is the primary discriminator, but it under-separates projects that own overlapping behaviour. Two services can both "Issue session token" (identical `surface[]`) yet differ in architectural commitment — one decided on DPoP sender-constrained tokens, the other on opaque bearer tokens. When the reconciliation agent binds a slice about *token rotation* or *replay protection*, the decision axis is the signal that routes it to the right project. Decisions also carry the constraints a new slice must respect, so surfacing them at plan time lets the agent flag a lead that contradicts an accepted decision before Gate 1, not at build.

### Projection contract

The projection obeys RFC-36 **D36-6 (deterministic projection only)** — it is structural and byte-stable, never an LLM summary, because the topology lock is committed and verified by regenerate-and-compare:

- **Source filter:** only baseline records with `status: accepted`. `superseded` and `rejected` records describe past or not-taken posture, so they are excluded from *current* identity (the same way `surface[]` reflects current requirements, not removed ones).
- **`decisions[].id`** is the `DEC-NNNN`; **`decisions[].title`** is the record's H1 heading text. No body, `Context`, or `Consequences` prose is projected.
- **Order** is `DEC-NNNN` ascending — stable, never relevance-ranked (ranking would break D36-6).
- **Bounded** the same way `surface[]` is: up to `K` decisions (default `K = 8`, the most recent `DEC` ids), with a `more:` integer counting the elided remainder. Decisions are a slow-growing, project-wide set, so this stays small.

```yaml
# .specify/topology.lock — one project entry, decisions[] beside surface[]/recent[]
- name: identity-service
  target: omnia@v1
  description: "Omnia identity service implementing auth and password flows."
  surface:
    - unit: session
      requirements: ["Issue session token", "Revoke session"]
  decisions:
    - { id: DEC-0007, title: "Use PostgreSQL for the identity store" }
    - { id: DEC-0011, title: "DPoP sender-constrained access tokens" }
  recent:
    - "session: added token revocation"
```

Empty `decisions` stays off the wire, so a greenfield project (no merged decisions yet) degrades cleanly to `surface[]` + `description`, exactly as RFC-36 specifies. Identity auto-sharpens as decisions merge — zero operator tag maintenance, consistent with RFC-36's "derive, don't author" thesis.

### Reused machinery

This axis adds **no new mechanism** — it rides RFC-36's substrate:

- `specrun workspace sync` extends its baseline projection to read `.specify/decisions/` alongside `.specify/specs/` and write the bounded `decisions[]` into `topology.lock` (caps applied in the projection, per RFC-36's "caps live in the projection, not the wire").
- `specrun plan validate` extends its existing `topology-cache-stale` comparison to include the decisions projection (RFC-36 **D36-5**: staleness, not synchronisation). No new finding code.
- The reconciliation request (`proposal.schema.json#/$defs/projectRef`) gains an optional `decisions[]`, forwarded unchanged from the lock (RFC-36 **D36-4**: derived identity reaches the envelope).

## Wire contracts

Appends to the shared RFC-29 wire-contract registry:

- **Schema:** `decision.schema.json` (`DECISION_JSON_SCHEMA`) — validates the front-matter block (`slug` required; `status` closed enum `accepted | rejected | superseded`; optional `supersedes[]`, `related[]`; engine-stamped `id` / `slice` / `date` / `superseded-by` optional in the slice-authored form, required on the persisted baseline form).
- **Validation codes:** `decision-record-schema`, `decision-record-section-missing`, `decision-slug-grammar`, `decision-slug-collision`, `decision-supersede-orphan` — all `Error::Validation` / slice-doctor findings.
- **Ledger field:** `slice.archive.created` gains an optional `decisions[]` array of the `DEC-NNNN` ids promoted by the merge (no new event kind). This is the durable record alongside git history of `.specify/decisions/`.
- **Identity projection (extends RFC-36):** `topology-lock.schema.json` gains an optional `decisions[]` per project (`{ id, title }`, plus optional `more:` count); `proposal.schema.json#/$defs/projectRef` gains the same optional `decisions[]`. No new validation code — staleness folds into RFC-36's existing `topology-cache-stale`.

## Operator surface

```bash
# authored at refine (agent-written); inspected by:
specrun slice validate <slice>          # decision-record-* findings
specrun slice merge <slice>             # promotes decisions/*.md → .specify/decisions/DEC-NNNN-*.md
```

No new top-level verb. A read-only `specrun decisions list` projection is a candidate follow-on but is **not** required for this RFC — `ls .specify/decisions/` and git are sufficient at first.

## Implementation checklist

The workflow contract spans both repos (parent-repo AGENTS §"Note to the implementing agent").

**`augentic/specify-cli`:**
- Add `crates/schema/src/` `DECISION_JSON_SCHEMA` + `schemas/decision.schema.json`.
- Add a `decisions` parser (front-matter + section check) under `crates/model/src/`.
- Add the five `decision-*` findings to `crates/validate/`.
- Extend the `specrun slice merge` engine (`crates/workflow/src/slice/`) with the decisions promotion pass, id assignment, and supersede flip; thread the injected clock for `date`.
- Add the optional `decisions[]` field to the `slice.archive.created` journal event (`crates/workflow/src/journal.rs`).
- Identity (extends RFC-36): add `decisions[]` to `topology-lock.schema.json` and `proposal.schema.json#/$defs/projectRef`; extend the baseline projection in `specrun workspace sync` (`crates/workflow/src/change/plan/core/propose.rs` topology path) to read `.specify/decisions/` and emit the bounded `decisions[]`; extend the `topology-cache-stale` comparison to cover it.
- Tests: golden merge fixtures (fresh add, supersede, multi-record id ordering); validate fixtures for each finding; a topology-lock projection fixture asserting accepted-only, `DEC` ordering, and the `K`/`more:` cap.

**`augentic/specify` (this repo):**
- `/spec:refine` skill + the refine artifact-conventions reference: document the optional `decisions/<slug>.md` artifact and its format.
- `/spec:merge` SKILL + `references/merge-runbook.md`: add a "Decisions Promoted" line to the summary template.
- `docs/explanation/artifacts.md` and `docs/explanation/augentic-specify-usage.md`: add Decision Records to the artifact lifecycle and the merge-folds-into-baseline description.
- Add a decision-log entry recording this choice (decisions over state; opaque-add over prose delta-merge; `.specify/decisions/` home).

## Out of scope

- **Structured `model.yaml` decisions sub-tree.** A kernel-projected `decisions[]` sub-tree on `model.yaml` (the richer, merge-keyed enrichment path RFC-29c deferred its non-requirements sub-trees toward) is a future RFC. RFC-37 deliberately ships the lighter opaque-add Markdown catalogue first and does **not** depend on that sub-tree landing; if it lands, these `DEC-NNNN` records become its projection target.
- **Relevance-ranked identity.** The `decisions[]` projection is in scope (§"Decision Records as an identity source"), but only as a stable, `DEC`-ordered, capped list. Ranking decisions by relevance to a lead, or weighting the binding score by decision overlap, would break RFC-36 **D36-6** determinism and is deferred to whatever future RFC makes binding itself score-based.
- **Design *state* / living architecture docs.** C4 / arc42-style living architecture documentation and any auto-derived design-state surface are not in scope; design state stays the job of the code and the future model sub-trees.
- **Code-to-decision back-links.** Tooling to enforce that code seams reference a `DEC-NNNN` (the `grep -r 'DEC-' src/` discipline) is a standards-layer concern for a later RFC, not part of the merge contract.

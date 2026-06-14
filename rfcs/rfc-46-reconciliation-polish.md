# RFC-46: Reconciliation Polish — Typed Lead-Side Fields and Deterministic Consumers

> Status: Proposed · Depends: [From sources to slices](../../plugins/spec/references/reconciliation.md), the plan-time propose envelope (`schemas/discovery/proposal.schema.json`, `crates/workflow/src/change/plan/core/propose/`), the discovery lead schema (`schemas/discovery/lead.schema.json`) · Roadmap: the "Reconciliation polish" current-priority track; answers Open Question #1 ("which reconciliation-polish surface lands first?").

## Abstract

Plan-time reconciliation — the propose sub-step of `/spec:plan` that groups surveyed leads into slices — carries exactly one cross-source signal today: a freeform `synopsis` string per `(source, lead)`. The agent reads every `synopsis` and emits `slices[]` directly. This is correct as far as the *judgment* goes ("are these two leads the same work?"), but it leaves the substrate around that judgment thin: there is nothing typed for a deterministic layer to join on, no reproducible candidate-set the agent reasons over, and no mechanical check that the agent's grouping honoured the coverage and conflict invariants the [reconciliation reference](../../plugins/spec/references/reconciliation.md) already names as *rules*.

This RFC adds **typed lead-side fields** and the **deterministic consumers** built on them, *without moving the matching decision off the agent*:

1. **`topics[]`** — a typed, agent-populated field on each discovery lead. It is the substrate the rest of the track is blocked on.
2. **Advisory `clusters[]`** — deterministic groupings over `topics[]`, computed by the CLI during request assembly and surfaced to the propose agent as a hint it may override.
3. **Binding `affinity`** — a typed pre-grouping signal on a lead derived from its source binding, narrowing the agent's candidate space.
4. **Decision-conflict warnings** — a deterministic pass that surfaces, per lead, which accepted Decision Records share a topic, so the agent (and Gate 1) can check for contradiction.
5. **`advisory-context`** — wiring baseline context into slice-time synthesis (`/spec:refine`), separate from the plan-time items above.
6. **Greenfield identity seed** — a registry-level seed so a fresh project routes before any baseline exists.

The determinism lives in the **consumers** (clustering, coverage, conflict warnings), not in producing the fields: survey is agent-driven, so the agent populates `topics[]`. The win is moving the agent's output from un-checkable prose into typed facts the CLI can join, check, and reproduce. This RFC adds no lifecycle authority: every hint is advisory and the agent still emits `slices[]`; the deterministic checks are validation findings, not silent rewrites.

## Motivation

The reconciliation reference already states three invariants that "keep this predictable" — one-lead-per-source-per-slice, at-least-once coverage, and surfaced-not-hidden uncertain matches — and the roadmap principle *Core owns reconciliation* commits that "if a rule decides how sources combine… it belongs in the CLI or a CLI-owned schema — not only in a skill body." Today those invariants live partly in prose and partly in the propose kernel, while the *input* the agent groups on is an untyped `synopsis`. Three concrete gaps follow:

- **The candidate set is not reproducible.** With only `synopsis` blobs, the agent re-derives "which leads might match" from scratch each run. The grouping is therefore non-reproducible at the *candidate* level, which undercuts the reference's opening promise of a clear trail back to where every requirement came from.
- **Matching is O(N·M) prose comparison and degrades with scale.** A change with many sources and many leads forces the agent to weigh every `synopsis` against every other. The roadmap notes the loop is "proven on realistic multi-repo flows" — which is exactly where unbounded pairwise prose reasoning starts to cost quality.
- **Coverage and conflict are asserted, not enforced on a typed surface.** "Every lead referenced at least once" and "matched leads that materially disagree → `divergence: likely`" are set operations and predicates. Built on a typed `topics[]` substrate they become deterministic findings; built on prose they remain an LLM re-reading itself.

### Trigger conditions

This RFC is **proposed for activation now** — it is the roadmap's stated highest-attention track and is blocked on a sequencing decision rather than on prerequisites. The sequencing answer (see §Phasing) is the decision this RFC asks Gate-equivalent reviewers to ratify.

## Principles

- **The agent owns the judgment; the CLI owns the substrate.** The agent decides whether two leads are the same work. The CLI owns candidate-set construction, the coverage/conflict guarantees, and the audit trail. No field in this RFC makes the *matching decision* for the agent.
- **Typed, not deterministic-at-source.** Survey is agent-driven, so `topics[]` is agent-populated. "Deterministic" describes the *consumers* of the typed fields, not their production.
- **Advisory by default; binding only where named.** `clusters[]` and decision-conflict surfacing are advisory hints the agent may override. Only `affinity` is a binding pre-grouping, and even it does not force a match — it narrows the candidate space.
- **No lifecycle authority.** Every check this RFC adds is a `specify slice validate` / propose-time finding. It never transitions a slice, stamps a plan, or rewrites a `slices[]` row.
- **The CLI is authoritative.** The schema, the clustering pass, and the coverage/conflict checks live in `augentic/specify-cli`. The survey and propose skill briefs in `augentic/specify` consume them; they do not reimplement them.
- **Additive and back-compatible.** Every new field is optional. A lead with no `topics[]` reconciles exactly as today (synopsis-only); the hints raise the floor without breaking the degenerate N=1 path.

## Design

### Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 `topics[]` lead field** | Add an optional `topics[]` (array of kebab-case slugs) to the discovery lead. The survey agent populates it; absent means "unclassified" and falls back to synopsis-only matching. | Widen `schemas/discovery/lead.schema.json` and the embedded `LEAD_JSON_SCHEMA` additively; mirror onto `LeadCatalogEntry` (`crates/workflow/src/change/plan/core/propose/wire.rs`) so it rides the `kind: request` envelope. Update the `specify_model::discovery::Lead` parser. |
| **D2 Advisory `clusters[]` in the request** | Add an optional advisory `clusters[]` to `ProposalRequest`: deterministic groupings of `(source, lead)` rows that share one or more topics, each cluster naming its topic basis. Kernel-ignored on the response — purely an agent hint. | `build_request` / `build_catalog` compute clusters by topic set-intersection; widen `schemas/discovery/proposal.schema.json` request branch. No change to `ProposalResponse`. |
| **D3 Binding `affinity`** | Add an optional `affinity` signal to a catalog row indicating a binding-level pre-grouping (e.g. leads a single source declares as related). Narrows the agent's candidate space; never forces a match. | Optional field on `LeadCatalogEntry` (and its discovery-lead origin); populated by the source adapter's `survey` brief, schema-validated by the CLI. |
| **D4 Decision-conflict warnings** | A deterministic pass that, per lead, surfaces which of a bound project's accepted Decision Records share a topic with the lead — a *candidate contradiction* hint. The contradiction judgment stays with the agent and Gate 1. | Join `ProjectRef.decisions[]` (already projected into the request) against lead `topics[]`; emit as advisory request annotations and/or a `specify slice validate` finding. |
| **D5 Coverage as a typed check** | Promote at-least-once coverage and the "materially-disagreeing matched leads → `divergence: likely`" rule to deterministic checks over the typed substrate, surfaced as propose-time / `specify slice validate` findings rather than relying on prose self-grading. | Extend the propose kernel's response validation and/or `crates/workflow/src/change/plan/core/validate.rs`; reuse the existing orphan/coverage plumbing. |
| **D6 `advisory-context` at synthesis** | Surface baseline spec context (owned domains, related requirement titles) to the slice-time synthesize agent in `/spec:refine`, mirroring how `ProjectRef.surface[]` already informs plan-time binding. Advisory only. | Extend the slice synthesize request the CLI assembles for `/spec:refine`; no change to `spec.md` / `model.yaml` shape. |
| **D7 Greenfield identity seed** | Allow `registry.yaml` to carry an optional greenfield routing seed so a fresh project with no baseline projection still routes leads at plan time. | Optional seed field consumed by `resolve_topology` / `build_request` when `surface[]` is empty; documented under the *One authored home per fact* principle. |
| **D8 Repo split** | Schema, clustering pass, coverage/conflict checks, and the request annotations live in `augentic/specify-cli`. The survey brief (populating `topics[]` / `affinity`) and the propose brief (consuming `clusters[]` / warnings) live in `augentic/specify`. | New/extended schemas in `crates/schema/`; consumers in `crates/workflow/`; brief edits under `adapters/sources/*/briefs/survey.md` and `plugins/spec/skills/plan/` + `plugins/spec/references/reconciliation.md`. |

### `topics[]` on a discovery lead (D1)

```yaml
## Lead inventory

- lead: user-registration
  source: legacy-monolith
  synopsis: >-
    POST /users creates an account; enforces unique email and a min-length
    password; emits a UserCreated domain event.
  topics: [identity, account-creation, validation]
```

### Advisory `clusters[]` in the `kind: request` envelope (D2)

```json
{
  "version": 1,
  "kind": "request",
  "projects": [ /* … ProjectRef … */ ],
  "leads": [
    { "source": "legacy-monolith", "lead": "user-registration", "synopsis": "…", "topics": ["identity", "account-creation"] },
    { "source": "design-notes",    "lead": "user-registration", "synopsis": "…", "topics": ["identity", "account-creation"] }
  ],
  "clusters": [
    {
      "topics": ["identity", "account-creation"],
      "members": [
        { "source": "legacy-monolith", "lead": "user-registration" },
        { "source": "design-notes",    "lead": "user-registration" }
      ]
    }
  ]
}
```

The agent reads `clusters[]` as "these rows are likely the same work — confirm or split." It still emits `slices[]` in the existing `ProposalResponse` shape; the kernel ignores `clusters[]` on the way back (it is request-only).

### CLI surface

No new top-level verbs. The hints flow through the existing propose envelope and validation:

```bash
specify plan propose --dry-run --format json   # request now carries topics[]/affinity/clusters[]/decision warnings
specify plan propose --from <response.json>     # response validation now enforces D5 coverage/divergence on the typed substrate
specify slice validate <slice>                   # decision-conflict + coverage findings surface here too
```

### Relationship to the existing surfaces

| Concern | Today (`synopsis` only) | This RFC |
| --- | --- | --- |
| Cross-source candidate set | re-derived by the agent each run | deterministic `clusters[]` over `topics[]` (D2) |
| Same-source pre-grouping | none | binding `affinity` (D3) |
| Contradicts an accepted decision | agent reads `decisions[]` ad hoc | deterministic topic-join warning (D4) |
| Coverage / divergence | prose invariant + partial kernel checks | typed checks / findings (D5) |
| Baseline context at synthesis | plan-time `surface[]` only | `advisory-context` at refine (D6) |
| Greenfield routing | `description` alone | optional registry seed (D7) |

## Phasing — the answer to Open Question #1

The roadmap's Open Question #1 asks which reconciliation-polish surface lands first. This RFC's answer:

1. **Phase 1 — `topics[]` (D1).** Ship the typed field first. It is inert on its own but is the substrate every deterministic consumer joins on; nothing else in the track can be built without it. Risk is low (additive optional field, schema + parser).
2. **Phase 2 — advisory `clusters[]` (D2) + coverage/divergence checks (D5).** The first consumers: deterministic grouping the agent reads, and the typed coverage/conflict guarantees. This is where the track starts paying for itself.
3. **Phase 3 — binding `affinity` (D3) + decision-conflict warnings (D4).** Further candidate-space narrowing and the decision-record cross-check, both layered on the now-stable substrate.
4. **Phase 4 — `advisory-context` (D6) + greenfield seed (D7).** Slice-time synthesis context and greenfield routing — independent of the plan-time chain above and sequenceable last.

Phases 1–3 are strictly ordered (each consumes the prior). Phase 4 may proceed in parallel once Phase 1 lands.

## Alternatives considered

- **Make the matching decision deterministic (topic-equality merges leads).** Rejected. Semantic equivalence of two leads genuinely needs judgment; a deterministic topic/string match would be brittle and silently wrong. The hints are explicitly the inputs and guardrails around the decision, not the decision.
- **Richer free-text `synopsis` instead of typed fields.** Rejected. Prose cannot be joined, clustered, or checked deterministically, which leaves the candidate set non-reproducible and the coverage/conflict invariants un-enforceable — the exact gaps this RFC closes.
- **Compute `topics[]` deterministically in the CLI (keyword extraction).** Rejected for v1. Survey is agent-driven and the agent already reads each source in depth; a separate deterministic extractor would duplicate that read with worse recall. The CLI's determinism is best spent on the *consumers*, not on re-deriving topics.
- **Make `clusters[]` binding (kernel auto-merges clustered leads).** Rejected. That would move the matching decision off the agent, violating the lead principle. `clusters[]` stays advisory and request-only.
- **Put the clustering/checks in the propose skill body.** Rejected by *Core owns reconciliation* — any rule deciding how sources combine belongs in the CLI or a CLI-owned schema.

## Non-Goals

- Making the lead-matching judgment deterministic, or auto-merging leads from clusters/affinity.
- Any lifecycle authority — no slice transition, plan stamp, or `slices[]` rewrite from these hints.
- Deterministic *production* of `topics[]` (the agent populates them during survey).
- Grading synthesized prose quality; `advisory-context` only supplies context, it does not judge output.
- A new top-level CLI verb — the hints ride the existing propose envelope and validation surfaces.
- Backstage/catalog import of topics (RM-12 territory); `topics[]` here is survey-authored.

## Open Questions

1. **Topic vocabulary.** Should `topics[]` be free kebab slugs, or a per-change controlled vocabulary the survey agent must draw from (better clustering, more authoring friction)? Current preference: free slugs in Phase 1, revisit a vocabulary if cluster precision is poor.
2. **Cluster overlap.** May a `(source, lead)` row appear in more than one cluster (multi-topic leads), or should clusters partition? Current preference: allow overlap — multi-homed leads are already a first-class case.
3. **`affinity` provenance.** Is binding `affinity` declared by the source adapter's survey brief, or derived by the CLI from co-occurrence in one source's lead set? Current preference: adapter-declared, CLI-validated.
4. **Decision-conflict strength.** Should a topic-sharing accepted decision produce a propose-time warning only, or also a `specify slice validate` finding at slice time? Current preference: both, advisory at each.
5. **Greenfield seed shape.** What is the minimal seed in `registry.yaml` that routes leads before any baseline exists, without re-introducing adapter/description duplication the *One authored home per fact* principle forbids?

## References

- [From sources to slices](../../plugins/spec/references/reconciliation.md) — the two reconciliation moments, the lead/slice invariants, and the provenance trail this RFC builds on.
- [Authority hierarchy](../../plugins/spec/references/synthesis/authority.md) — the disagreement-resolution order the `divergence` check (D5) interacts with.
- `schemas/discovery/lead.schema.json` and `schemas/discovery/proposal.schema.json` (in `augentic/specify-cli`) — the schemas D1/D2/D3 widen.
- `crates/workflow/src/change/plan/core/propose/` (`wire.rs`, `catalog.rs`, `kernel.rs`) — the propose envelope and projection kernel the consumers extend.
- [Roadmap](../roadmap.md) — the "Reconciliation polish" current-priority track and Open Question #1 this RFC answers.

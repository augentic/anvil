# RFC-46: Reconciliation Polish — Typed Lead-Side Fields and Deterministic Checks

> Status: Accepted (sequencing ratified — see §Phasing) · Depends: [From sources to slices](../../plugins/spec/references/reconciliation.md), the plan-time propose envelope (`schemas/discovery/proposal.schema.json`, `crates/workflow/src/change/plan/core/propose/`), the discovery lead schema (`schemas/discovery/lead.schema.json`) · Roadmap: the "Reconciliation polish" current-priority track; answers Open Question #1 ("which reconciliation-polish surface lands first?").

## Abstract

Plan-time reconciliation — the propose sub-step of `/spec:plan` — groups surveyed leads into slices. **Every grouping judgment belongs to the agent and stays there**: the agent reads each lead, decides which describe the same work, and emits `slices[]` directly. The CLI computes no groupings. What it owns is the **typed schema** those judgments are recorded in and the **mechanical checks** that verify the result — nothing that decides how leads combine.

The RFC adds the following, *without moving any matching or grouping decision off the agent*:

1. **`topics[]`** — a typed, agent-populated field on each discovery lead, giving the agent richer per-lead context and giving the checks below a typed surface to verify against. It is the substrate the rest of the track builds on.
2. **Coverage check** — a deterministic set-difference over what the agent emitted: every surveyed lead must be referenced by at least one slice. Verification of the agent's output, not a grouping.
3. **Decision-contradiction warning** — a deterministic surfacing of which accepted Decision Records share a topic with a lead, so the agent (and Gate 1) can judge contradiction. A warning, not a decision.
4. **`baseline`** — wiring baseline context into slice-time synthesis (`/spec:refine`), separate from the plan-time items above.
5. **Greenfield identity seed** — a registry-level seed so a fresh project routes before any baseline exists.

The determinism lives only in **checking and surfacing what the agent emitted**, never in producing or grouping leads. The headline win is **verification**: the coverage invariant the [reconciliation reference](../../plugins/spec/references/reconciliation.md) names as a *rule* runs as a mechanical set-operation over a typed surface, and the decision-record cross-check surfaces candidate contradictions the agent then judges. This RFC carries no lifecycle authority: every typed field is a hint the agent may ignore, and the checks are validation findings, not silent rewrites.

## Motivation

The roadmap principle *Core owns reconciliation* commits that "if a rule decides how sources combine… it belongs in the CLI **or a CLI-owned schema** — not only in a skill body." The emphasis matters: the principle is discharged by owning the *schema* the agent's judgments are recorded in and the *checks* over the result — not by the CLI computing groupings. Typed lead-side fields put the reconciliation invariants the [reconciliation reference](../../plugins/spec/references/reconciliation.md) names — one-lead-per-source-per-slice, at-least-once coverage, and surfaced-not-hidden uncertain matches — on a surface the CLI can enforce without making any matching decision. Two things follow:

- **Coverage becomes an enforced guarantee.** This is the capability that most justifies the RFC. "Every surveyed lead referenced by at least one slice" is a set-difference over what the agent emitted; the CLI verifies it *after* the agent emits `slices[]`. It needs no topics at all — it runs over the slice/lead sets directly — so the guarantee holds regardless of how the agent labelled anything.
- **Typed facts make the result auditable.** `topics[]` is an agent-authored fact the CLI can validate, diff, surface at Gate 1, and join against accepted decisions for a contradiction *warning*. The contrast with a freeform `synopsis` is that a typed field can be checked and joined; it is not an attempt to have the CLI re-derive what the agent already judged.

### Trigger conditions

This RFC is **active** — it is the roadmap's stated highest-attention track and was blocked on a sequencing decision rather than on prerequisites. The sequencing answer (see §Phasing) is ratified, and Phase 1 (the D2 coverage check) is ready to implement.

## Principles

- **The agent owns every grouping judgment; the CLI computes none.** The agent decides whether two leads are the same work and expresses that decision in `slices[]`. The CLI never groups, clusters, or pre-judges leads. No field in this RFC produces a grouping for the agent.
- **The CLI's only determinism is checking and surfacing.** It owns the typed schema, a coverage check over the agent's emitted slices, and a decision-contradiction *warning*. These verify or surface; they never construct a candidate set or decide a match.
- **Typed facts are authored, not derived.** `topics[]` is agent-populated during survey. The CLI validates and joins it but does not generate it, and a missing field never blocks reconciliation.
- **No lifecycle authority.** Every check this RFC adds is a `specify slice validate` / propose-time finding. It never transitions a slice, stamps a plan, or rewrites a `slices[]` row.
- **The CLI owns the schema and the checks; skills own the judgment.** The typed schema and the coverage/contradiction checks live in `augentic/specify-cli`. The survey and propose skill briefs in `augentic/specify` author the fields and make the grouping calls; neither side reimplements the other.
- **Additive and optional.** Every new field is optional. A lead with no `topics[]` reconciles on its synopsis alone; the typed fields raise the floor without breaking the degenerate N=1 path.

## Design

### Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 `topics[]` lead field** | Add an optional `topics[]` (array of kebab-case slugs) to the discovery lead. The survey agent populates it as per-lead context; absent means "unclassified". The CLI computes no grouping from it — it is context for the agent and a join key for the decision-contradiction warning. | Widen `schemas/discovery/lead.schema.json` and the embedded `LEAD_JSON_SCHEMA` additively; mirror onto `LeadCatalogEntry` (`crates/workflow/src/change/plan/core/propose/wire.rs`) so it rides the `kind: request` envelope. Update the `specify_model::discovery::Lead` parser. |
| **D2 Coverage check** | A deterministic set-difference: every surveyed lead must be referenced by at least one slice the agent emitted. Verification of the agent's output — needs no topics. | Extend the propose kernel's response validation and/or `crates/workflow/src/change/plan/core/validate.rs`; reuse the orphan/coverage plumbing. Surfaced as propose-time / `specify slice validate` findings. |
| **D3 Decision-contradiction warning** | A deterministic surfacing of which of a bound project's accepted Decision Records share a topic with a lead — a *candidate contradiction* the agent and Gate 1 then judge. Never a grouping or a match; a warning only. | Four coordinated changes, since both sides of the join need a topic key today: (1) add an optional `topics[]` to `decision.schema.json` (reuse the `slug` pattern, `uniqueItems`, slice-authored); (2) `/spec:refine`'s `decisions/<slug>.md` front-matter gains optional `topics`, documented in `artifact-conventions.md` + the refine SKILL; (3) add `topics: Vec<String>` to the `Decision` projection in `crates/workflow/src/registry/topology.rs`, populated by the merge-time projection over `.specify/decisions/`; (4) join `lead.topics × decision.topics` in the propose kernel over the already-projected `ProjectRef.decisions[]`. A lead carries no `REQ` id at plan time, so the `related[] → REQ-NNN` link cannot bridge — topics is the only viable key. Emit `lead-decision-topic-overlap` as a `kind: review` (non-blocking) finding at propose time and `specify slice validate`. Degrades gracefully: until decisions carry topics the join is empty. |
| **D4 Divergence stays agent-flagged** | The "matched leads that materially disagree → `divergence: likely`" rule is *not* mechanical — "materially disagree" is judgment, so the agent flags it in the propose **response** with a disagreement payload; the operator keeps final adjudication (`accepted` / `rejected`) at `specify plan amend --divergence`. The CLI's role is a structural-consistency check: a slice flagged `divergence` must record the disagreeing values. | `Divergence` is today a bare `none` / `likely` / `accepted` / `rejected` label with no slot for the disagreeing values, so add an optional `disagreements[]` (`{ field, values: [{ source, value }] }`) to the proposal slice row and mirror it into `model.yaml`. The check: if `divergence` ∈ {`likely`, `accepted`} then `disagreements[]` is non-empty and each entry records ≥2 distinct source values; the CLI never decides materiality. Emit `slice-divergence-unrecorded` (flag without values, `Important`) and `slice-divergence-orphan-values` (values without flag, `Suggestion`) at propose-response validation and `specify slice validate`. |
| **D5 `baseline` at synthesis** | Surface baseline spec context — the full owned-domain `surface[]` (all domains plus capped requirement titles, **no relevance selection**) — to the slice-time synthesize agent in `/spec:refine`, mirroring how `ProjectRef.surface[]` already informs plan-time binding. The agent decides relevance; the CLI only projects. Advisory only. | Add `baseline: Vec<Surface>` (reuse `crates/workflow/src/registry/topology::Surface`, capped by the existing `SURFACE_TITLE_CAP`) to the synthesize request the CLI assembles for `/spec:refine`, projected from `topology.lock`; empty stays off the wire (greenfield). No change to `spec.md` / `model.yaml` shape and no check. |
| **D6 Greenfield identity seed** | Allow `registry.yaml` to carry an optional `greenfield_seed.domains[]` (kebab slugs only) so a fresh project with no baseline projection still routes leads at plan time — the greenfield analog of `surface[]`'s domain list. Carries only intended domain slugs (a fact with no other home); never adapter or description material. | `resolve_topology` / `build_request` projects the seed domains into `ProjectRef.surface[]` as domains with empty `requirements[]` when the real `surface[]` is empty; the CLI ignores the seed once `.specify/specs/` exists (real surface supersedes it), optionally emitting `greenfield-seed-shadowed` (info) suggesting removal. Documented under the *One authored home per fact* principle. |
| **D7 Repo split** | The typed schema and the coverage/contradiction/consistency checks live in `augentic/specify-cli`. The survey brief (authoring `topics[]`) and the propose brief (making the grouping calls, reading the warnings) live in `augentic/specify`. | New/extended schemas in `crates/schema/` (`lead.schema.json` for D1; `decision.schema.json` for D3's join key); check logic in `crates/workflow/`; brief edits under `adapters/sources/*/briefs/survey.md` and `plugins/spec/skills/plan/` + `plugins/spec/references/reconciliation.md`. |

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

The agent reads each lead — `synopsis` and `topics[]` — and emits `slices[]` in the existing `ProposalResponse` shape. The CLI carries the typed fields into the `kind: request` envelope and runs its checks over the response; it never adds a grouping of its own.

### CLI surface

No new top-level verbs. The typed fields and checks flow through the existing propose envelope and validation:

```bash
specify plan propose --dry-run --format json   # request carries topics[] + decision-contradiction warnings
specify plan propose --from <response.json>     # response validation runs the D2 coverage + D4 consistency checks
specify slice validate <slice>                   # coverage + decision-contradiction findings surface here
```

### Finding codes

Every check rides the existing kebab-coded `Diagnostic` currency (cf. `orphan-slice-dir`, `slice-authority-override-orphan-source`). The wire contract this RFC adds:

| Code | Decision | Severity / kind | Raised when |
| --- | --- | --- | --- |
| `lead-coverage-orphan` | D2 | `Important` / violation | a surveyed lead is referenced by no slice |
| `slice-divergence-unrecorded` | D4 | `Important` / violation | a slice flags `divergence` but records no `disagreements[]` |
| `slice-divergence-orphan-values` | D4 | `Suggestion` / violation | `disagreements[]` present but `divergence: none` |
| `lead-decision-topic-overlap` | D3 | review (non-blocking) | a lead shares a `topic` with an accepted Decision Record |
| `greenfield-seed-shadowed` | D6 | info (non-blocking) | a `greenfield_seed` is still declared after `.specify/specs/` exists |

D1 (`topics[]`) and D5 (`baseline`) add no finding — both are inert typed inputs.

### Test plan

Each decision lands with golden coverage: propose golden fixture pairs (request + response) exercising every finding above plus a clean negative case, homed in `tests/workflow/propose.rs` and `tests/slice/validate.rs`, regenerated via `REGENERATE_GOLDENS`. D5's baseline projection gets a synthesize-envelope assertion in `tests/slice/synthesize.rs`; D6's seed gets a `resolve_topology` projection test. `cargo make ci` (under `RUSTFLAGS=-Dwarnings`) gates the lot.

## Phasing — the answer to Open Question #1

The roadmap's Open Question #1 asks which reconciliation-polish surface lands first. This RFC's answer:

1. **Phase 1 — the coverage check (D2).** The highest-value, judgment-free piece, and it needs no new field: coverage runs as a set-difference over the leads `survey` already emits and the `slices[]` the agent already returns. Shipping it first delivers the typed *guarantee* with the smallest surface.
2. **Phase 2 — `topics[]` (D1).** The typed per-lead context field, plus the structural divergence-consistency check (D4) over the agent's flag. Inert as input, but the substrate D3 joins on.
3. **Phase 3 — decision-contradiction warning (D3).** The topic-join warning, layered on the Phase 2 substrate. D3 additionally requires the `decision.schema.json` topic key called out in its row.
4. **Phase 4 — `baseline` (D5) + greenfield seed (D6).** Slice-time synthesis context and greenfield routing — independent of the plan-time chain above and sequenceable last.

Only D3 has a hard ordering (it consumes D1's `topics[]` and the decision topic key); D2 (coverage) stands alone. Phase 4 may proceed in parallel throughout.

## Alternatives considered

- **CLI-computed advisory `clusters[]` (a `GROUP BY topic` the agent reads as a hint).** Rejected — this was an earlier shape of the RFC and is the option that prompted the rewrite. It is circular: the agent authors the topics, and the propose agent already reads every lead across every source in one pass, so a CLI grouping by those same topics hands the agent back a weaker version of what it just produced. It is also garbage-in-garbage-out — topic-equality over free slugs over-clusters generic topics and misses synonyms — which is the exact brittleness the next bullet rejects, merely relabelled "advisory." The agent's grouping judgment already lives in `slices[]`; a separate CLI-computed cluster adds no information and muddies the agent-judges / CLI-checks split.
- **Make the matching decision deterministic (topic-equality merges leads).** Rejected. Semantic equivalence of two leads genuinely needs judgment; a deterministic topic/string match would be brittle and silently wrong.
- **Agent-authored `clusters[]` as a distinct artifact.** Rejected as redundant. An agent grouping leads *is* proposing slices; expressing the same judgment twice (once as clusters, once as `slices[]`) adds a second surface to keep consistent with no new signal.
- **Richer free-text `synopsis` instead of typed fields.** Rejected. Prose cannot be joined or checked, which leaves the coverage invariant un-enforceable and the decision-contradiction join impossible — exactly what the typed fields exist to provide.
- **Compute `topics[]` deterministically in the CLI (keyword extraction).** Rejected. Survey is agent-driven and the agent already reads each source in depth; a separate extractor would duplicate that read with worse recall. The CLI's determinism is spent on *checking* the agent's output, not on re-deriving its inputs.
- **Put the checks in the propose skill body.** Rejected by *Core owns reconciliation* — a coverage or consistency check belongs in the CLI or a CLI-owned schema, not only in a skill body.

## Non-Goals

- Any CLI-computed grouping of leads — no clustering, candidate-set construction, or auto-merge. Grouping is the agent's, expressed in `slices[]`.
- Making the lead-matching or divergence judgment deterministic.
- Any lifecycle authority — no slice transition, plan stamp, or `slices[]` rewrite from these fields or checks.
- Deterministic *production* of `topics[]` (the agent populates them during survey).
- Grading synthesized prose quality; `baseline` only supplies context, it does not judge output.
- A new top-level CLI verb — the typed fields and checks ride the existing propose envelope and validation surfaces.
- Backstage/catalog import of topics (RM-12 territory); `topics[]` here is survey-authored.

## Resolved decisions

The open questions from earlier drafts are ratified as follows:

1. **Topic vocabulary.** Free kebab slugs — no per-change controlled vocabulary. Topics feed agent context and the decision-contradiction join, not a CLI grouping, so the precision bar is low.
2. **Decision-contradiction strength.** Both surfaces: a propose-time annotation and a `specify slice validate` finding, emitted as `kind: review` (non-blocking) at each.
3. **Divergence-consistency scope.** Both placements: propose-response validation (plan time) and `specify slice validate` over `model.yaml` (slice time) — wherever the divergence flag is recorded.
4. **Greenfield seed shape.** `registry.yaml` carries `greenfield_seed.domains[]` (kebab slugs only), projected into an empty `surface[]` and ignored once a baseline exists — see D6. Carries only intended domain slugs, never adapter/description material, satisfying *One authored home per fact*.

## References

- [From sources to slices](../../plugins/spec/references/reconciliation.md) — the two reconciliation moments, the lead/slice invariants, and the provenance trail this RFC builds on.
- [Authority hierarchy](../../plugins/spec/references/synthesis/authority.md) — the disagreement-resolution order the agent-flagged `divergence` (D4) interacts with.
- `schemas/discovery/lead.schema.json` (in `augentic/specify-cli`) — the schema D1 widens; `decision.schema.json` gains the D3 join key.
- `crates/workflow/src/change/plan/core/propose/` (`wire.rs`, `catalog.rs`, `kernel.rs`) — the propose envelope and the validation the checks extend.
- [Roadmap](../roadmap.md) — the "Reconciliation polish" current-priority track and Open Question #1 this RFC answers.

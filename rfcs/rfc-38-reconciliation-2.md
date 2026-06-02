# RFC-38: Completing Lead Reconciliation

> Status: Draft — Depends: [From sources to slices §Plan time](../docs/explanation/reconciliation.md#plan-time-leads-become-slices) (plan-time lead reconciliation, the `proposal.schema.json` envelope) and [`DECISIONS.md` §Slice synthesis engine](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#slice-synthesis-engine-rfc-29-m2b) (slice synthesis, the deferred `advisory-context` input) — Sequenced-after: [RFC-36](rfc-36-project-identity.md) (shipped; this RFC enriches the *lead* side of the same envelope RFC-36 enriched on the *project* side) — Related: the [roadmap principle "Core owns reconciliation"](roadmap.md#principles) and [From sources to slices](../docs/explanation/reconciliation.md)

## Problem

[RFC-36](rfc-36-project-identity.md) sharpened exactly **one seam** of RFC-29's lead → slice reconciliation: *project binding*. It replaced description-prose-plus-rotting-tags with a deterministic baseline projection (`surface[]` / `decisions[]` / `recent[]`) that reaches the propose envelope. That is a genuine quality and correctness upgrade to the `projects[]` side of the request ([`DECISIONS.md` §Lead reconciliation](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lead-reconciliation-d2)).

They leave the rest of the flow untouched, and the enrichment is **asymmetric**:

- **The lead side is still thin.** A catalog row carries only `source`, `lead`, and per-source `synopsis` ([`DECISIONS.md` §Lead reconciliation](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lead-reconciliation-d2)). Cross-source *grouping* — the load-bearing "do these leads describe the same work" judgment — rests entirely on agent reading of synopses, with shape-only kernel guards. The two deterministic aids that would help (token-intersection clustering, per-lead `topics[]` survey metadata) are explicitly deferred ([`DECISIONS.md` §Lead reconciliation](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lead-reconciliation-d2) items 1–2; [From sources to slices §Plan time](../docs/explanation/reconciliation.md#plan-time-leads-become-slices)).
- **Binding is enriched but not surfaced as signal.** The richer `surface[]` / `decisions[]` axes are presented as raw lists for the agent to eyeball. Nothing computes or surfaces *affinity* between a lead and a project, and nothing flags a lead that contradicts an accepted decision before Gate 1 — RFC-36 names that guard as aspirational agent judgment with no field, finding, or check behind it ([RFC-36 §"Why decisions improve routing"](rfc-36-project-identity.md)).
- **The baseline `decisions[]` never reaches synthesis.** RFC-36 routes accepted decisions into *topology identity* only. RFC-29's Q2 "amnesiac synthesis" seam — feed prior wording, settled conflicts, and house terminology into the synthesis step as read-only context — stays open, and RFC-29c's `advisory-context` input stays deferred ([`DECISIONS.md` §Slice synthesis engine](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#slice-synthesis-engine-rfc-29-m2b)). The decision catalogue is the obvious feedstock, left unwired.
- **Greenfield gets nothing.** `surface[]` / `recent[]` / `decisions[]` are all empty until merges happen, so RFC-36 improves the *N*-th change, not the *first* — the common bootstrap still binds on `description` alone ([RFC-36 §"Greenfield seed"](rfc-36-project-identity.md)).

## Solution

Close the remaining seams with **additive, deterministic** surfaces that mirror the discipline RFC-36 set on the project side — never an LLM-ranked input, never a new authored fact, and never kernel-side auto-merge that overrides agent judgment.

- Give the *lead* side the same first-class signal the *project* side now has: a survey-time `topics[]` facet and a kernel-computed, advisory clustering hint.
- Surface project binding as a **deterministic affinity hint** (token overlap counts, not relevance ranking) plus an advisory decision-conflict flag, leaving the bind itself agent-owned.
- Wire the RFC-36 `decisions[]` catalogue (and the baseline `spec.md`) into the synthesis request as RFC-29c's read-only `advisory-context`, closing Q2.
- Seed greenfield identity so the first change is not blind.
- Reconcile the drifted RFC-29b prose in the same change.

Every kernel addition is a *hint or a warning*, never a lock: the agent still decides grouping and binding, and the operator still curates at Gate 1.

## Decisions

| ID | Decision |
| -- | -------- |
| **D38-1 Lead-side signal** | `lead.schema.json` gains an optional survey-time `topics[]` facet (source-adapter authored, kebab-case tokens). `proposal.schema.json` forwards it on each catalog row, and `specrun plan propose --dry-run` emits a deterministic advisory `clusters[]` block grouping rows whose `{lead} ∪ topics[]` token sets intersect across source keys. Clustering is a **hint** the agent may follow or ignore; the kernel never auto-merges (preserves [From sources to slices §Propose reconciles leads across sources](../docs/explanation/reconciliation.md#propose-reconciles-leads-across-sources)). |
| **D38-2 Deterministic binding affinity** | For each `(lead, project)` pair the dry-run envelope carries an advisory `affinity` integer — the count of shared kebab tokens between the lead's signal set and the project's `surface[]` requirement titles + `decisions[]` titles. It is structural and byte-stable (no ranking, no LLM), so it obeys [RFC-36 **D36-6**](rfc-36-project-identity.md). The agent still binds `project` explicitly; affinity never auto-binds. |
| **D38-3 Decision-conflict advisory** | When a lead's `topics[]` intersect an accepted decision's `conflicts-with[]` token set, propose surfaces a non-blocking `plan-reconcile-decision-conflict` advisory naming the `(lead, project, DEC-NNNN)` triple, so the agent can flag it in `change.md` before Gate 1 rather than at build. Advisory only — it never aborts propose and never transitions anything. |
| **D38-4 Decisions reach synthesis** | The D3 synthesis request (RFC-29c) gains the deferred read-only `advisory-context` input, populated deterministically from the bound project's accepted `.specify/decisions/` titles and the relevant baseline `spec.md` requirement titles. This closes [From sources to slices](../docs/explanation/reconciliation.md). It is context, not authority: synthesis never treats advisory context as Evidence and the authority enum is unchanged ([`DECISIONS.md` §Slice synthesis engine](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#slice-synthesis-engine-rfc-29-m2b)). |
| **D38-5 Greenfield identity seed** | When a project has no merged baseline, `specrun workspace sync` projects a one-line `seed:` derived deterministically from `project.yaml.description` into the lock entry, so the first change binds on a normalised signal rather than raw prose. The seed is dropped from the projection the moment `surface[]` becomes non-empty (auto-sharpen, per RFC-36). |
| **D38-6 RFC-29b errata (shipped)** | [From sources to slices §Plan time](../docs/explanation/reconciliation.md#plan-time-leads-become-slices) §"Envelope" and §"Project Binding" document `surface[]` / `recent[]` / `decisions[]` on `projects[]` and no longer reference the retired `capabilities[]` / `keywords[]` facets or the old `rfc-36-registry-projection.md` filename. |

## Lead-side signal (D38-1)

The project side gained structured signal (`surface[]`); the lead side should too. Two additions, both deterministic:

- **`topics[]` (survey time).** A source adapter may tag each lead with kebab-case domain tokens (`session-token`, `replay-protection`, …) it observed while sizing. This is the metadata [`DECISIONS.md` §Lead reconciliation](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lead-reconciliation-d2) said advisory clustering needed and [From sources to slices §Plan time](../docs/explanation/reconciliation.md#plan-time-leads-become-slices) deferred. Optional: a source that emits none degrades to today's `synopsis`-only behaviour.
- **Advisory `clusters[]` (propose time).** The kernel computes connected components over catalog rows whose `{lead} ∪ topics[]` token sets intersect *across different source keys* (never within one source — same-source fusion stays forbidden, [From sources to slices §Propose reconciles leads across sources](../docs/explanation/reconciliation.md#propose-reconciles-leads-across-sources)). The result is emitted as a hint:

```yaml
# specrun plan propose --dry-run output, beside leads[]
clusters:
  - members: [{ source: docs, lead: password-reset }, { source: legacy, lead: reset-password }]
    shared: [password-reset]            # the intersecting tokens that formed the edge
```

This is the token-intersection idea from [`DECISIONS.md` §Lead reconciliation](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lead-reconciliation-d2), demoted from a kernel *lock* to a kernel *hint*: the agent still emits `slices[]`, the kernel still enforces shape-only invariants, and Gate 1 still curates. A wrong cluster costs nothing because nothing keys off it.

## Deterministic binding affinity (D38-2)

`affinity` makes the RFC-36 identity axes *actionable* without ranking. For each `(lead, project)`:

```text
affinity = | tokens(lead.lead ∪ lead.topics)
          ∩ tokens(project.surface[].requirements ∪ project.decisions[].title) |
```

It is a plain set-intersection cardinality — order-free, byte-stable, and trivially regenerate-and-compare verifiable, so it cannot violate [RFC-36 **D36-6**](rfc-36-project-identity.md) the way a relevance ranking would ([RFC-36 §"Out of scope"](rfc-36-project-identity.md) deferred exactly that ranking). It is surfaced per project on each lead row as advisory context; the agent reads it as "this project most plausibly owns this lead" but remains the sole binder, and `plan-reconcile-project-binding-required` / `-orphan` are unchanged. For greenfield projects (empty `surface[]` / `decisions[]`) affinity is `0` for all candidates and the agent falls back to `description` exactly as today.

## Decisions reach synthesis (D38-4)

RFC-36 created the durable decision catalogue but pointed it only at topology routing. The same catalogue is the natural answer to [From sources to slices](../docs/explanation/reconciliation.md) (the "feed the existing baseline `spec.md` into synthesis as read-only context" litmus). This RFC populates RFC-29c's deferred `advisory-context` input deterministically from the bound project's baseline:

- **`decisions[]`** — accepted-decision titles for the bound project (the RFC-36 projection, reused verbatim).
- **`requirements[]`** — baseline `spec.md` requirement titles for units the slice touches, so prior wording and house terminology are visible to synthesis.

The boundary is strict, matching RFC-29c's existing posture: advisory context is **never** Evidence. It does not carry a `Sources:` line, never participates in authority resolution, and never produces `[conflict]` / `[divergence]` tags. It is read-only prior art that helps synthesis phrase a requirement consistently with the baseline — the consistency win the reconciliation loop currently lacks because each slice synthesises blind.

## Greenfield seed (D38-5)

The seed closes the cold-start gap without re-authoring a fact: it is a deterministic normalisation (lowercase, tokenise, dedupe) of the already-authored `project.yaml.description`, projected as `seed:` only while `surface[]` is empty. It gives `affinity` (D38-2) something to bite on for the first change and disappears automatically once a slice merges, so it never competes with real baseline signal and never needs operator maintenance — consistent with RFC-36's "derive, don't author" thesis.

## Wire contracts

Appends to [`DECISIONS.md` §Lead reconciliation](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lead-reconciliation-d2):

- **`lead.schema.json` (`LEAD_JSON_SCHEMA`):** gains an optional `topics[]` array of kebab tokens (survey-authored).
- **`proposal.schema.json` (`PROPOSAL_JSON_SCHEMA`):** request `leads[]` rows gain optional `topics[]` and a per-project advisory `affinity` integer; the request gains a top-level advisory `clusters[]` block. All advisory — none change the response schema or any coverage invariant.
- **`decision.schema.json` (`DECISION_JSON_SCHEMA`):** gains an optional `conflicts-with[]` token array on a record's front-matter, consumed by the D38-3 advisory.
- **Topology lock:** `topology-lock.schema.json` gains an optional `seed:` string per project (greenfield only; omitted once `surface[]` is non-empty).
- **Synthesis request (RFC-29c):** gains the optional read-only `advisory-context` object (`decisions[]`, `requirements[]`), forwarded from the bound project's baseline.
- **Validation codes:** `plan-reconcile-decision-conflict` — a non-blocking **advisory** finding (not an `Error::Validation` abort; it never changes the exit code). The advisory clustering and affinity surfaces add no finding codes.

## Implementation checklist

The workflow contract spans both repos (parent-repo AGENTS §"Note to the implementing agent").

**`augentic/specify-cli`:**
- `crates/schema/src/`: extend `LEAD_JSON_SCHEMA` (`topics[]`), `PROPOSAL_JSON_SCHEMA` (`topics[]`, `affinity`, `clusters[]`), `DECISION_JSON_SCHEMA` (`conflicts-with[]`), `TOPOLOGY_LOCK_JSON_SCHEMA` (`seed:`), and the synthesis request schema (`advisory-context`).
- `crates/workflow/src/change/plan/core/propose.rs`: compute the deterministic `clusters[]` (connected components over cross-source token intersections), per-`(lead, project)` `affinity`, and the `plan-reconcile-decision-conflict` advisory in the `--dry-run` projection. None touch `--from` invariants.
- `crates/workflow/src/` synthesis path (RFC-29c D3): populate `advisory-context` from `.specify/decisions/` accepted titles + touched-unit `spec.md` requirement titles for the bound project.
- `specrun workspace sync` (topology projection): emit the greenfield `seed:` and drop it once `surface[]` is non-empty.
- Tests: cluster fixtures (cross-source edge, no within-source edge, transitive components); an affinity fixture asserting byte-stability under reordering; a decision-conflict advisory fixture asserting exit 0; an `advisory-context` synthesis fixture asserting it never becomes Evidence; a greenfield seed fixture asserting the seed vanishes after first merge.

**`augentic/specify` (this repo):**
- `/spec:plan` skill: document reading `clusters[]` / `affinity` as hints and rendering any `plan-reconcile-decision-conflict` advisory into `change.md` for Gate 1.
- `/spec:refine` skill: document that synthesis may read `advisory-context` as read-only prior art, never as Evidence.
- `docs/explanation/`: note the lead-side signal and the synthesis advisory-context in the reconciliation narrative; add a decision-log entry (advisory-not-lock clustering; deterministic affinity over ranking; decisions-as-synthesis-context).

## Out of scope

- **Relevance-ranked binding / scored auto-bind.** `affinity` is a deterministic count surfaced to the agent, not a ranking or an auto-binder. LLM-weighted binding scores remain deferred ([RFC-36 §"Out of scope"](rfc-36-project-identity.md), [RFC-36 **D36-6**](rfc-36-project-identity.md)).
- **Kernel-side auto-merge.** Clustering stays advisory; the kernel never fuses leads or overrides agent grouping ([From sources to slices §Propose reconciles leads across sources](../docs/explanation/reconciliation.md#propose-reconciles-leads-across-sources)), and same-source fusion stays a Gate 1 operator action.
- **Semantic decision-conflict detection.** D38-3 is a deterministic token-intersection advisory, not an LLM contradiction judge; richer semantic conflict detection is a later RFC.
- **An agent-side defer bucket.** Total coverage stays the propose invariant ([From sources to slices §Propose reconciles leads across sources](../docs/explanation/reconciliation.md#propose-reconciles-leads-across-sources)); a defer bucket remains deferred to its own RFC.
- **Slice-time authority / conflict / divergence mechanics.** RFC-29c's authority enum and `[conflict]` / `[divergence]` derivation are unchanged; D38-4 adds context beside them, never inside them.

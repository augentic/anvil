# RFC-92: Operation Model Policy

> Status: Draft — model-economics track over implemented [RFC-86](rfc-86-change-facts.md) and [RFC-90](rfc-90-build-verification.md)
>
> Owns: the closed operation-key set for model routing, the per-operation route and bounded escalation table on the pinned capability profile, deployment binding of route names to providers, the model-usage fact, and cost attribution projections.
>
> Depends on implemented RFC-86 facts and RFC-90 phase identity. Extends [RFC-88](rfc-88-detached-changes.md)'s model-capability profile. Supplies the objects [RFC-103](rfc-103-outcome-learning.md) already references as `model-route-change` proposals and the `cost.model-usage` outcome field.
>
> Patch ownership: this RFC amends RFC-88 D3 by adding a `routes` table to the model-capability profile and RFC-103's outcome DTO by populating `cost`. Both remain unchanged.
>
> Evidence posture: [platform evaluation](platform.md#evidence-and-iteration-posture).

## Intent

*Make model selection a recorded policy and model spend an attributable fact.*

Every judgment leg in Emery — survey, extract, propose, decompose, synthesize, build, repair, review — currently draws whatever model the deployment provides. Two things follow. There is no lever: a boundary-review leg reading three summaries and a build leg writing a module have very different value per token, and today they cost the same. And there is no attribution: RFC-103's outcome record carries `cost: model-usage: unknown` with a DTO note that it must never be zero, precisely because nothing populates it.

For delivery priced on outcomes rather than hours, spend per slice is both a margin line and the input to the next quote. Emery can attribute it far more precisely than a session-level dashboard, because every model call already happens inside a typed operation bound to a slice, a phase ordinal, and an input digest. The identity exists; only the record is missing.

## Problem

**Selection is implicit and unrecorded.** A change built through a fast model and the same change built through a frontier model produce different artifacts from identical inputs, and nothing in the fact log distinguishes them. That is a hole in the reproducibility claim the rest of the architecture is built on.

**Spend is unattributable.** Provider dashboards aggregate by API key and time window. Neither dimension maps to a slice, a requirement, or a target, which are the only units an engagement is priced in.

**RFC-103 has a consumer with no producer.** Its `model-route-change` proposal kind and `cost` block both assume objects that no RFC defines. Learning cannot propose a change to a route that does not exist as a named, versioned thing.

## Terms

- An **operation key** is one member of the closed set of model-invoking operations below.
- A **route** is a named model binding: a model identity plus its reasoning-effort and context settings.
- A **route policy** selects one starting route and an optional ordered, bounded escalation ladder whose transitions are triggered only by closed engine-observed facts.
- A **route table** maps operation keys to route policies on a pinned capability profile. Its digest is part of the profile digest.
- A **route binding** is deployment-owned data mapping a route name to a concrete provider, endpoint, and credential.
- A **usage fact** is one journal event recording one completed model call's provider-reported consumption.

## Decisions

### D1 — Routes live on the pinned capability profile

RFC-88's model-capability profile already pins the values that decide how the model is asked to work. The route table joins it rather than becoming a second policy object with its own lifecycle:

```yaml
# model-capability profile — additive
id: delivery-v3
digest: sha256:…
thresholds: { … }
routes:
  default: { start: balanced }
  engine.topology: { start: frontier }
  engine.decompose: { start: frontier }
  engine.synthesize: { start: frontier }
  engine.boundary: { start: economy }
  source.survey: { start: economy }
  source.extract:
    start: economy
    escalate:
      - { on: answer-repair-exhausted, to: balanced }
  target.build: { start: frontier }
  target.repair:
    start: economy
    escalate:
      - { on: unchanged-failure-set, to: balanced }
  target.review: { start: frontier }
  target.merge: { start: economy }
```

The closed operation-key set derives from the existing typed enums plus the engine judgment legs, so it adds no new identity:

| Family | Keys |
| --------- | ------------------------------------------------------------------------------------------------ |
| Source | `source.survey`, `source.extract` |
| Target | `target.guidance`, `target.build`, `target.verify`, `target.repair`, `target.review`, `target.merge` |
| Engine | `engine.topology`, `engine.propose`, `engine.decompose`, `engine.synthesize`, `engine.boundary`, `engine.readiness` |

An absent key falls to `default`. The shorthand scalar `target.review: frontier` is canonicalized to `{ start: frontier }` with no escalation. A key whose operation has no model leg in a given deployment — `target.verify` under RFC-97 host verification, for instance — is simply never consulted; a route policy is a binding, not a requirement to invoke.

That closed key set is also Emery's form of **model specialization by role**: survey, extract, synthesize, build, verify, and review may bind different tiers without introducing an orchestrator-agent model. Comparable products pair a strong planning model with cheaper workers and skeptical validators; the analogue here is the pinned route table over operation keys, not a conversational scheduler that chooses models mid-run ([platform.md § Absorbed lessons](platform.md#lessons-absorbed-from-comparable-systems)).

Pinning is the point. A starting-route or escalation-ladder change produces a new profile digest and invalidates the epoch on exactly the same rule as a threshold change, because a plan authored under an economy-first policy is not the same plan as one authored under a frontier route, and coverage that pretended otherwise would be false.

### D2 — Routes name capability tiers; deployment binds them to providers

The profile names `frontier`, `balanced`, `economy`. Deployment policy maps those names to a provider, endpoint, model identity, and credential, on the same shape as RFC-97's profile policies and for the same reasons.

This keeps two things out of change artifacts. Provider topology never enters `plan.yaml`, so a change home stays portable and a client's gateway, self-hosted endpoint, or model-vendor contract is a deployment concern rather than something committed into a repository. And credentials never come near an artifact that gets archived and handed over.

The engine records the resolved route name and the provider-reported model identity in the usage fact, so the fact log stays honest about what actually ran without the profile having to know where it ran.

### D3 — Usage is a raw fact, never lifecycle authority

One event per completed model call, on RFC-97 D9's posture:

```yaml
kind: model.usage.recorded
operation: target.build
slice: orders-checkout
phase-ordinal: 2
route: frontier
route-step: 0
trigger: initial
model: cursor:…
input-tokens: 48210
cached-tokens: 31904
output-tokens: 6122
cost: unknown
elapsed-ms: 41307
```

`cost` carries the provider-reported figure when the provider supplies one and `unknown` when it does not. It is never zero and never locally estimated from a token price table, because an estimate that looks like a measurement will eventually be quoted as one.

Usage facts are observations. No count, threshold, or total participates in status projection, gap gating, member admission, or a lifecycle transition. A model call that succeeded but exceeded a budget is a successful model call.

### D4 — Cost is a projection over the fact union

```text
emery cost --by slice | requirement | phase | route | target
```

The projection joins usage facts to the units an engagement is priced in. Slice, phase, route, and target attribution is exact, because each is already a field on the operation key. Requirement attribution is derived through the slice's `model.yaml` requirement set, so it distributes a slice's spend across its requirements and is approximate by construction. The projection labels it as derived rather than presenting it beside exact figures without distinction.

`plan archive` summarizes terminal spend beside the carried-debt summary, and RFC-103 outcome records populate `cost` from the same union, which is what lets recurrence analysis ask whether an economy route on `source.extract` actually cost more downstream in repair rounds than it saved.

### D5 — Route policy is pinned; escalation is bounded and engine-selected

A run never invents or edits a route policy. It starts each operation on the profile's selected route and may advance only through that policy's ordered escalation ladder when a closed engine trigger is present.

The initial closed triggers are `answer-repair-exhausted`, `context-limit-refusal`, and RFC-97's `unchanged-failure-set`. Each names an engine fact or typed provider outcome; no model, adapter, prompt, readiness judgment, token total, elapsed-time threshold, or free-form classifier may select it. A trigger either advances to the next declared route for that operation or stops when no declared step remains. Every dispatch records the route step and triggering fact.

This is a deliberate divergence from opaque adaptive routing. An unrecorded router that quietly upgrades a model makes epoch coverage a partial description of what was authorized. A pinned economy → balanced → frontier ladder remains fully described by the covered profile, and its fact-triggered transitions are comparable across runs even though model outputs are not deterministic.

Route policy remains learnable offline. RFC-103 aggregates usage facts against escalation, repair counts, amendment rates, terminal outcomes, and conservation coverage, emits a `model-route-change` proposal, evaluates it blind, and promotes a new profile generation for future runs. Readiness may recommend a route policy before authorization, but a readiness score cannot trigger or redirect one in flight.

A route may separately name a provider-unavailability fallback, resolved from the pinned profile and recorded in the usage fact. Availability fallback does not consume an escalation step: it changes deployment binding, not capability intent.

## Implementation requirements

- Add the closed `OperationKey` enum derived from the existing `SourceOperation` / `TargetOperation` enums plus the engine judgment legs, with kebab-case wire ids.
- Add the `routes` table and closed escalation triggers to the model-capability-profile DTO, folded into the existing profile digest, rejecting unknown keys, route names, triggers, non-advancing steps, and ladders above the compiled bound.
- Add route binding to the deployment-provider layer beside RFC-97 profile policies. Engine crates carry no provider, endpoint, or credential constant.
- Thread the resolved route policy through the existing judgment dispatch so every model call carries its operation key, slice, phase ordinal, route step, and closed trigger without a new plumbing seam.
- Add the `model.usage.recorded` event to RFC-86's closed `EventKind` taxonomy.
- Add read-only `emery cost` with the four exact attributions and one clearly labelled derived attribution.
- Populate RFC-103's `cost` block and extend its aggregation dimensions with route and operation key.
- Integration coverage for profile-digest invalidation on route or ladder change, absent-key fallback to `default`, unknown-route/trigger rejection, deterministic escalation, availability-fallback recording, and the absence of any usage- or readiness-driven lifecycle effect.

## Acceptance criteria

1. Changing one route or escalation entry produces a new capability-profile digest and invalidates the prior closed-plan epoch.
2. Every completed model call emits exactly one usage fact carrying its operation key, slice, phase ordinal, resolved route, route step, closed trigger, and provider-reported model identity.
3. A provider that reports no cost yields `unknown`; no code path emits zero or a locally computed price.
4. `emery cost --by slice|phase|route|target` sums exactly to the usage fact union; `--by requirement` is derived and labelled as such.
5. No usage total, budget, or threshold changes a status projection, gap gate, admission decision, or lifecycle transition.
6. A run uses only routes in its pinned policy. Escalation occurs only in declared order on a matching engine trigger, and a provider-unavailability fallback resolves only from the pinned profile; both are recorded in usage facts.
7. Outcome records carry populated `cost` and support aggregation by route and operation key.
8. Route bindings, endpoints, and credentials appear in no change artifact, plan, or archive.

## Rejected alternatives

- **Automatic per-task model routing decided by a model, readiness score, or opaque classifier inside the run.** Convenient and directly opposed to the reproducibility claim: the recorded authorization would no longer describe what executed. The permitted form is a pinned bounded ladder advanced by closed engine facts; changes to that ladder are learned offline and promoted.
- **A single “orchestrator” route that plans and schedules.** Role specialization belongs on the closed operation-key set above; putting planning authority on a model route would recreate an agent orchestration layer, which [platform.md](platform.md#deliberately-rejected) rejects.
- **Cost as a lifecycle gate.** A budget that fails a merge turns spend into a correctness signal and creates pressure to accept unverified work to stay under a number. Cost informs pricing and learning; it never gates.
- **Provider endpoints and credentials in `plan.yaml`.** A change home is portable and archivable, which is exactly what deployment topology and secrets must not be.
- **A separate model-policy object with its own lifecycle.** Two pinned policies covering one epoch means two invalidation rules and an ordering question between them. The capability profile already pins how the model is asked to work.
- **Local cost estimation from a token price table.** A number that looks measured but is modelled will eventually be quoted to a client as measured. `unknown` is more useful than a plausible estimate.
- **A second identity for model calls.** The operation key already identifies the call uniquely within a change.

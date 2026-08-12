# RFC-92: Operation Model Policy

> Status: Draft — model-economics track over implemented [RFC-86](rfc-86-change-facts.md) and [RFC-90](rfc-90-build-verification.md)
>
> Owns: the closed operation-key set for model routing, the per-operation route table on the pinned capability profile, deployment binding of route names to providers, the model-usage fact, and cost attribution projections.
>
> Depends on implemented RFC-86 facts and RFC-90 phase identity. Extends [RFC-88](rfc-88-detached-changes.md)'s model-capability profile. Supplies the objects [RFC-93](rfc-93-outcome-learning.md) already references as `model-route-change` proposals and the `cost.model-usage` outcome field.
>
> Patch ownership: this RFC amends RFC-88 D3 by adding a `routes` table to the model-capability profile and RFC-93's outcome DTO by populating `cost`. Both remain unchanged.
>
> Evidence posture: [platform evaluation](platform.md#evidence-and-iteration-posture).

## Intent

*Make model selection a recorded policy and model spend an attributable fact.*

Every judgment leg in Emery — survey, extract, propose, decompose, synthesize, build, repair, review — currently draws whatever model the deployment provides. Two things follow. There is no lever: a boundary-review leg reading three summaries and a build leg writing a module have very different value per token, and today they cost the same. And there is no attribution: RFC-93's outcome record carries `cost: model-usage: unknown` with a DTO note that it must never be zero, precisely because nothing populates it.

For delivery priced on outcomes rather than hours, spend per slice is both a margin line and the input to the next quote. Emery can attribute it far more precisely than a session-level dashboard, because every model call already happens inside a typed operation bound to a slice, a phase ordinal, and an input digest. The identity exists; only the record is missing.

## Problem

**Selection is implicit and unrecorded.** A change built through a fast model and the same change built through a frontier model produce different artifacts from identical inputs, and nothing in the fact log distinguishes them. That is a hole in the reproducibility claim the rest of the architecture is built on.

**Spend is unattributable.** Provider dashboards aggregate by API key and time window. Neither dimension maps to a slice, a requirement, or a target, which are the only units an engagement is priced in.

**RFC-93 has a consumer with no producer.** Its `model-route-change` proposal kind and `cost` block both assume objects that no RFC defines. Learning cannot propose a change to a route that does not exist as a named, versioned thing.

## Terms

- An **operation key** is one member of the closed set of model-invoking operations below.
- A **route** is a named model binding: a model identity plus its reasoning-effort and context settings.
- A **route table** maps operation keys to route names on a pinned capability profile. Its digest is part of the profile digest.
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
  default: balanced
  engine.topology: frontier
  engine.decompose: frontier
  engine.synthesize: frontier
  engine.boundary: economy
  source.survey: economy
  source.extract: balanced
  target.build: frontier
  target.repair: balanced
  target.review: frontier
  target.merge: economy
```

The closed operation-key set derives from the existing typed enums plus the engine judgment legs, so it adds no new identity:

| Family | Keys |
| --------- | ------------------------------------------------------------------------------------------------ |
| Source | `source.survey`, `source.extract` |
| Target | `target.guidance`, `target.build`, `target.verify`, `target.repair`, `target.review`, `target.merge` |
| Engine | `engine.topology`, `engine.propose`, `engine.decompose`, `engine.synthesize`, `engine.boundary`, `engine.readiness` |

An absent key falls to `default`. A key whose operation has no model leg in a given deployment — `target.verify` under RFC-97 host verification, for instance — is simply never consulted; a route is a binding, not a requirement to invoke.

That closed key set is also Emery's form of **model specialization by role**: survey, extract, synthesize, build, verify, and review may bind different tiers without introducing an orchestrator-agent model. Comparable products pair a strong planning model with cheaper workers and skeptical validators; the analogue here is the pinned route table over operation keys, not a conversational scheduler that chooses models mid-run ([platform.md § Absorbed lessons](platform.md#absorbed-lessons-not-the-opposite-bet)).

Pinning is the point. A route table change produces a new profile digest and invalidates the epoch on exactly the same rule as a threshold change, because a plan authored under an economy route is not the same plan as one authored under a frontier route, and coverage that pretended otherwise would be false.

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

`plan archive` summarizes terminal spend beside the carried-debt summary, and RFC-93 outcome records populate `cost` from the same union, which is what lets recurrence analysis ask whether an economy route on `source.extract` actually cost more downstream in repair rounds than it saved.

### D5 — Routes are promoted, never adapted in flight

A run never re-routes itself. Not on a failed verification, not on a repair round, not on an observed budget.

This is a deliberate divergence from adaptive model-routing designs, and the reasoning is the same one that gives Emery its audit posture: identical pinned inputs must produce a comparable run under a recorded policy. A router that quietly upgrades a model mid-attempt makes the epoch's coverage a partial description of what was authorized, and makes replay and outcome comparison meaningless.

Route selection is still learnable — it is just learned offline. RFC-93 aggregates usage facts against repair counts, amendment rates, terminal outcomes, and conservation coverage, emits a `model-route-change` proposal, evaluates it blind, and promotes it into a new profile generation that future runs pin. The lever exists; it is pulled between runs, by a reviewed act, and it leaves a version.

The one permitted in-flight variation is deterministic and pre-declared: a route may name a fallback for provider unavailability, resolved from the pinned profile and recorded in the usage fact. Unavailability is not a quality judgment, and refusing to record the substitution would be worse than making it.

## Implementation requirements

- Add the closed `OperationKey` enum derived from the existing `SourceOperation` / `TargetOperation` enums plus the engine judgment legs, with kebab-case wire ids.
- Add the `routes` table to the model-capability-profile DTO, folded into the existing profile digest, rejecting unknown keys and unknown route names.
- Add route binding to the deployment-provider layer beside RFC-97 profile policies. Engine crates carry no provider, endpoint, or credential constant.
- Thread the resolved route through the existing judgment dispatch so every model call carries its operation key, slice, and phase ordinal without a new plumbing seam.
- Add the `model.usage.recorded` event to RFC-86's closed `EventKind` taxonomy.
- Add read-only `emery cost` with the four exact attributions and one clearly labelled derived attribution.
- Populate RFC-93's `cost` block and extend its aggregation dimensions with route and operation key.
- Integration coverage for profile-digest invalidation on route change, absent-key fallback to `default`, unknown-route rejection, fallback recording, and the absence of any usage-driven lifecycle effect.

## Acceptance criteria

1. Changing one route entry produces a new capability-profile digest and invalidates the prior closed-plan epoch.
2. Every completed model call emits exactly one usage fact carrying its operation key, slice, phase ordinal, resolved route, and provider-reported model identity.
3. A provider that reports no cost yields `unknown`; no code path emits zero or a locally computed price.
4. `emery cost --by slice|phase|route|target` sums exactly to the usage fact union; `--by requirement` is derived and labelled as such.
5. No usage total, budget, or threshold changes a status projection, gap gate, admission decision, or lifecycle transition.
6. A run completes on the routes it started with. A provider-unavailability fallback resolves only from the pinned profile and is recorded in the usage fact.
7. Outcome records carry populated `cost` and support aggregation by route and operation key.
8. Route bindings, endpoints, and credentials appear in no change artifact, plan, or archive.

## Rejected alternatives

- **Automatic per-task model routing decided inside the run.** Convenient and directly opposed to the reproducibility claim: the recorded authorization would no longer describe what executed. Selection is learned offline and promoted, which gets the same benefit with a version attached. An orchestrator agent that upgrades workers mid-mission is the same failure mode with a chat surface.
- **A single “orchestrator” route that plans and schedules.** Role specialization belongs on the closed operation-key set above; putting planning authority on a model route would recreate an agent orchestration layer, which [platform.md](platform.md#deliberately-rejected) rejects.
- **Cost as a lifecycle gate.** A budget that fails a merge turns spend into a correctness signal and creates pressure to accept unverified work to stay under a number. Cost informs pricing and learning; it never gates.
- **Provider endpoints and credentials in `plan.yaml`.** A change home is portable and archivable, which is exactly what deployment topology and secrets must not be.
- **A separate model-policy object with its own lifecycle.** Two pinned policies covering one epoch means two invalidation rules and an ordering question between them. The capability profile already pins how the model is asked to work.
- **Local cost estimation from a token price table.** A number that looks measured but is modelled will eventually be quoted to a client as measured. `unknown` is more useful than a plausible estimate.
- **A second identity for model calls.** The operation key already identifies the call uniquely within a change.

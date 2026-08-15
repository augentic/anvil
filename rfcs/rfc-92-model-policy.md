# RFC-92: Model Policy

> Status: Draft — model-economics track in the [Services Delivery Programme](platform.md). Startable on implemented [RFC-90](rfc-90-build-verification.md); parallel with [RFC-88](rfc-88-detached-changes.md).
>
> Owns: closed operation keys, per-operation routes and a minimal bounded escalation table on the pinned capability profile, deployment binding of route names to providers, the model-usage fact, and cost attribution.
>
> Amends RFC-88 D3 with a `routes` table when that cut lands; does not wait for it. Populates [RFC-103](rfc-103-outcome-learning.md)'s `cost` and supplies the routes its `model-route-change` proposals patch. Both otherwise unchanged.
>
> Evidence posture: [platform evaluation](platform.md#evidence-and-iteration-posture).

## Intent

*Make model selection a recorded policy and model spend an attributable fact.*

Every judgment leg currently draws whatever model the deployment provides. Three holes follow:

- **Selection is unrecorded.** Fast vs frontier on identical inputs produces different artifacts; the fact log cannot tell them apart.
- **Spend is unattributable.** Provider dashboards aggregate by API key and window, not by slice, requirement, or target.
- **RFC-103 has a consumer with no producer.** `model-route-change` and `cost` assume named, versioned routes.

Different operations have different value per token; the identity to attribute them already exists — typed operation, slice, phase ordinal, input digest. Only the record is missing.

## Terms

- An **operation key** is one member of the closed set of model-invoking operations below.
- A **route** is a named model binding: a model identity plus its reasoning-effort and context settings.
- A **route policy** selects one starting route and an optional ordered, bounded escalation ladder whose transitions are triggered only by closed engine-observed facts.
- A **route table** maps operation keys to route policies on a pinned capability profile. Its digest is part of the profile digest.
- A **route binding** is deployment-owned data mapping a route name to a concrete provider, endpoint, and credential.
- A **usage fact** is one journal event recording one completed model call's provider-reported consumption.

## Decisions

### D1 — Routes live on the pinned capability profile

RFC-88's model-capability profile already pins how the model is asked to work. The route table joins it rather than becoming a second policy object:

```yaml
# model-capability profile — additive
id: delivery-v3
digest: sha256:…
thresholds: { … }
routes:
  default: { start: balanced }
  engine.synthesize: { start: frontier }
  source.extract:
    start: economy
    escalate:
      - { on: answer-repair-exhausted, to: balanced }
  target.repair:
    start: economy
    escalate:
      - { on: unchanged-failure-set, to: balanced }
```

The closed operation-key set derives from the existing typed enums plus the engine judgment legs:

| Family | Keys |
| --------- | ------------------------------------------------------------------------------------------------ |
| Source | `source.survey`, `source.extract` |
| Target | `target.guidance`, `target.build`, `target.verify`, `target.repair`, `target.review`, `target.merge` |
| Engine | `engine.topology`, `engine.propose`, `engine.decompose`, `engine.synthesize`, `engine.boundary`, `engine.readiness` |

An absent key falls to `default`. The shorthand `target.review: frontier` canonicalizes to `{ start: frontier }` with no escalation. A key whose operation has no model leg in a given deployment — `target.verify` under RFC-97 host verification, for instance — is never consulted; a route policy is a binding, not a requirement to invoke. The set is closed per programme state, not forever: [RFC-106](rfc-106-task-graphs.md)'s `target.decompose` joins it when that RFC lands.

That closed key set is **model specialization by role**: survey, extract, synthesize, build, verify, and review may bind different tiers without an orchestrator-agent model ([platform.md § Design principles](platform.md#design-principles-at-the-call-site)). A starting-route or ladder change produces a new profile digest and invalidates the epoch on the same rule as a threshold change.

### D2 — Routes name capability tiers; deployment binds them to providers

The profile names `frontier`, `balanced`, `economy`. Deployment policy maps those names to a provider, endpoint, model identity, and credential, on the same shape as RFC-97's profile policies.

Provider topology never enters `plan.yaml`, so a change home stays portable and a client's gateway, self-hosted endpoint, or model-vendor contract remains a deployment concern. Credentials never come near an archived artifact. The usage fact records the resolved route name and the provider-reported model identity, so the fact log stays honest about what ran without the profile knowing where it ran.

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

`cost` is the provider-reported figure when the provider supplies one and `unknown` when it does not. It is never zero and never estimated from a token price table.

Usage facts are observations. No count, threshold, or total participates in status projection, gap gating, member admission, or a lifecycle transition. A model call that succeeded but exceeded a budget is a successful model call.

### D4 — Cost is a projection over the fact union

```text
emery cost --by slice | requirement | phase | route | target
```

Slice, phase, route, and target attribution is exact: each is already a field on the operation. Requirement attribution is derived through the slice's `model.yaml` requirement set and labelled as derived.

`plan archive` summarizes terminal spend beside the carried-debt summary. RFC-103 outcome records populate `cost` from the same union, so recurrence analysis can ask whether an economy route on `source.extract` cost more downstream in repair than it saved.

### D5 — Route policy is pinned; escalation stays minimal, closed, and engine-selected

A run never invents or edits a route policy. It starts each operation on the profile's selected route and may advance only through that policy's ordered ladder when a closed engine trigger is present.

The initial closed triggers are `answer-repair-exhausted`, `context-limit-refusal`, and RFC-97's `unchanged-failure-set`. Each names an engine fact or typed provider outcome; no model, adapter, prompt, readiness judgment, token total, elapsed-time threshold, or free-form classifier may select it. A trigger advances to the next declared route or stops when no declared step remains. Every dispatch records the route step and triggering fact. `unchanged-failure-set` has no producer until [RFC-97](rfc-97-native-verification.md) Phase A lands — declared, but it never fires before then.

Keep the ladder small: prefer a single starting route plus offline RFC-103 promotion over growing in-flight escalation. RFC-103 aggregates usage against escalation, repair, amendment, terminal outcome, and conservation, emits a `model-route-change` proposal, evaluates it blind, and promotes a new profile generation for future runs. Readiness may recommend a policy before authorization; it cannot trigger or redirect one in flight.

A route may separately name a provider-unavailability fallback, resolved from the pinned profile and recorded in the usage fact. Availability fallback does not consume an escalation step: it changes deployment binding, not capability intent.

## Implementation requirements

- Closed `OperationKey` enum from `SourceOperation` / `TargetOperation` plus the engine judgment legs, kebab-case wire ids.
- `routes` table and closed escalation triggers on the model-capability-profile DTO, folded into the existing profile digest; reject unknown keys, route names, triggers, non-advancing steps, and ladders above the compiled bound.
- Route binding at the deployment-provider layer beside RFC-97 profile policies. Engine crates carry no provider, endpoint, or credential constant.
- Thread the resolved route policy through existing judgment dispatch so every model call carries its operation key, slice, phase ordinal, route step, and closed trigger.
- `model.usage.recorded` in RFC-86's closed `EventKind` taxonomy.
- Read-only `emery cost` with four exact attributions and one labelled derived attribution.
- Populate RFC-103's `cost` block; extend its aggregation with route and operation key.
- Integration coverage: profile-digest invalidation on route or ladder change; absent-key fallback to `default`; unknown-route/trigger rejection; deterministic escalation; availability-fallback recording; no usage- or readiness-driven lifecycle effect.

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

- **Automatic per-task routing by a model, readiness score, spend signal, or opaque classifier inside the run.** The recorded authorization would no longer describe what executed. The permitted form is a minimal pinned ladder advanced only by closed engine facts; prefer offline promotion over expanding that ladder.
- **Growing escalation into a soft adaptive router.** Readiness-, latency-, token-, or classifier-driven steps recreate opaque mid-run routing under another name.
- **A single “orchestrator” route that plans and schedules.** Role specialization belongs on the closed operation-key set; a planning-authority route would recreate an agent orchestration layer, which [platform.md](platform.md#deliberately-rejected) rejects.
- **Cost as a lifecycle gate.** A budget that fails a merge turns spend into a correctness signal and creates pressure to accept unverified work to stay under a number.
- **Provider endpoints and credentials in `plan.yaml`.** A change home is portable and archivable; deployment topology and secrets must not be.
- **A separate model-policy object with its own lifecycle.** Two pinned policies covering one epoch means two invalidation rules and an ordering question. The capability profile already pins how the model is asked to work.
- **Local cost estimation from a token price table.** A number that looks measured but is modelled will eventually be quoted as measured. `unknown` is more useful than a plausible estimate.
- **A second identity for model calls.** The operation key already identifies the call uniquely within a change.

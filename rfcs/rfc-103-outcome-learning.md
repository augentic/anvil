# RFC-103: Outcome Learning

> Status: Draft — evidence and promotion track designed over implemented [RFC-90](rfc-90-build-verification.md), with its first useful delivery following [RFC-92](rfc-92-model-policy.md) usage facts and the existing blind eval harness; extended by [RFC-104](rfc-104-system-archaeology.md), [RFC-94](rfc-94-target-readiness.md), [RFC-97](rfc-97-native-verification.md), [RFC-98](rfc-98-behavioural-conservation.md), and [RFC-96](rfc-96-concurrent-execution.md). Parked [RFC-99](rfc-99-streaming-execution.md) may add progressive-execution dimensions if it is reopened. Owns outcome records, cross-run diagnostic aggregation, bounded advisory observations, offline policy/prompt/model proposals, blind evaluation, and versioned promotion. It does not change lifecycle, authority, or an in-flight run.
>
> Patch ownership: this RFC amends RFC-88 D1 after RFC-88 lands by adding `outcome.yaml` to the detached change root and `.emery/change/outcome.yaml` to in-place mode. RFC-104 separately amends RFC-88's handoff and supplies additional outcome dimensions.
>
> Producers this RFC assumes: [RFC-92](rfc-92-model-policy.md) defines the routes its `model-route-change` proposal patches and populates the `cost` block; the blind eval harness supplies promotion evidence. Those two are required for the first cut to learn rather than merely archive. RFC-104 supplies definition coverage, reviewed handoff and review-event identities, system-model, migration-plan, and wave identities; [RFC-94](rfc-94-target-readiness.md) supplies per-target execution-readiness bands; [RFC-97](rfc-97-native-verification.md) supplies execution-assurance and oracle-assurance facts; and [RFC-98](rfc-98-behavioural-conservation.md) supplies conservation coverage as aggregation dimensions.

## Intent

Make Emery improve from retained outcomes without giving a running model mutable memory or permission to rewrite its own prompts, budgets, policies, or acceptance gates.

Learning is an offline release loop:

```text
run facts and reports
    → outcome records
    → recurrence and effectiveness analysis
    → inert change proposals
    → blind evaluation
    → reviewed versioned release
    → future runs pin the new version
```

Self-healing inside one run remains bounded engine orchestration. Learning changes only future pinned inputs.

## Problem

RFC-90 retains every build phase and diagnostic, RFC-96 adds task and domain outcomes, RFC-97 adds host attestations, and the live harness records blind grades. Today those records answer what happened in one attempt but no contract:

- projects recurring diagnostic fingerprints across changes;
- measures which repair, decomposition, prompt, model, or policy revision resolved them;
- turns repeated estate-specific discoveries into bounded reusable guidance;
- promotes a successful change through blind evaluation into a version future runs can pin.

Without that loop, "self-healing" means retrying within an attempt. The organisation does not accumulate governed knowledge.

## Principles

1. **Facts, not chat memory.** Learning consumes immutable records and emits immutable proposals.
2. **Future runs only.** A run never changes the prompt, model, adapter, policy, or budget it started with.
3. **Evidence below authority.** Learned observations may advise judgment but cannot override Emery artifacts, source authority, protected oracles, or lifecycle gates.
4. **Blind evaluation stays outside workflow authority.** A held-out grade can approve a release but cannot retroactively pass a production run.
5. **No silent tenant mixing.** Source-derived bodies and identifiers remain within their declared product and tenancy scope.
6. **Promotion is explicit.** Runtime recurrence may create a proposal; only a reviewed release act creates a version eligible for future runs.

## Terms

- An **outcome record** is a content-addressed projection of one terminal change or retained candidate run.
- A **repair trajectory** is the ordered finding, repair origin, candidate, and verification sequence for one attempt lineage.
- An **observation** is a small, immutable, scoped advisory statement supported by outcome references.
- A **learning proposal** is an inert candidate change to an adapter prompt, engineering rule, run policy, engine constant, model route, or training corpus.
- A **promotion set** is the proposal, evaluation cases, blind results, compatibility floor, and exact released version considered together.
- A **policy generation** is the immutable version of a run or autonomy policy selected by a future invocation.

## Outcome records

`plan archive` projects `outcome.yaml` at the RFC-88 change root before moving the change into archive (`.emery/change/outcome.yaml` in the in-place deployment). The record contains identities and summaries, not copied source bodies:

```yaml
version: 1
change: sha256:…
inputs:
  plan: sha256:…
  definition:
    handoff: sha256:…
    review-event: sha256:…
    system-model: sha256:…
    migration-plan: sha256:…
    wave-id: extract-orders
  adapters:
    omnia: emery:omnia@1.4.0
  prompts:
    synthesis: sha256:…
  models:
    project: cursor:…
  policies:
    run: sha256:…
results:
  accepted-targets:
    orders: sha256:…
  definition:
    coverage:
      included: 12
      inaccessible: 1
      unresolved: 0
    architecture-amendments: 2
    assumptions-invalidated: 1
  requirements:
    agreed: 41
    divergence: 3
    unknown: 0
    conflict: 0
diagnostics:
  - fingerprint: sha256:…
    occurrences: 3
    terminal: false
    resolved-by: verification-repair
assurance:
  execution-assurance:
    model-assisted: 4
    host-attested: 2
    hybrid: 0
  oracle:
    candidate: 4
    protected: 2
    mixed: 0
cost:
  model-usage: unknown
```

The DTO rejects unknown fields. Optional provider cost remains `unknown`, never zero.

The projection may reference:

- refinement, build, domain, merge, and publication records;
- definition coverage, architecture revisions, selected migration wave, and invalidated assumptions;
- canonical diagnostic fingerprints and blocking state;
- repair count and origin;
- graph reuse and re-decomposition;
- candidate invalidation and speculative discard;
- host profile attestations and assurance;
- model, adapter, prompt, rule, policy, and engine versions;
- wall-clock phase timing and provider-reported usage.

It never includes credentials, raw prompts, continuations, private source bodies, workspace paths, or blind acceptance material.

## Aggregation

The learning runner reads outcome records from an explicitly selected scope:

```text
emery outcomes analyze --scope project|product|adapter --since <revision>
```

Analysis is deterministic over the selected record set. It groups by stable diagnostic fingerprint and closed dimensions such as:

- source adapter and claim kind;
- system element, relationship, modernization disposition, and migration-wave kind;
- target, platform, and verification profile;
- prompt, model, policy, and adapter version;
- repair origin and round;
- task and domain depth;
- protected-oracle assurance;
- terminal result.

Free-form source text is not an aggregation key. A fingerprint group below the configured minimum remains report-only and cannot create a proposal.

## Advisory observations

Repeated estate-specific findings may produce a proposed observation:

```yaml
version: 1
scope:
  product: orders
statement: "The legacy order id is case-sensitive at the gateway boundary."
evidence:
  - outcome: sha256:…
    finding: sha256:…
expires-after: 2027-01-01
```

Observations are:

- at most five lines of statement and rationale;
- scoped to one product, project, adapter, or target/profile tuple;
- digest-bound and expiry-bounded;
- advisory below artifacts and Source Evidence;
- included in an operation key whenever consumed.

They do not enter mutable ambient memory. Promotion into durable product `decisions/` or adapter references is a separate operator act.

## Learning proposals

The analyzer may emit one of these closed proposal kinds:

- `prompt-patch`;
- `engineering-rule-patch`;
- `run-policy-patch`;
- `engine-budget-change`;
- `model-route-change`;
- `training-example-set`;
- `observation`;
- `decision-owner`;
- `planning-amendment-pattern`.

Every proposal names:

- supporting and contradicting outcome records;
- the exact current and proposed input digests;
- expected improvement and possible regression dimensions;
- required evaluation cases;
- expiry if not evaluated;
- source and tenancy scope.

A proposal cannot edit the repository, policy store, adapter component, plan, or active run. It is a review artifact.

A `decision-owner` proposal is available only when retained diagnostics show the same cross-domain semantic decision diverging in more than one change. It names one existing product domain or `decisions/` path as owner, the exact affected dependant scopes, and supporting and contradicting outcomes. Promotion remains an operator-authored product decision; the analyzer cannot invent durable intent or rewrite dependencies.

## Evaluation and promotion

Promotion runs the current and candidate versions against:

- deterministic integration fixtures;
- retained replay cases whose inputs are legally reusable;
- a blind acceptance set unavailable to every workflow model call;
- cap-one and concurrent scheduler variants where ordering is relevant.

The report separates:

- accepted outcome, execution assurance, and oracle assurance;
- first-pass success and repair trajectory;
- unresolved-finding recurrence;
- latency and provider-reported cost;
- speculative discard and amendment rate;
- generated structure as coordination evidence, not quality.

No single aggregate score hides a regression. Required dimensions and non-regression thresholds belong to the proposal kind and policy.

Promotion creates a versioned release:

- adapter prompt or engineering-rule changes ship in a new adapter version;
- engine constants ship in a new Emery version;
- run policies receive a new policy generation;
- model routes receive a new deployment-policy generation;
- training examples enter a versioned corpus consumed by [RFC-18](future/rfc-18-slm.md).

Future runs record the selected version. Existing runs remain pinned.

## Relation to automatic execution

[RFC-99](rfc-99-streaming-execution.md) records progressive-run outcomes and policy identity. [RFC-102](rfc-102-policy-gated-autonomy.md) may select only promoted policy generations.

Outcome learning never authorizes build or merge. A good historical success rate cannot:

- clear or exclude a current unknown or conflict;
- replace exact member admission;
- lower oracle-assurance or host-execution requirements;
- increase a retry budget at runtime;
- accept stale work;
- turn a model review into host-attested execution.

## Retention and privacy

Outcome records follow archive retention. Aggregate datasets retain record digests and closed summaries after source-specific records expire only when policy permits.

Cross-product or cross-tenant aggregation is denied by default. Adapter-wide learning requires an explicit export that:

- removes project and source identifiers;
- proves the source licence and data policy permit reuse;
- retains enough version, execution-assurance, and oracle-assurance identity for evaluation;
- is reviewed before entering a shared corpus.

Deletion removes the record from future datasets and corpus builds. Released binaries or models follow their own provenance and revocation policy.

## Implementation requirements

- Add the closed outcome DTO and deterministic projection from existing facts and records.
- Add `emery outcomes {show, analyze, propose}` as read-only or inert-proposal operations.
- Reuse diagnostic fingerprints; add no second finding identity.
- Add closed learning-proposal DTOs and atomic persistence.
- Add scoped, expiry-bounded observation records and operation-key inclusion.
- Extend `probe` to compare current and candidate versions while preserving blind-input isolation.
- Record promotion sets and released adapter, engine, policy, route, or corpus versions.
- Keep aggregation and evaluation outside lifecycle projection and the engine guest's active run.

## Acceptance criteria

1. Two byte-identical archives project byte-identical outcome records independent of event arrival order.
2. A repeated diagnostic fingerprint reports recurrence and repair effectiveness without exposing source bodies.
3. One isolated occurrence cannot create a learning proposal below the configured minimum.
4. A proposal cannot mutate an adapter, policy, engine constant, plan, or active run.
5. Blind evaluation material is unavailable to every planning, refinement, build, repair, review, and verification model call.
6. A promoted policy, adapter, prompt, route, or corpus receives a new immutable version; an active run remains on its original version.
7. An observation is scope-, digest-, line-, and expiry-bounded and enters every operation key that consumes it.
8. Removing an eligible outcome from the selected dataset changes the analysis digest and prevents stale promotion.
9. Cross-tenant aggregation fails without an explicit policy-approved export.
10. Existing RFC-90 repair budgets, RFC-96 scheduling, RFC-99 admission, and RFC-97 verification remain runtime authority.

## Rejected alternatives

- **Mutable agent memory.** It is neither input-fenced nor reviewable and makes replay depend on hidden history.
- **Let a successful run rewrite its own prompt.** One outcome is weak evidence, and in-run mutation destroys the meaning of its recorded input identity.
- **Use blind grading as production verification.** Hidden evaluation protects the learning harness; it is not customer workflow authority.
- **One scalar reward for every proposal.** Build correctness, requirement coverage, execution source, oracle assurance, latency, and cost have different failure meanings.
- **Copy source bodies into a global corpus by default.** Product and tenancy boundaries remain authoritative.
- **Automatically promote recurring observations into decisions.** Observations stay advisory until an operator accepts durable product intent.

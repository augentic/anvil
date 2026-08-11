# RFC-98: Policy-Gated Autonomy

> Status: Draft — autonomy follow-on to [RFC-94](future/rfc-94-streaming-execution.md) Phase B, [RFC-95](rfc-95-native-verification.md), and [RFC-97](rfc-97-outcome-learning.md). Owns unattended accepted-CID mutation, promoted autonomy policies, exact commit admission, bounded structural recovery, and stop conditions. Publication remains operator-owned under [RFC-89](rfc-89-publication-sets.md).
>
> Patch ownership: this RFC amends RFC-88 D7 / D8 after RFC-88 lands by adding policy-gated commit authorization beside closed `plan execute`; it does not revise RFC-88. It extends RFC-89's seal gate without granting forge publication.

## Intent

Allow an operator to authorize a bounded policy once and let Emery run:

```text
author → refine → build → verify/repair/review → merge
```

without human pauses when every exact artifact, gap, protected oracle, native verification profile, dependency frontier, and recovery action stays inside that policy.

Autonomy means policy-gated progression, not model authority. The model cannot choose its policy, waive a gap, broaden scope, lower assurance, reset a budget, or decide that its own output may merge.

## Prerequisite boundary

RFC-94 may run unattended only through `built`. Its candidate frontiers are non-authoritative.

This RFC extends the runner:

```text
emery plan run --publication progressive --through merged --policy <profile>
```

The command may progressively author, refine, and build before final closure. Entering its first commit-capable phase is unavailable unless the deployment resolves:

- a promoted autonomy-policy generation;
- every required RFC-95 host-verification profile;
- every protected input and oracle required by that policy;
- the RFC-88 final planning closure and current accepted target state.

Missing capability fails closed. There is no fallback to model-only merge when policy requires stronger assurance.

Single-node autonomy does not wait on distribution. Multi-node or multi-tenant autonomy additionally requires RFC-93 distributed execution and RFC-96 deployment, policy-registry, secret, and tenancy conformance.

## Terms

- An **autonomy policy** is a closed, versioned, deployment-owned profile governing one unattended run class.
- A **risk class** is one of `low | moderate | high | critical`; it selects one closed assurance and recovery profile for a target or slice.
- A **commit admission** is the exact fact authorizing one closed target wave to mutate the accepted CID.
- A **standing amendment rule** is a narrow, pre-authorized deterministic predicate over one inert RFC-88/RFC-92 amendment proposal.
- A **recovery ladder** is the ordered bounded set of actions the engine may try before stopping.
- A **policy generation** is a promoted RFC-97 version; changing it starts a new run grant.

## Autonomy policy

The policy is selected by name but recorded by digest:

```yaml
version: 1
name: protected-migration
scope:
  products: [orders]
  targets: [orders-api]
  default-risk: high
  target-risk:
    orders-api: high
admission:
  gaps: agreed-or-divergence
  require-final-closure: true
risk-classes:
  high:
    assurance:
      minimum: mixed
      profiles: [build, test, clippy]
      required-oracles: [legacy-replays]
    budgets:
      verification-repairs: 3
      review-remediations: 1
      amendment-applications: 1
amendments:
  allow: [boundary-split]
publication:
  seal: true
  forge: false
```

Closed enums and engine maxima bound every field. A policy may lower a compiled budget but cannot exceed it.

Every referenced risk class must have one profile. The compiler selects the exact class from the slice override, target override, or default, records it on member and commit admission, and rejects an unclassified slice. A standing amendment cannot change the selected class.

Projects, source artifacts, adapters, prompts, and models cannot write or redirect the policy registry. Deployment policy resolves the profile before the engine starts and returns its immutable generation and digest.

The policy resolver has closed `run | autonomy` kinds. RFC-94 `--through built` accepts a run policy. `--through merged` requires an autonomy policy promoted through RFC-97; the same CLI flag cannot silently upgrade one kind to the other.

## Admission rules

### Pre-commit planning and refinement gate

Before the first commit-capable phase, unattended progression requires:

- final RFC-88 closure with no open or superseded branch in the leaf projection;
- exact source, target, adapter, model, and profile identities within policy scope;
- fresh RFC-91 refinement manifests for every in-scope leaf;
- no `[conflict]` or `[unknown]`;
- every divergence resolved by deterministic authority with complete provenance;
- protected inputs and oracles fixed by the covered decomposition revision.

No autonomy policy can waive unknowns or conflicts. A policy may require `agreed` only.

### Build

Build admission reuses RFC-94 member admission and candidate frontiers. Every result remains non-authoritative until commit admission.

The selected risk class's minimum assurance is one of:

- `model-assisted` — candidate self-consistency only;
- `protected` — at least one read-only protected input or oracle contributed to every required check;
- `host-attested` — every required RFC-95 profile produced a valid host attestation;
- `mixed` — the closed required combination.

Operator output and facts carry the actual assurance. A stronger label is never inferred from policy intent.

### Commit

Before each target-wave merge, the engine atomically writes commit admission covering:

- final plan and decomposition digests;
- every member branch and refinement-manifest digest;
- build, domain, candidate-batch, and candidate-frontier records;
- current accepted target and dependency frontier;
- protected input and oracle closure;
- advisory observation-set digest, empty when none was consumed;
- required host profile attestations;
- terminal build and review reports;
- autonomy-policy generation and selected risk classes;
- complete recovery history;
- target-wave membership and proposed result CID.

The journal fact is `target.wave.commit-admitted`. Any changed input makes it stale before merge preflight. Claims, successful builds, plan closure, or historical success rates cannot substitute for commit admission.

Target-wave merge and postflight retain RFC-88/RFC-90 semantics. A postflight failure remains non-rollback and stops the autonomous run for operator acknowledgement.

A blocking RFC-92 target-domain complete round after an accepted wave is also a hard stop. The autonomous run cannot repair or amend already accepted state under the same grant.

## Recovery ladder

The engine attempts only the compiled sequence:

1. bounded judgment-answer repair;
2. RFC-90 verification repair and review remediation;
3. one validated task-graph replacement for an RFC-92 graph-attributable failure;
4. one standing-rule amendment application when explicitly allowed, followed by ordinary refinement and build of the new planning revision;
5. stop with retained evidence.

Each rung has a separate counter. Success at one rung does not reset another.

A policy may lower RFC-90's verification and review budgets and standing amendment applications. Judgment-answer repair remains the fixed engine `MAX_REPAIRS`; graph replacement remains RFC-92's fixed two-round limit. Neither is policy-increasable.

A retry always creates a new attempt or revision with a new input digest. The engine never edits a successful record in place.

## Standing amendments

RFC-88 and RFC-92 amendment proposals remain inert by default. A standing rule may apply a proposal automatically only when deterministic validation proves:

- the proposal kind is explicitly allowed;
- source, target, product, and protected-oracle scope do not expand;
- no adapter, model profile, authority override, gap waiver, or risk class changes;
- every new leaf remains inside the original lead and ownership envelope;
- decomposition depth, node, and application budgets remain;
- the old planning revision has not changed since proposal creation.

The initial closed allowed kind is `boundary-split`: replace one incoherent leaf with independently acceptable children supported by focused child leads.

A standing rule is eligible only before the first `target.wave.commit-admitted` fact. If it applies after a prior final closure, the new planning revision reopens authoring, invalidates affected member admissions and candidate lineages, and must reach final closure again before any commit-capable phase. Once commit admission or accepted-state mutation begins, structural recovery is a hard stop for operator action.

These proposal kinds are permanently operator-only:

- source or target addition;
- authority override;
- unknown waiver;
- protected-input or oracle removal;
- model, adapter, profile, or policy change;
- ownership-envelope widening;
- target-wave membership edit after commit admission;
- publication or forge action.

An automatic application writes the same compare-and-set amendment fact as an operator application plus the standing-rule and policy digests.

## Behavioural assurance

Legacy migrations should prefer externally grounded checks. A policy may require:

- capture replay digests projected from `kind: example` Evidence;
- imported API or schema contracts;
- baseline tests mounted read-only;
- target-specific RFC-95 `build`, `test`, static-analysis, or platform-build profiles.

The build agent may see protected inputs needed for repair, but cannot change them. Blind acceptance sets remain evaluation-only and unavailable to the production run.

Passing candidate-authored tests alone is model-assisted assurance and cannot satisfy a protected or host-attested policy.

## Policy learning and promotion

RFC-97 may propose a new autonomy-policy generation from outcome records. Promotion requires:

- deterministic schema and scope validation;
- current-versus-candidate integration and replay runs;
- blind non-regression evaluation;
- explicit review and release;
- a new immutable generation.

The active run never adopts the candidate generation. A historical success rate cannot automatically lower assurance or broaden standing amendments.

Emergency revocation prevents new runs from selecting a policy generation. It does not rewrite facts or hide already accepted changes.

## Completion

An autonomous run projects success only when:

- final planning closure remains current;
- every in-scope leaf is merged or legally excluded;
- every target root domain passes its complete round;
- every commit admission and merge fact remains current;
- required project seals exist;
- no postflight acknowledgement or policy stop remains.

RFC-89 forge publication remains a separate operator act. This RFC may create local seals but cannot push branches, open pull requests, merge pull requests, or delete the change home.

## Failure and restart

- Re-entry uses the same policy generation and reconstructs counters from facts.
- An exhausted rung parks the smallest affected domain and its dependants.
- Independent domains may continue only when their commit admissions share no stale dependency or target frontier.
- Policy-registry unavailability stops before new admission; already recorded valid operations may finish but cannot merge without a resolvable current policy generation.
- Revocation, source drift, accepted-target drift, or protected-oracle drift invalidates pending commit admission.
- Operator intervention starts a new run grant when it changes policy-covered input.

## Implementation requirements

- Extend RFC-94 `plan run` with `--through merged` only when an autonomy policy resolves.
- Add the closed autonomy-policy DTO, provider capability, generation identity, and fail-closed resolver.
- Add exact `target.wave.commit-admitted` facts and preflight revalidation.
- Add recovery-ladder counters and typed terminal reasons.
- Add standing amendment rules over the existing compare-and-set amendment kernel.
- Project capture replay and contract Evidence into protected-oracle requirements without changing Evidence authority.
- Resolve and enforce RFC-95 host profiles before commit admission.
- Emit RFC-97 outcome fields for admissions, recovery, assurance, and policy generation.
- Keep forge publication, policy promotion, and policy registry mutation outside the engine run.

## Acceptance criteria

1. A clean fixture reaches merged from one `plan run` gesture with no human pause, no gap waiver, and exact policy, member, and commit admissions.
2. The same fixture under cap one and cap four produces the same accepted target CIDs and requirement identities.
3. A conflict, unknown, missing protected oracle, missing host profile, stale attestation, or weaker actual assurance stops before commit admission.
4. A three-leaf dependency chain builds through RFC-94 candidate frontiers and commits in topological waves against current accepted CIDs.
5. A permitted boundary split applies once by compare-and-set; scope widening or a second application stops.
6. An authority override, unknown waiver, protected-input removal, adapter/model/policy change, or forge action can never be standing-authorized.
7. Exhausting any recovery budget stops; success elsewhere never resets its counter.
8. Changing one admitted digest, including the observation set, invalidates exactly the affected wave and dependant admissions.
9. Policy generation changes and revocation affect new admission without rewriting historical facts.
10. Candidate-authored tests cannot satisfy protected or host-attested assurance.
11. Postflight failure remains non-rollback and requires operator acknowledgement.
12. A blocking post-acceptance target complete round stops without autonomous repair or amendment under the same grant.
13. Publication remains operator-owned, and all repository quality gates and blind autonomy fixtures pass.

## Rejected alternatives

- **Treat unattended invocation as blanket approval.** The policy constrains scope; exact member and commit admissions constrain artifacts and results.
- **Auto-waive gaps.** Missing or conflicting intent is not a repairable implementation defect.
- **Let a model choose recovery actions.** Models return typed judgments and reports; the engine owns the ladder and counters.
- **Apply every valid amendment automatically.** Structural validity does not imply policy authority.
- **Use historical success as current assurance.** Every commit proves its own exact protected and host checks.
- **Publish to the forge automatically.** Accepted local state and external publication retain separate authority and recovery domains.
- **Mutate policy from runtime learning.** RFC-97 promotion creates a future generation; active runs remain pinned.

# RFC-102: Policy-Gated Autonomy

> Status: **Parked.** Reopen only after RFC-99 Phase B, RFC-97 Phase B, RFC-103 outcome promotion, RFC-94 target execution readiness, and RFC-93 operator grants have landed **and** a client engagement requires unattended accepted-state mutation. Owns promoted autonomy policies, exact commit admission, bounded structural recovery, and stop conditions. Publication remains operator-owned under [RFC-95](rfc-95-publication-sets.md).
>
> Patch ownership: this RFC amends RFC-88 D7 / D8 after RFC-88 lands by adding policy-gated commit authorization beside closed `plan execute`; it does not revise RFC-88. It extends RFC-95's worktree-materialize gate without granting a Git commit or forge publication.

## Intent

Allow an operator to authorize a bounded policy once and let Emery run:

```text
author → refine → build → verify/repair/review → merge
```

without human pauses when every exact artifact, gap, protected oracle, native verification profile, dependency frontier, and recovery action stays inside that policy.

Autonomy means policy-gated progression, not model authority. The model cannot choose its policy, waive a gap, broaden scope, lower assurance, reset a budget, or decide that its own output may merge.

### Scope: engine autonomy and caller authority are separate policies

The autonomy policy governs what the **engine** may do inside one unattended run. [RFC-93](rfc-93-operator-boundary.md) separately governs which CLI acts the caller may dispatch. The distinction matters because the operator need not be a person: nothing in the CLI requires one, and an autonomous driver can issue `plan drop`, `plan amend --authority-override`, or `--force` exactly as a human can.

Those verbs remain out of **autonomy-policy** scope: result admission must not vary merely because a human or agent invoked it. They are not therefore universally authorized. RFC-93 resolves and enforces the caller's operator grant before guest dispatch. This RFC consumes the resulting actor and grant records when compiling commit admission.

An agent may drive the engine; an agent may not be the engine, and it may not extend its RFC-93 grant by driving harder.

## Prerequisite boundary

RFC-99 may run unattended only through `built`. Its candidate frontiers are non-authoritative.

This RFC extends the runner:

```text
emery plan run --publication progressive --through merged --policy <profile>
```

The command may progressively author, refine, and build before final closure. Entering its first commit-capable phase is unavailable unless the deployment resolves:

- a promoted autonomy-policy generation;
- a current operator grant admitting this invocation and its product/target scope;
- an RFC-94 `unattended` readiness band for every written target, with current authority digests;
- every required RFC-97 host-verification profile;
- every protected input and oracle required by that policy;
- the RFC-88 final planning closure and current accepted target state.

Missing capability fails closed. There is no fallback to model-only merge when policy requires stronger assurance.

Single-node autonomy does not wait on distribution. Multi-node or multi-tenant autonomy additionally requires RFC-100 distributed execution and RFC-101 deployment, policy-registry, secret, and tenancy conformance.

## Terms

- An **autonomy policy** is a closed, versioned, deployment-owned profile governing one unattended run class.
- An **operator grant** is RFC-93's deployment-owned caller capability, consumed here by identity and digest.
- A **policy bundle** is the immutable deployment resolution containing the applicable operator grant, autonomy policy, model routes, readiness profile, verification profiles, corpus governance, and egress/secret policy plus their individual and aggregate digests.
- A **risk class** is one of `low | moderate | high | critical`; it selects one closed assurance and recovery profile for a target or slice.
- A **commit admission** is the exact fact authorizing one closed target wave to mutate the accepted CID.
- A **standing amendment rule** is a narrow, pre-authorized deterministic predicate over one inert RFC-88/RFC-96 amendment proposal.
- A **recovery ladder** is the ordered bounded set of actions the engine may try before stopping.
- A **policy generation** is a promoted RFC-103 version; changing it starts a new run grant.

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
      execution-assurance: host-attested
      oracle-assurance: protected
      profiles: [build, test, clippy]
      required-oracles: [legacy-replays]
    budgets:
      verification-repairs: 3
      review-remediations: 1
      amendment-applications: 1
amendments:
  allow: [boundary-split]
publication:
  materialize: true
  forge: false
```

Closed enums and engine maxima bound every field. A policy may lower a compiled budget but cannot exceed it.

Every referenced risk class must have one profile. The compiler selects the exact class from the slice override, target override, or default, records it on member and commit admission, and rejects an unclassified slice. A standing amendment cannot change the selected class.

Projects, source artifacts, adapters, prompts, and models cannot write or redirect the policy registry. Deployment policy resolves one immutable policy bundle before the engine starts and returns the typed policies the invocation needs plus their individual and aggregate digests. Organization or deployment minimums may be strengthened but never weakened by a project or invocation. Missing, malformed, contradictory, expired, or unverifiable policy fails closed.

RFC-93 owns the desktop `local-owner` binding and restricted ingress. A regulated or multi-tenant autonomy profile requires host-attested identity and a deployment-issued grant; a self-declared actor label can never satisfy that requirement.

The policy resolver has closed `run | autonomy` kinds. RFC-99 `--through built` accepts a run policy. `--through merged` requires an autonomy policy promoted through RFC-103; the same CLI flag cannot silently upgrade one kind to the other.

## Admission rules

### Pre-commit planning and refinement gate

Before the first commit-capable phase, unattended progression requires:

- final RFC-88 closure with no open or superseded branch in the leaf projection;
- exact source, target, adapter, model, and profile identities within policy scope;
- current RFC-94 target-execution-readiness authority digests whose bands admit the selected autonomy policy;
- fresh RFC-91 refinement manifests for every in-scope leaf;
- no `[conflict]` or `[unknown]`;
- every divergence resolved by deterministic authority with complete provenance;
- protected inputs and oracles fixed by the covered decomposition revision.

No autonomy policy can waive unknowns or conflicts. Reviewed execution may have auto-deferred an open row at its build gate; that durable disposition preserves debt but does not change the requirement's `[unknown]` or `[conflict]` status and therefore remains ineligible for autonomous commit. A policy may require `agreed` only. A future policy kind that excludes a requirement from autonomous scope must record the exact requirement digest and carried-debt consequence; silence never counts as exclusion.

### Build

Build admission reuses RFC-99 member admission and candidate frontiers. Every result remains non-authoritative until commit admission.

The selected risk class states two orthogonal minima:

- `execution-assurance: host-attested | hybrid` — every required profile produced a valid RFC-97 host attestation; `hybrid` additionally requires the declared deterministic in-component contribution;
- `oracle-assurance: candidate | protected | mixed` — whether checks consumed only candidate-writable inputs, at least one read-only protected input or oracle, or the policy's declared combination.

`execution-assurance: model-assisted` may produce an RFC-99 non-authoritative candidate but can never satisfy autonomous commit admission. A green host-attested run over candidate-authored tests is `execution-assurance: host-attested` and `oracle-assurance: candidate`; host execution does not manufacture oracle independence. Operator output and facts carry both actual axes. A stronger label is never inferred from policy intent.

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
- per-target readiness authority digests and bands;
- operator-grant identity, actor attestation, and policy-bundle digest;
- complete recovery history;
- target-wave membership and proposed result CID.

The journal fact is `target.wave.commit-admitted`. Any changed input makes it stale before merge preflight. Claims, successful builds, plan closure, or historical success rates cannot substitute for commit admission.

Target-wave merge and postflight retain RFC-88/RFC-90 semantics. A postflight failure remains non-rollback and stops the autonomous run for operator acknowledgement.

A blocking RFC-96 target-domain complete round after an accepted wave is also a hard stop. The autonomous run cannot repair or amend already accepted state under the same grant.

## Recovery ladder

The engine attempts only the compiled sequence:

1. bounded judgment-answer repair;
2. RFC-90 verification repair and review remediation;
3. one validated task-graph replacement for an RFC-96 graph-attributable failure;
4. one standing-rule amendment application when explicitly allowed, followed by ordinary refinement and build of the new planning revision;
5. stop with retained evidence.

Each rung has a separate counter. Success at one rung does not reset another.

A policy may lower RFC-90's verification and review budgets and standing amendment applications. Judgment-answer repair remains the fixed engine `MAX_REPAIRS`; graph replacement remains RFC-96's fixed two-round limit. Neither is policy-increasable.

A retry always creates a new attempt or revision with a new input digest. The engine never edits a successful record in place.

## Standing amendments

RFC-88 and RFC-96 amendment proposals remain inert by default. A standing rule may apply a proposal automatically only when deterministic validation proves:

- the proposal kind is explicitly allowed;
- source, target, product, and protected-oracle scope do not expand;
- no adapter, model profile, authority override, open-gap status, or risk class changes;
- every new leaf remains inside the original lead and ownership envelope;
- decomposition depth, node, and application budgets remain;
- the old planning revision has not changed since proposal creation.

The initial closed allowed kind is `boundary-split`: replace one incoherent leaf with independently acceptable children supported by focused child leads.

A standing rule is eligible only before the first `target.wave.commit-admitted` fact. If it applies after a prior final closure, the new planning revision reopens authoring, invalidates affected member admissions and candidate lineages, and must reach final closure again before any commit-capable phase. Once commit admission or accepted-state mutation begins, structural recovery is a hard stop for operator action.

These proposal kinds are permanently operator-only:

- source or target addition;
- authority override;
- requirement amendment that clears or excludes an open unknown or conflict;
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
- target-specific RFC-97 `build`, `test`, static-analysis, or platform-build profiles.

The build agent may see protected inputs needed for repair, but cannot change them. Blind acceptance sets remain evaluation-only and unavailable to the production run.

Passing candidate-authored tests under a host profile is host-attested execution with candidate oracle assurance. It may satisfy a greenfield policy that explicitly permits `candidate`, but a modernization policy requiring protected conservation cannot be satisfied without its admitted oracle. Model-assisted execution alone never satisfies autonomous commit.

## Policy learning and promotion

RFC-103 may propose a new autonomy-policy generation from outcome records. Promotion requires:

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
- required publication worktrees exist;
- no postflight acknowledgement or policy stop remains.

RFC-95 forge publication remains a separate operator act. This RFC may authorize publication-worktree materialize but cannot author a Git commit, push branches, open pull requests, merge pull requests, or delete the change home.

## Failure and restart

- Re-entry uses the same policy generation and reconstructs counters from facts.
- An exhausted rung parks the smallest affected domain and its dependants.
- Independent domains may continue only when their commit admissions share no stale dependency or target frontier.
- Policy-registry unavailability stops before new admission; already recorded valid operations may finish but cannot merge without a resolvable current policy generation.
- Revocation, source drift, accepted-target drift, or protected-oracle drift invalidates pending commit admission.
- Operator intervention starts a new run grant when it changes policy-covered input.

## Implementation requirements

- Extend RFC-99 `plan run` with `--through merged` only when an autonomy policy resolves.
- Add the closed autonomy-policy DTO, provider capability, generation identity, and fail-closed resolver.
- Consume RFC-93's actor and operator-grant records when resolving the immutable policy bundle; this RFC adds no second caller-authorization path.
- Add exact `target.wave.commit-admitted` facts and preflight revalidation, including current RFC-94 target-execution-readiness authority per written target.
- Add recovery-ladder counters and typed terminal reasons.
- Add standing amendment rules over the existing compare-and-set amendment kernel.
- Project capture replay and contract Evidence into protected-oracle requirements without changing Evidence authority.
- Resolve and enforce RFC-97 host profiles with separate execution-assurance and oracle-assurance minima before commit admission.
- Emit RFC-103 outcome fields for admissions, recovery, assurance, and policy generation.
- Keep forge publication, policy promotion, and policy registry mutation outside the engine run.

## Acceptance criteria

1. A clean fixture reaches merged from one `plan run` gesture with no human pause and exact operator grant, policy bundle, member, and commit admissions.
2. The same fixture under cap one and cap four produces the same accepted target CIDs and requirement identities.
3. A conflict, unknown, auto-deferred open row, below-`unattended` or stale readiness assessment, missing protected oracle, missing host profile, stale attestation, model-only verification, or weaker actual assurance on either axis stops before commit admission.
4. A three-leaf dependency chain builds through RFC-99 candidate frontiers and commits in topological waves against current accepted CIDs.
5. A permitted boundary split applies once by compare-and-set; scope widening or a second application stops.
6. An authority override, open-requirement status change, protected-input removal, adapter/model/policy change, or forge action can never be standing-authorized.
7. Exhausting any recovery budget stops; success elsewhere never resets its counter.
8. Changing one admitted digest, including the observation set, invalidates exactly the affected wave and dependant admissions.
9. Policy generation changes and revocation affect new admission without rewriting historical facts.
10. Candidate-authored tests run by the host report host-attested execution and candidate oracle assurance; they cannot satisfy protected oracle assurance, while model-assisted execution cannot satisfy any autonomous commit.
11. Postflight failure remains non-rollback and requires operator acknowledgement.
12. A blocking post-acceptance target complete round stops without autonomous repair or amendment under the same grant.
13. Publication remains operator-owned, and all repository quality gates and blind autonomy fixtures pass.

## Rejected alternatives

- **Treat unattended invocation as blanket approval.** The policy constrains scope; exact member and commit admissions constrain artifacts and results.
- **Auto-waive gaps.** Missing or conflicting intent is not a repairable implementation defect.
- **Let a model choose recovery actions.** Models return typed judgments and reports; the engine owns the ladder and counters.
- **Invent free-form fix work from validation gaps in model context.** Under this RFC the only allowed forms of “fix work invented from findings” are the engine-owned recovery ladder and standing amendments over inert, digest-bound proposals — never mid-run conversational re-plan that widens scope, resets budgets, or invents topology outside those predicates. See [platform.md § Design principles](platform.md#design-principles-at-the-call-site).
- **Apply every valid amendment automatically.** Structural validity does not imply policy authority.
- **Use historical success as current assurance.** Every commit proves its own exact protected and host checks.
- **Publish to the forge automatically.** Accepted local state and external publication retain separate authority and recovery domains.
- **Mutate policy from runtime learning.** RFC-103 promotion creates a future generation; active runs remain pinned.
- **Put caller authorization inside the engine lifecycle or autonomy policy.** Caller authority belongs at the host dispatch boundary and composes with RFC-93 attribution. The guest still gates artifacts and results identically for every actor; it gains no approval rung, countersign transition, or per-epoch waiver surface.

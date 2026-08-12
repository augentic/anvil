# RFC-94: Target Readiness Profiles

> Status: Draft — engine-side readiness follow-on outside the platform-migration critical path
>
> Owns: the closed target-readiness dimension set, the versioned readiness profile, discovery-time assessment and its record, the readiness band, execution-policy eligibility, and the remediation projection.
>
> Depends on [RFC-88](rfc-88-detached-changes.md) for discovery, pinned target CIDs, and the model-capability-profile shape this RFC mirrors. Consumed by [RFC-97](rfc-97-native-verification.md) preflight, [RFC-98](rfc-98-behavioural-conservation.md) corpus expectations, and [RFC-102](rfc-102-policy-gated-autonomy.md) admission.
>
> Patch ownership: this RFC amends RFC-88 D5 by adding a readiness record to each `discovery.yaml` target row and RFC-88 D8 by adding the readiness digest to closed-plan coverage. RFC-88 remains unchanged.
>
> Evidence posture: [platform evaluation](platform.md#evidence-and-iteration-posture).

## Intent

*Know before authorization whether a target can support the loop Emery is about to run on it.*

RFC-88 pins targets and scores the *scope* of work against a model-capability profile. Nothing scores the *target*. The engine discovers that a repository has no runnable checks, no reproducible build, or no way to exercise the running system when the RFC-90 build phase machine fails on it — after the epoch is open, the workspace is prepared, the wave is open, and the model spend is committed.

Emery's loop consumes specific, assessable target properties. A pinned CID is enough to assess them. This RFC assesses them once, at discovery, records the result as a fact, and uses it to decide which execution policies the target is eligible for.

## Problem

Two failures follow from the missing assessment.

**Late failure costs the most expensive part of the run.** Preflight in RFC-97 D2 rejects a missing tool before the first build model call, which is the right instinct applied one stage too late. By then the operator has already authored, refined, reviewed, and authorized against a target that was never going to close a build phase.

**Every policy is offered on every target.** The four orchestration policies in [platform.md](platform.md#orchestration-policies) are uniformly available today. An unattended candidate policy on a target with no attestable verification surface produces confident, unverifiable output — precisely the outcome the audit posture exists to prevent. The gate belongs on the target, not on the operator's judgment about the target.

Readiness is also the honest answer to a delivery question. Most estates Emery is pointed at were built before any of this existed. Knowing which dimension blocks which policy, before quoting the work, converts an unbounded risk into a scoped one.

## Terms

- A **readiness dimension** is one member of the closed set below, scored as an integer from zero through ten.
- A **readiness profile** is deployment-owned, versioned data carrying per-dimension weights, per-dimension floors, and band thresholds. Its digest enters every assessment.
- A **readiness assessment** is one target's scored dimensions, rationale, and computed band, bound to that target's pinned CID and the profile digest.
- A **readiness band** is the closed value `unready | assisted | reviewed | progressive | unattended`.
- A **readiness finding** is one dimension's gap expressed as an ordinary `diagnostics::Diagnostic`.

## Decisions

### D1 — Readiness is a discovery-time property of a pinned target

Discovery already resolves each target to an exact `locator` and `cid` (RFC-88 D5). The readiness assessment runs inside RFC-88 D3's Discover-topology phase against that immutable tree, under the same D9 read budgets, and writes its result into the target's `discovery.yaml` row.

Readiness is never assessed against an ambient checkout, a working tree, or a live service. It is a property of the value the change pinned, so it is reproducible, and re-running discovery on a moved branch produces a new assessment bound to a new CID rather than silently drifting.

Target discovery and source selection are independent (RFC-88 D5), and only targets are assessed: readiness describes where Emery writes, not what it reads. A repository that appears as both resolves to one CID by content addressing and therefore carries one assessment.

### D2 — The dimension set is closed and derived from what the loop consumes

Each dimension exists because a named part of Emery consumes it. A property no operation consumes is not a readiness dimension, however good an engineering practice it is.

| Dimension | Question it answers | Consumed by |
| ------------------------------ | ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `build-determinism` | Does a declared command set build this target from a clean tree with pinned dependencies? | RFC-87 private workspaces; RFC-97 `build` profile |
| `verification-attestability` | Are there automated checks a host can run and attest, and do they reach the behaviour under change? | RFC-90 verify / repair; RFC-97 `test` and `ci` profiles |
| `environment-reproducibility` | Can a workspace be materialized without hidden host state, manual setup, or ambient credentials? | RFC-87 prepare; RFC-100 worker placement |
| `behavioural-observability` | Can the running system be started, driven, and observed programmatically, with output durable enough to read back? | `captures` source adapter; RFC-98 replay |
| `boundary-legibility` | Are ownership and module boundaries clear enough to partition without ambiguous overlap? | RFC-88 decomposition; ownership envelopes |
| `intent-recoverability` | Do documentation, tests, contracts, or runtime captures carry enough recoverable intent to synthesize requirements? | Source adapters; synthesis; authority resolution |

The judgment supplies the six integers and a rationale per dimension. It never supplies the score, the band, or the thresholds. This is the same split RFC-88 D3 already applies to complexity assessment, for the same reason.

**These dimensions are not RFC-88's.** The model-capability profile scores a *scope*: how much behavioural breadth, coupling, uncertainty, context volume, and verification surface one unit of work carries, to decide whether it splits. This profile scores a *target*: what the repository can support, to decide which policies may run against it. RFC-88 asking "how much verification surface does this work have" and this RFC asking "can a host attest any check here at all" are different questions about different subjects, which is why the dimension here is named `verification-attestability` rather than reusing the capability profile's `verification-surface`. The two profiles are resolved independently and neither reads the other.

`behavioural-observability` is deliberately a first-class dimension rather than a note under testing. It is the dimension that decides whether [RFC-98](rfc-98-behavioural-conservation.md) can run at all, and on a legacy estate it is usually the lowest-scoring and the most valuable to raise.

### D3 — Scoring is deterministic, and floors bind independently of the weighted sum

The engine computes the weighted sum and then applies **per-dimension floors** for each band. A band is reached only when the weighted sum clears its threshold *and* every dimension clears that band's floor for that dimension.

Floors are the load-bearing half. An aggregate alone lets a strong average conceal a fatal dimension — a target with excellent documentation, clean boundaries, and a reproducible build can still be unverifiable, and averaging says it is ready. The floor for `verification-attestability` at the `unattended` band is what stops that.

```yaml
# readiness profile — deployment-owned, versioned
id: brownfield-v1
digest: sha256:…
weights:
  build-determinism: 3
  verification-attestability: 4
  environment-reproducibility: 2
  behavioural-observability: 3
  boundary-legibility: 2
  intent-recoverability: 2
bands:
  assisted:
    threshold: 30
    floors: { build-determinism: 2 }
  reviewed:
    threshold: 60
    floors: { build-determinism: 4, verification-attestability: 3, environment-reproducibility: 3 }
  progressive:
    threshold: 85
    floors: { build-determinism: 6, verification-attestability: 5, environment-reproducibility: 5, boundary-legibility: 5 }
  unattended:
    threshold: 110
    floors: { build-determinism: 7, verification-attestability: 7, environment-reproducibility: 7, behavioural-observability: 6, boundary-legibility: 6 }
```

A target that clears no band's threshold and floors is `unready`. Profiles are deployment data on the RFC-97 profile-policy shape: the engine owns the arithmetic, the deployment owns the numbers, and no project file, prompt, model answer, or source Evidence may edit either.

### D4 — The band gates execution policy, never slice authoring

Authoring is how an operator learns an unfamiliar estate. Blocking it on readiness would deny the operator the survey that explains why readiness is low.

`emery plan author` therefore records readiness and continues. The band binds at `emery plan execute`, which refuses a policy the target is not eligible for, typed `target-readiness-insufficient`, naming the target, band, and the exact dimensions below their floors.

| Band | Eligible policies | Notes |
| -------------- | ----------------------------------------------------- | ----------------------------------------------------------------------------- |
| `unready` | Authoring and refinement only | `plan execute` refuses; the plan is still a legitimate deliverable |
| `assisted` | Reviewed policy with `gap-policy: strict` forced | Deferral under an effective `defer` policy is unavailable at this band |
| `reviewed` | Reviewed policy ([platform.md](platform.md#1-reviewed-policy) 1) | The ordinary supervised loop |
| `progressive` | Adds progressive specs-only (policy 2) | Specification work may run ahead of review |
| `unattended` | Adds unattended candidate build (policy 3) | Candidate build only; the accepted CID is untouched |

Readiness never grants merge authority. [RFC-102](rfc-102-policy-gated-autonomy.md) policy-gated autonomy requires its own promoted policy, protected assurance, and commit admission on top of an `unattended` band; a high band is necessary and never sufficient.

A change spanning several targets takes the **lowest** band across the targets it will write to. The weakest repository in the estate sets the policy for the change, because a wave that commits across targets is only as trustworthy as its least verifiable member.

### D5 — Remediation is projected, never applied

`emery target readiness <target>` is a read-only projection: the assessment, the band, the blocking dimensions, and one `diagnostics::Diagnostic` per gap with `source: hybrid` and `kind: review`. Findings carry the dimension, the observed evidence, and the band each gap blocks.

There is no fix verb. Raising a dimension means changing the target — adding a check, pinning a dependency, making the app scriptable — and that is ordinary product work that belongs in a slice under an epoch, with a spec, a review, and a merge fact, exactly like every other change Emery makes. A privileged unreviewed write into a client repository, performed before any authorization epoch exists, is the one shape this architecture refuses everywhere else; readiness is not the place to introduce it.

An operator who wants remediation authors a change whose intent is the readiness findings. Its slices raise dimensions, the next discovery re-assesses against the new CID, and RFC-93 outcome records make the before/after measurable.

### D6 — Readiness binds into coverage and outcome records

The assessment is recorded once and referenced by digest thereafter:

```yaml
# discovery.yaml — additive to the RFC-88 D5 target row
targets:
  orders:
    adapter: emery:omnia@1.4.0
    locator: https://github.com/acme/orders@0123…4567
    cid: sha256:…
    readiness:
      profile: { id: brownfield-v1, digest: sha256:… }
      dimensions:
        build-determinism: 7
        verification-attestability: 4
        environment-reproducibility: 6
        behavioural-observability: 2
        boundary-legibility: 6
        intent-recoverability: 5
      score: 77
      band: reviewed
      digest: sha256:…
```

That target reaches `reviewed` on both tests and stops there on both: 77 is below `progressive`'s threshold of 85, and `verification-attestability: 4` is below its floor of 5. Its `behavioural-observability: 2` is also why RFC-98 replay is not yet viable against it, which is the single most useful thing a remediation change could fix.

`discovery.yaml` also records the resolved profile's closed body beside its id and digest, on RFC-88 D3's rule for model-capability profiles, so an archived change explains its own bands without the deployment registry that produced them.

`plan.yaml` copies the readiness digest and band per target. The plan digest transitively binds them, so RFC-88 D8's closed-plan coverage already carries readiness without a new coverage field. Changing a readiness profile creates a new digest and invalidates the epoch, on the same rule as a model-capability profile.

RFC-93 outcome records carry the band and dimensions of every touched target, so recurrence analysis can ask which dimension actually predicted repair count, amendment rate, and terminal failure. The weights in D3 are a starting hypothesis; the outcome record is how they stop being one.

## Implementation requirements

- Add the closed `ReadinessDimension` enum, `ReadinessProfile`, `ReadinessAssessment`, and `ReadinessBand` DTOs to `crates/project`, rejecting unknown fields, with a canonical digest independent of YAML formatting.
- Ship the first-party readiness profile registry in the deployment-provider layer beside RFC-97's profile policies. Engine crates carry no concrete weight or floor constant.
- Extend RFC-88's Discover-topology phase with one readiness judgment per target under the existing D9 read and concurrency budgets. The judgment returns dimensions and rationale only; band computation is deterministic engine code.
- Extend `discovery.yaml` and `plan.yaml` target rows with the readiness record and digest. Validation rejects hand-edited scores, bands inconsistent with the profile, and assessments bound to a stale CID.
- Add the band eligibility gate to `emery plan execute` ahead of epoch creation, typed `target-readiness-insufficient`, and take the minimum band across written targets.
- Add read-only `emery target readiness` projecting the assessment and its per-dimension findings through the existing `diagnostics` substrate. Add no second finding identity and no fix verb.
- Extend RFC-93 outcome records with per-target band and dimensions.
- Crate-level integration coverage for assessment determinism, floor-versus-threshold gating, minimum-band selection across targets, epoch invalidation on profile change, and stale-CID rejection.

## Acceptance criteria

1. Discovery assesses every pinned target against a versioned profile and records dimensions, score, band, and digest in `discovery.yaml`. Two runs over the same CID and profile produce byte-identical assessments.
2. A target whose weighted sum clears a band threshold but whose `verification-attestability` is below that band's floor does not reach the band. The refusal names the dimension.
3. `plan author` and `plan refine` complete normally on an `unready` target. `plan execute` refuses before opening an epoch, workspace, or wave, typed `target-readiness-insufficient`.
4. A change writing to two targets takes the lower band; raising the weaker target's readiness and re-discovering raises the change's eligibility.
5. Changing the readiness profile creates a new digest, invalidates the prior closed-plan epoch, and requires a fresh execute.
6. `emery target readiness` emits one diagnostic per gap with its blocked band, and exposes no verb that writes into the target.
7. An assessment bound to a superseded CID fails validation rather than being reused.
8. Outcome records carry per-target band and dimensions, and removing a target's record from the analysed set changes the analysis digest.

## Rejected alternatives

- **A general repository maturity model.** Scoring branch protection, issue templates, or analytics instrumentation measures organisational practice, not whether the RFC-90 phase machine can close on this tree. A repository can score well on a generic rubric and still be one Emery cannot build. Every dimension here names the operation that consumes it.
- **Aggregate-only gating, such as "pass 80% of the previous level".** Percentage-of-level gating is an average, and an average lets a strong majority conceal the one dimension that makes autonomous work unsafe. Per-dimension floors are the actual gate; the weighted sum only orders targets within a band.
- **Let the judgment return the band.** The same target would become eligible for different policies under an unrecorded policy. RFC-88 D3 already settled this for complexity thresholds and the reasoning is identical.
- **A `readiness fix` verb.** Privileged, unreviewed writes into a client repository before any authorization epoch exists, with no spec, no gap gate, and no merge fact. Remediation is ordinary slice work.
- **Readiness as a lint over the operator's checkout.** Readiness must be reproducible and archivable, so it is a property of a pinned CID, not of an ambient tree.
- **Blocking authoring below a band.** Authoring is the cheapest way to learn why readiness is low; only privileged work needs the gate.
- **Per-slice readiness.** Readiness describes the target's capacity to be built and verified, which does not vary by slice. Scope difficulty is already the model-capability profile's job.

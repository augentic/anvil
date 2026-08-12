# Emery vs. Factory.ai — services programme evaluation

**Date:** 2026-08-12
**Scope:** the active and parked RFC-86 through RFC-103 programme, `brand/strategy.md`, `rfcs/platform.md`, `rfcs/roadmap.md`, and the current public record on Factory (Missions, Agent Readiness, Legacy-Bench, enterprise controls).
**Status:** Working review note — not an RFC. Edit freely.

## Verdict

Emery has not gone off track. Factory's public record validates Emery's two central bets rather than undermining them: verification needs an independent feedback loop, and repository readiness constrains what autonomy can achieve. The useful corrections are not architectural reversals: make readiness deterministic-first, turn readiness findings into ordinary change inputs, add a public evidence asset, add preventative operator authorization beside attribution, and pull serial native verification plus conservation ahead of speculative scale.

The programme now applies those corrections. RFC-97 is split into serial Phase A and post-concurrency domain Phase B; RFC-93 owns the operator boundary; RFC-99 through RFC-102 are parked; and the active sequence is services-first rather than platform-completeness-first.

## The load-bearing difference

Factory and Emery attack the same failure — agents drowning in context and lying to themselves about success — with opposite mechanisms.

Factory's answer is **behavioural**: specialized droids with scoped context, mission plans with a finite validation contract, fresh worker and validator contexts, and prompt hygiene against context dilution. Its public guidance and demonstrated Mission runs favour frontier-class models, even though the product also supports role-specific and local/custom models.

Emery's answer is **structural**: typed artifacts with digests, one operation per dispatch, and an engine-owned phase machine that separates generation, verification, repair, and review while preventing the target from choosing phase order or terminal success (RFC-90). RFC-97 then moves verification outside the model into host-attested profiles, and RFC-98 supplies protected oracles outside every writer's reach. The intended assurance does not depend on the model policing itself, although RFC-90's shipped verification remains model-assisted until RFC-97 lands.

This is precisely the property that may make SLMs viable. A small model need not hold a mission-sized context or self-orchestrate if it executes one typed operation over a 16-finding repair brief. Factory is optimized and publicly validated around stronger models; Emery is designed to permit smaller ones. That is the thesis, not yet a measured moat, and every RFC that makes it testable (90, 91, 92, 97, 98) is well-aimed. Do not dilute it; prove it.

## Where Factory independently validates Emery

Three findings from Factory's published work are effectively empirical evidence for Emery's design:

**"Agents don't know when they're wrong."** Legacy-Bench reports that in 97% of failures the agent *believed it had solved the task*. This is strong external justification for RFC-90's separation of build from verify and review, and for RFC-97's insistence that attestation come from native tool execution rather than model self-report. Legacy-Bench's held-out tests and RFC-98's protected capture corpus are not the same authority surface — one grades a benchmark after the run, while the other participates in production admission — but both validate the principle that the implementation process must not rewrite its oracle.

**Feedback-loop quality dominates agent performance.** Legacy-Bench's core result is that agents score highest where errors are visible (Java stack traces) and collapse where failure is silent (COBOL packed decimals). This is RFC-94's determinism/observability/legibility dimensions, discovered independently at benchmark scale.

**Readiness before privileged execution.** Factory recommends Agent Readiness Level 4+ for the best Mission results and requires High autonomy for Mission orchestration; readiness level itself is guidance, not a hard product gate. RFC-94 applies the stronger principle to Emery's current reviewed workflow by binding policy eligibility to a pinned target CID. Parked RFC-102 would later extend that principle to unattended accepted-state mutation. Neither is a current product claim.

## Where Factory is ahead — the corrections worth making

### 1. Make readiness evidence deterministic-first (RFC-94)

Factory evaluates 60+ binary criteria with LLM assistance, uses deterministic checks where available, and reports that grounding each evaluation in the prior report reduced average variance from 7% to 0.6%. It did not make the complete assessment deterministic. RFC-94 should go further because its band is policy authority: host-observed probes supply facts wherever possible; model judgment supplies advisory findings and cited evidence; and unattended eligibility requires deterministic or host-attested minimum predicates. The assessment remains digest-bound and labels every contributing source honestly.

### 2. Close the readiness → remediation loop

Factory's `/readiness-fix` spins up an agent that opens a PR fixing failing criteria. Emery should not copy that privileged pre-epoch write path. RFC-94 D5 already has the right lifecycle: readiness gaps are diagnostics, not source-adapter leads, and remediation is an ordinary change. Strengthen the projection so each finding carries machine-readable remediation intent and the affected target; an operator or outer agent can then feed that intent through `plan author → plan refine → plan execute`, with full provenance. If automation later needs real leads, a source adapter may consume the report and emit them under the ordinary source contract.

### 3. Let readiness select policy eligibility, not execution mechanics

Legacy-Bench shows that feedback quality changes the expected value of another repair, but a model-assisted readiness score must not directly mutate RFC-90 budgets or RFC-92 routes. RFC-94 should determine which named policy classes a target is eligible for. The deployment then resolves one pinned policy before authorization. That policy may lower compiled repair maxima; RFC-92 independently owns any predeclared route ladder, and RFC-97 exposes the deterministic `unchanged-failure-set` predicate that can trigger it. Readiness may recommend a stronger profile, but no score changes execution in flight and no policy may raise an engine maximum.

### 4. Build the evidence asset

Factory ships Legacy-Bench and publishes numbers; that credibility is worth more than another feature claim. Emery's SLM thesis — that governed structure lets smaller models produce competitive verified outcomes on enterprise tasks — is currently an assertion. RFC-92 supplies route and usage instrumentation; RFC-97/98 supply the assurance measurements; the existing `probe` rung owns the experiment. Add a separate reproducible, publishable evaluation program measuring cost per verified result, repair and escalation rates, protected-oracle success, human correction load, and conservation coverage. A Harbor-compatible outer runner would make a subset directly comparable without forcing Emery's multi-stage artifact workflow into Harbor's single-task contract.

### 5. One bounded enterprise integration: journal → OpenTelemetry projection

Factory's enterprise story leans on OTel export and centralized visibility. Emery's per-writer fact journal is a richer semantic substrate; a read-only projection to OTel spans/events would let it join existing enterprise observability. This does not require activating parked RFC-101, so it is [RM-30](roadmap.md#rm-30-journal-opentelemetry-projection). It is not lifecycle authority and is not free: the design must bound cardinality, redact sensitive evidence, respect retention, tolerate exporter backpressure, and keep message content disabled by default.

## Where Emery should explicitly not follow Factory

- **Session/chat-centric lifecycle authority.** Emery's no-undo, resume-by-rerun model is harsher but produces one semantic, digest-bound delivery record. Factory has Mission artifacts, audit logs, telemetry, and CI records; it does not publicly demonstrate an equivalent unified substrate. Keep Emery's distinction precise.
- **Runtime-only specialization.** Factory can encode specialist behaviour in Custom Droids, skills, and hierarchical instructions. Emery's stronger seam is versioned Wasm adapters with typed operations, deterministic metadata, and embedded standards that the engine constrains. Keep that enforceable boundary.
- **Autonomy as the headline.** Factory sells autonomy and retrofits governance. Emery sells evidence-backed reviewed delivery. RFC-102's promoted autonomy model remains parked until a client needs unattended accepted-state mutation and its evidence prerequisites exist.

## Internal critique of the series itself

Independent of Factory, the careful read surfaced one real risk: **sequencing weight**. RFC-96 (concurrency), RFC-99 (streaming), RFC-100 (distribution), RFC-101 (fleet), and RFC-102 (autonomy) stack substantial machinery onto an engine whose shipped wave is one-member. None differentiates Emery for an evidence-backed modernization engagement as directly as RFC-92, RFC-94, RFC-97, and RFC-98. The verification and conservation tracks are also where Legacy-Bench says frontier agents fail hardest: silent-failure legacy domains.

The dependency graph originally prevented the right sequence because RFC-97 hard-depended on RFC-96. The accepted correction splits RFC-97 into a serial `slice-attempt` cut over RFC-87/90 and later domain contexts over RFC-96. The active order is now RFC-92/93 → RFC-88 → RFC-94 → RFC-97 Phase A → RFC-98 → public RM-29 → RFC-103. RFC-95 follows RFC-88 on the product lane. RFC-96 remains active but evidence-gated; RFC-97 Phase B remains unchanged behind it. RFC-99 through RFC-102 are parked.

Two smaller notes:

- RFC-103's outcome learning is correctly inert-by-design, but it has no signal source until RFC-92's usage facts and an eval grading harness exist — it should be explicitly sequenced after both.
- RFC-88's conflict-domain decomposition is the series' most judgment-heavy leg; the refinement-feedback loop it specifies is the right containment, but it's the RFC to watch most closely for scope creep once implementation starts.

## Governance gaps the Factory comparison exposes

**Attribution is not authorization.** RFC-93 now owns both sides of the operator boundary without collapsing them: actor class and attestation remain descriptive, while a deployment-owned operator grant may refuse an otherwise legal `drop`, authority override, or `--force` before guest dispatch. The grant carries principal binding, allowed acts and scope, digest, expiry, and optional external-approval requirements. It adds no lifecycle rung and cannot weaken an engine gate.

**Assurance has two axes.** RFC-97's `oracle-assurance: candidate | protected | mixed` describes oracle independence. `execution-assurance: model-assisted | host-attested | hybrid` describes how verification ran. Do not collapse them into one enum. Commit admission should state both, and model-assisted-only results should remain candidates rather than autonomously mutating accepted state.

**Auto-deferral and autonomy need one rule.** Reviewed execution auto-defers every open row at the build gate, while parked RFC-102 requires no `[unknown]` or `[conflict]`. An auto-deferred row therefore remains ineligible for autonomous commit; regulated autonomy profiles should default to zero live deferred gaps.

**Protected data needs governance as well as write denial.** RFC-98 keeps the corpus outside every writer, but regulated captures also need classification, redaction, encryption, access control, retention, residency, replay-side-effect controls, and access audit in deployment policy.

**Semantic audit is not tamper evidence.** Local append-only facts are a reconstructable delivery record, not non-repudiation. Parked RFC-101 retains authenticated writers and immutable or sealed audit storage as a hosted-deployment trigger. Product prose must not claim tamper resistance before that capability exists.

## Disposition of recommendations

1. **Incorporated in RFC-94:** deterministic-first authority, host-attested criteria, and machine-readable remediation intent without a privileged fix verb.
2. **Incorporated in RFC-92:** pinned, engine-triggered route escalation; usage remains observation and readiness cannot reroute in flight.
3. **Incorporated in RFC-97:** serial Phase A and post-RFC-96 domain Phase B, with oracle assurance separate from execution assurance.
4. **Incorporated in RFC-98:** protected-corpus data governance.
5. **Preserved in parked RFC-102:** no model-only autonomous merge, explicit deferred-gap semantics, and two-axis assurance.
6. **Moved to active RFC-93:** deployment-owned operator authorization beside, but not inferred from, attribution. Authenticated/sealed shared audit storage remains parked with RFC-101.
7. **Added as RM-29:** reproducible SLM-under-governance evaluation, optionally Harbor-compatible at its outer boundary.
8. **Applied in platform.md:** active services sequence; RFC-96 evidence-gated; RFC-99 through RFC-102 parked and removed from the dependency map.
9. **Applied in brand/strategy.md:** behavioural conservation and the joined delivery record support the modernization wedge and long-lived relationship.
10. **Added as RM-30:** non-authoritative journal → OpenTelemetry projection without activating the fleet programme.

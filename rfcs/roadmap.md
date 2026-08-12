# Emery Roadmap

> Status: Draft

The [Services Delivery Programme](platform.md) is the active delivery spine; [architecture.md](architecture.md) owns the runtime boundary. RFC-99 through RFC-102 are parked in the programme and do not appear here as active follow-ons. This file lists smaller engagement- or evidence-triggered opportunities that do not need lifecycle RFCs. Each starts when its trigger is observed, not merely when its prerequisites exist.

## Engagement-triggered

### RM-17: Forge publication providers

**Goal:** Extend RFC-88/95's read-only forge capability with operator-triggered branch transport, PR/MR create-or-update, CI/mergeability, and provider links, then add GitLab, Bitbucket, and self-hosted bindings.
**Trigger:** RFC-88/95 are landed and manual branch/PR handoff becomes a material operator bottleneck. Emery still does not create repositories or acquire publication lifecycle authority.

## Evidence-triggered

### RM-29: Governed model evaluation asset

**Goal:** Publish reproducible evidence for the claim that typed operations, bounded recovery, host verification, and protected conservation let smaller models deliver competitive verified outcomes at lower cost.
**Trigger:** RFC-92 usage facts and route policies plus RFC-97 Phase A host verification are stable enough to compare; RFC-98 adds the modernization-specific conservation rung when available.
**Likely shape:** an evaluation suite owned by `probe`, not RFC-92 lifecycle code, reporting cost per verified result, escalation and repair rates, protected-oracle success, conservation coverage, elapsed time, and human correction load across pinned route policies. Add a Harbor-compatible outer runner for directly comparable single-task cases without flattening Emery's plan/refine/execute artifacts into Harbor's task contract. Publish case definitions, policy/model identities, repeated-run methodology, failures, and confidence intervals; vendor benchmark numbers are context, not ground truth.

### RM-30: Journal OpenTelemetry projection

**Goal:** Project selected workflow facts into a client's existing observability system without making the exporter lifecycle authority or activating the parked fleet programme.
**Trigger:** an enterprise engagement requires centralized run visibility and cannot consume `emery journal show` or archived projections directly.
**Likely shape:** a read-only, lossy journal → OpenTelemetry exporter with bounded span/event cardinality, stable semantic attributes, sensitive evidence and model content excluded by default, deployment-owned redaction and retention, and backpressure or exporter failure isolated from workflow progression. The fact log remains the source of truth; OTel data is an operational projection and never a replay substrate.

### RM-11: Dependency-aware compatibility gates

**Goal:** Account for breaking producer changes with consumer follow-up before publication.
**Trigger:** RFC-88/95 are landed and a real multi-repository contract change demonstrates that publication order plus adapter findings are insufficient.
**Likely shape:** adapter-owned compatibility classification feeding an engine `plan impact` projection and a merge/finalize gate.

### RM-21: Adapter ecosystem operating model

**Goal:** Support dependable third-party adapters over the versioned `emery:adapter` WIT contract.
**Trigger:** the first external adapter author.
**Remaining:** public SDK distribution, third-party namespaces and trust, release indexes, compatibility policy, migration guidance, and author quality gates.

### RM-22: Measured prose budgets

**Goal:** Replace the reviewed line caps on adapter prose with a measured per-operation budget, so prompt corpus growth is bounded by evidence rather than by house style.
**Trigger:** a build operation demonstrably degrades because its parent and phase prompts crowd out slice artifacts, or a target's prose corpus grows past the point where review can hold the caps.
**Likely shape:** adapter metadata declares a per-operation prose budget; the embed-time walker measures the loaded set against it and fails the component build on overrun. The references server already serves on demand, so this measures what is *loaded*, not what is embedded. No runtime trimming — a budget the engine can silently satisfy by dropping prose is not a budget.

### RM-23: Specification write-back

**Goal:** Close the loop between the specification Emery recovers and the documentation the client keeps, so a delivered change leaves the estate's own documentation current rather than newly stale.
**Trigger:** a client engagement where the `documentation` source adapter reads material that a prior Emery change invalidated.
**Likely shape:** a documentation *target* adapter whose build operation writes back agreed requirements and decisions. Adapter names are unique across axes, so it cannot reuse the `documentation` name. Authority ordering is the hard part and must not invert: `spec.md` remains authoritative, write-back is a projection of it, and the next survey must not treat Emery's own output as independent corroboration of Emery's own conclusions.

### RM-24: Operator control surface

**Goal:** Make topology, recovery proposals, and typed stops reviewable by a person and directly actionable by an outer agent without putting a model inside the scheduler.
**Trigger:** the first agent-driven run spends material time interpreting an ambiguous stop; an operator skips topology or proposal review because the artifacts are too large; or proposal review measurably dominates a slice's wall-clock.
**Likely shape:** three read-only projections over existing authority:

- a decomposition tree over `plan.yaml` / `discovery.md`, with per-domain provenance and [RFC-94](rfc-94-target-readiness.md) bands;
- a diff of an inert amendment proposal's candidate lead and decomposition revisions against current authority, plus a retrospective view of which proposal was applied and under which RFC-93 actor/grant;
- an exact next-action card on every refine/execute stop naming the verb, selectors, input digests, and artifact paths the caller must fix or supply.

Amendment stays on `emery plan amend`; the projections never apply work, add an “apply all” path, or become another dashboard authority. Recovery remains stop → inspect → fix inputs or apply one reviewed proposal → re-run. When an agent drives Emery these projections are the human audit surface over what it accepted, proposed, and changed, so this is assurance work rather than presentation polish.

### RM-25: Target runtime harness declaration

**Goal:** Let a target adapter declare how the system it builds is started, driven, and observed, so [RFC-94](rfc-94-target-readiness.md) can assess `behavioural-observability` against something concrete and [RFC-98](rfc-98-behavioural-conservation.md) replay drivers can bind per target.
**Trigger:** RFC-98 lands and its second replay driver needs a per-target hook rather than a deployment-wide default.
**Likely shape:** declarative target metadata — launch command, readiness signal, drive surface, observation sink — resolved by deployment policy into a driver. Declarative only: no adapter-supplied command crosses WIT, on [RFC-97](rfc-97-native-verification.md) D3's rule.

### RM-26: Client-controlled model endpoint

**Goal:** Establish whether model traffic can be routed through a client-owned gateway, proxy, or self-hosted endpoint end to end, and close the gap if it cannot.
**Trigger:** before quoting any engagement in a regulated sector — government, banking, insurance, health — not after. This is the one item on this list that is a potential engagement blocker rather than an improvement, so the investigation precedes the commitment.
**Open question first:** [RFC-92](rfc-92-model-policy.md) D2 already places provider, endpoint, and credential binding in deployment policy rather than in change artifacts, so the design admits a client endpoint. What is unverified is the running deployment: model calls resolve through the Cursor-backed provider, and nobody has traced whether an alternative endpoint, an outbound proxy, or a custom CA is supported through that path. Trace it before designing anything.
**Likely shape, if the gap is real:** deployment-policy endpoint binding beside RFC-92's route table, standard proxy environment handling, and custom CA trust — all host-side, none of it visible to the engine guest or to any adapter. Data residency and egress posture are deployment concerns for the same reason credentials are.


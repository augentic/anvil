# Emery Roadmap

> Status: Draft

The [platform RFC chain](platform.md) is the current delivery spine; [architecture.md](architecture.md) owns the runtime boundary. [RFC-101 Platform Readiness](rfc-101-platform-readiness.md) is the hosted/fleet readiness spine that follows RFC-100, and the [evidence track](platform.md#evidence-track--what-the-series-asserts-but-cannot-yet-show) (RFC-92, RFC-94, RFC-98, RFC-103) closes the gaps between what the series asserts and what it can demonstrate. This file lists only distinct follow-ons that remain useful after those series. Each starts when its trigger is observed, not merely when its prerequisites exist.

## After the platform

### RFC-101: Platform readiness

**Goal:** Retire desktop-shaped host authority (`$EMERY_HOME` adapter stores, bare-name newest-local, process-local journals/MCP/Cursor credentials, homogeneous toolchains) so a hosted multi-node deployment is the same Emery with swapped Omnia backends and host policy.
**Trigger:** RFC-100's distributed contract is stable enough to bind, or Omnia backends approach the D10 conformance bar.
**Sequence:** Phases A–E in [rfc-101-platform-readiness.md](rfc-101-platform-readiness.md) (conformance & homes → adapter values & locks → worker capabilities → hosted judgment & ingress → tenancy & operations).

### RM-17: Forge publication providers

**Goal:** Extend RFC-88/95's read-only forge capability with operator-triggered branch transport, PR/MR create-or-update, CI/mergeability, and provider links, then add GitLab, Bitbucket, and self-hosted bindings.
**Trigger:** RFC-88/95 are landed and manual branch/PR handoff becomes a material operator bottleneck. Emery still does not create repositories or acquire publication lifecycle authority.

## Evidence-triggered

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

### RM-24: Topology review projections

**Goal:** Keep the post-authoring topology review genuinely reviewable at platform scale.
**Trigger:** the first change where operators skip topology review because the artifact is too large to read — likely the first deep RFC-88 recursive decomposition across several repositories. Reaching this trigger through agent-driven operation counts and arrives sooner, since a driver never finds an artifact too large.
**Likely shape:** read-only projections over `plan.yaml` and `discovery.md` (a decomposition tree, per-domain provenance, [RFC-94](rfc-94-target-readiness.md) bands in place). Deliberately not an editing surface and not a dashboard: amendment stays on `emery plan amend`, and the single-writer contract is what makes the review meaningful.
**Why it is not only ergonomics:** the operator need not be a person ([Operator identity](platform.md#operator-identity-an-agent-may-drive-the-engine)). When a driver authors and proceeds without pausing, this projection is no longer the operator's convenience — it is the only place a human sees the topology an agent accepted. Read with [RFC-103](rfc-103-operator-attribution.md) attribution, that makes it an assurance surface, which raises its priority above the "too large to read" trigger alone.

### RM-27: Amendment-proposal review ergonomics

**Goal:** Make reviewing and applying an inert amendment proposal fast, and make an applied proposal legible after the fact — operator time is the expensive input in a services engagement, and every recovery path in this architecture routes through a proposal.
**Trigger:** the first change where a boundary escalation or ownership fan-in proposal is applied without being properly read, or where proposal review measurably dominates a slice's wall-clock. An agent operator applying a proposal is the same trigger arriving without the warning sign, since nothing about it looks slow.
**Likely shape:** a read-only diff projection over a proposal's candidate lead-catalog and decomposition revisions against the current ones, plus a `plan status` projection that names the parked proposal and what applying it would change. The primitive is already right — RFC-88 keeps proposals inert and `plan amend` is the single writer. This is presentation only, and it must stay presentation only: the value of stop → inspect → fix inputs → re-run comes from the operator actually reading the proposal, so nothing here may add an "apply all" path or let a proposal apply itself.
**Why it is not only ergonomics:** proposals are where recovery decisions are made, and an autonomous driver can apply one as readily as a person can — the engine records the act but gates nothing ([RFC-103](rfc-103-operator-attribution.md) D6). A retrospective projection of *which* proposals were applied and by which actor class is therefore the human's audit surface over agent-driven recovery, not a convenience. That argues for building the after-the-fact view alongside the before-the-fact diff rather than deferring it.

### RM-25: Target runtime harness declaration

**Goal:** Let a target adapter declare how the system it builds is started, driven, and observed, so [RFC-94](rfc-94-target-readiness.md) can assess `behavioural-observability` against something concrete and [RFC-98](rfc-98-behavioural-conservation.md) replay drivers can bind per target.
**Trigger:** RFC-98 lands and its second replay driver needs a per-target hook rather than a deployment-wide default.
**Likely shape:** declarative target metadata — launch command, readiness signal, drive surface, observation sink — resolved by deployment policy into a driver. Declarative only: no adapter-supplied command crosses WIT, on [RFC-97](rfc-97-native-verification.md) D3's rule.

### RM-26: Client-controlled model endpoint

**Goal:** Establish whether model traffic can be routed through a client-owned gateway, proxy, or self-hosted endpoint end to end, and close the gap if it cannot.
**Trigger:** before quoting any engagement in a regulated sector — government, banking, insurance, health — not after. This is the one item on this list that is a potential engagement blocker rather than an improvement, so the investigation precedes the commitment.
**Open question first:** [RFC-92](rfc-92-operation-model-policy.md) D2 already places provider, endpoint, and credential binding in deployment policy rather than in change artifacts, so the design admits a client endpoint. What is unverified is the running deployment: model calls resolve through the Cursor-backed provider, and nobody has traced whether an alternative endpoint, an outbound proxy, or a custom CA is supported through that path. Trace it before designing anything.
**Likely shape, if the gap is real:** deployment-policy endpoint binding beside RFC-92's route table, standard proxy environment handling, and custom CA trust — all host-side, none of it visible to the engine guest or to any adapter. Data residency and egress posture are deployment concerns for the same reason credentials are.


# Emery Roadmap

> Status: Draft

The [platform RFC chain](platform.md) is the current delivery spine; [architecture.md](architecture.md) owns the runtime boundary. [RFC-96 Platform Readiness](rfc-96-platform-readiness.md) is the hosted/fleet readiness spine that follows RFC-93. This file lists only distinct follow-ons that remain useful after those series. Each starts when its trigger is observed, not merely when its prerequisites exist.

## After the platform

### RFC-96: Platform readiness

**Goal:** Retire desktop-shaped host authority (`$EMERY_HOME` adapter stores, bare-name newest-local, process-local journals/MCP/Cursor credentials, homogeneous toolchains) so a hosted multi-node deployment is the same Emery with swapped Omnia backends and host policy.
**Trigger:** RFC-93's distributed contract is stable enough to bind, or Omnia backends approach the D10 conformance bar.
**Sequence:** Phases A–E in [rfc-96-platform-readiness.md](rfc-96-platform-readiness.md) (conformance & homes → adapter values & locks → worker capabilities → hosted judgment & ingress → tenancy & operations).

### RM-17: Forge publication providers

**Goal:** Extend RFC-88/89's read-only forge capability with operator-triggered branch transport, PR/MR create-or-update, CI/mergeability, and provider links, then add GitLab, Bitbucket, and self-hosted bindings.
**Trigger:** RFC-88/89 are landed and manual branch/PR handoff becomes a material operator bottleneck. Emery still does not create repositories or acquire publication lifecycle authority.

## Evidence-triggered

### RM-11: Dependency-aware compatibility gates

**Goal:** Account for breaking producer changes with consumer follow-up before publication.
**Trigger:** RFC-88/89 are landed and a real multi-repository contract change demonstrates that publication order plus adapter findings are insufficient.
**Likely shape:** adapter-owned compatibility classification feeding an engine `plan impact` projection and a merge/finalize gate.

### RM-21: Adapter ecosystem operating model

**Goal:** Support dependable third-party adapters over the versioned `emery:adapter` WIT contract.
**Trigger:** the first external adapter author.
**Remaining:** public SDK distribution, third-party namespaces and trust, release indexes, compatibility policy, migration guidance, and author quality gates.


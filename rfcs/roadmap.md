# Emery Roadmap

> Status: Draft

The [platform RFC chain](platform.md) is the current delivery spine; [architecture.md](architecture.md) owns the runtime boundary. This file lists only distinct follow-ons that remain useful after that series. Each starts when its trigger is observed, not merely when its prerequisites exist.

## After the platform

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


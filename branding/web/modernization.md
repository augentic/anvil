# Modernize a critical system without rediscovering its contract in production

Most modernization programmes begin with a target architecture and a delivery plan. The highest-risk assumption is often left implicit: that the organisation already knows what the existing system must continue to do.

We begin by testing that assumption.

Propellerhead recovers the system's effective contract from the evidence available, turns uncertainty into explicit decisions and delivery boundaries, and modernizes the system in reviewable waves.

**Primary action:** Start with readiness  
**Secondary action:** Discuss a modernization programme

## When modernization becomes unavoidable

The trigger may be:

- an unsupported platform or dependency;
- a vendor exit or licensing change;
- a regulatory deadline;
- unacceptable operating cost or fragility;
- a security or resilience concern;
- key people approaching departure;
- a strategic product change the current system cannot support;
- a previous rewrite that failed to reach acceptance.

These triggers create urgency. They do not create understanding. Beginning implementation before recovering the behavioural boundary transfers unresolved risk into the new system.

## What conventional rewrites miss

Documentation describes part of the system. Code describes part of the system. Runtime behaviour describes what happens, but not always what should happen. Tests may protect important rules or merely reproduce old assumptions.

The real contract is distributed across all of them, along with operational practice and stakeholder intent.

We preserve those sources separately long enough to see where they agree, where authority can resolve a disagreement, and where uncertainty must remain visible. The result is not a claim of perfect knowledge. It is a defensible boundary for action.

## What is different from conventional modernization

Good modernization firms already perform discovery, use incremental replacement patterns, and deliver in stages. Our distinction is not a proprietary architecture pattern.

It is continuity between discovery and acceptance: source accounts retain their identity, material decisions name their authority, unresolved gaps survive planning, reviewed specifications become exact delivery inputs, and the agreed result becomes the starting point for the next wave.

That record must be demonstrated in delivery evidence and client outcomes. Method language alone is not proof.

## The modernization programme

### Establish the baseline

We identify the system and repository topology, critical journeys, interfaces, dependencies, operational constraints, and available verification evidence. Requirements remain connected to their sources.

### Shape bounded waves

We decompose the programme into outcomes that can be specified, implemented, verified, and accepted coherently. Dependencies and shared constraints remain visible; arbitrary work packages do not become delivery boundaries merely because they fit a sprint.

### Review before privileged work

Client stakeholders review the proposed topology and the refined specifications before implementation begins. Unknowns and conflicts are not silently converted into generated requirements.

### Build and verify

Senior engineers and AI-assisted delivery agents implement against the reviewed artifacts. Verification combines the checks the estate and target can support with engineering review and operational acceptance evidence. Where independent fixtures or captures exist, the engagement states whether and how they are isolated from the candidate writer; absence remains a verification gap.

### Accept and carry forward

Accepted specifications and decisions become the baseline for subsequent waves. Deferred gaps remain visible as debt rather than disappearing when a project board closes.

## What a wave leaves behind

Depending on the estate and target, a wave can leave:

- reviewed behavioural specifications with source provenance;
- explicit decisions and authority outcomes;
- a record of known unknowns, conflicts, and deferrals;
- target architecture and implementation artifacts;
- verification results tied to the delivered candidate;
- updated operational and client documentation;
- an agreed baseline for the next change.

The client owns its code, artifacts, evidence, and deployment.

## Modernize without hard-coding the next infrastructure trap

Where Omnia fits the target system, recovered domain behaviour is implemented in WebAssembly components that run in a fresh isolated instance for each invocation. Persistent state and external effects use typed infrastructure capabilities; the host runtime supplies concrete services for storage, messaging, identity, observability, and other effects.

A supported backend can then be changed in the host without changing or recompiling guest application logic. Data migration and provider-specific operational qualities still require engineering, but the application's business behaviour is less entangled with today's infrastructure choice.

[Explore infrastructure portability](infrastructure-portability.md)

## Commercial shape

Readiness establishes whether a responsible outcome boundary exists. Where the evidence and acceptance conditions support it, subsequent waves can be priced around the accepted outcome rather than around an open-ended allocation of people.

We do not manufacture fixed-price certainty before discovery. Unknowns that materially affect scope are exposed, resolved, deferred, or reflected in the commercial boundary.

## The role of AI

AI can accelerate codebase research, synthesis, implementation, testing, and review. Open-source agents will make much of that capacity widely available. Neither proprietary nor open agents decide what the organisation meant, who has authority over a conflict, which consequences are acceptable, or whether a business-critical outcome is ready.

Our delivery system keeps AI-produced work subordinate to durable evidence, explicit lifecycle gates, considered human judgment, and accountability.

## A responsible first step

If the system is not understood well enough to define the first wave, begin with readiness rather than a rewrite proposal.

[See the readiness engagement](readiness.md)

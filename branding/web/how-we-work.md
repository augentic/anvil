# Delivery built around evidence, decisions, and acceptance

Modernization fails when activity is mistaken for progress: more tickets, more generated code, and more parallel teams without a shared account of what the result must preserve.

Our method keeps discovery, decision-making, implementation, verification, and acceptance connected.

## Human judgment creates the value

Software delivery is not a contest to turn instructions into the largest volume of code. People decide which problem is worth solving, whose needs and constraints count, which trade-offs are responsible, and whether the result is good enough to enter the world.

We make reflection part of the delivery rhythm. Review points give practitioners and accountable stakeholders time to challenge assumptions and consider consequences while decisions remain reversible. This is not approval theatre or resistance to automation. It is the work that makes automation useful.

## Start with evidence, not a preferred answer

We gather distinct accounts of the system from intent, documentation, source code, interfaces, tests, runtime observations, and the people who operate it.

We do not merge those accounts prematurely. Provenance matters because two identical-looking requirements have different reliability when one comes from a current regulation and the other from an inference about old code.

## Keep uncertainty visible

Unknowns and conflicts are legitimate findings. Concealing them makes a plan look complete while moving risk into implementation and production.

Each material gap is resolved by appropriate authority, deferred with its consequence understood, or left outside the delivery boundary. It does not disappear because an agent can generate plausible text.

## Separate observed behaviour from intended behaviour

A running system is powerful evidence, but it is not automatically the specification. Existing behaviour may encode a contractual obligation, an undocumented user dependency, a defect, or an obsolete compromise.

We preserve what was observed and record who decided what should happen next.

## Deliver bounded outcomes

Large programmes are shaped into waves that can be reviewed and accepted coherently. Each wave has:

- an explicit behavioural boundary;
- known source evidence;
- identified dependencies and affected systems;
- visible gaps and deferrals;
- agreed verification and acceptance conditions;
- a durable result that becomes input to the next wave.

The boundary follows the system and the outcome, not an arbitrary allocation of people or calendar time.

## Review before implementation

Our modernization engagements define two review points before privileged delivery work:

1. review the proposed topology — whether the programme has been divided at the right boundaries;
2. review the refined specifications — whether the proposed outcome reflects the evidence and decisions.

For this service, implementation begins only after the approvals required by the engagement's change-control agreement. Emery exposes the review seams and binds execution to refined inputs; the agreement defines who must review, what constitutes approval, and how that approval is recorded.

## Use automation without delegating accountability

We use deterministic tooling where the answer can be computed and AI-assisted agents where research, synthesis, implementation, or review needs judgment.

Agents may perform work. They do not become the source of business authority, silently close evidence gaps, or decide whether a critical outcome is acceptable.

Propellerhead remains accountable for delivery. Client stakeholders remain accountable for business intent and acceptance.

We expect open-source models, agents, skills, and evaluation tools to make capable coding assistance broadly available. We welcome that democratisation. Our differentiation cannot be access to an agent; it must be the quality of judgment, context, delivery discipline, verification, and accountability around it.

## Verify the candidate, not the confidence of its author

Verification is strongest when the writer cannot alter the evidence that judges the result. Depending on what the estate currently supports, the verification plan can combine:

- existing and newly authored automated tests;
- contract, schema, build, and static verification;
- target-specific engineering review;
- independent fixtures or captures where they can be isolated from the candidate writer;
- operational acceptance evidence.

We describe the assurance actually earned. Candidate-authored tests are useful but do not become independent evidence merely because they pass. Where independent protection is not available, we report the gap rather than imply it.

## Preserve a living baseline

Agreed specifications, decisions, verification records, and remaining debt continue with the product. The next change starts from that baseline instead of reconstructing the estate from chat logs, tickets, and memory.

## Preserve infrastructure options

Where Omnia is appropriate, application logic runs as WebAssembly components in a fresh isolated instance for each invocation. Persistent state and external effects use typed capabilities; the host supplies concrete infrastructure backends.

This lets a supported backend change without changing or recompiling guest application logic. It does not automate data migration or guarantee identical service behaviour, but it reduces infrastructure coupling and keeps deployment choices explicit.

[Explore infrastructure portability](infrastructure-portability.md)

## Clients retain control

The client owns its code, specifications, evidence, decisions, and deployment. Publication remains part of the client's normal Git, review, release, and governance process.

We aim for continuity without lock-in: staying with Propellerhead should be valuable because the practice and accumulated understanding improve delivery, not because the client is technically prevented from leaving.

## The practical rhythm

### Plan

Survey the sources, identify the change topology, and propose bounded outcomes.

### Review

Confirm that the topology reflects the system and the programme's intent.

### Refine

Extract evidence, reconcile sources, expose gaps, and produce reviewable specifications and delivery tasks.

### Review again

Confirm what will be built and what uncertainty remains.

### Execute

Authorize the exact refined inputs, then build, verify, repair, review, and merge within the agreed boundary.

### Accept and continue

Publish through the client's normal controls, preserve the agreed baseline, and use it for the next wave.

[Explore modernization](modernization.md)  
[Explore continuous assurance](continuous-assurance.md)

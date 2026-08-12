# Change critical systems without losing what matters

When a critical existing system must change but nobody fully understands what it does, a conventional rewrite is a bet. This is our primary focus.

Propellerhead recovers what the system really does, creates room for the people accountable for it to consider what must be preserved or changed, and modernizes it in bounded, reviewable waves.

We selectively apply the same discipline to new critical systems, where rushed implementation can turn today's assumptions and infrastructure choices into tomorrow's undocumented constraints.

Our method is designed to keep the delivered result connected to the evidence, stakeholder decisions, and verification that support it.

**Primary action:** Start with a readiness engagement  
**Secondary action:** Talk to us about a critical system

## The difficult part is not producing new code

For existing systems, the real contract accumulates across source code, runtime behaviour, documentation, operating procedures, integrations, and the knowledge of people who keep them running. For new systems, the challenge is to make intent, trade-offs, acceptance, and operating constraints explicit before code volume creates an illusion of progress.

Producing code is becoming easier. Deciding what is worth building, reflecting on who and what it affects, and accepting responsibility for the result remain human work.

A delivery process that conceals disagreement or skips consideration creates false certainty. A programme that attempts to resolve everything before delivering creates delay. We make uncertainty explicit, decide what matters with the right stakeholders, and turn the result into practical delivery boundaries.

## From intent and evidence to an agreed result

### Establish or recover what the system must do

For a new system, we begin with intent, constraints, affected people, and the conditions for acceptance. For an existing system, we also examine code, documentation, interfaces, runtime evidence, tests, and operational knowledge.

### Consider the consequences

We create explicit moments for practitioners and accountable stakeholders to question assumptions, resolve ambiguity, and consider operational, social, security, and long-term consequences. Reflection is part of building well, not a pause from delivery.

### Build or modernize in bounded waves

Each wave has an explicit scope, known evidence, visible gaps, and reviewable acceptance boundaries. Smaller decisions become accepted foundations for the next wave.

### Demonstrate the result

We connect requirements and decisions to implementation and verification. The organisation can see what changed, why it changed, what was checked, and what uncertainty remains.

## Modernization first, with a selective extension

### Modernize an existing system

Understand the estate before committing to a programme. Recover critical behaviour, assess delivery and verification readiness, identify uncertainty, and define the first credible modernization wave.

[Explore modernization](modernization.md)

### Build a new critical system

For selected long-lived or high-consequence systems, establish the intent and operating boundaries deliberately, preserve decisions, and structure application logic so future infrastructure choices do not require avoidable rewrites.

[Explore new systems](new-systems.md)

## Three connected stages

### 1. Definition and readiness

Recover an existing system's effective contract or establish a new system's intent, constraints, consequences, acceptance conditions, and required capabilities.

[Explore readiness](readiness.md)

### 2. Build or modernization in bounded waves

Review topology and specifications before implementation, then deliver and accept one coherent outcome at a time.

### 3. Continuous assurance and evolution

Keep the knowledge alive. Maintain the behavioural baseline, decision history, verification profile, infrastructure capability map, and known debt so future changes do not restart from zero.

[Explore continuous assurance](continuous-assurance.md)

## Where this approach fits

This work is a strong fit when:

- the system is operationally or societally important;
- its behaviour is poorly documented or distributed across many sources;
- a platform, vendor, regulatory, or workforce trigger makes change unavoidable;
- a previous replacement stalled or failed;
- a new system will be long-lived, high-consequence, or subject to changing deployment requirements;
- the organisation wants application logic to remain portable across supported infrastructure backends;
- the organisation cannot safely use production as the place where missing requirements are discovered;
- stakeholders need a defensible basis for scope, acceptance, and investment.

It is unnecessary for a straightforward product, a well-bounded application replacement, or work where conventional delivery already provides sufficient confidence.

## Infrastructure portability is part of the product outcome

For systems built on Omnia, application logic compiles to WebAssembly components. Omnia runs each invocation in a fresh isolated instance; persistent state and external effects use typed capabilities supplied by the host runtime.

Where a supported alternative exists, the host backend can change without changing or recompiling guest application logic. This does not automate data migration or erase differences between services. It gives the client a cleaner boundary around infrastructure and reduces the amount of application code coupled to today's provider.

[Explore infrastructure portability](infrastructure-portability.md)

## Experienced people, assisted by purpose-built delivery infrastructure

Propellerhead has delivered complex software for government, commercial, and non-profit organisations since 2002. Our teams combine system discovery, architecture, product analysis, engineering, verification, and operational delivery.

We expect open-source models, agents, and skills to make capable coding assistance widely available. We welcome that shift. It makes human judgment, domain understanding, careful reflection, and accountability more—not less—important.

We use Emery, our delivery system, to keep source evidence, provenance, reviewed specifications, execution inputs, and delivery facts durable. We use Omnia to keep guest application logic separate from host infrastructure. Neither replaces professional judgment or client accountability.

[See how we work](how-we-work.md)  
[See selected work](work.md)

## Begin with the decision the system needs

For an existing system, the first useful decision is whether it can be understood well enough to define a responsible modernization wave. For a new system, it is whether the intent, consequences, acceptance, and operating boundaries are clear enough to begin.

Definition and readiness produce that decision and the evidence behind it.

**Action:** Discuss a critical system

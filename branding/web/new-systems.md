# Build a new critical system deliberately

Agent-assisted development can move from an idea to working code remarkably quickly. That is useful, but speed can hide decisions that the organisation will live with for years: whose needs define the system, what failure means, how acceptance will be judged, and which infrastructure choices become embedded in application logic.

Propellerhead builds new critical systems with the same discipline we bring to modernization: explicit intent, considered decisions, bounded delivery, evidence proportionate to risk, and a durable basis for future change.

**Primary action:** Discuss a new critical system

## Code is not the first irreversible decision

Before implementation creates momentum around one answer, we work with the people accountable for the system to establish:

- the outcome and the people affected by it;
- authoritative intent and material constraints;
- important failure modes and operating conditions;
- security, privacy, regulatory, and assurance obligations;
- the decisions that need human judgment;
- acceptance conditions and the evidence needed to support them;
- required infrastructure capabilities without prematurely binding them to one provider.

This is not a request to specify everything upfront. It is a way to identify the decisions that are expensive or dangerous to discover accidentally.

## Reflection is productive work

Software systems embody judgments about users, organisations, risk, and the future. A useful delivery process creates time to examine those judgments while they can still change.

Our review points are working sessions, not approval theatre. Practitioners, client stakeholders, operators, and relevant assurance specialists inspect the proposed boundary and refined specifications, challenge assumptions, and record material decisions before implementation proceeds under the engagement's change controls.

The aim is neither exhaustive certainty nor continuous hesitation. It is the smallest responsible boundary that a team can build, verify, and learn from.

## Open-source agents change where value sits

Open-source models, coding agents, skills, and evaluation tools will make capable software production available to far more organisations. We see that as progress.

As implementation capacity becomes abundant, durable value moves toward:

- choosing worthwhile problems;
- understanding context and consequences;
- making coherent architectural and product decisions;
- verifying results independently where risk warrants it;
- accepting responsibility for what enters production;
- leaving the system easier for the next team to understand and change.

We use agents to widen research, implementation, testing, and review capacity. We do not use them to replace stakeholder authority or professional accountability.

## Build in bounded waves

Each wave establishes:

- a coherent product and behavioural boundary;
- the decisions and evidence supporting it;
- explicit gaps and assumptions;
- required infrastructure capabilities;
- review and acceptance responsibilities;
- verification appropriate to the consequence of failure.

The agreed result becomes the basis for the next wave, so the system accumulates knowledge rather than only code.

## Keep application logic portable with Omnia

Where Omnia fits the system, business logic compiles to WebAssembly components that run in a fresh isolated instance for each invocation. Persistent state and external effects such as storage, messaging, identity, and observability are expressed through typed capabilities and supplied by the host runtime.

This separates what the application does from which supported infrastructure service performs the work. A backend can be substituted in the host without changing or recompiling guest application logic.

Portability is not automatic cloud migration. Data movement, service semantics, operational characteristics, and backend availability still require engineering. The benefit is a cleaner boundary: infrastructure change should demand less application rewrite.

[Explore infrastructure portability](infrastructure-portability.md)

## A good fit

This approach is strongest when a new system:

- will become operationally, commercially, or societally important;
- must remain understandable over a long life;
- has material security, regulatory, or assurance obligations;
- needs to run under client-controlled or changing infrastructure;
- will evolve across products, teams, and integration boundaries;
- would be expensive to rediscover after its original team has moved on.

Straightforward, reversible products do not need this weight. The method should match the consequence of getting the system wrong.

## Start with definition, not generated code

Bring us the outcome, the people it affects, the constraints already known, and the decisions that feel difficult. We will determine the smallest responsible first wave.

[Start a conversation](contact.md)

# Keep application logic independent of today's infrastructure

Infrastructure decisions change. Cloud strategies shift, managed services become expensive, residency requirements tighten, and clients need more control over where critical systems run.

When application logic imports a provider's SDK at every boundary, each infrastructure change becomes an application rewrite.

Omnia is designed to separate those concerns.

## Application logic in portable components

Application logic compiles to WebAssembly components. Omnia runs each invocation in a fresh isolated instance; persistent state and external effects remain behind host capabilities.

Domain operations declare the typed capabilities they need—such as configuration, HTTP, messaging, key-value state, identity, SQL, blob storage, or document storage—without embedding a concrete infrastructure client.

The native host runtime supplies implementations of those capabilities and retains the endpoints, credentials, and deployment configuration.

The result is a clear boundary:

- **guest component:** application and domain behaviour;
- **typed capability:** the infrastructure service the behaviour requires;
- **host backend:** the concrete technology selected for this deployment.

## Change the host, not the application

Where Omnia has a conforming backend, the host can substitute it without changing or recompiling guest application logic.

Examples include moving from development defaults to production services, or selecting among supported key-value, messaging, SQL, blob, document, identity, observability, and model backends.

This gives clients practical options:

- develop and test against lightweight local implementations;
- deploy against production infrastructure selected for the environment;
- change a supported provider with less application-code churn;
- keep credentials and connection details out of guest application code;
- exercise the same guest component through different backend implementations;
- retain more bargaining and deployment flexibility over the system's life.

## Portability supports considered architecture

Portability is not only a future migration feature. It changes design conversations now.

Teams can describe the capability the system needs before selecting the service that supplies it. Client stakeholders can consider cost, residency, security, operability, and strategic control without forcing those concerns into domain logic. Infrastructure choices remain explicit host decisions rather than accidental imports scattered through the application.

## A security boundary as well as an abstraction

Omnia guests run in a WebAssembly sandbox. They have no ambient filesystem, network, process, or environment access. External effects are available only through interfaces linked by the host, and persistent state remains behind host-controlled capabilities.

This does not remove the need for infrastructure security. Credentials must still be scoped, network policy remains an infrastructure concern, and a linked capability must be treated as a real grant. It makes the boundary visible and enforceable.

## What portability does not mean

Omnia does not promise that every service has a backend, that all providers behave identically, or that moving data is automatic.

A responsible infrastructure change still considers:

- availability of a conforming backend;
- data migration and consistency;
- service-specific semantics and limits;
- performance, resilience, and cost;
- identity, networking, and operational controls;
- verification against the real target service.

The precise promise is that supported infrastructure substitution does not require changing or recompiling guest application logic. Operational equivalence must still be demonstrated.

## How Emery and Omnia work together

Emery governs the delivery process: intent and evidence, reviewable specifications, visible gaps, exact execution inputs, verification, and durable change facts.

The Omnia target turns appropriate specifications into application components built around typed capabilities. Omnia then runs those components against host-selected infrastructure.

Emery helps preserve why the system was built. Omnia helps prevent where it runs from becoming inseparable from what it does.

## A client-valued outcome

Client feedback has identified this infrastructure portability as one of the most significant aspects of Propellerhead's offer. It should be designed and verified as an explicit product outcome, not treated as internal framework detail.

For an engagement using Omnia, the portability plan should state:

- which capabilities the application requires;
- which backends are available for development and production;
- which substitutions matter to the client;
- how backend conformance will be tested;
- which data or operational concerns remain provider-specific.

[Explore new systems](new-systems.md)  
[Explore modernization](modernization.md)

**Action:** Discuss a system that needs infrastructure options

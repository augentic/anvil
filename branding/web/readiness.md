# Know what you are committing to before critical delivery begins

A readiness engagement turns “we need to build or replace this critical system” into an evidence-backed decision about what can responsibly happen next.

It is a paid, bounded engagement. Its purpose is not to produce a generic assessment or a speculative target architecture. It establishes how well the system is understood, what evidence can support the change, which uncertainties affect delivery, and whether a credible first modernization wave can be defined.

**Primary action:** Discuss a readiness engagement

## Questions readiness answers

- What does the system appear to do today?
- For a new system, what outcome, affected people, constraints, and consequences define responsible delivery?
- Which behaviours are critical to users, operations, obligations, and connected systems?
- Where do code, documentation, runtime behaviour, tests, and stakeholder accounts disagree?
- Who has authority to resolve each material disagreement?
- Which important behaviours have verification independent of the implementation team?
- Which repositories, services, teams, vendors, and environments participate in the change?
- What would make a first delivery wave coherent and reviewable?
- Which unknowns must be resolved before pricing, and which can be carried explicitly?
- Is modernization feasible under the organisation's security, data, model, and deployment constraints?
- Which infrastructure capabilities are required, and which provider choices should remain replaceable?

## What we do

### Survey the context

For an existing system, we establish the relevant system, repository, interface, environment, and ownership topology. For a new system, we establish the outcome, stakeholders, operating context, constraints, dependencies, and required capabilities. The survey is deliberately bounded by the decision the engagement needs to support.

### Recover behavioural evidence

We inspect the evidence available: stated intent and policy for every system, plus source code, documentation, tests, interfaces, captures, and operational records where a system already exists. Different sources retain their identity so agreement and disagreement remain visible.

### Assess verification readiness

We identify what can be checked mechanically, what depends on candidate-authored tests, and where stronger independent evidence is needed. A missing verification path is reported as a gap, not converted into confidence by prose.

### Reconcile intent and authority

Observed behaviour may be essential, accidental, defective, or obsolete. We identify the stakeholders and policies able to decide what the modernized system should preserve.

### Define the first wave

We propose a bounded outcome with explicit inputs, dependencies, acceptance conditions, known gaps, and commercial assumptions. If the evidence does not support a responsible wave, that is itself a useful finding.

## What you receive

The exact artifacts depend on the estate, but the engagement is designed to produce:

- a system and change topology;
- an inventory of critical behaviours or intended outcomes and their supporting evidence;
- a reviewable behavioural or requirements baseline for the proposed scope;
- a record of conflicts, unknowns, decisions, and verification gaps;
- repository, runtime, security, and delivery readiness findings;
- a build or modernization roadmap expressed as bounded outcomes;
- a defined first wave with assumptions and acceptance boundaries;
- a commercial recommendation for the next stage.

These are working assets, not a presentation that becomes obsolete when delivery begins.

## What readiness does not promise

Definition and readiness do not settle every question about a new system or recover every fact about a large estate. They do not declare observed behaviour correct merely because it exists. They do not remove programme risk or guarantee a fixed price.

It reduces the uncertainty that matters to the next investment decision and preserves the remainder honestly.

## Typical triggers

Readiness is useful when:

- the board or executive team needs a credible modernization decision;
- procurement needs a defensible scope before approaching delivery partners;
- a programme has produced plans but little confidence in acceptance;
- estimates vary because teams disagree about what the current system does;
- a deadline is approaching and the organisation needs to identify the safest first boundary;
- a failed rewrite needs to be re-grounded in evidence;
- a new critical system needs considered product, assurance, and infrastructure boundaries before implementation accelerates;
- the organisation wants to assess whether AI-assisted delivery is appropriate under its governance constraints.

## Starting conditions

We normally need access to a representative set of code, documentation, environments, operational knowledge, and stakeholders. We agree data handling, model access, confidentiality, and evidence-retention constraints before analysis begins.

The engagement can start narrow. A single critical journey or service is often enough to test the method and reveal the shape of the wider estate.

## Practical engagement shape

Before commencement, the written scope establishes:

- the decision readiness must support and the system boundary it will examine;
- the evidence sources and environments Propellerhead may access;
- the executive sponsor, technical lead, subject-matter experts, operators, and assurance stakeholders the client will make available;
- review dates and who can decide material questions of intent;
- what information may be processed by AI-assisted tools, through which approved deployment, and under what retention conditions;
- outputs, duration, fee, and the treatment of assumptions or scope changes.

No client material is submitted to a model merely because it is technically accessible. If the available model deployment cannot satisfy the agreed data, residency, or retention constraints, that material is excluded from model-assisted analysis; if the exclusion prevents a responsible result, the engagement does not proceed on the proposed basis.

Readiness artifacts belong to the client and can be handed to another delivery partner. Tool-specific files may accompany them, but the decision record, findings, evidence inventory, and proposed boundaries must remain reviewable without adopting Emery.

[Editorial decision required before publication: add a tested typical duration and price range rather than inventing one before the offer has been exercised.]

## The decision at the end

The outcome should support one of three honest decisions:

1. proceed with a defined first build or modernization wave;
2. perform specific remediation or evidence collection before delivery;
3. stop or reframe the programme because the current boundary is not responsibly deliverable.

All three are more valuable than beginning a rewrite on assumptions.

**Action:** Bring us the system you cannot yet price with confidence

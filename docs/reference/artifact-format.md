# Artifact Format

This is the definitive reference for the structure and conventions of Specify's four artifacts. For a high-level overview, see [Artifacts](../orientation/artifacts.md).

## Spec files (behavioral "what")

One spec file per capability, at `specs/<name>/spec.md`.

Specs are behavioral. They describe what the system must do, not how it should be implemented in a particular framework.

### Baseline / new capability format

New specs and merged baselines use a flat requirement format:

````markdown
# <Capability Name> Specification

## Purpose

<1-2 sentence description of what this capability does>

### Requirement: <Behavior Name>

ID: REQ-001

The system SHALL <behavioral description>.
Source: <source function or design section>

#### Scenario: <Happy Path>

- **WHEN** <trigger or input>
- **THEN** <expected behavior>

#### Scenario: <Error Case>

- **WHEN** <invalid input or failing condition>
- **THEN** <expected error behavior>

## Error Conditions

- <error type>: <description and trigger conditions>

## Metrics

- `<metric_name>` -- type: <counter|gauge|histogram>; emitted: <when>
````

Key rules:

- Each requirement has a stable `ID: REQ-XXX` line. This is the merge key used during `/spec:merge`.
- Scenarios use WHEN/THEN format. GIVEN is optional context.
- Error conditions are listed at the end as a cross-cutting summary.
- Metrics are only included when they are explicit in the source material.

### Delta spec format (modified capability)

When modifying an existing capability, specs use operation headers to describe what changed:

````markdown
# <Capability Name> Specification

## Purpose

<updated purpose if changed>

## ADDED Requirements

### Requirement: <New Behavior>

ID: REQ-005

The system SHALL <new behavioral description>.

#### Scenario: <Happy Path>
...

## MODIFIED Requirements

### Requirement: <Changed Behavior>

ID: REQ-002

The system SHALL <updated behavioral description>.

#### Scenario: <Updated Scenario>
...

## REMOVED Requirements

### Requirement: <Removed Behavior>

ID: REQ-003

Reason: <why this requirement is being removed>

## RENAMED Requirements

### Requirement: <New Name> (was: <Old Name>)

ID: REQ-004
````

The four operation sections are `## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements`, and `## RENAMED Requirements`. All are optional -- include only the sections relevant to the change.

The stable `ID: REQ-XXX` line is the merge key, not the requirement title. Requirement titles may change across deltas; IDs must not.

## Design document (technical "how")

`design.md` carries the technical shape needed to implement the change.

### Format

````markdown
# Technical Design

## Context

- Source: <TypeScript component path | design document>
- Purpose: <component or change summary>
- Source paths: <analyzed files, if applicable>

## Domain Model

## API Contracts

## External Services

## Constants & Configuration

## Business Logic

## Publication & Timing Patterns

## Implementation Constraints

## Source Capabilities Summary

## Dependencies

## Risks / Open Questions

## Notes
````

### When to create a full design

Create a full design if any of the following apply:

- Cross-cutting change (multiple services or modules) or new architectural pattern
- New external dependency or significant data model changes
- Security, performance, or migration complexity
- Ambiguity that benefits from technical decisions before coding

If none apply, create a minimal `design.md` noting that a full design is not warranted and referencing the proposal and specs.

For multi-capability changes, structure the document with capability-specific sections (`## Crate: <name>` or equivalent) each containing the relevant subsections.

### Business logic tags

Tags classify business logic blocks in the design:

| Tag | Use case | Example |
|-----|----------|---------|
| `[domain]` | Business rules and validation | Validate order total matches line items |
| `[infrastructure]` | External calls or persistence | Fetch data from API or publish to queue |
| `[mechanical]` | Data transformations | Parse JSON or map fields |
| `[unknown]` | Explicitly unresolved detail | Dependency behavior not specified |

### Unknown tokens

Use explicit unknown markers instead of guessing:

| Token | When to use |
|-------|-------------|
| `unknown -- not specified in source` | The source material does not say |
| `unknown -- ambiguous requirement` | The requirement is unclear or conflicting |
| `unknown -- inferred from context` | Best-effort summary, not explicitly stated |
| `unknown -- open question` | The design material marks it as unresolved |

### Design ownership

The design document is where project-specific technical detail belongs: domain models, type shapes, API contracts, message shapes, business logic, external integrations, configuration, risks, and migration notes. Generator-owned binding decisions (e.g. Omnia trait composition, Crux effect types) remain in specialist skills and references.

When design sections reference behavior from specs, cite the stable requirement IDs (e.g. `REQ-003`) rather than relying on requirement titles.

## Proposal document

`proposal.md` captures why the change exists and what is in scope. The schema's brief file provides the full output template.

The **Crates** (or **Capabilities**) section creates the contract between proposal and specs phases -- each capability listed will need a corresponding spec file at `specs/<name>/spec.md`.

Keep proposals concise (one to two pages). Focus on the "why" not the "how" -- implementation details belong in the design.

## Tasks document

`tasks.md` is an implementation checklist, not a requirements document.

### Format

```markdown
## 1. Implementation

- [ ] 1.1 Refine proposal/spec/design artifacts
- [ ] 1.2 Implement code changes via `/spec:build`
- [ ] 1.3 Verify and review output
```

### Rules

- Group related tasks under `##` numbered headings.
- Each task MUST be a checkbox: `- [ ] X.Y Task description`. The build phase parses this format to track progress.
- Tasks should be small enough to complete in one session.
- Order tasks by dependency (what must be done first).
- Reference specs for what needs to be built, design for how to build it.
- Each task should be verifiable -- you know when it is done.

### Skill directive tags

Tasks may include a skill directive as an HTML comment. The build phase parses these tags and delegates the task to the named specialist skill:

```markdown
- [ ] 2.1 Generate the domain crate <!-- skill: omnia:crate-writer -->
- [ ] 2.2 Generate test suites <!-- skill: omnia:test-writer -->
- [ ] 2.3 Manual integration step
```

Tasks without a skill tag are implemented via the schema's default build instruction.

## Validation checklists

### Behavioral specs

- One spec file per capability
- Each spec has Purpose, flat Requirement blocks, stable `ID: REQ-XXX` lines, Scenarios, and Error Conditions
- Specs stay behavioral and avoid platform-binding detail
- Traceability is present for each requirement via stable IDs

### Technical design

- `design.md` captures domain model, APIs, business logic, integrations, and configuration
- Unknowns are marked explicitly with unknown tokens
- Technical decisions live in design, not in specs

### Tasks

- `tasks.md` exists when `/spec:build` depends on it
- Tasks are implementation steps and checkpoints only
- Every task uses checkbox format (`- [ ]`)

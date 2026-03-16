# Specify Guidance Supplement

This repository uses stock Specify as the executable workflow contract. This document is a repository-specific supplement describing how Augentic specialists use `proposal.md`, `spec.md`, `design.md`, and `tasks.md` during `/spec:define -> /spec:build -> /spec:promote`, with `/spec:drop` available when a change should be discarded instead of promoted and `/spec:verify` available to detect drift between code and baseline specs. Artifact validation is performed automatically by `/spec:build` before implementation begins.

## Overview

Specify artifacts split change intent into four human-facing layers:

| Artifact | Purpose |
| --- | --- |
| `proposal.md` | Why the change exists, what is in scope, and which capabilities are affected |
| `specs/*/spec.md` | Behavioral requirements only: what the system must do |
| `design.md` | Technical shape and decisions needed to implement the behavior |
| `tasks.md` | Implementation sequencing and checkpoints |

Specialist skills in this repo consume those artifacts, but they should not redefine the Specify runtime contract.

## Artifact Lifecycle

Artifacts move through the normal Specify lifecycle:

1. `.specify/changes/<change>/` holds the working change.
2. `.specify/specs/` holds the promoted baseline specs.
3. `.specify/changes/archive/` holds finalized changes, including promoted and dropped changes.

The human workflow is:

```text
/spec:define -> /spec:build -> /spec:promote
/spec:define -> /spec:drop
/spec:build  -> /spec:drop
/spec:verify (anytime -- compare code against baseline specs)
```

## Artifact Locations

```text
$PROJECT_DIR      = <workspace root>
$CHANGE_DIR       = $PROJECT_DIR/.specify/changes/<change-name>
$SPECS_DIR        = $CHANGE_DIR/specs
$DESIGN_PATH      = $CHANGE_DIR/design.md
$PROPOSAL_PATH    = $CHANGE_DIR/proposal.md
$TASKS_PATH       = $CHANGE_DIR/tasks.md
$BASELINE_SPECS   = $PROJECT_DIR/.specify/specs
```

## Spec Files (Behavioral "What")

One spec file per capability or crate, at `specs/<name>/spec.md`.

Specs are behavioral. They should not encode Omnia trait bindings, WASM implementation details, or generator-specific instructions.

### Spec File Format (Baseline / New Crate)

New crate specs and promoted baselines use a flat requirement format. The
hard-coded spec format (`plugins/spec/references/spec-format.md`) defines
the requirement, scenario, and delta-operation headings used by all
downstream skills.

```markdown
# <Crate Name> Specification

## Purpose

<1-2 sentence description of what this crate or capability does>

### Requirement: <Behavior Name>

ID: REQ-001

The system SHALL <behavioral description>.
Source: <source function, JIRA story, or design section>

#### Scenario: <Happy Path>

- **WHEN** <trigger or input>
- **THEN** <expected behavior>

#### Scenario: <Error Case>

- **WHEN** <invalid input or failing condition>
- **THEN** <expected error behavior>

## Error Conditions

- <error type>: <description and trigger conditions>

## Metrics

- `<metric_name>` — type: <counter|gauge|histogram>; emitted: <when>
```

### Delta Spec Format (Modified Crate)

When modifying an existing crate, delta specs use the operation headers
defined in the spec format (`## ADDED Requirements`,
`## MODIFIED Requirements`, `## REMOVED Requirements`,
`## RENAMED Requirements`). Requirement
blocks still use `### Requirement:` and `#### Scenario:` headings, but the
stable merge key is the `ID: REQ-XXX` line rather than the display name.
See the schema's `instructions/specs.md` for the full delta structure and the
promote skill for how deltas merge into the baseline.

### Deriving Specs From Source Code (code-analyzer)

Create a consolidated spec file from the source behavior:

1. Purpose from the role of the handler or function.
2. Requirements from distinct business rules, assigning stable IDs in spec order (`REQ-001`, `REQ-002`, ...).
3. Scenarios from happy paths, edge cases, and failures.
4. Error conditions from observed failure behavior.
5. Metrics only when they are explicit in the source.

### Deriving Specs From JIRA (epic-analyzer)

Create or update spec files from user stories and acceptance criteria:

1. Purpose from story summaries.
2. Requirements from acceptance criteria, assigning stable IDs in spec order (`REQ-001`, `REQ-002`, ...).
3. Scenarios from BDD or equivalent examples.
4. Error conditions from explicit failure or validation requirements.
5. Traceability back to JIRA stories or criteria, with requirement IDs available for design and test references.

## Design Document (Technical "How")

`design.md` carries the technical shape needed to implement the change. It may reference constraints relevant to generation, but it should not hardcode target-specific bindings as part of the behavioral contract. When design sections refer to behavior from specs, cite the stable requirement IDs (for example, `REQ-003`) rather than relying on requirement titles staying unchanged.

### Design Document Format

````markdown
# Technical Design

## Context

- Source: <TypeScript component path | JIRA epic key | design document>
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

### Design Ownership

`design.md` is where Augentic-specific technical detail belongs:

- domain models and type shapes
- API contracts and message shapes
- business logic pseudocode
- external integrations
- configuration
- risks, trade-offs, and migration notes

Generator-owned binding decisions such as Omnia trait composition remain in specialist skills and references.

## Proposal Document

Use `proposal.md` to capture why the change exists and what is in scope.

The Omnia schema uses **Crates** (`New Crates` / `Modified Crates`). The
Realtime schema uses **Crates** (`New Crates` / `Modified Crates`).
The schema-specific proposal instruction determines which heading names
to use. Both map to `specs/<name>/spec.md`.

```markdown
## Why

<problem or opportunity>

## Source

- **Repository**: URL of the repository to migrate
- **Epic**: JIRA/ADO/Linear epic key
- **Manual**: Requirements described directly in this proposal

## What Changes

- <scoped change>

## Crates (Omnia) / Capabilities (Realtime)

### New Crates / New Capabilities

- `name`: one-line description

### Modified Crates / Modified Capabilities

- `existing-name`: one-line description

## Impact

- affected services, APIs, dependencies, or teams
```

## Tasks Document

Use `tasks.md` as an implementation checklist, not as another requirements or design document.

```markdown
## 1. Implementation

- [ ] 1.1 Refine proposal/spec/design artifacts
- [ ] 1.2 Implement code changes via `/spec:build`
- [ ] 1.3 Verify and review output
```

Tasks should describe sequencing, checkpoints, and ownership. They should not introduce new behavioral requirements.

### Skill Directive Tags

Tasks may optionally include a skill directive as an HTML comment. The build phase parses these tags and delegates the task to the named specialist skill instead of following the default build instruction.

```markdown
- [ ] 2.1 Generate the domain crate <!-- skill: omnia:crate-writer -->
- [ ] 2.2 Generate test suites <!-- skill: omnia:test-writer -->
- [ ] 2.3 Manual integration step
```

Tasks without a skill tag are implemented via the schema's default build instruction (mode detection, verification loop, etc.). Use skill tags when a task maps directly to a single specialist skill invocation.

## Tags Reference

Tags are used in `design.md` business logic blocks.

| Tag | Use Case | Example |
| --- | --- | --- |
| `[domain]` | Business rules and validation | Validate order total matches line items |
| `[infrastructure]` | External calls or persistence | Fetch data from API or publish to queue |
| `[mechanical]` | Data transformations | Parse JSON or map fields |
| `[unknown]` | Explicit unresolved detail | Dependency behavior not specified |

## Unknown Tokens Reference

Use explicit unknown markers instead of guessing.

| Token | When to Use |
| --- | --- |
| `unknown — not specified in source` | The source material does not say |
| `unknown — ambiguous requirement` | The requirement is unclear or conflicting |
| `unknown — inferred from context` | Best effort summary, not explicitly stated |
| `unknown — open question` | The design material marks it as unresolved |

## Validation Checklists

### Behavioral Specs

- [ ] One spec file per capability or crate
- [ ] Each spec has Purpose, flat Requirement blocks, stable `ID: REQ-XXX` lines, Scenarios, and Error Conditions
- [ ] Specs stay behavioral and avoid platform-binding detail
- [ ] Traceability is present for each requirement and can refer to its stable ID

### Technical Design

- [ ] `design.md` captures the domain model, APIs, business logic, integrations, and configuration
- [ ] Unknowns are marked explicitly
- [ ] Technical decisions live in design, not in specs

### Tasks

- [ ] `tasks.md` exists when `/spec:build` depends on it
- [ ] Tasks are implementation steps and checkpoints only

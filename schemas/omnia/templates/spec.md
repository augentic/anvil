## Template: New Crate

Use this template for crates listed under **New Crates** in the proposal.
This format matches what code-analyzer and epic-analyzer produce, and is
what crate-writer and test-writer expect as input.

```markdown
# <Crate Name> Specification

## Handler: <handler-name>

### Purpose

<1-2 sentence description of what this handler does>

### Requirements

#### Requirement: <Behavior Name>

The system SHALL <behavioral description>.

##### Scenario: <Happy Path>

- **WHEN** <trigger or input>
- **THEN** <expected behavior>

##### Scenario: <Error Case>

- **WHEN** <invalid input or failing condition>
- **THEN** <expected error behavior>

### Error Conditions

- <error type>: <description and trigger conditions>

### Metrics

- `<metric_name>` — type: <counter|gauge|histogram>; emitted: <when>
```

Repeat `## Handler:` sections for each handler in the crate.

## Template: Modified Crate

Use this template for crates listed under **Modified Crates** in the
proposal. This delta format describes changes to an existing baseline spec.

```markdown
## ADDED Requirements

### Requirement: <!-- requirement name -->
<!-- requirement text -->

#### Scenario: <!-- scenario name -->
- **WHEN** <!-- condition -->
- **THEN** <!-- expected outcome -->

## MODIFIED Requirements

### Requirement: <!-- existing requirement name (must match baseline) -->
<!-- full updated requirement text -->

#### Scenario: <!-- scenario name -->
- **WHEN** <!-- condition -->
- **THEN** <!-- expected outcome -->

## REMOVED Requirements

### Requirement: <!-- existing requirement name -->
**Reason**: <!-- why this requirement is being removed -->
**Migration**: <!-- how to handle the removal -->

## RENAMED Requirements

FROM: <!-- old requirement name -->
TO: <!-- new requirement name -->
```

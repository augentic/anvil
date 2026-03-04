# Specifications

Create or update spec files for each capability affected by this change.

## Instructions

1. Read the approved proposal
2. Read existing specs in `openspec/specs/` for affected capabilities
3. For each capability in the proposal:

### New capability (no existing spec)
Create `specs/<capability-name>/spec.md` with:
- Purpose statement
- Requirements (each prefixed with +)
- BDD scenarios in Given/When/Then format
- Provider capabilities needed
- Error conditions and expected responses

### Modified capability (existing spec)
Copy the existing spec and apply changes:
- Changed requirements show old line with - and new line with +
- New requirements prefixed with +
- Unchanged requirements left as-is

### Removed capability (existing spec being deleted)
Copy the existing spec with all requirements prefixed with -

## Format per spec file

# <capability-name> Specification

## Purpose
<what this capability does>

## Requirements

### Requirement: <name>
<+ or - prefix> The system SHALL <requirement text>.
  Source: <JIRA story key or design doc section>

#### Scenario: <name>
- Given: <precondition>
- When: <action>
- Then: <expected outcome>

## Provider Capabilities
- <trait>: <why needed>

## Error Conditions
- <condition>: <expected error response>

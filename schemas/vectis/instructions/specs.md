# Vectis Specs Instructions

Create specification files that define WHAT the system should do.

The source for Crux projects is always Manual. Create one spec file per
module listed in the proposal's Modules section.

---

**New Modules**: Use the exact kebab-case name from the proposal
(`specs/<module>/spec.md`). The spec format depends on the module type:

### Core module specs

Core module specs describe behavioral requirements for the Crux shared
crate. Map the application's features and business rules into the
Specify requirement format:

```markdown
# <Module Name> Specification

## Purpose

<1-2 sentence description of what this module does>

### Requirement: <Feature or Behavior Name>

ID: REQ-001

The system SHALL <behavioral description>.

#### Scenario: <Happy Path>

- **WHEN** <trigger or input>
- **THEN** <expected behavior>

#### Scenario: <Error Case>

- **WHEN** <invalid input or failing condition>
- **THEN** <expected error behavior>

## Error Conditions

- <error type>: <description and trigger conditions>
```

Guidance for core module specs:

- Each user-facing **feature** (add item, delete item, toggle state)
  becomes a `### Requirement:` block with at least one scenario.
- Each **business rule** (validation, constraints, edge cases) becomes
  a requirement with scenarios showing valid and invalid inputs.
- **Capabilities** (HTTP, KV, SSE, Time, Platform) belong in design.md,
  not in specs. Specs describe *what* the system does, not *how*.
- **Data Model** details (field names, types) belong in design.md.
  Specs reference domain concepts by name without defining their
  internal structure.
- **Views** (screens the user sees) become requirements describing
  what the user sees and when. View structure details belong in design.
- Include requirements for page transitions (Loading -> Main, Error
  -> retry) and navigation behavior.

### iOS shell module specs

iOS shell specs describe platform-specific behavioral requirements
beyond what the core spec covers:

```markdown
# <Module Name> iOS Shell Specification

## Purpose

<Which core module this shell renders, and where the shell lives>

### Requirement: <Platform Behavior>

ID: REQ-001

The iOS shell SHALL <platform-specific behavioral description>.

#### Scenario: <Platform Interaction>

- **WHEN** <iOS-specific trigger (swipe, pull-to-refresh, haptic)>
- **THEN** <expected platform behavior>
```

Guidance for iOS shell specs:

- Navigation style (single, stack, tabs) becomes a requirement.
- Per-screen iOS-specific interactions (swipe actions, pull-to-refresh,
  toolbar items) become requirements with scenarios.
- Platform features (haptics, share sheet) become requirements.
- Design system overrides become requirements if they affect behavior.
- Do NOT duplicate core module requirements — reference the core spec
  for business logic.

### Design system module specs

Design system specs capture token change requirements:

```markdown
# <Module Name> Design System Specification

## Purpose

<What token changes are being made and why>

### Requirement: <Token Change>

ID: REQ-001

The design system SHALL <token change description>.

#### Scenario: <Token Application>

- **WHEN** <the token is applied>
- **THEN** <expected visual outcome>
```

---

Repeat `### Requirement:` blocks for each distinct behavior,
incrementing `ID: REQ-XXX` for each new requirement.

**Modified Modules**: Use the existing spec folder name from
`.specify/specs/<module>/` when creating the delta spec at
`specs/<module>/spec.md`. Follow this structure:

```markdown
## ADDED Requirements

### Requirement: <!-- requirement name -->
ID: REQ-<!-- next available id -->
<!-- requirement text -->

#### Scenario: <!-- scenario name -->
- **WHEN** <!-- condition -->
- **THEN** <!-- expected outcome -->

## MODIFIED Requirements

### Requirement: <!-- existing requirement name -->
ID: REQ-<!-- existing id (must match baseline) -->
<!-- full updated requirement text -->

#### Scenario: <!-- scenario name -->
- **WHEN** <!-- condition -->
- **THEN** <!-- expected outcome -->

## REMOVED Requirements

### Requirement: <!-- existing requirement name -->
ID: REQ-<!-- existing id -->
**Reason**: <!-- why this requirement is being removed -->
**Migration**: <!-- how to handle the removal -->

## RENAMED Requirements

ID: REQ-<!-- existing id -->
TO: <!-- new requirement name -->
```

Follow the spec format conventions defined in the define skill for
delta operations, format rules, and the MODIFIED/ADDED workflows.

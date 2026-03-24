# Vectis Specs Instructions

Create specification files that define WHAT the system should do.

The source for Crux projects is always Manual. Create one spec file per
feature listed in the proposal's Features section.

Each spec file is organized as a single document covering the feature's
behavioral requirements. Core (platform-neutral) requirements form the
main body. Platform-specific requirements go in dedicated sections at
the end of the file.

---

**New Features**: Use the exact kebab-case name from the proposal
(`specs/<feature>/spec.md`).

### Core requirements (main body)

The main body of the spec describes platform-neutral behavioral
requirements — what the app does regardless of which shell renders it.
These requirements drive the Crux shared crate implementation.

```markdown
# <Feature Name> Specification

## Purpose

<1-2 sentence description of what this feature does>

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

Guidance for core requirements:

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

### iOS Shell Requirements section

If the proposal lists `ios` in Platforms, add a `## iOS Shell
Requirements` section after the core requirements. This section captures
iOS-specific behavioral requirements beyond what the core spec covers.

Continue sequential numbering from the last core requirement ID. All
requirements share one flat `REQ-[0-9]{3}` namespace — do not use
platform prefixes like `REQ-IOS-xxx`.

```markdown
## iOS Shell Requirements

### Requirement: <Platform Behavior>

ID: REQ-<next>

The iOS shell SHALL <platform-specific behavioral description>.

#### Scenario: <Platform Interaction>

- **WHEN** <iOS-specific trigger (swipe, pull-to-refresh, haptic)>
- **THEN** <expected platform behavior>
```

Guidance for iOS shell requirements:

- Navigation style (single, stack, tabs) becomes a requirement.
- Per-screen iOS-specific interactions (swipe actions, pull-to-refresh,
  toolbar items) become requirements with scenarios.
- Platform features (haptics, share sheet) become requirements.
- Design system overrides become requirements if they affect behavior.
- Do NOT duplicate core requirements — reference the core spec
  for business logic.

### Android Shell Requirements section

If the proposal lists `android` in Platforms, add a `## Android Shell
Requirements` section. Continue sequential `REQ-XXX` numbering.

### Design System Requirements section

If the proposal lists `design-system` in Platforms and the feature
involves token changes, add a `## Design System Requirements` section.
Continue sequential `REQ-XXX` numbering.

```markdown
## Design System Requirements

### Requirement: <Token Change>

ID: REQ-009

The design system SHALL <token change description>.

#### Scenario: <Token Application>

- **WHEN** <the token is applied>
- **THEN** <expected visual outcome>
```

---

Repeat `### Requirement:` blocks for each distinct behavior,
incrementing `ID: REQ-XXX` sequentially across all sections (core,
iOS, Android, design-system share one namespace).

**Modified Features**: Use the existing spec folder name from
`.specify/specs/<feature>/` when creating the delta spec at
`specs/<feature>/spec.md`. Follow this structure:

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

Delta operations apply to all sections. Platform requirements can be
added, modified, or removed using delta operations with their `REQ-XXX`
IDs (e.g., `REQ-008`).

Follow the spec format conventions defined in the define skill for
delta operations, format rules, and the MODIFIED/ADDED workflows.

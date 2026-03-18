Create the proposal document that establishes WHY this change is needed.

Sections:
- **Why**: 1-2 sentences on the problem or opportunity. What problem does
  this solve? Why now?
- **Source**: Always **Manual** for Crux projects. Requirements are
  described directly in the proposal and subsequent artifacts.
- **What Changes**: Bullet list of changes. Be specific about new
  capabilities, modifications, or removals. Mark breaking changes with
  **BREAKING**.
- **Modules**: Identify which modules will be created or modified.
  Each module has a type that determines which skills are used:
  - **core** — Rust Crux shared crate (business logic, state, effects).
    Uses `vectis:core-writer` for generation and `vectis:core-reviewer`
    for review.
  - **ios-shell** — SwiftUI iOS shell for an existing Crux core. Uses
    `vectis:ios-writer` for generation and `vectis:ios-reviewer` for
    review. Requires a core module to already exist.
  - **design-system** — VectisDesign Swift package generated from
    tokens.yaml. Uses `vectis:design-system-writer`.
  - **New Modules**: List modules being introduced. Each
    becomes a new `specs/<name>/spec.md`. Use kebab-case names
    (e.g., `todo-core`, `todo-ios`, `design-tokens`).
  - **Modified Modules**: List existing modules whose
    REQUIREMENTS are changing. Only include if spec-level behavior
    changes (not just implementation details). Each needs a delta spec
    file. Check `.specify/specs/` for existing spec names. Leave empty if
    no requirement changes.
- **Impact**: Affected code, APIs, dependencies, or systems.

IMPORTANT: The Modules section creates the contract between proposal
and specs phases. Research existing specs before filling this in — each
module listed will need a corresponding spec file.

Keep it concise (1-2 pages). Focus on the "why" not the "how" -
implementation details belong in design.md.

This is the foundation - specs, design, and tasks all build on this.

## Output Structure

```markdown
## Why

<!-- Explain the motivation for this change. What problem does this solve? -->

## Source

Manual

## What Changes

<!-- Describe what will change. Be specific about new capabilities or modifications. -->

## Modules

### New Modules

<!-- List modules being introduced with their type. Each becomes a new specs/<name>/spec.md.
Use kebab-case names.

Example:
- **todo-core** (core) — Crux shared crate for the todo application.
- **todo-ios** (ios-shell) — SwiftUI shell for the todo app.
- **design-tokens** (design-system) — Updated design tokens. -->

### Modified Modules

<!-- List existing modules whose REQUIREMENTS are changing.
Use existing spec folder names from .specify/specs/.
Leave empty if no requirement changes. -->

## Impact

<!-- Affected code, APIs, dependencies, systems.
Call out risks such as cross-module contract changes, breaking changes,
complexity concerns -->
```

Create the proposal document that establishes WHY this change is needed.

Sections:
- **Why**: 1-2 sentences on the problem or opportunity. What problem does
  this solve? Why now?
- **Source**: Always **Manual** for Crux projects. Requirements are
  described directly in the proposal and subsequent artifacts.
- **What Changes**: Bullet list of changes. Be specific about new
  capabilities, modifications, or removals. Mark breaking changes with
  **BREAKING**.
- **Features**: Identify which features will be created or modified.
  Each feature describes a business capability — not a software component.
  Name features after what the app does (e.g., `todo-app`,
  `weather-forecast`, `user-settings`), not after implementation layers
  (avoid names like `todo-core` or `todo-ios`).
  - **New Features**: List features being introduced. Each
    becomes a new `specs/<name>/spec.md`. Use kebab-case names.
  - **Modified Features**: List existing features whose
    REQUIREMENTS are changing. Only include if spec-level behavior
    changes (not just implementation details). Each needs a delta spec
    file. Check `.specify/specs/` for existing spec names. Leave empty if
    no requirement changes.
- **Platforms**: Declare which platforms this change targets. This
  determines which skills the build phase invokes:
  - **core** — always required. The Rust Crux shared crate containing
    all business logic. Uses `vectis:core-writer` for generation and
    `vectis:core-reviewer` for review.
  - **ios** — SwiftUI iOS shell. Uses `vectis:ios-writer` for generation
    and `vectis:ios-reviewer` for review. Requires a core to exist.
  - **android** — Android shell (future).
  - **web** — Web shell (future).
  - **design-system** — VectisDesign Swift package generated from
    tokens.yaml. Uses `vectis:design-system-writer`.
- **Impact**: Affected code, APIs, dependencies, or systems.

IMPORTANT: The Features section creates the contract between proposal
and specs phases. Research existing specs before filling this in — each
feature listed will need a corresponding spec file. Platforms determine
implementation scope, not spec scope — a single feature spec covers all
platforms.

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

## Features

### New Features

<!-- List features being introduced. Each becomes a new specs/<name>/spec.md.
Use kebab-case names that describe the business capability.

Example:
- **todo-app** — Todo list with CRUD, persistence, and sync.
- **weather-forecast** — 5-day weather forecast with offline caching. -->

### Modified Features

<!-- List existing features whose REQUIREMENTS are changing.
Use existing spec folder names from .specify/specs/.
Leave empty if no requirement changes. -->

## Platforms

<!-- Which platforms this change targets. Determines which build skills run.
core is always required. Add others as applicable.

Example:
- core
- ios
- design-system -->

## Impact

<!-- Affected code, APIs, dependencies, systems.
Call out risks such as cross-platform contract changes, breaking changes,
complexity concerns -->
```

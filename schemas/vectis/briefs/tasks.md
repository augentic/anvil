---
id: tasks
description: Create the task list that breaks down the implementation work
generates: tasks.md
needs: [specs, design]
---

Follow the task format conventions defined in the define skill for checkbox format, grouping, ordering, and skill directive tags.

Tasks are organized by build phase, not by feature. All features in the change share a single task list ordered: design-system first, core second, shells last.

Each task references the single feature spec at `specs/<feature>/spec.md`. The spec contains both core requirements and any platform-specific requirements in dedicated sections.

## Agent-Completable Constraint

Generate only tasks that an agent can complete and verify with code or local tooling. Do not generate manual mobile app testing, real-world API, production credentials, visual inspection, physical-device-only, app store review, or user-confirmation tasks.

When mobile or external-service behavior must be verified, express it as an agent-verifiable task:

- Use `vectis:core-writer` or `vectis:test-writer` patterns to add Rust core tests, mocked effect tests, and fixture-backed API behavior checks.
- Use `vectis:ios-writer` and `vectis:android-writer` for shell implementation tasks, followed by build or test commands available to the agent.
- Use local fixture-backed contract tests instead of connecting the mobile app to a real-world API.
- Use reviewer skills for code review instead of human inspection.

Invalid: `Manually test the iOS and Android apps against the real API`.
Valid: `Add fixture-backed effect tests covering the API success and failure responses, then verify iOS and Android shells build against the generated core`.

## Available Skills

| Directive                      | Skill                              | When to Use                            |
| ------------------------------ | ---------------------------------- | -------------------------------------- |
| `vectis:core-writer`           | Generate or update Crux core       | Core implementation tasks              |
| `vectis:test-writer`           | Generate or update test suites     | Test generation tasks                  |
| `vectis:core-reviewer`         | AI code review for Crux core       | Post-implementation review of core     |
| `vectis:ios-writer`            | Generate or update iOS shell       | iOS shell implementation tasks         |
| `vectis:ios-reviewer`          | AI code review for iOS shell       | Post-implementation review of iOS      |
| `vectis:android-writer`        | Generate or update Android shell   | Android shell implementation tasks     |
| `vectis:android-reviewer`      | AI code review for Android shell   | Post-implementation review of Android  |
| `vectis:design-system-writer`  | Regenerate iOS + Android design system from tokens | Design system generation tasks         |

## Composition Awareness

When a `composition.yaml` exists in the change directory, express the dependency between shell tasks and the composition artifact in the task ordering:

- Shell writer tasks (`vectis:ios-writer`, `vectis:android-writer`) depend on `composition.yaml` when present. When composition validation fails, the corresponding shell task is blocked.
- When `composition.yaml` is absent, shell writers fall back to inference and no composition-related blocking applies.

This is not a hard requirement — pre-RFC-7 changes have no composition artifact.

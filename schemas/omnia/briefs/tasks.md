---
id: tasks
description: Create the task list that breaks down the implementation work
generates: tasks.md
needs: [specs, design]
---

Follow the task format conventions defined in the define skill for checkbox format, grouping, ordering, and skill directive tags.

## Agent-Completable Constraint

Generate only tasks that an agent can complete and verify with code or local tooling. Do not generate manual verification, real-world API, production credentials, visual inspection, or user-confirmation tasks.

When external behavior must be verified, express it as an agent-verifiable task:

- Use `omnia:test-writer` to add MockProvider, fixture-backed, or contract-aligned tests for API and side-effect behavior.
- Use build tasks for `cargo check`, `cargo test`, `cargo clippy`, and WASM target builds through the build brief's verify-repair loop.
- Use `omnia:code-reviewer` for post-implementation review instead of human review tasks.

## Available Skills

| Directive             | Skill                           | When to Use                |
| --------------------- | ------------------------------- | -------------------------- |
| `omnia:guest-writer`  | Generate WASM guest project     | New crate, first task      |
| `omnia:crate-writer`  | Generate or update domain crate | Crate implementation tasks |
| `omnia:test-writer`   | Generate or update test suites  | Test generation tasks      |
| `omnia:code-reviewer` | AI code review                  | Post-implementation review |

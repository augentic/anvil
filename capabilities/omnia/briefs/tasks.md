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

## Self-Review

After drafting `tasks.md`, re-read every checkbox line and ask, for each task:

1. Could a coding agent perform this action using code, tooling, mocks, fixtures, contract validators, build commands, or one of the reviewer skills available below?
2. If the task mentions humans, manual steps, visual inspection, real services, app store review, or user confirmation, is the action genuinely avoiding them (e.g. "without manual testing") or genuinely requiring them? Requiring them is a rewrite. Avoiding them is fine, but prefer to omit the reference entirely so future readers don't have to parse the negation.
3. Does the list as a whole include at least one task that verifies outcomes — `omnia:test-writer`, `omnia:code-reviewer`, fixture-backed tests, or build/check steps?

Rewrite any task that fails (1) or (2) before handing the file off. If (3) fails, add a verification task using a skill from the table below.

For `tasks.md`, `specify slice validate` checks checkbox/grouping shape only — it does not inspect task intent. Agent-completability is judged here at write-time and re-checked by `/spec:build` as a preflight.

## Available Skills

| Directive             | Skill                           | When to Use                |
| --------------------- | ------------------------------- | -------------------------- |
| `omnia:guest-writer`  | Generate WASM guest project     | New crate, first task      |
| `omnia:crate-writer`  | Generate or update domain crate | Crate implementation tasks |
| `omnia:test-writer`   | Generate or update test suites  | Test generation tasks      |
| `omnia:code-reviewer` | AI code review                  | Post-implementation review |

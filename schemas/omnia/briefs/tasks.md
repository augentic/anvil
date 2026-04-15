---
id: tasks
description: Create the task list that breaks down the implementation work
generates: tasks.md
needs: [specs, design]
---

Follow the task format conventions defined in the define skill for
checkbox format, grouping, ordering, and skill directive tags.

## Available Skills

| Directive             | Skill                           | When to Use                |
| --------------------- | ------------------------------- | -------------------------- |
| `omnia:guest-writer`  | Generate WASM guest project     | New crate, first task      |
| `omnia:crate-writer`  | Generate or update domain crate | Crate implementation tasks |
| `omnia:test-writer`   | Generate or update test suites  | Test generation tasks      |
| `omnia:code-reviewer` | AI code review                  | Post-implementation review |

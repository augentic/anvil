Create the task list that breaks down the implementation work.

**IMPORTANT: Follow the format below exactly.** The apply phase parses
checkbox format to track progress. Tasks not using `- [ ]` won't be
tracked.

Guidelines:
- Group related tasks under ## numbered headings
- Each task MUST be a checkbox: `- [ ] X.Y Task description`
- Tasks should be small enough to complete in one session
- Order tasks by dependency (what must be done first?)

Example:

```markdown
## 1. Setup
- [ ] 1.1 Create new module structure
- [ ] 1.2 Add dependencies to Cargo.toml

## 2. Core Implementation
- [ ] 2.1 Implement data export function
- [ ] 2.2 Add CSV formatting utilities
```

Reference specs for what needs to be built, design for how to build it.
Each task should be verifiable - you know when it's done.

## Skill Directives (Optional)

Tasks may include an HTML comment tag that names a specialist skill to
invoke during apply. The apply phase parses these tags and delegates
the task to the referenced skill instead of following the default
apply instruction.

Format: `- [ ] X.Y Task description <!-- skill: plugin:skill-name -->`

Available skills for Realtime:

| Directive             | Skill                           | When to Use                |
| --------------------- | ------------------------------- | -------------------------- |
| `omnia:guest-writer`  | Generate WASM guest project     | New crate, first task      |
| `omnia:crate-writer`  | Generate or update domain crate | Crate implementation tasks |
| `omnia:test-writer`   | Generate or update test suites  | Test generation tasks      |
| `omnia:code-reviewer` | AI code review                  | Post-implementation review |

Tasks without a skill tag are implemented via the default apply
instruction (mode detection, verification loop, etc.). Use skill tags
when a task maps cleanly to a single specialist skill invocation.

# Artifact-writing conventions

This file collects the format conventions, delta workflows, and self-review obligations the define skill applies when generating spec, design, and tasks artifacts. The SKILL.md body keeps the algorithmic spine; the depth lives here so a single read covers every artifact format.

## Spec format conventions

Follow the heading conventions in [`spec-format.md`](./spec-format.md) and the baseline / delta format in [`specify.md`](./specify.md) (Spec Files section). The instruction file provides templates and workflow routing; these conventions govern the content written into those templates.

### Delta-specific workflows (modified-crate specs)

**MODIFIED requirements:**

1. Locate the existing requirement in `.specify/specs/<crate>/spec.md`.
2. Copy the **entire** requirement block (from `### Requirement:` through all scenarios), including the `ID:` line.
3. Paste under the MODIFIED heading and edit to reflect the new behavior.
4. Preserve the original `ID:` value exactly.

**ADDED requirements:**

1. Inspect `.specify/specs/<crate>/spec.md` for the highest existing requirement ID.
2. Assign the next sequential ID to the new requirement block.
3. Do not reuse IDs from removed requirements.

**Common pitfalls:**

- Using MODIFIED with partial content loses detail at merge time.
- If adding new concerns without changing existing behavior, use ADDED instead.

## Design writing guidance

Follow the design format and decision criteria in [`specify.md`](./specify.md) (Design Document section, including "When To Create A Full Design"). The instruction file provides the output template.

## Task format conventions

Follow the task format and guidelines in [`specify.md`](./specify.md) (Tasks Document section). The instruction file provides the available-skills table per adapter. The build phase parses checkbox format to track progress.

### Agent-completable task invariant

Every generated task **must** be executable and verifiable by an agent using code, local tooling, mocks, fixtures, contract validators, build commands, or reviewer skills. Never generate tasks that depend on:

- manual app testing,
- real-world API credentials,
- visual inspection,
- physical-device-only checks,
- app-store review, or
- asking the user to verify behavior.

If a requirement appears to call for human validation, encode the equivalent code-based test or scripted verification task instead.

After writing `tasks.md`, complete the **Self-Review** step in the adapter's `tasks` brief: re-read every checkbox in context and rewrite any task that fails the agent-completability check. For `tasks.md`, `specify slice validate` checks checkbox / grouping shape only — it does not inspect task intent, so agent-completability must be judged here at write-time (and is re-checked by `/spec:build` as a preflight).

### Skill directives (optional)

Tasks may include an HTML comment tag that names a specialist skill to invoke during build. The build phase parses these tags and delegates the task to the referenced skill instead of following the default build instruction.

Format:

```text
- [ ] X.Y Task description <!-- skill: plugin:skill-name -->
```

Tasks without a skill tag are implemented via the default build instruction (mode detection, verification loop, etc.). Use skill tags when a task maps cleanly to a single specialist-skill invocation. The instruction file lists available skills per adapter.

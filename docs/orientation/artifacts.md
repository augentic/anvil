# Artifacts

Every Specify change produces four interdependent artifacts. Together they form the contract between human intent and agent execution.

## The four artifacts

| Artifact | Question it answers | Location |
|----------|-------------------|----------|
| `proposal.md` | *Why* does this change exist? What is in scope? | `.specify/changes/<name>/proposal.md` |
| `spec.md` | *What* must the system do? (behavioral requirements) | `.specify/changes/<name>/specs/<capability>/spec.md` |
| `design.md` | *How* will the behavior be implemented? | `.specify/changes/<name>/design.md` |
| `tasks.md` | In what *sequence* should it be built? | `.specify/changes/<name>/tasks.md` |

### Proposal

The proposal captures motivation and scope. It names the capabilities (crates, features) that the change will affect. Each capability listed in the proposal will need a corresponding spec file.

Proposals are concise -- one to two pages. They focus on the "why" and the "what", not the "how".

### Specs

Specs are behavioral. Each capability gets its own spec file at `specs/<name>/spec.md`. A spec contains:

- A **purpose** statement.
- **Requirements** with stable IDs (`REQ-001`, `REQ-002`, ...).
- **Scenarios** for each requirement (WHEN/THEN format).
- **Error conditions**.
- Optional **metrics**.

Specs stay platform-neutral -- they describe what the system must do, not how it should be implemented in a particular framework or language.

When modifying an existing capability, specs use a **delta format** with `ADDED`, `MODIFIED`, `REMOVED`, and `RENAMED` sections. The stable `REQ-XXX` IDs serve as merge keys.

### Design

The design document captures the technical shape needed for implementation: domain models, API contracts, business logic, external integrations, configuration, and risks. It references the proposal for motivation and the specs for behavioral requirements (by stable requirement ID).

Business logic blocks are tagged to indicate their nature:

| Tag | Meaning |
|-----|---------|
| `[domain]` | Business rules and validation |
| `[infrastructure]` | External calls or persistence |
| `[mechanical]` | Data transformations |
| `[unknown]` | Explicitly unresolved detail |

### Tasks

The task list is an implementation checklist. Each task is a checkbox (`- [ ]`) that the build phase marks off as it completes work. Tasks may carry **skill directive tags** that route them to specialist skills:

```markdown
- [ ] 2.1 Generate the domain crate <!-- skill: omnia:crate-writer -->
- [ ] 2.2 Generate test suites <!-- skill: omnia:test-writer -->
- [ ] 2.3 Manual integration step
```

Tasks describe sequencing and checkpoints. They do not introduce new requirements -- those belong in specs.

## Artifact lifecycle

Artifacts move through three locations:

1. **Working change** -- `.specify/changes/<name>/` holds the active change and its artifacts.
2. **Baseline** -- `.specify/specs/` holds the merged specs that represent the current known state of the system.
3. **Archive** -- `.specify/archive/YYYY-MM-DD-<name>/` holds finalized changes (both merged and dropped) for audit.

When you run `/spec:merge`, the change's spec deltas are applied to the baseline. The baseline grows over time, giving future changes a foundation to build on and enabling `/spec:verify` to detect drift between your code and your specifications.

For full format details, see the [Artifact Format](../reference/artifact-format.md) reference.

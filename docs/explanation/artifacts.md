# Artifacts

Every Specify change produces a set of interdependent artifacts. Together they form the contract between human intent and agent execution.

## Artifact dependency chain

The define pipeline generates artifacts in dependency order. Each artifact builds on the ones before it:

```d2
direction: right

proposal: "proposal.md\n(why)" {shape: page}
specs: "spec.md\n(what)" {shape: page}
contracts: "contracts/\n(shape)" {shape: page}
design: "design.md\n(how)" {shape: page}
tasks: "tasks.md\n(sequence)" {shape: page}

proposal -> specs: "scopes capabilities"
specs -> contracts: "alignment check"
contracts -> design: "shapes referenced"
specs -> design: "behavioral input"
design -> tasks: "what to build"
```

The Vectis schema inserts a `composition.yaml` stage between contracts and design.

## Core artifacts

All schemas produce these four artifacts:

| Artifact | Question it answers | Location |
|----------|-------------------|----------|
| `proposal.md` | *Why* does this change exist? What is in scope? | `.specify/changes/<name>/proposal.md` |
| `spec.md` | *What* must the system do? (behavioral requirements) | `.specify/changes/<name>/specs/<capability>/spec.md` |
| `design.md` | *How* will the behavior be implemented? | `.specify/changes/<name>/design.md` |
| `tasks.md` | In what *sequence* should it be built? | `.specify/changes/<name>/tasks.md` |

## Schema-specific artifacts

Some schemas add artifacts to the define pipeline. The Vectis schema adds:

| Artifact | Question it answers | Location |
|----------|-------------------|----------|
| `composition.yaml` | *Where* does each element appear on screen? | `.specify/changes/<name>/composition.yaml` |

The composition artifact describes the spatial layout of each screen -- regions (`header`, `body`, `footer`, `fab`), container structure (`group` nodes with flexbox-like properties), item placement, data bindings, and event wiring. It sits between specs and design in the define pipeline: specs define behavior, composition defines visual arrangement, design defines the type system. Shell writers (`ios-writer`, `android-writer`) consume composition for deterministic layout rather than inferring structure from the ViewModel.

## Contract artifacts

Contract artifacts capture the machine-readable shapes of APIs and message interfaces -- the *structure* of interactions that behavioral specs describe in prose. They use three standard formats:

| Format | Purpose | Location |
|--------|---------|----------|
| JSON Schema | Payload definitions (domain types) | `contracts/schemas/<type>.yaml` |
| OpenAPI 3.1 | HTTP endpoint bindings | `contracts/http/<domain>-api.yaml` |
| AsyncAPI 3.0 | Messaging bindings | `contracts/messages/<domain>-events.yaml` |

Contracts complement specs: specs describe *what* the system does; contracts describe *what the interfaces look like*. Neither replaces the other -- both are needed for requirements traceability and machine-readable integration.

Contract artifacts live in two locations:

- **Baseline:** `.specify/contracts/` -- the current platform contract vocabulary, co-located with `registry.yaml` and `plan.yaml`.
- **Per-change delta:** `.specify/changes/<name>/contracts/` -- proposed additions or replacements for this change only.

Contracts are a platform concern -- they describe interfaces *between* components, not internals of any one project. Both producer and consumer reference the same central contracts.

The composition artifact supports two modes:
- **Skeleton mode** -- regions and layout structure without data bindings. Produced by external tools (Figma adapters, legacy extractors) or manual authoring.
- **Wired mode** -- the same regions enriched with `bind`, `event`, and `maps_to` keys. Produced by the define pipeline.

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

When you run `/spec:merge`, the change's spec deltas are applied to the baseline. For Vectis changes, composition deltas are also merged into the baseline `composition.yaml` alongside spec files. When the change includes contract artifacts, they are copied into `.specify/contracts/` using opaque file replacement -- each file is replaced wholesale rather than delta-merged. The baseline grows over time, giving future changes a foundation to build on.

For full format details, see the [Artifact Format](../reference/artifact-format.md) reference.

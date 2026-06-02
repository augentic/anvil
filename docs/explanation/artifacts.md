# Artifacts

Every Specify slice produces a set of interdependent artifacts. Together they form the contract between human intent and agent execution.

<div class="callout">
  <strong>Read order.</strong> Start with the dependency chain, then the core four artifacts table below.
</div>

## Artifact dependency chain

The define pipeline generates artifacts in dependency order. Each artifact builds on the ones before it:

<div class="pipeline">

![Artifact dependency chain](../assets/diagrams/artifacts/dependency-chain.svg)

<p class="pipeline-caption">proposal → spec → contracts → design → tasks; Vectis adds composition.yaml between contracts and design.</p>
</div>

The Vectis adapter inserts a `composition.yaml` stage between contracts and design.

## Core artifacts

All adapters produce these four artifacts:

| Artifact | Question it answers | Location |
|----------|-------------------|----------|
| `proposal.md` | *Why* does this slice exist? What is in scope? | `.specify/slices/<name>/proposal.md` |
| `spec.md` | *What* must the system do? (behavioral requirements) | `.specify/slices/<name>/specs/<adapter>/spec.md` |
| `design.md` | *How* will the behavior be implemented? | `.specify/slices/<name>/design.md` |
| `tasks.md` | In what *sequence* should it be built? | `.specify/slices/<name>/tasks.md` |

## Adapter-specific artifacts

Some adapters add artifacts to the define pipeline. The Vectis adapter adds:

| Artifact | Question it answers | Location |
|----------|-------------------|----------|
| `composition.yaml` | *Where* does each element appear on screen? | `.specify/slices/<name>/composition.yaml` |

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

- **Baseline:** `contracts/` at the repo root -- the current platform contract vocabulary, co-located with `registry.yaml` and `plan.yaml`.
- **Per-slice delta:** `.specify/slices/<name>/contracts/` -- proposed additions or replacements for this slice only.

Contracts are a platform concern -- they describe interfaces *between* components, not internals of any one project. Both producer and consumer reference the same central contracts.

The composition surface ships as two sibling artifacts, both validated against the same JSON Schema:
- **`layout.yaml` (unwired layout input)** -- regions and layout structure with token / asset references but no data bindings. Produced by layout inferers (the [`screenshots` source adapter](../../adapters/sources/screenshots/adapter.yaml) is the first-party producer; future Figma and source-code inferers reuse the same contract) or hand-authored before the define pipeline runs. Validated by `specrun tool run vectis -- validate layout`.
- **`composition.yaml` (wired lifecycle artifact)** -- the same regions enriched with `bind`, `event`, `maps_to`, overlay `trigger`, navigation, and `*-when` keys. Produced by the define pipeline and consumed by shell writers. Validated by `specrun tool run vectis -- validate composition` (which auto-invokes `tokens` / `assets` modes when sibling manifests exist).

### Proposal

The proposal captures motivation and scope. It names the adapters (crates, features) that the slice will affect. Each adapter listed in the proposal will need a corresponding spec file.

Proposals are concise -- one to two pages. They focus on the "why" and the "what", not the "how".

### Specs

Specs are behavioral. Each adapter gets its own spec file at `specs/<name>/spec.md`. A spec contains:

- A **purpose** statement.
- **Requirements** with stable IDs (`REQ-001`, `REQ-002`, ...).
- **Scenarios** for each requirement (WHEN/THEN format).
- **Error conditions**.
- Optional **metrics**.

Specs stay platform-neutral -- they describe what the system must do, not how it should be implemented in a particular framework or language.

When modifying an existing adapter, specs use a **delta format** with `ADDED`, `MODIFIED`, `REMOVED`, and `RENAMED` sections. The stable `REQ-XXX` IDs serve as merge keys.

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

### Decision Records

A slice may author zero or more **Decision Records** -- the durable *why* behind a design choice plus the alternatives it rejected. Each is a YAML front-matter header plus a Nygard-shaped Markdown body (`## Context` / `## Decision` / `## Consequences`) at `.specify/slices/<name>/decisions/<slug>.md`. The slice author writes `slug` and `status` (`accepted` / `rejected`) only; `/spec:merge` promotes each record into the append-only baseline catalogue at `.specify/decisions/DEC-NNNN-<slug>.md`, assigning the durable project-global `DEC-NNNN` id.

Decision Records store the *why*, never design *state*: domain models and API shapes stay in `design.md` and the code. Unlike `design.md` (a per-slice artifact archived with its slice), accepted decisions accumulate into a reviewable, diffable baseline -- the design counterpart to what `.specify/specs/` is for behaviour. They are opt-in; a slice that takes no notable decision authors none. See the [Decision Records section of the artifact-writing conventions](../../plugins/spec/references/artifact-conventions.md) for the authored format.

## Artifact lifecycle

Artifacts move through three locations:

1. **Working slice** -- `.specify/slices/<name>/` holds the active slice and its artifacts.
2. **Baseline** -- `.specify/specs/` holds the merged specs that represent the current known state of the system; `.specify/decisions/` holds the append-only catalogue of accepted (and superseded) Decision Records.
3. **Archive** -- `.specify/archive/YYYY-MM-DD-<name>/` holds finalized slices (both merged and dropped) for audit.

When you run `/spec:merge`, the slice's spec deltas are applied to the baseline. For Vectis slices, composition deltas are also merged into the baseline `composition.yaml` alongside spec files. When the slice includes contract artifacts, they are copied into the root-level `contracts/` directory using opaque file replacement -- each file is replaced wholesale rather than delta-merged. Any Decision Records the slice authored are promoted into `.specify/decisions/` by the same opaque-add strategy -- whole-file add with an engine-assigned `DEC-NNNN` id, never a prose delta-merge -- and a newer record's `supersedes:` flips its named targets to `status: superseded`. The baseline grows over time, giving future slices a foundation to build on. Accepted decisions also sharpen the project's routing identity at plan time (a third axis beside *what the project does* and *what recently changed*).

For full format details, see the [Artifact Format](../reference/artifact-format.md) reference.

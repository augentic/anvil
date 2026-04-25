# RFC-8: API Contracts

> Status: Draft · Depends: [RFC-1](archive/rfc-1-cli.md), [RFC-2](archive/rfc-2-execution.md), [RFC-3](archive/rfc-3a-monoliths.md)

## Abstract

Introduce machine-readable API contracts as a first-class artifact in the Specify define pipeline. Contracts capture the interface shapes that behavioral specs describe — request/response payloads, message envelopes, error types — in a format that tooling can validate and generate code from. JSON Schema defines the shared payload vocabulary; OpenAPI and AsyncAPI provide protocol-specific bindings for HTTP endpoints and messaging respectively.

The integration is schema-level: a new `contracts` brief is added to the define pipeline between `specs` and `design`. No CLI changes are required for the initial implementation. The existing pipeline machinery (`specify schema pipeline`, `specify validate`, delta merge) handles the new artifact without modification in Layer 1; baseline tracking and cross-repo contract sharing are layered on top.

## Motivation

Specify specs are behavioral requirements. They describe *what* a system must do — `WHEN/THEN` scenarios, `SHALL` statements, error conditions — but not the *shape* of the interfaces those behaviors expose. The Omnia `design.md` brief has sections for `## API Contracts` and `## Publication & Timing Patterns`, but these are prose: useful for human context, useless for machine validation or code generation.

This gap matters in two scenarios:

1. **Single-repo services.** A WASM service's API surface is implicit in the code. When specs change, there is no machine-readable artifact that captures the *before* and *after* of the API shape. Consumers discover breaking changes at integration time, not at define time.

2. **Multi-repo initiatives.** A mobile frontend and a WASM backend communicate over HTTP and/or messaging. RFC-3b explicitly defers cross-repo spec references (`@peer:capability`) and states that "API contracts, auth libraries, protocol definitions are a framework-level concern addressed by the platform's own dependency management and build tooling, not by Specify's code-generation pipeline." This is the right scope boundary for *behavioral* specs, but it leaves a gap: there is no Specify-managed artifact that captures the interface contract between components. Change ordering via `depends-on` edges ensures sequencing but not compatibility.

Machine-readable contracts close both gaps. Within a single repo, they make the API surface explicit and diffable. Across repos, they provide a shared artifact that both producer and consumer can validate against.

### Why not `@peer:capability`?

The deferred `@peer:capability` syntax from RFC-3b would let one spec reference a capability in another repo. This is a *behavioral* cross-reference — "this capability depends on that capability" — not an interface contract. Two repos could have perfectly complementary behavioral specs and still produce incompatible HTTP interfaces. Interface compatibility requires wire-level detail (endpoint paths, payload schemas, status codes, message topics, envelope formats) that behavioral specs deliberately do not encode.

`@peer:capability` remains useful for planning and ordering. API contracts are orthogonal: they describe the *shape* of the interface, not the *behavior* behind it.

## Design

### Format choice

The contract format must cover both HTTP endpoints and messaging payloads. The practical options:

| Format | HTTP | Messaging | Payload schemas | Code-gen ecosystem |
|---|---|---|---|---|
| JSON Schema (payload only) | Shapes only | Shapes only | Native | Broad |
| OpenAPI 3.1 | Full | No | JSON Schema subset | Broad |
| AsyncAPI 3.0 | No | Full | JSON Schema subset | Growing |
| OpenAPI + AsyncAPI | Full | Full | JSON Schema (shared) | Both |
| Smithy | Full | Full | Native IDL | AWS-centric |
| Protobuf / gRPC | gRPC only | Via schema | Native | Broad |

This RFC adopts **JSON Schema as the shared payload vocabulary** with **OpenAPI 3.1** and **AsyncAPI 3.0** as protocol-specific bindings. The rationale:

- **JSON Schema is the common denominator.** Both OpenAPI 3.1 and AsyncAPI 3.0 use JSON Schema for payload definitions. Defining domain types as JSON Schema files means both protocol bindings reference a single source of truth. A `UserRegistration` type used in both an HTTP response and a message payload is defined once.

- **Separation of concerns.** The payload shape ("what does a `UserRegistration` look like?") is a different concern from the transport binding ("this shape arrives via `POST /users`" or "this shape is published on `user.registered`"). Keeping them separate avoids duplication and makes it clear which part of the contract is shared versus protocol-specific.

- **Rust code generation.** `schemars` + `typify` can generate Rust types from JSON Schema. OpenAPI generators (`progenitor`) produce Rust client/server stubs. AsyncAPI has emerging Rust tooling. The Omnia and Vectis build pipelines can consume these artifacts directly.

- **No new IDL.** JSON Schema, OpenAPI, and AsyncAPI are widely adopted standards with mature tooling. Introducing a proprietary contract format or a less common IDL (Smithy, Protobuf) would narrow the ecosystem without clear benefit.

### Artifact structure

The `contracts` brief generates files under `.specify/changes/<name>/contracts/`:

```text
contracts/
├── schemas/                    # JSON Schema payload definitions
│   ├── user-registration.yaml
│   ├── order-placed.yaml
│   └── error-response.yaml
├── http/                       # OpenAPI binding (when specs describe HTTP)
│   └── api.yaml                # $ref → ../schemas/
└── messages/                   # AsyncAPI binding (when specs describe messaging)
    └── events.yaml             # $ref → ../schemas/
```

Directory rules:

- **`contracts/schemas/`** is always present. Every contract generates at least one payload schema.
- **`contracts/http/`** is present when the specs describe HTTP interactions (REST endpoints, request/response patterns). Omitted when the capability is purely event-driven.
- **`contracts/messages/`** is present when the specs describe messaging interactions (pub/sub, event-driven, queue-based). Omitted when the capability is purely synchronous HTTP.
- Both `http/` and `messages/` may be present for capabilities that expose both transport types.

### Pipeline placement

The `contracts` brief sits between `specs` and `design` in the define pipeline:

```yaml
pipeline:
  define:
    - id: proposal
      brief: briefs/proposal.md
    - id: specs
      brief: briefs/specs.md
    - id: contracts
      brief: briefs/contracts.md
    - id: design
      brief: briefs/design.md
    - id: tasks
      brief: briefs/tasks.md
```

The ordering is deliberate:

1. **`specs` → `contracts`**: Contracts are derived from behavioral specs. The specs brief establishes *what* the system does; the contracts brief captures the *interface shapes* those behaviors imply. The `contracts` brief declares `needs: [specs]`.
2. **`contracts` → `design`**: The design document references contracts rather than re-describing API shapes in prose. The `design` brief declares `needs: [proposal, contracts]` (adding `contracts` to its existing `needs`). The `## API Contracts` and `## Publication & Timing Patterns` sections in `design.md` become pointers to the generated contract files rather than hand-authored descriptions.
3. **`contracts` → `tasks`**: Task generation can reference contracts for code-generation tasks (e.g. "generate Rust types from `contracts/schemas/`"). The `tasks` brief already declares `needs: [specs, design]`; adding `contracts` is optional — design transitively carries the contract context.

### Brief frontmatter

```yaml
---
id: contracts
description: Generate machine-readable API and message contracts from behavioral specs
generates: contracts/**/*.yaml
needs: [specs]
---
```

The `generates` glob (`contracts/**/*.yaml`) follows the same pattern as the specs brief's `specs/**/*.md` — the brief determines how many files to create and where within the pattern.

### Contract generation rules

The `contracts` brief body instructs the agent to:

1. **Read all spec files** under `.specify/changes/<name>/specs/` and identify requirements that describe API interactions (HTTP endpoints, request/response patterns) or message exchanges (pub/sub, event-driven patterns).

2. **Extract domain types.** For each requirement's scenarios, identify the data shapes: request payloads, response payloads, message envelopes, error types. Each distinct shape becomes a JSON Schema file under `contracts/schemas/`.

3. **Generate JSON Schema files.** Each schema file defines one domain type with:
   - `$id` for stable cross-referencing
   - `title` matching the type name
   - `description` from the spec's behavioral description
   - `properties`, `required`, and type constraints derived from scenario data
   - `$ref` pointers for shared sub-types

4. **Generate OpenAPI spec** (when applicable). For capabilities whose specs describe HTTP interactions, produce `contracts/http/api.yaml` with:
   - Paths and methods derived from spec scenarios
   - Request/response schemas as `$ref` pointers to `../schemas/`
   - Error responses derived from the spec's error conditions
   - OpenAPI 3.1 format (native JSON Schema support)

5. **Generate AsyncAPI spec** (when applicable). For capabilities whose specs describe messaging interactions, produce `contracts/messages/events.yaml` with:
   - Channels and operations derived from spec scenarios
   - Message payload schemas as `$ref` pointers to `../schemas/`
   - AsyncAPI 3.0 format (native JSON Schema support)

6. **Validate internal consistency.** All `$ref` pointers in OpenAPI and AsyncAPI files must resolve to files under `contracts/schemas/`. The agent verifies this before completing the brief.

### Relationship to `design.md`

The Omnia `design.md` brief currently has:

```markdown
## API Contracts
<!-- Endpoints with method, path, request/response shapes, errors -->

## Publication & Timing Patterns
<!-- Topics, message shapes, timing, partition keys -->
```

With the `contracts` brief in place, these sections change role. Instead of being the primary description of interface shapes, they become **references** to the generated contract files with additional implementation-level context (e.g. rate limits, retry policies, authentication schemes) that the contract format does not capture:

```markdown
## API Contracts

See `contracts/http/api.yaml` for the full OpenAPI specification.

<!-- Implementation notes: auth, rate limits, caching, versioning strategy -->

## Publication & Timing Patterns

See `contracts/messages/events.yaml` for the full AsyncAPI specification.

<!-- Implementation notes: ordering guarantees, retry policies, DLQ strategy -->
```

The `design` brief's `needs` gains `contracts` so the agent has the generated files available when writing design.md.

## Multi-repo contract sharing

### Layer 1: Ordering via `depends-on` (no framework changes)

In a multi-repo initiative, the plan sequences changes so that the producer's contracts are generated before the consumer's define phase begins. The consumer's specs and design reference the producer's contracts from the workspace clone:

```yaml
# plan.yaml
changes:
  - name: user-api
    project: backend
    description: "Define and implement the user registration API."
    status: pending

  - name: registration-screen
    project: mobile
    description: "Build the registration screen consuming the user API."
    depends-on: [user-api]
    status: pending
```

After `user-api` completes its define-build-merge cycle, the contracts are available in the backend's baseline at `.specify/specs/user-api/contracts/` (if contracts are merged into baseline — see Layer 2) or in the archived change at `.specify/archive/`. The mobile project's define phase can read these contracts from the workspace clone at `.specify/workspace/backend/specs/user-api/contracts/`.

This requires no framework changes — it uses existing `depends-on` ordering and the workspace materialisation from RFC-3a/3b. The consumer's brief reads the producer's contracts as context; compatibility is validated by the agent during the consumer's define phase rather than by automated tooling.

### Layer 2: Contracts in the baseline (requires merge extension)

To make contracts a persistent, diffable artifact, extend the merge system to track contract files alongside specs in the baseline:

```text
.specify/specs/<capability>/
├── spec.md                     # behavioral spec (existing)
└── contracts/                  # API contracts (new)
    ├── schemas/
    │   └── *.yaml
    ├── http/
    │   └── api.yaml
    └── messages/
        └── events.yaml
```

When `specify merge` processes a change, it copies the `contracts/` directory from the change into the baseline alongside `spec.md`. Contract files are treated as **opaque replacements** per capability — unlike spec files which use the ADDED/MODIFIED/REMOVED delta format, contract files are replaced wholesale on merge. The rationale: JSON Schema and OpenAPI/AsyncAPI files have their own versioning semantics (schema `$id`, OpenAPI `info.version`); introducing a second delta-merge algorithm for YAML contract files would add complexity without clear benefit over replacement.

`specify spec preview` and `specify spec conflict-check` are extended to include contract files in their output so operators see when a merge will update contracts.

This layer requires CLI changes:

- `specify merge` copies `contracts/` into baseline alongside spec files.
- `specify spec preview` reports contract file changes (added/replaced/removed).
- `specify spec conflict-check` detects when baseline contracts have been modified after the change was defined.

### Layer 3: Automated contract validation (future)

A future extension could add automated validation that a consumer's usage of a contract is compatible with the producer's definition — for example, verifying that a mobile frontend's API client calls match the backend's OpenAPI spec, or that a message consumer's expected payload matches the producer's AsyncAPI schema. This is analogous to contract testing (Pact, Spring Cloud Contract) but integrated into the Specify workflow.

This layer is explicitly deferred. It requires:

- A `specify contract validate` CLI verb that loads contracts from workspace clones and checks compatibility.
- A contract-validation brief in the build pipeline that runs after code generation.
- A definition of "compatibility" that accounts for backwards-compatible changes (additive fields, optional-to-required transitions, etc.).

The workspace already materialises peer baselines under `.specify/workspace/<peer>/specs/`, so the read path exists. The validation rules can be designed against real examples when the need arises — the same deferral logic RFC-3b applies to `@peer:capability`.

## Schema integration

### New schema via `extends`

The contracts brief is not added to the base `omnia` or `vectis` schemas directly. Instead, contract-aware variants are created using schema composition:

```yaml
# schemas/omnia-contracts/schema.yaml
name: omnia-contracts
version: 1
extends: https://github.com/augentic/specify/schemas/omnia
description: Omnia with API contract generation

pipeline:
  define:
    - id: proposal
      brief: briefs/proposal.md
    - id: specs
      brief: briefs/specs.md
    - id: contracts
      brief: briefs/contracts.md
    - id: design
      brief: briefs/design.md
    - id: tasks
      brief: briefs/tasks.md
```

The child schema overrides the `define` pipeline to insert the `contracts` entry. `build` and `merge` pipelines are inherited from the parent. The `contracts.md` brief and the updated `design.md` brief (which adds `contracts` to its `needs`) live in the child schema's `briefs/` directory; all other briefs fall back to the parent via the `extends` resolution algorithm documented in `plugins/spec/references/schema-resolution.md`.

A `vectis-contracts` schema follows the same pattern, extending `vectis` and inserting the `contracts` brief after `specs` (and before `composition` in Vectis's pipeline).

### Why not modify the base schemas?

Not every project needs machine-readable contracts. A single-crate WASM service with no external consumers may never expose an API that warrants a formal contract. Making contracts opt-in via schema composition keeps the base schemas lean and avoids generating unused artifacts. Projects that need contracts select the `omnia-contracts` or `vectis-contracts` schema at init time; projects that don't, use the base schema unchanged.

## Non-goals

- **Inventing a new IDL.** The contract format uses JSON Schema, OpenAPI, and AsyncAPI — existing standards with existing tooling. No proprietary schema language.
- **Automated code generation from contracts.** The build phase *can* use contracts as input for code generation (via skill directives in tasks), but this RFC does not prescribe how. Code generation strategies differ by schema (Omnia generates Rust; Vectis generates Rust + Swift + Kotlin); the contract-to-code mapping belongs in the build brief and specialist skills, not in the contract artifact itself.
- **Contract versioning or backwards-compatibility checking.** Semantic versioning of contracts, breaking-change detection, and consumer compatibility validation are deferred to Layer 3. The initial implementation treats contracts as define-time artifacts that are generated fresh each change.
- **Replacing behavioral specs.** Contracts complement specs; they do not replace them. Specs describe *what* the system must do. Contracts describe *what the interface looks like*. Both are needed — one for requirements traceability, the other for machine-readable integration.
- **Cross-repo contract enforcement in Layer 1.** The initial implementation relies on `depends-on` ordering and agent judgment for cross-repo compatibility. Automated validation is explicitly deferred.

## Implementation scope

### Layer 1 (no CLI changes)

1. **`contracts` brief** — author `briefs/contracts.md` with the generation rules described above.
2. **Updated `design` brief** — modify `briefs/design.md` to declare `needs: [proposal, contracts]` and update the `## API Contracts` / `## Publication & Timing Patterns` sections to reference generated contract files.
3. **Child schemas** — create `schemas/omnia-contracts/` and `schemas/vectis-contracts/` with `extends` and the `contracts` pipeline entry.
4. **Fixture** — author a worked example under `schemas/omnia-contracts/fixtures/` showing the generated contract tree for a representative capability.

Layer 1 is independently useful: it produces machine-readable contracts during define that the agent and human can review, and that the build phase can consume for code generation.

### Layer 2 (CLI changes)

5. **`specify merge` extension** — copy `contracts/` into baseline alongside spec files during merge. Contract files are replaced wholesale per capability (no delta merge).
6. **`specify spec preview` extension** — include contract file changes in the preview output.
7. **`specify spec conflict-check` extension** — detect baseline contract modifications after the change's `defined-at` timestamp.
8. **`specify validate` extension** — check that `contracts/schemas/` contains at least one file when the schema declares a `contracts` brief, and that `$ref` pointers in OpenAPI/AsyncAPI files resolve.

### Layer 3 (future, deferred)

9. **`specify contract validate`** — cross-repo contract compatibility checking.
10. **Contract-validation build brief** — automated verification during build that generated code matches the contract.

## Implementation order

1. Author the `contracts.md` brief (Layer 1, item 1). This is the core deliverable — the brief body contains the generation rules and the agent follows them during define.
2. Update the `design.md` brief (Layer 1, item 2). A small `needs` change plus section rewording.
3. Create the child schemas (Layer 1, item 3). Schema composition handles the pipeline insertion.
4. Author the fixture (Layer 1, item 4). Validates the brief against a representative input.
5. Extend `specify merge` (Layer 2, item 5). The merge engine gains a contract-copy step.
6. Extend `specify spec preview` and `conflict-check` (Layer 2, items 6–7). Preview and conflict surfaces gain contract awareness.
7. Extend `specify validate` (Layer 2, item 8). Structural validation of contract artifacts.

Steps 1–4 can ship without any CLI changes. Steps 5–7 require specify-cli changes and can follow independently.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md)
- [RFC-2: Execution](archive/rfc-2-execution.md)
- [RFC-3a: Initiative Planning](archive/rfc-3a-monoliths.md)
- [RFC-3b: Platform Changes](archive/rfc-3b-platform.md)
- [OpenAPI 3.1 Specification](https://spec.openapis.org/oas/v3.1.0)
- [AsyncAPI 3.0 Specification](https://www.asyncapi.com/docs/reference/specification/v3.0.0)
- [JSON Schema](https://json-schema.org/specification)

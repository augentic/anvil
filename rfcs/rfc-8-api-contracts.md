# RFC-8: API Contracts

> Status: Draft · Depends: [RFC-1](archive/rfc-1-cli.md), [RFC-2](archive/rfc-2-execution.md), [RFC-3a](archive/rfc-3a-monoliths.md), [RFC-3b](archive/rfc-3b-platform.md)

## Abstract

Introduce machine-readable API contracts as a first-class, platform-level artifact in Specify. Contracts capture the interface shapes that behavioral specs describe — request/response payloads, message envelopes, error types — in a format that tooling can validate and generate code from. JSON Schema defines the shared payload vocabulary; OpenAPI and AsyncAPI provide protocol-specific bindings for HTTP endpoints and messaging respectively.

Contracts are co-located with `registry.yaml` and `plan.yaml` at `.specify/contracts/`. They are a platform concern — they describe interfaces *between* components, not internals of any one — and every project references the same central contracts, including the implementer of the API itself.

Contracts are authored before implementation begins — either in dedicated contract changes that precede implementation changes in the plan, or imported from external systems. Implementation changes then validate their specs against the baseline contracts rather than deriving contracts from scratch. A `contracts` brief in the define pipeline reads the baseline contracts and the change's specs, verifying alignment and producing new contract artifacts only when specs describe interactions not covered by the baseline. For single-repo services with no external consumers, inline derivation from specs is available as a convenience fallback. `/spec:plan` automatically selects the right authorship pattern (contract-first, spec-first, or contract-given) based on registry topology, source declarations, and API boundary detection; the operator does not make this choice manually.

No CLI changes are required for the initial implementation; baseline tracking, cross-repo distribution, contract import tooling, and registry role declarations are layered on top.

## Motivation

Specify specs are behavioral requirements. They describe *what* a system must do — `WHEN/THEN` scenarios, `SHALL` statements, error conditions — but not the *shape* of the interfaces those behaviors expose. The Omnia `design.md` brief has sections for `## API Contracts` and `## Publication & Timing Patterns`, but these are prose: useful for human context, useless for machine validation or code generation.

This gap matters in three scenarios:

1. **Single-repo services.** A WASM service's API surface is implicit in the code. When specs change, there is no machine-readable artifact that captures the *before* and *after* of the API shape. Consumers discover breaking changes at integration time, not at define time.

2. **Multi-repo initiatives.** A mobile frontend and a WASM backend communicate over HTTP and/or messaging. RFC-3b explicitly defers cross-repo spec references (`@peer:capability`) and states that "API contracts, auth libraries, protocol definitions are a framework-level concern addressed by the platform's own dependency management and build tooling, not by Specify's code-generation pipeline." This is the right scope boundary for *behavioral* specs, but it leaves a gap: there is no Specify-managed artifact that captures the interface contract between components. Change ordering via `depends-on` edges ensures sequencing but not compatibility.

3. **External and legacy systems.** When migrating a legacy service or integrating with a partner API, the interface shape is a given — mandated by the external system, not derived from new specs. The existing `specs → contracts` derivation model assumes Specify authors the contract; there is no mechanism to import a pre-existing contract, validate that specs conform to it, or record that the contract's authoritative source is outside the platform.

Machine-readable contracts close all three gaps. Within a single repo, they make the API surface explicit and diffable. Across repos, they provide a shared artifact that both producer and consumer validate against. For external interfaces, they bring the pre-existing contract into the Specify workflow so specs can be validated against it and implementation changes can reference it.

### Why central, not per-project?

An API contract is a shared agreement between parties. It does not belong to the producer any more than to the consumer — it is the interface between them. Nesting contracts inside a single project's capability tree misattributes ownership and forces consumers to navigate workspace clones to find the producer's contract files.

Co-locating contracts with `registry.yaml` makes the neutrality structural:

- **`registry.yaml`** declares *who* the participants are.
- **`plan.yaml`** declares *what* changes are planned.
- **`.specify/contracts/`** declares *how* participants communicate.

Three platform concerns, three co-located locations. Both sides of an interface — producer and consumer — reference the same central contracts. Neither owns them; both are bound by them.

This mirrors established industry practice: proto repos, shared OpenAPI spec repos, and contract-first design all place interface definitions in a single canonical location rather than in one party's source tree.

### Why not `@peer:capability`?

`@peer:capability` (deferred by RFC-3b) is a *behavioral* cross-reference — "this capability depends on that capability." Two repos could have perfectly complementary behavioral specs and still produce incompatible HTTP interfaces. Interface compatibility requires wire-level detail (endpoint paths, payload schemas, status codes, message topics) that behavioral specs deliberately do not encode. `@peer:capability` remains useful for planning and ordering; API contracts are orthogonal.

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

Contracts live at `.specify/contracts/` — a platform-level directory alongside `registry.yaml` and `plan.yaml`:

```text
.specify/
├── registry.yaml
├── plan.yaml
├── contracts/                  # Platform API contracts
│   ├── schemas/                # JSON Schema payload definitions
│   │   ├── user-registration.yaml
│   │   ├── order-placed.yaml
│   │   └── error-response.yaml
│   ├── http/                   # OpenAPI bindings (when HTTP is used)
│   │   └── user-api.yaml       # $ref → ../schemas/
│   └── messages/               # AsyncAPI bindings (when messaging is used)
│       └── order-events.yaml   # $ref → ../schemas/
```

Directory rules:

- **`contracts/schemas/`** is always present. Every contract includes at least one payload schema.
- **`contracts/http/`** is present when the platform includes HTTP interactions (REST endpoints, request/response patterns). Omitted for purely event-driven systems.
- **`contracts/messages/`** is present when the platform includes messaging interactions (pub/sub, event-driven, queue-based). Omitted for purely synchronous HTTP systems.
- Both `http/` and `messages/` may be present when the platform uses both transport types.

Contracts sit outside the per-capability spec tree. This is correct because a single OpenAPI document or schema type often spans multiple capabilities — a `POST /users` endpoint might touch `user-registration`, `auth`, and `notifications` capabilities. Flattening contracts out of the capability hierarchy avoids the question of "which capability owns this schema?" — nobody does; it is platform vocabulary.

### Naming conventions

All contract files use **kebab-case** names with `.yaml` extensions, consistent with Specify's existing naming conventions for spec files, change directories, and plan entries.

- **Schema files** are named after the domain type they define: `user-registration.yaml`, `error-response.yaml`, `order-placed.yaml`. One type per file.
- **HTTP binding files** are named after the API domain they describe: `user-api.yaml`, `billing-api.yaml`. A single OpenAPI file may contain multiple related endpoints (e.g. `POST /users`, `GET /users/{id}`, `DELETE /users/{id}` all in `user-api.yaml`).
- **Message binding files** are named after the event domain: `order-events.yaml`, `notification-events.yaml`. A single AsyncAPI file may contain multiple related channels.

The `$id` field in JSON Schema files must be a valid URI per the JSON Schema specification. The writer skill defines the exact format; the constraint is that `$id` values are stable, unique within the contract tree, and compatible with standard JSON Schema tooling (`ajv`, `typify`, etc.). A natural convention is a URN-shaped identifier derived from the file path (e.g. `$id: "urn:specify:schemas/user-registration"`), but the precise format is a writer-skill implementation detail.

### Contract modifcation

During a change's define phase, proposed contract modifications live in the change directory:

```text
.specify/changes/add-oauth/
├── contracts/                  # Proposed contract changes (this change only)
│   ├── schemas/
│   │   └── oauth-token.yaml    # New type
│   └── http/
│       └── user-api.yaml       # Updated OpenAPI (additional paths)
├── specs/
├── design.md
└── ...
```

The change-level `contracts/` directory contains only the files this change adds or replaces — not a full copy of the baseline. This keeps the diff reviewable and makes it clear what a single change contributes to the platform's contract surface.

**Contract deletion.** The change-level directory can express additions and replacements but not deletions — there is no mechanism to say "remove this file from the baseline." For specs, the delta format has `## REMOVED Requirements`; contracts use opaque replacement and have no equivalent. Contract deletion is rare (retiring an endpoint or decommissioning a message channel) and is handled as a manual baseline edit. A deletion mechanism could be added in a future layer if the need arises.

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

1. **`specs` → `contracts`**: The specs brief establishes *what* the system does; the contracts brief then validates that the change's specs align with the baseline contracts and produces new contract artifacts only for interactions not already covered. The `contracts` brief declares `needs: [specs]`. When baseline contracts exist (the recommended default — see §*Authorship patterns*), the contracts brief operates primarily in validation mode, verifying alignment rather than generating from scratch.
2. **`contracts` → `design`**: The design document references contracts rather than re-describing API shapes in prose. The `design` brief declares `needs: [proposal, contracts]` (adding `contracts` to its existing `needs`). The `## API Contracts` and `## Publication & Timing Patterns` sections in `design.md` become pointers to the contract files rather than hand-authored descriptions.
3. **`contracts` → `tasks`**: Task generation can reference contracts for code-generation tasks (e.g. "generate Rust types from `contracts/schemas/`"). The `tasks` brief already declares `needs: [specs, design]`; adding `contracts` is optional — design transitively carries the contract context.

#### Baseline contract visibility in the specs brief

Because contracts are typically authored before implementation changes begin (§*Authorship patterns*), the baseline at `.specify/contracts/` usually contains the relevant contracts by the time an implementation change's define phase runs. The `specs` brief benefits from seeing these as context — spec authors should write behavioral requirements that *conform to* the existing interface shapes rather than inventing new ones.

No schema-resolution change is needed for this. Baseline contracts are files on disk at a well-known path (`.specify/contracts/`). The `specs` brief body instructs the agent: "if `.specify/contracts/` exists, read its contents as read-only context and write scenarios consistent with the existing endpoint paths, payload schemas, and error responses." When no baseline contracts exist (e.g. a single-repo service using the spec-first fallback pattern), the directory is absent and the instruction has no effect.

This preserves the `specs → contracts` pipeline ordering — specs still run before the `contracts` brief — while giving spec authors visibility into contracts that their requirements must conform to. The mechanism is a brief-body instruction, not a schema-resolution primitive.

### Brief frontmatter

```yaml
---
id: contracts
description: Validate spec alignment with baseline contracts; generate delta for uncovered interactions
generates: contracts/**/*.yaml
needs: [specs]
---
```

The `generates` glob (`contracts/**/*.yaml`) scopes output to the change-level `contracts/` directory.

### Brief body

The `contracts` brief is a thin orchestrator — it delegates to `/contracts:writer` and `/contracts:validator`, following the same pattern as the `build` brief's delegation to `/omnia:crate-writer` and `/omnia:test-writer`.

#### Algorithm

1. `/contracts:writer` — read baseline contracts and specs, validate alignment, produce the minimal contract delta.
2. `/contracts:validator` — verify internal consistency of the produced artifacts.

There is no mode switch. The writer always follows the same algorithm: read the baseline, read the specs, validate alignment, produce a delta for what the specs require that the baseline does not already cover. In the recommended contract-first workflow, baseline contracts already exist when implementation changes run their define phase — the writer validates that the specs align with those contracts and produces a small or empty delta, flagging mismatches rather than silently overwriting. When no baseline contracts exist (the spec-first fallback for single-repo services), the delta is the full contract set, derived from the change's specs.

#### Verify-repair loop

If the validator reports failures, re-enter the writer with the validation output for targeted repair. Repeat until clean or a maximum of 2 iterations — analogous to the build brief's verify-repair loop. If still failing after 2 iterations, stop and surface the issues for human review.

### Specialist skills

#### `/contracts:writer`

Validates spec alignment with baseline contracts and produces the minimal contract delta for uncovered interactions. The algorithm is always the same regardless of whether the baseline is empty, rich, or externally imported — the difference is in outcome, not in code path:

1. **Read the baseline contracts** at `.specify/contracts/` to understand the current platform vocabulary — existing domain types, HTTP bindings, and messaging bindings.

2. **Read all spec files** under the change's `specs/` directory and identify requirements that describe API interactions (HTTP endpoints, request/response patterns) or message exchanges (pub/sub, event-driven patterns). When the change has no specs (a contract-only import change), skip to step 3's normalisation path — the delta consists of metadata normalisation only, and the brief delegates directly to `/contracts:validator`.

3. **Validate alignment and determine the minimal delta.** Compare what the specs require against the existing baseline:
   - **Already covered (primary path):** When the baseline already defines an endpoint, channel, or schema that the specs describe, validate alignment — verify that endpoint paths, methods, payload shapes, error codes, channel names, and message structures match. Flag mismatches as warnings for human review. Do not regenerate what already exists. In the recommended contract-first workflow, most or all spec interactions fall into this category.
   - **New or modified (fallback):** When the specs require types, endpoints, or channels absent from the baseline, generate the corresponding contract files. This is the primary path only in the spec-first fallback pattern (single-repo services with no external consumers) where the baseline is empty.
   - **Normalisation:** When baseline files lack Specify conventions (missing `$id` on schemas, inconsistent `description` fields), propose a normalisation delta that adds the missing metadata without changing the interface shapes.

4. **Generate JSON Schema files** for new or modified domain types (when step 3 identifies uncovered interactions), each with:
   - `$id` for stable cross-referencing
   - `title` matching the type name
   - `description` from the spec's behavioral description
   - `properties`, `required`, and type constraints derived from scenario data
   - `$ref` pointers for shared sub-types (referencing baseline schemas where they already exist)

5. **Generate or update OpenAPI spec** (when applicable and step 3 identifies uncovered HTTP interactions). Produce contract files under `contracts/http/` with:
   - Paths and methods derived from spec scenarios
   - Request/response schemas as `$ref` pointers to `../schemas/`
   - Error responses derived from the spec's error conditions
   - OpenAPI 3.1 format (native JSON Schema support)

6. **Generate or update AsyncAPI spec** (when applicable and step 3 identifies uncovered messaging interactions). Produce contract files under `contracts/messages/` with:
   - Channels and operations derived from spec scenarios
   - Message payload schemas as `$ref` pointers to `../schemas/`
   - AsyncAPI 3.0 format (native JSON Schema support)

The writer reports what it found: how many spec interactions were already covered by the baseline (with alignment results), how many required new contract artifacts, and any mismatches flagged for review. A clean alignment report with an empty delta is the expected outcome for implementation changes in a contract-first workflow.

#### `/contracts:validator`

Validates internal consistency of contract artifacts after the writer completes. The validator does not generate or modify contract files — it reports issues for the brief's verify-repair loop to act on.

Checks:

1. **`$ref` resolution.** All `$ref` pointers in OpenAPI and AsyncAPI files must resolve — either to files in the change's `contracts/schemas/` or to existing files in the baseline `.specify/contracts/schemas/`.
2. **Schema metadata.** Every JSON Schema file has `$id`, `title`, and `description`.
3. **Binding completeness.** Every schema that appears as a top-level request body, response body, or message payload in a spec scenario has at least one protocol binding (an OpenAPI path or AsyncAPI channel). Shared vocabulary types (`ErrorResponse`, `Pagination`, etc.) that appear only as `$ref` targets inside other schemas are exempt — they are reusable building blocks, not standalone endpoints.

The validator reports each issue with the file path and a description of the problem.

#### `/contracts:importer` (Layer 2)

The importer is deferred to Layer 2. It codifies format detection, version upgrade (Swagger 2.0 / OpenAPI 3.0 → 3.1, AsyncAPI 2.x → 3.0), inline schema decomposition, and Specify metadata injection for external contract files (§*Implementation scope*, item 14).

In Layer 1, external contracts are imported manually: the operator places OpenAPI 3.1 / AsyncAPI 3.0 / JSON Schema files into the change's `contracts/` directory, following the artifact structure described in §*Artifact structure*. The `/contracts:writer` normalises metadata gaps (missing `$id`, `description`) as part of its standard delta, and `/contracts:validator` catches structural issues. The agent can assist with format conversion when the source files are not already in the target versions.

### Relationship to `design.md`

The Omnia `design.md` brief currently has:

```markdown
## API Contracts
<!-- Endpoints with method, path, request/response shapes, errors -->

## Publication & Timing Patterns
<!-- Topics, message shapes, timing, partition keys -->
```

With the `contracts` brief in place, these sections change role. Instead of being the primary description of interface shapes, they become **references** to the platform contract files with additional implementation-level context that the contract format does not capture:

```markdown
## API Contracts

See `.specify/contracts/http/` for the full OpenAPI specifications.

<!-- Implementation notes: auth, rate limits, caching, versioning strategy -->

## Publication & Timing Patterns

See `.specify/contracts/messages/` for the full AsyncAPI specifications.

<!-- Implementation notes: ordering guarantees, retry policies, DLQ strategy -->
```

**Scope boundary:** Contracts capture the *structural shape* of interfaces — endpoint paths, methods, payload schemas, error codes, channel names, message structures. Everything else stays in `design.md`: authentication schemes and `securitySchemes` (which are implementation policy, not interface shape), rate limits, retry policies, caching strategies, versioning approaches, and ordering guarantees. If a concern affects wire compatibility, it belongs in the contract; if it affects operational behavior, it belongs in `design.md`.

The `design` brief's `needs` gains `contracts` so the agent has the generated files available when writing design.md.

## Authorship patterns

`/spec:plan` automatically determines how contracts enter the plan. The operator does not choose an authorship pattern — the plan skill reads the registry, source declarations, and project topology and inserts the right contract changes in the right positions. Three patterns emerge from this algorithm, but they are implementation details of the plan skill, not decisions the operator makes.

All three patterns use the same `contracts` brief, the same `/contracts:writer` and `/contracts:validator` skills, the same central `.specify/contracts/` location, and the same merge semantics. The writer always follows the same algorithm — read baseline, read specs, validate alignment, produce delta — so the patterns differ in plan structure and baseline state, not in code path.

### `/spec:plan` algorithm

`/spec:plan` applies the following rules when constructing the plan:

1. **API boundary between projects.** When the plan contains changes in multiple projects that share an API boundary (identified from registry descriptions, source analysis, or operator input), `/spec:plan` inserts a dedicated contract change before the implementation changes on both sides. The contract change carries interface-level behavioral specs and derives contracts from them; implementation changes `depends-on` the contract change and validate alignment. This is the **contract-first** pattern and is the most common case for multi-service platforms.

2. **External system or legacy migration.** When a source is flagged as an external system or legacy API, `/spec:plan` inserts an import change before the implementation changes. The operator places the external contract files into the import change's `contracts/` directory; the change's build phase validates structural correctness. Implementation changes depend on the import change and write specs that conform to the imported contract. This is the **contract-given** pattern.

3. **Single-repo, no API boundary.** When the plan contains a single-repo change with no identified API boundary and no external consumers, no separate contract change is inserted. The `contracts` brief derives interface shapes inline during the change's define phase — the baseline is empty, so the delta is the full contract set. This is the **spec-first** pattern, a convenience fallback for isolated services.

The contract-first pattern is the default. Most Augentic work involves multi-service platforms where API boundaries exist between projects. Spec-first inline derivation should not be used when the API is shared across repos or consumed by external systems — `/spec:plan` enforces this by inserting a contract change whenever it detects a cross-project interface.

### Contract-first (dedicated contract change)

The plan skill inserts a dedicated contract change before implementation changes. The contract change is a regular Specify change that produces interface-level behavioral specs and derives contracts from them. It carries no implementation code — its build phase validates the contract artifacts rather than generating code.

**Build phase for contract-only changes.** The build brief delegates to code-generation skills based on the tasks in `tasks.md`. For a contract-only change, the tasks brief generates validation tasks rather than code-generation tasks — the task list contains items like "validate contract structural correctness" and "verify `$ref` resolution" rather than "generate Rust crate." The build brief's mode detection (check whether `Cargo.toml` exists) naturally falls through to a no-op for code generation, and the validation tasks are satisfied by the `/contracts:validator` output from the define phase. This requires no special handling in the build brief — the task-driven model adapts to the change's content.

The interface has its own behavioral specs, distinct from any project's internal specs:

> The user registration API SHALL accept a `POST /users` request with a `UserRegistration` payload.
> WHEN the registration succeeds, the API SHALL respond with `201 Created` and a `User` payload including the assigned `id`.
> WHEN the email is already registered, the API SHALL respond with `409 Conflict` and an `ErrorResponse` payload.

These are behavioral requirements for the interface itself — not the producer's internal logic or the consumer's UI flows. `/contracts:writer` derives the machine-readable shapes from these specs: JSON Schema for `UserRegistration`, `User`, and `ErrorResponse`; an OpenAPI binding for the endpoint. The spec-driven model is intact: specs describe *what* the interface does; contracts capture *what the interface looks like*.

The plan makes the sequencing explicit and enables parallelism:

```yaml
changes:
  - name: user-api-contract
    description: "Define the user registration API contract"
    status: pending

  - name: user-api-backend
    project: backend
    description: "Implement the user registration API"
    depends-on: [user-api-contract]
    status: pending

  - name: registration-screen
    project: mobile
    description: "Build the registration screen"
    depends-on: [user-api-contract]
    status: pending
```

Once the contract change merges, producer and consumer run in parallel — both depend on the contract, not on each other:

```
                         ┌── user-api-backend (backend)
user-api-contract ───────┤
                         └── registration-screen (mobile)
```

### Spec-first (inline derivation)

For single-repo services with no external consumers and no multi-party coordination, `/spec:plan` does not insert a separate contract change. The `contracts` brief derives interface shapes inline during the change's define phase — one change, one define phase, specs and contracts produced together. This is appropriate when the API surface is a thin projection of a single capability and there is no second party to agree with.

### Contract-given (external or legacy contracts)

When `/spec:plan` identifies a source flagged as an external system or legacy migration, it inserts an import change before the implementation changes. The derivation direction is reversed: the machine-readable contract already exists or its shape is dictated by a third party, and the specs need to *conform to* that contract rather than *generate* it.

The workflow:

1. **Import.** The operator places the external contract files into `.specify/changes/<name>/contracts/`, following the artifact structure (§*Artifact structure*). The files should be in the target formats (OpenAPI 3.1, AsyncAPI 3.0, JSON Schema). When the source files are in older formats (Swagger 2.0, OpenAPI 3.0, AsyncAPI 2.x), the agent assists with conversion, or Layer 2's `/contracts:importer` skill automates it. For legacy systems, the RT plugin's `wiretapper` skill can capture the API shape as input.

2. **Build (import change).** The import change carries no implementation code. Its build phase runs `/contracts:validator` against the change's `contracts/` directory to verify structural correctness: well-formed OpenAPI/AsyncAPI, resolvable `$ref` pointers, and schema metadata present. The import change's contracts are held to the same standard as derived contracts. The build phase does not invoke code-generation skills — its sole output is validated contract artifacts.

3. **Define (implementation changes).** Implementation changes that `depends-on` the import change find the imported contracts in the baseline after the import change merges. `/contracts:writer` reads the rich baseline, finds that the specs' interactions are already covered, validates alignment, and produces a small or empty delta:
   - Every endpoint or channel described in the specs has a corresponding binding in the baseline contracts.
   - Payload shapes referenced in spec scenarios match the JSON Schema definitions.
   - Error conditions in specs correspond to error responses in the contract.
   - Mismatches are flagged for human review rather than silently overwritten.

4. **Delta for extensions only.** If the change extends the external contract (e.g. adding a new endpoint to a legacy API during migration), the writer produces contract files for the *additions* only. The imported baseline files are not modified — they remain the external system's authoritative shapes.

The plan structure for a migration:

```yaml
changes:
  - name: import-legacy-api
    description: "Import legacy billing API contract"
    status: pending

  - name: user-api-backend
    project: backend
    description: "Implement user API on new platform"
    depends-on: [import-legacy-api]
    status: pending
```

The import change produces validated contract artifacts. Implementation changes depend on it and write specs that conform to the imported contract.

### Contract references in plan entries

In Layer 1, the `depends-on` edges in the plan provide sufficient signal for the agent to identify which baseline contracts are relevant to a change. When a change `depends-on` a contract change, the agent reads the contract change's output to understand which contracts apply.

Producer/consumer role information — whether a change *implements* a contract or *calls* an API described by one — comes from the registry, not the plan. Layer 2's `contracts` block on `registry.yaml` project entries (§*Contract roles in `registry.yaml`*) declares `produces`, `consumes`, and `imports` per project. The plan entry's `project` field cross-references to the registry; briefs look up the project's role when they need to determine validation strategy (completeness for producers, compatibility for consumers).

In large baselines with many unrelated contracts, a narrowing mechanism helps the agent focus. This is not specific to contracts — the same problem applies to large spec baselines, large source trees, and any other context the define phase reads. Rather than a contract-specific `uses-contracts` field, Layer 2 introduces a generic `context` field on plan entries that declares which baseline paths are relevant to a change regardless of artifact type. See §*Generic context narrowing (`context`)* for the design.

## Single-repo and multi-repo — same model

The central co-location model works identically in both topologies:

| Concern | Single-repo | Multi-repo |
|---------|-------------|------------|
| Contracts location | `.specify/contracts/` | `.specify/contracts/` (initiating repo) |
| Who writes (new APIs) | Dedicated contract change (inserted by `/spec:plan` when API boundary detected) or inline fallback for isolated services | Dedicated contract change (inserted by `/spec:plan`), then `/contracts:writer` validates alignment in implementation changes |
| Who writes (external APIs) | Import into baseline, then `/contracts:writer` validates alignment | Import change in plan, then alignment validation in implementation changes |
| How projects read | Direct filesystem read | Materialised by `workspace sync` |
| How changes propose updates | `.specify/changes/<name>/contracts/` | Same — in the project clone's change directory |
| How updates merge | `specify merge` → `.specify/contracts/` | Same — then `workspace push` propagates |

Phase skills see the same paths regardless of topology. The only difference is the distribution mechanism, which is handled by the existing workspace infrastructure.

## Multi-repo contract sharing

Central co-location with `registry.yaml` makes multi-repo contract sharing straightforward. The same `.specify/contracts/` directory serves all projects; distribution uses the existing workspace infrastructure.

### Layer 1: Central contracts in the initiating repo (no CLI changes)

In a multi-repo initiative, contracts live in the initiating repo alongside `registry.yaml`. The plan uses dedicated contract changes (see §*Authorship patterns*) to define interfaces before implementation begins. Contract changes run in the initiating repo, where `.specify/contracts/` is directly accessible.

In Layer 1, workspace clones do not automatically receive the central contracts — the `workspace sync` extension that materialises `.specify/contracts/` into project clones is a Layer 2 CLI change (§*Implementation scope*, item 13). Until then, the `/spec:execute` driver can copy the central `.specify/contracts/` into each project clone's `.specify/contracts/` as a pre-change distribution step — the same directory structure, achieved by the agent rather than by the CLI.

The target state (Layer 2) looks like:

```text
.specify/                           # Initiating repo
├── registry.yaml
├── contracts/                      # Central source of truth
│   ├── schemas/
│   │   └── user.yaml
│   ├── http/
│   │   └── user-api.yaml
│   └── messages/
│       └── order-events.yaml
├── workspace/
│   ├── backend/
│   │   └── .specify/
│   │       └── contracts/          # ← materialised from central
│   └── mobile/
│       └── .specify/
│           └── contracts/          # ← materialised from central
```

Phase skills always read from `.specify/contracts/` relative to their working directory — they do not need to know whether the contracts were authored locally or materialised from a central source. This preserves RFC-3b's design principle that phase skills are unaware of the multi-repo topology.

When a contract change completes its define-build-merge cycle, its contracts merge into the central `.specify/contracts/`. The distribution step (agent-driven in Layer 1, automated by `workspace sync` in Layer 2) propagates the updated contracts to all project clones before dependent implementation changes begin their define phases. Implementation changes read the materialised contracts as their baseline; `/contracts:writer` proposes only additions or modifications their specs require — typically a small or empty delta, since the interface was already defined by the contract change.

### Layer 2: Baseline tracking and merge (CLI changes)

To make contracts a persistent, diffable artifact with full merge support:

When `specify merge` processes a change, it copies the change's `contracts/` files into `.specify/contracts/`, replacing files that share a path. Contract files use **opaque replacement** semantics — unlike spec files which use the ADDED/MODIFIED/REMOVED delta format, contract files are replaced wholesale. The rationale: JSON Schema and OpenAPI/AsyncAPI files have their own versioning semantics (`$id`, `info.version`); introducing a second delta-merge algorithm for YAML contract files would add complexity without clear benefit over replacement.

`specify spec preview` and `specify spec conflict-check` are extended to include contract files in their output so operators see when a merge will update contracts.

This layer requires CLI changes:

- `specify merge` copies change-level `contracts/` files into `.specify/contracts/`. Files that share a path are replaced; files absent from the change are left untouched.
- `specify spec preview` reports contract file changes (added/replaced).
- `specify spec conflict-check` detects when baseline contracts have been modified after the change's `defined-at` timestamp.

#### Merge conflicts

Because contracts use opaque replacement semantics, two concurrent changes that both modify the same contract file (e.g. both add paths to `http/user-api.yaml`) will conflict. `specify spec conflict-check` detects this: if the baseline file was modified after the change's `defined-at` timestamp, the merge is blocked. The resolution is to re-run the change's define phase against the updated baseline — the writer reads the current baseline and produces a fresh delta that accounts for the other change's additions. This is the same workflow as for spec conflicts, but without the delta-merge fallback.

#### Drift detection (`/spec:verify`)

`/spec:verify` detects drift between code and baseline specs. Contract drift — where the implementation diverges from the baseline contracts (e.g. a field added to a Rust struct without a corresponding schema update, or an endpoint handler without an OpenAPI path) — is a natural extension. This RFC does not add contract-aware drift detection to `/spec:verify`; the mechanism is deferred to Layer 3 alongside automated contract validation. In the interim, the agent can compare contracts against implementation code during manual review.

### Layer 3: Automated contract validation (future)

A future extension could add automated validation that a consumer's usage of a contract is compatible with the producer's definition — for example, verifying that a mobile frontend's API client calls match the backend's OpenAPI spec, or that a message consumer's expected payload matches the producer's AsyncAPI schema. This is analogous to contract testing (Pact, Spring Cloud Contract) but integrated into the Specify workflow.

This layer is explicitly deferred. It requires:

- A `specify contract validate` CLI verb that loads the central contracts and each project's specs, checking compatibility.
- A contract-validation brief in the build pipeline that runs after code generation.
- A definition of "compatibility" that accounts for backwards-compatible changes (additive fields, optional-to-required transitions, etc.).

The central contracts directory makes validation simpler than the per-project model — all contracts are in one place, and all project specs are accessible via workspace clones. The validation rules can be designed against real examples when the need arises.

## Contract roles in `registry.yaml` (Layer 2)

In Layer 1, contract ownership is implicit in the plan structure: a contract change's `depends-on` edges reveal which projects produce and consume each contract. The plan is the source of truth for who depends on what.

Layer 2 introduces an explicit `contracts` block on `registry.yaml` project entries that persists role information beyond a single initiative. This becomes valuable when contracts outlive the plan that created them — when the question shifts from "who depends on this contract in the current plan?" to "who owns this contract in the platform?"

### Schema

The `contracts` block is optional on each project entry. When present, it declares the project's relationship to specific contract files:

```yaml
version: 1
projects:
  - name: backend
    url: git@github.com:org/backend.git
    schema: omnia@v1
    description: >
      User registration API and account management.
    contracts:
      produces:
        - http/user-api.yaml
        - schemas/user-registration.yaml
        - schemas/user.yaml
        - schemas/error-response.yaml
      consumes:
        - messages/order-events.yaml

  - name: mobile
    url: ../mobile
    schema: vectis@v1
    description: >
      iOS and Android mobile application.
    contracts:
      consumes:
        - http/user-api.yaml
        - schemas/user-registration.yaml
        - schemas/user.yaml
        - schemas/error-response.yaml

  - name: billing
    url: git@github.com:org/billing.git
    schema: omnia@v1
    description: >
      Order processing, invoicing, and payment integration.
    contracts:
      produces:
        - messages/order-events.yaml
        - schemas/order-placed.yaml
      imports:
        - http/payment-gateway-api.yaml
```

| Field | Type | Description |
|-------|------|-------------|
| `contracts` | object | Optional. Declares this project's relationship to central contract files. |
| `contracts.produces` | list of paths | Contract files this project is the authoritative implementer of. Paths are relative to `.specify/contracts/`. |
| `contracts.consumes` | list of paths | Contract files this project calls or subscribes to as a client. Paths are relative to `.specify/contracts/`. |
| `contracts.imports` | list of paths | Contract files whose shape is dictated by an external system that this project integrates with. Paths are relative to `.specify/contracts/`. |

All three lists are optional within the `contracts` block. A project that only produces needs no `consumes` or `imports`.

### Validation invariants

`specify initiative registry validate` (and `specify validate` when registry contract roles are present) enforces:

1. **Single producer.** Each contract path appears in `produces` for at most one project. If two projects both claim to produce `http/user-api.yaml`, validation fails. This prevents conflicting implementations of the same interface.
2. **Produce/import mutual exclusion.** A contract path must not appear in both `produces` and `imports` across the registry. A produced contract is authored within the platform; an imported contract is dictated by an external system. The same file cannot be both.
3. **Path validity.** Every path listed in `produces`, `consumes`, or `imports` must be a valid relative path under `.specify/contracts/` (no `..`, no absolute paths). The file need not exist yet — a `produces` declaration may precede the contract change that creates it.
4. **Self-consistency.** A project must not list the same path in both `produces` and `consumes` — a project cannot be both the authoritative implementer and a client of the same interface. `imports` and `consumes` may overlap (a project can both import an external contract and consume it).

### How skills use contract roles

- **`/spec:plan`** populates `contracts` roles when inserting contract changes. When the plan skill creates a contract change for an API boundary between projects, it adds the contract paths to the producer's `produces` and the consumer's `consumes` via `specify initiative registry validate` (which remains advisory — the operator can adjust). For import changes, it adds the paths to the importing project's `imports`.
- **`/contracts:writer`** cross-references the current project's role to determine validation strategy. For a `produces` project, the writer validates *completeness* — every endpoint or channel in the contract must have a corresponding spec scenario. For a `consumes` project, the writer validates *compatibility* — the spec scenarios must be consistent with the contract's types and endpoints but need not cover every endpoint.
- **`/contracts:validator`** uses roles to scope binding-completeness checks. A consumer project's change need not have bindings for every schema in the contract — only the ones its specs reference.
- **`specify validate`** checks the invariants above when the registry declares contract roles.

## Generic context narrowing (`context`)

### Motivation

The define phase reads context from the baseline to inform artifact generation: specs briefs read `.specify/contracts/` for interface conformance, design briefs read prior specs, and future artifact types will follow the same pattern. In small baselines, reading everything at a well-known path works. In large baselines — dozens of contract files, hundreds of spec capabilities — the agent benefits from knowing which subset is relevant to the change at hand.

This narrowing problem is not specific to contracts. A change that modifies a single capability in a platform with 50 baseline specs faces the same issue: the specs brief must decide which baselines to read as context. A contract-specific `uses-contracts` field would solve the problem for one artifact type while leaving every other type unaddressed. The generic alternative is a `context` field on plan entries that declares relevant baseline paths regardless of artifact type.

### Design

Layer 2 introduces an optional `context` field on plan entries:

```yaml
changes:
  - name: user-api-backend
    project: backend
    description: "Implement the user registration API"
    depends-on: [user-api-contract]
    context:
      - contracts/http/user-api.yaml
      - contracts/schemas/user-registration.yaml
      - contracts/schemas/error-response.yaml
      - specs/user-registration/spec.md
    status: pending
```

Field semantics:

- **Values are paths relative to `.specify/`.** A brief that sees `contracts/http/user-api.yaml` reads `.specify/contracts/http/user-api.yaml` from the baseline (or from the materialised workspace clone in multi-repo).
- **The list is a focus hint, not an access restriction.** Briefs may still read other baseline paths when the brief body instructs them to (e.g. the contracts writer always reads the full baseline to detect mismatches). `context` tells the brief "start here" — it narrows default scanning, not capability.
- **`/spec:plan` populates `context` automatically.** When the plan skill inserts a contract change and wires `depends-on` edges from implementation changes, it also populates `context` on those implementation entries with the contract paths the contract change will produce. For changes with `affects` targeting existing capabilities, `/spec:plan` populates `context` with the corresponding baseline spec paths.
- **Manual authoring is supported.** Operators using the Layer 1 hand-driven workflow can add `context` via `specify plan create --context <path>...` or `specify plan amend --context <path>...`.

### How briefs use `context`

The `context` field is available to every brief in the define pipeline. Each brief consults it according to its own needs:

- **`specs` brief.** When `context` contains `contracts/` paths, the brief reads those specific contract files as conformance context rather than scanning the entire `.specify/contracts/` directory. When `context` contains `specs/` paths, the brief reads those baselines for delta composition.
- **`contracts` brief.** When `context` contains `contracts/` paths, `/contracts:writer` uses them as the primary alignment targets — validating that the change's specs align with the listed contracts before scanning the broader baseline for mismatches. When `context` is absent, the writer falls back to scanning the full baseline (the Layer 1 behavior).
- **`design` brief.** When `context` contains paths, the brief uses them to identify which contracts and specs to reference in the design document's `## API Contracts` and `## Publication & Timing Patterns` sections.
- **Future briefs.** Any new brief type can consult `context` for the same narrowing benefit. The mechanism is artifact-agnostic.

### Relationship to existing plan entry fields

| Field | Purpose | Consumed by |
|---|---|---|
| `depends-on` | Ordering constraint — "don't start until these are done" | `specify plan next` |
| `sources` | External input — "analyze these repos" | `/spec:define` via `--source` flags |
| `affects` | Impact annotation — "this change modifies these capabilities" | Delta-target inference in specs brief |
| `description` | Scoping intent — free-text guidance for the define phase | Scope and delta-target inference in briefs |
| `context` | Context narrowing — "these baseline paths are relevant" | All define briefs as a focus filter |

`context` does not overlap with `depends-on` (which governs execution order, not content focus), `sources` (which points to external input, not baseline artifacts), or `affects` (which declares impact on capabilities, not which files to read). It complements `description` by providing machine-readable path precision alongside the human-readable scoping prose.

### Consistency with the information-flow model

Contracts follow the same context-delivery model as every other artifact type:

| Context type | How it reaches `/spec:define` |
|---|---|
| Legacy source code | `sources` on plan entry → `--source` flag → `/spec:extract` |
| Prior change's specs | `depends-on` ordering → merged into `.specify/specs/` → brief reads baseline |
| Baseline contracts | `depends-on` ordering → merged into `.specify/contracts/` → brief reads baseline |
| Producer/consumer role | `project` on plan entry → cross-reference with `registry.yaml` contract roles |
| Context narrowing | `context` on plan entry → brief filters baseline scanning to listed paths |

No artifact type gets a special-purpose narrowing mechanism. The `context` field is the single generic solution.

## Schema integration

The `contracts` brief is added directly to the base `omnia` and `vectis` schemas. No child schemas or schema composition is needed.

The updated Omnia `define` pipeline:

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

The updated Vectis `define` pipeline inserts `contracts` between `specs` and `composition`:

```yaml
pipeline:
  define:
    - id: proposal
      brief: briefs/proposal.md
    - id: specs
      brief: briefs/specs.md
    - id: contracts
      brief: briefs/contracts.md
    - id: composition
      brief: briefs/composition.md
    - id: design
      brief: briefs/design.md
    - id: tasks
      brief: briefs/tasks.md
```

The `contracts.md` brief and the updated `design.md` brief (which adds `contracts` to its `needs`) are added to each base schema's `briefs/` directory alongside the existing briefs. `build` and `merge` pipelines are unchanged.

### Why not opt-in via child schemas?

An earlier draft considered creating `omnia-contracts` and `vectis-contracts` child schemas via `extends`, keeping contracts opt-in at schema selection time. This was rejected for three reasons:

1. **Maintenance coupling.** Schema composition overrides at the file level. The child's `design.md` brief must duplicate the parent's entire brief body; the child's `define` pipeline must reproduce the full stage list to place `contracts` in the correct position. Any future change to the parent's briefs or pipeline must be mirrored manually into each child. Two child schemas means two copies of every affected brief.

2. **The contracts pipeline is a well-behaved no-op.** When a change's specs describe no API interactions, `/contracts:writer` produces an empty delta and `/contracts:validator` has nothing to check. The `contracts/` directory is only created when the first contract change runs. Including the `contracts` stage in the base schema adds no overhead for projects that never use it.

3. **Most projects need contracts.** Almost all Augentic work targets multi-service platforms and applications with frontends consuming backends. Contracts are not an optional extra for these projects — they are the structural link between behavioral specs on both sides of an interface. Making contracts opt-in creates a decision point at init time that most projects should answer the same way, and the cost of answering wrong (missing contracts on a project that needs them) is late-discovered integration risk.

## Non-goals

- **Inventing a new IDL.** The contract format uses JSON Schema, OpenAPI, and AsyncAPI — existing standards with existing tooling. No proprietary schema language.
- **Automated code generation from contracts.** The build phase *can* use contracts as input for code generation (via skill directives in tasks), but this RFC does not prescribe how. Code generation strategies differ by schema (Omnia generates Rust; Vectis generates Rust + Swift + Kotlin); the contract-to-code mapping belongs in the build brief and specialist skills, not in the contract artifact itself.
- **Contract versioning or backwards-compatibility checking.** Semantic versioning of contracts, breaking-change detection, and consumer compatibility validation are deferred to Layer 3. The initial implementation treats contracts as define-time artifacts that are evolved incrementally.
- **Replacing behavioral specs.** Contracts complement specs; they do not replace them. Specs describe *what* the system must do. Contracts describe *what the interface looks like*. Both are needed — one for requirements traceability, the other for machine-readable integration.
- **Cross-repo contract enforcement in Layer 1.** The initial implementation relies on `depends-on` ordering and agent judgment for cross-repo compatibility. Automated validation is explicitly deferred.
- **Contract ownership by a single project.** Contracts are platform-level shared artifacts. No project "owns" a contract; both producer and consumer reference the same central definition. Layer 2 introduces registry roles that record which project is the authoritative producer, but this is a role declaration for conflict prevention, not ownership.

## Implementation scope

### Layer 1 (no CLI changes)

1. **`/contracts:writer` skill** — author `plugins/contracts/skills/writer/SKILL.md` with the alignment-validation and delta-production algorithm. The writer reads baseline contracts from `.specify/contracts/` and the change's specs, validates alignment, and produces the minimal contract delta for uncovered interactions. Includes reference docs for JSON Schema, OpenAPI 3.1, and AsyncAPI 3.0 conventions.
2. **`/contracts:validator` skill** — author `plugins/contracts/skills/validator/SKILL.md` with post-generation validation checks (`$ref` resolution, schema metadata, binding completeness).
3. **`contracts` brief** — author `briefs/contracts.md` as a thin orchestrator: delegation to `/contracts:writer` (alignment validation and delta production) and `/contracts:validator`, plus a verify-repair loop.
4. **Updated `specs` brief** — add a brief-body instruction to read `.specify/contracts/` as optional context when the directory exists. No schema-resolution changes.
5. **Updated `design` brief** — modify `briefs/design.md` to declare `needs: [proposal, contracts]` and update the `## API Contracts` / `## Publication & Timing Patterns` sections to reference the central contract files.
6. **Base schema updates** — add the `contracts` pipeline entry and `briefs/contracts.md` to `schemas/omnia/` and `schemas/vectis/`. Update `briefs/design.md` in each schema to declare `needs: [proposal, contracts]`.
7. **Fixture (generation)** — author a worked example under `schemas/omnia/fixtures/` showing the baseline contracts, the writer's output, the validator's output, and a change-level delta for a representative capability.
8. **Fixture (conformance)** — author a worked example showing pre-existing baseline contracts (manually imported), a change whose specs describe behavior against that baseline, the writer's alignment output, and the validator's results.

Layer 1 is independently useful: for contract-first changes, it produces machine-readable contracts that the agent and human can review and that the build phase can consume for code generation; for implementation changes, it validates that specs align with predefined baseline contracts and flags mismatches. For external contracts, the operator places normalised files into the change directory; the writer validates alignment and the validator catches structural issues.

### Layer 2 (CLI changes + additional skills)

9. **`specify merge` extension** — copy change-level `contracts/` files into `.specify/contracts/`. Files that share a path are replaced; files absent from the change are left untouched.
10. **`specify spec preview` extension** — include contract file changes in the preview output.
11. **`specify spec conflict-check` extension** — detect baseline contract modifications after the change's `defined-at` timestamp.
12. **`specify validate` extension** — check that `.specify/contracts/schemas/` contains at least one file when the schema declares a `contracts` brief, and that `$ref` pointers in OpenAPI/AsyncAPI files resolve. When registry contract roles are declared, check producer-uniqueness and `produces`/`imports` mutual exclusion.
13. **`workspace sync` extension** — materialise `.specify/contracts/` from the initiating repo into each project clone. Same copy/symlink mechanism used for peer baselines.
14. **`/contracts:importer` skill** — author `plugins/contracts/skills/importer/SKILL.md` with format detection, version upgrade (Swagger 2.0 / OpenAPI 3.0 → 3.1, AsyncAPI 2.x → 3.0), schema decomposition, and Specify metadata injection. Includes reference docs for supported input formats and the normalisation rules.
15. **Registry contract roles** — implement the optional `contracts` block on `registry.yaml` project entries as designed in §*Contract roles in `registry.yaml`*. Add `produces`, `consumes`, and `imports` lists to the registry schema. Update `specify initiative registry validate` and `specify validate` to enforce the invariants (single producer, produce/import mutual exclusion, path validity, self-consistency). Update `/spec:plan` to populate roles when inserting contract changes.
16. **`context` plan entry field (generic context narrowing)** — define the optional `context` field on plan entries (list of paths relative to `.specify/`). `/spec:plan` populates `context` automatically when inserting changes — contract paths, spec baselines, or any other context the define phase should focus on. Briefs use `context` as a focused filter when scanning baseline directories. This is a general-purpose mechanism that applies to contracts, specs, and any future artifact type equally. See §*Generic context narrowing (`context`)* for the full design.

### Layer 3 (future, deferred)

17. **`specify contract validate`** — cross-repo contract compatibility checking against the central contracts.
18. **Contract-validation build brief** — automated verification during build that generated code matches the contract.
19. **Contract-aware `/spec:verify`** — drift detection between baseline contracts and implementation code.

## Implementation order

1. Author the `/contracts:writer` skill (item 1). This is the core deliverable — the alignment-validation and delta-production algorithm, plus reference docs for JSON Schema, OpenAPI, and AsyncAPI conventions.
2. Author the `/contracts:validator` skill (item 2). Post-generation consistency checks that the brief's verify-repair loop acts on.
3. Author the `contracts.md` brief (item 3). Thin orchestrator wiring skill delegation and the verify-repair loop. Depends on steps 1–2.
4. Update the `specs.md` brief (item 4). Add the baseline-contracts-as-context instruction.
5. Update the `design.md` brief (item 5). A small `needs` change plus section rewording.
6. Update the base schemas (item 6). Add the `contracts` pipeline entry and brief to `schemas/omnia/` and `schemas/vectis/`.
7. Author the generation fixture (item 7). Validates the writer and validator against a representative new-API input.
8. Author the conformance fixture (item 8). Validates the writer's alignment checking and validator against pre-existing baseline contracts.

Steps 1–8 can ship without any CLI changes. Steps 9–16 require specify-cli changes and can follow independently. Steps 14–16 (importer, registry roles, `context` field) can land in any order once the CLI merge/validate extensions are in place.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md)
- [RFC-2: Execution](archive/rfc-2-execution.md)
- [RFC-3a: Initiative Planning](archive/rfc-3a-monoliths.md)
- [RFC-3b: Platform Changes](archive/rfc-3b-platform.md)
- [OpenAPI 3.1 Specification](https://spec.openapis.org/oas/v3.1.0)
- [AsyncAPI 3.0 Specification](https://www.asyncapi.com/docs/reference/specification/v3.0.0)
- [JSON Schema](https://json-schema.org/specification)

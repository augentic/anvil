# RFC-8: API Contracts

> Status: Draft · Depends: [RFC-1](archive/rfc-1-cli.md), [RFC-2](archive/rfc-2-execution.md), [RFC-3](archive/rfc-3a-monoliths.md)

## Abstract

Introduce machine-readable API contracts as a first-class, platform-level artifact in Specify. Contracts capture the interface shapes that behavioral specs describe — request/response payloads, message envelopes, error types — in a format that tooling can validate and generate code from. JSON Schema defines the shared payload vocabulary; OpenAPI and AsyncAPI provide protocol-specific bindings for HTTP endpoints and messaging respectively.

Contracts are co-located with `registry.yaml` and `plan.yaml` at `.specify/contracts/`. They are a platform concern — they describe interfaces *between* components, not internals of any one — and every project references the same central contracts, including the implementer of the API itself. A `contracts` brief in the define pipeline reads the current baseline contracts and proposes the minimal set of changes a given change requires.

For multi-repo initiatives, contracts are defined via a dedicated **contract change** in the plan — a regular Specify change whose specs are interface-level behavioral requirements and whose `contracts` brief derives the machine-readable shapes from them. Implementation changes on both sides depend on the contract change, enabling parallel execution. For single-repo projects, contracts are derived inline during a change's define phase. When a contract is mandated by an external system or inherited from a legacy system being migrated, it is imported into the baseline and the `contracts` brief validates that specs conform to the pre-existing interface rather than generating new artifacts. All three patterns use the same brief and the same central location; the difference is plan structure and brief mode, not mechanism.

No CLI changes are required for the initial implementation; baseline tracking and cross-repo distribution are layered on top.

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

### Working contracts during define

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
2. **`contracts` → `design`**: The design document references contracts rather than re-describing API shapes in prose. The `design` brief declares `needs: [proposal, contracts]` (adding `contracts` to its existing `needs`). The `## API Contracts` and `## Publication & Timing Patterns` sections in `design.md` become pointers to the contract files rather than hand-authored descriptions.
3. **`contracts` → `tasks`**: Task generation can reference contracts for code-generation tasks (e.g. "generate Rust types from `contracts/schemas/`"). The `tasks` brief already declares `needs: [specs, design]`; adding `contracts` is optional — design transitively carries the contract context.

#### External contracts and the specs brief

When the baseline at `.specify/contracts/` contains pre-existing contracts (imported from an external system or a preceding import change), the `specs` brief benefits from seeing them as context — spec authors should write behavioral requirements that *conform to* the existing interface shapes rather than inventing new ones.

The `specs` brief gains an optional context dependency on baseline contracts:

```yaml
---
id: specs
description: Behavioral requirements
generates: specs/**/*.md
needs: [proposal]
context: [contracts]   # read-only; does not block if absent
---
```

The `context` field is distinct from `needs`: a `needs` dependency is required and blocks execution if the artifact is missing; a `context` dependency is advisory and provides the artifact when it exists. When baseline contracts are present, the specs brief reads them and instructs the agent to write scenarios consistent with the existing endpoint paths, payload schemas, and error responses. When no baseline contracts exist (the common case for new APIs), the field has no effect.

This preserves the `specs → contracts` pipeline ordering — specs still run before the `contracts` brief — while giving spec authors visibility into external contracts that their requirements must conform to.

### Brief frontmatter

```yaml
---
id: contracts
description: Evolve platform API contracts or validate conformance to external contracts
generates: contracts/**/*.yaml
needs: [specs]
---
```

The `generates` glob (`contracts/**/*.yaml`) scopes output to the change-level `contracts/` directory. The brief reads the baseline at `.specify/contracts/` and the registry roles as context, then operates in one of two modes: **generation** (deriving new contracts from specs) or **conformance** (validating specs against pre-existing baseline contracts). See rule 8 in §*Contract generation rules* for the mode-selection heuristic.

### Contract generation rules

The `contracts` brief body instructs the agent to:

1. **Read the baseline contracts** at `.specify/contracts/` to understand the current platform vocabulary — existing domain types, HTTP bindings, and messaging bindings.

2. **Read all spec files** under `.specify/changes/<name>/specs/` and identify requirements that describe API interactions (HTTP endpoints, request/response patterns) or message exchanges (pub/sub, event-driven patterns).

3. **Determine the minimal contract delta.** Compare what the specs require against the existing baseline. Identify new domain types, modified types, new endpoints or channels, and modified bindings. Only produce files for what this change adds or modifies.

4. **Generate JSON Schema files** for new or modified domain types, each with:
   - `$id` for stable cross-referencing
   - `title` matching the type name
   - `description` from the spec's behavioral description
   - `properties`, `required`, and type constraints derived from scenario data
   - `$ref` pointers for shared sub-types (referencing baseline schemas where they already exist)

5. **Generate or update OpenAPI spec** (when applicable). For changes whose specs describe HTTP interactions, produce contract files under `contracts/http/` with:
   - Paths and methods derived from spec scenarios
   - Request/response schemas as `$ref` pointers to `../schemas/`
   - Error responses derived from the spec's error conditions
   - OpenAPI 3.1 format (native JSON Schema support)

6. **Generate or update AsyncAPI spec** (when applicable). For changes whose specs describe messaging interactions, produce contract files under `contracts/messages/` with:
   - Channels and operations derived from spec scenarios
   - Message payload schemas as `$ref` pointers to `../schemas/`
   - AsyncAPI 3.0 format (native JSON Schema support)

7. **Validate internal consistency.** All `$ref` pointers in OpenAPI and AsyncAPI files must resolve — either to files in the change's `contracts/schemas/` or to existing files in the baseline `.specify/contracts/schemas/`. The agent verifies this before completing the brief.

8. **Conformance mode (external contracts).** When the baseline contains contracts that already cover the interface described by the change's specs — i.e. the specs describe behavior against a pre-existing API rather than defining a new one — the brief switches to conformance mode:
   - **Validate alignment.** For each spec scenario that describes an API interaction, verify that the endpoint path, HTTP method, request/response payload shapes, and error codes match the baseline contract. For messaging scenarios, verify that the channel, operation, and message payload match the baseline AsyncAPI definition. Flag mismatches as warnings for human review.
   - **Suppress generation for covered interfaces.** Do not generate new contract files for endpoints, channels, or schemas that already exist in the baseline. The external contract is authoritative for its interfaces.
   - **Generate delta for extensions only.** If specs describe interactions that go beyond the baseline contract (new endpoints added during a migration, additional message channels), generate contract files for those additions following the standard rules above.
   - **Normalise imported files.** If imported contract files lack Specify conventions (missing `$id` on schemas, inconsistent `description` fields), propose a normalisation delta that adds the missing metadata without changing the interface shapes. This delta is written to the change's `contracts/` directory as replacements for the imported files.

   The brief detects conformance mode by comparing the set of API interactions in the specs against the set of bindings in the baseline contracts. When the overlap is substantial (the majority of spec scenarios describe interactions already present in the baseline), the brief operates in conformance mode. When the overlap is minimal or absent, it operates in the standard generation mode. The brief reports which mode it selected and why.

### Relationship to `design.md`

The Omnia `design.md` brief currently has:

```markdown
## API Contracts
<!-- Endpoints with method, path, request/response shapes, errors -->

## Publication & Timing Patterns
<!-- Topics, message shapes, timing, partition keys -->
```

With the `contracts` brief in place, these sections change role. Instead of being the primary description of interface shapes, they become **references** to the platform contract files with additional implementation-level context (e.g. rate limits, retry policies, authentication schemes) that the contract format does not capture:

```markdown
## API Contracts

See `.specify/contracts/http/` for the full OpenAPI specifications.

<!-- Implementation notes: auth, rate limits, caching, versioning strategy -->

## Publication & Timing Patterns

See `.specify/contracts/messages/` for the full AsyncAPI specifications.

<!-- Implementation notes: ordering guarantees, retry policies, DLQ strategy -->
```

The `design` brief's `needs` gains `contracts` so the agent has the generated files available when writing design.md.

## Authorship patterns

The `contracts` brief handles creating new contracts, evolving existing ones, and validating conformance to externally mandated ones — it reads the baseline and adapts its behavior accordingly. This single mechanism supports three authorship patterns depending on context.

### Contract-first (dedicated contract change)

For multi-repo initiatives or APIs shared with external consumers, contracts are defined as their own change in the plan. A contract change is a regular Specify change that produces interface-level behavioral specs and derives contracts from them. It carries no implementation code — its build phase validates the contract artifacts rather than generating code.

The interface has its own behavioral specs, distinct from any project's internal specs:

> The user registration API SHALL accept a `POST /users` request with a `UserRegistration` payload.
> WHEN the registration succeeds, the API SHALL respond with `201 Created` and a `User` payload including the assigned `id`.
> WHEN the email is already registered, the API SHALL respond with `409 Conflict` and an `ErrorResponse` payload.

These are behavioral requirements for the interface itself — not the producer's internal logic or the consumer's UI flows. The `contracts` brief derives the machine-readable shapes from these specs: JSON Schema for `UserRegistration`, `User`, and `ErrorResponse`; an OpenAPI binding for the endpoint. The spec-driven model is intact: specs describe *what* the interface does; contracts capture *what the interface looks like*.

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

The `/spec:plan` skill can automate this: when it identifies an interface between projects during planning, it inserts a contract change before the implementation changes on both sides.

### Spec-first (inline derivation)

For single-repo projects or changes that don't cross API boundaries, contracts are derived inline during a single change's define phase. The change's specs describe the system's behavior; the `contracts` brief extracts the interface shapes from those specs in the same pipeline run. No separate contract change is needed.

This is the simpler pattern — one change, one define phase, specs and contracts produced together. It applies when there is no multi-party agreement to negotiate.

### Contract-given (external or legacy contracts)

When a contract is mandated by an external system — a partner API, a regulatory interface, or a legacy system being migrated — the derivation direction is reversed. The machine-readable contract already exists or its shape is dictated by a third party, and the specs need to *conform to* that contract rather than *generate* it.

A contract-given change imports the external contract into `.specify/contracts/` and then writes behavioral specs that describe the pre-existing interface. The `contracts` brief operates in **conformance mode**: instead of deriving new schemas from specs, it validates that the specs correctly describe the given contract and proposes only the minimal amendments (e.g. adding `$id` values, aligning `description` fields with spec language) needed to bring the imported files into Specify's structural conventions.

The workflow:

1. **Import.** The external contract files (OpenAPI specs, AsyncAPI specs, JSON Schema definitions) are placed into `.specify/contracts/` — either manually, via a dedicated import step in the plan, or via the RT plugin's `wiretapper` skill for legacy systems. This happens *before* the change's define phase begins.

2. **Define with conformance.** The change's `contracts` brief detects that baseline contracts already cover the interface the specs describe. Rather than generating new contract files, it validates spec-to-contract alignment:
   - Every endpoint or channel described in the specs has a corresponding binding in the baseline contracts.
   - Payload shapes referenced in spec scenarios match the JSON Schema definitions.
   - Error conditions in specs correspond to error responses in the contract.
   - Mismatches are flagged for human review rather than silently overwritten.

3. **Delta for extensions only.** If the change extends the external contract (e.g. adding a new endpoint to a legacy API during migration), the brief produces contract files for the *additions* only. The imported baseline files are not modified — they remain the external system's authoritative shapes.

The plan structure for a migration looks like:

```yaml
changes:
  - name: import-legacy-api
    description: "Import legacy user API contract"
    status: pending

  - name: user-api-backend
    project: backend
    description: "Implement user API on new platform"
    depends-on: [import-legacy-api]
    status: pending
```

The import change carries the external contract files but no implementation code — its build phase validates the contract artifacts structurally (well-formed OpenAPI, resolvable `$ref` pointers) rather than generating code. Implementation changes depend on it and write specs that conform to the imported contract.

### Pattern selection

The choice between patterns is a planning decision, not a mechanism difference. All three use the same `contracts` brief, the same central `.specify/contracts/` location, and the same merge semantics. The brief's behavior adapts to context:

- **Contract-first**: baseline is sparse; the change produces substantial new contracts from interface-level specs.
- **Spec-first**: baseline may already be rich; the change proposes a small delta derived from implementation-level specs.
- **Contract-given**: baseline contains imported external contracts; the brief validates conformance and produces delta only for extensions.

`/spec:plan` applies heuristics to select the pattern: if the plan contains changes in multiple projects that share an API boundary, insert a contract change (contract-first). If a source is flagged as an external system or legacy migration, insert an import change before the implementation changes (contract-given). Otherwise, rely on inline derivation (spec-first).

## Multi-repo contract sharing

Central co-location with `registry.yaml` makes multi-repo contract sharing straightforward. The same `.specify/contracts/` directory serves all projects; distribution uses the existing workspace infrastructure.

### Layer 1: Central contracts with workspace distribution (no CLI changes)

In a multi-repo initiative, contracts live in the initiating repo alongside `registry.yaml`. The plan uses dedicated contract changes (see §*Authorship patterns*) to define interfaces before implementation begins. `workspace sync` distributes `.specify/contracts/` from the initiating repo into each project clone:

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

Each workspace clone gets the central contracts at `.specify/contracts/`. Phase skills always read from `.specify/contracts/` relative to their working directory — they do not need to know whether the contracts were authored locally or materialised from a central source. This preserves RFC-3b's design principle that phase skills are unaware of the multi-repo topology.

When a contract change completes its define-build-merge cycle, its contracts merge into the central `.specify/contracts/`. The driver's pre-change distribution step propagates the updated contracts to all project clones before dependent implementation changes begin their define phases. Implementation changes read the materialised contracts as their baseline; their `contracts` brief proposes only additions or modifications their specs require — typically a small or empty delta, since the interface was already defined by the contract change.

Compatibility is validated by the agent during each implementation change's define phase rather than by automated tooling. This requires no framework changes beyond extending `workspace sync` to include the `contracts/` directory in what it materialises — a single additional path using the same copy/symlink mechanism it already uses for peer baselines.

### Layer 2: Baseline tracking and merge (CLI changes)

To make contracts a persistent, diffable artifact with full merge support:

When `specify merge` processes a change, it copies the change's `contracts/` files into `.specify/contracts/`, replacing files that share a path. Contract files use **opaque replacement** semantics — unlike spec files which use the ADDED/MODIFIED/REMOVED delta format, contract files are replaced wholesale. The rationale: JSON Schema and OpenAPI/AsyncAPI files have their own versioning semantics (`$id`, `info.version`); introducing a second delta-merge algorithm for YAML contract files would add complexity without clear benefit over replacement.

`specify spec preview` and `specify spec conflict-check` are extended to include contract files in their output so operators see when a merge will update contracts.

This layer requires CLI changes:

- `specify merge` copies change-level `contracts/` files into `.specify/contracts/`. Files that share a path are replaced; files absent from the change are left untouched.
- `specify spec preview` reports contract file changes (added/replaced).
- `specify spec conflict-check` detects when baseline contracts have been modified after the change's `defined-at` timestamp.

### Layer 3: Automated contract validation (future)

A future extension could add automated validation that a consumer's usage of a contract is compatible with the producer's definition — for example, verifying that a mobile frontend's API client calls match the backend's OpenAPI spec, or that a message consumer's expected payload matches the producer's AsyncAPI schema. This is analogous to contract testing (Pact, Spring Cloud Contract) but integrated into the Specify workflow.

This layer is explicitly deferred. It requires:

- A `specify contract validate` CLI verb that loads the central contracts and each project's specs, checking compatibility.
- A contract-validation brief in the build pipeline that runs after code generation.
- A definition of "compatibility" that accounts for backwards-compatible changes (additive fields, optional-to-required transitions, etc.).

The central contracts directory makes validation simpler than the per-project model — all contracts are in one place, and all project specs are accessible via workspace clones. The validation rules can be designed against real examples when the need arises.

## Single-repo and multi-repo — same model

The central co-location model works identically in both topologies:

| Concern | Single-repo | Multi-repo |
|---------|-------------|------------|
| Contracts location | `.specify/contracts/` | `.specify/contracts/` (initiating repo) |
| Who writes (new APIs) | `contracts` brief during define (inline) | Dedicated contract change, then `contracts` brief in implementation changes |
| Who writes (external APIs) | Import into baseline, then `contracts` brief validates conformance | Import change in plan, then conformance in implementation changes |
| How projects read | Direct filesystem read | Materialised by `workspace sync` |
| How changes propose updates | `.specify/changes/<name>/contracts/` | Same — in the project clone's change directory |
| How updates merge | `specify merge` → `.specify/contracts/` | Same — then `workspace push` propagates |
| Contract roles | Optional — sole project is implicit producer and consumer | `contracts` block in `registry.yaml` per project (`produces`, `consumes`, `imports`; see §*Contract roles*) |

Phase skills see the same paths regardless of topology. The only difference is the distribution mechanism, which is handled by the existing workspace infrastructure.

## Contract roles in `registry.yaml`

Contracts are platform-level shared artifacts — neither producer nor consumer owns them. Internally produced contracts have exactly one authoritative producer and one or more consumers. Externally imported contracts have no internal producer — the authoritative source is outside the platform. Without an explicit record of these roles, two projects could independently write specs that describe producing the same endpoint or publishing to the same message channel, with no tooling to detect the conflict.

`registry.yaml` gains an optional `contracts` block per project that declares which contracts each project produces, consumes, or imports from external systems:

```yaml
projects:
  - name: backend
    description: "User management API and order processing"
    contracts:
      produces: [http/user-api, messages/order-events]
  - name: mobile
    description: "iOS and Android registration flows"
    contracts:
      consumes: [http/user-api]
  - name: notifications
    description: "Email and push notification delivery"
    contracts:
      consumes: [messages/order-events]
```

When a contract is owned by an external system (a partner API, a legacy system, a third-party service), no project in the registry is the producer. The `imports` field declares contracts whose authoritative source is outside the platform:

```yaml
projects:
  - name: backend
    description: "New platform backend"
    contracts:
      produces: [http/user-api]
      imports: [http/legacy-billing-api]   # external system, no internal producer
  - name: billing-adapter
    description: "Adapter for legacy billing system"
    contracts:
      consumes: [http/legacy-billing-api]
```

An `imports` entry means: this project is responsible for importing and maintaining the contract files, but does not author the interface — the external system does. Imported contracts are not expected to have an internal producer; the validation invariant (§*Validation invariant*) skips the producer-uniqueness check for contracts that appear in any project's `imports` list. A contract path must not appear in both `produces` and `imports` across the registry — it is either internally produced or externally imported, never both.

Contract paths are relative to `.specify/contracts/` (e.g. `http/user-api` refers to `.specify/contracts/http/user-api.yaml`). This completes the triad of platform metadata:

- **`registry.yaml`** declares *who* the participants are and *which contract roles they play*.
- **`plan.yaml`** declares *what* changes are planned.
- **`.specify/contracts/`** declares *what the interfaces look like*.

### Validation invariant

Two rules enforce consistency:

1. **Each contract path has at most one producer.** Multiple consumers are expected and unrestricted.
2. **A contract path must not appear in both `produces` and `imports`.** It is either internally produced or externally imported.

`specify validate` checks both invariants when the registry declares contract roles:

```
error: contract "http/user-api" has multiple producers: backend, payments
error: contract "http/billing-api" appears in both produces (payments) and imports (backend)
```

Contracts that appear only in `imports` lists are exempt from the producer-uniqueness check — they have no internal producer by definition.

The `contracts` brief also reads the registry roles as context. When generating contracts for a change, the brief verifies that the producing project's specs are consistent with its declared role and flags specs that describe producing an interface the project does not own. For imported contracts, the brief operates in conformance mode (see §*Contract generation rules*, rule 8) and flags specs that would modify the external interface shape.

### Role lifecycle

Contract roles are declared when the contract is first defined — typically during a contract-first change or an import change (see §*Authorship patterns*). `/spec:plan` populates the registry's `contracts` block when it inserts contract changes into a plan: the project that will implement the API is recorded as the producer; projects that will consume it are recorded as consumers; when a source is flagged as an external system or legacy migration, the importing project is recorded with an `imports` entry.

For single-repo projects, the roles are implicit (the sole project is both producer and consumer) and the `contracts` block is optional.

Roles evolve as the platform evolves. When a new consumer adopts an existing contract, its registry entry gains a `consumes` reference. When a contract is retired, the producing project removes it from `produces`. When a legacy system is fully migrated and the new platform takes over the API, the `imports` entry is replaced with a `produces` entry — the contract transitions from externally imported to internally produced. These are edits to `registry.yaml` — contract roles change infrequently and warrant human review.

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
- **Contract versioning or backwards-compatibility checking.** Semantic versioning of contracts, breaking-change detection, and consumer compatibility validation are deferred to Layer 3. The initial implementation treats contracts as define-time artifacts that are evolved incrementally.
- **Replacing behavioral specs.** Contracts complement specs; they do not replace them. Specs describe *what* the system must do. Contracts describe *what the interface looks like*. Both are needed — one for requirements traceability, the other for machine-readable integration.
- **Cross-repo contract enforcement in Layer 1.** The initial implementation relies on `depends-on` ordering and agent judgment for cross-repo compatibility. Automated validation is explicitly deferred.
- **Contract ownership by a single project.** Contracts are platform-level shared artifacts. No project "owns" a contract; both producer and consumer reference the same central definition. The registry records which project is the authoritative *producer* of each contract (see §*Contract roles*), but this is a role declaration for conflict prevention, not ownership — the contract artifact remains shared and editable through the normal change process.

## Implementation scope

### Layer 1 (no CLI changes)

1. **`contracts` brief** — author `briefs/contracts.md` with the generation and conformance rules described above. The brief reads baseline contracts from `.specify/contracts/`, detects whether to operate in generation or conformance mode, and writes proposed changes into the change directory.
2. **Updated `specs` brief** — add `context: [contracts]` to the `specs` brief frontmatter so spec authors see pre-existing baseline contracts when writing behavioral requirements for external interfaces.
3. **Updated `design` brief** — modify `briefs/design.md` to declare `needs: [proposal, contracts]` and update the `## API Contracts` / `## Publication & Timing Patterns` sections to reference the central contract files.
4. **Child schemas** — create `schemas/omnia-contracts/` and `schemas/vectis-contracts/` with `extends` and the `contracts` pipeline entry.
5. **Fixture (generation)** — author a worked example under `schemas/omnia-contracts/fixtures/` showing the baseline contracts and a change-level delta for a representative capability.
6. **Fixture (conformance)** — author a worked example showing an imported external contract, a change whose specs conform to it, and the conformance validation output.
7. **Registry contract roles** — define the optional `contracts` block schema for `registry.yaml` project entries (`produces`, `consumes`, and `imports` lists). Update `/spec:plan` to populate roles when inserting contract changes — including `imports` entries when a source is flagged as an external system. Update the `contracts` brief to read registry roles as context and trigger conformance mode for imported contracts.

Layer 1 is independently useful: it produces machine-readable contracts during define that the agent and human can review, and that the build phase can consume for code generation. For external contracts, it validates conformance and flags mismatches before implementation begins.

### Layer 2 (CLI changes)

8. **`specify merge` extension** — copy change-level `contracts/` files into `.specify/contracts/`. Files that share a path are replaced; files absent from the change are left untouched.
9. **`specify spec preview` extension** — include contract file changes in the preview output.
10. **`specify spec conflict-check` extension** — detect baseline contract modifications after the change's `defined-at` timestamp.
11. **`specify validate` extension** — check that `.specify/contracts/schemas/` contains at least one file when the schema declares a `contracts` brief, that `$ref` pointers in OpenAPI/AsyncAPI files resolve, that each contract path has at most one producer in `registry.yaml`, and that no contract path appears in both `produces` and `imports` (see §*Contract roles*).
12. **`workspace sync` extension** — materialise `.specify/contracts/` from the initiating repo into each project clone. Same copy/symlink mechanism used for peer baselines.

### Layer 3 (future, deferred)

13. **`specify contract validate`** — cross-repo contract compatibility checking against the central contracts.
14. **Contract-validation build brief** — automated verification during build that generated code matches the contract.

## Implementation order

1. Author the `contracts.md` brief (Layer 1, item 1). This is the core deliverable — the brief body contains the generation and conformance rules and the agent follows them during define.
2. Update the `specs.md` brief (Layer 1, item 2). Add `context: [contracts]` for baseline contract visibility during spec authoring.
3. Update the `design.md` brief (Layer 1, item 3). A small `needs` change plus section rewording.
4. Create the child schemas (Layer 1, item 4). Schema composition handles the pipeline insertion.
5. Author the generation fixture (Layer 1, item 5). Validates the brief against a representative new-API input.
6. Author the conformance fixture (Layer 1, item 6). Validates the brief's conformance mode against an imported external contract.
7. Define registry contract roles (Layer 1, item 7). Schema for the `contracts` block (`produces`, `consumes`, `imports`), plus `/spec:plan` and `contracts` brief updates.
8. Extend `specify merge` (Layer 2, item 8). The merge engine gains a contract-copy step targeting `.specify/contracts/`.
9. Extend `specify spec preview` and `conflict-check` (Layer 2, items 9–10). Preview and conflict surfaces gain contract awareness.
10. Extend `specify validate` (Layer 2, item 11). Structural validation of contract artifacts, producer-uniqueness checking, and `produces`/`imports` mutual exclusion.
11. Extend `workspace sync` (Layer 2, item 12). Distribution of central contracts to project clones.

Steps 1–7 can ship without any CLI changes. Steps 8–11 require specify-cli changes and can follow independently.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md)
- [RFC-2: Execution](archive/rfc-2-execution.md)
- [RFC-3a: Initiative Planning](archive/rfc-3a-monoliths.md)
- [RFC-3b: Platform Changes](archive/rfc-3b-platform.md)
- [OpenAPI 3.1 Specification](https://spec.openapis.org/oas/v3.1.0)
- [AsyncAPI 3.0 Specification](https://www.asyncapi.com/docs/reference/specification/v3.0.0)
- [JSON Schema](https://json-schema.org/specification)

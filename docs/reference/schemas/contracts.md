# Contracts Schema

- **URL:** `https://github.com/augentic/specify/schemas/contracts`
- **Purpose:** Dedicated API contract changes -- defining or importing machine-readable interface shapes
- **Target:** Contract artifacts (JSON Schema, OpenAPI 3.1, AsyncAPI 3.0) at root `contracts/`

## Brief pipeline

### Define phase

| Brief | Output | Dependencies |
|-------|--------|-------------|
| `proposal.md` | `proposal.md` | -- |
| `specs.md` | `specs/<capability>/spec.md` | proposal |
| `tasks.md` | `tasks.md` | specs |

There is no `design` stage. Contract changes define interface shapes, not implementation design. Implementation-level concerns (auth schemes, retry policies, caching strategies) belong in the implementing project's change, not in the contract change.

### Build phase

| Brief | Skills invoked |
|-------|---------------|
| `build.md` | `/contract:openapi`, `/contract:asyncapi`, `/contract:json-schema` (author, importer, and verifier intents) |

The build brief delegates to the relevant `/contract:*` skill (`/contract:openapi` for HTTP / resource APIs, `/contract:asyncapi` for evented / pub-sub / streaming, `/contract:json-schema` for shared payload schemas). It runs author intent for prose-derived specs, importer intent for supplied contract artifacts, and verifier intent for structural correctness -- `$ref` resolution, schema metadata, and binding completeness. There are no implementation code-generation skills to invoke because contract changes produce only contract artifacts.

A verify-repair loop runs up to 2 iterations: if the verifier reports failures, the same skill's producing intent (author or importer) makes targeted repairs, then the verifier re-checks. If issues remain after 2 iterations, they are surfaced for human review.

### Merge phase

| Brief | Skills invoked |
|-------|---------------|
| `merge.md` | -- (standard merge operations) |

Contract files use **opaque replacement** semantics during merge -- the entire file is replaced rather than delta-merged. When `specify merge` processes the change, it copies the change's `contracts/` files into root `contracts/`, replacing files that share a path.

## When to use

Use the `contracts` schema when:

- **Contract-first:** Defining a new API contract before implementation begins. `/spec:plan` inserts these automatically when it detects an API boundary between projects.
- **Contract-given:** Importing an external or legacy API contract into the platform. The operator places the external files into the change's `contracts/` directory.
- **Standalone modification:** Modifying existing platform contracts independently of implementation changes.

Use Omnia or Vectis schemas when implementing code that conforms to existing contracts. Their specs and design briefs read baseline contracts as context, but implementation changes do not author contract deltas. Use a separate `contracts@v1` change when an implementation needs a new or changed interface shape.

## Contracts schema vs implementation schemas

The `contracts` schema and the implementation schemas serve complementary purposes:

| Concern | Contracts schema | Omnia/Vectis schemas |
|---------|-----------------|--------------------------------|
| Purpose | Author or import contract artifacts | Implement code that conforms to baseline contracts |
| Plan entry | `schema: contracts@v1` (no `project`) | Normal project-bound entry |
| Build phase | Author/import + validation | Code generation |
| Typical delta | Full contract set (new API), import normalisation, or contract modification | Spec/design/code changes with no contract artifact delta |

## Domain context

The Contracts schema injects domain context about:

- JSON Schema (draft 2020-12) conventions for payload definitions.
- OpenAPI 3.1 structure for HTTP endpoint bindings.
- AsyncAPI 3.0 structure for messaging bindings.
- Artifact structure and naming conventions for root `contracts/`.

## Schema framework

For general schema concepts -- directory structure, field reference for `schema.yaml`, schema resolution, composition, caching, and rules override -- see the [Schemas overview](index.md).

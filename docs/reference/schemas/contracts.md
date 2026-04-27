# Contracts Schema

- **URL:** `https://github.com/augentic/specify/schemas/contracts`
- **Purpose:** Dedicated API contract changes -- defining or importing machine-readable interface shapes
- **Target:** Contract artifacts (JSON Schema, OpenAPI 3.1, AsyncAPI 3.0) at `.specify/contracts/`

## Brief pipeline

### Define phase

| Brief | Output | Dependencies |
|-------|--------|-------------|
| `proposal.md` | `proposal.md` | -- |
| `specs.md` | `specs/<capability>/spec.md` | proposal |
| `contracts.md` | `contracts/**/*.yaml` | specs |
| `tasks.md` | `tasks.md` | specs, contracts |

There is no `design` stage. Contract changes define interface shapes, not implementation design. Implementation-level concerns (auth schemes, retry policies, caching strategies) belong in the implementing project's change, not in the contract change.

### Build phase

| Brief | Skills invoked |
|-------|---------------|
| `build.md` | `/contracts:validator` |

The build brief delegates to `/contracts:validator` to verify structural correctness -- `$ref` resolution, schema metadata, binding completeness. There are no code-generation skills to invoke because contract changes produce no implementation code.

A verify-repair loop runs up to 2 iterations: if the validator reports failures, `/contracts:writer` makes targeted repairs, then the validator re-checks. If issues remain after 2 iterations, they are surfaced for human review.

### Merge phase

| Brief | Skills invoked |
|-------|---------------|
| `merge.md` | -- (standard merge operations) |

Contract files use **opaque replacement** semantics during merge -- the entire file is replaced rather than delta-merged. When `specify merge` processes the change, it copies the change's `contracts/` files into `.specify/contracts/`, replacing files that share a path.

## When to use

Use the `contracts` schema when:

- **Contract-first:** Defining a new API contract before implementation begins. `/spec:plan` inserts these automatically when it detects an API boundary between projects.
- **Contract-given:** Importing an external or legacy API contract into the platform. The operator places the external files into the change's `contracts/` directory.
- **Standalone modification:** Modifying existing platform contracts independently of implementation changes.

Use Omnia or Vectis schemas when implementing code that conforms to existing contracts. The `contracts` brief in those schemas validates alignment automatically -- you do not need a separate contract change for alignment validation.

## Contracts schema vs contracts brief

The `contracts` schema and the `contracts` brief in Omnia/Vectis serve complementary purposes:

| Concern | Contracts schema | Contracts brief in Omnia/Vectis |
|---------|-----------------|--------------------------------|
| Purpose | Author or import contract artifacts | Validate spec alignment with baseline contracts |
| Plan entry | `schema: contracts@v1` (no `project`) | Normal project-bound entry |
| Build phase | Validation only | Code generation |
| Typical delta | Full contract set (new API) or import normalisation | Small or empty (alignment confirmation) |

## Domain context

The Contracts schema injects domain context about:

- JSON Schema (draft 2020-12) conventions for payload definitions.
- OpenAPI 3.1 structure for HTTP endpoint bindings.
- AsyncAPI 3.0 structure for messaging bindings.
- Artifact structure and naming conventions for `.specify/contracts/`.

## Schema framework

For general schema concepts -- directory structure, field reference for `schema.yaml`, schema resolution, composition, caching, and rules override -- see the [Schemas overview](index.md).

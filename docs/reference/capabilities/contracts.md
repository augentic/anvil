# Contracts Capability

- **Identifier:** `contracts` (bundled, first-party)
- **URL:** `https://github.com/augentic/specify/capabilities/contracts`
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
| `merge.md` | `specify-merge` driver + `specify tool run contract` post-merge gate |

Contract files use **opaque replacement** semantics during merge -- the entire file is replaced rather than delta-merged. When `specify slice merge run` processes the slice, it copies the slice's `contracts/` files into root `contracts/`, replacing files that share a path.

After the standard delta merge succeeds, the merge brief shells out to the declared [`contract` WASI tool](../cli/contract.md) with `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json`. The tool enforces the RFC-12 §Validation rules (SemVer `info.version`, kebab-case `info.x-specify-id` when present, cross-repo id uniqueness) and is the contracts capability's adoption gate per RFC-13 §"Merge and adoption contract". The merge brief maps the tool's exit code to the §Merge and adoption contract three-branch outcome contract (`success` / `failure` / `deferred`); see [`capabilities/contracts/briefs/merge.md`](../../../capabilities/contracts/briefs/merge.md) for the full wiring.

## When to use

Use the `contracts` capability when:

- **Contract-first:** Defining a new API contract before implementation begins. `/change:plan` inserts these automatically when it detects an API boundary between projects.
- **Contract-given:** Importing an external or legacy API contract into the platform. The operator places the external files into the slice's `contracts/` directory.
- **Standalone modification:** Modifying existing platform contracts independently of implementation slices.

Use the Omnia or Vectis capabilities when implementing code that conforms to existing contracts. Their specs and design briefs read baseline contracts as context, but implementation slices do not author contract deltas. Use a separate `contracts@v1` change when an implementation needs a new or changed interface shape.

## Contracts capability vs implementation capabilities

The `contracts` capability and the implementation capabilities serve complementary purposes:

| Concern | Contracts capability | Omnia/Vectis capabilities |
|---------|----------------------|---------------------------|
| Purpose | Author or import contract artifacts | Implement code that conforms to baseline contracts |
| Plan entry | `schema: contracts@v1` (no `project`) | Normal project-bound entry |
| Build phase | Author/import + validation | Code generation |
| Typical delta | Full contract set (new API), import normalisation, or contract modification | Spec/design/code changes with no contract artifact delta |

> The plan-entry key on the table above is still spelled `schema:` because the `plan.yaml` per-entry field name is intentionally kept distinct from the capability rename — it identifies the artefact-path identifier the entry targets, not the capability that owns the work. RFC-13 leaves that key unchanged; any future plan-field rename is a separate cut-over.

## Domain context

The Contracts capability's briefs and skills carry domain context about:

- JSON Schema (draft 2020-12) conventions for payload definitions.
- OpenAPI 3.1 structure for HTTP endpoint bindings.
- AsyncAPI 3.0 structure for messaging bindings.
- Artifact structure and naming conventions for root `contracts/`.

## Capability framework

For general capability concepts -- directory structure, manifest field reference for `capability.yaml`, capability resolution, and pipeline declarations -- see the [Capabilities overview](index.md).

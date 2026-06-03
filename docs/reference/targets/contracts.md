# Contracts Adapter

- **Identifier:** `contracts` (bundled, first-party)
- **URL:** `https://github.com/augentic/specify/adapters/targets/contracts`
- **Purpose:** Dedicated API contract changes -- defining or importing machine-readable interface shapes
- **Target:** Contract artifacts (JSON Schema, OpenAPI 3.1, AsyncAPI 3.0) at root `contracts/`

## Brief pipeline

### Define phase

| Brief | Output | Dependencies |
|-------|--------|-------------|
| `proposal.md` | `proposal.md` | -- |
| `specs.md` | `specs/<unit>/spec.md` | proposal |
| `tasks.md` | `tasks.md` | specs |

There is no `design` stage. Contract changes define interface shapes, not implementation design. Implementation-level concerns (auth schemes, retry policies, caching strategies) belong in the implementing project's change, not in the contract change.

### Build phase

| Brief | Skills invoked |
|-------|---------------|
| `build.md` | `openapi`, `asyncapi`, and `json-schema` format sub-flows (author, importer, and verifier intents) |

The build brief dispatches to the relevant format sub-flow from [`adapters/targets/contracts/briefs/build.md`](../../../adapters/targets/contracts/briefs/build.md): `openapi` for HTTP / resource APIs, `asyncapi` for evented / pub-sub / streaming, and `json-schema` for shared payload schemas. It runs author intent for prose-derived specs, importer intent for supplied contract artifacts, and verifier intent for structural correctness -- `$ref` resolution, schema metadata, and binding completeness. There are no implementation code-generation skills to invoke because contract changes produce only contract artifacts.

A verify-repair loop runs up to 2 iterations: if the verifier reports failures, the same skill's producing intent (author or importer) makes targeted repairs, then the verifier re-checks. If issues remain after 2 iterations, they are surfaced for human review.

### Merge phase

| Brief | Skills invoked |
|-------|---------------|
| `merge.md` | `specify-merge` driver + `specify tool run contract` post-merge gate |

Contract files use **opaque replacement** semantics during merge -- the entire file is replaced rather than delta-merged. When `specify slice merge run` processes the slice, it copies the slice's `contracts/` files into root `contracts/`, replacing files that share a path.

After the standard delta merge succeeds, the merge brief shells out to the declared [`contract` WASI tool](../cli/contract.md) with `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json`. The tool enforces the contract validation rules (SemVer `info.version`, kebab-case `info.x-specify-id` when present, cross-repo id uniqueness) and is the contracts adapter's adoption gate. The merge brief maps the tool's exit code to the three-branch merge outcome (`success` / `failure` / `deferred`); see [`adapters/targets/contracts/briefs/merge.md`](../../../adapters/targets/contracts/briefs/merge.md) for the full wiring.

## When to use

Use the `contracts` adapter when:

- **Contract-first:** Defining a new API contract before implementation begins. Operators bind a contracts-only source (or rely on the contracts target's `shape` brief) when planning the change.
- **Contract-given:** Importing an external or legacy API contract into the platform. The operator places the external files into the slice's `contracts/` directory.
- **Standalone modification:** Modifying existing platform contracts independently of implementation slices.

Use the Omnia or Vectis adapters when implementing code that conforms to existing contracts. Their specs and design briefs read baseline contracts as context, but implementation slices do not author contract deltas. Use a separate `contracts@v1` change when an implementation needs a new or changed interface shape.

## Contracts adapter vs implementation adapters

The `contracts` adapter and the implementation adapters serve complementary purposes:

| Concern | Contracts adapter | Omnia/Vectis adapters |
|---------|----------------------|---------------------------|
| Purpose | Author or import contract artifacts | Implement code that conforms to baseline contracts |
| Plan entry | `adapter: contracts@v1` (no `project`) | Normal project-bound entry |
| Build phase | Author/import + validation | Code generation |
| Typical delta | Full contract set (new API), import normalisation, or contract modification | Spec/design/code changes with no contract artifact delta |

> The plan-entry key on the table above is spelled `adapter:` because it identifies the artefact-path identifier the entry targets, not the adapter that owns the work.

## Domain context

The Contracts adapter's briefs and skills carry domain context about:

- JSON Schema (draft 2020-12) conventions for payload definitions.
- OpenAPI 3.1 structure for HTTP endpoint bindings.
- AsyncAPI 3.0 structure for messaging bindings.
- Artifact structure and naming conventions for root `contracts/`.
- Rules under [`adapters/targets/contracts/rules/`](../../../adapters/targets/contracts/rules/) for stable `IFACE-*` reviewer guidance on compatibility, versioning, and consumer-impact classification.

## Adapter framework

For general adapter concepts -- directory structure, manifest field reference for `adapter.yaml`, adapter resolution, and pipeline declarations -- see the [Adapters overview](index.md).

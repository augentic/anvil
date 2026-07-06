# Contracts Adapter

- **Identifier:** `contracts` (bundled, first-party)
- **URL:** `https://github.com/augentic/specify/adapters/targets/contracts`
- **Purpose:** Dedicated API contract changes -- defining or importing machine-readable interface shapes
- **Target:** Contract artifacts (JSON Schema, OpenAPI 3.1, AsyncAPI 3.0) at root `contracts/`

## Operations

The Contracts target implements exactly three operations — `guidance`, `build`, `merge` — matching its [`adapter.yaml`](https://github.com/augentic/specify-adapters/blob/main/targets/contracts/adapter.yaml). Core `/spec:refine` synthesises the canonical artifacts (`proposal.md` / `spec.md` / `design.md` / `tasks.md`); the target adapter never writes them.

### guidance

`guidance` is idiom guidance read into context when core synthesis writes `spec.md` and `design.md` for a `target: contracts` slice — see [`adapters/targets/contracts/prose/prompts/guidance.md`](https://github.com/augentic/specify-adapters/blob/main/targets/contracts/prose/prompts/guidance.md). The prompt is input to synthesis: it does not read sources or write artifacts. It tells the synthesiser to shape `spec.md` around what the contract promises (endpoints, channels, payloads, error responses, status codes) and `design.md` around how the contract is expressed and constrained — the format choice (OpenAPI 3.1 / AsyncAPI 3.0 / JSON Schema 2020-12), the file layout under `contracts/`, cross-contract dependencies, and the merge-gate validation rules (SemVer `info.version`, kebab-case `info.x-specify-id`, cross-repo id uniqueness).

Contract changes define interface shapes, not implementation design, so synthesis must keep application-layer concerns (auth schemes, retry policies, caching strategies, provider traits, crate layout) out of the artifacts — those belong in the implementing project's change or to the Omnia / Vectis target shapes.

### build

| Brief | Format sub-flows |
|-------|------------------|
| [`build.md`](https://github.com/augentic/specify-adapters/blob/main/targets/contracts/prose/prompts/build.md) | `json-schema`, `openapi`, and `asyncapi` (each with author / import / verify intents) |

The build prompt dispatches to the relevant format sub-flow from [`adapters/targets/contracts/prose/prompts/build.md`](https://github.com/augentic/specify-adapters/blob/main/targets/contracts/prose/prompts/build.md): `openapi` for HTTP / resource APIs, `asyncapi` for evented / pub-sub / streaming, and `json-schema` for shared payload schemas. It runs author intent for prose-derived specs, importer intent for supplied contract artifacts, and verifier intent for structural correctness -- `$ref` resolution, schema metadata, and binding completeness. There are no implementation code-generation skills to invoke because contract changes produce only contract artifacts under the slice-local `contracts/` directory.

A verify-repair loop runs up to 2 iterations: if the verifier reports failures, the same producing intent (author or importer) makes targeted repairs, then the verifier re-checks. If issues remain after 2 iterations, they are surfaced for human review. A final tool gate runs the contracts adapter's in-guest validator against the slice delta. `build` writes the result to `build/report.yaml`; the build orchestration's finalize tail owns the `built` transition.

### merge

The merge prompt lands the built slice through `specify slice merge` per the shared [`/spec:merge`](../../../plugins/spec/skills/merge/SKILL.md) skill body, then runs the contracts adoption gate — see [`adapters/targets/contracts/prose/prompts/merge.md`](https://github.com/augentic/specify-adapters/blob/main/targets/contracts/prose/prompts/merge.md).

Contract files use **opaque replacement** semantics during merge -- the entire file is replaced rather than delta-merged. When `specify slice merge run` processes the slice, it copies the slice's `contracts/` files into root `contracts/`, replacing files that share a path.

After the standard delta merge succeeds, the merge prompt runs the in-guest [`contract` validator](../cli/contract.md) against `$PROJECT_ROOT/contracts`. The tool enforces the contract validation rules (SemVer `info.version`, kebab-case `info.x-specify-id` when present, cross-repo id uniqueness) and is the contracts adapter's adoption gate. The merge prompt maps the tool's exit code to the three-branch merge outcome (`success` / `failure` / `deferred`); see [`adapters/targets/contracts/prose/prompts/merge.md`](https://github.com/augentic/specify-adapters/blob/main/targets/contracts/prose/prompts/merge.md) for the full wiring.

## When to use

Use the `contracts` adapter when:

- **Contract-first:** Defining a new API contract before implementation begins. Operators bind a contracts-only source (or rely on the contracts target's `guidance` prompt) when planning the change.
- **Contract-given:** Importing an external or legacy API contract into the platform. The operator places the external files into the slice's `contracts/` directory.
- **Standalone modification:** Modifying existing platform contracts independently of implementation slices.

Use the Omnia or Vectis adapters when implementing code that conforms to existing contracts. Their `guidance` prompts guide core synthesis to read baseline contracts as context, but implementation slices do not author contract deltas. Use a separate `contracts@1.0.0` change when an implementation needs a new or changed interface shape.

## Contracts adapter vs implementation adapters

The `contracts` adapter and the implementation adapters serve complementary purposes:

| Concern | Contracts adapter | Omnia/Vectis adapters |
|---------|----------------------|---------------------------|
| Purpose | Author or import contract artifacts | Implement code that conforms to baseline contracts |
| Plan entry | `adapter: contracts@1.0.0` (no `project`) | Normal project-bound entry |
| Build phase | Author/import + validation | Code generation |
| Typical delta | Full contract set (new API), import normalisation, or contract modification | Spec/design/code changes with no contract artifact delta |

> The plan-entry key on the table above is spelled `adapter:` because it identifies the artefact-path identifier the entry targets, not the adapter that owns the work.

## Domain context

The Contracts adapter's prompts and references carry domain context about:

- JSON Schema (draft 2020-12) conventions for payload definitions.
- OpenAPI 3.1 structure for HTTP endpoint bindings.
- AsyncAPI 3.0 structure for messaging bindings.
- Artifact structure and naming conventions for root `contracts/`.
- Rules under [`adapters/targets/contracts/prose/rules/`](https://github.com/augentic/specify-adapters/tree/main/targets/contracts/rules/) for stable `IFACE-*` reviewer guidance on compatibility, versioning, and consumer-impact classification.

## Adapter framework

For general adapter concepts -- directory structure, manifest field reference for `adapter.yaml`, adapter resolution, and pipeline declarations -- see the [Adapters overview](index.md).

# RFC-12: Interfaces

> Status: Draft · Depends: [RFC-8](archive/rfc-8-api-contracts.md), [RFC-9](archive/rfc-9-platform.md), [RFC-10](archive/rfc-10-skills.md)

## Abstract

RFC-8 introduced machine-readable API contracts as first-class Specify artifacts under `.specify/contracts/`. RFC-10 then renamed the Cursor plugin surface from `/contracts:*` to `/interfaces:*` while preserving the persisted artifact names: the `contracts` brief id, the `contracts@v1` schema, and the `.specify/contracts/` baseline directory.

This RFC completes that integration by making **interfaces** the product concept and **contracts** the persisted artifact format. It also revisits RFC-8's central-only placement decision and adopts a hybrid model: `.specify/contracts/` remains the canonical Specify baseline, while registry metadata records producer, consumer, importer, and external-authority relationships.

The result is a single framework story:

- specs describe behavior;
- interface contracts describe wire shape;
- the registry records who produces, consumes, or imports each interface;
- workspace sync materializes the same baseline contracts into each project;
- review and execute surface compatibility findings consistently.

## Motivation

RFC-8 correctly identified the missing layer between behavioral specs and implementation: wire-level interface shape. Since then, the model has partly landed.

- `contracts@v1` exists for dedicated interface changes.
- Omnia and Vectis define pipelines include a `contracts` stage.
- `/interfaces:openapi`, `/interfaces:asyncapi`, and `/interfaces:json-schema` replaced the older lifecycle-oriented `/contracts:*` surface.
- Cross-project compatibility warnings now run after producer contract merges.

What remains incomplete is the framework-level story.

- "Contracts" and "interfaces" are still semantically split across docs, schemas, skills, and registry language.
- RFC-8's central-only contract home is neutral, but it under-describes producer responsibility and external authority.
- Cross-project validation is warning-only and not yet part of a broader review surface.
- Registry data can list produced and consumed files, but lacks a logical interface inventory.
- Contract lifecycle is under-specified: there is no standard way to deprecate, retire, version, or delete interface shapes.
- Code generation can consume contracts, but the source-of-truth boundary between contract artifacts and generated code is not explicit enough for production use.

This RFC treats RFC-8 as the foundation, not a mistake. The central baseline remains the right merge target for Specify, but the framework needs clearer vocabulary and richer metadata around that baseline.

## Terminology

Specify adopts the following terms:

- **Interface**: a logical API, message boundary, or reusable payload boundary between systems.
- **Contract**: a machine-readable artifact describing an interface, using JSON Schema, OpenAPI, or AsyncAPI.
- **Baseline contracts**: merged contract artifacts under `.specify/contracts/`.
- **Contract delta**: change-local proposed contract files under `.specify/changes/<name>/contracts/`.
- **Interface inventory**: registry metadata that maps logical interfaces to contract files and project roles.

The distinction is deliberate. Operators and planning language should talk about interfaces because that is the domain concept. The filesystem continues to use `contracts/` because those files are concrete artifacts.

## Decision

The home model for interface contracts is **hybrid**:

1. `.specify/contracts/` remains the canonical Specify baseline and merge target.
2. Producers are recorded in registry metadata, not by moving contract files into producer repos.
3. Consumers read materialized baseline contracts from `.specify/contracts/` relative to their current project root.
4. External contracts record upstream authority metadata, while their normalized Specify copy still lives in `.specify/contracts/`.

This preserves RFC-8's neutral shared baseline while making responsibility and source authority explicit.

## Design

### Artifact naming

Persisted paths do not change:

```text
.specify/
├── contracts/
│   ├── schemas/
│   ├── http/
│   └── messages/
└── changes/
    └── <change-name>/
        └── contracts/
            ├── schemas/
            ├── http/
            └── messages/
```

The schema id remains `contracts@v1`.

Operator-facing and documentation language should prefer "interfaces" for the concept:

- interfaces plugin;
- interface inventory;
- interface compatibility;
- interface review.

Documentation should use "contract artifacts" when referring specifically to files under `.specify/contracts/`.

### Registry interface inventory

Add an optional registry-level `interfaces` section:

```yaml
interfaces:
  - id: user-api
    description: User registration and account management API.
    contracts:
      - http/user-api.yaml
      - schemas/user-registration.yaml
      - schemas/user.yaml
      - schemas/error-response.yaml
    authority:
      kind: producer
      project: backend
    consumers:
      - mobile
    stability: public
    compatibility: warn
    lifecycle: active
    version: "1.2.0"
```

Field semantics:

| Field | Type | Description |
|---|---|---|
| `id` | string | Stable kebab-case logical interface id. |
| `description` | string | Human-readable purpose and boundary. |
| `contracts` | list of paths | Contract files relative to `.specify/contracts/`. |
| `authority.kind` | enum | `producer`, `external`, or `central`. |
| `authority.project` | string | Required when `kind: producer`; registry project that implements the interface. |
| `authority.source` | string | Optional URI or description for external authority. |
| `consumers` | list of strings | Registry project names that consume the interface. |
| `stability` | enum | `private`, `internal`, or `public`. |
| `compatibility` | enum | `warn`, `block`, or `manual`. |
| `lifecycle` | enum | `draft`, `active`, `deprecated`, or `retired`. |
| `version` | string | Optional logical interface version, usually SemVer when the platform follows SemVer. |

Authority kinds:

- `producer`: authored inside the platform and implemented by one registry project.
- `external`: dictated by a third-party or legacy system.
- `central`: authored as a platform-level interface with no single producer yet.

Compatibility policies:

- `warn`: cross-project incompatibilities are recorded but do not halt execution.
- `block`: breaking compatibility findings block merge or review once the enforcement layer supports it.
- `manual`: findings are informational only; humans own compatibility.

Lifecycle states:

- `draft`: contract files may exist, but no implementation or consumer is expected to rely on the interface yet.
- `active`: the interface is available for producer and consumer implementation.
- `deprecated`: the interface remains valid, but new consumers should not be added and replacement guidance should exist in the description or linked docs.
- `retired`: the interface is no longer available. Contract files may remain for historical review, but plan and review should reject new implementation changes that consume it.

Existing project-level `contracts.produces`, `contracts.consumes`, and `contracts.imports` may remain as shorthand. Registry validation should ensure both views are consistent when both are present.

### Registry invariants

When `interfaces` is present, registry validation enforces:

1. **Unique interface ids.** Each `interfaces[].id` is unique and kebab-case.
2. **Valid contract paths.** Every listed contract path is relative, stays under `.specify/contracts/`, and has a recognized subdirectory (`schemas/`, `http/`, or `messages/`).
3. **Known projects.** `authority.project` and every `consumers[]` entry reference registry projects.
4. **Single producer authority.** `authority.kind: producer` names exactly one producing project.
5. **External authority separation.** `authority.kind: external` does not also name a producing project.
6. **Project-role consistency.** If project-level `contracts` roles are present, they agree with registry-level `interfaces`.
7. **Compatibility policy validity.** `compatibility` is one of the supported policy values.
8. **Lifecycle policy validity.** `lifecycle` is one of the supported states, and `retired` interfaces are not referenced by new plan entries except retirement or migration changes.
9. **Version syntax validity.** When `version` is present, it follows the platform's declared version policy; SemVer is the default recommendation.

The registry remains a projection, not a full developer catalog. Rich ownership data may come from Backstage or another catalog, but the Specify registry records the reviewable subset needed by plan, execute, and review.

### Interface lifecycle

Production use needs an explicit lifecycle for interfaces, not just file additions and replacements.

New interfaces start as `draft` while the contract-first change is being authored. When the contract change merges and implementation work can begin, the interface becomes `active`. A later change may mark the interface `deprecated` to signal that consumers should migrate away while the contract remains valid. A final retirement change may mark it `retired` once known consumers have moved.

Lifecycle transitions are registry changes, not contract-file moves. The contract artifacts remain in `.specify/contracts/` for review history unless the operator performs a deliberate baseline cleanup outside the normal contract-delta path.

Allowed transitions:

| From | To | Meaning |
|---|---|---|
| `draft` | `active` | Interface is ready for producer and consumer implementation. |
| `active` | `deprecated` | Interface still works, but consumers should migrate. |
| `deprecated` | `retired` | Interface is no longer available for new work. |
| `retired` | `active` | Disallowed by default; create a new interface id or explicit revival change. |

This keeps deletion separate from retirement. RFC-8 deliberately avoided ordinary deletion semantics for opaque contract files; RFC-12 keeps that constraint and makes retirement the normal production path.

### Versioning and compatibility

Contract formats already carry version-like fields (`info.version` in OpenAPI and AsyncAPI, `$id` and optional annotations in JSON Schema). Specify should not invent a new IDL versioning system, but it does need one registry-level place to express the logical interface version.

The optional `interfaces[].version` field records the logical version of the interface as a whole. Format-specific version fields remain inside the contract artifacts and should agree with the registry version when a platform uses SemVer.

Compatibility policy is evaluated at the interface level:

- **Patch-compatible** changes include documentation changes, examples, additive optional fields, additive enum values, and new optional endpoints or channels.
- **Minor-compatible** changes include new endpoints, new message channels, and new optional payload members that existing consumers can ignore.
- **Breaking** changes include removed endpoints or channels, removed fields, newly required fields, narrowed types, removed enum values, and stricter additional-property constraints.

The registry's `compatibility` field controls enforcement. `warn` is the default while teams establish trust in the checks. `block` is reserved for mature interfaces where compatibility findings are expected to fail review or merge. `manual` is appropriate for external systems where the platform cannot control the upstream contract.

### Planning behavior

`/spec:plan` treats interfaces as first-class planning objects.

When an initiative crosses an API or message boundary, planning inserts a `contracts@v1` change before producer and consumer implementation changes. It also populates:

- the registry interface inventory;
- project role metadata;
- plan entry `context` paths for relevant contract files;
- `depends-on` edges from implementation changes to the interface change.

For example:

```yaml
changes:
  - name: user-api-interface
    schema: contracts@v1
    description: Define the user registration API interface
    status: pending

  - name: user-api-backend
    project: backend
    description: Implement the user registration API
    depends-on: [user-api-interface]
    context:
      - contracts/http/user-api.yaml
      - contracts/schemas/user-registration.yaml
      - contracts/schemas/user.yaml
    status: pending

  - name: registration-screen
    project: mobile
    description: Build the registration screen against the user API
    depends-on: [user-api-interface]
    context:
      - contracts/http/user-api.yaml
      - contracts/schemas/user-registration.yaml
      - contracts/schemas/user.yaml
    status: pending
```

External or legacy APIs use the same plan shape, except `authority.kind` is `external` and the initial contract change imports or normalizes existing contract files.

### External authority and imports

External and legacy APIs need source metadata because Specify is not the ultimate authority for their shape. For `authority.kind: external`, the registry entry should record:

```yaml
interfaces:
  - id: payment-gateway
    contracts:
      - http/payment-gateway-api.yaml
      - schemas/payment-request.yaml
      - schemas/payment-response.yaml
    authority:
      kind: external
      source: https://partner.example.com/openapi.yaml
      imported-version: "2026-04-01"
      normalized-at: "2026-04-30"
    consumers:
      - billing
    stability: public
    compatibility: manual
    lifecycle: active
```

Import changes may normalize older formats into Specify's target formats, decompose inline schemas, add missing metadata, and write the result as a contract delta. They must not imply that the platform can freely evolve the upstream interface. Local extensions to external contracts should either be represented as separate platform-owned interfaces or flagged explicitly for human review.

### Define pipeline

The `contracts` brief remains between `specs` and downstream design artifacts.

For Omnia:

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

For Vectis, `contracts` remains before `composition` and `design`.

The `contracts` brief delegates by format:

1. `/interfaces:json-schema`
2. `/interfaces:openapi`
3. `/interfaces:asyncapi`

The brief decides which format skills are relevant from the specs, baseline contracts, and plan `context`. Implementation changes usually produce an empty or small delta because the interface baseline already exists.

### Format skill responsibilities

Each `/interfaces:*` skill owns author, import, and verify intents for one format family.

| Skill | Owns | Does not own |
|---|---|---|
| `/interfaces:json-schema` | Reusable payload schemas under `contracts/schemas/` | HTTP paths, message channels |
| `/interfaces:openapi` | HTTP bindings under `contracts/http/` | Standalone schema vocabulary, message channels |
| `/interfaces:asyncapi` | Message bindings under `contracts/messages/` | Standalone schema vocabulary, HTTP paths |

The cross-format ordering rule is stable: payload schemas are authored or imported first, then OpenAPI and AsyncAPI bindings reference them.

### Merge semantics

Contract artifacts continue to use opaque replacement semantics:

- files present in the change-local `contracts/` directory replace the matching baseline files;
- files absent from the change are left untouched;
- new files are added;
- deletion remains out of scope for normal contract deltas.

This RFC does not introduce semantic YAML merging. The conflict resolution path remains: re-run define against the updated baseline and produce a fresh delta.

### Workspace distribution

`specify workspace sync` materializes `.specify/contracts/` into each workspace clone. Phase skills continue to read `.specify/contracts/` relative to their current working directory.

This preserves RFC-3b's execution invariant: phase skills are unaware of multi-repo routing. The execute driver chooses the CWD; the brief and format skills see the same paths in every topology.

Project-local materialized copies are read-only from the perspective of interface authoring. Changes still write contract deltas to `.specify/changes/<name>/contracts/` and merge through the normal Specify lifecycle.

### Generated code boundary

Contracts are source artifacts. Generated clients, generated server stubs, generated Rust types, Swift models, Kotlin models, and test fixtures are downstream build artifacts.

Build skills may consume contracts to generate or update code, but they must not silently rewrite `.specify/contracts/` or change-local contract deltas. Any contract shape change discovered during implementation belongs in the define phase as a contract delta, followed by merge and review. This keeps the direction of authority clear:

```text
specs + registry intent
  -> contract artifacts
  -> generated implementation support
  -> tests and review
```

Generated code may live in producer or consumer repositories, but it is not the Specify source of truth for the interface.

### Validation and review

Structural validation should become CLI-backed over time, with skills retaining authoring and import judgment. Production use needs one review surface that combines artifact validity, registry consistency, lifecycle policy, and compatibility findings.

Candidate CLI surface:

```bash
specify interface list
specify interface show <id>
specify interface validate [--change <name>]
specify interface diff <id> [--against baseline]
```

`specify review` should include interface findings:

- unresolved `$ref` pointers;
- missing schema metadata;
- producer/consumer registry inconsistencies;
- breaking changes against known consumers;
- contract deltas not reflected in specs;
- specs that describe interface behavior without a matching contract.
- active interfaces with no producing authority;
- active interfaces with no known consumers when the stability is `public`;
- deprecated interfaces without replacement guidance;
- retired interfaces referenced by new plan entries;
- generated-code changes that imply contract drift.

Cross-project compatibility remains warning-first by default. `compatibility: block` should only become enforceable after warning-mode output has proven stable across real projects.

### Compatibility findings

The existing cross-project verifier vocabulary remains the starting point:

- removed field;
- required field added;
- type narrowed;
- enum value removed;
- additional properties tightened;
- removed endpoint;
- removed status code;
- removed channel;
- removed operation.

This RFC adds a policy layer around those findings. The same finding may be informational, warning-only, or blocking depending on the interface inventory's `compatibility` value and the review mode being run.

## Alternatives Considered

### Central only

This is RFC-8's model. It is simple and neutral, but incomplete: it does not say who is responsible for implementing a contract, who consumes it, or whether an external system is authoritative.

### Producer owned

Contracts live beside producer code. This improves producer locality, but makes consumers chase workspace clones and weakens the platform-level review story. It also breaks down for external APIs, shared schemas, and interfaces where no single project is the right owner.

### Consumer owned

Consumers record the contracts they depend on. This gives callers local context, but it creates multiple divergent copies of the same interface. It is useful as a generated client artifact, not as the Specify source of truth.

### Hybrid

The chosen model keeps the Specify baseline central and neutral while recording authority and roles in the registry. This gives producers responsibility without making producer repos the artifact home.

## Non-goals

- Rename `.specify/contracts/`.
- Rename `contracts@v1`.
- Replace OpenAPI, AsyncAPI, or JSON Schema.
- Make behavioral specs wire-format documents.
- Require all interface changes to block execution by default.
- Introduce a live dependency on Backstage or another catalog.
- Generate client or server code directly from this RFC.

## Implementation Scope

### Layer 1: Documentation and vocabulary

1. Update reference docs to consistently use "interfaces" for the concept and "contracts" for files.
2. Add a decision-log entry for the hybrid interface home.
3. Update quick-reference docs to describe `/interfaces:*` as format-family skills invoked by the `contracts` brief.

### Layer 2: Registry inventory

1. Extend the registry schema with the optional `interfaces` list.
2. Add validation for interface ids, contract paths, authority, consumers, lifecycle, version, and compatibility policy.
3. Add consistency checks between registry-level interface inventory and project-level `contracts` roles.
4. Update `/spec:plan` guidance so generated plans populate interface inventory entries when it detects cross-project API or message boundaries.
5. Define lifecycle transition validation for `draft`, `active`, `deprecated`, and `retired`.
6. Define the default version policy and how registry `version` relates to OpenAPI / AsyncAPI `info.version` and JSON Schema `$id`.

### Layer 3: Review integration

1. Add CLI-backed interface validation and diff commands, or fold equivalent checks into `specify review`.
2. Route cross-project compatibility findings through a shared review output shape.
3. Record compatibility findings with the interface id when available, not only the contract file path.
4. Add lifecycle review findings for deprecated interfaces without replacement guidance and retired interfaces referenced by new work.
5. Add drift review findings for generated-code changes that imply contract changes.
6. Support `compatibility: block` in review mode after warning-mode output stabilizes.

### Layer 4: Catalog projection

1. Allow registry importers to map catalog API entities into `interfaces` inventory entries.
2. Preserve Specify's local registry as a reviewable projection rather than performing live catalog lookups during plan or execute.
3. Record catalog source identifiers only as metadata, not as execution dependencies.
4. Map external API source metadata into `authority.kind: external` entries without granting Specify authority to evolve upstream interfaces.

### Layer 5: Build consumption

1. Document how Omnia and Vectis build skills consume contract artifacts for generated types, clients, stubs, and tests.
2. Add guardrails that build skills never modify `.specify/contracts/` or change-local `contracts/` deltas.
3. Route implementation-discovered interface changes back to define-phase contract deltas rather than patching generated code as the source of truth.

## Implementation Order

1. Land vocabulary and documentation cleanup so "interfaces" and "contracts" are used consistently.
2. Extend the registry schema with interface inventory, lifecycle, version, and external-authority fields.
3. Add registry validation for inventory consistency and lifecycle policy.
4. Update `/spec:plan` so interface inventory, `context`, and dependency edges are populated together.
5. Fold interface validation and compatibility findings into `specify review`.
6. Add warning-mode lifecycle and drift findings before enabling any blocking policy.
7. Document build-skill consumption boundaries and add checks for accidental contract mutation during build.

## Migration

Existing projects remain valid.

No file moves are required. Existing `.specify/contracts/` baselines, `contracts@v1` plan entries, and `/interfaces:*` skills continue to work.

Recommended migration path:

1. Add registry-level `interfaces` entries for multi-project or externally consumed APIs.
2. Keep existing project-level `contracts.produces`, `contracts.consumes`, and `contracts.imports` fields until the registry validator can derive or check both views.
3. Mark existing interfaces `active` by default unless they are known to be deprecated or retired.
4. Add `version` only where the platform already has an interface versioning convention; avoid inventing synthetic versions during migration.
5. For external APIs, add `authority.kind: external` and source metadata before enabling compatibility enforcement.
6. Run warning-mode compatibility checks before enabling any blocking policy.
7. Update documentation and examples to prefer "interface" for the logical boundary and "contract" for the artifact file.

## Open Questions

- Should registry-level `interfaces` be required in multi-repo registries, or only recommended?
- Should `compatibility: block` block merge, execute, or only CI/review?
- Should external catalog imports populate interface inventory directly, or only project registry entries?
- Do shared payload schemas need their own logical interface ids, or should they remain attached to OpenAPI and AsyncAPI bindings?
- Should project-level `contracts` roles eventually be deprecated in favor of registry-level `interfaces`, or remain as denormalized convenience fields?
- Should `version` be required for `public` interfaces, or remain optional everywhere?
- Should retirement ever delete contract files from `.specify/contracts/`, or should deletion stay a manual baseline cleanup outside normal deltas?
- Should generated-code drift be a best-effort review warning, or should schemas opt into stronger build-time enforcement?

## References

- [RFC-8: API Contracts](archive/rfc-8-api-contracts.md)
- [RFC-9: Platform](archive/rfc-9-platform.md)
- [RFC-10: Skill Improvements](archive/rfc-10-skills.md)
- [OpenAPI 3.1 Specification](https://spec.openapis.org/oas/v3.1.0)
- [AsyncAPI 3.0 Specification](https://www.asyncapi.com/docs/reference/specification/v3.0.0)
- [JSON Schema](https://json-schema.org/specification)

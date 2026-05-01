# RFC-12: Embedded Interface Metadata

> Status: Draft · Depends: [RFC-8](archive/rfc-8-api-contracts.md), [RFC-9](archive/rfc-9-platform.md), [RFC-10](archive/rfc-10-skills.md)

## Abstract

RFC-8 introduced machine-readable contract artifacts under `.specify/contracts/` and a `contracts@v1` schema for dedicated contract changes. RFC-10 renamed the operator-facing slash-command surface to `/interfaces:*` while preserving the persisted artifact names. The remaining gap — and the only thing this RFC addresses — is **logical interface identity**: a stable id, a SemVer version, a lifecycle status, and an authority record for each contract that the platform treats as a governed boundary.

This RFC adds those four properties as a small metadata block embedded in the contract artifact itself, using the format-native extension mechanism that OpenAPI 3.1, AsyncAPI 3.0, and JSON Schema all already provide. There is no separate inventory file, no new top-level config, no change-local inventory delta, no whole-file replacement merge path, and no new workspace-distribution wiring. The block travels with the contract.

The result is RFC-8 plus four optional fields per top-level contract.

## Motivation

RFC-8 deliberately deferred logical interface identity, versioning, and lifecycle to a future RFC. Three concrete needs have emerged since:

1. **Identity.** Contracts are referenced today by file path. Renames and refactors break references; cross-project diffs cannot tell when "the user API" simply moved. A stable kebab-case id is the smallest fix.
2. **Versioning.** OpenAPI and AsyncAPI carry their own `info.version`; JSON Schema has none. There is no shared SemVer convention that callers can lift into release notes, deprecation policy, or compatibility decisions.
3. **Lifecycle.** Reviewers cannot tell whether a contract is `draft`, `active`, or `deprecated`, and deprecated contracts have no machine-readable replacement pointer.

A separate inventory file (an earlier sketch of this RFC) would record the same information out-of-band. That sketch was rejected because the contract artifact already exists, already has its own merge cycle, and already has format-native extension keys. A second file would duplicate identity that is intrinsic to the contract, introduce a second mutation lifecycle, and require its own validation, merge, conflict-detection, and workspace-distribution wiring. Embedding the metadata in the contract avoids every one of those problems and matches the same authoring path operators use for the rest of the contract today.

## Design

### Embedded metadata block

Every top-level contract file MAY declare an `x-specify-interface` block at the document root. The block is the same shape across all three formats:

```yaml
# contracts/http/user-api.yaml (OpenAPI 3.1)
openapi: 3.1.0
x-specify-interface:
  id: user-api
  version: "1.2.0"
  status: active
  authority:
    kind: platform
info:
  title: User Registration API
  version: "1.2.0"
paths:
  /users:
    ...
```

```yaml
# contracts/messages/order-events.yaml (AsyncAPI 3.0)
asyncapi: 3.0.0
x-specify-interface:
  id: order-events
  version: "0.3.0-draft.1"
  status: draft
  authority:
    kind: platform
info:
  title: Order Events
  version: "0.3.0-draft.1"
channels:
  ...
```

```yaml
# contracts/schemas/payment-token.yaml (governed JSON Schema)
$schema: "https://json-schema.org/draft/2020-12/schema"
$id: "urn:specify:schemas/payment-token"
x-specify-interface:
  id: payment-token
  version: "2.0.0"
  status: active
  authority:
    kind: external
    source: "https://partner.example.com/payment-spec"
title: PaymentToken
type: object
...
```

OpenAPI 3.1 §4.2, AsyncAPI 3.0 §A.4, and JSON Schema (per RFC 8259 / draft 2020-12) all permit additional properties at the document root. `x-specify-interface` lives inside that escape hatch in every format.

### Field semantics

| Field                | Type   | Required                                | Description                                                                              |
| -------------------- | ------ | --------------------------------------- | ---------------------------------------------------------------------------------------- |
| `id`                 | string | yes (when block present)                | Kebab-case logical id, unique across the contracts tree.                                 |
| `version`            | string | yes                                     | SemVer, e.g. `1.2.0`. Prerelease labels (`1.0.0-draft.1`) are valid.                     |
| `status`             | enum   | no (defaults to `active`)               | One of `draft`, `active`, `deprecated`.                                                  |
| `authority.kind`     | enum   | yes                                     | One of `platform`, `external`.                                                           |
| `authority.source`   | string | yes when `authority.kind: external`     | URI, repository path, or human description of the upstream authority.                    |
| `replacement`        | string | yes when `status: deprecated`           | Replacement interface id, contract path, or short guidance for callers.                  |

Anything outside this list (e.g. owner team, documentation URL, SLA tier) is out of scope for RFC-12. Skills and downstream tools may add their own `x-` extensions alongside `x-specify-interface` without coordination with this RFC.

### What counts as "top-level"

A contract is **top-level** when it carries an `x-specify-interface` block. That is the only marker. This convention removes the need for tools to infer governance from directory layout or schema shape.

In practice:

- Files under `contracts/http/` and `contracts/messages/` are usually top-level and SHOULD carry the block.
- Files under `contracts/schemas/` are usually payload vocabulary referenced via `$ref` and SHOULD NOT carry the block. A standalone JSON Schema is top-level only when the schema itself is the governed boundary (a published payload type that other systems version against), in which case the block is added.
- A change converts a payload schema into a top-level interface by adding the block; it converts a top-level interface back into payload vocabulary by removing the block. Both transitions are reviewed as ordinary contract diffs.

### Coherence with format-native version fields

To keep the embedded version and the format-native version from drifting:

- OpenAPI and AsyncAPI: `info.version` MUST equal `x-specify-interface.version`. Validation reads both and fails if they differ.
- JSON Schema: no native version field exists. `x-specify-interface.version` stands alone. Tools that need a URI carrying the version may compose one from `$id` and `version` (e.g. `urn:specify:schemas/payment-token#2.0.0`); that composition is a tooling concern, not a contract requirement.

`$id` in JSON Schema remains an identity URI as RFC-8 specified; the RFC-12 SemVer is orthogonal.

### Compatibility with RFC-8 contracts

Existing RFC-8 contracts without an `x-specify-interface` block remain fully valid. Without the block:

- the file does not appear in `specify interface list`;
- registry roles cannot reference it by id (path references still work as RFC-8 specified);
- lifecycle and SemVer validation rules do not apply.

Adoption is per-file and incremental. A repo gains identity for one interface by adding the block to one file; it never has to migrate the whole baseline at once.

### Validation

`specify validate` gains the following checks, run over every file under `contracts/` that carries an `x-specify-interface` block:

1. `id` is kebab-case (`^[a-z][a-z0-9-]*$`) and ≤ 64 characters.
2. `id` is unique across all top-level interfaces in the repo.
3. `version` parses as SemVer per [semver.org](https://semver.org), including optional prerelease labels.
4. `status` is one of `draft`, `active`, `deprecated`.
5. `status: deprecated` requires a non-empty `replacement`.
6. `authority.kind` is one of `platform`, `external`.
7. `authority.kind: external` requires a non-empty `authority.source`.
8. For OpenAPI and AsyncAPI files, `info.version` equals `x-specify-interface.version`.
9. **Single producer.** Each interface id appears in at most one project's `contracts.produces` in `registry.yaml`. RFC-8 already validates this by file path; RFC-12 extends the same invariant to id-based references.

Findings are emitted with the file path and the failing rule. The validator does not attempt structural compatibility comparison between baseline and proposed contracts; that judgment remains with the `/interfaces:*` verifier intents (RFC-8 §"Specialist skills", RFC-10 §C.3).

### CLI surface

Two new commands plus one extension. The whole footprint:

```bash
specify interface list           # scan contracts/, project the embedded blocks, render a table
specify interface show <id>      # print one block plus the file path that carries it
specify validate                 # extended to run the checks above
```

`specify interface list` is a deterministic projection over the filesystem: glob-walk `contracts/**/*.yaml`, parse the embedded block when present, group by id. The command takes no flags beyond standard `--format json` for machine-readable output.

`specify interface show <id>` resolves the id to its file, then prints the file path and the block. Useful for scripts and review.

There is intentionally no `specify interface diff`. RFC-8 already supports `specify spec preview` for change-local contract files; structural diffs between contract files are git-diff territory; semantic compatibility findings live in the verifier intents.

There is intentionally no CLI compatibility-findings vocabulary. RFC-8's deferral of cross-project structural diff to "when the need arises" still holds.

### Registry roles

`registry.yaml` continues to use the RFC-8 `contracts.produces`, `contracts.consumes`, and `contracts.imports` lists. Two ergonomic additions:

1. **Id references.** A list entry may use `id:<interface-id>` as an alternative to a file path. The validator resolves both forms to the same interface, applies the single-producer invariant, and reports drift if the path and id disagree.
2. **`contracts.imports` keeps its RFC-8 meaning.** A project lists a contract under `imports` when it integrates with an external authority for that interface. The embedded `authority.kind: external` records *where* the shape comes from; `contracts.imports` records *which project* depends on it. Both pieces of information are useful and not redundant.

No registry schema field is removed or renamed by this RFC.

### Merge, workspace distribution, and conflicts

Nothing new.

- The embedded block is part of the contract YAML; RFC-8's opaque-replacement merge semantics (§"Merge semantics") apply unchanged.
- `workspace sync` materialises `contracts/` into project clones as RFC-8 specified; the embedded block rides along automatically.
- Conflict detection (`specify spec conflict-check`) treats a file with a changed embedded block exactly like any other contract file change.

There is no `interfaces.yaml`, no whole-file inventory replacement, no inventory-specific drift check, no inventory-specific workspace-sync step.

### `/interfaces:*` skill responsibilities

The format-family skills introduced by RFC-10 absorb the small additional authoring and verification work:

- **Author intents** (`/interfaces:openapi`, `/interfaces:asyncapi`, `/interfaces:json-schema`): when generating or extending a top-level contract, populate `x-specify-interface` from the change's proposal Source Material, registry context, and operator intent. For OpenAPI/AsyncAPI files, keep `info.version` synchronised with `x-specify-interface.version`.
- **Import intents**: when normalising an external contract, set `authority.kind: external` and record the upstream URL or repository in `authority.source`. Set `version` from the upstream's own version field when present, or `0.0.0` and `status: draft` when unknown.
- **Verify intents**: enforce the validator rules above on the change-local file and continue to flag warning-mode compatibility findings between baseline and proposed contracts as RFC-8 specified.

Cross-project compatibility checks keep the RFC-8 algorithm: post-merge of a producer-side change, walk from the changed file → producing project → consuming projects → run the appropriate verifier per pair. Findings may now reference the interface id (preferred) or the file path (RFC-8 fallback).

## Alternatives Considered

### Separate `interfaces.yaml` inventory file

The earlier draft of this RFC. Rejected: duplicates identity that is intrinsic to the contract; introduces a second mutation lifecycle (operator-driven vs change-driven); requires its own merge semantics, conflict-detection, validation, and workspace-sync wiring; doubles the surface that documentation and skills must reference.

### `interfaces:` block inside `registry.yaml`

Rejected: `registry.yaml` is operator-mutated (`specify registry add` / `remove`) and `interfaces` would be change-mutated. Mixing the two lifecycles inside one file forces every reader and writer to reason about which sub-tree is change-managed, and every change PR gains a registry-shaped diff regardless of whether it touches projects.

### Enrich registry roles with version/authority/status

Move version, status, and authority onto each entry in `contracts.produces`. Rejected: the producer is no longer the single source of truth (consumers and importers need to know the version too), and each interface ends up declared in N+1 places (once per project plus once per registry list).

### Status quo (RFC-8 + RFC-10 only)

The path already shipped. Rejected only weakly: it leaves identity, SemVer, lifecycle, and external authority unspecified. RFC-12 adds those four things at minimum cost.

## Non-goals

- A separate `interfaces.yaml` file or a registry `interfaces:` block.
- A `specify interface diff` command, or any CLI surface that performs structural compatibility comparison.
- A CLI compatibility-findings taxonomy. Findings remain skill-side.
- Blocking compatibility enforcement during `/spec:execute`. Compatibility checks remain warning-only as RFC-8 specified.
- Removing or renaming `contracts.imports` from registry roles.
- Removing or renaming the `contracts@v1` schema, the `contracts/` artifact tree, or the `contracts` brief id.
- Catalog integration (Backstage and similar). Out of scope.
- Generated-code drift detection. Out of scope.
- Inferring interface identity from directory layout, file name, or schema content. Identity is declared, not inferred.

## Implementation Scope

### specify-cli

1. New module `crates/validate/src/interfaces.rs`: parse `x-specify-interface` from any YAML file under `contracts/`, run the nine validation rules, return findings.
2. Extension to `specify validate` (`src/main.rs` / `src/commands/`): invoke the new validator and merge findings into the standard validate output.
3. New command group `src/commands/interface.rs`: `list` and `show <id>` subcommands.
4. Extension to `crates/schema/src/registry.rs`: accept `id:<name>` notation in `contracts.produces`, `contracts.consumes`, `contracts.imports`; resolve to a contract path during validation.

Estimated total: well under 500 lines of Rust. No new merge code, no new workspace code, no new schema files, no plan-entry changes.

### specify

1. Update `/interfaces:openapi`, `/interfaces:asyncapi`, `/interfaces:json-schema` author intents to populate `x-specify-interface` when authoring or importing a top-level contract.
2. Update the same skills' verifier intents to enforce the embedded-block rules.
3. Update the `contracts@v1` build brief to instruct the format skills to populate or update the block.
4. Update `docs/explanation/glossary.md` and `docs/reference/quick-reference.md` with the "interface identity" terminology.
5. Add a worked example under `schemas/contracts/fixtures/` showing a contract with the block, the validator output, and a `specify interface list` projection.

No changes to Omnia or Vectis schemas, briefs, or skills. No changes to RFC-8's `contracts` brief, `contracts@v1` schema, or `/spec:plan` algorithm.

## Implementation Order

1. In `specify-cli`, add the `x-specify-interface` parser and the validator rules behind `specify validate`.
2. In `specify-cli`, add `specify interface list` and `specify interface show <id>`.
3. In `specify-cli`, extend registry role parsing to accept `id:<name>` references.
4. In `specify`, update the three `/interfaces:*` skills' author and verify intents.
5. In `specify`, update the `contracts@v1` build brief and add the fixture.
6. Document the model in the glossary and quick-reference.

Steps 1–3 land independently; steps 4–6 depend on step 1 reaching `main`.

## Migration

No file moves. No registry edits required. No baseline contracts broken.

To adopt identity for an interface, add an `x-specify-interface` block to the relevant file, choose a SemVer value, and run `specify validate`. Repeat per interface as needed; partial adoption is supported indefinitely.

For external contracts, set `authority.kind: external` and `authority.source` when adding the block. The corresponding project entry's `contracts.imports` continues to list the file path.

## References

- [RFC-8: API Contracts](archive/rfc-8-api-contracts.md)
- [RFC-9: Platform](archive/rfc-9-platform.md)
- [RFC-10: Skill Improvements](archive/rfc-10-skills.md)
- [OpenAPI 3.1 Specification](https://spec.openapis.org/oas/v3.1.0)
- [AsyncAPI 3.0 Specification](https://www.asyncapi.com/docs/reference/specification/v3.0.0)
- [JSON Schema](https://json-schema.org/specification)
- [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html)

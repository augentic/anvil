#  RFC-12: Refine RFC-8

> Status: Landed · Parent: [RFC-8](rfc-8-api-contracts.md) · Depends: [RFC-9](rfc-9-platform.md), [RFC-10](rfc-10-skills.md)

## Abstract

RFC-8 introduced machine-readable contract artifacts under `.specify/contracts/` and a `contracts@v1` schema. RFC-10 renamed the slash-command surface to `/interfaces:*`. RFC-12 is a refinement on top of those, not a new architectural layer: it tightens contract versioning to SemVer, adds an opt-in rename-stable identifier as an `info` extension, and removes the `contracts.imports` role. None of these change the artifact tree, the `contracts` brief, the `contracts@v1` schema id, the merge semantics, or the workspace flow. They are recorded as an RFC because two of the three are breaking changes to validator behavior or a published registry schema, and because the design alternatives considered are worth preserving for future contributors.

Throughout the rest of this RFC, bare `contracts/` is shorthand for `.specify/contracts/`.

## Motivation

RFC-8 deliberately deferred two concerns:

1. **Versioning.** OpenAPI 3.1 and AsyncAPI 3.0 both require an `info.version` field, but RFC-8 said nothing about its format. Without a shared convention, callers cannot lift a contract version into release notes, compatibility decisions, or operator-facing tooling.
2. **Project roles for externally-authored contracts.** RFC-8 introduced a third role (`contracts.imports`) alongside `produces` and `consumes`. In practice the distinction between "consumes a contract" and "consumes a contract whose shape is dictated externally" has not been load-bearing; a binary role set is easier to reason about.

A stable kebab-case identifier is offered as an **opt-in** capability via `info.x-specify-id`. File-path references in `registry.yaml` remain the canonical reference; the id is a rename-stable hint that tooling can use to notice when a contract has moved (the path changed but the id did not). Mandating an id today is out of scope — the validator only enforces format and uniqueness when the field is present.

Top-level JSON Schemas are **explicitly out of scope**. Every concrete RFC-8 case routes payload schemas through an OpenAPI or AsyncAPI binding; standalone JSON Schemas remain payload vocabulary referenced via `$ref` from a top-level contract.

A separate inventory file (the earliest draft of RFC-12) and a root-level `x-specify` extension block carrying both id and version (a later draft) were both rejected. The inventory file duplicated information that already lives in `info` and required its own mutation lifecycle. The root-level block duplicated `info.version` and invented a second top-level slot for identity rather than co-locating it with the other identity-shaped fields the format specifications already define. RFC-12 keeps `info.version` as the version source of truth and places the optional id alongside it as `info.x-specify-id`.

## Design

### Top-level contracts

A file under `contracts/` is **top-level** when its YAML root carries an `openapi:` field (OpenAPI 3.1 document) or an `asyncapi:` field (AsyncAPI 3.0 document). Format detection decides what is top-level — never directory layout, file name, or a custom marker.

### Versioning convention

Every top-level contract MUST set `info.version` to a value that parses as SemVer per [semver.org](https://semver.org), including optional prerelease labels (`1.0.0-draft.1`). Bump rules (when to advance major, minor, or patch) are skill-side judgement; the validator only checks that the value parses.

### Optional rename-stable identity

A top-level contract MAY set `info.x-specify-id` to a kebab-case string that uniquely identifies the contract across the repo, independent of its file path. The field is an OpenAPI 3.1 / AsyncAPI 3.0 specification extension (both formats permit `x-` keys inside the Info Object — OpenAPI 3.1 §4.8.2, AsyncAPI 3.0 §4.6).

When present, the id is a **rename-stable hint**, not a substitute for path-based references. The registry continues to reference contracts by file path; the id lets tooling and humans notice when a contract has moved.

The id remains stable across version bumps. A `2.0.0` release of the same contract retains the same `info.x-specify-id` as the `1.x` line; only `info.version` moves. New top-level contracts SHOULD set the id; existing contracts MAY add it at any time.

### Validation

`specify validate` gains the following checks, run over every YAML file under `contracts/`:

1. For each top-level contract (`openapi:` or `asyncapi:` present at the root), `info.version` MUST parse as SemVer per [semver.org](https://semver.org), including optional prerelease labels.
2. When a top-level contract sets `info.x-specify-id`, the value MUST match `^[a-z][a-z0-9-]*$` and be ≤ 64 characters.
3. When two or more top-level contracts both set `info.x-specify-id`, the values MUST be distinct across the repo.

Findings are emitted with the file path and the failing rule. Structural compatibility comparison between baseline and proposed contracts remains the responsibility of the `/interfaces:`* verifier intents (RFC-8 §"Specialist skills").

### CLI surface

```bash
specify interface list           # scan contracts/, project top-level OpenAPI/AsyncAPI documents
specify validate                 # extended to run the SemVer and id checks
```

`specify interface list` is a deterministic projection: glob-walk `contracts/**/*.yaml`, identify top-level documents by format, render `(file path, format, info.title, info.version, info.x-specify-id)`. Standard `--format json` is supported; `info.x-specify-id` is rendered as `null` when absent.

### Registry roles, merge, workspace distribution

RFC-8's registry references, opaque-replacement merge semantics, `workspace sync`, and conflict detection are unchanged except for the removal of `contracts.imports` (see §*Drop `contracts.imports`* below).

### Drop `contracts.imports`

RFC-8 defined three project roles per contract: `produces`, `consumes`, and `imports`. The third was meant to flag contracts whose shape is dictated by an external system, but the distinction is not load-bearing — both `consumes` and `imports` describe a project that calls or subscribes to an interface it does not author. RFC-12 collapses the role set to two: `produces` (this project authoritatively implements the contract) and `consumes` (this project calls or subscribes to the contract). A contract that no project produces is, by definition, externally authored; no separate field is needed to mark it. RFC-8's produces/consumes self-consistency invariant is unchanged and continues to forbid a single project from both producing and consuming the same path. The schema, validator, and `/spec:plan` changes that follow from removing the field are listed in §*Implementation Scope*; the operator-facing migration is in §*Migration*.

### `/interfaces:`* skill responsibilities

The format-family skills introduced by RFC-10 absorb the small additional work:

- `**/interfaces:openapi` and `/interfaces:asyncapi` author intents**: set `info.version` to a SemVer value on every top-level contract. SHOULD also set `info.x-specify-id` on new top-level contracts; MUST preserve any pre-existing id on update and MUST NOT change it across version bumps.
- `**/interfaces:openapi` and `/interfaces:asyncapi` verify intents**: enforce §*Validation* checks 1–3 on the change-local file, in addition to the existing warning-mode compatibility checks RFC-8 specified.
- `**/interfaces:json-schema`** is unchanged.

Cross-project compatibility checks keep the RFC-8 algorithm; findings continue to reference contract file paths.

## Alternatives Considered

**Root-level `x-specify` extension block carrying both a stable id and an explicit version.** A draft of RFC-12. Rejected: the version half duplicated `info.version` (which both formats already require), and the id half invented a new top-level slot for identity rather than co-locating it with `info.title` and `info.version` where the format specifications already place identity-shaped metadata. RFC-12 keeps `info.version` as the version source of truth and adds the optional id under the same `info` block as `info.x-specify-id`.

**Top-level JSON Schemas with their own version field.** Considered for data-interchange-style contracts with no transport binding. Rejected for now: every concrete RFC-8 case routes payload schemas through an OpenAPI or AsyncAPI binding. A standalone JSON Schema contract type can be added in a follow-up RFC if a real consumer appears.

**Separate `interfaces.yaml` inventory file.** The earliest draft of RFC-12. Rejected: duplicates information that already lives in `info.title` / `info.version`; introduces a second mutation lifecycle (operator-driven vs change-driven); requires its own merge, conflict-detection, validation, and workspace-distribution wiring.

`**interfaces:` block inside `registry.yaml`.** Rejected: `registry.yaml` is operator-mutated (`specify registry add` / `remove`) and contract metadata would be change-mutated. Mixing the two lifecycles inside one file forces every reader and writer to reason about which sub-tree is change-managed.

## Non-goals

- Extending `info.x-specify-id`'s role: making the field required, or promoting it to a registry-reference shorthand (e.g. `contracts.consumes: [user-api]` instead of `contracts.consumes: [http/user-api.yaml]`). Both deferred to a follow-up RFC.
- Top-level JSON Schemas. Deferred to a follow-up RFC if and when a concrete consumer appears.
- A separate `interfaces.yaml` file or a registry `interfaces:` block (see Alternatives Considered).
- Lifecycle `status` (`draft` / `active` / `deprecated`) and replacement pointers. Deferred.
- External-authority metadata (`authority.kind` / `source`). A contract with no project listed under `contracts.produces` is, by convention, externally authored; the explicit "where the shape comes from" pointer is deferred.
- Any CLI surface for structural comparison or compatibility findings (`specify interface diff`, a findings taxonomy). Comparison and findings remain skill-side.
- Blocking compatibility enforcement during `/spec:execute`. Compatibility checks remain warning-only as RFC-8 specified.
- Removing or renaming the `contracts@v1` schema, the `contracts/` artifact tree, or the `contracts` brief id.
- Inferring contract identity from anything other than the file path or an explicit `info.x-specify-id`. No derived id, no schema-content fingerprint.

## Implementation Scope

### specify-cli

1. New module `crates/validate/src/interfaces.rs` implementing §*Validation* checks 1–3.
2. Wire the new module into `specify validate`.
3. New command `src/commands/interface.rs` implementing `specify interface list` per §*CLI surface*.
4. Drop `contracts.imports` from the registry-entry schema, drop the produce/import mutual-exclusion invariant from `specify registry validate`, and drop the `imports`-population branch from `/spec:plan`'s registry-update step.

Estimated total: under 250 lines of Rust. No new merge, workspace-sync, plan-entry, or schema-file code.

### specify

1. `/interfaces:openapi` and `/interfaces:asyncapi` SKILL.md updates: author intents per §*Versioning convention* + §*Optional rename-stable identity*; verify intents per §*Validation*.
2. Update the `contracts@v1` build brief to require SemVer `info.version` and recommend `info.x-specify-id` for new contracts.
3. Add glossary entries for "top-level contract" and "interface id".

`/interfaces:json-schema`, Omnia and Vectis schemas, RFC-8's `contracts` brief, the `contracts@v1` schema, and the `/spec:plan` algorithm are unchanged.

## Migration

No file moves. No baseline contracts broken. Existing OpenAPI 3.1 / AsyncAPI 3.0 contracts whose `info.version` is already SemVer remain valid; contracts with a non-SemVer value (e.g. a `YYYY-MM-DD` date) need a one-line edit before `specify validate` will pass after upgrade.

Adding `info.x-specify-id` to existing contracts is **per-file and optional**. Contracts without one remain valid indefinitely; the validator enforces the format and cross-repo uniqueness rules only on contracts that declare the field. Adoption is mechanical: choose a kebab-case slug (typically the file's stem), add one line under `info:`, and run `specify validate`.

Projects that currently declare `contracts.imports` in `registry.yaml` need a one-time edit: rename the field to `contracts.consumes` (merging with any existing `consumes` list and deduplicating paths). `specify registry validate` rejects the unknown `imports` key after upgrade, so the migration is mechanical and immediately surfaces if missed.

## References

- [RFC-8: API Contracts](rfc-8-api-contracts.md)
- [RFC-9: Platform](rfc-9-platform.md)
- [RFC-10: Skill Improvements](rfc-10-skills.md)
- [OpenAPI 3.1 Specification](https://spec.openapis.org/oas/v3.1.0)
- [AsyncAPI 3.0 Specification](https://www.asyncapi.com/docs/reference/specification/v3.0.0)
- [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html)


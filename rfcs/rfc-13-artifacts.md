# RFC-13: Extensibility

> Status: Draft · Depends: [RFC-1](archive/rfc-1-cli.md), [RFC-8](archive/rfc-8-api-contracts.md), [RFC-9](archive/rfc-9-platform.md), [RFC-12](archive/rfc-12-refine-rfc-8.md)

## Abstract

Specify-cli today bakes the lifecycle of every output artifact into Rust code: specs and contracts get a change-local delta + post-merge promotion to a baseline; crates, shared cores, iOS shells, Android shells, and design-system packages are written directly into the project tree with no Specify-managed baseline at all. The split is meaningful — Specify-owned artifacts need transactional baseline updates, externally-toolchained artifacts do not — but it is implicit, hard-coded, and unavailable to new schemas. RFC-13 makes the split explicit by adding a per-schema `artifacts:` declaration that tags each output as `managed` (delta-then-promote) or `external` (direct-write), and refactors the merge / drop / validate paths to be data-driven from that declaration. Behaviour for existing schemas is preserved exactly; future schemas pick the right lifecycle in their own `schema.yaml`.

## Motivation

### Three lifecycles, one runtime

A change running under any current schema produces outputs governed by one of three lifecycles:


| Artifact                                             | Build writes to                      | Baseline location      | Merge promotes?              | Drop reverts?    |
| ---------------------------------------------------- | ------------------------------------ | ---------------------- | ---------------------------- | ---------------- |
| Specs (all schemas)                                  | `.specify/changes/<name>/specs/`     | `.specify/specs/`      | Yes (file-level merge)       | Yes              |
| Contracts (`contracts@v1`)                           | `.specify/changes/<name>/contracts/` | `<root>/contracts/`    | Yes (whole-file replacement) | Yes              |
| Crates (`omnia@v1`)                                  | `<root>/crates/<crate>/`             | (no separate baseline) | No                           | No (git-managed) |
| Shared / iOS / Android / design-system (`vectis@v2`) | `<root>/<dir>/`                      | (no separate baseline) | No                           | No (git-managed) |


The first two rows are *managed*: Specify owns a versioned baseline copy under `.specify/specs/` or `<root>/contracts/`, the change-local copy is a proposed delta, and the merge step is a transactional promotion with conflict detection. The bottom two rows are *external*: the project tree holds the only copy, the build writers edit it directly, and git provides versioning, conflict detection, and rollback.

### Why uniformity in either direction is the wrong answer

Operators occasionally ask why contracts use a delta directory while crates do not, or — pulling the other way — why specs and contracts can't simply be edited in place. Both flavours of uniformity have been considered and ruled out:

- **Pull all artifacts into the delta-then-promote model.** Forcing crates and shells under `.specify/changes/<name>/...` breaks `cargo`/Xcode/Gradle workspace discovery, IDE indexing, and the standard developer loop. Promotion at merge would be a bulk file copy that git already does better. Build artefacts (`target/`, `build/`, gradle caches) under `.specify/` are wasteful.
- **Push specs and contracts into the direct-write model.** Direct writes to root `contracts/` or `.specify/specs/` lose four properties the platform currently provides:
  1. **Sibling-change isolation.** A change post-build / pre-merge would expose its proposal to other in-flight changes that read the baseline as conformance context (per `schemas/contracts/briefs/specs.md`).
  2. **Drop semantics.** `/spec:drop` becomes destructive once build has run; the operator either gives up rollback or schemas must hand-write per-artifact undo.
  3. **File-granularity conflict detection.** `specify change merge conflict-check` compares a change's `defined-at` timestamp against the baseline's last-merged timestamp. Lossless and precise; git's three-way merge is property-agnostic and noisier.
  4. **Reviewable diff.** A reviewer sees exactly the files a change contributes, not a tangled mix of unrelated baseline edits.

The asymmetry is therefore correct as a design intent. The defect is that it is encoded in `crates/merge/`, `crates/validate/`, `src/config.rs`, and individual brief prose, not in a place where a new schema can declare its own choice.

### What this blocks

- A future `infra@v1` schema generating Terraform plans cannot express "the `terraform/` directory is a managed baseline that other changes treat as read-only conformance context" without patching `specify-cli`.
- A future `client-sdk@v1` generating npm packages cannot declare "the `clients/` directory is external — git owns the lifecycle" and inherit drop / validate / overlap behaviour for free.
- Hybrid schemas that own *both* managed contracts and externally-built generated SDKs cannot describe the split.
- The brief-level rule "build must not edit root `contracts/` directly" (currently a sentence in `schemas/contracts/briefs/build.md`) cannot be enforced by the runtime; it relies on author discipline.

## Design

### The model

Every schema declares each output location it owns and tags it with one of two lifecycle modes:

- `**managed*`* — Specify-owned baseline. Build writes to `$CHANGE_DIR/<delta-path>/`. Merge promotes the delta to a baseline path with file-granularity conflict detection. Drop discards the delta. Verifier briefs receive resolved delta and baseline paths. Sibling changes read the baseline (never the delta) as conformance context. Today: specs, contracts.
- `**external**` — downstream output managed by another toolchain. Build writes directly to a path in the project tree. Specify does not maintain a separate baseline, does not promote at merge, does not roll back at drop, and does not run conflict-check. Today: omnia crates, vectis shared / iOS / Android / design-system.

`managed` ≈ "Specify is the authority for this artifact's baseline." `external` ≈ "another toolchain is the authority and Specify just orchestrates the writers."

A single schema may mix both lifecycles. Omnia, for example, owns specs as managed and crates as external in the same `schema.yaml`.

### Schema surface

Add an `artifacts:` block to `schemas/<name>/schema.yaml`. The contracts schema becomes:

```yaml
name: contracts
version: 1
description: API contract definition and validation

artifacts:
  - id: specs
    lifecycle: managed
    delta: specs/
    baseline: .specify/specs/
  - id: contracts
    lifecycle: managed
    delta: contracts/
    baseline: contracts/             # repo root, per the v2 layout

pipeline:
  define:
    - { id: proposal, brief: briefs/proposal.md }
    - { id: specs,    brief: briefs/specs.md }
    - { id: tasks,    brief: briefs/tasks.md }
  build:
    - { id: build,    brief: briefs/build.md }
  merge:
    - { id: merge,    brief: briefs/merge.md }
```

Omnia becomes:

```yaml
name: omnia
version: 1

artifacts:
  - id: specs
    lifecycle: managed
    delta: specs/
    baseline: .specify/specs/
  - id: crates
    lifecycle: external
    project-path: crates/
  - id: guest
    lifecycle: external
    project-path: ./                 # the guest is at the repo root
```

Vectis becomes:

```yaml
name: vectis
version: 2

artifacts:
  - id: specs
    lifecycle: managed
    delta: specs/
    baseline: .specify/specs/
  - id: composition
    lifecycle: managed
    delta: composition.yaml
    baseline: .specify/specs/composition.yaml
  - id: shared
    lifecycle: external
    project-path: shared/
  - id: ios
    lifecycle: external
    project-path: iOS/
  - id: android
    lifecycle: external
    project-path: Android/
  - id: design-system
    lifecycle: external
    project-path: design-system/
```

Field semantics:

- `id` — stable identifier, kebab-case, unique within the schema. Used as a substitution key in briefs (`$ARTIFACT_DELTA[<id>]`).
- `lifecycle` — `managed` or `external`. Required.
- `delta` — for `managed` only. Path relative to `$CHANGE_DIR`. May be a directory (`specs/`) or a single file (`composition.yaml`).
- `baseline` — for `managed` only. Path relative to the project root. The v2-layout choice (root vs. `.specify/`) is per artifact, not global.
- `project-path` — for `external` only. Path relative to the project root, primarily informational (see §"What this enables").

A `managed` artifact MUST declare both `delta` and `baseline`. An `external` artifact MUST declare `project-path` and MUST NOT declare `delta` or `baseline`.

### Behavioural changes per CLI verb

The runtime stops hard-coding "specs and contracts" and iterates over the active schema's `managed` artifacts:

- `**specify change merge {preview, conflict-check, run}**` — the merge crate becomes data-driven. Each `managed` artifact is processed with the same opaque-file-replacement semantics RFC-12 §"Drop `contracts.imports`" / RFC-8 §"Merge" already require.
- `**specify change drop**` — already correct (deletes `.specify/changes/<name>/`); no code change needed because all `managed` deltas live there.
- `**specify change validate**` — gains an `--artifact <id>` filter; verifier briefs receive `$ARTIFACT_DELTA[<id>]` and `$ARTIFACT_BASELINE[<id>]` rather than hard-coded `$CHANGE_DIR/contracts/`.
- `**specify change overlap**` — extended to detect overlap on every `managed` artifact (today: only specs).
- `**specify status` / `specify change status**` — surface "managed artifacts pending merge" and "external artifacts touched" as separate lines so operators see the boundary.
- `**specify contract validate**` (RFC-12) — reads the contracts baseline from the artifact declaration rather than hard-coding `<root>/contracts/`. Side-effect: per-schema contract layouts become possible without re-patching the CLI.

### Brief surface

Build briefs gain a small contract: instead of literal `.specify/changes/<name>/contracts/` paths, a brief receives `$ARTIFACT_DELTA[<id>]` / `$ARTIFACT_BASELINE[<id>]` substitutions resolved by the brief renderer. The current rule in `schemas/contracts/briefs/build.md`:

> Build must not edit root `contracts/` directly. Baseline updates happen only during merge.

becomes a runtime invariant: `lifecycle: managed` declares the rule, the renderer substitutes the delta path into every reference the brief makes, and the verifier flags any direct write to a baseline file as a hard failure (the contract verifier already does this — it just becomes generic).

### Sub-paths

Real schemas care about subdirectories of an artifact (`contracts/http/`, `contracts/messages/`, `contracts/schemas/`). RFC-13 keeps the surface flat: an `artifacts:` entry declares one path, and any sub-layout inside it is the format skill's concern. If a future schema needs sub-path-level lifecycle differences, an extension that adds `sub-paths:` can be considered then. None currently do.

### Schema-version compatibility

A new field `artifacts:` is additive at the YAML level, but the runtime needs to accept both shapes during migration:

- Schemas that declare `artifacts:` use the new code path.
- Schemas that omit it fall back to the current hard-coded behaviour (specs as managed under `.specify/specs/`; everything else untouched), with a deprecation warning emitted by `specify check`.

The deprecation warning is removed (and the fallback retired) one minor version after every first-party schema declares `artifacts:`.

## Alternatives Considered

**Make all artifacts `managed`.** Discussed in §"Why uniformity in either direction is the wrong answer." Rejected: breaks the developer loop; promotion at merge is bulk copy that git already handles; build artefacts under `.specify/` are wasteful.

**Make specs and contracts `external`.** Discussed in §"Why uniformity in either direction is the wrong answer." Rejected: loses sibling-change isolation, drop semantics, file-granularity conflict detection, and reviewable diff.

**Declare lifecycle per `pipeline:` stage instead of per artifact.** Considered. Rejected: a single build stage often produces multiple artifacts with different lifecycles (omnia produces both an external crate and external guest project; a hypothetical hybrid schema produces a managed contract and an external SDK). Tying lifecycle to the writer rather than the output forces an unnatural one-stage-one-artifact decomposition.

**Add a third lifecycle, `audited`, that direct-writes but records a checksum baseline.** Considered for cases where the artifact is too large to live under `.specify/` but Specify still wants tamper detection. Rejected for now: the use case is hypothetical, and `external` plus a downstream `specify drift` (RFC-1's `specify-drift` crate) covers the same ground when needed.

**Top-level `artifacts:` schema in a separate `artifacts.yaml` file.** Considered for symmetry with `registry.yaml`. Rejected: artifacts are schema-bound, not project-bound; they belong in the schema definition where the briefs that produce them already live. A separate file would duplicate the schema-id binding and require its own load / validate path.

## Non-Goals

- **How code is generated.** The internals of `/omnia:crate-writer`, `/vectis:core-writer`, `/contract:openapi`, etc. are unchanged. RFC-13 covers only the lifecycle bookkeeping around their outputs.
- **Format-level contract evolution.** SemVer rules, `info.x-specify-id`, cross-repo uniqueness — RFC-12 owns those and continues to.
- **Git integration.** The relationship to git, `git mv`, `git rebase`, and PR review flows is unchanged.
- **Cloud execution semantics.** RFC-13 stays orthogonal to the cloud-execute roadmap (roadmap §7); both `managed` and `external` artifacts serialise the same way they do today.
- **A sandboxed write-fence.** Refusing writes outside declared paths is powerful but turns Specify into an enforcer; deferred to a follow-up RFC if real misrouting incidents accumulate.
- **Cardinality > 1.** Multiple delta paths per artifact, or multiple baseline targets, are out of scope; the rule is one delta, one baseline.
- **Removing or relocating the existing `specs/` and `contracts/` baselines.** The baseline locations are preserved exactly; only the *declaration* of those locations moves into `schema.yaml`.

## Implementation Scope

### specify-cli

1. New schema field `artifacts:` parsed in `crates/schema/src/`. JSON Schema additions in `.cursor/schemas/specify-schema.schema.json`.
2. `crates/merge/` refactor: replace the hard-coded specs/contracts pair with iteration over the active schema's `managed` artifacts. Tests in `crates/merge/tests/merge_change.rs` extended to cover a third managed artifact (synthetic) to lock in the data-drive.
3. `crates/validate/`: add the `--artifact <id>` filter and the `$ARTIFACT_DELTA[<id>]` / `$ARTIFACT_BASELINE[<id>]` substitutions in the brief renderer.
4. `src/config.rs`: replace `ProjectConfig::specs_dir` and `ProjectConfig::contracts_dir` with a generic `ProjectConfig::baseline_path(&schema, artifact_id)` helper. The two existing helpers remain as thin wrappers during the deprecation window.
5. `specify-check` (RFC-5): add a lint that flags hard-coded `\.specify/changes/.*/contracts/`-shaped paths in skill / brief markdown and recommends the substitution variables.

Estimated total: ~600 lines of Rust. No new merge, workspace-sync, or registry code.

### specify (this repo)

1. Add `artifacts:` blocks to `schemas/contracts/schema.yaml`, `schemas/omnia/schema.yaml`, `schemas/vectis/schema.yaml` (and any other first-party schema not yet listed). Each block expresses **today's** behaviour exactly — no path moves.
2. Update brief prose to use `$ARTIFACT_DELTA[<id>]` / `$ARTIFACT_BASELINE[<id>]` substitutions. The contracts build brief, the contracts specs brief, the omnia build brief, and the vectis build brief are the primary touch points.
3. Update `docs/reference/directory-layout.md`, `docs/explanation/decision-log.md`, and the per-schema READMEs to describe the lifecycle declaration. Cross-link to RFC-13.
4. Add glossary entries for "managed artifact" and "external artifact".

The `contracts@v1` schema id, the `contracts/` baseline directory, the merge semantics, the workspace flow, and every brief / skill name remain unchanged.

## Migration

RFC-13 commits to **zero behavioural change at the file-system level** for existing schemas. The migration is two passes:

1. **Land the surface, fall back to current behaviour.** Schema parsing accepts `artifacts:` when present; the runtime falls back to the existing hard-coded specs/contracts pair when it is absent. Existing tests pass without modification.
2. **Adopt the surface in first-party schemas.** Add `artifacts:` blocks to `schemas/contracts/`, `schemas/omnia/`, `schemas/vectis/` declaring today's paths. Run the merge / drop / validate suites against the data-driven path. Cut over briefs to substitution variables.

A subsequent minor version retires the fallback once every first-party schema declares `artifacts:`. Third-party schemas that have not adopted the field remain functional during the deprecation window and emit a `specify check` warning.

A linter rule in `specify-check` enforces per-managed-artifact invariants:

- A `managed` artifact MUST have a verifier brief OR a documented opt-out.
- A `managed` artifact's `baseline` MUST NOT collide with another schema's `external` `project-path`.
- A schema's pipeline MUST NOT name an output path outside its declared `artifacts:` set (catches "writer skill drifted into an undeclared directory").

## Open Questions

1. **Should the specs baseline move to the repo root?** The v2 layout moved registry, plan, initiative, and contracts. `.specify/specs/` stayed under `.specify/`. With explicit declaration the choice becomes per-artifact rather than per-codebase, but a default policy is still useful. Provisional answer: keep `.specify/specs/` — specs are heavily change-coupled and noisy at root — but the RFC review should confirm.
2. **Cardinality.** Is one-delta / one-baseline a permanent rule or a starting point? Start one-to-one; revisit when a real schema needs more.
3. **Strict mode for write-fencing.** Should the runtime *prevent* writes outside declared `external` paths? Powerful but invasive; deferred to a follow-up RFC.
4. **Versioning the artifacts surface itself.** Add an explicit `artifacts.version: 1` field so future RFCs can extend the shape without breaking older schema readers? Provisional answer: no — the schema's own `version:` already gates compatibility.
5. **Cross-project / workspace-clone semantics.** Do `external` paths resolve relative to the clone's project root or the hub's? Provisional answer: the clone's root, matching how `specify workspace sync` already treats every other path.
6. **Naming.** `managed` vs. `external` for clarity. Alternatives considered: `specify-owned` / `tooling-owned`, `baselined` / `live`, `transactional` / `direct`. RFC review picks one.
7. **Backwards compatibility window.** How long does the fallback for schemas without `artifacts:` survive? Provisional answer: one minor version after every first-party schema adopts the field, gated on a `specify check` warning.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) — the workspace foundation that owns `crates/merge/`, `crates/validate/`, and `src/config.rs`.
- [RFC-8: API contracts](archive/rfc-8-api-contracts.md) — introduced the `contracts@v1` schema and the change-local delta directory; supplies the merge semantics RFC-13 generalises.
- [RFC-9: Platform](archive/rfc-9-platform.md) — moved registry, plan, initiative, and contracts to the repo root in the v2 layout; established the operator-vs-framework path boundary RFC-13 builds on.
- [RFC-12: Refine RFC-8](archive/rfc-12-refine-rfc-8.md) — SemVer + `info.x-specify-id` rules and `specify contract validate`; the contract baseline path it consumes becomes data-driven under RFC-13.
- [RFC-5: Framework Linter](rfc-5-lint.md) — owner of the lint that enforces RFC-13's per-managed-artifact invariants.
- `plugins/contract/references/baseline-vs-delta.md` — the cross-format author rules whose path constants become substitution variables under RFC-13.
- `docs/how-to/migrate-to-v2-layout.md` — operator-facing description of the v2 layout boundary RFC-13 makes per-artifact configurable.


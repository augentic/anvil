# Capabilities

> Status: Draft (Phase 0.1 of [RFC-13](../../rfcs/rfc-13-extensibility.md)). The post-RFC manifest shape and dependency invariants are pinned; the first-party capabilities (`omnia`, `contracts`, `vectis`) migrate from `schemas/<name>/schema.yaml` to `capabilities/<name>/capability.yaml` in Phase 1.5.

## What is a capability?

A capability is a versioned Specify extension that describes how the fixed `define → build → merge` slice loop creates an outcome domain's artefacts. **Capabilities own outcome artefacts and their mechanics; platform components coordinate where and when those per-project slices run** ([RFC-13 §Principle](../../rfcs/rfc-13-extensibility.md#principle)). The phase set, transition DAG, and per-phase outcome contract recorded in `.metadata.yaml` are part of the immutable Specify core. Capabilities populate the loop with per-domain briefs and skills, but never declare the phases themselves.

Outcomes are not necessarily code: a capability can deliver contracts, documentation, policy, infrastructure, fixtures, generated clients, or any other reviewable artefact. Imperative behaviour (validation, generation, review, adoption, cleanup) lives in the capability's skills and helper scripts, not in the manifest.

## Manifest shape

Every capability ships a single `capability.yaml` at its root. The post-RFC manifest carries exactly four top-level fields and a closed `pipeline` object:

```yaml
name: vectis
version: 2
description: Vectis Crux application workflow

pipeline:
  define:
    - id: draft-proposal
      brief: briefs/proposal.md
    - id: draft-specs
      brief: briefs/specs.md
    - id: draft-composition
      brief: briefs/composition.md
    - id: draft-design
      brief: briefs/design.md
    - id: draft-tasks
      brief: briefs/tasks.md
  build:
    - id: implement
      brief: briefs/build.md
  merge:
    - id: prepare-merge
      brief: briefs/merge.md
```

| Field         | Required | Meaning                                                                                                               |
| ------------- | -------- | --------------------------------------------------------------------------------------------------------------------- |
| `name`        | yes      | Kebab-case capability identifier. Must match the directory name under `capabilities/`.                                |
| `version`     | yes      | Integer ≥ 1. Increments when the capability ships breaking pipeline or contract changes.                              |
| `description` | yes      | Single-sentence summary of the capability's outcome domain.                                                           |
| `pipeline`    | yes      | Closed object with exactly three keys: `define`, `build`, `merge`. Each is an ordered list of pipeline entries.       |

Each pipeline entry is `{ id, brief }`:

| Field   | Required | Meaning                                                                                                              |
| ------- | -------- | -------------------------------------------------------------------------------------------------------------------- |
| `id`    | yes      | Kebab-case brief identifier. Unique within the manifest and equal to the brief file's frontmatter `id`.              |
| `brief` | yes      | Relative path (from the manifest) to the markdown brief template. URIs and absolute paths are rejected.              |

The post-RFC manifest deliberately drops the legacy `domain` and `extends` fields. Tech-stack guidance, architectural notes, and testing context belong in capability references and skills, not in always-loaded manifest metadata. The schema is closed (`additionalProperties: false`) so attempts to reintroduce these fields fail loudly at load time.

## Pipeline and the slice loop

`pipeline:` declares which briefs the core renders for each fixed slice phase. The set of phases is frozen by [RFC-13](../../rfcs/rfc-13-extensibility.md#design): exactly **`define`**, **`build`**, and **`merge`**. Variation that capabilities legitimately want lives in the briefs they enumerate per phase and in skill-owned imperative behaviour — never in the phase list itself.

`pipeline.plan` is intentionally **absent** from the manifest. Planning is orchestration, not capability-owned slice work, and lives on the `specify change` platform component:

- the change brief (`change.md`) records operator intent,
- the change plan (`plan.yaml`) sequences slices across one or more projects,
- planning briefs and the `/spec:plan` (later: `/change:plan`) authoring loop sit on the change surface rather than inside any capability's manifest.

A slice flowing through `define → build → merge` therefore reads exactly one capability's pipeline. Cross-capability outcomes are coordinated by change plan entries, not by fusing capabilities into a larger hidden pipeline.

The merge brief signals go/no-go through the existing slice outcome contract (`specify slice outcome set` and `specify slice journal append`); the core does not parse capability diagnostics — they round-trip as opaque journal entries. See [RFC-13 §Merge and adoption contract](../../rfcs/rfc-13-extensibility.md#merge-and-adoption-contract).

## Dependency direction

The post-RFC dependency graph is one-way:

```text
specify-change ──▶ specify-registry ──▶ specify-core
```

`specify-core` owns the slice loop and capability resolution. `specify-registry` owns topology (`registry.yaml`) plus the local materialised view (`.specify/workspace/`). `specify-change` owns operator intent (`change.md`) plus the executable plan (`plan.yaml`) and orchestrates slices through the core loop, possibly across projects materialised by the registry.

The invariant is: **`specify-core` does not depend on `specify-registry` or `specify-change`**, and `specify-registry` does not depend on `specify-change`. Platform components compose downward; they never re-enter the core. This is enforced as a lint via [RFC-5](../../rfcs/rfc-5-lint.md) (see also [RFC-13 §Migration](../../rfcs/rfc-13-extensibility.md#migration), invariant 4).

Registry and the change component are first-party Specify components, but they are **not** capabilities: they do not appear in any `capability.yaml`, they are not activated through the manifest protocol, and the core never switches on a capability name to invoke them.

## Distribution

A capability ships a manifest plus the skills and references that implement domain behaviour. The manifest is the only declarative surface; everything imperative — provider configuration, file generation, format validation, drift detection, fixture replay — lives in skills under `plugins/<name>/` and in checked-in helper scripts.

The security posture is therefore the skill and tooling posture: capability skills run through the host agent's tool execution model. RFC-13 deliberately does not introduce a second plugin runtime hidden behind `capability.yaml`.

## Validation

The wire-level schema is [`capabilities/capability.schema.json`](../../capabilities/capability.schema.json) (JSON Schema draft 2020-12). It enforces the field set and shape described above and is the source of truth for both first-party and third-party capability authors.

Once Phase 1.5 lands the `schemas/<name>/schema.yaml` → `capabilities/<name>/capability.yaml` move, `make checks` will validate every first-party manifest against this schema and run the existing pipeline-integrity checks (unique brief ids, brief paths resolve, brief frontmatter `id` matches the manifest, no cycles in the `needs:` graph).

## See also

- [RFC-13: Extensibility](../../rfcs/rfc-13-extensibility.md) — capability protocol, platform components, and migration plan.
- [RFC-14: Workspaces](../../rfcs/rfc-14-workspaces.md) — multi-domain repositories layered on top of the capability manifest protocol.
- `docs/reference/registry.md` — registry topology and workspace materialisation (lands in Phase 2.10 of the RFC-13 plan).
- `docs/reference/change-component.md` — change brief, plan, execution, and finalization (lands in Phase 2.10 of the RFC-13 plan).

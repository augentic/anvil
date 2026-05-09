# Capabilities

> Status: Draft (Phase 3.11 of [RFC-13](../../../rfcs/archive/rfc-13-extensibility.md) landed). The post-RFC manifest shape and dependency invariants are pinned and the first-party capabilities (`default`, `omnia`, `contracts`, `vectis`) now live at [`capabilities/<name>/capability.yaml`](../../../capabilities/). [`capability.schema.json`](../../../capabilities/capability.schema.json) actively rejects `pipeline.plan` — planning briefs live with the change-planning skill at [`plugins/change/skills/plan/briefs/<capability>/`](../../../plugins/change/skills/plan/briefs/).

## What is a capability?

A capability is a versioned Specify extension that describes how the fixed `define → build → merge` slice loop creates an outcome domain's artefacts. **Capabilities own outcome artefacts and their mechanics; platform components coordinate where and when those per-project slices run** ([RFC-13 §Principle](../../../rfcs/archive/rfc-13-extensibility.md#principle)). The phase set, transition DAG, and per-phase outcome contract recorded in `.metadata.yaml` are part of the immutable Specify core. Capabilities populate the loop with per-domain briefs and skills, but never declare the phases themselves.

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

The post-RFC manifest deliberately drops the legacy `domain` and `extends` fields. Tech-stack guidance, architectural notes, testing context, and codex rule directories belong beside the manifest in capability-owned files, not in always-loaded manifest metadata. The schema is closed (`additionalProperties: false`) so attempts to reintroduce these fields fail loudly at load time.

## Pipeline and the slice loop

`pipeline:` declares which briefs the core renders for each fixed slice phase. The set of phases is frozen by [RFC-13](../../../rfcs/archive/rfc-13-extensibility.md#design): exactly **`define`**, **`build`**, and **`merge`**. Variation that capabilities legitimately want lives in the briefs they enumerate per phase and in skill-owned imperative behaviour — never in the phase list itself.

`pipeline.plan` is intentionally **absent** from the post-RFC manifest — and actively rejected by [`capability.schema.json`](../../../capabilities/capability.schema.json) as of RFC-13 §3.11. Planning is orchestration, not capability-owned slice work, and lives on the `specify change` platform component:

- the slice brief (`change.md`) records operator intent,
- the slice plan (`plan.yaml`) sequences slices across one or more projects,
- planning briefs and the `/change:plan` (later: `/change:plan`) authoring loop sit on the slice surface rather than inside any capability's manifest.

A slice flowing through `define → build → merge` therefore reads exactly one capability's pipeline. Cross-capability outcomes are coordinated by change plan entries, not by fusing capabilities into a larger hidden pipeline.

The merge brief signals go/no-go through the existing slice outcome contract (`specify slice outcome set` and `specify slice journal append`); the core does not parse capability diagnostics — they round-trip as opaque journal entries. See [RFC-13 §Merge and adoption contract](../../../rfcs/archive/rfc-13-extensibility.md#merge-and-adoption-contract).

## Dependency direction

The post-RFC dependency graph is one-way:

```text
specify-change ──▶ specify-registry ──▶ specify-core
```

`specify-core` owns the slice loop and capability resolution. `specify-registry` owns topology (`registry.yaml`) plus the local materialised view (`.specify/workspace/`). `specify-change` owns operator intent (`change.md`) plus the executable plan (`plan.yaml`) and orchestrates slices through the core loop, possibly across projects materialised by the registry.

The invariant is: **`specify-core` does not depend on `specify-registry` or `specify-change`**, and `specify-registry` does not depend on `specify-change`. Platform components compose downward; they never re-enter the core. This is enforced as a lint via [RFC-5](../../../rfcs/rfc-5-lint.md) (see also [RFC-13 §Migration](../../../rfcs/archive/rfc-13-extensibility.md#migration), invariant 4).

Registry and the slice component are first-party Specify components, but they are **not** capabilities: they do not appear in any `capability.yaml`, they are not activated through the manifest protocol, and the core never switches on a capability name to invoke them.

## Distribution

A capability ships a manifest plus the skills and references that implement domain behaviour. The manifest is the only declarative surface; everything imperative — provider configuration, file generation, format validation, drift detection, fixture replay — lives in skills under `plugins/<name>/` and in checked-in helper scripts.

The security posture is therefore the skill and tooling posture: capability skills run through the host agent's tool execution model. RFC-13 deliberately does not introduce a second plugin runtime hidden behind `capability.yaml`.

Capabilities may also ship an optional `codex/` directory by convention. Codex files are Markdown review rules with their own frontmatter contract and are resolved outside `capability.yaml`; do not add a `codex` field to the manifest. The first-party `default` capability carries universal capability-independent rules, while domain capabilities may add rules specific to their artifact and implementation boundaries.

`specify codex *` resolves rule sources in this order: `default` capability, project capability, future shared catalogs, then the repo-root `codex/` overlay. First-party `default` is distributed as a normal capability under `capabilities/default`; regular `specify init <capability>` caches it into `.specify/.cache/default/` when the selected capability comes from a tree with that sibling. This makes the foundational codex available after init without adding manifest fields or a separate rule package.

## Validation

The wire-level schema is [`capabilities/capability.schema.json`](../../../capabilities/capability.schema.json) (JSON Schema draft 2020-12). It enforces the field set and shape described above and is the source of truth for both first-party and third-party capability authors.

`make checks` validates every first-party manifest under [`capabilities/`](../../../capabilities/) against this schema and runs the pipeline-integrity checks (unique brief ids, brief paths resolve, brief frontmatter `id` matches the manifest, no cycles in the `needs:` graph).

## See also

- [RFC-13: Extensibility](../../../rfcs/archive/rfc-13-extensibility.md) — capability protocol, platform components, and migration plan.
- [RFC-14: Workspaces](../../../rfcs/archive/rfc-14-workspace.md) — multi-domain repositories layered on top of the capability manifest protocol.
- [Registry](../registry.md) — registry topology and workspace materialisation.
- [Change Component](../change-component.md) — change brief, plan, execution, and finalization.

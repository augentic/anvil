# Adapters

> The adapter manifest shape and dependency invariants are pinned. The first-party adapters (`default`, `omnia`, `contracts`, `vectis`) live at [`adapters/<name>/adapter.yaml`](../../../adapters/). [`adapter.schema.json`](../../../adapters/adapter.schema.json) actively rejects `pipeline.plan` — planning briefs live with the change-draft skill at [`plugins/change/skills/draft/briefs/<adapter>/`](../../../plugins/change/skills/draft/briefs/).

## What is a adapter?

A adapter is a versioned Specify extension that describes how the fixed `define → build → merge` slice loop creates an outcome domain's artefacts. **Adapters own outcome artefacts and their mechanics; platform components coordinate where and when those per-project slices run.** The phase set, transition DAG, and the result each phase reports back via `.metadata.yaml` are part of the immutable Specify core. Adapters populate the loop with per-domain briefs and skills, but never declare the phases themselves.

Outcomes are not necessarily code: a adapter can deliver contracts, documentation, policy, infrastructure, fixtures, generated clients, or any other reviewable artefact. Imperative behaviour (validation, generation, review, adoption, cleanup) lives in the adapter's skills and helper scripts, not in the manifest.

## Manifest shape

Every adapter ships a single `adapter.yaml` at its root. The post-RFC manifest carries exactly four top-level fields and a closed `pipeline` object:

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
| `name`        | yes      | Kebab-case adapter identifier. Must match the directory name under `adapters/`.                                |
| `version`     | yes      | Integer ≥ 1. Increments when the adapter ships breaking pipeline or contract changes.                              |
| `description` | yes      | Single-sentence summary of the adapter's outcome domain.                                                           |
| `pipeline`    | yes      | Closed object with exactly three keys: `define`, `build`, `merge`. Each is an ordered list of pipeline entries.       |

Each pipeline entry is `{ id, brief }`:

| Field   | Required | Meaning                                                                                                              |
| ------- | -------- | -------------------------------------------------------------------------------------------------------------------- |
| `id`    | yes      | Kebab-case brief identifier. Unique within the manifest and equal to the brief file's frontmatter `id`.              |
| `brief` | yes      | Relative path (from the manifest) to the markdown brief template. URIs and absolute paths are rejected.              |

The post-RFC manifest deliberately drops the legacy `domain` and `extends` fields. Tech-stack guidance, architectural notes, testing context, and codex rule directories belong beside the manifest in adapter-owned files, not in always-loaded manifest metadata. The schema is closed (`additionalProperties: false`) so attempts to reintroduce these fields fail loudly at load time.

## Pipeline and the slice loop

`pipeline:` declares which briefs the core renders for each fixed slice phase. The set of phases is frozen: exactly **`define`**, **`build`**, and **`merge`**. Variation that adapters legitimately want lives in the briefs they enumerate per phase and in skill-owned imperative behaviour — never in the phase list itself.

`pipeline.plan` is intentionally **absent** from the manifest — and actively rejected by [`adapter.schema.json`](../../../adapters/adapter.schema.json). Planning is orchestration, not adapter-owned slice work, and lives on the `specify change` platform component:

- the slice brief (`change.md`) records operator intent,
- the slice plan (`plan.yaml`) sequences slices across one or more projects,
- planning briefs and the `/change:draft` authoring loop sit on the slice surface rather than inside any adapter's manifest.

A slice flowing through `define → build → merge` therefore reads exactly one adapter's pipeline. Cross-adapter outcomes are coordinated by change plan entries, not by fusing adapters into a larger hidden pipeline.

The merge brief signals go/no-go through the slice outcome that the phase reports back (`specify slice outcome set` and `specify slice journal append`); the core does not parse adapter diagnostics — they round-trip as opaque journal entries.

## Dependency direction

The post-RFC dependency graph is one-way:

```text
specify-change ──▶ specify-registry ──▶ specify-core
```

`specify-core` owns the slice loop and adapter resolution. `specify-registry` owns topology (`registry.yaml`) plus the local materialised view (`.specify/workspace/`). `specify-change` owns operator intent (`change.md`) plus the executable plan (`plan.yaml`) and orchestrates slices through the core loop, possibly across projects materialised by the registry.

The invariant is: **`specify-core` does not depend on `specify-registry` or `specify-change`**, and `specify-registry` does not depend on `specify-change`. Platform components compose downward; they never re-enter the core. A workspace lint enforces it.

Registry and the slice component are first-party Specify components, but they are **not** adapters: they do not appear in any `adapter.yaml`, they are not activated through the manifest protocol, and the core never switches on a adapter name to invoke them.

## Distribution

A adapter ships a manifest plus the skills and references that implement domain behaviour. The manifest is the only declarative surface; everything imperative — provider configuration, file generation, format validation, drift detection, fixture replay — lives in skills under `plugins/<name>/` and in checked-in helper scripts.

The security posture is therefore the skill and tooling posture: adapter skills run through the host agent's tool execution model. There is no second plugin runtime hidden behind `adapter.yaml`.

Adapters may also ship an optional `codex/` directory by convention. Codex files are Markdown review rules with their own frontmatter contract and are resolved outside `adapter.yaml`; do not add a `codex` field to the manifest. The first-party `default` adapter carries universal adapter-independent rules, while domain adapters may add rules specific to their artifact and implementation boundaries.

`specify codex *` resolves rule sources in this order: `default` adapter, project adapter, future shared catalogs, then the repo-root `codex/` overlay. First-party `default` is distributed as a normal adapter under `adapters/default`; regular `specify init <adapter>` caches it into `.specify/.cache/default/` when the selected adapter comes from a tree with that sibling. This makes the foundational codex available after init without adding manifest fields or a separate rule package.

## Validation

The wire-level schema is [`adapters/adapter.schema.json`](../../../adapters/adapter.schema.json) (JSON Schema draft 2020-12). It enforces the field set and shape described above and is the source of truth for both first-party and third-party adapter authors.

`make checks` validates every first-party manifest under [`adapters/`](../../../adapters/) against this schema and runs the pipeline-integrity checks (unique brief ids, brief paths resolve, brief frontmatter `id` matches the manifest, no cycles in the `needs:` graph).

## See also

- [Registry](../registry.md) — registry topology and workspace materialisation.
- [Change Component](../change-component.md) — change brief, plan, execution, and finalization.

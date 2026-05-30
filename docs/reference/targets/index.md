# Target Adapters

> Target adapters declare the output side of the source/target split (see [Anatomy of an adapter](../../explanation/adapter-anatomy.md) for the full contract). The first-party targets (`omnia`, `vectis`, `contracts`) live at [`adapters/targets/<name>/adapter.yaml`](https://github.com/augentic/specify/tree/main/adapters/targets). The source-side counterparts live at [`adapters/sources/<name>/`](https://github.com/augentic/specify/tree/main/adapters/sources).

## What is a target adapter?

A **target adapter** is the output role in the Specify 2.0 plugin model. It describes how the core `refine → build → merge` slice loop produces an outcome domain's artefacts. Three operations:

- `shape` — idiom guidance consumed by core synthesis. Read into context when `/spec:refine` writes `spec.md` / `design.md`. Empty `shape` is valid.
- `build` — implementation drive: read `spec.md` + `design.md`, write code (and any target-specific structured manifests like Vectis `composition.yaml`), run target-local validation.
- `merge` — landing gate: validate the slice's output against the baseline, surface conflicts, drive the target's verification commands.

Target adapters do not own `spec.md` or `design.md` synthesis — that is **core**'s responsibility. The plan-level `Slice.target` field selects the target; v1 supports one target per project.

## Manifest shape

Every target adapter ships a single `adapter.yaml` at `adapters/targets/<name>/`:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/augentic/specify-cli/main/schemas/target.schema.json
name: omnia
version: 1
axis: target
execution: agent
description: Omnia Rust WASM target adapter.
operations: [shape, build, merge]
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
```

| Field         | Required | Meaning |
| ------------- | -------- | ------- |
| `name`        | yes      | Kebab-case target identifier. Must match the directory name under `adapters/targets/`. |
| `version`     | yes      | Integer ≥ 1. Increments when the adapter ships breaking pipeline or contract changes. |
| `axis`        | yes      | Must be `target`. |
| `execution`   | yes      | Closed mode (`agent` \| `tool`). `agent` forces `cache: opt-out` and runs the brief via an agent; first-party targets currently declare `agent`. |
| `description` | yes      | Single-sentence summary of the target's outcome domain. |
| `operations`  | yes      | Closed list with exactly the three values `[shape, build, merge]`. |
| `briefs`      | yes      | Map of operation → brief markdown path relative to the manifest. |

Optional `tools[]` declares WASI helpers that the host runner caches under the per-axis manifest cache at `.specify/.cache/manifests/targets/<name>/`. See [Tool declarations](../../explanation/tool-declarations.md).

## How a target adapter participates in the loop

```text
/spec:refine  →  reads target.shape   (idiom guidance for synthesis)
/spec:build   →  reads target.build   (drives code generation)
/spec:merge   →  reads target.merge   (validates and lands the slice)
```

Core synthesis writes the canonical artifacts (`proposal.md` / `spec.md` / `design.md` / `tasks.md`) in a fixed substep order regardless of target. The `shape` brief is read into context as idiom guidance but never replaces synthesis output.

## Dependency direction

The dependency graph is one-way; `specify-core` never depends on the plugin loader's axis routing:

```text
specify (binary)
   └─ specify-workflow
        ├─ specify-tool
        └─ plugin loader (adapters/sources/ + adapters/targets/)
              └─ specify-error
```

The invariant: **adapter resolution is a downstream concern**. Core owns the slice loop; adapters supply briefs.

## Distribution

A target adapter ships a manifest plus the briefs that implement domain behaviour. Imperative behaviour (provider configuration, file generation, format validation, drift detection) lives in the briefs and in checked-in helper tools. There is no second plugin runtime hidden behind `adapter.yaml`.

Shared material used by multiple adapters lives outside the adapter roots under `adapters/shared/`:

- **`codex/universal/`** — shared **engineering standards** (`UNI-*`) at [`adapters/shared/rules/universal/`](../../../adapters/shared/rules/universal/); per-target overlays stay at `adapters/targets/<name>/rules/`. See [Standards layer](../../explanation/standards-layer.md).
- **`target-hooks/replay/`** — shared build-time replay hook contract at [`adapters/shared/target-hooks/replay/`](../../../adapters/shared/target-hooks/replay/); per-target runners stay at `adapters/targets/<name>/briefs/build/replay.md` when implemented.

## Validation

The wire-level schema is `schemas/target.schema.json` (distributed with the binary). It enforces the field set and shape described above. `specrun target resolve <value>` loads and validates the manifest on first use.

## See also

- [Anatomy of an adapter](../../explanation/adapter-anatomy.md) — full source/target contract.
- [Registry](../registry.md) — workspace topology that routes slices to target projects.
- Per-target reference: [Omnia](omnia.md), [Vectis](vectis.md), [Contracts](contracts.md).

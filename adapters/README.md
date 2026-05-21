# Adapters (legacy shared assets)

This directory hosts the **pre-RFC-25 `default/` adapter** and **shared Vectis assets** (schemas, codex, examples). The first-party source / target adapter manifests that drive RFC-25 workflows now live under [`sources/`](../sources/) and [`targets/`](../targets/); see [docs/explanation/adapter-anatomy.md](../docs/explanation/adapter-anatomy.md) for the new layout.

## What's here

- [`adapter.schema.json`](adapter.schema.json) — JSON Schema (draft 2020-12) for the legacy `adapter.yaml` shape. Validated by `make checks` against any `adapter.yaml` that remains in this tree. New source / target manifests instead validate against the per-axis `source.schema.json` / `target.schema.json` schemas distributed with the `specify` CLI.
- [`default/`](default/adapter.yaml) — **live.** Foundational Specify workflow and universal review codex (`UNI-*` rules). Kept under the legacy manifest shape until it is migrated to a target adapter.
- `omnia/` — **retired in RFC-25 W2.5**; moved to [`targets/omnia/`](../targets/omnia/) under the source / target split.
- `contracts/` — **retired in RFC-25 W2.7**; moved to [`targets/contracts/`](../targets/contracts/) under the source / target split.
- [`vectis/`](vectis/) — **manifest retired in RFC-25 W2.6** (now at [`targets/vectis/`](../targets/vectis/)); the directory retains the shared `composition.schema.json` / `tokens.schema.json` / `assets.schema.json` plus `codex/` rules and `examples/` consumed by Vectis writers and downstream tooling at their published `$id` URLs.

Each remaining first-party legacy adapter ships a directory of the shape:

```text
adapters/<name>/
├── adapter.yaml         # validates against adapter.schema.json
├── briefs/                 # markdown brief templates referenced from pipeline:
├── codex/                  # optional codex rule files owned by the adapter
└── README.md               # adapter-specific notes
```

Imperative behaviour (validation, generation, review, adoption, cleanup) lives in adapter skills and references under `plugins/<name>/`, **not** in the manifest. The manifest only declares the brief flow. Codex directories are an optional convention beside the manifest: they distribute review rules, but they are not fields inside `adapter.yaml`.

## Validation

`make checks` validates every `adapter.yaml` in this directory against `adapter.schema.json` and runs the integrity checks (unique brief ids, brief paths resolve, brief frontmatter `id` matches the manifest, no cycles in the `needs:` graph). RFC-25 source / target manifests are validated separately against `schemas/source.schema.json` and `schemas/target.schema.json`.

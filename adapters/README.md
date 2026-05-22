# Adapters (legacy shared assets)

This directory hosts **shared Vectis assets** (schemas, codex, examples). The first-party source / target adapter manifests that drive RFC-25 workflows live under [`sources/`](../sources/) and [`targets/`](../targets/); see [docs/explanation/adapter-anatomy.md](../docs/explanation/adapter-anatomy.md) for the layout.

## What's here

- [`adapter.schema.json`](adapter.schema.json) — JSON Schema (draft 2020-12) for the legacy `adapter.yaml` shape. Retained for migration fixtures and historical reference; live source / target manifests validate against the per-axis `source.schema.json` / `target.schema.json` schemas distributed with the `specify` CLI.
- `default/` — **retired in RFC-25 F2**; moved to [`targets/default/`](../targets/default/) as the foundational target adapter carrying universal review codex (`UNI-*` rules).
- `omnia/` — **retired in RFC-25 W2.5**; moved to [`targets/omnia/`](../targets/omnia/) under the source / target split.
- `contracts/` — **retired in RFC-25 W2.7**; moved to [`targets/contracts/`](../targets/contracts/) under the source / target split.
- [`vectis/`](vectis/) — **manifest retired in RFC-25 W2.6** (now at [`targets/vectis/`](../targets/vectis/)); the directory retains the shared `composition.schema.json` / `tokens.schema.json` / `assets.schema.json` plus `codex/` rules and `examples/` consumed by Vectis writers and downstream tooling at their published `$id` URLs.

## Validation

`make checks` validates every `sources/<name>/adapter.yaml` and `targets/<name>/adapter.yaml` against the axis-specific schemas in the sibling `specify-cli` repository (`schemas/source.schema.json` and `schemas/target.schema.json`). First-party codex rules are discovered under `sources/<cap>/codex/` and `targets/<cap>/codex/`.

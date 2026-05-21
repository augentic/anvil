# Adapters

This directory holds the first-party Specify **adapter manifests**. A adapter describes how Specify's `define → build → merge` slice loop creates an outcome domain's artefacts: it bundles a small declarative manifest with the skills and references that implement the domain's behaviour.

## What's here

- [`adapter.schema.json`](adapter.schema.json) — JSON Schema (draft 2020-12) that every `adapter.yaml` validates against. The post-RFC-13 manifest carries only `name`, `version`, `description`, and `pipeline { define, build, merge }`; the schema actively rejects the dropped `domain`, `extends`, and `pipeline.plan` fields. RFC-13 §3.11 moved the planning briefs the omnia and vectis manifests used to declare into the change-component planning skill (`plugins/change/skills/plan/briefs/<adapter>/`) — planning is orchestration, not adapter-owned slice work.
- [`default/`](default/adapter.yaml) — Foundational Specify workflow and universal review codex.
- `omnia/` — **Retired in RFC-25 W2.5**; moved to [`targets/omnia/`](../targets/omnia/) under the source / target split.
- `contracts/` — **Retired in RFC-25 W2.7**; moved to `targets/contracts/` under the source / target split.
- `vectis/` — **Retired in RFC-25 W2.6**; moved to `targets/vectis/` under the source / target split.

Each first-party adapter ships a directory of the shape:

```text
adapters/<name>/
├── adapter.yaml         # validates against adapter.schema.json
├── briefs/                 # markdown brief templates referenced from pipeline:
├── codex/                  # optional codex rule files owned by the adapter
└── README.md               # adapter-specific notes
```

Imperative behaviour (validation, generation, review, adoption, cleanup) lives in adapter skills and references under `plugins/<name>/`, **not** in the manifest. The manifest only declares the brief flow. Codex directories are an optional convention beside the manifest: they distribute review rules, but they are not fields inside `adapter.yaml`.

## Reading order

1. [`docs/reference/adapters/index.md`](../docs/reference/adapters/index.md) — manifest protocol, dependency direction (core ← change ← registry), and validation entry point.
2. [`adapter.schema.json`](adapter.schema.json) — the wire-level schema each manifest must validate against.
3. [`rfcs/archive/rfc-13-extensibility.md`](../rfcs/archive/rfc-13-extensibility.md) — the design rationale.

## Validation

`make checks` validates every `adapter.yaml` in this directory against `adapter.schema.json` and runs the integrity checks (unique brief ids, brief paths resolve, brief frontmatter `id` matches the manifest, no cycles in the `needs:` graph). The same schema is the source of truth for any third-party adapter author.

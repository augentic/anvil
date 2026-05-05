# Capabilities

This directory holds first-party Specify **capability manifests**. A capability describes how Specify's `define → build → merge` slice loop creates an outcome domain's artefacts: it bundles a small declarative manifest with the skills and references that implement the domain's behaviour.

Today this directory contains only [`capability.schema.json`](capability.schema.json). The first-party domain capabilities (`omnia`, `contracts`, `vectis`) still live under [`schemas/`](../schemas/) and migrate here in Phase 1.5 of the [RFC-13](../rfcs/rfc-13-extensibility.md) landing — at which point each will gain its own `capabilities/<name>/capability.yaml`.

## What's here

- `capability.schema.json` — JSON Schema (draft 2020-12) that every `capability.yaml` validates against. The post-RFC-13 manifest carries only `name`, `version`, `description`, and `pipeline { define, build, merge }`; the schema actively rejects the dropped `domain`, `extends`, and `pipeline.plan` fields.

## What's coming

Once Phase 1.5 lands, each first-party capability ships a directory of the shape:

```text
capabilities/<name>/
├── capability.yaml         # validates against capability.schema.json
├── briefs/                 # markdown brief templates referenced from pipeline:
└── README.md               # capability-specific notes
```

Imperative behaviour (validation, generation, review, adoption, cleanup) lives in capability skills and references under `plugins/<name>/`, **not** in the manifest. The manifest only declares the brief flow.

## Reading order

1. [`docs/reference/capabilities.md`](../docs/reference/capabilities.md) — manifest protocol, dependency direction (core ← change ← registry), and validation entry point.
2. [`capability.schema.json`](capability.schema.json) — the wire-level schema each manifest must validate against.
3. [`rfcs/rfc-13-extensibility.md`](../rfcs/rfc-13-extensibility.md) — the design rationale.

## Validation

`make checks` validates manifests in this directory against `capability.schema.json` once Phase 1.5 lands them; the same schema is the source of truth for any third-party capability author.

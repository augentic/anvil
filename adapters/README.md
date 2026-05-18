# Capabilities

This directory holds the first-party Specify **capability manifests**. A capability describes how Specify's `define → build → merge` slice loop creates an outcome domain's artefacts: it bundles a small declarative manifest with the skills and references that implement the domain's behaviour.

## What's here

- [`capability.schema.json`](capability.schema.json) — JSON Schema (draft 2020-12) that every `capability.yaml` validates against. The post-RFC-13 manifest carries only `name`, `version`, `description`, and `pipeline { define, build, merge }`; the schema actively rejects the dropped `domain`, `extends`, and `pipeline.plan` fields. RFC-13 §3.11 moved the planning briefs the omnia and vectis manifests used to declare into the change-component planning skill (`plugins/change/skills/plan/briefs/<capability>/`) — planning is orchestration, not capability-owned slice work.
- [`default/`](default/capability.yaml) — Foundational Specify workflow and universal review codex.
- [`omnia/`](omnia/capability.yaml) — Omnia Rust WASM workflow.
- [`contracts/`](contracts/capability.yaml) — API contract definition and validation (JSON Schema, OpenAPI 3.1, AsyncAPI 3.0).
- [`vectis/`](vectis/capability.yaml) — Vectis Crux cross-platform workflow (Rust core, iOS shell, Android shell, design system).

Each first-party capability ships a directory of the shape:

```text
capabilities/<name>/
├── capability.yaml         # validates against capability.schema.json
├── briefs/                 # markdown brief templates referenced from pipeline:
├── codex/                  # optional codex rule files owned by the capability
└── README.md               # capability-specific notes
```

Imperative behaviour (validation, generation, review, adoption, cleanup) lives in capability skills and references under `plugins/<name>/`, **not** in the manifest. The manifest only declares the brief flow. Codex directories are an optional convention beside the manifest: they distribute review rules, but they are not fields inside `capability.yaml`.

## Reading order

1. [`docs/reference/capabilities/index.md`](../docs/reference/capabilities/index.md) — manifest protocol, dependency direction (core ← change ← registry), and validation entry point.
2. [`capability.schema.json`](capability.schema.json) — the wire-level schema each manifest must validate against.
3. [`rfcs/archive/rfc-13-extensibility.md`](../rfcs/archive/rfc-13-extensibility.md) — the design rationale.

## Validation

`make checks` validates every `capability.yaml` in this directory against `capability.schema.json` and runs the integrity checks (unique brief ids, brief paths resolve, brief frontmatter `id` matches the manifest, no cycles in the `needs:` graph). The same schema is the source of truth for any third-party capability author.

# Interface Contracts

Format-specialist skills to author, import, and verify interface contracts — OpenAPI 3.1 HTTP APIs, AsyncAPI 3.0 evented messaging, and standalone JSON Schema (Draft 2020-12) — from Specify artifacts.

Each format skill owns three intents (author, import, verify) inside a single skill directory. The shared cross-format references under `references/` carry the format-neutral rules every skill obeys.

## Skills

| Skill | Description |
|-------|-------------|
| [openapi](skills/openapi/SKILL.md) | OpenAPI 3.1 HTTP API contracts — path operations, request and response schemas, parameters, security schemes, and baseline deltas. Author from specs, import external OpenAPI / Swagger 2.0 documents, and verify internal consistency or cross-project consumer compatibility. |
| [asyncapi](skills/asyncapi/SKILL.md) | AsyncAPI 3.0 evented contracts — pub/sub, streaming, queue, and WebSocket-style channels, operations, and message bindings. Author from specs, import external AsyncAPI 2.x or 3.0 documents, and verify internal consistency or cross-project consumer compatibility. |
| [json-schema](skills/json-schema/SKILL.md) | Standalone JSON Schema (Draft 2020-12) payload vocabulary shared by the protocol skills. Author reusable schemas from specs, import external schema files (Draft 4 / 6 / 7 / 2019-09 / 2020-12), and verify `$ref` resolution, `$id` discipline, and cross-format consumer compatibility against existing OpenAPI and AsyncAPI bindings. |

Each skill's `SKILL.md` dispatches to format-specific `author.md`, `importer.md`, and `verifier.md` siblings — load only the sibling that matches the current intent.

### Mixed-format ordering

When a change touches more than one format (HTTP + events + shared schemas), the contracts brief invokes the skills in fixed order:

1. `/interfaces:json-schema` first — the schema vocabulary is shared and must stabilise before any binding references it.
2. `/interfaces:openapi` — HTTP operations bind to the schemas above via `$ref: "../schemas/<type>.yaml"`.
3. `/interfaces:asyncapi` — message channels bind to the same schemas via `$ref: "../schemas/<type>.yaml"`.

Running OpenAPI or AsyncAPI ahead of json-schema produces dangling `$ref`s and forces protocol authors to either inline definitions (forbidden in the baseline) or guess at shapes (forbidden by the no-invention rule).

### Cross-project compatibility check (RFC-9 §3B)

Each format skill's `verifier.md` accepts a `--mode cross-project` flag. The `/spec:execute` driver invokes the appropriate verifier after every successful merge of a contract listed in the producer project's `registry.yaml:contracts.produces`. The verifier compares the merged contract against each consumer's tier-2 workspace clone (`.specify/workspace/<consumer>/.specify/contracts/...`) and emits a structured YAML report of breaking changes (removed fields, newly-required fields, narrowed types, removed endpoints / channels). Findings are non-fatal — the execute driver records them as `cross-project-warning:` entries in the merged change's `journal.yaml` and renders a warning block in the merge transcript, but never halts the loop.

See each format skill's `verifier.md` (§Cross-project mode) and [`/spec:execute` → §Cross-project contract check](../spec/skills/execute/SKILL.md#cross-project-contract-check-rfc-9-3b) for the post-merge invocation contract. The shared `change-kind` vocabulary lives at [`references/cross-project-compatibility.md`](references/cross-project-compatibility.md).

## References

Format-specific conventions:

- [JSON Schema Conventions](references/json-schema-conventions.md) — `$id` URN format, metadata rules, type mapping, `$ref` conventions
- [OpenAPI Conventions](references/openapi-conventions.md) — OpenAPI 3.1 structure, path / method conventions, `$ref → ../schemas/`
- [AsyncAPI Conventions](references/asyncapi-conventions.md) — AsyncAPI 3.0 structure, channel / operation conventions

Cross-format references shared by every format skill:

- [Artifact Structure](references/artifact-structure.md) — `.specify/contracts/` layout, naming, change-level delta rules
- [Baseline vs Delta](references/baseline-vs-delta.md) — three authorship patterns (contract-first / spec-first / contract-given), already-covered / new-or-modified / normalisation classification, opaque-file-replacement merge contract
- [Import / Upgrade Policy](references/import-upgrade-policy.md) — format detection, per-format upgrade targets (Swagger 2.0 → OpenAPI 3.1; AsyncAPI 2.x → 3.0; JSON Schema Draft 4 / 6 / 7 / 2019-09 → 2020-12), lossless-vs-lossy decisions, when to refuse and ask the operator
- [Report Shape](references/report-shape.md) — single-mode markdown and cross-project YAML report formats, severity levels, locator format, exit semantics
- [Cross-Project Compatibility](references/cross-project-compatibility.md) — `change-kind` enumeration, consumer-view resolution, breaking-change classification policy

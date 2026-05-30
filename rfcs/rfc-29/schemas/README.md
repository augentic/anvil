# RFC-29 draft schemas

Normative JSON Schema drafts for [RFC-29](../rfc-29-fan-in-fan-out.md). Implementation copies these into `augentic/specify-cli/schemas/` and embeds them in `specify-schema`. Register `model.schema.json`, `synthesis-draft-model.schema.json`, and `synthesis-envelope.schema.json` together so relative `$ref`s compile without a registry lookup (same pattern as the adapter loader's inlined `$defs`).

| File | Lands at |
| --- | --- |
| `slice/model.schema.json` | `specify-cli/schemas/slice/model.schema.json` |
| `slice/synthesis-draft-model.schema.json` | `specify-cli/schemas/slice/synthesis-draft-model.schema.json` |
| `slice/synthesis-envelope.schema.json` | `specify-cli/schemas/slice/synthesis-envelope.schema.json` |
| `discovery/proposal.schema.json` | `specify-cli/schemas/discovery/proposal.schema.json` |
| `target/build-request.schema.json` | `specify-cli/schemas/target/build-request.schema.json` |
| `target/build-report.schema.json` | `specify-cli/schemas/target/build-report.schema.json` |

Embed constants: `SLICE_MODEL_JSON_SCHEMA`, `SYNTHESIS_DRAFT_MODEL_JSON_SCHEMA`, `SYNTHESIS_ENVELOPE_JSON_SCHEMA`, `PROPOSAL_JSON_SCHEMA`, `BUILD_REQUEST_JSON_SCHEMA`, `BUILD_REPORT_JSON_SCHEMA`.

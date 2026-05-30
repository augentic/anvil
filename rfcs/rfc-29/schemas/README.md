# RFC-29 draft schemas

Normative JSON Schema drafts for the [RFC-29](../rfc-29-fan-in-fan-out.md) family. These are the **shared wire contracts** pinned by the umbrella (see [RFC-29 §"Shared wire contracts"](../rfc-29-fan-in-fan-out.md#shared-wire-contracts)); each schema is owned by the sub-RFC that ships it: `discovery/proposal.schema.json` by [RFC-29b](../rfc-29b-lead-reconciliation.md) (M2a); `slice/model.schema.json`, `slice/draft-model.schema.json`, and `slice/synthesis.schema.json` by [RFC-29c](../rfc-29c-synthesis-typed-model.md) (M2b); the `target/build-*.schema.json` envelopes are authored during [RFC-29d](../rfc-29d-target-build-envelope.md) (M3). Implementation copies these into `augentic/specify-cli/schemas/` and embeds them in `specify-schema`. Register `model.schema.json`, `draft-model.schema.json`, and `synthesis.schema.json` together so relative `$ref`s compile without a registry lookup (same pattern as the adapter loader's inlined `$defs`).

| File | Lands at |
| --- | --- |
| `slice/model.schema.json` | `specify-cli/schemas/slice/model.schema.json` |
| `slice/draft-model.schema.json` | `specify-cli/schemas/slice/draft-model.schema.json` |
| `slice/synthesis.schema.json` | `specify-cli/schemas/slice/synthesis.schema.json` |
| `discovery/proposal.schema.json` | `specify-cli/schemas/discovery/proposal.schema.json` |
| `target/build-request.schema.json` | `specify-cli/schemas/target/build-request.schema.json` |
| `target/build-report.schema.json` | `specify-cli/schemas/target/build-report.schema.json` |

Embed constants: `SLICE_MODEL_JSON_SCHEMA`, `DRAFT_MODEL_JSON_SCHEMA`, `SYNTHESIS_JSON_SCHEMA`, `PROPOSAL_JSON_SCHEMA`, `BUILD_REQUEST_JSON_SCHEMA`, `BUILD_REPORT_JSON_SCHEMA`.

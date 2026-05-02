# Cross-Project Compatibility

Shared vocabulary used by every format-skill verifier (`/contract:openapi`, `/contract:asyncapi`, `/contract:json-schema`) when running in `cross-project` mode. The mode runs after a producer's contract change merges and compares the merged contract against each consumer's tier-2 workspace clone to surface breaking changes that would propagate downstream (RFC-9 §3B).

For the report's structural shape and exit semantics, see [`report-shape`](report-shape.md). This reference documents the **vocabulary** — the `change-kind` enumeration the verifiers use to classify deltas, the workspace clone resolution rules, and the breaking-change classification policy.

## When the mode runs

The `/spec:execute` driver invokes `cross-project` mode after every successful `specify change merge run` of a contract change. For each contract listed in the producer project's `registry.yaml:contracts.produces`, the driver iterates over each consumer (every other project in the registry) and invokes the appropriate format-skill verifier once per `(producer-contract, consumer-workspace)` pair.

The mode is **non-fatal**: cross-project warnings never stop the execute loop. The driver records each warning to the merging change's `journal.yaml` (via `specify change journal append`) and renders a warning block in the merge transcript so the operator can triage. Findings are surfaced as `cross-project-warning:` entries in the journal.

## Producer / consumer terminology

| Term | Meaning |
|---|---|
| **Producer** | The project that owns the contract — typically the API provider. Listed in `registry.yaml:contracts.produces`. |
| **Consumer** | A project that uses the contract — typically an API caller, event subscriber, or schema consumer. Every other project in the registry is a candidate consumer; the verifier runs against each one. |
| **`$PRODUCER_CONTRACT`** | A path to the producer's updated contract file (e.g. `contracts/http/user-api.yaml`) after the producer change merges. |
| **`$CONSUMER_WORKSPACE`** | A tier-2 workspace clone at `.specify/workspace/<consumer-name>/`. `specify workspace sync` materialises consumer clones from the platform's central `contracts/`. |
| **`$CONSUMER_CONTRACTS`** | The consumer's view of central contracts at `$CONSUMER_WORKSPACE/contracts/`. |

The verifier compares the producer's updated contract against the consumer's last-known view of the same contract. The consumer's view is whatever `specify workspace sync` populated — typically the central baseline from before the producer change merged.

## Consumer view resolution

For each `(producer-contract, consumer-workspace)` pair, resolve the consumer's view of the contract in this order:

1. **`$CONSUMER_CONTRACTS/<relative-path>`** — the consumer's materialised baseline at the matching path. This is what `specify workspace sync` populates from the central `contracts/`.
2. If absent, search **`$CONSUMER_CONTRACTS/imports/`** for a file with the same logical name (legacy import path used by some consumer clones).
3. If still absent, the consumer **has no prior view** — emit a single `consumer-has-no-baseline` finding (severity `info`) and stop. There is nothing to compare against.

The verifier does not walk the consumer's spec, source code, or generated bindings in this mode — that level of analysis is out of scope and would re-couple the verifier to the consumer's implementation. The conservative output is "the wire shape changed in a backwards-incompatible direction; the operator should triage."

## `change-kind` enumeration

Every cross-project finding carries a `change-kind` field naming the breaking-change category. The vocabulary is shared across formats — the format-skill verifiers each implement a subset relevant to their format.

### Universal change kinds

These apply to every format:

| `change-kind` | Severity | Applies to | Description |
|---|---|---|---|
| `removed-field` | `warning` | All formats | A property the consumer's view defined is no longer in the producer's contract. Consumer code reading the field will receive `undefined` after the next workspace sync. |
| `required-field-added` | `warning` | All formats | A new field is `required` in a payload (request body, message payload, or schema). Consumer payloads built from the prior shape will be rejected. |
| `type-narrowed` | `warning` | All formats | A property's `type` (or `format`, `enum`, `pattern`, numeric range) became stricter. Consumer values that were valid before may now be rejected. |
| `enum-value-removed` | `warning` | All formats | A value disappeared from a property's `enum` array. Consumers emitting that value will be rejected. |
| `additional-properties-tightened` | `warning` | JSON Schema (transitively, OpenAPI / AsyncAPI payloads) | The schema flipped from `additionalProperties: true` (or absent) to `additionalProperties: false`. Consumers passing extra fields will be rejected. |
| `consumer-has-no-baseline` | `info` | All formats | The consumer's workspace clone has no prior view (first-time materialisation). No incompatibility — the consumer picks up the new shape on its next workspace sync. |
| `format-mismatch` | `warning` | All formats | The consumer's file at the same path is **not** the same format as the producer's contract (e.g. consumer has AsyncAPI where the producer has OpenAPI). Emit and exit zero — likely the wrong verifier was invoked. |

### OpenAPI-specific change kinds

| `change-kind` | Severity | Description |
|---|---|---|
| `removed-endpoint` | `warning` | An OpenAPI path or operationId the consumer's view defined is gone. Consumer calls to it will fail. |
| `status-code-removed` | `warning` | A response status code defined in the consumer's view is missing from the producer's update. Consumer error-handling for that code is dead. |

### AsyncAPI-specific change kinds

| `change-kind` | Severity | Description |
|---|---|---|
| `removed-channel` | `warning` | An AsyncAPI channel the consumer's view defined is gone. Consumer subscribers will receive nothing. |
| `removed-operation` | `warning` | An AsyncAPI operation the consumer's view defined is gone. Consumer publishers / subscribers wiring will break. |

### Backwards-compatible (not warnings)

The following deltas are **not warnings** — they are backwards-compatible and consumers keep working unchanged. The verifier may emit `info`-severity entries when useful for triage but never `warning`:

- Additive optional fields on a request body, response body, or schema.
- New optional message channels or operations.
- New endpoints (paths or methods) not previously defined.
- Wider `enum` arrays (additive values).
- Loosened `additionalProperties` (from `false` to `true`).
- New examples, documentation, or descriptions.
- Format upgrades (e.g. consumer view uses Draft 7, producer uses Draft 2020-12) — draft difference alone is not a breaking change.

The verifier does not emit findings for these; they are filtered out at the diff stage.

## Workspace consumer detection

The `/spec:execute` driver detects which consumers to check by walking the registry. The verifier itself does not inspect `registry.yaml` — it receives the list of `(producer-contract, consumer-workspace)` pairs from the driver.

A project counts as a consumer of a producer's contract when:

- The project appears in `registry.yaml` as a peer of the producer.
- The project's `.specify/workspace/<producer>/` clone has been materialised by `specify workspace sync`.
- The consumer's `$CONSUMER_CONTRACTS` contains a file at the matching relative path (or the resolution fall-back paths above).

The driver invokes the verifier once per `(contract-file, consumer)` pair. A producer-side contract change touching three files surfaces as 3 × N verifier invocations across N consumers. Each invocation is independent — a `cannot-read-producer-contract` error in one pair does not affect the others.

## Breaking-change classification policy

The verifier's output is **conservative**: when a delta is ambiguous between "breaking" and "safe," emit a `warning` and let the operator triage. The classification table above is the canonical decision surface; the verifier does not invent new `change-kind` values.

Two policy rules govern ambiguous cases:

1. **Type widening is safe; type narrowing is breaking.** If `type: string` becomes `type: ["string", "null"]`, that is widening — no warning. If `type: ["string", "null"]` becomes `type: string`, that is narrowing — `type-narrowed`.
2. **Field additions are safe when optional, breaking when required.** A new optional field on a response body or message payload is additive. A new required field on a request body or message payload is `required-field-added`.

When the source delta does not fit any `change-kind`, the verifier preserves the input and continues. It does not invent classifications. The operator's triage path through the merge transcript is the safety net for any uncategorised drift.

## Out-of-scope analysis

`cross-project` mode deliberately does **not**:

- Walk the consumer's source code, generated bindings, or spec files. Coupling the verifier to consumer implementations would defeat the purpose of a format-level safety net.
- Run consumer-side compilation or test suites. The verifier reports wire-shape risk; consumer-side validation is the consumer's responsibility.
- Mutate the consumer's workspace clone. The clone is read-only from the verifier's perspective.
- Halt the execute loop. Cross-project warnings always exit 0; only unreadable inputs exit non-zero (see [`report-shape`](report-shape.md) §Cross-project exit semantics).

The execute driver records findings on the merging change's `journal.yaml` and renders a transcript warning block. The operator decides whether to amend the producer change, file follow-up changes for consumers, or accept the breaking change as documented.

## See also

- [`report-shape`](report-shape.md) — the structural shape of cross-project YAML reports, including `severity` levels and locator format.
- [`baseline-vs-delta`](baseline-vs-delta.md) — single-project counterpart for delta computation; the cross-format compatibility check (Check 4 in the JSON Schema verifier) uses the same `change-kind` vocabulary against in-project baseline bindings.
- [`artifact-structure`](artifact-structure.md) — workspace clone layout (`.specify/workspace/<peer>/`) and the materialisation contract.
- Format-specific verifiers — `plugins/contract/skills/{openapi,asyncapi,json-schema}/verifier.md` (see `--mode cross-project` sections).

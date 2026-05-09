# Cross-Project Compatibility

Shared vocabulary for classifying producer contract deltas against consumer views. RM-04 moves this behavior out of the retired format-skill `cross-project` verifier mode and into the CLI-owned surface:

```bash
specify compatibility check
specify compatibility report --change <name>
```

The current post-merge baseline gate remains separate: `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json` validates the merged contract set for SemVer, `info.x-specify-id` format, and cross-file id uniqueness. Compatibility classification consumes the registry and workspace state to answer a different question: whether a producer contract delta is `additive`, `breaking`, `ambiguous`, or `unverifiable` for registered consumers.

For the report's structural shape and exit semantics, see [`report-shape`](report-shape.md). For reviewer-facing rule prose that cites this vocabulary, see [`IFACE-005 Consumer Impact Classification`](../../../capabilities/contracts/codex/consumer-impact-classification.md).

## When the check runs

`specify compatibility report --change <name>` recomputes a read-only report from the current project state. It reads `registry.yaml`, root `contracts/`, and consumer workspace clones under `.specify/workspace/<consumer>/contracts/`. `specify compatibility check` emits the same report and exits non-zero when any finding is `breaking`, `ambiguous`, or `unverifiable`.

The command is advisory in RM-04. It does not transition plan entries, write journals, mutate workspace clones, or replace the existing `specify tool run contract` baseline validation gate. Dependency-aware lifecycle blocking is reserved for RM-11.

## Producer / consumer terminology

| Term | Meaning |
|---|---|
| **Producer** | The project that owns the contract — typically the API provider. Listed in `registry.yaml:contracts.produces`. |
| **Consumer** | A project that uses the contract — typically an API caller, event subscriber, or schema consumer. Listed in `registry.yaml:contracts.consumes`. |
| **`$PRODUCER_CONTRACT`** | A path to the producer's current contract file (e.g. `contracts/http/user-api.yaml`) in the root baseline. |
| **`$CONSUMER_WORKSPACE`** | A tier-2 workspace clone at `.specify/workspace/<consumer-name>/`. `specify workspace sync` materialises consumer clones from the platform's central `contracts/`. |
| **`$CONSUMER_CONTRACTS`** | The consumer's view of central contracts at `$CONSUMER_WORKSPACE/contracts/`. |

The classifier compares the producer's current contract against the consumer's last-known view of the same contract. The consumer's view is whatever `specify workspace sync` populated — typically the central baseline from before the producer change merged.

## Consumer view resolution

For each `(producer-contract, consumer-workspace)` pair, resolve the consumer's view of the contract in this order:

1. **`$CONSUMER_CONTRACTS/<relative-path>`** — the consumer's materialised baseline at the matching path. This is what `specify workspace sync` populates from the central `contracts/`.
2. If absent, the consumer has no comparable prior view — emit `classification: unverifiable` and stop. There is nothing safe to infer.

The classifier does not walk the consumer's spec, source code, generated bindings, or test suite. Coupling compatibility reporting to consumer implementation details would defeat the purpose of a format-level safety net.

## RM-04 classifications

Every finding carries a `classification` field:

| Classification | Meaning | `compatibility check` exit |
|---|---|---|
| `additive` | Backwards-compatible from the consumer's prior view. | Success when all findings are additive or clean. |
| `breaking` | Recognized backwards-incompatible wire change. | Validation failure. |
| `ambiguous` | The producer and consumer views differ, but the classifier cannot prove whether the delta is safe. | Validation failure. |
| `unverifiable` | Inputs are missing, malformed, invalid, or unsupported for comparison. | Validation failure. |

## `change-kind` enumeration

Breaking findings should carry a `change-kind` field naming the recognized category. Additive, ambiguous, and unverifiable findings may omit `change-kind` when no stable vocabulary value applies.

### Universal change kinds

These apply to every format:

| `change-kind` | Classification | Applies to | Description |
|---|---|---|---|
| `removed-field` | `breaking` | All formats | A property the consumer's view defined is no longer in the producer's contract. |
| `required-field-added` | `breaking` | All formats | A new field is `required` in a payload (request body, message payload, or schema). |
| `type-narrowed` | `breaking` | All formats | A property's `type` or constraint became stricter. |
| `enum-value-removed` | `breaking` | All formats | A value disappeared from a property's `enum` array. |
| `additional-properties-tightened` | `breaking` | JSON Schema (transitively, OpenAPI / AsyncAPI payloads) | The schema flipped from `additionalProperties: true` or absent to `additionalProperties: false`. |

### OpenAPI-specific change kinds

| `change-kind` | Classification | Description |
|---|---|---|
| `removed-endpoint` | `breaking` | An OpenAPI path or operation the consumer's view defined is gone. |
| `status-code-removed` | `breaking` | A response status code defined in the consumer's view is missing from the producer's update. |

### AsyncAPI-specific change kinds

| `change-kind` | Classification | Description |
|---|---|---|
| `removed-channel` | `breaking` | An AsyncAPI channel the consumer's view defined is gone. |
| `removed-operation` | `breaking` | An AsyncAPI operation the consumer's view defined is gone. |

### Additive deltas

The following deltas are backwards-compatible and should be classified as `additive` when reported:

- Additive optional fields on a request body, response body, or schema.
- New optional message channels or operations.
- New endpoints (paths or methods) not previously defined.
- Wider `enum` arrays (additive values).
- Loosened `additionalProperties` (from `false` to `true`).
- New examples, documentation, or descriptions.
- Format upgrades (e.g. consumer view uses Draft 7, producer uses Draft 2020-12) — draft difference alone is not a breaking change.

The report may include additive findings for operator visibility, but `specify compatibility check` still exits successfully when every finding is additive.

## Workspace consumer detection

The CLI detects which consumers to check by walking the registry.

A project counts as a consumer of a producer's contract when:

- The project appears in `registry.yaml` as a peer of the producer.
- The consumer's `.specify/workspace/<consumer>/` clone has been materialised by `specify workspace sync`.
- The consumer's `$CONSUMER_CONTRACTS` contains a file at the matching relative path.

The CLI compares each `(producer-contract, consumer)` pair independently. A producer-side contract change touching three files surfaces as 3 × N comparisons across N consumers.

## Breaking-change classification policy

The classifier is conservative: when a delta is changed but unsupported by the deterministic table, classify it as `ambiguous` rather than dropping it.

Two policy rules govern ambiguous cases:

1. **Type widening is safe; type narrowing is breaking.** If `type: string` becomes `type: ["string", "null"]`, that is widening — no warning. If `type: ["string", "null"]` becomes `type: string`, that is narrowing — `type-narrowed`.
2. **Field additions are safe when optional, breaking when required.** A new optional field on a response body or message payload is additive. A new required field on a request body or message payload is `required-field-added`.

When the source delta does not fit any `change-kind`, the classifier preserves the input context with `classification: ambiguous`. It does not invent new `change-kind` values.

## Out-of-scope analysis

Compatibility classification deliberately does **not**:

- Walk the consumer's source code, generated bindings, or spec files. Coupling the verifier to consumer implementations would defeat the purpose of a format-level safety net.
- Run consumer-side compilation or test suites. The verifier reports wire-shape risk; consumer-side validation is the consumer's responsibility.
- Mutate the consumer's workspace clone. The clone is read-only from the verifier's perspective.
- Transition plan entries, write journals, or halt `/change:execute`. RM-04 reports compatibility; RM-11 owns lifecycle gates.

The operator decides whether to amend the producer change, file follow-up changes for consumers, or accept the breaking change as documented.

## See also

- [`report-shape`](report-shape.md) — report shapes for verifier output and the CLI compatibility report.
- [`baseline-vs-delta`](baseline-vs-delta.md) — single-project counterpart for delta computation; the cross-format compatibility check (Check 4 in the JSON Schema verifier) uses the same `change-kind` vocabulary against in-project baseline bindings.
- [`artifact-structure`](artifact-structure.md) — workspace clone layout (`.specify/workspace/<peer>/`) and the materialisation contract.
- [`IFACE-005 Consumer Impact Classification`](../../../capabilities/contracts/codex/consumer-impact-classification.md) — stable codex rule for reviewer and compatibility findings.

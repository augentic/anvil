---
name: contract-asyncapi
description: Author, import, and verify AsyncAPI 3.0 event, pub/sub, stream, and WebSocket-style contracts for Specify changes, including channels, messages, bindings, producers, consumers, and schema references. Use when a contracts build needs an evented contract, when an operator supplies or asks for an AsyncAPI document, or when verifying AsyncAPI compatibility after a merge.
argument-hint: "[slice-dir]"
---

# AsyncAPI

Specialist for AsyncAPI 3.0 evented contracts on Specify changes — pub/sub, streaming, queue, and WebSocket-style messaging. This skill owns three intents: authoring or extending the AsyncAPI document for a slice, importing or normalising an externally supplied AsyncAPI document, and verifying an AsyncAPI artefact (single-mode internal consistency or merge-time baseline validation).

The skill is AsyncAPI-only. Shared payload schemas under `contracts/schemas/` are owned by the json-schema format skill (`/contract:json-schema`); HTTP contracts under `contracts/http/` are owned by `/contract:openapi`.

## Critical Path

1. **Read the briefs and specs.** Open the active contracts build brief and the slice's `specs/` to identify what evented interactions the slice requires; read `contracts/messages/` (the AsyncAPI baseline) to know which channels, operations, and messages already exist.
2. **Identify the intent.** Map the trigger to one of three sibling files using the [Intent dispatch](#intent-dispatch) table — author, importer, or verifier. Stop reading SKILL.md once the sibling is selected; load only the relevant sibling.
3. **Dispatch to the sibling.** Open and follow [`author.md`](./author.md), [`importer.md`](./importer.md), or [`verifier.md`](./verifier.md). Each sibling owns its complete algorithm, decision rules, and output format.
4. **Write outputs to `contracts/messages/`.** Author and importer paths produce or normalise AsyncAPI 3.0 YAML files under `$SLICE_DIR/contracts/messages/`. Decomposed payload schemas land under `$SLICE_DIR/contracts/schemas/` (json-schema-skill territory) — never inline them.
5. **Run the verifier.** After authoring or importing, invoke the verifier sibling against the slice directory to check `$ref` resolution, message metadata completeness, and binding coverage.
6. **Surface diagnostics.** Render the markdown alignment / import / validation report (single mode) or the contract-tool JSON envelope (cross-project mode) so the calling brief or operator can triage. Cross-project consumer impact is reported by `specify compatibility`.
7. **Stay within change-local `contracts/messages/`.** Do not modify baseline files in root `contracts/`, do not touch `contracts/http/` or shared schemas beyond writing decomposed `$ref` targets, and do not invent constructs that the spec does not justify — mark unknowns with `[unknown]` instead.

## Invocation

```text
/contract:asyncapi <slice-dir>
```

Optional internal positionals (recognised by the verifier sibling):

- `mode single` — default. Validate the slice's AsyncAPI artefacts in isolation against the specs and baseline. Read-only, markdown report.
- `mode cross-project` — merge-time baseline validation delegate. Walks the merged `contracts/` directory through `specify tool run contract`; it does not compare consumer workspace clones. Read-only, JSON envelope. See `verifier.md` §Cross-project mode.

When invoked from the contracts adapter build brief during `/spec:build`, `<slice-dir>` is the active slice directory; the brief routes the intent (author or importer) based on whether the operator supplied an external document for `contracts/messages/`. Consumer-impact reporting is a CLI concern under `specify compatibility`.

## Artifact layout

AsyncAPI files live in two locations — the slice-local delta and the platform baseline:

```text
contracts/
└── messages/
    └── <event-domain>-events.yaml       # Baseline: merged contracts only

.specify/slices/<slice-name>/
└── contracts/
    ├── messages/
    │   └── <event-domain>-events.yaml   # Slice-local delta or normalised import
    └── schemas/
        └── <type>.yaml                  # Owned by /contract:json-schema
```

Conventions enforced for every AsyncAPI file in either location:

- **AsyncAPI 3.0.0** — never 2.x. Importer upgrades 2.x inputs. See [`asyncapi-conventions`](../../references/asyncapi-conventions.md).
- **Kebab-case `.yaml` filename** — named after the event domain (`order-events.yaml`, `user-events.yaml`, `notification-events.yaml`). One file may carry many related channels for a single domain.
- **`$ref` to `../schemas/`** — every message payload points at a standalone JSON Schema file. Inline payload schemas are forbidden in the baseline; the importer decomposes inline payloads into `contracts/schemas/` before the file enters the baseline.
- **Opaque file replacement** — the slice-level `contracts/messages/<domain>-events.yaml` replaces the baseline file wholesale. When extending an existing event domain, the delta file must contain both the existing channels and operations and the new ones (the writer's algorithm reads the baseline and merges).

For the broader directory layout, see [`artifact-structure`](../../references/artifact-structure.md); for the cross-format minimal-delta rules and merge semantics, see [`baseline-vs-delta`](../../references/baseline-vs-delta.md).

## Intent dispatch

Pick the sibling that matches the trigger. Each sibling is a self-contained algorithm — load only the one selected.

| Intent | Trigger | Sibling |
|---|---|---|
| Author or extend the AsyncAPI document from a spec | contracts adapter build brief during `/spec:build`; operator extending the baseline for new evented interactions | `author.md` |
| Import or normalise an external AsyncAPI document | operator drops an AsyncAPI file into a slice's `contracts/messages/` directory | `importer.md` |
| Verify internal consistency or run merge-time baseline validation | contracts adapter build verification; post-merge contract baseline gate; operator invoking validation against an existing AsyncAPI artefact | `verifier.md` |

The three intents share a common artefact contract (channel addresses, message naming, `$ref` discipline) but have distinct algorithms — never conflate them. An import must be followed by a verifier run before the brief considers the artefact ready for merge; an author run normally ends with a verifier run too.

## Sibling references

- [`author.md`](./author.md) — spec → channels mapping, message and operation modelling, schema reuse, baseline-delta computation rules.
- [`importer.md`](./importer.md) — AsyncAPI version detection (2.x vs 3.0), upgrade rules to 3.0, inline-payload decomposition, Specify metadata injection.
- [`verifier.md`](./verifier.md) — `$ref` resolution, message metadata completeness, binding coverage, single-mode verifier behavior, and cross-project baseline validation delegation.

## Shared format guidance

AsyncAPI-specific conventions live in [`asyncapi-conventions`](../../references/asyncapi-conventions.md) — file structure, channel naming (camelCase keys + dot-notation addresses), operation `action` semantics (`send` vs `receive`), message definitions in `components/messages`, content-type defaults, header conventions, and the contract scope boundary (what stays in `design.md`, not in the contract). Read it before authoring or normalising channels; the sibling files link back to it where relevant.

For the cross-format directory layout, baseline-vs-delta rules, and merge semantics, see [`artifact-structure`](../../references/artifact-structure.md).

## Cross-format coordination

When a slice touches more than one contract format (HTTP + events + shared schemas), the `contracts` brief invokes the format skills in this order:

1. `/contract:json-schema` first — the schema vocabulary is shared and must stabilise before the bindings reference it.
2. `/contract:openapi` — HTTP operations bind to the schemas authored above.
3. `/contract:asyncapi` — message channels bind to the same schemas.

This skill never writes files outside `contracts/messages/` (and the slice-local schema deltas it decomposes into `contracts/schemas/` during import). HTTP-shaped intents are out of scope — route them to `/contract:openapi`.

## Hard rules

These constraints are non-negotiable for any of the three sibling paths:

1. **Valid AsyncAPI 3.0.** Every output file must parse as AsyncAPI 3.0.0. The importer is the only entry point that accepts older inputs.
2. **`$ref` discipline.** All payload schema references use relative file paths into `../schemas/`. Internal references (channel → message, operation → channel) use `#/components/...` and `#/channels/...` per the conventions. No inline payload schemas in the baseline.
3. **`$id` stability.** Once a schema has a `$id`, do not change it. New schemas get new `$id` values; the writer and importer never reassign existing ones.
4. **Kebab-case filenames.** All `.yaml` files use kebab-case names; no PascalCase or snake_case variants.
5. **Baseline immutability.** All output goes in the slice-local `contracts/` directory; baseline `contracts/` is read-only here. See [shared guardrails — Baseline immutability](../../../references/guardrails.md#baseline-immutability-for-contract-authoring).
6. **No invention.** When the spec does not provide enough detail to derive a channel or message shape, mark the gap with `[unknown]` in the alignment report rather than guessing. Importer flags unrecognised constructs with `[import — manual review required]`.
7. **Read-only verifier.** The verifier sibling must not create, modify, or delete any files in either mode.

## Output hygiene

- Only emit `.yaml` files under `$SLICE_DIR/contracts/`.
- Create `contracts/messages/`, `contracts/schemas/` only when they will contain at least one file.
- Stay inside `$SLICE_DIR/contracts/`; the baseline is off-limits per [shared guardrails](../../../references/guardrails.md#baseline-immutability-for-contract-authoring).

## See also

- [`asyncapi-conventions`](../../references/asyncapi-conventions.md) — AsyncAPI 3.0 structure, channel and operation conventions, message definitions, `$ref → ../schemas/`.
- [`artifact-structure`](../../references/artifact-structure.md) — directory layout for root `contracts/`.
- [`baseline-vs-delta`](../../references/baseline-vs-delta.md) — cross-format rules for computing the minimal delta between baseline and change-local files.
- [`import-upgrade-policy`](../../references/import-upgrade-policy.md) — shared framework for the importer sibling (format detection, upgrade targets, lossless vs lossy decisions).
- [`report-shape`](../../references/report-shape.md) — single-mode markdown, baseline validator JSON, and compatibility report JSON formats.
- [`cross-project-compatibility`](../../references/cross-project-compatibility.md) — RM-04 compatibility classifications and `change-kind` vocabulary used by `specify compatibility`.
- [`json-schema-conventions`](../../references/json-schema-conventions.md) — payload schema rules (owned by `/contract:json-schema`; linked here so authors of AsyncAPI files understand the schema files they reference).

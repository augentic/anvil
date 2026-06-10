# contracts.build — asyncapi sub-flow

Loaded by [../build.md](../build.md) Phase 2 when the slice carries evented, pub/sub, streaming, queue, or WebSocket-style interactions. Specialist for AsyncAPI 3.0 evented contracts.

This sub-flow is AsyncAPI-only. Shared payload schemas under `contracts/schemas/` are owned by the [json-schema sub-flow](json-schema.md); HTTP contracts under `contracts/http/` are owned by the [openapi sub-flow](openapi.md).

## Critical path

1. **Read the briefs and specs.** Open [../build.md](../build.md) and the slice's `specs/<domain>/spec.md` files to identify what evented interactions the slice requires; read `contracts/messages/` (the AsyncAPI baseline) to know which channels, operations, and messages already exist.
2. **Identify the intent.** Map the trigger to one of three sibling references using the [intent dispatch](#intent-dispatch) table — author, importer, or verifier. Load only the relevant sibling.
3. **Dispatch to the sibling.** Open and follow [`../../references/asyncapi/author.md`](../../references/asyncapi/author.md), [`../../references/asyncapi/importer.md`](../../references/asyncapi/importer.md), or [`../../references/asyncapi/verifier.md`](../../references/asyncapi/verifier.md). Each sibling owns its complete algorithm, decision rules, and output format.
4. **Write outputs to `contracts/messages/`.** Author and importer paths produce or normalise AsyncAPI 3.0 YAML files under `$SLICE_DIR/contracts/messages/`. Decomposed payload schemas land under `$SLICE_DIR/contracts/schemas/` (json-schema-sub-flow territory) — never inline them.
5. **Run the verifier.** After authoring or importing, invoke the verifier sibling against the slice directory to check `$ref` resolution, message metadata completeness, and binding coverage.
6. **Surface diagnostics.** Render the markdown alignment / import / validation report (single mode) or the contract-tool JSON envelope (cross-project mode). Cross-project consumer impact is deferred until a real consumer workflow exists.
7. **Stay within change-local `contracts/messages/`.** Do not modify baseline files in root `contracts/`, do not touch `contracts/http/` or shared schemas beyond writing decomposed `$ref` targets, and do not invent constructs that the spec does not justify — mark unknowns with `[unknown]` instead.

## Artifact layout

```text
contracts/
└── messages/
    └── <event-domain>-events.yaml       # Baseline: merged contracts only

.specify/slices/<slice-name>/
└── contracts/
    ├── messages/
    │   └── <event-domain>-events.yaml   # Slice-local delta or normalised import
    └── schemas/
        └── <type>.yaml                  # Owned by the json-schema sub-flow
```

Conventions enforced for every AsyncAPI file in either location:

- **AsyncAPI 3.0.0** — never 2.x. The importer upgrades 2.x inputs. See [`../../references/asyncapi-conventions.md`](../../references/asyncapi-conventions.md).
- **Kebab-case `.yaml` filename** — named after the event domain (`order-events.yaml`, `user-events.yaml`, `notification-events.yaml`). One file may carry many related channels for a single domain.
- **`$ref` to `../schemas/`** — every message payload points at a standalone JSON Schema file. Inline payload schemas are forbidden in the baseline; the importer decomposes inline payloads into `contracts/schemas/` before the file enters the baseline.
- **Opaque file replacement** — the slice-level `contracts/messages/<domain>-events.yaml` replaces the baseline file wholesale. When extending an existing event domain, the delta file must contain both the existing channels and operations and the new ones.

## Intent dispatch

| Intent | Trigger | Sibling |
|---|---|---|
| Author or extend the AsyncAPI document from a spec | build brief during `/spec:build`; operator extending the baseline for new evented interactions | [`../../references/asyncapi/author.md`](../../references/asyncapi/author.md) |
| Import or normalise an external AsyncAPI document | operator drops an AsyncAPI file into a slice's `contracts/messages/` directory | [`../../references/asyncapi/importer.md`](../../references/asyncapi/importer.md) |
| Verify internal consistency or run merge-time baseline validation | build verification; post-merge contract baseline gate; operator invoking validation against an existing AsyncAPI artefact | [`../../references/asyncapi/verifier.md`](../../references/asyncapi/verifier.md) |

The three intents share a common artefact contract (channel addresses, message naming, `$ref` discipline) but have distinct algorithms — never conflate them.

## Hard rules

1. **Valid AsyncAPI 3.0.** Every output file must parse as AsyncAPI 3.0.0. The importer is the only entry point that accepts older inputs.
2. **`$ref` discipline.** All payload schema references use relative file paths into `../schemas/`. Internal references (channel → message, operation → channel) use `#/components/...` and `#/channels/...` per the conventions. No inline payload schemas in the baseline.
3. **`$id` stability.** Once a schema has a `$id`, do not change it. New schemas get new `$id` values; the writer and importer never reassign existing ones.
4. **Kebab-case filenames.** All `.yaml` files use kebab-case names; no PascalCase or snake_case variants.
5. **Baseline immutability.** All output goes in the slice-local `contracts/` directory; baseline `contracts/` is read-only here.
6. **No invention.** When the spec does not provide enough detail to derive a channel or message shape, mark the gap with `[unknown]`. The importer flags unrecognised constructs with `[import — manual review required]`.
7. **Read-only verifier.** The verifier sibling must not create, modify, or delete any files in either mode.

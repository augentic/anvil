# contracts.build — json-schema sub-flow

Loaded by [../build.md](../build.md) Phase 2 **first** (before openapi or asyncapi) when the slice has any shared payload vocabulary. Specialist for standalone JSON Schema (Draft 2020-12) documents — the shared payload vocabulary referenced by the openapi sub-flow's HTTP operations and the asyncapi sub-flow's message channels.

This sub-flow is JSON-Schema-only. Protocol bindings under `contracts/http/` belong to the [openapi sub-flow](openapi.md); evented bindings under `contracts/messages/` belong to the [asyncapi sub-flow](asyncapi.md). Both protocol sub-flows delegate every payload-schema decision (`$id` shape, naming, decomposition, draft policy, metadata) to this sub-flow — which is why it runs **first** in Phase 2.

## Critical path

1. **Read the briefs and specs.** Open [../build.md](../build.md) and the slice's `specs/<domain>/spec.md` files to identify which payload types the slice requires; read `contracts/schemas/` (the schema baseline) to know what shared vocabulary already exists.
2. **Identify the intent.** Map the trigger to one of three sibling references using the [intent dispatch](#intent-dispatch) table — author, importer, or verifier. Load only the relevant sibling.
3. **Dispatch to the sibling.** Open and follow [`../../references/json-schema/author.md`](../../references/json-schema/author.md), [`../../references/json-schema/importer.md`](../../references/json-schema/importer.md), or [`../../references/json-schema/verifier.md`](../../references/json-schema/verifier.md). Each sibling owns its complete algorithm, decision rules, and output format.
4. **Write outputs to `contracts/schemas/`.** Author and importer paths produce or normalise JSON Schema YAML files under `$SLICE_DIR/contracts/schemas/` — one named type per file, kebab-case filenames, URN `$id` derived from the file path.
5. **Run the verifier.** After authoring or importing, invoke the verifier sibling to check `$ref` resolution, metadata completeness, duplicate-`$id` collisions, and cross-format consumer compatibility against any HTTP and messaging bindings that already reference the schema.
6. **Surface diagnostics.** Render the markdown alignment / import / validation report (single mode) or the contract-tool JSON envelope (cross-project mode). Cross-project consumer impact is deferred until a real consumer workflow exists.
7. **Stay within change-local `contracts/schemas/`.** Do not modify baseline files in root `contracts/`, do not touch `contracts/http/` or `contracts/messages/`, and do not invent fields the spec does not justify — mark unknowns with `[unknown]`.

## Artifact layout

```text
contracts/
└── schemas/
    └── <type>.yaml                 # Baseline: merged schemas only

.specify/slices/<slice-name>/
└── contracts/
    └── schemas/
        └── <type>.yaml             # Slice-local delta or normalised import
```

Conventions enforced for every schema file in either location:

- **JSON Schema Draft 2020-12** — never older drafts. The importer upgrades draft-04, draft-06, draft-07, and draft 2019-09 inputs. See [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md).
- **One type per file** — each `.yaml` defines exactly one top-level named type. Shared sub-types extracted to their own files; file-local sub-types may live under `$defs`.
- **Kebab-case `.yaml` filename** — the filename is the kebab-case form of the PascalCase type name (`UserRegistration` → `user-registration.yaml`). The filename is canonical: `$id` and `title` derive from it.
- **URN `$id`** — every schema declares `$id: "urn:specify:schemas/<filename-without-extension>"`. `$id` is stable for the schema's lifetime; renaming requires a new file with a new `$id` and explicit deprecation of the old one.
- **Opaque file replacement** — the slice-level `contracts/schemas/<type>.yaml` replaces the baseline file wholesale at merge time. Schema deltas are by file, not by property.

## Intent dispatch

| Intent | Trigger | Sibling |
|---|---|---|
| Author or extend reusable schemas from a spec | build brief during `/spec:build`; operator extending the baseline for new payload types | [`../../references/json-schema/author.md`](../../references/json-schema/author.md) |
| Import or normalise external schema files | operator drops schema files into a slice's `contracts/schemas/` directory | [`../../references/json-schema/importer.md`](../../references/json-schema/importer.md) |
| Verify `$ref` consistency, metadata, cross-format consumer compatibility, or merge-time baseline validation | build verification; post-merge contract baseline gate | [`../../references/json-schema/verifier.md`](../../references/json-schema/verifier.md) |

The three intents share a common artefact contract (filename → `$id` derivation, one-type-per-file, draft policy) but have distinct algorithms — never conflate them.

## Hard rules

1. **Valid JSON Schema Draft 2020-12.** Every output file must parse against `https://json-schema.org/draft/2020-12/schema`. The importer is the only entry point that accepts older drafts.
2. **One type per file.** Each `.yaml` file under `contracts/schemas/` defines exactly one top-level named type. Shared sub-types are separate files; file-local sub-types may use `$defs`.
3. **`$id` stability.** Once a `$id` is assigned, it never changes. New schemas get new `$id` values from the file path; the writer and importer never reassign existing ones, even when a baseline schema's `$id` is malformed (surface the issue as a normalisation finding instead).
4. **Filename ↔ `$id` ↔ `title` coherence.** The filename (kebab-case), the `$id` URN segment (kebab-case suffix), and the `title` (PascalCase) all describe the same type. Drift between them is a verifier failure.
5. **Kebab-case filenames.** All `.yaml` files use kebab-case names; no PascalCase or snake_case variants.
6. **No invention.** When the spec does not provide enough detail to derive a shape, mark the gap with `[unknown]`. The importer flags unrecognised constructs with `[import — manual review required]`.
7. **No protocol-specific authoring.** This sub-flow never writes path operations, channels, operations, request bodies, or response wrappers. Those belong to the openapi and asyncapi sub-flows.
8. **Read-only verifier.** The verifier sibling must not create, modify, or delete any files in either mode.
9. **Baseline immutability.** All output goes in the slice-local `contracts/` directory; baseline `contracts/` is read-only here.

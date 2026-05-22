# contracts.build

Orchestrates `/spec:build` for slices that target the `contracts` adapter. The brief authors and validates machine-readable contract artifacts under the slice-local `contracts/` directory. Three format sub-flows (`openapi`, `asyncapi`, `json-schema`) live inline below; each carries an internal author / import / verify intent table — they are dispatched **inside** this brief, not separate target adapters.

## Scope

Build writes only change-local contract deltas under `.specify/slices/<slice>/contracts/`:

- `contracts/schemas/*.yaml` — reusable JSON Schema payload vocabulary (one named type per file).
- `contracts/http/*.yaml` — OpenAPI 3.1 HTTP / resource-style documents.
- `contracts/messages/*.yaml` — AsyncAPI 3.0 evented / pub-sub / streaming / WebSocket documents.

Build MUST NOT edit the root `contracts/` baseline directly. Baseline updates happen only during `merge` (see [`merge.md`](merge.md)).

## Inputs

Read:

- `proposal.md` — authorship mode (author vs import), source material, interface scope, producer/consumer roles.
- `spec.md` — behavioural requirements: endpoints / channels / payloads / errors. Provenance lines tell the brief whether the slice is author-driven (`Sources: [intent | <doc-key>]`) or import-driven (`Sources: [<code-or-contract-source>]`).
- `design.md` — the format selection (OpenAPI 3.1 / AsyncAPI 3.0 / JSON Schema), file-layout intent, and any cross-contract dependency notes (see [`shape.md`](shape.md)).
- The slice's `contracts/` subtree (if present) — partial deltas written by a prior pass.
- The root `contracts/` baseline — read-only context for `$ref` reuse and extension authoring.
- `tasks.md` — progress tracking.

Build consumes the synthesised Specify artifacts as its primary source. Do not treat raw design documentation as the contract source unless the proposal names it as Source Material and the synthesised `spec.md` has captured the required behaviour.

## Algorithm

### Phase 1 — Classify

Identify the authorship mode from `proposal.md`:

1. **Author from specs** — synthesise contract artifacts from `spec.md` requirements (typical for `intent` / `documentation` sources).
2. **Import existing contracts** — normalise supplied OpenAPI / AsyncAPI / JSON Schema files into Specify conventions.
3. **Modify existing contracts** — author a minimal delta against the root `contracts/` baseline driven by the behavioural delta in `spec.md`.
4. **Extract from source code** — author contract artifacts from `spec.md` requirements derived from observed code behaviour. Functionally identical to "Author from specs" at build time; the distinct mode is preserved for traceability and to surface the elevated `[unknown]` rate to the verifier.
5. **Mixed** — combine author and import paths when the proposal explicitly includes both prose-derived requirements and supplied contract files.

Then classify required formats from `design.md`:

- **JSON Schema** — reusable payload vocabulary referenced by HTTP and/or evented interactions, or standalone schema imports.
- **OpenAPI 3.1** — HTTP / resource interactions or supplied OpenAPI / Swagger artifacts.
- **AsyncAPI 3.0** — evented, pub/sub, streaming, WebSocket-style interactions, or supplied AsyncAPI artifacts.

### Phase 2 — Author or import (fixed format order)

When a slice touches more than one contract format, run the format sub-flows in this fixed order — the schema vocabulary is shared and must stabilise before the bindings reference it:

1. **`json-schema`** — author or import the minimal JSON Schema delta for reusable payload vocabulary. Owns `$id` assignment, one-type-per-file decomposition, and schema-file naming. Skip when the slice has no shared payload schemas. See [`§json-schema sub-flow`](#json-schema-sub-flow).
2. **`openapi`** — author or import the minimal OpenAPI delta for HTTP / resource interactions. Reuse change-local or baseline `contracts/schemas/` files; do not author competing schemas under different filenames or `$id`s. Skip when the slice has no HTTP interactions. See [`§openapi sub-flow`](#openapi-sub-flow).
3. **`asyncapi`** — author or import the minimal AsyncAPI delta for evented / pub-sub / streaming / WebSocket-style interactions. Follow the same schema-reuse rule. Skip when the slice has no evented interactions. See [`§asyncapi sub-flow`](#asyncapi-sub-flow).

Import paths must produce an import report covering lossless changes, lossy changes, unsupported constructs, and manual-review warnings. See [`references/import-upgrade-policy.md`](../references/import-upgrade-policy.md).

**Identity & version.** Every top-level OpenAPI / AsyncAPI document emitted into `$SLICE_DIR/contracts/` (root key `openapi:` or `asyncapi:`) MUST set an `info.version` value that parses as SemVer per [semver.org](https://semver.org), including optional prerelease labels. New top-level contracts SHOULD set `info.x-specify-id` to a kebab-case slug (typically the file stem; `^[a-z][a-z0-9-]*$`, ≤ 64 characters) — a rename-stable hint that survives file moves and version bumps. The author sub-flows enforce both rules; the import sub-flows preserve any source `info.x-specify-id` verbatim and surface non-SemVer `info.version` values as `[manual review required]` rather than auto-rewriting.

### Phase 3 — Verify

Verification runs the verifier intent of each format sub-flow that owns artifacts in the slice. Run only the formats that produced artifacts; skip the rest. The verifier siblings live under [`references/<format>/verifier.md`](../references/).

1. **`json-schema`** verifier — `$ref` resolution, schema metadata (`$id`, `title`, `description`) for every JSON Schema file under `contracts/schemas/`.
2. **`openapi`** verifier — `$ref` resolution across the OpenAPI delta and binding completeness (every spec-referenced HTTP schema has at least one binding).
3. **`asyncapi`** verifier — `$ref` resolution across the AsyncAPI delta and binding completeness (every spec-referenced evented schema has at least one binding).

For mixed-format slices, the final verifier pass must check cross-format `$ref` consistency and report duplicate schema identities before build can complete.

The format verifiers enforce the identity & version rules inline (SemVer `info.version`; kebab-case + ≤64-char `info.x-specify-id` when present; in-slice uniqueness on declared ids). The **cross-repo** uniqueness check — the same id declared by another top-level contract somewhere else in the root `contracts/` baseline — is **not** part of build-time verification; it is the merge gate's job, run by the declared `contract` WASI tool against the merged baseline (see [`merge.md`](merge.md)).

#### Single-mode verifier invocation

Run each format's verifier in `mode: single` against the slice directory. The verifier reads slice-local artefacts plus the baseline for binding-coverage cross-references and emits a markdown alignment report covering `$ref` resolution, metadata completeness, and binding coverage. The verifier siblings are read-only — they MUST NOT create, modify, or delete any files.

### Phase 4 — Verify-repair loop (max 2 iterations)

If a verifier reports failures:

1. Re-enter the same format sub-flow with the verifier output for targeted repair via the same intent that produced the artifact (author or import).
2. Re-run that format's verifier.
3. If still failing after 2 iterations, stop and surface issues for human review. Do not mark the task complete. Report the remaining failures with full output and escalate for guidance.

A clean verification pass with zero issues is the expected outcome.

### Phase 5 — Tool gate

Build's final step invokes the declared `contract` WASI tool to confirm the slice's contract files parse and pass the RFC-12 §Validation rules in single-mode against the slice's delta:

```bash
specify tool run contract -- "$SLICE_DIR/contracts" --format json > /tmp/contract-build.json
case $? in
  0) ;;  # clean — slice deltas are well-formed; proceed to task completion
  1) ;;  # findings — re-enter the failing format sub-flow per Phase 4
  2) ;;  # tool/validator could not run — escalate; do not mark the task complete
esac
```

The tool's `--format json` output shape is documented under [`references/report-shape.md`](../references/report-shape.md).

### No-op behaviour

When the slice's specs describe no API interactions and no Source Material lists importable contract artifacts, every format pass produces an empty delta and the verifiers have nothing to check. The brief completes as a no-op. This is normal for slices that touch only planning metadata or contract documentation without affecting an API surface.

## Format sub-flows

The three sub-flows below carry the same author / import / verify intent dispatch. Pick the sub-flow that matches the slice's format; load only the sibling intent files (`references/<format>/{author,importer,verifier}.md`) the selected sub-flow needs.

### openapi sub-flow

Specialist for OpenAPI 3.1 HTTP API contracts. Three intents — authoring or extending the OpenAPI document for a slice, importing or normalising an externally supplied OpenAPI document, and verifying an OpenAPI artefact (single-mode internal consistency or merge-time baseline validation).

This sub-flow is OpenAPI-only. Shared payload schemas under `contracts/schemas/` are owned by the [`§json-schema sub-flow`](#json-schema-sub-flow); evented contracts under `contracts/messages/` are owned by the [`§asyncapi sub-flow`](#asyncapi-sub-flow).

#### openapi — critical path

1. **Read the briefs and specs.** Open this build brief and the slice's `spec.md` to identify what the slice requires; read `contracts/http/` (the HTTP baseline) to know what already exists.
2. **Identify the intent.** Map the trigger to one of three sibling files using the [openapi intent dispatch](#openapi--intent-dispatch) table — author, importer, or verifier. Stop reading SKILL prose once the sibling is selected; load only the relevant sibling.
3. **Dispatch to the sibling.** Open and follow [`references/openapi/author.md`](../references/openapi/author.md), [`references/openapi/importer.md`](../references/openapi/importer.md), or [`references/openapi/verifier.md`](../references/openapi/verifier.md). Each sibling owns its complete algorithm, decision rules, and output format.
4. **Write outputs to `contracts/http/`.** Author and importer paths produce or normalise OpenAPI 3.1 YAML files under `$SLICE_DIR/contracts/http/`. Decomposed payload schemas land under `$SLICE_DIR/contracts/schemas/` (json-schema-sub-flow territory) — never inline them.
5. **Run the verifier.** After authoring or importing, invoke the verifier sibling against the slice directory to check `$ref` resolution, schema metadata completeness, and binding coverage.
6. **Surface diagnostics.** Render the markdown alignment / import / validation report (single mode) or the contract-tool JSON envelope (cross-project mode). Cross-project consumer impact is reported by `specify compatibility`.
7. **Stay within change-local `contracts/http/`.** Do not modify baseline files in root `contracts/`, do not touch `contracts/messages/` or shared schemas beyond writing decomposed `$ref` targets, and do not invent constructs that the spec does not justify — mark unknowns with `[unknown]` instead.

#### openapi — artifact layout

OpenAPI files live in two locations — the slice-local delta and the platform baseline:

```text
contracts/
└── http/
    └── <api-domain>.yaml              # Baseline: merged contracts only

.specify/slices/<slice-name>/
└── contracts/
    ├── http/
    │   └── <api-domain>.yaml          # Slice-local delta or normalised import
    └── schemas/
        └── <type>.yaml                # Owned by the json-schema sub-flow
```

Conventions enforced for every OpenAPI file in either location:

- **OpenAPI 3.1.0** — never 3.0.x. The importer upgrades 3.0 and Swagger 2.0 inputs. See [`references/openapi-conventions.md`](../references/openapi-conventions.md).
- **Kebab-case `.yaml` filename** — named after the API domain (`user-api.yaml`, `billing-api.yaml`). One file may carry many related operations.
- **`$ref` to `../schemas/`** — every request body, response body, and parameter schema points at a standalone JSON Schema file. Inline schemas are forbidden in the baseline; the importer decomposes inline schemas into `contracts/schemas/` before the file enters the baseline.
- **Opaque file replacement** — the slice-level `contracts/http/<domain>.yaml` replaces the baseline file wholesale. When extending an existing API domain, the delta file must contain both the existing operations and the new ones (the writer's algorithm reads the baseline and merges).

For the broader directory layout, see [`references/artifact-structure.md`](../references/artifact-structure.md); for the cross-format minimal-delta rules and merge semantics, see [`references/baseline-vs-delta.md`](../references/baseline-vs-delta.md).

#### openapi — intent dispatch

Pick the sibling that matches the trigger. Each sibling is a self-contained algorithm — load only the one selected.

| Intent | Trigger | Sibling |
|---|---|---|
| Author or extend the OpenAPI document from a spec | build brief during `/spec:build`; operator extending the baseline for new HTTP interactions | `references/openapi/author.md` |
| Import or normalise an external OpenAPI document | operator drops an OpenAPI file into a slice's `contracts/http/` directory | `references/openapi/importer.md` |
| Verify internal consistency or run merge-time baseline validation | build verification; post-merge contract baseline gate; operator invoking validation against an existing OpenAPI artefact | `references/openapi/verifier.md` |

The three intents share a common artefact contract (paths, file naming, `$ref` discipline) but have distinct algorithms — never conflate them. An import must be followed by a verifier run before the brief considers the artefact ready for merge; an author run normally ends with a verifier run too.

#### openapi — hard rules

These constraints are non-negotiable for any of the three sibling paths:

1. **Valid OpenAPI 3.1.** Every output file must parse as OpenAPI 3.1.0. The importer is the only entry point that accepts older inputs.
2. **`$ref` discipline.** All schema references use relative file paths into `../schemas/`. No `#/components/schemas/...` pointers in the baseline. No inline domain types.
3. **`$id` stability.** Once a schema has a `$id`, do not change it. New schemas get new `$id` values; the writer and importer never reassign existing ones.
4. **Kebab-case filenames.** All `.yaml` files use kebab-case names; no PascalCase or snake_case variants.
5. **Baseline immutability.** All output goes in the slice-local `contracts/` directory; baseline `contracts/` is read-only here.
6. **No invention.** When the spec does not provide enough detail to derive a shape, mark the gap with `[unknown]` in the alignment report rather than guessing. The importer flags unrecognised constructs with `[import — manual review required]`.
7. **Read-only verifier.** The verifier sibling must not create, modify, or delete any files in either mode.

### asyncapi sub-flow

Specialist for AsyncAPI 3.0 evented contracts — pub/sub, streaming, queue, and WebSocket-style messaging. Three intents — authoring or extending the AsyncAPI document for a slice, importing or normalising an externally supplied AsyncAPI document, and verifying an AsyncAPI artefact.

This sub-flow is AsyncAPI-only. Shared payload schemas under `contracts/schemas/` are owned by the [`§json-schema sub-flow`](#json-schema-sub-flow); HTTP contracts under `contracts/http/` are owned by the [`§openapi sub-flow`](#openapi-sub-flow).

#### asyncapi — critical path

1. **Read the briefs and specs.** Open this build brief and the slice's `spec.md` to identify what evented interactions the slice requires; read `contracts/messages/` (the AsyncAPI baseline) to know which channels, operations, and messages already exist.
2. **Identify the intent.** Map the trigger to one of three sibling files using the [asyncapi intent dispatch](#asyncapi--intent-dispatch) table — author, importer, or verifier. Load only the relevant sibling.
3. **Dispatch to the sibling.** Open and follow [`references/asyncapi/author.md`](../references/asyncapi/author.md), [`references/asyncapi/importer.md`](../references/asyncapi/importer.md), or [`references/asyncapi/verifier.md`](../references/asyncapi/verifier.md). Each sibling owns its complete algorithm, decision rules, and output format.
4. **Write outputs to `contracts/messages/`.** Author and importer paths produce or normalise AsyncAPI 3.0 YAML files under `$SLICE_DIR/contracts/messages/`. Decomposed payload schemas land under `$SLICE_DIR/contracts/schemas/` (json-schema-sub-flow territory) — never inline them.
5. **Run the verifier.** After authoring or importing, invoke the verifier sibling against the slice directory to check `$ref` resolution, message metadata completeness, and binding coverage.
6. **Surface diagnostics.** Render the markdown alignment / import / validation report (single mode) or the contract-tool JSON envelope (cross-project mode). Cross-project consumer impact is reported by `specify compatibility`.
7. **Stay within change-local `contracts/messages/`.** Do not modify baseline files in root `contracts/`, do not touch `contracts/http/` or shared schemas beyond writing decomposed `$ref` targets, and do not invent constructs that the spec does not justify — mark unknowns with `[unknown]` instead.

#### asyncapi — artifact layout

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

- **AsyncAPI 3.0.0** — never 2.x. The importer upgrades 2.x inputs. See [`references/asyncapi-conventions.md`](../references/asyncapi-conventions.md).
- **Kebab-case `.yaml` filename** — named after the event domain (`order-events.yaml`, `user-events.yaml`, `notification-events.yaml`). One file may carry many related channels for a single domain.
- **`$ref` to `../schemas/`** — every message payload points at a standalone JSON Schema file. Inline payload schemas are forbidden in the baseline; the importer decomposes inline payloads into `contracts/schemas/` before the file enters the baseline.
- **Opaque file replacement** — the slice-level `contracts/messages/<domain>-events.yaml` replaces the baseline file wholesale. When extending an existing event domain, the delta file must contain both the existing channels and operations and the new ones.

#### asyncapi — intent dispatch

| Intent | Trigger | Sibling |
|---|---|---|
| Author or extend the AsyncAPI document from a spec | build brief during `/spec:build`; operator extending the baseline for new evented interactions | `references/asyncapi/author.md` |
| Import or normalise an external AsyncAPI document | operator drops an AsyncAPI file into a slice's `contracts/messages/` directory | `references/asyncapi/importer.md` |
| Verify internal consistency or run merge-time baseline validation | build verification; post-merge contract baseline gate; operator invoking validation against an existing AsyncAPI artefact | `references/asyncapi/verifier.md` |

The three intents share a common artefact contract (channel addresses, message naming, `$ref` discipline) but have distinct algorithms — never conflate them.

#### asyncapi — hard rules

1. **Valid AsyncAPI 3.0.** Every output file must parse as AsyncAPI 3.0.0. The importer is the only entry point that accepts older inputs.
2. **`$ref` discipline.** All payload schema references use relative file paths into `../schemas/`. Internal references (channel → message, operation → channel) use `#/components/...` and `#/channels/...` per the conventions. No inline payload schemas in the baseline.
3. **`$id` stability.** Once a schema has a `$id`, do not change it. New schemas get new `$id` values; the writer and importer never reassign existing ones.
4. **Kebab-case filenames.** All `.yaml` files use kebab-case names; no PascalCase or snake_case variants.
5. **Baseline immutability.** All output goes in the slice-local `contracts/` directory; baseline `contracts/` is read-only here.
6. **No invention.** When the spec does not provide enough detail to derive a channel or message shape, mark the gap with `[unknown]`. The importer flags unrecognised constructs with `[import — manual review required]`.
7. **Read-only verifier.** The verifier sibling must not create, modify, or delete any files in either mode.

### json-schema sub-flow

Specialist for standalone JSON Schema (Draft 2020-12) documents — the shared payload vocabulary referenced by the openapi sub-flow's HTTP operations and the asyncapi sub-flow's message channels. Three intents — authoring or extending reusable payload schemas, importing or normalising externally supplied schema files, and verifying schema artefacts.

This sub-flow is JSON-Schema-only. Protocol bindings under `contracts/http/` belong to the [`§openapi sub-flow`](#openapi-sub-flow); evented bindings under `contracts/messages/` belong to the [`§asyncapi sub-flow`](#asyncapi-sub-flow). Both protocol sub-flows delegate every payload-schema decision (`$id` shape, naming, decomposition, draft policy, metadata) to this sub-flow — which is why it runs **first** in Phase 2.

#### json-schema — critical path

1. **Read the briefs and specs.** Open this build brief and the slice's `spec.md` to identify which payload types the slice requires; read `contracts/schemas/` (the schema baseline) to know what shared vocabulary already exists.
2. **Identify the intent.** Map the trigger to one of three sibling files using the [json-schema intent dispatch](#json-schema--intent-dispatch) table — author, importer, or verifier. Load only the relevant sibling.
3. **Dispatch to the sibling.** Open and follow [`references/json-schema/author.md`](../references/json-schema/author.md), [`references/json-schema/importer.md`](../references/json-schema/importer.md), or [`references/json-schema/verifier.md`](../references/json-schema/verifier.md). Each sibling owns its complete algorithm, decision rules, and output format.
4. **Write outputs to `contracts/schemas/`.** Author and importer paths produce or normalise JSON Schema YAML files under `$SLICE_DIR/contracts/schemas/` — one named type per file, kebab-case filenames, URN `$id` derived from the file path.
5. **Run the verifier.** After authoring or importing, invoke the verifier sibling to check `$ref` resolution, metadata completeness, duplicate-`$id` collisions, and cross-format consumer compatibility against any HTTP and messaging bindings that already reference the schema.
6. **Surface diagnostics.** Render the markdown alignment / import / validation report (single mode) or the contract-tool JSON envelope (cross-project mode). Cross-project consumer impact is reported by `specify compatibility`.
7. **Stay within change-local `contracts/schemas/`.** Do not modify baseline files in root `contracts/`, do not touch `contracts/http/` or `contracts/messages/`, and do not invent fields the spec does not justify — mark unknowns with `[unknown]`.

#### json-schema — artifact layout

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

- **JSON Schema Draft 2020-12** — never older drafts. The importer upgrades draft-04, draft-06, draft-07, and draft 2019-09 inputs. See [`references/json-schema-conventions.md`](../references/json-schema-conventions.md).
- **One type per file** — each `.yaml` defines exactly one top-level named type. Shared sub-types extracted to their own files; file-local sub-types may live under `$defs`.
- **Kebab-case `.yaml` filename** — the filename is the kebab-case form of the PascalCase type name (`UserRegistration` → `user-registration.yaml`). The filename is canonical: `$id` and `title` derive from it.
- **URN `$id`** — every schema declares `$id: "urn:specify:schemas/<filename-without-extension>"`. `$id` is stable for the schema's lifetime; renaming requires a new file with a new `$id` and explicit deprecation of the old one.
- **Opaque file replacement** — the slice-level `contracts/schemas/<type>.yaml` replaces the baseline file wholesale at merge time. Schema deltas are by file, not by property.

#### json-schema — intent dispatch

| Intent | Trigger | Sibling |
|---|---|---|
| Author or extend reusable schemas from a spec | build brief during `/spec:build`; operator extending the baseline for new payload types | `references/json-schema/author.md` |
| Import or normalise external schema files | operator drops schema files into a slice's `contracts/schemas/` directory | `references/json-schema/importer.md` |
| Verify `$ref` consistency, metadata, cross-format consumer compatibility, or merge-time baseline validation | build verification; post-merge contract baseline gate | `references/json-schema/verifier.md` |

The three intents share a common artefact contract (filename → `$id` derivation, one-type-per-file, draft policy) but have distinct algorithms — never conflate them.

#### json-schema — hard rules

1. **Valid JSON Schema Draft 2020-12.** Every output file must parse against `https://json-schema.org/draft/2020-12/schema`. The importer is the only entry point that accepts older drafts.
2. **One type per file.** Each `.yaml` file under `contracts/schemas/` defines exactly one top-level named type. Shared sub-types are separate files; file-local sub-types may use `$defs`.
3. **`$id` stability.** Once a `$id` is assigned, it never changes. New schemas get new `$id` values from the file path; the writer and importer never reassign existing ones, even when a baseline schema's `$id` is malformed (surface the issue as a normalisation finding instead).
4. **Filename ↔ `$id` ↔ `title` coherence.** The filename (kebab-case), the `$id` URN segment (kebab-case suffix), and the `title` (PascalCase) all describe the same type. Drift between them is a verifier failure.
5. **Kebab-case filenames.** All `.yaml` files use kebab-case names; no PascalCase or snake_case variants.
6. **No invention.** When the spec does not provide enough detail to derive a shape, mark the gap with `[unknown]`. The importer flags unrecognised constructs with `[import — manual review required]`.
7. **No protocol-specific authoring.** This sub-flow never writes path operations, channels, operations, request bodies, or response wrappers. Those belong to the openapi and asyncapi sub-flows.
8. **Read-only verifier.** The verifier sibling must not create, modify, or delete any files in either mode.
9. **Baseline immutability.** All output goes in the slice-local `contracts/` directory; baseline `contracts/` is read-only here.

## Output hygiene

- Only emit `.yaml` files under `$SLICE_DIR/contracts/`.
- Create `contracts/http/`, `contracts/messages/`, `contracts/schemas/` only when they will contain at least one file.
- Stay inside `$SLICE_DIR/contracts/`; baseline `contracts/` is off-limits to build.

## See also

- [`shape.md`](shape.md) — synthesis-time idiom guidance for the contracts target.
- [`merge.md`](merge.md) — landing brief, including the post-merge `contract` WASI tool gate.
- [`references/artifact-structure.md`](../references/artifact-structure.md) — directory layout for root `contracts/`.
- [`references/baseline-vs-delta.md`](../references/baseline-vs-delta.md) — cross-format minimal-delta rules and merge semantics.
- [`references/import-upgrade-policy.md`](../references/import-upgrade-policy.md) — shared framework for the importer siblings.
- [`references/report-shape.md`](../references/report-shape.md) — single-mode markdown, baseline validator JSON, and compatibility report JSON formats.
- [`references/cross-project-compatibility.md`](../references/cross-project-compatibility.md) — compatibility classifications and `change-kind` vocabulary used by `specify compatibility`.

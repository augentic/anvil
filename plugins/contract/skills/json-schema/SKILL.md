---
name: contract-json-schema
description: Author, import, and verify standalone JSON Schema documents shared by OpenAPI, AsyncAPI, and other contract formats. Use when a Specify change needs reusable payload schemas, when an operator supplies schema files without a protocol wrapper, or when validating schema compatibility across generated contracts.
argument-hint: "[slice-dir]"
---

# JSON Schema

Specialist for standalone JSON Schema (Draft 2020-12) documents on Specify changes — the shared payload vocabulary referenced by `/contract:openapi` HTTP operations and `/contract:asyncapi` message channels. This skill owns three intents: authoring or extending reusable payload schemas, importing or normalising externally supplied schema files, and verifying schema artefacts (single-mode internal consistency or cross-format consumer compatibility plus merge-time baseline validation).

The skill is JSON-Schema-only. Protocol bindings under `contracts/http/` belong to `/contract:openapi`; evented bindings under `contracts/messages/` belong to `/contract:asyncapi`. Both protocol skills delegate every payload-schema decision (`$id` shape, naming, decomposition, draft policy, metadata) to this skill.

## Critical Path (Quick Reference)

1. **Read the briefs and specs.** Open the active contracts build brief and the slice's `specs/` to identify which payload types the slice requires; read `contracts/schemas/` (the schema baseline) to know what shared vocabulary already exists.
2. **Identify the intent.** Map the trigger to one of three sibling files using the [Intent dispatch](#intent-dispatch) table — author, importer, or verifier. Stop reading SKILL.md once the sibling is selected; load only the relevant sibling.
3. **Dispatch to the sibling.** Open and follow [`author.md`](./author.md), [`importer.md`](./importer.md), or [`verifier.md`](./verifier.md). Each sibling owns its complete algorithm, decision rules, and output format.
4. **Write outputs to `contracts/schemas/`.** Author and importer paths produce or normalise JSON Schema YAML files under `$SLICE_DIR/contracts/schemas/` — one named type per file, kebab-case filenames, URN `$id` derived from the file path.
5. **Run the verifier.** After authoring or importing, invoke the verifier sibling to check `$ref` resolution, metadata completeness, duplicate-`$id` collisions, and cross-format consumer compatibility against any HTTP and messaging bindings that already reference the schema.
6. **Surface diagnostics.** Render the markdown alignment / import / validation report (single mode) or the contract-tool JSON envelope (cross-project mode) so the calling brief or operator can triage. Cross-project consumer impact is reported by `specify compatibility`.
7. **Stay within change-local `contracts/schemas/`.** Do not modify baseline files in root `contracts/`, do not touch `contracts/http/` or `contracts/messages/`, and do not invent fields the spec does not justify — mark unknowns with `[unknown]` instead.

## Invocation

```text
/contract:json-schema <slice-dir>
```

Optional internal positionals (recognised by the verifier sibling):

- `mode single` — default. Validate the slice's schema artefacts in isolation against the specs and any baseline bindings that reference them. Read-only, markdown report.
- `mode cross-project` — merge-time baseline validation delegate. Walks the merged `contracts/` directory through `specify tool run contract`; it does not compare consumer workspace clones. Read-only, JSON envelope. See `verifier.md` §Cross-project mode.

When invoked from the contracts capability build brief during `/spec:build`, `<slice-dir>` is the active slice directory; the brief routes the intent (author or importer) based on whether the operator supplied external schema files for `contracts/schemas/`. Consumer-impact reporting is a CLI concern under `specify compatibility`.

## Artifact layout

JSON Schema files live in two locations — the slice-local delta and the platform baseline:

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

- **JSON Schema Draft 2020-12** — never older drafts. Importer upgrades draft-04, draft-06, draft-07, and draft 2019-09 inputs. See [`json-schema-conventions`](../../references/json-schema-conventions.md).
- **One type per file** — each `.yaml` defines exactly one top-level named type. Shared sub-types extracted to their own files; file-local sub-types may live under `$defs`.
- **Kebab-case `.yaml` filename** — the filename is the kebab-case form of the PascalCase type name (`UserRegistration` → `user-registration.yaml`). The filename is canonical: `$id` and `title` derive from it.
- **URN `$id`** — every schema declares `$id: "urn:specify:schemas/<filename-without-extension>"`. `$id` is stable for the schema's lifetime; renaming requires a new file with a new `$id` and explicit deprecation of the old one.
- **Opaque file replacement** — the slice-level `contracts/schemas/<type>.yaml` replaces the baseline file wholesale at merge time. Schema deltas are by file, not by property.

For the broader directory layout, baseline-vs-delta semantics, and merge rules, see [`artifact-structure`](../../references/artifact-structure.md).

## Intent dispatch

Pick the sibling that matches the trigger. Each sibling is a self-contained algorithm — load only the one selected.

| Intent | Trigger | Sibling |
|---|---|---|
| Author or extend reusable schemas from a spec | contracts capability build brief during `/spec:build`; operator extending the baseline for new payload types | `author.md` |
| Import or normalise external schema files | operator drops schema files into a slice's `contracts/schemas/` directory | `importer.md` |
| Verify `$ref` consistency, metadata, cross-format consumer compatibility, or merge-time baseline validation | contracts capability build verification; post-merge contract baseline gate | `verifier.md` |

The three intents share a common artefact contract (filename → `$id` derivation, one-type-per-file, draft policy) but have distinct algorithms — never conflate them. An import must be followed by a verifier run before the brief considers the artefacts ready for merge; an author run normally ends with a verifier run too.

## Mixed-format ordering (run this skill first)

When a slice touches more than one contract format (HTTP + events + shared schemas), the contracts capability build brief invokes the format skills in this fixed order:

1. **`/contract:json-schema` first** — the schema vocabulary is shared and must stabilise before any binding references it. Authoring or importing a schema is a precondition for the protocol skills, not a peer step.
2. **`/contract:openapi`** — HTTP operations bind to the schemas authored above via `$ref: "../schemas/<type>.yaml"`.
3. **`/contract:asyncapi`** — message channels bind to the same schemas via `$ref: "../schemas/<type>.yaml"`.

This ordering is non-negotiable. Running OpenAPI or AsyncAPI ahead of json-schema produces dangling `$ref`s and forces protocol authors to either inline definitions (forbidden in the baseline) or guess at shapes (forbidden by the no-invention rule). The contracts capability build brief enforces the ordering; agent operators invoking these skills directly must follow it manually.

A corollary: this skill **never** writes outside `contracts/schemas/`. Schema-shaped intents that would land in `contracts/http/` or `contracts/messages/` are out of scope — route them to the appropriate protocol skill, which then references the shared schema authored here.

## Sibling references

- [`author.md`](./author.md) — `$id` assignment policy, one-type-per-file decomposition, schema-file naming, vocabulary for shared payloads, spec → schema mapping rules.
- [`importer.md`](./importer.md) — schema-only file detection, OpenAPI / AsyncAPI bundle rejection, draft upgrades (draft-04 / -06 / -07 / 2019-09 → 2020-12), Specify metadata injection.
- [`verifier.md`](./verifier.md) — `$ref` resolution, metadata completeness, duplicate-`$id` detection, cross-format consumer compatibility checks against existing OpenAPI / AsyncAPI bindings, single-mode verifier behavior, and cross-project baseline validation delegation.

## Shared format guidance

JSON-Schema-specific conventions live in [`json-schema-conventions`](../../references/json-schema-conventions.md) — `$id` URN format, required metadata fields (`$schema`, `title`, `description`, `type`), `$ref` conventions between schema files, type-mapping table from spec concepts to JSON Schema types, snake_case property naming, error-type structure. Read it before authoring or normalising; the sibling files link back to it where relevant.

For the cross-format directory layout, see [`artifact-structure`](../../references/artifact-structure.md); for the cross-format minimal-delta rules and merge semantics, see [`baseline-vs-delta`](../../references/baseline-vs-delta.md).

## Hard rules

These constraints are non-negotiable for any of the three sibling paths:

1. **Valid JSON Schema Draft 2020-12.** Every output file must parse against `https://json-schema.org/draft/2020-12/schema`. The importer is the only entry point that accepts older drafts.
2. **One type per file.** Each `.yaml` file under `contracts/schemas/` defines exactly one top-level named type. Shared sub-types are separate files; file-local sub-types may use `$defs`.
3. **`$id` stability.** Once a `$id` is assigned, it never changes. New schemas get new `$id` values from the file path; the writer and importer never reassign existing ones, even when a baseline schema's `$id` is malformed (surface the issue as a normalisation finding instead).
4. **Filename ↔ `$id` ↔ `title` coherence.** The filename (kebab-case), the `$id` URN segment (kebab-case suffix), and the `title` (PascalCase) all describe the same type. Drift between them is a verifier failure.
5. **Kebab-case filenames.** All `.yaml` files use kebab-case names; no PascalCase or snake_case variants.
6. **No invention.** When the spec does not provide enough detail to derive a shape, mark the gap with `[unknown]` in the alignment report rather than guessing. Importer flags unrecognised constructs with `[import — manual review required]`.
7. **No protocol-specific authoring.** This skill never writes path operations, channels, operations, request bodies, or response wrappers. Those belong to `/contract:openapi` and `/contract:asyncapi`.
8. **Read-only verifier.** The verifier sibling must not create, modify, or delete any files in either mode.
9. **Baseline immutability.** Never modify files in root `contracts/`. All output goes in the slice-local `contracts/` directory.

## Cross-format coordination

Because protocol bindings reference these schemas, edits in this skill can break already-merged HTTP and messaging contracts. The verifier sibling is the safety net:

- In `single` mode, verifier Check 4 (cross-format consumer compatibility) cross-references each touched schema against `$BASELINE_CONTRACTS/http/` and `$BASELINE_CONTRACTS/messages/` and flags any backwards-incompatible change before the brief approves the artefact.
- Cross-project producer-to-consumer impact is reported by `specify compatibility`; the verifier's `cross-project` mode is only the merge-time baseline validation delegate.

When authoring or importing, never silently delete or narrow a baseline schema's fields; if the spec requires it, surface the slice as a warning and let a human operator decide whether to bump the schema's `$id` (effectively introducing a new type with a deprecation path on the old one).

## Output hygiene

- Only emit `.yaml` files under `$SLICE_DIR/contracts/schemas/`.
- Create `contracts/schemas/` only when it will contain at least one file.
- Do not modify any file outside `$SLICE_DIR/contracts/schemas/`.
- Do not modify baseline files in root `contracts/`.
- Do not touch `contracts/http/` or `contracts/messages/` from this skill — even when the verifier reads them, it never writes.

## See also

- [`json-schema-conventions`](../../references/json-schema-conventions.md) — JSON Schema Draft 2020-12 conventions, `$id` URN format, type mapping, naming.
- [`artifact-structure`](../../references/artifact-structure.md) — directory layout for root `contracts/`.
- [`baseline-vs-delta`](../../references/baseline-vs-delta.md) — cross-format rules for computing the minimal delta between baseline and change-local files, including the `$id` stability rule and opaque-file-replacement merge contract.
- [`import-upgrade-policy`](../../references/import-upgrade-policy.md) — shared framework for the importer sibling (format detection, draft upgrades, lossless-vs-lossy decisions).
- [`report-shape`](../../references/report-shape.md) — single-mode markdown, baseline validator JSON, and compatibility report JSON formats.
- [`cross-project-compatibility`](../../references/cross-project-compatibility.md) — RM-04 compatibility classifications and `change-kind` vocabulary used by `specify compatibility` and by Check 4 in `single` mode.
- [`openapi-conventions`](../../references/openapi-conventions.md) — referenced for understanding how `/contract:openapi` consumes the schemas authored here.
- [`asyncapi-conventions`](../../references/asyncapi-conventions.md) — referenced for understanding how `/contract:asyncapi` consumes the schemas authored here.

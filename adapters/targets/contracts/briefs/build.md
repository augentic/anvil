# contracts.build

Orchestrates `/spec:build` for slices targeting the `contracts` adapter. Authors and validates machine-readable contract artifacts under the slice-local `contracts/` directory. Dispatches to three per-format sub-briefs (`build/json-schema.md`, `build/openapi.md`, `build/asyncapi.md`); each carries an internal author / import / verify intent table that fans out to references under `adapters/targets/contracts/references/<format>/`.

## Scope

Build writes only change-local contract deltas under `.specify/slices/<slice>/contracts/`:

- `contracts/schemas/*.yaml` — reusable JSON Schema payload vocabulary (one named type per file).
- `contracts/http/*.yaml` — OpenAPI 3.1 HTTP / resource-style documents.
- `contracts/messages/*.yaml` — AsyncAPI 3.0 evented / pub-sub / streaming / WebSocket documents.

Build MUST NOT edit the root `contracts/` baseline directly. Baseline updates happen only during `merge` (see [`merge.md`](merge.md)).

## Inputs

- `proposal.md` — authorship mode (author vs import), source material, interface scope, producer/consumer roles.
- `specs/<unit>/spec.md` — behavioural requirements: endpoints / channels / payloads / errors (one file per `proposal.md ## Units` entry). Provenance lines tell the brief whether the slice is author-driven (`Sources: [intent | <doc-key>]`) or import-driven (`Sources: [<code-or-contract-source>]`).
- `design.md` — the format selection (OpenAPI 3.1 / AsyncAPI 3.0 / JSON Schema), file-layout intent, and any cross-contract dependency notes (see [`shape.md`](shape.md)).
- The slice's `contracts/` subtree (if present) — partial deltas written by a prior pass.
- The root `contracts/` baseline — read-only context for `$ref` reuse and extension authoring.
- `tasks.md` — progress tracking.

Build consumes the synthesised Specify artifacts as its primary source. Do not treat raw design documentation as the contract source unless the proposal names it as Source Material and the synthesised `specs/<unit>/spec.md` files have captured the required behaviour.

## Algorithm

### Phase 1 — Classify

Identify the authorship mode from `proposal.md`: author-from-specs, import-existing-contracts, modify-existing-contracts, extract-from-source-code, or mixed. Then classify required formats from `design.md`: JSON Schema (reusable payload vocabulary), OpenAPI 3.1 (HTTP / resource), AsyncAPI 3.0 (evented / pub-sub / streaming / WebSocket).

### Phase 2 — Author or import (fixed format order)

When a slice touches more than one contract format, run the format sub-briefs in this fixed order — the schema vocabulary is shared and must stabilise before the bindings reference it:

1. **[build/json-schema.md](build/json-schema.md)** — author or import the minimal JSON Schema delta for reusable payload vocabulary. Owns `$id` assignment, one-type-per-file decomposition, and schema-file naming. Skip when the slice has no shared payload schemas.
2. **[build/openapi.md](build/openapi.md)** — author or import the minimal OpenAPI delta for HTTP / resource interactions. Reuse change-local or baseline `contracts/schemas/` files; do not author competing schemas under different filenames or `$id`s. Skip when the slice has no HTTP interactions.
3. **[build/asyncapi.md](build/asyncapi.md)** — author or import the minimal AsyncAPI delta for evented / pub-sub / streaming / WebSocket-style interactions. Follow the same schema-reuse rule. Skip when the slice has no evented interactions.

Import paths must produce an import report covering lossless changes, lossy changes, unsupported constructs, and manual-review warnings. See [`references/import-upgrade-policy.md`](../references/import-upgrade-policy.md).

**Identity & version.** Every top-level OpenAPI / AsyncAPI document emitted into `$SLICE_DIR/contracts/` (root key `openapi:` or `asyncapi:`) MUST set an `info.version` value that parses as SemVer per [semver.org](https://semver.org). New top-level contracts SHOULD set `info.x-specify-id` to a kebab-case slug (typically the file stem; `^[a-z][a-z0-9-]*$`, ≤ 64 characters). The author sub-flows enforce both rules; the import sub-flows preserve any source `info.x-specify-id` verbatim and surface non-SemVer `info.version` values as `[manual review required]` rather than auto-rewriting.

### Phase 3 — Verify

Verification runs the verifier intent of each format sub-brief that owns artifacts in the slice. Run only the formats that produced artifacts; skip the rest. The verifier siblings live under [`references/<format>/verifier.md`](../references/).

For mixed-format slices, the final verifier pass must check cross-format `$ref` consistency and report duplicate schema identities before build can complete. The format verifiers enforce the identity & version rules inline (SemVer `info.version`; kebab-case + ≤64-char `info.x-specify-id` when present; in-slice uniqueness on declared ids). The **cross-repo** uniqueness check is **not** part of build-time verification; it is the merge gate's job (see [`merge.md`](merge.md)).

Run each format's verifier in `mode: single` against the slice directory. The verifier reads slice-local artefacts plus the baseline for binding-coverage cross-references and emits a markdown alignment report. The verifier siblings are read-only — they MUST NOT create, modify, or delete any files.

### Phase 4 — Verify-repair loop (max 2 iterations)

If a verifier reports failures:

1. Re-enter the same format sub-brief with the verifier output for targeted repair via the same intent that produced the artifact (author or import).
2. Re-run that format's verifier.
3. If still failing after 2 iterations, stop and surface issues for human review. Do not mark the task complete. Report the remaining failures with full output and escalate for guidance.

A clean verification pass with zero issues is the expected outcome.

### Phase 5 — Tool gate

Build's final step invokes the declared `contract` WASI tool to confirm the slice's contract files parse and pass the validation rules in single-mode against the slice's delta:

```bash
specrun tool run contract -- "$SLICE_DIR/contracts" --format json > /tmp/contract-build.json
case $? in
  0) ;;  # clean — slice deltas are well-formed; proceed to task completion
  1) ;;  # findings — re-enter the failing format sub-brief per Phase 4
  2) ;;  # tool/validator could not run — escalate; do not mark the task complete
esac
```

The tool's `--format json` output shape is documented under [`references/report-shape.md`](../references/report-shape.md).

### No-op behaviour

When the slice's specs describe no API interactions and no Source Material lists importable contract artifacts, every format pass produces an empty delta and the verifiers have nothing to check. The brief completes as a no-op. This is normal for slices that touch only planning metadata or contract documentation without affecting an API surface.

## Output hygiene

- Only emit `.yaml` files under `$SLICE_DIR/contracts/`.
- Create `contracts/http/`, `contracts/messages/`, `contracts/schemas/` only when they will contain at least one file.
- Stay inside `$SLICE_DIR/contracts/`; baseline `contracts/` is off-limits to build.

## See also

- [`shape.md`](shape.md) — synthesis-time idiom guidance for the contracts target.
- [`merge.md`](merge.md) — landing brief, including the post-merge `contract` WASI tool gate.
- [`build/json-schema.md`](build/json-schema.md), [`build/openapi.md`](build/openapi.md), [`build/asyncapi.md`](build/asyncapi.md) — per-format sub-briefs.
- [`references/artifact-structure.md`](../references/artifact-structure.md) — directory layout for root `contracts/`.
- [`references/baseline-vs-delta.md`](../references/baseline-vs-delta.md) — cross-format minimal-delta rules and merge semantics.
- [`references/import-upgrade-policy.md`](../references/import-upgrade-policy.md) — shared framework for the importer siblings.
- [`references/report-shape.md`](../references/report-shape.md) — single-mode markdown, baseline validator JSON, and compatibility report JSON formats.
- [`references/cross-project-compatibility.md`](../references/cross-project-compatibility.md) — archived vocabulary for future consumer-impact reporting; today use the contract WASI verifier reports.

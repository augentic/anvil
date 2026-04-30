---
id: build
description: Build, import, repair, and validate contract artifacts
needs: [proposal, specs, tasks]
tracks: tasks
---

The build phase for contract-only changes produces and validates machine-readable contract artifacts. It owns contract authoring from specs, import normalization from supplied files, verifier-driven repair, and final structural validation.

Build writes only change-local contract deltas:

- `contracts/schemas/*.yaml`
- `contracts/http/*.yaml`
- `contracts/messages/*.yaml`

Build must not edit `.specify/contracts/` directly. Baseline updates happen only during merge.

Arguments:
- CHANGE_ID: the name of this change (from specify status)

## Inputs

Read:

- `proposal.md` for Authorship Mode, Source Material, interface scope, and producer/consumer roles.
- `specs/**/*.md` for prose-derived behavioral requirements or lightweight import-mode scope.
- Any source files named in proposal Source Material. If they are external contract artifacts, copy or normalize them into the change-local `contracts/` tree before verification.
- `.specify/contracts/` as read-only baseline context.
- `tasks.md` for progress tracking.

Build consumes Specify artifacts as its primary source. Do not treat raw design documentation as the contract source unless the proposal names it as Source Material and the define phase has captured the required behavior in `specs/**/*.md`.

## Algorithm

### Phase 1: Classify

Classify the change's Authorship Mode from `proposal.md`:

1. **Generate from prose** — author contract artifacts from `specs/**/*.md`.
2. **Import existing contracts** — normalize supplied OpenAPI, AsyncAPI, or JSON Schema files into Specify conventions.
3. **Modify existing contracts** — author a minimal delta against `.specify/contracts/` from the behavioral delta in `specs/**/*.md`.
4. **Mixed** — combine author and importer paths when the proposal explicitly includes both prose-derived requirements and supplied contract files.

Then classify required formats:

- JSON Schema: reusable payload vocabulary referenced by HTTP and/or evented interactions, or standalone schema imports.
- OpenAPI: HTTP/resource interactions or supplied OpenAPI/Swagger artifacts.
- AsyncAPI: evented, pub/sub, streaming, WebSocket-style interactions, or supplied AsyncAPI artifacts.

### Phase 2: Author or import

For prose-driven and modification work, run author intent in this order:

1. `/interfaces:json-schema` — author the minimal JSON Schema delta for reusable payload vocabulary. Owns `$id` assignment, one-type-per-file decomposition, and schema-file naming. Skip when the change has no shared payload schemas.
2. `/interfaces:openapi` — author the minimal OpenAPI delta for HTTP/resource interactions. Reuse change-local or baseline `contracts/schemas/` files; do not author competing schemas under different filenames or `$id`s. Skip when the change has no HTTP interactions.
3. `/interfaces:asyncapi` — author the minimal AsyncAPI delta for evented, pub/sub, streaming, or WebSocket-style interactions. Follow the same schema-reuse rule as `/interfaces:openapi`. Skip when the change has no evented interactions.

For import-driven work, run importer intent for each supplied format:

1. `/interfaces:json-schema` — import and normalize standalone JSON Schema files into `contracts/schemas/`.
2. `/interfaces:openapi` — import and normalize OpenAPI or Swagger artifacts into `contracts/http/`, decomposing payload schemas into `contracts/schemas/` as needed.
3. `/interfaces:asyncapi` — import and normalize AsyncAPI artifacts into `contracts/messages/`, decomposing payload schemas into `contracts/schemas/` as needed.

Importer paths must produce an import report covering lossless changes, lossy changes, unsupported constructs, and manual-review warnings.

### Phase 3: Verify

Verification runs the verifier intent of each `/interfaces:*` format skill that owns artifacts in the change. Run only the formats that produced artifacts; skip the rest.

1. `/interfaces:json-schema` — verifier intent: `$ref` resolution and schema metadata (`$id`, `title`, `description`) for every JSON Schema file under `contracts/schemas/`.
2. `/interfaces:openapi` — verifier intent: `$ref` resolution across the OpenAPI delta and binding completeness (every spec-referenced HTTP schema has at least one binding).
3. `/interfaces:asyncapi` — verifier intent: `$ref` resolution across the AsyncAPI delta and binding completeness (every spec-referenced evented schema has at least one binding).

For mixed-format changes, the final verifier pass must check cross-format `$ref` consistency and report duplicate schema identities before the build can complete.

When this brief runs as the post-merge cross-project consumer check (RFC-9 §3B), thread `--mode cross-project` through the format verifier paths so they emit the cross-project compatibility report instead of the single-change report.

### Phase 4: Verify-repair loop (max 2 iterations)

If a verifier reports failures:
1. Re-enter the same format skill (`/interfaces:openapi`, `/interfaces:asyncapi`, or `/interfaces:json-schema`) with the verifier output for targeted repair via the same intent that produced the artifact (author or importer).
2. Re-run that format's verifier intent.
3. If still failing after 2 iterations, stop and surface issues for human review. Do not mark the task complete. Report the remaining failures with full output and escalate for guidance.

A clean verification pass with zero issues is the expected outcome.

### No-op behavior

When the change's specs describe no API interactions and no Source Material lists importable contract artifacts, every format pass produces an empty delta and the verifiers have nothing to check. The brief completes as a no-op. This is normal for changes that modify planning metadata without affecting API surfaces.

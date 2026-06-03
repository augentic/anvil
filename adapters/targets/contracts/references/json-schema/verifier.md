# JSON Schema — Verifier

> **When to read this.** Read this when verifying a JSON Schema artefact — invoked by the contracts adapter build brief in `single` mode after the author or importer sibling produces output, by the contracts adapter merge brief in `cross-project` mode against the merged baseline (the contracts adapter merge contract), or directly by an operator running validation against existing artefacts. Skip this file when authoring (use [`author.md`](./author.md)) or normalising external documents (use [`importer.md`](./importer.md)).

The verifier is **read-only**. It MUST NOT generate, modify, or delete any files. Its sole output is a list of issues rendered as a validation report.

## Modes

The verifier accepts a `--mode {single, cross-project}` flag. The mode determines the report shape and the exit semantics.

| Mode               | Caller                                         | Trigger                                                   | Scope                                                                                                           | Output                                                |
| ------------------ | ---------------------------------------------- | --------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `single` (default) | contracts adapter build brief in `/spec:build` | Post-author or post-import                                | One slice's `contracts/schemas/` inside one project, plus the slice's and baseline's HTTP / messaging consumers | Markdown report for the verify-repair loop            |
| `cross-project`    | contracts adapter merge brief                  | Producer-side merge of a contract change touching schemas | Walk the merged `contracts/` baseline; enforce contract identity/version validation                                               | JSON envelope produced by `specify tool run contract` |

`single` mode feeds the brief's verify-repair loop and is the natural exit point for both author and importer runs. `cross-project` mode is a thin delegate over the declared `contract` WASI tool (the declared `contract` WASI tool) — the verifier shells out through `specify tool run contract` and surfaces its findings; it does not implement its own cross-baseline check. Both modes share the read-only contract.

`--mode` is an internal flag of the format-specific verifier. Cross-project consumer-impact analysis is deferred until a real consumer workflow exists; this verifier owns deterministic single-slice and merged-baseline checks.

Note: `specify tool run contract` walks **top-level OpenAPI 3.1 / AsyncAPI 3.0 documents only** (root key `openapi:` or `asyncapi:`). Standalone JSON Schema files under `contracts/schemas/` are payload vocabulary, not top-level contracts, and are skipped by the validator filter (the top-level contract filter). The `cross-project` invocation is therefore identical across all three format verifiers — the tool handles format selection internally.

## Inputs

### `single` mode

Inferred from the active slice context — no positional arguments required:

```text
$SLICE_DIR          = .specify/slices/<slice-name>
$CHANGE_CONTRACTS    = $SLICE_DIR/contracts/
$CHANGE_SCHEMAS      = $CHANGE_CONTRACTS/schemas/
$BASELINE_CONTRACTS  = contracts/
$BASELINE_SCHEMAS    = $BASELINE_CONTRACTS/schemas/
$CHANGE_SPECS        = $SLICE_DIR/specs/
```

### `cross-project` mode

Caller passes the merged baseline directory:

```text
$BASELINE_CONTRACTS = $PROJECT_ROOT/contracts   # the merged baseline, post-`specify slice merge run`
```

The verifier shells out to `specify tool run contract -- "$BASELINE_CONTRACTS" --format json` (the declared `contract` WASI tool), which walks every top-level OpenAPI 3.1 / AsyncAPI 3.0 document under `$BASELINE_CONTRACTS` and enforces the contract identity/version validation rules. Standalone schemas under `$BASELINE_CONTRACTS/schemas/` are not validated by the tool — they are payload vocabulary, not top-level contracts (the top-level contract filter + §Non-goals). Schema-side issues are caught earlier, in `single` mode, during the build verify-repair loop.

## Prerequisites

### `single` mode

- The author or importer sibling has completed and produced artefacts under `$CHANGE_SCHEMAS`.
- `.specify/project.yaml` exists (Specify is initialised).

If `$CHANGE_SCHEMAS` does not exist or contains no files, report all checks as passed — there is nothing to verify.

### `cross-project` mode

- `specify` is on `$PATH` and supports `specify tool run`.
- The current project resolves the contracts adapter sidecar that declares the `contract` tool.
- `$BASELINE_CONTRACTS` (`$PROJECT_ROOT/contracts`) is the directory the tool will walk. The declared read permission points at `$PROJECT_DIR/contracts`; if that directory is absent, `specify tool run` exits `2`. Callers MUST NOT pre-stat the path.

## Single-mode checks

Four independent checks run against `$CHANGE_SCHEMAS` and the artefacts that consume it. Run them in order; collect findings; produce a single markdown report at the end.

### Check 1 — `$ref` resolution

Every `$ref` in every schema file under `$CHANGE_SCHEMAS` must resolve. Three resolution scopes apply depending on the kind of `$ref`:

- **Cross-file refs to siblings** (`$ref: "<other-name>.yaml"`) — must resolve to a file in `$CHANGE_SCHEMAS` or in `$BASELINE_SCHEMAS`. Both are valid resolution scopes; mixed resolution (one delta + one baseline) is fine.
- **In-document refs** (`$ref: "#/$defs/<name>"`) — must resolve to a sibling key inside the same file's `$defs` map.
- **External URL refs** (`$ref: "https://..."`) — flagged as `WARN`. The verifier never chases external URLs.

For each `.yaml` file in `$CHANGE_SCHEMAS`:

1. Read the file and find every `$ref` value (top-level, nested in `properties`, nested in `items`, nested in `oneOf` / `anyOf` / `allOf`, nested in `$defs`).
2. Classify each `$ref` as cross-file, in-document, or external URL.
3. Resolve cross-file refs against `$CHANGE_SCHEMAS` and `$BASELINE_SCHEMAS`; resolve in-document refs against the same file's `$defs`.
4. Report any `$ref` that does not resolve.

Report format (one entry per failure):

```
FAIL: contracts/schemas/order.yaml — $ref "missing-type.yaml" does not resolve (checked change contracts/schemas/ and baseline contracts/schemas/)
FAIL: contracts/schemas/user.yaml — $ref "#/$defs/MissingSubType" does not resolve in-document
WARN: contracts/schemas/legacy.yaml — $ref "https://example.com/schemas/foo" is an external URL; not chased
```

### Check 2 — Metadata completeness

Every JSON Schema file in `$CHANGE_SCHEMAS` must have the required metadata fields defined in [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md).

| Field         | Rule                                                                                                                                                                                 |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `$schema`     | Present and equal to `"https://json-schema.org/draft/2020-12/schema"`. Older drafts emit `WARN` (importer should have upgraded).                                                     |
| `$id`         | Present, well-formed URN of the shape `urn:specify:schemas/<segment>` where `<segment>` matches the kebab-case filename.                                                             |
| `title`       | Present, non-empty, PascalCase, and corresponds to the filename (kebab-case → PascalCase round-trips).                                                                               |
| `description` | Present, non-empty. The placeholder `"[imported — description pending review]"` counts as present but **emits a `WARN`** to surface the gap for the verify-repair loop before merge. |
| `type`        | Present (almost always `object`; primitives are rare).                                                                                                                               |

Report format (one entry per failure):

```
FAIL: contracts/schemas/user-registration.yaml — missing required field "$id"
FAIL: contracts/schemas/error-response.yaml — "description" is empty
FAIL: contracts/schemas/user.yaml — "$id" is "urn:example:user"; expected "urn:specify:schemas/user"
FAIL: contracts/schemas/order.yaml — "title" is "order"; expected PascalCase ("Order")
WARN: contracts/schemas/payment.yaml — "$schema" is Draft 7; expected Draft 2020-12 (importer normalisation needed)
WARN: contracts/schemas/oauth-token.yaml — "description" is "[imported — description pending review]"; replace before merge
```

### Check 3 — Duplicate-`$id` detection

Across every schema file in `$CHANGE_SCHEMAS` plus every schema file in `$BASELINE_SCHEMAS`, the `$id` values must be globally unique. The author's filename → `$id` derivation guarantees this when the one-type-per-file rule holds, but importer paths and manual edits can break the invariant.

Algorithm:

1. Read every `.yaml` under `$CHANGE_SCHEMAS` and `$BASELINE_SCHEMAS`. Record `(filename, $id)` pairs.
2. Group by `$id`. Any group with more than one entry is a collision.
3. Classify each collision:

| Collision kind                                                            | Severity | Description                                                                                |
| ------------------------------------------------------------------------- | -------- | ------------------------------------------------------------------------------------------ |
| Two delta files share `$id`                                               | `FAIL`   | The slice is internally inconsistent.                                                      |
| Delta file shares `$id` with a baseline file but the **filenames** differ | `FAIL`   | The author / importer reassigned a baseline `$id` (forbidden by the `$id` stability rule). |
| Delta file shares `$id` with a baseline file and the filenames match      | `INFO`   | Expected — the delta replaces the baseline file at merge time.                             |

Report format:

```
FAIL: contracts/schemas/user-billing.yaml and contracts/schemas/user-platform.yaml share $id "urn:specify:schemas/user-billing"
FAIL: contracts/schemas/oauth-token.yaml shares $id "urn:specify:schemas/auth-token" with contracts/schemas/auth-token.yaml (filenames differ; $id reassignment is forbidden)
INFO: contracts/schemas/user.yaml replaces contracts/schemas/user.yaml at merge ($id "urn:specify:schemas/user"); shape diff documented in alignment report
```

### Check 4 — Cross-format consumer compatibility

Every schema in `$CHANGE_SCHEMAS` is potentially referenced by an existing OpenAPI or AsyncAPI binding in the baseline. Changing the schema can silently break those bindings — and, transitively, every downstream consumer that generates code from them.

Resolution scope:

- **Producers of `$ref`** are the binding files in `$BASELINE_CONTRACTS/http/` and `$BASELINE_CONTRACTS/messages/`. The verifier inspects each binding's `$ref` values to discover which schemas it consumes.
- **Producers of `$ref`** in the slice directory (`$CHANGE_CONTRACTS/http/`, `$CHANGE_CONTRACTS/messages/`) are also inspected — a mixed-format change may be authoring its own bindings concurrently.

Algorithm:

1. **Build the consumer graph.** For each schema file `<name>.yaml` in `$CHANGE_SCHEMAS`, scan baseline and change-local bindings for `$ref` values that resolve to it. Record the list of consumers per schema.
2. **For each schema with at least one baseline consumer**, diff the delta schema against the baseline schema (both at `<name>.yaml`) and classify each property-level change:

| `change-kind`                     | Severity     | Description                                                                                                                                                                        |
| --------------------------------- | ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `removed-field`                   | `WARN`       | A property the baseline schema defined is absent in the delta. Every binding that exposes this field will produce smaller payloads; consumer code reading it will see `undefined`. |
| `required-field-added`            | `WARN`       | A property became `required` in the delta. Every binding accepting this schema as a request body will reject prior consumer requests.                                              |
| `type-narrowed`                   | `WARN`       | A property's `type`, `format`, `enum`, `pattern`, or numeric range narrowed. Consumer values that were valid before may now be rejected.                                           |
| `enum-value-removed`              | `WARN`       | A value disappeared from a property's `enum` array. Consumers emitting that value will be rejected.                                                                                |
| `additional-properties-tightened` | `WARN`       | The schema flipped from `additionalProperties: true` (or absent) to `additionalProperties: false`. Consumers passing extra fields will be rejected.                                |
| `optional-field-added`            | (no warning) | Backwards-compatible additive change.                                                                                                                                              |
| `enum-value-added`                | (no warning) | Backwards-compatible additive change.                                                                                                                                              |
| `description-changed`             | (no warning) | Behavioural docstring drift; not a wire change.                                                                                                                                    |

3. **For each schema with no consumers**, skip Check 4 — there is no binding-side risk surface inside the slice. The compatibility risk lives entirely in `cross-project` mode (downstream projects may have their own consumers).

The `change-kind` enum above is a contract-domain classification — not the `LintFinding` severity enum. The Severity column gives the markdown-report ladder for the verify-repair loop; if a Check 4 finding is later surfaced as a `LintFinding`, the closed `LintFinding` severity enum (`critical` / `important` / `suggestion` / `optional`) sits on the envelope while `change-kind` and the per-property contract context (schema pointer, property name, baseline binding paths) travel inside `evidence.kind: structured` under `evidence.data` (see `schemas/diagnostics/diagnostic.schema.json`).

Report format:

```
WARN: contracts/schemas/user.yaml — removed property `email`; baseline binding contracts/http/user-api.yaml exposes it on GET /users/{user_id} response (REQ-007)
WARN: contracts/schemas/order.yaml — added required property `currency`; baseline binding contracts/messages/order-events.yaml uses it as message payload (channel `order.placed`)
WARN: contracts/schemas/error-response.yaml — narrowed enum on `code` field (removed value `RATE_LIMITED`); 4 baseline bindings reference this schema
```

When the slice has **no specs**, Check 4 still runs — the binding consumers exist independently of the spec scenarios.

## Single-mode algorithm

1. **Determine scope.**
   - `$CHANGE_SCHEMAS`, `$BASELINE_SCHEMAS`.
   - `$CHANGE_CONTRACTS/http/`, `$CHANGE_CONTRACTS/messages/`, `$BASELINE_CONTRACTS/http/`, `$BASELINE_CONTRACTS/messages/` (for Check 4 consumer discovery).
   - If `$CHANGE_SCHEMAS` is empty or absent, report all checks as passed and stop.
2. **Run Check 1** (`$ref` resolution) on every `.yaml` file in `$CHANGE_SCHEMAS`.
3. **Run Check 2** (metadata completeness) on every `.yaml` file in `$CHANGE_SCHEMAS`.
4. **Run Check 3** (duplicate-`$id` detection) across `$CHANGE_SCHEMAS` ∪ `$BASELINE_SCHEMAS`.
5. **Run Check 4** (cross-format consumer compatibility) by walking the consumer graph and diffing delta schemas against their baseline equivalents.
6. **Collect findings** and produce the markdown validation report.

## Single-mode output format

When issues are found:

```markdown
## Validation Report (Schemas)

### $ref Resolution
- ✗ contracts/schemas/order.yaml — $ref "missing-type.yaml" does not resolve
- ✓ 18 of 19 $ref pointers resolve

### Metadata Completeness
- ✗ contracts/schemas/user-registration.yaml — missing "description"
- ⚠ contracts/schemas/payment.yaml — "$schema" is Draft 7; expected Draft 2020-12
- ✓ 5 of 7 schemas have complete metadata

### Duplicate $id
- ✓ All $id values unique within change ∪ baseline

### Cross-format Consumer Compatibility
- ⚠ contracts/schemas/user.yaml — removed property `email`; baseline binding contracts/http/user-api.yaml exposes it on GET /users/{user_id}
- ✓ 6 of 7 changed schemas are backwards-compatible

### Summary
- **Checks passed:** 1 of 4
- **Issues found:** 3 (1 fail, 2 warn)
```

When all checks pass:

```markdown
## Validation Report (Schemas)

All checks passed (19 $ref pointers, 7 schemas, 0 $id collisions, 0 backwards-incompatible changes).
```

`single` mode preserves its existing exit semantics: zero on clean reports, non-zero on read errors.

## Cross-project mode

`cross-project` mode runs **after** a producer's contract change merges. The contracts adapter merge brief invokes it as the post-merge baseline gate (the contracts adapter merge contract); `/spec:execute` re-uses the same gate per project after a producer-side merge (the workspace execution contract).

The mode is a thin delegate over `specify tool run contract` (the declared `contract` WASI tool). The verifier sibling does not implement an independent cross-project algorithm — it shells out to the declared WASI tool, exits with the tool's exit code, and lets the caller (the merge brief, or `/spec:execute`) consume the JSON envelope. The deterministic checks the tool enforces are the contract identity/version validation rules over the merged baseline's top-level OpenAPI / AsyncAPI documents:

- `contract.version-is-semver` — every top-level document's `info.version` parses as SemVer (per [semver.org](https://semver.org), prerelease labels included).
- `contract.id-format` — when `info.x-specify-id` is present, the value matches `^[a-z][a-z0-9-]*$` and is ≤ 64 characters.
- `contract.id-unique` — every present `info.x-specify-id` is unique across all top-level contracts under `$BASELINE_CONTRACTS`.

Standalone JSON Schemas under `$BASELINE_CONTRACTS/schemas/` are payload vocabulary and are skipped by the binary's `openapi:` / `asyncapi:` filter. The schema-side cross-format consumer compatibility check (single-mode Check 4) catches breakage before the merge phase; the binary's role at merge time is the deterministic top-level-contract gate.

### Invocation

```bash
specify tool run contract -- "$PROJECT_ROOT/contracts" --format json > /tmp/contract-findings.json
case $? in
  0) ;;  # clean — no findings, baseline is well-formed
  1) ;;  # findings present — caller MUST treat as `failure`
  2) ;;  # tool/validator could not run — caller MUST treat as `failure` and journal diagnostics
esac
```

`--format json` is the canonical shape callers parse; `--format text` is the human-readable variant the operator can re-run by hand. Both produce the same exit code.

### JSON envelope

The declared tool writes a single JSON object to stdout in `--format json` when the validator runs:

```json
{
  "envelope-version": 2,
  "contracts-dir": "<absolute-baseline-path>",
  "ok": false,
  "findings": [
    { "path": "contracts/http/user-api.yaml", "rule-id": "contract.version-is-semver", "detail": "info.version `2024-01-15` is not valid SemVer (must parse per semver.org, including optional prerelease labels)" },
    { "path": "contracts/messages/billing-events.yaml", "rule-id": "contract.id-unique", "detail": "info.x-specify-id `shared` also appears in contracts/http/legacy-api.yaml" }
  ],
  "exit-code": 1
}
```

Field semantics:

- `envelope-version` — currently `2`; bumps follow the contract envelope versioning policy. Callers MUST validate this before parsing the rest of the envelope.
- `contracts-dir` — the absolute path the tool walked, echoing the positional argument.
- `ok` — `true` iff `findings` is empty.
- `findings[].path` — repo-relative when the parent of `<baseline-dir>` matches the path's prefix, otherwise absolute. Suitable for verbatim rendering in operator-facing reports.
- `findings[].rule-id` — one of `contract.version-is-semver`, `contract.id-format`, `contract.id-unique`.
- `findings[].detail` — single-line human-readable description.
- `exit-code` — mirrors the process exit code (`0` clean / `1` findings / `2` invocation error).

Callers that surface post-merge validator failures (the merge brief on a blocking finding) parse `findings[]` and include `{ rule-id, path, detail }` triples in the stop hint's `paths` field. The load-bearing finding is typically `findings[0].rule-id` plus a one-line restatement of `findings[0].detail`; the full envelope is captured at the log path referenced in the stop hint.

When a caller re-surfaces an envelope finding as a `LintFinding` (see `schemas/diagnostics/diagnostic.schema.json` in the CLI repo), the mapping is: `findings[].rule-id` → `rule-id`, `findings[].path` → `location.path`, `target-adapter: contracts`. Schema-side contract metadata (schema pointer, `$id`, plus any single-mode Check 4 `change-kind` such as `removed-field` / `required-field-added` / `type-narrowed` / `enum-value-removed` / `additional-properties-tightened`) lives inside `evidence.kind: structured` with the contract data under `evidence.data`. The closed `LintFinding` severity enum (`critical` / `important` / `suggestion` / `optional`) is separate from any compatibility classification — classifiers remain contract-domain evidence fields, not severity.

### Exit semantics

| Exit code | Meaning                                                                                                | Caller action                                                                                        |
| --------- | ------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------- |
| `0`       | Clean — no findings.                                                                                   | Proceed to the next merge-brief step (e.g. `specify slice merge run`).                               |
| `1`       | One or more findings.                                                                                  | Record `failure`; halt the merge brief. The slice's deltas remain unmerged.                          |
| `2`       | The tool could not run (resolver, permission, runtime, path missing, not a directory, internal error). | Record `failure`; journal the stderr line or JSON error message. The slice's deltas remain unmerged. |

The mode is **deterministic**: the WASI tool is a thin shell over `specify_validate::validate_baseline_contracts` (in the `specify-cli` workspace). Repeated invocations against the same baseline produce identical findings.

### Why a WASI tool delegate?

The contracts adapter owns merge gating through the declared `contract` WASI tool: a deterministic, adapter-owned gate the merge brief can run through `specify` without crossing the core boundary or re-introducing concern-specific behavior into core crates.

Schema-side breakage is caught earlier, in `single`-mode Check 4 (cross-format consumer compatibility), before the merge phase. The deterministic binary is the canonical post-merge gate.

## Edge cases

### `single` mode

| Scenario                                                                     | Behavior                                                                                        |
| ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Change directory has no `contracts/schemas/`                                 | Pass — nothing to verify.                                                                       |
| Baseline has schemas but change does not                                     | Pass — verifier only checks change-level artefacts.                                             |
| `$ref` target exists in baseline but not in change                           | Pass — baseline is a valid resolution target.                                                   |
| `$ref` target exists in change but not in baseline                           | Pass — change-level schemas are valid resolution targets.                                       |
| Change adds a schema with no consumers in either change or baseline bindings | Skip Check 4 for that schema (no consumer surface inside the slice).                            |
| Schema declares `additionalProperties` neither true nor false                | Pass — the field is genuinely optional. Authors default to `false`; importers preserve absence. |
| File-local `$defs` entry referenced only inside its parent                   | Pass — file-local sub-types are valid.                                                          |
| File contains a `$schema` URI for an older draft                             | Emit `WARN` recommending importer normalisation; do not fail.                                   |

### `cross-project` mode

| Scenario                                                                         | Behavior                                                                                                                                                                        |
| -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `$BASELINE_CONTRACTS` is absent                                                  | `specify tool run` exits `2` because the declared read preopen is missing or because the validator reports the missing directory. Caller records `failure`.                     |
| `$BASELINE_CONTRACTS` is empty (no top-level contracts)                          | Tool exits `0` with `findings: []`. Treated as clean.                                                                                                                           |
| Top-level contract has non-SemVer `info.version`                                 | Finding `contract.version-is-semver`; exit `1`. Caller records `failure`.                                                                                                       |
| Top-level contract has malformed `info.x-specify-id`                             | Finding `contract.id-format`; exit `1`.                                                                                                                                         |
| Two top-level contracts share the same `info.x-specify-id`                       | Finding `contract.id-unique` against each colliding path; exit `1`.                                                                                                             |
| Standalone JSON Schema under `$BASELINE_CONTRACTS/schemas/` has missing metadata | **Not** a `cross-project` concern — schema-only files are skipped by the binary's `openapi:` / `asyncapi:` filter. Schema-side issues are caught in `single` mode (Checks 1–4). |
| YAML file under `$BASELINE_CONTRACTS` is malformed                               | Skipped by the validator (the format-verifier owns YAML diagnostics in `single` mode); does not surface as a cross-project finding.                                             |
| `specify` is not on `$PATH`                                                      | The shell-out fails with `command not found`; caller records `failure` with the resolve diagnostic on `--context`.                                                              |

## Guardrails

- **Read-only.** Never create, modify, or delete files. Both modes share this contract.
- Report every issue with the file path and a description of the problem.
- Use `WARN` rather than `FAIL` (in `single` mode) when classification is ambiguous, e.g. when a schema is referenced by no bindings but might be shared vocabulary the spec brief just hasn't bound yet.
- Do not attempt to fix issues — report them. Repair belongs to the author or importer sibling.
- **`cross-project` mode is fatal.** Treat exit codes `1` (findings) and `2` (invocation error) as `failure` per the contracts adapter merge contract. The merge brief MUST halt; the slice's deltas remain unmerged until the operator resolves the finding.
- Do not re-implement the tool's checks. The verifier sibling's `cross-project` mode is a delegate; the canonical algorithm lives in `crates/validate/src/contracts.rs` of the `specify-cli` workspace.

## Verification checklist

### `single` mode

Before completing the run:

- [ ] All `.yaml` files in `$CHANGE_SCHEMAS` scanned for `$ref` resolution.
- [ ] All `.yaml` files in `$CHANGE_SCHEMAS` checked for `$schema`, `$id`, `title`, `description`, `type`.
- [ ] All `$id` values across `$CHANGE_SCHEMAS` ∪ `$BASELINE_SCHEMAS` checked for duplicates.
- [ ] Cross-format consumer compatibility checked for every change-touched schema with at least one baseline or change-local binding consumer.
- [ ] Validation report produced with per-check results and summary.
- [ ] No files created or modified.

### `cross-project` mode

Before completing the run:

- [ ] `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json` invoked exactly once.
- [ ] Stdout (the JSON envelope) captured for the caller (typically the merge brief's `--context`).
- [ ] Exit code propagated verbatim to the caller (`0` clean / `1` findings / `2` tool or validator error).
- [ ] No findings re-classified, suppressed, or downgraded — the tool's output is authoritative.
- [ ] No files created or modified.

## See also

- [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md) — Draft 2020-12 conventions, `$id` URN format, metadata rules.
- [`../../references/artifact-structure.md`](../../references/artifact-structure.md) — directory layout for the slice-local delta and the baseline.
- [`../../references/report-shape.md`](../../references/report-shape.md) — single-mode markdown report shape this verifier emits.
- [`../../references/cross-project-compatibility.md`](../../references/cross-project-compatibility.md) — `change-kind` enumeration used by Check 4 (single-mode cross-format consumer compatibility).
- [`../../briefs/merge.md`](../../briefs/merge.md) — merge brief that owns the post-merge `specify tool run contract` invocation and the §Merge and adoption contract three-branch outcome wiring.
- [`author.md`](./author.md) — sibling for spec-driven authoring.
- [`importer.md`](./importer.md) — sibling for normalising external documents.

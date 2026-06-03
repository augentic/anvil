# AsyncAPI — Verifier

> **When to read this.** Read this when verifying an AsyncAPI artefact — invoked by the contracts adapter build brief in `single` mode after the author or importer sibling produces output, by the contracts adapter merge brief in `cross-project` mode against the merged baseline (the contracts adapter merge contract), or directly by an operator running validation against an existing artefact. Skip this file when authoring (use [`author.md`](./author.md)) or normalising an external document (use [`importer.md`](./importer.md)).

The verifier is **read-only**. It MUST NOT generate, modify, or delete any files. Its sole output is a list of issues rendered as a validation report.

## Modes

The verifier accepts a `--mode {single, cross-project}` flag. The mode determines the report shape and the exit semantics.

| Mode               | Caller                                         | Trigger                                            | Scope                                                             | Output                                                |
| ------------------ | ---------------------------------------------- | -------------------------------------------------- | ----------------------------------------------------------------- | ----------------------------------------------------- |
| `single` (default) | contracts adapter build brief in `/spec:build` | Post-author or post-import                         | One slice's `contracts/messages/` inside one project              | Markdown report for the verify-repair loop            |
| `cross-project`    | contracts adapter merge brief                  | Producer-side merge of an AsyncAPI contract change | Walk the merged `contracts/` baseline; enforce contract identity/version validation | JSON envelope produced by `specify tool run contract` |

`single` mode feeds the brief's verify-repair loop. `cross-project` mode is a thin delegate over the declared `contract` WASI tool (the declared `contract` WASI tool) — the verifier shells out through `specify tool run contract` and surfaces its findings; it does not implement its own cross-baseline check. Both modes share the read-only contract.

`--mode` is an internal flag of the format-specific verifier. Cross-project consumer-impact analysis is deferred until a real consumer workflow exists; this verifier owns deterministic single-slice and merged-baseline checks.

## Inputs

### `single` mode

Inferred from the active slice context — no positional arguments required:

```text
$SLICE_DIR          = .specify/slices/<slice-name>
$CHANGE_CONTRACTS    = $SLICE_DIR/contracts/
$BASELINE_CONTRACTS  = contracts/
$CHANGE_SPECS        = $SLICE_DIR/specs/
```

### `cross-project` mode

Caller passes the merged baseline directory:

```text
$BASELINE_CONTRACTS = $PROJECT_ROOT/contracts   # the merged baseline, post-`specify slice merge run`
```

The verifier shells out to `specify tool run contract -- "$BASELINE_CONTRACTS" --format json` (the declared `contract` WASI tool), which walks every top-level OpenAPI 3.1 / AsyncAPI 3.0 document under `$BASELINE_CONTRACTS` and enforces the contract identity/version validation rules. No producer / consumer arguments are accepted — the tool's scope is the baseline as a whole.

## Prerequisites

### `single` mode

- The author or importer sibling has completed and produced artefacts under `$CHANGE_CONTRACTS/messages/`.
- `.specify/project.yaml` exists (Specify is initialised).

If `$CHANGE_CONTRACTS/messages/` does not exist or contains no files, report all checks as passed — there is nothing to verify.

### `cross-project` mode

- `specify` is on `$PATH` and supports `specify tool run`.
- The current project resolves the contracts adapter sidecar that declares the `contract` tool.
- `$BASELINE_CONTRACTS` (`$PROJECT_ROOT/contracts`) is the directory the tool will walk. The declared read permission points at `$PROJECT_DIR/contracts`; if that directory is absent, `specify tool run` exits `2`. Callers MUST NOT pre-stat the path.

## Single-mode checks

Three independent checks run against `$CHANGE_CONTRACTS/messages/` and the schemas it references. Run them in order; collect findings; produce a single markdown report at the end.

### Check 1 — `$ref` resolution

All `$ref` pointers in AsyncAPI files must resolve. Two resolution scopes apply depending on the kind of `$ref`:

- **Cross-file payload refs** (`$ref: "../schemas/<name>.yaml"`) — must resolve to a file in `$CHANGE_CONTRACTS/schemas/` **or** `$BASELINE_CONTRACTS/schemas/`.
- **In-document refs** (`$ref: "#/components/messages/..."`, `$ref: "#/channels/..."`, `$ref: "#/components/messageTraits/..."`, `$ref: "#/components/operationTraits/..."`) — must resolve to a sibling key inside the same AsyncAPI file.

For each `.yaml` file in `$CHANGE_CONTRACTS/messages/`:

1. Read the file and find every `$ref` value (channel→message, operation→channel, operation→message, message→payload, message→trait, operation→trait).
2. Classify each `$ref` as cross-file or in-document.
3. Resolve cross-file refs against `$CHANGE_CONTRACTS` and `$BASELINE_CONTRACTS`; resolve in-document refs against the same file.
4. Report any `$ref` that does not resolve.

Report format (one entry per failure):

```
FAIL: contracts/messages/order-events.yaml — $ref "../schemas/missing-type.yaml" does not resolve (checked change contracts/schemas/ and baseline contracts/schemas/)
FAIL: contracts/messages/order-events.yaml — $ref "#/components/messages/MissingMessage" does not resolve in-document
```

The verifier does not chase external URL `$ref`s; it flags them as `WARN` (cross-format URL refs are out of scope).

### Check 2 — Message and schema metadata

Every message in `components/messages` and every JSON Schema file referenced by an AsyncAPI message must have the required metadata fields.

#### Message metadata

| Field         | Rule                                                                                                                                                |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `name`        | Present and PascalCase, matching the message key.                                                                                                   |
| `contentType` | Present. Default to `application/json` when the spec does not require otherwise; flag any other value for human review unless explicitly justified. |
| `payload`     | Present and either an inline schema (flagged for normalisation) or a `$ref` to `../schemas/`.                                                       |

#### Payload schema metadata

For every JSON Schema file in `$CHANGE_CONTRACTS/schemas/` referenced by a message payload:

| Field         | Rule                                                                                                                                                                       |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `$id`         | Present and a valid URI (URN format: `urn:specify:schemas/<name>`).                                                                                                        |
| `title`       | Present and non-empty, matching the type name.                                                                                                                             |
| `description` | Present and non-empty. The placeholder `"[imported — description pending review]"` counts as present but **emits a `WARN`** to surface the gap for the verify-repair loop. |

Report format (one entry per failure):

```
FAIL: contracts/messages/order-events.yaml — message "OrderPlacedMessage" missing required field "contentType"
FAIL: contracts/schemas/order-placed.yaml — missing required field "$id"
WARN: contracts/schemas/order-cancelled.yaml — "description" is "[imported — description pending review]"; replace before merge
```

### Check 3 — Binding completeness

Every schema that appears as a top-level message payload in a spec scenario must have at least one AsyncAPI message that references it via `$ref`, and at least one operation that points at the channel carrying that message.

Resolution scope for the binding:

- `$CHANGE_CONTRACTS/messages/` — channels and operations added by this slice.
- `$BASELINE_CONTRACTS/messages/` — channels and operations already in the platform baseline.

#### Determining spec-referenced schemas

Read the `*.md` files under `$CHANGE_SPECS` and identify schemas that the spec mentions as message payloads:

- Pub/sub event payloads (e.g. "publish an `OrderPlaced` event").
- Stream record payloads (e.g. "emit a `MetricRecord` to the metrics topic").
- Command payloads (e.g. "send a `CancelOrder` command").

Cross-reference these against the schema files in `$CHANGE_CONTRACTS/schemas/`, the messages in `$CHANGE_CONTRACTS/messages/` + `$BASELINE_CONTRACTS/messages/`, and the operations in those same files.

#### Shared vocabulary exemption

Shared vocabulary types that appear only as `$ref` targets inside other schemas are exempt — they are reusable building blocks, not standalone payloads. A schema qualifies as shared vocabulary if both:

1. It is referenced via `$ref` from within other schema files (not directly from message payload definitions), AND
2. It does not appear as a top-level message payload in any spec scenario.

Common examples: `error-response.yaml`, `pagination.yaml`, `audit-metadata.yaml`.

Report format:

```
FAIL: contracts/schemas/order-placed.yaml — appears as message payload in spec scenario REQ-021 but has no AsyncAPI message binding
WARN: contracts/schemas/order-event-metadata.yaml — appears in spec but has no protocol binding (may be shared vocabulary — verify intent)
FAIL: contracts/messages/order-events.yaml — channel "orderPlaced" has no operation (neither send nor receive)
```

Use `FAIL` when the schema is unambiguously a top-level message payload in a spec scenario, or when a channel has no operations at all. Use `WARN` when classification is ambiguous — the verify-repair loop surfaces the warning for human review.

When the slice has **no specs**, skip Check 3 — there are no scenarios to cross-reference. Record this in the report so the brief knows the check was deliberately bypassed.

### Check 4 — Identity & version (contract identity/version validation)

For every top-level AsyncAPI document under `$CHANGE_CONTRACTS/messages/` (root key `asyncapi:`), enforce the contract identity/version validation rules:

1. **`info.version` MUST parse as SemVer.** Per [semver.org](https://semver.org), including optional prerelease labels (`1.0.0-draft.1`). Missing, non-string, or non-SemVer values are `FAIL`.
2. **`info.x-specify-id` (when present) MUST match `^[a-z][a-z0-9-]*$` and be ≤ 64 characters.** Format violations are `FAIL`.
3. **Within the slice directory, `info.x-specify-id` values MUST be unique.** When two top-level AsyncAPI documents in `$CHANGE_CONTRACTS/messages/` declare the same id, both are `FAIL`.

The cross-repo uniqueness check (the same id declared by a top-level contract somewhere else under root `contracts/`) is **not** part of single mode — it is the merge-phase gate's job, run by `specify tool run contract` against the merged baseline (the contracts adapter merge contract). The single-mode skill only flags duplicates inside the slice to keep the verifier deterministic and self-contained.

Report format (one entry per failure):

```
FAIL: contracts/messages/order-events.yaml — info.version `2024-01-15` is not valid SemVer
FAIL: contracts/messages/billing-events.yaml — info.x-specify-id `Billing-Events` must match `^[a-z][a-z0-9-]*$` and be ≤ 64 characters
FAIL: contracts/messages/admin-events.yaml — info.x-specify-id `shared` is also declared by contracts/messages/legacy-events.yaml in this slice
```

## Single-mode algorithm

1. **Determine scope.**
   - `$CHANGE_CONTRACTS/messages/`, `$CHANGE_CONTRACTS/schemas/`.
   - `$BASELINE_CONTRACTS/messages/`, `$BASELINE_CONTRACTS/schemas/`.
   - `$CHANGE_SPECS/`.
   - If `$CHANGE_CONTRACTS/messages/` is empty or absent, report all checks as passed and stop.
2. **Run Check 1** ($ref resolution) on every `.yaml` file in `$CHANGE_CONTRACTS/messages/`.
3. **Run Check 2** (message and schema metadata) on every message in `components/messages` and every payload schema in `$CHANGE_CONTRACTS/schemas/` that an AsyncAPI message references.
4. **Run Check 3** (binding completeness) by cross-referencing spec scenarios with messages and operations across change and baseline. Skip if no specs.
5. **Run Check 4** (identity & version) on every top-level AsyncAPI document in `$CHANGE_CONTRACTS/messages/`.
6. **Collect findings** and produce the markdown validation report.

## Single-mode output format

When issues are found:

```markdown
## Validation Report (Messaging)

### $ref Resolution
- ✗ contracts/messages/order-events.yaml — $ref "../schemas/missing-type.yaml" does not resolve
- ✓ 8 of 9 $ref pointers resolve

### Message & Schema Metadata
- ✗ contracts/messages/order-events.yaml — message "OrderCancelledMessage" missing "contentType"
- ✗ contracts/schemas/order-placed.yaml — "description" is empty
- ✓ 4 of 6 schemas have complete metadata

### Binding Completeness
- ✓ All spec-referenced payload schemas have AsyncAPI bindings

### Summary
- **Checks passed:** 1 of 3
- **Issues found:** 3
```

When all checks pass:

```markdown
## Validation Report (Messaging)

All checks passed (9 $ref pointers, 6 schemas, 4 channels, 5 operations verified).
```

`single` mode preserves its existing exit semantics: zero on clean reports, non-zero on read errors.

## Cross-project mode

`cross-project` mode runs **after** a producer's contract change merges. The contracts adapter merge brief invokes it as the post-merge baseline gate (the contracts adapter merge contract); `/spec:execute` re-uses the same gate per project after a producer-side merge (the workspace execution contract).

The mode is a thin delegate over `specify tool run contract` (the declared `contract` WASI tool). The verifier sibling does not implement an independent cross-project algorithm — it shells out to the declared WASI tool, exits with the tool's exit code, and lets the caller (the merge brief, or `/spec:execute`) consume the JSON envelope. The deterministic checks the tool enforces are the contract identity/version validation rules:

- `contract.version-is-semver` — every top-level OpenAPI 3.1 / AsyncAPI 3.0 document's `info.version` parses as SemVer (per [semver.org](https://semver.org), prerelease labels included).
- `contract.id-format` — when `info.x-specify-id` is present, the value matches `^[a-z][a-z0-9-]*$` and is ≤ 64 characters.
- `contract.id-unique` — every present `info.x-specify-id` is unique across all top-level contracts under `$BASELINE_CONTRACTS`.

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
    { "path": "contracts/messages/order-events.yaml", "rule-id": "contract.version-is-semver", "detail": "info.version `2024-01-15` is not valid SemVer (must parse per semver.org, including optional prerelease labels)" },
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

When a caller re-surfaces an envelope finding as a `LintFinding` (see `schemas/diagnostics/diagnostic.schema.json` in the CLI repo), the mapping is: `findings[].rule-id` → `rule-id`, `findings[].path` → `location.path`, `target-adapter: contracts`. The contract-domain payload (`findings[].rule-id`, `path`, `detail`, plus any compatibility classification such as `additive` / `breaking` / `ambiguous` / `unverifiable`, channel id, message id) lives inside `evidence.kind: structured` with the contract data under `evidence.data`. The closed `LintFinding` severity enum (`critical` / `important` / `suggestion` / `optional`) is separate from any compatibility classification — classifiers remain contract-domain evidence fields, not severity.

### Exit semantics

| Exit code | Meaning                                                                                                | Caller action                                                                                        |
| --------- | ------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------- |
| `0`       | Clean — no findings.                                                                                   | Proceed to the next merge-brief step (e.g. `specify slice merge run`).                               |
| `1`       | One or more findings.                                                                                  | Record `failure`; halt the merge brief. The slice's deltas remain unmerged.                          |
| `2`       | The tool could not run (resolver, permission, runtime, path missing, not a directory, internal error). | Record `failure`; journal the stderr line or JSON error message. The slice's deltas remain unmerged. |

The mode is **deterministic**: the WASI tool is a thin shell over `specify_validate::validate_baseline_contracts` (in the `specify-cli` workspace). Repeated invocations against the same baseline produce identical findings.

### Why a WASI tool delegate?

The contracts adapter owns merge gating through the declared `contract` WASI tool: a deterministic, adapter-owned gate the merge brief can run through `specify` without crossing the core boundary or re-introducing concern-specific behavior into core crates.

The deterministic baseline check is the canonical post-merge gate.

## Edge cases

### `single` mode

| Scenario                                                   | Behavior                                                                                               |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Change directory has no `contracts/messages/`              | Pass — nothing to verify.                                                                              |
| Baseline has messaging contracts but change does not       | Pass — verifier only checks change-level artefacts.                                                    |
| `$ref` target exists in baseline but not in change         | Pass — baseline is a valid resolution target.                                                          |
| `$ref` target exists in change but not in baseline         | Pass — change-level schemas are valid resolution targets.                                              |
| Mixed resolution: some targets in baseline, some in change | Pass — both directories are valid resolution scope.                                                    |
| No spec files in the slice                                 | Skip Check 3; record the skip in the report.                                                           |
| Schema referenced only via `$ref` from other schemas       | Exempt from Check 3 (shared vocabulary).                                                               |
| Channel uses inline payload (legacy from a manual import)  | `$ref` resolution still verified inside the document; emit `WARN` recommending importer normalisation. |
| Channel declared with no operations                        | Emit `FAIL` — every channel must have at least one `send` or `receive` operation.                      |

### `cross-project` mode

| Scenario                                                   | Behavior                                                                                                                                                    |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `$BASELINE_CONTRACTS` is absent                            | `specify tool run` exits `2` because the declared read preopen is missing or because the validator reports the missing directory. Caller records `failure`. |
| `$BASELINE_CONTRACTS` is empty (no top-level contracts)    | Tool exits `0` with `findings: []`. Treated as clean.                                                                                                       |
| Top-level contract has non-SemVer `info.version`           | Finding `contract.version-is-semver`; exit `1`. Caller records `failure`.                                                                                   |
| Top-level contract has malformed `info.x-specify-id`       | Finding `contract.id-format`; exit `1`.                                                                                                                     |
| Two top-level contracts share the same `info.x-specify-id` | Finding `contract.id-unique` against each colliding path; exit `1`.                                                                                         |
| YAML file under `$BASELINE_CONTRACTS` is malformed         | Skipped by the validator (the format-verifier owns YAML diagnostics in `single` mode); does not surface as a cross-project finding.                         |
| `specify` is not on `$PATH`                                | The shell-out fails with `command not found`; caller records `failure` with the resolve diagnostic on `--context`.                                          |

## Guardrails

- **Read-only.** Never create, modify, or delete files. Both modes share this contract.
- Report every issue with the file path and a description of the problem.
- When uncertain whether a schema is shared vocabulary or a standalone payload, use `WARN` rather than `FAIL` (in `single` mode).
- Do not attempt to fix issues — report them. Repair belongs to the author or importer sibling.
- **`cross-project` mode is fatal.** Treat exit codes `1` (findings) and `2` (invocation error) as `failure` per the contracts adapter merge contract. The merge brief MUST halt; the slice's deltas remain unmerged until the operator resolves the finding.
- Do not re-implement the tool's checks. The verifier sibling's `cross-project` mode is a delegate; the canonical algorithm lives in `crates/validate/src/contracts.rs` of the `specify-cli` workspace.

## Verification checklist

### `single` mode

Before completing the run:

- [ ] All `.yaml` files in `$CHANGE_CONTRACTS/messages/` scanned for `$ref` resolution (cross-file and in-document).
- [ ] All messages in `components/messages` checked for `name`, `contentType`, `payload`.
- [ ] All `.yaml` files in `$CHANGE_CONTRACTS/schemas/` referenced by AsyncAPI messages checked for `$id`, `title`, `description`.
- [ ] Every channel has at least one `send` or `receive` operation.
- [ ] Spec scenarios cross-referenced against AsyncAPI bindings (when specs exist).
- [ ] Shared vocabulary exemption applied correctly.
- [ ] Identity & version (Check 4) enforced on every top-level AsyncAPI document in `$CHANGE_CONTRACTS/messages/`: SemVer `info.version`, kebab-case + ≤64-char `info.x-specify-id` when present, in-change uniqueness on declared ids.
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

- [`../../references/asyncapi-conventions.md`](../../references/asyncapi-conventions.md) — AsyncAPI 3.0 structure rules.
- [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md) — schema metadata rules.
- [`../../references/artifact-structure.md`](../../references/artifact-structure.md) — directory layout for the slice-local delta and the baseline.
- [`../../references/report-shape.md`](../../references/report-shape.md) — single-mode markdown report shape this verifier emits.
- [`../../briefs/merge.md`](../../briefs/merge.md) — merge brief that owns the post-merge `specify tool run contract` invocation and the §Merge and adoption contract three-branch outcome wiring.
- [`author.md`](./author.md) — sibling for spec-driven authoring.
- [`importer.md`](./importer.md) — sibling for normalising external documents.

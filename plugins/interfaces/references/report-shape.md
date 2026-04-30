# Report Shape

Output formats for the format-skill verifiers (`/interfaces:openapi`, `/interfaces:asyncapi`, `/interfaces:json-schema`) and — by convention — the matching alignment / import reports produced by the author and importer paths.

The verifier runs in two modes (see also [`cross-project-compatibility`](cross-project-compatibility.md)):

| Mode | Output format | Caller | Trigger |
|---|---|---|---|
| `single` (default) | Markdown | contracts schema build brief in `/spec:build` | Post-author or post-import; verify-repair loop |
| `cross-project` | Structured YAML | `/spec:execute` post-merge step (RFC-9 §3B) | Producer-side merge of a contract change |

`single` mode is human-readable; the contracts schema build brief drives a verify-repair loop until the report is clean. `cross-project` mode is machine-readable; the execute driver parses `summary.total-findings` to decide whether to render a warning block in the merge transcript.

Both modes share the **read-only** contract — the verifier MUST NOT generate, modify, or delete any files in either mode.

## Severity levels

The severity vocabulary is shared across formats and modes:

| Severity | Markdown glyph | Meaning |
|---|---|---|
| `FAIL` (`error` in YAML) | `✗` | A hard failure. The artefact does not conform; the verify-repair loop must repair before the brief proceeds. |
| `WARN` (`warning` in YAML) | `⚠` | A finding that requires human review. Common in cross-format compatibility checks where the conservative output is "the wire shape changed in a backwards-incompatible direction; the operator should triage." |
| `INFO` (`info` in YAML) | `ℹ` | A neutral observation. Common when the consumer's view matches the producer's update or when the consumer has no prior view. |

Single-mode markdown reports use `FAIL` / `WARN` / `INFO` words plus the corresponding glyph in summary tables. Cross-project YAML reports use lowercase `error` / `warning` / `info` strings in the `severity` field.

## Single-mode output (markdown)

Each format-skill verifier produces a markdown report of the same shape. The check names are format-specific (`$ref Resolution` / `Schema Metadata` / `Binding Completeness` for OpenAPI and AsyncAPI; an extra `Duplicate $id` and `Cross-format Consumer Compatibility` section for JSON Schema), but the structure is identical.

### When issues are found

```markdown
## Validation Report (<Format>)

### <Check 1 name>
- ✗ <file path> — <one-sentence description>
- ✓ <count> of <total> <thing> resolve

### <Check 2 name>
- ✗ <file path> — <one-sentence description>
- ⚠ <file path> — <one-sentence description>
- ✓ <count> of <total> <thing> have <property>

### <Check 3 name>
- ✓ All <thing> verified

### Summary
- **Checks passed:** <N> of <M>
- **Issues found:** <N> (<X> fail, <Y> warn)
```

### When all checks pass

```markdown
## Validation Report (<Format>)

All checks passed (<N> $ref pointers, <N> schemas, <N> bindings verified).
```

### Per-finding format

Each finding is a single bullet with a glyph, a file path (relative to the change directory), and a one-sentence description. Common shapes:

```
FAIL: contracts/http/user-api.yaml — $ref "../schemas/missing-type.yaml" does not resolve (checked change contracts/schemas/ and baseline .specify/contracts/schemas/)
FAIL: contracts/schemas/user-registration.yaml — missing required field "$id"
FAIL: contracts/schemas/error-response.yaml — "description" is empty
WARN: contracts/schemas/oauth-token.yaml — appears in spec but has no protocol binding (may be shared vocabulary — verify intent)
WARN: contracts/schemas/payment.yaml — "$schema" is Draft 7; expected Draft 2020-12 (importer normalisation needed)
```

### Single-mode exit semantics

`single` mode preserves classical exit semantics: zero on a clean report, non-zero on read errors. A clean report with `WARN`-only findings still exits zero — `WARN` is informational for human review, not a blocker. Only `FAIL` findings block the verify-repair loop, but exit code 0 is preserved across the loop iterations because the brief drives repair, not the verifier.

## Cross-project mode output (structured YAML)

The execute driver consumes the cross-project report directly. The schema is stable and machine-readable; format skills must emit the structure verbatim.

### Findings present

```yaml
mode: cross-project
producer:
  contract: contracts/http/user-api.yaml
consumer:
  workspace: .specify/workspace/mobile/
findings:
  - severity: warning
    contract: contracts/http/user-api.yaml
    change-kind: removed-field
    locator: paths./users/{user_id}.get.responses.200.content.application/json.schema.properties.email
    details: >
      The producer's update removes the `email` field from the
      GET /users/{user_id} response body. The consumer's last-known view
      defines this field; consumer code that reads it will receive
      `undefined` after the next workspace sync.
  - severity: warning
    contract: contracts/http/user-api.yaml
    change-kind: required-field-added
    locator: paths./users.post.requestBody.content.application/json.schema.required
    details: >
      Producer adds `phone-number` to the required field list of
      POST /users. Consumer requests built from the prior shape will
      be rejected with HTTP 400.
summary:
  total-findings: 2
  warnings: 2
  errors: 0
```

### No findings

```yaml
mode: cross-project
producer:
  contract: contracts/http/user-api.yaml
consumer:
  workspace: .specify/workspace/mobile/
findings: []
summary:
  total-findings: 0
  warnings: 0
  errors: 0
```

The report is well-formed even when empty — the execute driver always parses `summary.total-findings` to decide whether to render a warning block in the merge transcript.

### Top-level fields

| Field | Type | Description |
|---|---|---|
| `mode` | string (literal `cross-project`) | Disambiguates from single-mode reports for any consumer parsing both. |
| `producer.contract` | path | The producer-side contract path that was compared. Relative to the producer repo root. |
| `consumer.workspace` | path | The tier-2 workspace clone directory under `.specify/workspace/`. |
| `findings` | array | One entry per detected change. Empty array when the consumer's view matches the producer's update or no consumer view was found. |
| `summary.total-findings` | integer | `len(findings)`. The execute driver checks this for the warning-block threshold. |
| `summary.warnings` | integer | Count of `severity: warning` entries. |
| `summary.errors` | integer | Count of `severity: error` entries. Always `0` in `cross-project` mode unless the producer's input is unreadable (which exits non-zero before emitting a report). |

### Per-finding fields

| Field | Type | Description |
|---|---|---|
| `severity` | `error` / `warning` / `info` | See severity table above. |
| `contract` | path | The producer-side contract that contains the change. Same path as `producer.contract` for single-format checks. |
| `change-kind` | string | A value from the [`cross-project-compatibility`](cross-project-compatibility.md) `change-kind` enumeration. |
| `locator` | string | A dot-separated path into the contract document. See §Locator format below. |
| `details` | string (typically multi-line) | Human-prose explanation. Surfaces in the merge transcript and the change's `journal.yaml`. Use `>` (folded scalar) for multi-line entries. |

### Locator format

Locators are dot-separated paths into the contract document, following the format's natural traversal order. They are emitted for human triage, not parsed.

| Format | Path shape |
|---|---|
| OpenAPI request fields | `paths.<path>.<method>.requestBody.content.<media-type>.schema.properties.<field>` |
| OpenAPI response fields | `paths.<path>.<method>.responses.<status>.content.<media-type>.schema.properties.<field>` |
| OpenAPI required-field changes | `paths.<path>.<method>.requestBody.content.<media-type>.schema.required` |
| OpenAPI removed endpoints | `paths.<path>.<method>` |
| AsyncAPI message fields | `channels.<name>.messages.<message-id>.payload.properties.<field>` |
| AsyncAPI removed channels / operations | `channels.<name>` / `operations.<name>` |
| JSON Schema field changes | `properties.<field>` (with nested objects: `properties.<field>.properties.<nested>`) |
| JSON Schema required-list changes | `required` |
| JSON Schema enum changes | `properties.<field>.enum` |
| JSON Schema range changes | `properties.<field>.minimum` (or `maximum`, `exclusiveMinimum`, etc.) |
| JSON Schema file-local sub-types | `$defs.<name>.properties.<field>` |

Path segments containing dots (e.g. `application/json`) are kept verbatim — locators are not parsed by the verifier or the execute driver, only emitted for human triage.

### Cross-project exit semantics

`cross-project` mode exits **0** even when warnings are present. The mode is non-fatal by design (RFC-9 §3B). Exit non-zero only when the inputs are unreadable:

- `cannot-read-producer-contract` — the producer-side contract path does not exist or is not readable.
- `producer-contract-malformed` — the producer-side contract is malformed YAML and cannot be parsed.
- Permission denied / unreachable workspace clone.

In these cases the verifier exits non-zero **before** emitting a YAML report (or with a minimal report carrying the diagnostic), and the execute driver records the failure in the merging change's `journal.yaml`.

## Author / importer report shape

The author and importer paths produce **alignment reports** and **import reports** respectively. These are not verifier outputs — they are markdown documents the format-skill author / importer paths emit alongside the artefact files. The shape is format-specific (each `author.md` and `importer.md` documents its own report), but they follow the same conventions as single-mode verifier reports:

- Markdown headings name the major output categories (`Coverage`, `Generated Delta`, `Manual Review Required`, etc.).
- Each finding is a single bullet with file paths in code spans.
- Glyphs (`✓` / `✗` / `⚠` / `ℹ`) match the severity vocabulary.
- A summary section closes the report with counts.

The author / importer paths always run the verifier afterwards. If the verifier reports issues, the author / importer re-enters its repair steps before finalising the report.

## See also

- [`cross-project-compatibility`](cross-project-compatibility.md) — `change-kind` enumeration used inside cross-project YAML findings.
- [`baseline-vs-delta`](baseline-vs-delta.md) — alignment report structure for the author paths.
- [`import-upgrade-policy`](import-upgrade-policy.md) — import report's "Manual Review Required" section.
- Format-specific verifiers — `plugins/interfaces/skills/{openapi,asyncapi,json-schema}/verifier.md`.

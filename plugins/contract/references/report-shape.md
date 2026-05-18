# Report Shape

Output formats for the format-skill verifiers (`/contract:openapi`, `/contract:asyncapi`, `/contract:json-schema`) and — by convention — the matching alignment / import reports produced by the author and importer paths.

The verifier runs in two modes, but RM-04 compatibility reporting is a CLI surface rather than a format-skill report (see also [`cross-project-compatibility`](cross-project-compatibility.md)):

| Surface | Output format | Caller | Trigger |
|---|---|---|---|
| Format verifier `single` (default) | Markdown | contracts adapter build brief in `/spec:build` | Post-author or post-import; verify-repair loop |
| Format verifier `cross-project` | JSON envelope from `specify tool run contract` | contracts adapter merge brief | Post-merge baseline validation gate |
| `specify compatibility check --change <name> --report-only` | Versioned CLI JSON or text | operator / CI | Read-only producer-to-consumer compatibility classification |

`single` mode is human-readable; the contracts adapter build brief drives a verify-repair loop until the report is clean. Format-verifier `cross-project` mode delegates to the declared `contract` WASI tool and preserves its baseline-validation JSON envelope. The RM-04 compatibility report is produced by the `specify compatibility` CLI family and classifies consumer impact as `additive`, `breaking`, `ambiguous`, or `unverifiable`.

Both modes share the **read-only** contract — the verifier MUST NOT generate, modify, or delete any files in either mode.

## Severity levels

The severity vocabulary is shared across formats and modes:

| Severity | Markdown glyph | Meaning |
|---|---|---|
| `FAIL` (`error` in YAML) | `✗` | A hard failure. The artefact does not conform; the verify-repair loop must repair before the brief proceeds. |
| `WARN` (`warning` in YAML) | `⚠` | A finding that requires human review. Common in cross-format compatibility checks where the conservative output is "the wire shape changed in a backwards-incompatible direction; the operator should triage." |
| `INFO` (`info` in YAML) | `ℹ` | A neutral observation. Common when the consumer's view matches the producer's update or when the consumer has no prior view. |

Single-mode markdown reports use `FAIL` / `WARN` / `INFO` words plus the corresponding glyph in summary tables. The compatibility CLI does not use this severity vocabulary; it uses the RM-04 `classification` field instead.

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

Each finding is a single bullet with a glyph, a file path (relative to the slice directory), and a one-sentence description. Common shapes:

```
FAIL: contracts/http/user-api.yaml — $ref "../schemas/missing-type.yaml" does not resolve (checked change contracts/schemas/ and baseline contracts/schemas/)
FAIL: contracts/schemas/user-registration.yaml — missing required field "$id"
FAIL: contracts/schemas/error-response.yaml — "description" is empty
WARN: contracts/schemas/oauth-token.yaml — appears in spec but has no protocol binding (may be shared vocabulary — verify intent)
WARN: contracts/schemas/payment.yaml — "$schema" is Draft 7; expected Draft 2020-12 (importer normalisation needed)
```

### Single-mode exit semantics

`single` mode preserves classical exit semantics: zero on a clean report, non-zero on read errors. A clean report with `WARN`-only findings still exits zero — `WARN` is informational for human review, not a blocker. Only `FAIL` findings block the verify-repair loop, but exit code 0 is preserved across the loop iterations because the brief drives repair, not the verifier.

## Compatibility report output (CLI JSON)

`specify compatibility check --change <name> --report-only` emits a normal versioned Specify CLI JSON envelope when `--format json` is selected. The bare `specify compatibility check` (with or without `--change`) emits the same payload and exits validation-failed when any finding is `breaking`, `ambiguous`, or `unverifiable`; `--report-only` suppresses that exit code.

### Findings present

```json
{
  "envelope-version": 3,
  "change": "user-api-v2",
  "checked-pairs": 1,
  "ok": false,
  "findings": [
    {
      "classification": "breaking",
      "change-kind": "removed-field",
      "producer-project": "backend",
      "consumer-project": "mobile",
      "producer-contract": "contracts/http/user-api.yaml",
      "consumer-contract": "contracts/http/user-api.yaml",
      "locator": "paths./users.get.responses.200.content.application/json.schema.properties.email",
      "details": "Consumer view defines property `email`, but the producer contract removed it"
    }
  ],
  "summary": {
    "total-findings": 1,
    "additive": 0,
    "breaking": 1,
    "ambiguous": 0,
    "unverifiable": 0
  }
}
```

### No findings

```json
{
  "envelope-version": 3,
  "change": "user-api-v2",
  "checked-pairs": 0,
  "ok": true,
  "findings": [],
  "summary": {
    "total-findings": 0,
    "additive": 0,
    "breaking": 0,
    "ambiguous": 0,
    "unverifiable": 0
  }
}
```

The report is well-formed even when empty.

### Top-level fields

| Field | Type | Description |
|---|---|---|
| `envelope-version` | integer | Standard Specify CLI JSON envelope version. |
| `change` | string | Change name supplied with `--change`; absent when no `--change` flag was passed. |
| `checked-pairs` | integer | Number of producer / consumer contract pairs inspected. |
| `ok` | boolean | `true` iff no `breaking`, `ambiguous`, or `unverifiable` findings are present. |
| `findings` | array | One entry per detected compatibility delta or unverifiable pair. |
| `summary.total-findings` | integer | `len(findings)`. |
| `summary.additive` | integer | Count of `classification: additive` entries. |
| `summary.breaking` | integer | Count of `classification: breaking` entries. |
| `summary.ambiguous` | integer | Count of `classification: ambiguous` entries. |
| `summary.unverifiable` | integer | Count of `classification: unverifiable` entries. |

### Per-finding fields

| Field | Type | Description |
|---|---|---|
| `classification` | `additive` / `breaking` / `ambiguous` / `unverifiable` | RM-04 classification. |
| `change-kind` | string | Optional value from the [`cross-project-compatibility`](cross-project-compatibility.md) `change-kind` enumeration. Present when a stable vocabulary value applies. |
| `producer-project` | string | Registry project that produces the contract. |
| `consumer-project` | string | Registry project that consumes the contract. |
| `producer-contract` | path | Contract path from the producer baseline, relative to the repo root. |
| `consumer-contract` | path | Contract path from the consumer workspace view, relative to the repo root. |
| `locator` | string | A dot-separated path into the contract document. See §Locator format below. |
| `details` | string | Human-prose explanation suitable for terminal, CI, or review output. |

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

### Compatibility exit semantics

`specify compatibility check --report-only` exits `0` when it can render a report, even if the report contains `breaking`, `ambiguous`, or `unverifiable` findings. Without `--report-only`, `specify compatibility check` exits `0` only when `ok: true`; otherwise it exits with the normal Specify validation-failed code.

## Author / importer report shape

The author and importer paths produce **alignment reports** and **import reports** respectively. These are not verifier outputs — they are markdown documents the format-skill author / importer paths emit alongside the artefact files. The shape is format-specific (each `author.md` and `importer.md` documents its own report), but they follow the same conventions as single-mode verifier reports:

- Markdown headings name the major output categories (`Coverage`, `Generated Delta`, `Manual Review Required`, etc.).
- Each finding is a single bullet with file paths in code spans.
- Glyphs (`✓` / `✗` / `⚠` / `ℹ`) match the severity vocabulary.
- A summary section closes the report with counts.

The author / importer paths always run the verifier afterwards. If the verifier reports issues, the author / importer re-enters its repair steps before finalising the report.

## See also

- [`cross-project-compatibility`](cross-project-compatibility.md) — RM-04 classification policy and `change-kind` vocabulary.
- [`baseline-vs-delta`](baseline-vs-delta.md) — alignment report structure for the author paths.
- [`import-upgrade-policy`](import-upgrade-policy.md) — import report's "Manual Review Required" section.
- Format-specific verifiers — `plugins/contract/skills/{openapi,asyncapi,json-schema}/verifier.md`.

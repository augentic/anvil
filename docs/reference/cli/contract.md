# Contract validator (in-guest)

The contracts adapter ships a `contract` validator as in-guest library code inside its published component; there is no host dispatch verb. It walks a baseline `contracts/` directory (typically `<project>/contracts/`), projects every top-level OpenAPI 3.1 / AsyncAPI 3.0 document, and enforces the contract validation rules. It is read-only and never modifies files. The contracts build and merge orchestrations invoke it directly; the JSON envelope below is the canonical output shape.

## Validation Rules

| Rule id | Field | Constraint |
|---|---|---|
| `contract.version-is-semver` | `info.version` | MUST parse as SemVer per [semver.org](https://semver.org), including optional prerelease labels (`1.0.0-draft.1`). Bump rules remain skill-side judgement; the validator only checks that the value parses. |
| `contract.id-format` | `info.x-specify-id` | When present, the value MUST match `^[a-z][a-z0-9-]*$` and be ≤ 64 characters. |
| `contract.id-unique` | `info.x-specify-id` | When two or more top-level contracts both set `info.x-specify-id`, the values MUST be distinct across the walked directory. Both colliding paths are reported. |

Format detection: a YAML file is top-level iff its root carries `openapi:` or `asyncapi:`. Standalone JSON Schemas under `<BASELINE_DIR>/schemas/` are payload vocabulary and are skipped by the same filter.

## Exit Codes

| Code | Meaning |
|---|---|
| `0` | Clean — no findings. The baseline is well-formed under all three rules. |
| `1` | One or more findings. Caller (typically the contracts merge brief) MUST treat as `failure`. |
| `2` | The validator could not run or rejected its invocation. Caller MUST treat as `failure`. |

The `0` / `1` / `2` mapping is the conventional shell-friendly shape so adapter skills can branch on the exit code without needing the broader `specify` `Exit` taxonomy. For normal validator runs, the JSON envelope's `"exit-code"` field reflects the same value.

## JSON Envelope

The validator envelope is emitted directly, unwrapped:

```json
{
  "envelope-version": 6,
  "contracts-dir": "<absolute-baseline-path>",
  "ok": false,
  "findings": [
    {
      "path": "contracts/http/user-api.yaml",
      "rule-id": "contract.version-is-semver",
      "detail": "info.version `2024-01-15` is not valid SemVer (must parse per semver.org, including optional prerelease labels)"
    }
  ],
  "exit-code": 1
}
```

Field semantics:

- `envelope-version` — currently `6`, matching the CLI JSON envelope version.
- `contracts-dir` — the absolute path the tool walked, echoing the positional argument.
- `ok` — `true` iff `findings` is empty.
- `findings[].path` — repo-relative when the parent of `<BASELINE_DIR>` matches the path's prefix, otherwise absolute. Suitable for verbatim rendering.
- `findings[].rule-id` — one of `contract.version-is-semver`, `contract.id-format`, `contract.id-unique`.
- `findings[].detail` — single-line human-readable description.
- `exit-code` — mirrors the validator's process-style exit code.

This tool is the baseline-validation gate only. It does not compare producer contracts against consumer workspace views; that product surface is deferred until a real consumer workflow exists.

## Distribution

The validator is library code inside the contracts adapter's published component ([`targets/contracts/`](https://github.com/augentic/specify-adapters/tree/main/targets/contracts)). Operators install `specify`; no separate contract-validator binary exists.

## See Also

- [`adapters/targets/contracts/prose/briefs/build.md`](https://github.com/augentic/specify-adapters/blob/main/targets/contracts/prose/briefs/build.md) — the contracts target build brief whose OpenAPI, AsyncAPI, and JSON Schema sub-flows produce the artefacts this tool inspects.
- [Configuration Files → contracts/](../configuration.md) — the baseline directory layout.
- [`adapters/targets/contracts/prose/briefs/merge.md`](https://github.com/augentic/specify-adapters/blob/main/targets/contracts/prose/briefs/merge.md) — merge brief that owns the post-merge invocation and the three-branch merge outcome wiring.

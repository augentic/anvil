# Contract validator (WASI tool)

The contracts adapter declares a `contract` WASI tool that walks a baseline `contracts/` directory, projects every top-level OpenAPI 3.1 / AsyncAPI 3.0 document, and enforces the contract validation rules. It is read-only and never modifies files.

The legacy in-binary `specify contract` family has been retired. The canonical user-visible merge gate is the declared WASI tool:

```bash
specify tool run contract -- <BASELINE_DIR> [--format text|json]
```

`<BASELINE_DIR>` is typically `<project>/contracts/`. `--format json` is the canonical output shape for briefs and skills; `--format text` is a human-readable variant for local debugging.

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
| `2` | The tool could not run, either because `specify` could not resolve/instantiate it or because the validator rejected its invocation. Caller MUST treat as `failure`. |

The `0` / `1` / `2` mapping is the conventional shell-friendly shape so adapter skills can branch on the exit code without needing the broader `specify` `Exit` taxonomy. For normal validator runs, the JSON envelope's `"exit-code"` field reflects the same value.

## JSON Envelope

`--format json` writes the validator envelope directly to stdout. `specify tool run` does not wrap successful guest output:

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

Resolver, permission, or runtime failures come from `specify tool run` and use the standard Specify error envelope.

This tool is the baseline-validation gate only. It does not compare producer contracts against consumer workspace views. Use `specify compatibility check` (optionally `--change <name>` and `--report-only`) for RM-04 consumer-impact classification.

## Distribution

The contracts adapter ships `adapters/contracts/tools.yaml`, a sidecar declaration next to `adapter.yaml`. That sidecar declares the exact `specify:contract@0.3.0` package request; the CLI derives the tool name and applies the embedded read-only permission on `$PROJECT_DIR/contracts`.

Operators install `specify`; no separate contract-validator binary is required for the canonical path. `specify tool run contract` resolves and caches the WASI component through wasm-pkg package metadata, applies the filesystem preopen, and runs it through the embedded WASI host.

During local development, project authors may override the adapter declaration with a project-scope object declaration whose `file://` source points at a locally built `contract.wasm`.

## See Also

- [specify tool](tool.md) — the declared WASI tool runner surface.
- [Tool declarations](../../explanation/tool-declarations.md) — project and adapter declaration sites, precedence, cache, permissions, and digest pins.
- [Contract plugin](../plugins/contract.md) — the per-slice `/contract:openapi`, `/contract:asyncapi`, and `/contract:json-schema` skills that produce the artefacts this tool inspects.
- [Configuration Files → contracts/](../configuration.md) — the baseline directory layout.
- [`targets/contracts/briefs/merge.md`](../../../targets/contracts/briefs/merge.md) — merge brief that owns the post-merge invocation and the three-branch merge outcome wiring.

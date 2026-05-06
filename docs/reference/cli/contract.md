# specify-contract-validate

Standalone validator for the contracts capability. Walks a baseline `contracts/` directory, projects every top-level OpenAPI 3.1 / AsyncAPI 3.0 document, and enforces the RFC-12 §Validation rules. Read-only; never modifies files. (RFC-13 §"Merge and adoption contract" + chunk 4.2a.)

`specify-contract-validate` is a *capability-owned* binary, not a `specify` subcommand. The legacy in-binary `specify contract` family was retired in chunk 2.7 of the RFC-13 landing — domain-specific validation behavior left core when contracts became a first-party capability. The standalone binary is the post-merge gate the contracts capability skills shell out to (see [`capabilities/contracts/briefs/merge.md`](../../../capabilities/contracts/briefs/merge.md) for the wiring).

## Synopsis

```bash
specify-contract-validate <BASELINE_DIR> [--format text|json]
```

- `<BASELINE_DIR>` — path to the contracts baseline directory (typically `<project>/contracts/`). Required positional.
- `--format` — output shape. `json` (default) emits a single JSON envelope on stdout; `text` emits a human-readable summary on stdout with finding lines on stderr. Both shapes produce the same exit code.

## Validation rules

| Rule id | Field | Constraint |
|---|---|---|
| `contract.version-is-semver` | `info.version` | MUST parse as SemVer per [semver.org](https://semver.org), including optional prerelease labels (`1.0.0-draft.1`). Bump rules (when to advance major / minor / patch) remain skill-side judgement; the validator only checks that the value parses. |
| `contract.id-format` | `info.x-specify-id` | When present, the value MUST match `^[a-z][a-z0-9-]*$` and be ≤ 64 characters. |
| `contract.id-unique` | `info.x-specify-id` | When two or more top-level contracts both set `info.x-specify-id`, the values MUST be distinct across the walked directory. Both colliding paths are reported. |

Format detection follows RFC-12 §"Top-level contracts": a YAML file is top-level iff its root carries `openapi:` or `asyncapi:`. Standalone JSON Schemas under `<BASELINE_DIR>/schemas/` are payload vocabulary and are skipped by the same filter.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Clean — no findings. The baseline is well-formed under all three rules. |
| `1` | One or more findings. Caller (typically the contracts merge brief) MUST treat as `failure`. |
| `2` | Validator could not run (path missing, not a directory, internal error). Caller MUST treat as `failure`. |

The `0` / `1` / `2` mapping is the conventional shell-friendly shape so capability skills can branch on the exit code without needing the broader `specify` `CliResult` taxonomy. The JSON envelope's `"exit-code"` field reflects the same value.

## JSON envelope

`--format json` writes a single object to stdout:

```json
{
  "schema-version": 2,
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

- `schema-version` — currently `2`; bumps follow RFC-12.
- `contracts-dir` — the absolute path the binary walked, echoing the positional argument.
- `ok` — `true` iff `findings` is empty.
- `findings[].path` — repo-relative when the parent of `<BASELINE_DIR>` matches the path's prefix, otherwise absolute. Suitable for verbatim rendering.
- `findings[].rule-id` — one of `contract.version-is-semver`, `contract.id-format`, `contract.id-unique`.
- `findings[].detail` — single-line human-readable description.
- `exit-code` — mirrors the process exit code.

The shape is byte-for-byte identical to the envelope the retired in-binary contract validator emitted before chunk 2.7; downstream consumers that parsed that legacy shape continue to work unchanged against this binary.

## Distribution

The binary ships alongside `specify` in the `specify-cli` workspace. Operators install it via the same channels:

```bash
brew install augentic/tap/specify           # macOS + Linux (primary)
cargo install specify                        # any platform with Rust toolchain
curl -sSfL https://specify.sh/install.sh | sh   # pre-built binary (installs the workspace's binaries)
```

CI and platform runtimes that consume capability skills should pre-install the same package; capability skills that shell out to `specify-contract-validate` assume it is on `$PATH`.

## See also

- [Contract plugin](../plugins/contract.md) — the per-slice `/contract:openapi`, `/contract:asyncapi`, and `/contract:json-schema` skills that produce the artefacts this binary inspects.
- [Configuration Files → contracts/](../configuration.md) — the baseline directory layout.
- [`capabilities/contracts/briefs/merge.md`](../../../capabilities/contracts/briefs/merge.md) — merge brief that owns the post-merge invocation and the §Merge and adoption contract three-branch outcome wiring.
- RFC-12 (archived) — original specification of the SemVer / id-format / id-unique rules.
- RFC-13 §"Merge and adoption contract" — capability adoption protocol the contracts capability follows.

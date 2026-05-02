# specify contract

Inspect and validate the platform's baseline contracts under the project's `contracts/` directory. Read-only; never modifies files. (RFC-12 §"CLI surface".)

The verbs are the CLI counterpart to the per-change `/contract:*` skills: skills produce contract artefacts inside a change, and these verbs project / validate the merged baseline once the change has landed in `.specify/contracts/`.

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`list`](#specify-contract-list) | Project every top-level OpenAPI / AsyncAPI document under `contracts/` as `(file, format, info.title, info.version, info.x-specify-id)`. |
| [`validate`](#specify-contract-validate) | Run the RFC-12 §Validation checks across `contracts/` (SemVer `info.version`; kebab-case ≤64-char `info.x-specify-id` when present; cross-repo id uniqueness). |

Both verbs no-op with exit 0 when `contracts/` is absent — matching `specify registry validate`'s posture for absent registries.

## Subcommands

### specify contract list

Project every top-level contract under `contracts/`.

```bash
specify contract list [--format json]
```

Format detection per RFC-12 §"Top-level contracts": a YAML file is top-level iff its root carries `openapi:` or `asyncapi:`. Standalone JSON Schemas under `contracts/schemas/` are payload vocabulary and are skipped.

Each row reports `(path, format, info.title, info.version, info.x-specify-id)`. `info.x-specify-id` renders as `null` in JSON when absent.

### specify contract validate

Run the RFC-12 §Validation checks across `contracts/`.

```bash
specify contract validate [--format json]
```

Three rules:

1. `contract.version-is-semver` — `info.version` MUST parse as SemVer per [semver.org](https://semver.org), including optional prerelease labels (`1.0.0-draft.1`). Bump rules (when to advance major / minor / patch) remain skill-side judgement; the validator only checks that the value parses.
2. `contract.id-format` — when `info.x-specify-id` is present, the value MUST match `^[a-z][a-z0-9-]*$` and be ≤ 64 characters.
3. `contract.id-unique` — when two or more top-level contracts both set `info.x-specify-id`, the values MUST be distinct across the repo.

Exits `CliResult::ValidationFailed` (`2`) on any finding. Findings carry the relative file path, the rule id, and a human-readable detail.

## See also

- [Contract plugin](../plugins/contract.md) — the per-change `/contract:openapi`, `/contract:asyncapi`, and `/contract:json-schema` skills that produce the artefacts these verbs inspect.
- [Configuration Files → contracts/](../configuration.md) — the baseline directory layout.
- RFC-12 (archived) — original specification of the SemVer / id-format / id-unique rules.

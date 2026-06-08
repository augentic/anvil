---
id: CORE-055
title: Framework Authoring Config Schema
severity: critical
trigger: Root `Specify.toml` fails to validate against the framework authoring config schema (missing `[cli]`, unknown keys, or malformed `version` / `binary` / `path` values).
rule_hints:
  - kind: path-pattern
    value: Specify.toml
    description: Narrow the candidate set to the framework authoring config before schema validation.
  - kind: schema
    value: framework
    description: Validate Specify.toml against the embedded `framework.schema.json` shape (`cli.version` ∈ {next, latest, X.Y.Z}, required `cli.binary`, optional `cli.path`).
---

## Rule

The framework repo carries a single authoring blueprint at `Specify.toml` that declares how `make lint` binds the `specify` binary. The file is distinct from runtime `.specify/project.yaml` and must match the closed schema shipped from `specify-cli` under `schemas/authoring/framework.schema.json`.

`framework.schema.json` pins the closed shape:

- `cli.version` — `next`, `latest`, or semver `X.Y.Z`.
- `cli.binary` — non-empty repo-local executable path (materialized by `scripts/specify.sh`).
- `cli.path` — optional directory for `make install-specify` PATH symlinks.

## Look For

- A missing `Specify.toml` at the repo root (presence is enforced elsewhere once the file is required).
- A `[cli]` table missing `version` or `binary`.
- A `version` value outside the closed `{next, latest, X.Y.Z}` set.
- Extra top-level or `[cli]` keys not declared by the schema.

## Fix

Open `Specify.toml`, compare it against the schema fields listed above, and align missing or malformed keys with the closed enums and patterns. The schema is the canonical authority — `scripts/specify.sh`, CI, and `make install-specify` all read the same `[cli]` contract.

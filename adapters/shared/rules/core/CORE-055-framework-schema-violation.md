---
id: CORE-055
title: Framework Authoring Config Schema
severity: critical
trigger: Root `Specify.toml` fails to validate against the framework authoring config schema (missing `cli`, an unknown form, or a `cli` source spec that is not exactly one of `{ version }` / `{ git }` / `{ git, rev|branch|tag }` / `{ path }`).
rule_hints:
  - kind: path-pattern
    value: Specify.toml
    description: Narrow the candidate set to the framework authoring config before schema validation.
  - kind: schema
    value: framework
    description: Validate Specify.toml against the embedded `framework.schema.json` shape (`cli` is a `oneOf` over `{ version }`, `{ git }`, `{ git, rev|branch|tag }`, and `{ path }`).
---

## Rule

The framework repo carries a single authoring blueprint at `Specify.toml` that declares which `specify-cli` **source** `make lint` builds. The file is distinct from runtime `.specify/project.yaml` and must match the closed schema shipped from `specify-cli` under `schemas/authoring/framework.schema.json`.

`cli` is a Cargo-shaped inline-table **source spec** — never a published binary, channel, or crates.io range — taking exactly one of three forms:

- `cli = { version = "X.Y.Z" }` — an exact `specify-cli` release; builds git tag `vX.Y.Z`. `version` is pinned to `^\d+\.\d+\.\d+$` (no `next` / `latest`, no caret ranges).
- `cli = { git = "<url>" }` — the default remote; builds branch `main` when no ref is given.
- `cli = { git = "<url>", rev|branch|tag = "…" }` — a git ref; `git` plus exactly one of `rev` / `branch` / `tag`.
- `cli = { path = "<dir>" }` — a local checkout, built in place. Belongs in a gitignored `Specify.local.toml` overlay, not the committed file.

## Look For

- A missing `Specify.toml` at the repo root (presence is enforced elsewhere once the file is required).
- A `cli` value that is not an inline table, or that matches none of the three forms.
- A `version` outside `^\d+\.\d+\.\d+$` (e.g. `next`, `latest`, `^0.2`).
- A `git` form with more than one of `rev` / `branch` / `tag`, or missing `git`.
- Extra top-level keys, or extra keys inside the `cli` table (the schema is closed).

## Fix

Open `Specify.toml`, compare it against the three forms above, and align it to exactly one. The committed `cli` must always be a **fetchable** form (`version` or `git` + ref) so CI and clean clones build the same source; use a gitignored `Specify.local.toml` `cli = { path = … }` for local co-development. The schema is the canonical authority — `scripts/specify.rs`, CI, and `make install-cli` all read the same `cli` contract.

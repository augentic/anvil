# `plan.schema.json`

Canonical JSON Schema (2020-12) for `.specify/plan.yaml` — the initiative plan described in [RFC-2](../../rfcs/archive/rfc-2-execution.md) §"The Plan".

## What it validates

- Top-level `name` (kebab-case) and `changes` (ordered list) are required.
- Optional top-level `sources` map (kebab-case keys → path-or-URL values).
- Each change carries a required kebab-case `name`, a required `status` drawn from `{pending, in-progress, done, blocked, failed, skipped}`, plus optional `project`, `depends-on`, `sources`, `description`, and `status-reason` fields. `project` (RFC-3b) is the target registry project name — required for multi-project registries, optional for single-project.
- `additionalProperties: false` everywhere — unknown fields are a hard error. The schema is strict; additive optional fields (like `project`) can be introduced without a version bump.

Scope and delta-targeting intent are carried in the `description` field as prose. The define skill infers extract filters and baseline targets from the description at execution time. The former `scope` and `affects` structured fields (introduced by RFC-3a) have been removed in favour of this description-driven approach.

Semantic checks (cycle detection, referential integrity of `depends-on` / `sources` targets, at-most-one `in-progress`, etc.) are performed by `Plan::validate` in [`specify-change`](https://github.com/augentic/specify-cli/tree/main/crates/change); this schema covers shape only.

The JSON response produced by `specify plan validate --format json` (both the shape checks above and the semantic ones) is itself covered by a sibling schema at [`../plan-validate-output/schema.json`](../plan-validate-output/schema.json); skill authors consuming the validator should match the response against that schema.

## Editor integration

Add the following header to `.specify/plan.yaml` to opt in to autocomplete and diagnostics in editors with `yaml-language-server` support (VS Code, Helix, Neovim, Zed):

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/augentic/specify/main/schemas/plan/plan.schema.json
```

Pin to a commit or tag by replacing `main` with the desired ref.

## Mirror

A byte-identical copy lives at [`augentic/specify-cli/schemas/plan/plan.schema.json`](https://github.com/augentic/specify-cli/tree/main/schemas/plan/plan.schema.json) and is shipped with the CLI for embedded validation. When you edit the canonical file here, mirror the change to `specify-cli` in the same commit pair.

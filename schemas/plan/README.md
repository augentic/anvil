# `plan.schema.json`

Canonical JSON Schema (2020-12) for `.specify/plan.yaml` — the initiative plan
described in [RFC-2](../../rfcs/archive/rfc-2-execution.md) §"The Plan".

## What it validates

- Top-level `name` (kebab-case) and `changes` (ordered list) are required.
- Optional top-level `sources` map (kebab-case keys → path-or-URL values).
- Each change carries a required kebab-case `name`, a required `status`
  drawn from `{pending, in-progress, done, blocked, failed, skipped}`, plus
  optional `depends-on`, `affects`, `sources`, `description`, and
  `status-reason` fields.
- `additionalProperties: false` everywhere — unknown fields are a hard error.
  The schema is strict, not forward-extensible; new fields land via schema
  version bumps.

Semantic checks (cycle detection, referential integrity of `depends-on` /
`affects` / `sources` targets, at-most-one `in-progress`, etc.) are performed
by `Plan::validate` in [`specify-change`](https://github.com/augentic/specify-cli/tree/main/crates/change);
this schema covers shape only.

The JSON response produced by `specify initiative validate --format json` (both
the shape checks above and the semantic ones) is itself covered by a
sibling schema at
[`../plan-validate-output/schema.json`](../plan-validate-output/schema.json);
skill authors consuming the validator should match the response against
that schema.

## Editor integration

Add the following header to `.specify/plan.yaml` to opt in to autocomplete and
diagnostics in editors with `yaml-language-server` support (VS Code,
Helix, Neovim, Zed):

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/augentic/specify/main/schemas/plan/plan.schema.json
```

Pin to a commit or tag by replacing `main` with the desired ref.

## Mirror

A byte-identical copy lives at
[`augentic/specify-cli/schemas/plan/plan.schema.json`](https://github.com/augentic/specify-cli/tree/main/schemas/plan/plan.schema.json)
and is shipped with the CLI for embedded validation. When you edit the
canonical file here, mirror the change to `specify-cli` in the same commit pair.

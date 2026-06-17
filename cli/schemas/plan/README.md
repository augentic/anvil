# `plan.schema.json`

Canonical JSON Schema (2020-12) for `plan.yaml` (at the repo root).

## What it validates

- Top-level `name` (kebab-case) and `slices` (ordered list) are required.
- Optional top-level `lifecycle` enum — `pending | approved` per workflow §Workflow vocabulary. Two stored states only; `drained` and `currently executing` are computed from per-entry `status` at read time.
- Optional top-level `sources` map (kebab-case keys to path-or-URL values, or the structured `{ adapter, path?, value? }` object form).
- Each slice carries a required kebab-case `name`, a required `status` drawn from `{pending, in-progress, done}` (per the workflow contract the collapsed three-state per-entry enum — v1 has no `blocked`/`failed`/`skipped`), plus optional `project`, `target`, `depends-on`, `sources`, `context`, `description`, and `divergence` fields.
- `additionalProperties: false` everywhere; unknown fields are a hard error.

Scope and delta-targeting intent are carried in the `description` and `context` fields. The define skill infers extract filters and baseline targets from those fields at execution time.

Semantic checks (cycle detection, referential integrity of `depends-on` / `sources` targets, at-most-one `in-progress`, registry project checks, etc.) are performed by the CLI; this schema covers shape only.

The JSON response produced by `specify plan validate --format json` is the neutral diagnostic envelope shared by every Specify check surface — `{ version, summary, findings }` — validated by [`../diagnostics/diagnostic-report.schema.json`](../diagnostics/diagnostic-report.schema.json) (each finding is a [`../diagnostics/diagnostic.schema.json`](../diagnostics/diagnostic.schema.json)). Skill authors consuming the validator should match the response against those schemas and branch on the exit code (`0` clean, `2` when a blocking finding is present) rather than a bespoke `passed` flag. The structural check codes (`duplicate-name`, `cycle-in-depends-on`, `orphan-source`, `stale-workspace-clone`, `topology-cache-stale`, …) surface as each finding's `rule-id`; the health checks carry their machine-readable payload on the finding's `evidence` (`kind: structured`).

## Editor integration

Add the following header to `plan.yaml` to opt in to autocomplete and diagnostics in editors with `yaml-language-server` support:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/augentic/specify-cli/main/schemas/plan/plan.schema.json
```

Pin to a commit or tag by replacing `main` with the desired ref.

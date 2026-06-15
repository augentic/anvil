---
id: CORE-004
title: Adapter Briefs Cover Operations
severity: important
trigger: "An `adapters/{sources,targets}/<name>/adapter.yaml` declares a `briefs:` map whose key set does not cover every operation the axis requires (`survey` + `extract` for source adapters; `shape` + `build` + `merge` for target adapters), leaving the loader without a brief to dispatch for at least one declared operation."
rule_hints:
  - kind: path-pattern
    value: "adapters/sources/*/adapter.yaml"
    description: Source adapter manifests; `briefs.keys()` must cover `survey` and `extract`.
  - kind: path-pattern
    value: "adapters/targets/*/adapter.yaml"
    description: Target adapter manifests; `briefs.keys()` must cover `shape`, `build`, and `merge`.
  - kind: set-coverage
    value: adapter-briefs
    config:
      expected-operations:
        sources: [survey, extract]
        targets: [shape, build, merge]
    description: For each `AdapterManifest` fact in the candidate set, assert that `briefs.keys()` covers the per-axis operation set the rule supplies in `config`. One finding per missing `(adapter, operation)` pair; extras are silent (the default `subset` mode — `CORE-007`'s exact mode tightens that).
---

## Rule

Every `adapters/{sources,targets}/<name>/adapter.yaml` declares its operation dispatch through the `briefs:` map. The set of keys in that map must cover the closed axis-appropriate operation enum: `SourceOperation::{Survey, Extract}` for source adapters, `TargetOperation::{Shape, Build, Merge}` for target adapters. A missing key leaves the workflow loader without a brief to dispatch when the per-axis verb fires (`specify source resolve survey <name>`, `specify target resolve build <name>`, …).

The deterministic-hint interpreter consumes the `AdapterManifest` facts the framework-profile indexer already produced (`crates/standards/src/lint/index/adapter.rs::extract`, including the new `brief-keys` field that mirrors the manifest's `briefs:` map keys verbatim), so the rule cost is one set-difference per candidate manifest at lint time. The path scope intentionally pins the canonical `adapters/{sources,targets}/<name>/adapter.yaml` shape; nested `adapter.yaml` files (e.g. inside `briefs/` subtrees) are dropped upstream by the extractor and never reach this layer.

`set-coverage` defaults to one-sided (`mode: subset`): extras (`briefs.keys()` values not in the expected operation set) stay silent here. The JSON schema in `source.schema.json` / `target.schema.json` already rejects unknown keys via `additionalProperties: false`; [`CORE-007`](CORE-007-adapter-briefs-equal-operations.md) carries the same `set-coverage` kind with `config: { mode: exact }` to tighten the check to both sides.

## Look For

- A newly added source adapter whose `briefs:` map declares `survey:` but forgets `extract:` (or vice versa).
- A target adapter scaffolded with only `shape:` and `build:`, missing the `merge:` brief that the lifecycle's landing gate dispatches against.
- A refactor that renames a brief key (`shape:` → `define:`) and leaves the original axis operation uncovered.

## Fix

Add the missing brief key to the manifest's `briefs:` map and create the matching `briefs/<operation>.md` file under the adapter directory. The JSON schema check (`CORE-001` ≅ `adapter.schema`) will also flag the missing required key from the schema's `required: […]` list — `CORE-004` complements it by attributing the failure to the specific axis operation (`survey`, `shape`, …) rather than the generic schema error envelope.

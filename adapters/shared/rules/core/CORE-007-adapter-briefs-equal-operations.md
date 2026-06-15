---
id: CORE-007
title: Adapter Briefs Equal Operations
severity: important
trigger: "An `adapters/{sources,targets}/<name>/adapter.yaml` declares a `briefs:` map whose key set is not exactly the closed axis-appropriate operation enum (`survey` + `extract` for source adapters; `shape` + `build` + `merge` for target adapters), either omitting a required operation or carrying an unexpected key the loader has no operation to dispatch."
rule_hints:
  - kind: path-pattern
    value: "adapters/sources/*/adapter.yaml"
    description: Source adapter manifests; `briefs.keys()` must equal exactly `survey` and `extract`.
  - kind: path-pattern
    value: "adapters/targets/*/adapter.yaml"
    description: Target adapter manifests; `briefs.keys()` must equal exactly `shape`, `build`, and `merge`.
  - kind: set-coverage
    value: adapter-briefs
    config:
      mode: exact
      expected-operations:
        sources: [survey, extract]
        targets: [shape, build, merge]
    description: For each `AdapterManifest` fact in the candidate set, assert that `briefs.keys()` is exactly the per-axis operation set the rule supplies in `config` (the `exact` mode tightens the default one-sided `subset` comparison to two-sided). One finding per divergence — `missing` for an absent required operation, `unexpected` for a stray key.
---

## Rule

Every `adapters/{sources,targets}/<name>/adapter.yaml` declares its operation dispatch through the `briefs:` map. The set of keys in that map must equal — exactly — the closed axis-appropriate operation enum: `SourceOperation::{Survey, Extract}` for source adapters, `TargetOperation::{Shape, Build, Merge}` for target adapters. A missing key leaves the workflow loader without a brief to dispatch when the per-axis verb fires; a stray key declares a brief the loader will never reach, masking a typo or an abandoned operation rename.

Where [`CORE-004`](CORE-004-adapter-briefs-cover-operations.md) (`set-coverage`) is one-sided — it flags only operations the manifest fails to cover — this rule is the two-sided tightening: it fires on both halves of the symmetric difference. The deterministic-hint interpreter consumes the `AdapterManifest` facts the framework-profile indexer already produced (including the `brief-keys` field that mirrors the manifest's `briefs:` map keys verbatim), so the rule cost is one set-comparison per candidate manifest at lint time. The path scope intentionally pins the canonical `adapters/{sources,targets}/<name>/adapter.yaml` shape; nested `adapter.yaml` files (for example inside `briefs/` subtrees) are dropped upstream by the extractor and never reach this layer.

This rule overlaps two neighbours by design and the overlap is harmless — distinct findings dedupe through the shared fingerprint algorithm. The `missing` half overlaps [`CORE-004`](CORE-004-adapter-briefs-cover-operations.md) and the `required` list enforced by [`CORE-001`](CORE-001-adapter-schema.md) (`adapter.schema`); the `unexpected` half overlaps the `additionalProperties: false` clause the same schema enforces. The value this rule adds is attribution: a stray brief key is named as the specific `unexpected` operation rather than surfacing only inside the generic schema error envelope. Every adapter manifest in the framework repo already declares exactly its axis operation set, so the rule fires zero findings against the current tree and surfaces only on drift.

## Look For

- A source adapter whose `briefs:` map declares `survey:` but forgets `extract:` (or vice versa) — surfaces as a `missing` divergence.
- A target adapter scaffolded with `shape:` and `build:` plus a leftover `define:` key after a partial rename — surfaces as a `missing` `merge:` and an `unexpected` `define:` in the same manifest.
- A copy-pasted manifest carrying a brief key from another adapter's vocabulary that has no matching axis operation — surfaces as an `unexpected` divergence.

## Fix

For a `missing` divergence, add the operation key to the manifest's `briefs:` map and create the matching `briefs/<operation>.md` file under the adapter directory. For an `unexpected` divergence, remove the stray key (and its brief file) or rename it to the intended axis operation. The closed operation enum is fixed per axis; widening it is a coordinated CLI change, not a per-manifest edit.

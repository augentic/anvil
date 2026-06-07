---
id: CORE-010
title: Adapter Missing Manifest
severity: important
trigger: An adapter directory under adapters/sources or adapters/targets lacks adapter.yaml.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-010-adapter-missing-manifest.md
    description: Sentinel include so the rule carries a candidate set; the `cross-reference` join evaluates the whole adapter-dir / adapter-manifest fact families and ignores the candidate set.
  - kind: cross-reference
    value: adapter-dir
    config:
      target: adapter-manifest
    description: Join the `adapter-dir` fact family (every immediate child of adapters/sources and adapters/targets) against the `adapter-manifest` family on the manifest's containing directory; flag any adapter directory with no resolvable adapter.yaml. The source and target family selectors are policy carried here, not in the engine.
---

## Rule

Every adapter directory under `adapters/sources/` and `adapters/targets/` ships an `adapter.yaml` manifest. The loader resolves an adapter by reading that manifest, so a directory with no `adapter.yaml` is an orphan the loader cannot bind.

This check is whole-tree and relational: the `kind: cross-reference` hint with `value: adapter-dir` and `config: { target: adapter-manifest }` joins the adapter-directory fact family (one fact per immediate child of `adapters/{sources,targets}`) against the adapter-manifest fact family, keyed on the manifest's containing directory, and flags any adapter directory that has no corresponding manifest. The rule's `path-pattern` is a sentinel include; the cross-reference join evaluates the whole fact families regardless of the candidate set.

## Look For

- A directory directly under `adapters/sources/` or `adapters/targets/` with no `adapter.yaml`.

## Fix

Add an `adapter.yaml` manifest to the adapter directory, or remove the stray directory if it is not an adapter.

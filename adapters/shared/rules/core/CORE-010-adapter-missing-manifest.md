---
id: CORE-010
title: Adapter Missing Manifest
severity: important
trigger: An adapter directory under adapters/sources or adapters/targets lacks adapter.yaml.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-010-adapter-missing-manifest.md
    description: Sentinel path so the whole-tree adapter tool runs exactly once; the tool walks PROJECT_DIR/adapters itself rather than the passed candidate.
  - kind: tool
    value: adapter
    description: Run the `adapter` framework checker, which flags any adapter directory under adapters/sources or adapters/targets that has no adapter.yaml manifest. Cross-fact presence check; carries no policy.
---

## Rule

Every adapter directory under `adapters/sources/` and `adapters/targets/` ships an `adapter.yaml` manifest. The loader resolves an adapter by reading that manifest, so a directory with no `adapter.yaml` is an orphan the loader cannot bind.

This check is whole-tree and cross-fact: the manifest-fact passes only see present-but-incomplete manifests, so an axis directory missing its `adapter.yaml` is invisible to them. The `adapter` framework tool discovers every immediate directory under `adapters/{sources,targets}` and flags those with no manifest. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- A directory directly under `adapters/sources/` or `adapters/targets/` with no `adapter.yaml`.

## Fix

Add an `adapter.yaml` manifest to the adapter directory, or remove the stray directory if it is not an adapter.

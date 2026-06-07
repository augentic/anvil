---
id: CORE-049
title: Tools Invalid Declaration
severity: important
trigger: A first-party WASI tool declaration in a target adapter manifest is missing or version-mismatched.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-049-tools-invalid-declaration.md
    description: Sentinel path so the whole-tree adapter tool runs exactly once; the tool walks PROJECT_DIR/adapters itself rather than the passed candidate.
  - kind: tool
    value: adapter
    config:
      expected-tools:
        - adapter: contracts
          tool: contract
          package: specify:contract@0.3.0
        - adapter: vectis
          tool: vectis
          package: specify:vectis@0.4.0
    description: Run the `adapter` framework checker, which validates each target adapter's first-party WASI tool declarations against the pinned policy table. The {adapter, tool, package} table is policy carried here, not in the tool.
---

## Rule

Each first-party WASI tool a target adapter ships must be declared under that adapter's `adapter.yaml` `tools[]` with the exact pinned package request. The `{adapter, tool, package}` policy table lives in this rule's `config:` so the version pins are owned by the framework, not baked into the checker. The tool reads the table from the forwarded config; the engine only relays it.

A declaration is invalid when the named tool is absent, when its package request does not equal the pinned value, or when a `tools[]` entry is not a `{ name, version }` object as required by `target.schema.json`.

This check is whole-tree: the `adapter` framework tool resolves each policy row's adapter manifest under `adapters/targets/`. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- A pinned first-party tool missing from its target adapter's `tools[]`.
- A declared tool whose package request does not match the pinned `<adapter, tool, package>` policy row.
- A `tools[]` entry that is not a `{ name, version }` object, or whose `name` / `version` is not a string.

## Fix

Declare each first-party tool under the target adapter's `tools[]` with the exact pinned `name` and `version`; when a pin legitimately changes, update the `expected-tools` table in this rule first.

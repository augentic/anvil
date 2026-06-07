---
id: CORE-049
title: Tools Invalid Declaration
severity: important
trigger: A first-party WASI tool declaration in a target adapter manifest is missing or version-mismatched.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-049-tools-invalid-declaration.md
    description: Sentinel include so the rule carries a candidate set; the `cross-reference` join evaluates the whole adapter-manifest fact family and ignores the candidate set.
  - kind: cross-reference
    value: expected-set
    config:
      target: adapter-tool
      entries:
        - key: contracts/contract
          value: 0.3.0
        - key: vectis/vectis
          value: 0.4.0
    description: Join the rule-declared expected tool table against the `adapter-tool` fact family (each target adapter manifest's `tools[]`, keyed `<adapter-dir>/<tool>` with the declared version) on the entry key; flag any pinned tool that is missing from, or version-mismatched in, its adapter's manifest. The expected `{key, value}` pins are policy carried here, not in the engine.
---

## Rule

Each first-party WASI tool a target adapter ships must be declared under that adapter's `adapter.yaml` `tools[]` with the exact pinned version. The expected `<adapter-dir>/<tool>` → version pins live in this rule's `config: { entries }` so the version pins are owned by the framework, not baked into the engine.

The `kind: cross-reference` hint with `value: expected-set` and `config: { target: adapter-tool }` joins the rule's expected-set entries against the adapter-tool fact family (one fact per declared tool, keyed by its adapter directory name and tool name). An entry is flagged when its tool is absent from the adapter's manifest, or present with a version that does not equal the pinned value. When an adapter directory carries no manifest at all the join skips its entries — the absent-manifest case is the loader-orphan concern of CORE-010, not this rule.

The `tools[]` object shape (`{ name, version }` with a semver-pinned version) is validated separately by CORE-001 (`kind: schema, value: adapter`), which runs the `adapter.schema.json` `toolDeclaration` shape over every `adapters/**/adapter.yaml`; a malformed entry is never recorded as an adapter-tool fact, so this rule reasons only about well-formed declarations.

## Look For

- A pinned first-party tool missing from its target adapter's `tools[]`.
- A declared tool whose version does not match the pinned `<adapter-dir>/<tool>` → version policy row.

## Fix

Declare each first-party tool under the target adapter's `tools[]` with the exact pinned `name` and `version`; when a pin legitimately changes, update the `entries` table in this rule first.

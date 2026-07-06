---
id: CORE-018
title: Links Prompt Schema Link Resolve
severity: important
trigger: An adapter prompt references an unknown schemas.specify.dev tool schema URL.
rule_hints:
  - kind: path-pattern
    value: adapters/codex/rules/core/CORE-018-links-prompt-schema-link-resolve.md
    description: Sentinel path so the whole-tree links-registry tool runs exactly once; the tool walks PROJECT_DIR/adapters itself rather than the passed candidate.
  - kind: tool
    value: links-registry
    config:
      known-schemas:
        - tool: vectis
          schemas: [tokens, assets, composition]
    description: Run the `links-registry` framework checker, which flags any schemas.specify.dev URL in an adapter prompt that does not resolve to a known tool-owned schema. The tool→schema registry is policy carried here, not in the tool.
---

## Rule

Every `https://schemas.specify.dev/<tool>/<name>.schema.json` URL in an adapter prompt or reference must resolve to a schema owned by a known tool. The tool→schema registry lives in this rule's `config:` so schema ownership is framework-owned policy, not baked into the checker. The tool reads the registry from the forwarded config; the engine only relays it.

This check is whole-tree: the `links-registry` framework tool walks `PROJECT_DIR/adapters` for markdown, ignoring URLs inside fenced or inline code. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- A `schemas.specify.dev` URL naming a tool not present in the `known-schemas` registry.
- A `schemas.specify.dev` URL naming a schema the registry does not list under its owning tool.

## Fix

Point the citation at a registered `<tool>/<schema>` pair, or — when a tool legitimately grows a new schema — add the schema to this rule's `known-schemas` registry in the same change that introduces it.

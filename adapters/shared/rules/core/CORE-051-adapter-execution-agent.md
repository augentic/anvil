---
id: CORE-051
title: Adapter Execution Agent
severity: suggestion
trigger: "A first-party adapter.yaml declares execution: agent (RFC-29 D9 informational)."
rule_hints:
  - kind: authoring-predicate
    value: adapter.execution-agent
    description: Run the retired imperative `adapter.execution-agent` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `adapter.execution-agent` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `adapter.execution-agent` on the framework tree.

## Fix

Resolve the violation described in the finding message for `adapter.execution-agent`.

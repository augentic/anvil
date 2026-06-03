---
id: CORE-011
title: Agent Teams Missing Canonical
severity: important
trigger: The canonical review-team-protocol document is missing so overlays cannot be validated.
rule_hints:
  - kind: authoring-predicate
    value: agent-teams.missing-canonical
    description: Run the retired imperative `agent-teams.missing-canonical` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `agent-teams.missing-canonical` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `agent-teams.missing-canonical` on the framework tree.

## Fix

Resolve the violation described in the finding message for `agent-teams.missing-canonical`.

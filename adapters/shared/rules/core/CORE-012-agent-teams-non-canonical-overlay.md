---
id: CORE-012
title: Agent Teams Non Canonical Overlay
severity: important
trigger: A target adapter agent-teams.md symlink does not resolve to the canonical review-team protocol.
rule_hints:
  - kind: authoring-predicate
    value: agent-teams.non-canonical-overlay
    description: Run the retired imperative `agent-teams.non-canonical-overlay` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `agent-teams.non-canonical-overlay` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `agent-teams.non-canonical-overlay` on the framework tree.

## Fix

Resolve the violation described in the finding message for `agent-teams.non-canonical-overlay`.

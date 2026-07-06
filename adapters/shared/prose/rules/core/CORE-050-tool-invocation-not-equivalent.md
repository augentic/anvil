---
id: CORE-050
title: Tool Invocation Not Equivalent
severity: important
trigger: Skills or target prompts invoke retired host helper commands whose capabilities now run in-guest inside the bound target adapter.
rule_hints:
  - kind: path-pattern
    value: "{plugins/**/skills/**/SKILL.md,adapters/targets/**/prose/prompts/**/*.md}"
  - kind: regex
    value: "\\bspecify-contract-validate\\b|\\bspecify-vectis\\s+(validate|init|add-shell)\\b|\\bspecify\\s+vectis\\s+(validate|init|add-shell)\\b"
    description: One alternation over every unconditionally retired invocation form (hyphenated and spaced vectis variants plus the retired contract-validate helper).
  - kind: regex
    value: "\\bspecify-contract\\b"
    config:
      suffix-must-not-start-with: "-validate"
    description: Kept separate from the alternation because the suffix guard applies to this token only.
---

## Rule

Retired helper invocations (`specify-contract`, `specify-contract-validate`, `specify-vectis …`, and spaced variants) must not survive in agent-facing prose: their capabilities are in-guest library code the target adapter runs deterministically, not host commands. The bare `specify-contract` hint carries a `-validate` suffix guard only so the longer `specify-contract-validate` token is reported once (by the alternation hint), not twice.

## Look For

- `specify-contract` or `specify-contract-validate` in skills or target prompts.
- `specify-vectis validate`, `specify vectis init`, `add-shell`, and sibling retired tokens.

## Fix

Remove the invocation and describe the surviving surface instead: the contract and vectis validators run deterministically in-guest at the adapter's build / merge gates, so agent-facing prose points at the gate (and at the files the agent edits), never at a host helper command.

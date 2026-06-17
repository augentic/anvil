---
id: CORE-050
title: Tool Invocation Not Equivalent
severity: important
trigger: Skills or target briefs invoke retired host helper commands that have `specify extension run` equivalents.
rule_hints:
  - kind: path-pattern
    value: "{plugins/**/skills/**/SKILL.md,adapters/targets/**/briefs/**/*.md}"
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

Retired helper invocations (`specify-contract`, `specify-contract-validate`, `specify-vectis …`, and spaced variants) must be replaced with declared-tool `specify extension run` forms. The bare `specify-contract` hint carries a `-validate` suffix guard only so the longer `specify-contract-validate` token is reported once (by the alternation hint), not twice.

## Look For

- `specify-contract` or `specify-contract-validate` in skills or target briefs.
- `specify-vectis validate`, `specify vectis init`, `add-shell`, and sibling retired tokens.

## Fix

Use `specify extension run contract -- …` or `specify extension run vectis -- …` per the bound target adapter manifest.

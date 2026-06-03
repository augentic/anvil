---
id: CORE-050
title: Tool Invocation Not Equivalent
severity: important
trigger: Skills or target briefs invoke retired host helper commands that have `specify tool run` equivalents.
rule_hints:
  - kind: path-pattern
    value: "plugins/**/skills/**/SKILL.md"
  - kind: path-pattern
    value: "adapters/targets/**/briefs/**/*.md"
  - kind: regex
    value: "\\bspecify-contract-validate\\b"
  - kind: regex
    value: "\\bspecify-contract\\b"
    config:
      suffix-must-not-start-with: "-validate"
  - kind: regex
    value: "\\bspecify-vectis\\s+validate\\b"
  - kind: regex
    value: "\\bspecify\\s+vectis\\s+validate\\b"
  - kind: regex
    value: "\\bspecify-vectis\\s+init\\b"
  - kind: regex
    value: "\\bspecify\\s+vectis\\s+init\\b"
  - kind: regex
    value: "\\bspecify-vectis\\s+add-shell\\b"
  - kind: regex
    value: "\\bspecify\\s+vectis\\s+add-shell\\b"
---

## Rule

Retired helper invocations (`specify-contract`, `specify-vectis …`, and spaced variants) must be replaced with declared-tool `specify tool run` forms. `specify-contract-validate` is allowed; bare `specify-contract` without the `-validate` suffix is not.

## Look For

- `specify-contract` not followed by `-validate` in skills or target briefs.
- `specify-vectis validate`, `specify vectis init`, `add-shell`, and sibling retired tokens.

## Fix

Use `specify tool run contract -- …` or `specify tool run vectis -- …` per the bound target adapter manifest.

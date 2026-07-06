---
id: UNI-013
title: Dead Code and Unreachable Paths
severity: suggestion
trigger: Code, handlers, branches, or model variants appear impossible to execute.
---

## Rule

Remove or justify code that cannot execute. Dead code and unreachable paths hide stale behavior, missing dispatch wiring, and incorrect assumptions even when the compiler cannot prove they are unreachable.

## Look For

- Functions or methods with no call site in the codebase.
- Match or switch arms that are unreachable because an earlier arm catches all matching values.
- Code after unconditional `return`, `break`, `continue`, or `throw`.
- Conditional branches guarded by conditions that are always true or always false given the surrounding context.
- Event variants defined in the data model but never dispatched by any view or handler.

---
id: UNI-014
title: Hardcoded Configuration Values
severity: important
trigger: Operational parameters are embedded as literals instead of named constants or configuration.
---

## Rule

Keep operational parameters discoverable and reviewable. Timeouts, URLs, retry counts, buffer sizes, page sizes, polling intervals, and similar values should be named constants or configuration rather than unexplained literals in behavior code.

## Look For

- Numeric literals in function calls that represent tunable parameters, such as timeout durations, retry counts, page sizes, or polling intervals.
- URL strings embedded directly in handler code rather than sourced from configuration.
- Magic numbers that require domain knowledge to understand, such as `42`, `1000`, or `86400`.
- The same literal value repeated in multiple locations where a shared constant should be used.

## Spec Guidance

When the spec does not define operational parameters such as timeouts, page sizes, retry limits, or polling intervals, propose adding them as explicit design decisions so they are reviewed and documented.

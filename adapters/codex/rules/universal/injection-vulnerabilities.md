---
id: UNI-019
title: Injection Vulnerabilities
severity: critical
trigger: Untrusted input can reach an execution, query, path, template, markup, or structured-output sink without sanitization or parameterization.
---

## Rule

Treat untrusted input as data, not executable code or structure. Validate at the boundary and use parameterization, escaping, canonicalization, builders, or allowlists before the input reaches dangerous sinks.

## Look For

- SQL or query injection: string concatenation or interpolation used to build database queries, search filters, or ORM conditions from user-supplied values.
- Command injection: user input passed to shell execution, process spawning, or system command APIs without escaping or allowlisting.
- Cross-site scripting: user-supplied text embedded in HTML, XML, or markup output without escaping.
- Path traversal: user-controlled values used in file path construction without canonicalization, prefix validation, or allowlisting.
- Template injection: user input interpolated into template engines, expression evaluators, or DSL interpreters that can execute arbitrary logic.

## See Also

- [UNI-002 — Unvalidated Input](unvalidated-input.md): the boundary-validation counterpart. UNI-002 ensures input is present and well-shaped; UNI-019 ensures it is treated as data (escaped, parameterized, canonicalized, or allowlisted) at the sink. A finding often satisfies one without the other, so cite the rule that names the missing control instead of duplicating across both.

---
id: UNI-008
title: Instrumentation and Logging Balance
severity: important
trigger: Error paths lack diagnostics, hot paths log excessively, or logs can expose sensitive data.
---

## Rule

Keep instrumentation useful and safe. Important failures should leave enough diagnostic signal, hot paths should not emit noisy per-item logs, and logs must not include sensitive data or debug-only output in production paths.

## Look For

- Error or failure branches that silently discard the error with no log statement, metric, or diagnostic output.
- Log statements inside tight loops or per-item processing that would produce excessive output at scale.
- Personally identifiable information, tokens, or credentials interpolated into log messages.
- Debug-only output such as `println!`, `dbg!`, `print()`, or `debugPrint()` remaining in production code.

## Spec Guidance

When the spec has no observability requirements but the app clearly needs them, propose adding an observability section that covers error tracking, performance metrics, or other required diagnostics.

---
id: UNI-016
title: Error Message Quality
severity: suggestion
trigger: An error path reports a generic message without enough context to identify the failed operation, data, or cause.
---

## Rule

Error messages should carry enough diagnostic context to identify what operation failed, on what data, and why. Avoid messages that require reproducing the issue just to locate the failing path.

## Look For

- Generic error messages with no specifics, such as "operation failed", "invalid input", or "something went wrong".
- Error messages that omit the item ID, field name, or value that caused the failure.
- Catch blocks that log the error type but not the error message or underlying cause.
- Multiple error sites using identical messages, making it impossible to determine which site produced the error in logs.

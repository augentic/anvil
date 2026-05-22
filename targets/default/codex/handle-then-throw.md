---
id: UNI-009
title: Handle-Then-Throw Anti-Pattern
severity: important
trigger: An error path performs partial handling or side effects and then rethrows, replaces, or swallows the error.
---

## Rule

Do not partially handle an error in a way that leaves visible side effects while still reporting the operation as failed. Handle the error completely at the right layer, or propagate it with enough context for the caller to handle it.

## Look For

- Catch blocks that mutate shared state, such as a model, view, or database, before re-throwing or returning an error so the mutation persists even though the operation failed.
- Error handlers that convert a specific, informative error into a generic one, losing diagnostic context.
- Try/catch at a low level that swallows errors which the caller needs to know about, such as returning a default from a helper when the caller should show an error state.
- Nested error handling where an inner handler partially handles and the outer handler also partially handles, with neither completing the job.

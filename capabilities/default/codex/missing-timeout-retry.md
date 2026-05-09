---
id: UNI-011
title: Missing Timeout or Retry on External Calls
severity: important
trigger: An external call or connection can hang or fail transiently without timeout, retry, cancellation, or user-visible failure behavior.
---

## Rule

External calls should define timeout, retry, cancellation, and failure behavior appropriate to the operation. Hanging calls must not block effect chains indefinitely or leave the app non-responsive without explanation.

## Look For

- HTTP requests dispatched without a configured timeout.
- SSE or WebSocket connections with no reconnection strategy after disconnect.
- Effect handlers that await a response indefinitely with no timeout or cancellation path.
- Retry logic that retries without backoff, risking a tight retry loop on persistent failures.

## Spec Guidance

When the spec does not define timeout or retry behavior for external calls, propose resilience requirements such as timeout duration, retry count, backoff strategy, and user-facing indication of failure.

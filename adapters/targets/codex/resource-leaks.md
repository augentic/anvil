---
id: UNI-005
title: Unbounded Growth and Resource Leaks
severity: important
trigger: Collections, queues, caches, subscriptions, observers, or async work can grow or remain alive without bounds.
---

## Rule

Bound retained resources and release them when they are no longer needed. Accumulated data, subscriptions, retained references, and long-lived async work need caps, eviction, cancellation, or cleanup paths.

## Look For

- Collections such as lists, maps, or queues that receive `.push()`, `.append()`, or `.insert()` without a corresponding removal, cap, or eviction policy.
- Event listeners, observers, or subscriptions registered without a matching unsubscribe or cancellation path.
- Strong reference cycles between objects, especially closures capturing their owning object.
- Long-lived async tasks or futures that are never cancelled when they become irrelevant.

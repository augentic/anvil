---
id: UNI-007
title: Unnecessarily Chatty External Calls
severity: important
trigger: External systems are called redundantly or excessively where batching, caching, debouncing, or deduplication is expected.
---

## Rule

Avoid redundant or excessive external calls. Prefer batching, debouncing, deduplication, caching, or using data already available when those choices preserve the required behavior.

## Look For

- Re-fetching data the app already has, such as a full reload after receiving a real-time update that already contains the new state.
- N+1 call patterns: looping over items and making one external call per item when a batch API exists.
- Missing debounce on rapid-fire user actions that each trigger a network call.
- Fetch-on-navigate patterns that re-request unchanged data without caching or staleness checks.

## Spec Guidance

When the spec mandates behavior that inherently creates chattiness, such as refreshing the full list on every keystroke, propose a spec amendment with a more efficient interaction pattern.

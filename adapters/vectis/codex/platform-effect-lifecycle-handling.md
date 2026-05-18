---
id: VECTIS-004
title: Platform Effect Lifecycle Handling
severity: important
trigger: Swift, Kotlin, or another Vectis shell handles Crux Effects, calls CoreFfi update/view/resolve methods, manages async platform work, or updates rendered ViewModel state.
---

## Rule

Platform shells must execute Crux Effects using platform-appropriate lifecycle, threading, cancellation, error, and serialization patterns. Shell bridge code must keep UI state observable on the platform main thread, preserve the current ViewModel when rendering fails after startup, surface CoreFfi and serialization errors for debugging, and ensure each async Effect request is resolved, cancelled, or completed in a way the core can handle.

Effect handlers must not crash, leak, silently drop requests, or leave the core waiting forever. Platform-specific fallbacks are acceptable only when they are explicit response values that the core models, such as an HTTP error response, an SSE done response, or a cleared timer response.

## Look For

- iOS `Core` wrappers missing `@MainActor`, `ObservableObject`, or `@Published` ViewModel state.
- Bare Swift `Task {}` blocks in effect handlers where `Task { @MainActor in ... }` is required.
- Kotlin coroutines for SSE, timers, or long-running work without `SupervisorJob`, cancellation handling, or fallback resolution.
- CoreFfi `update`, `view`, or `resolve` calls that throw or fail without preserving diagnostics.
- Render handlers that replace the current view with a loading/default ViewModel after a transient serialization or CoreFfi error.
- Effect branches that log and return without resolving the original request id or completing the modeled stream.
- Timers, subscriptions, or platform callbacks that are not cancelled or cleaned up when the core sends a clear/stop request.

## Spec Guidance

If the core needs to recover from a platform failure, model that failure as an Effect response and specify the resulting state transition. Do not rely on shell-only logs, crashes, or swallowed exceptions as the recovery behavior.

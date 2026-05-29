---
id: VECTIS-002
title: Crux State Transition Commands
severity: critical
trigger: Crux core update logic mutates model state, page state, sync state, pending operations, persisted state, or other data read by view(), tests, or follow-up effects.
---

## Rule

Every core state transition must return the commands required to make the transition observable and durable. A mutation to any field read by `view()` must schedule a render. A mutation to persisted state must schedule the persistence command. A transition that starts, advances, retries, or cancels asynchronous work must emit the matching Effect or completion command and must leave enough state for later responses to resolve correctly.

State machines must be complete for the Events and async responses the app can receive. The core must handle success, failure, retry, cancellation, duplicate action, rapid user action, and out-of-order response paths without entering stale UI, lost update, phantom pending operation, or unresolved request states.

## Look For

- Event arms that assign to `model.page`, route, loading, error, sync, connection, form, or list fields without returning `render()` in the command chain.
- Changes to persisted or recoverable state without a matching save command.
- Async start paths without a corresponding response, failure, retry, or cancellation path.
- Pending-operation queues that cannot distinguish create/update/delete ordering, in-flight items, or local-versus-server timestamps.
- Destructive operations that fail to coalesce a pending create before sending a delete to the server.
- State enums with transitions covered in one direction but not the reverse or failure path.
- Tests that cover happy-path Events but not the interleavings that can occur while Effects are in flight.

## Spec Guidance

If a transition edge is missing because the spec never described the state, response, or retry path, add or update the scenario instead of inventing a local implementation rule. Important Crux state machines should have scenario coverage for both UI-visible transitions and async interleavings.

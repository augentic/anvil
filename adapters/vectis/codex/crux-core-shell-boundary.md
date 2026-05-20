---
id: VECTIS-001
title: Crux Core And Shell Boundary
severity: critical
trigger: A Vectis Crux app changes business state, domain decisions, ViewModel construction, Event handling, Effect definitions, or platform shell behavior.
---

## Rule

The Rust Crux core owns business state, domain decisions, state transitions, ViewModel construction, Event handling, and Effect requests. Platform shells must remain thin adapters: render the current ViewModel, dispatch user-initiated Events, execute requested Effects with platform APIs, and resolve those Effects back to the core.

Do not duplicate or override domain state in Swift, Kotlin, or other shells. Do not move business rules, validation decisions, conflict resolution, routing policy, or derived ViewModel fields out of the core to make one platform work. Shell-local state is acceptable only for platform presentation concerns such as focus, transient animation, native picker state, snackbar visibility, permission prompts, or lifecycle bookkeeping that does not decide domain behavior.

## Look For

- SwiftUI or Compose code that filters, sorts, validates, coalesces, conflicts, routes, or mutates domain records independently of the core.
- Shell-specific state that shadows core model fields and can drift from the ViewModel.
- ViewModel fields that are missing because the shell reconstructs them from raw domain data.
- Platform-only fixes for behavior that should be expressed as a core Event, core state transition, ViewModel field, or Effect response.
- Business rules implemented differently between iOS and Android shells.
- Core changes that expose platform framework types or shell-specific presentation details through domain models.

## Spec Guidance

When a shell needs information to render correctly, add the platform-neutral field to the core ViewModel or to the relevant platform section of the spec. When a platform requires native handling, keep the domain decision in the core and express the platform work as an Effect or shell presentation concern.

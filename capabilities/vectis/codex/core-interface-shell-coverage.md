---
id: VECTIS-003
title: Core Interface Shell Coverage
severity: critical
trigger: A Vectis shell is generated or updated after the Rust core changes ViewModel variants, Event variants, Effect variants, Route variants, generated FFI types, or serialization shape.
---

## Rule

Every platform shell must cover the complete shell-facing interface exposed by the Crux core. Each ViewModel variant needs a reachable render branch and screen or component. Each shell-facing Event needs at least one appropriate UI dispatch path unless the spec intentionally leaves it system-driven. Each Effect variant needs a handler that performs the platform work and resolves, completes, or explicitly fails the request using the generated serialization types. Each Route or navigation state needs a matching platform navigation path when the app exposes navigation.

Coverage must be checked against the generated bindings and the Rust source, not inferred from stale shell code. Missing branches are defects even when the language compiler accepts a default, fallback, or ignored case.

## Look For

- ViewModel variants present in `app.rs` or generated bindings with no SwiftUI view, Compose screen, root switch branch, or `when` branch.
- Event variants that represent user actions but are never dispatched by the shell.
- Effect variants missing from `processEffect`, `processRequest`, or equivalent shell bridge code.
- Route variants with no tab, button, link, destination, or explicit navigation mapping.
- Catch-all/default shell branches that hide missing core variants.
- Generated type imports or package names that point at old bindings after core regeneration.
- iOS and Android shells that diverge in which core variants they support for the same feature.

## Spec Guidance

When a core variant intentionally has no UI trigger or no platform implementation, record that reason in the relevant spec or design artifact. Otherwise treat missing shell coverage as an implementation defect, not as a platform-specific omission.

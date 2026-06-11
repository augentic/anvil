# iOS Shell References

Index of the iOS shell corpus for Vectis generation. Each file covers one concern of the SwiftUI shell around the shared Crux core.

| File | Read when |
| --- | --- |
| [shell-pattern.md](shell-pattern.md) | Scaffolding or reviewing the iOS shell itself — the thin SwiftUI layer that renders `ViewModel` and dispatches `Event` values (0.17+ API). |
| [view-patterns.md](view-patterns.md) | Writing SwiftUI views that consume Crux `ViewModel` data and send events back to the core. |
| [token-templates.md](token-templates.md) | Emitting Swift token code from a `tokens.yaml` design-system file. |
| [design-system-integration.md](design-system-integration.md) | Wiring `tokens.yaml` and `assets.yaml` into the generated iOS project end to end. |

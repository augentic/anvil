# Android Shell References

Index of the Android shell corpus for Vectis generation. Each file covers one concern of the Kotlin/Jetpack Compose shell around the shared Crux core.

| File | Read when |
| --- | --- |
| [shell-pattern.md](shell-pattern.md) | Scaffolding or reviewing the Android shell itself — the thin Compose layer that renders `ViewModel` and dispatches `Event` values (0.17+ API). |
| [view-patterns.md](view-patterns.md) | Writing Compose UI that consumes Crux `ViewModel` data and sends events back to the core. |
| [token-templates.md](token-templates.md) | Emitting Material 3 Kotlin token code from a `tokens.yaml` design-system file. |
| [design-system-integration.md](design-system-integration.md) | Wiring `tokens.yaml` and `assets.yaml` into the generated Android project end to end. |

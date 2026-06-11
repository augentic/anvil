# Crux Core References

Index of the Crux core corpus for Vectis generation: the shared Rust crate's app shape, capabilities, and testing patterns (0.17+ API).

| File | Read when |
| --- | --- |
| [app-pattern.md](app-pattern.md) | Scaffolding or reviewing the core `App` — `Model`, `Event`, `ViewModel`, and the `update`/`view` shape. |
| [update-change-patterns.md](update-change-patterns.md) | Implementing `update` arms — how each event class mutates the model and what it commands. |
| [command-api.md](command-api.md) | Issuing effects through the Crux command API (render, HTTP, KV composition). |
| [capabilities.md](capabilities.md) | Using the stock HTTP and key-value capabilities. |
| [custom-capabilities.md](custom-capabilities.md) | A required effect has no stock capability and a custom one must be built. |
| [generated-type-conventions.md](generated-type-conventions.md) | Naming and shaping the types shared across the FFI boundary to the shells. |
| [artifact-to-code-mapping.md](artifact-to-code-mapping.md) | Translating Specify artifacts (spec/design requirements) into core code structures. |
| [testing-patterns.md](testing-patterns.md) | Writing core tests — driving `update` directly and asserting on commands and view models. |

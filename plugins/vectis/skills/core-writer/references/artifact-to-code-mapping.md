# Artifact-to-Code Mapping

Each Specify artifact section maps to a specific code construct. The core-writer reads this table during the diff analysis step (Update Mode U4) to systematically identify what changed and where to apply the edit.

| Artifact Section | Code Construct | File(s) | Diff Indicators |
|---|---|---|---|
| **Design** -- Overview | App struct name, `impl App for X` | `app.rs`, `ffi.rs` | Renamed app |
| **Spec** -- Feature requirements | Shell-facing Event variants | `app.rs` `enum Event` | New/removed/renamed user actions |
| **Spec** -- Feature requirements | `update()` match arms | `app.rs` `fn update()` | New/removed/changed handler logic |
| **Design** -- Domain Model (entities) | Domain structs and enums | `app.rs` top section | New/changed/removed fields or types |
| **Design** -- Domain Model (state) | `struct Model` fields | `app.rs` `struct Model` | New/changed/removed state fields |
| **Spec** -- View/page requirements | `enum ViewModel` variants | `app.rs` `enum ViewModel` | New/removed/renamed views |
| **Spec** -- View/page requirements | `enum Page` variants | `app.rs` `enum Page` | New/removed/renamed internal page states |
| **Spec** -- View/page requirements | `enum Route` variants | `app.rs` `enum Route` | New/removed navigable destinations |
| **Spec** -- View/page requirements | `Event::Navigate` match arm | `app.rs` `fn update()` | Changed navigation handling |
| **Spec** -- View/page requirements | `fn view()` match arms | `app.rs` `fn view()` | New/removed page-to-view mappings |
| **Spec** -- View/page requirements | Per-page view struct fields | `app.rs` per-page structs | New/changed/removed display data |
| **Spec** -- View/page requirements | `fn view()` body | `app.rs` `fn view()` | Changed model-to-view mapping |
| **Design** -- Capabilities | `enum Effect` variants | `app.rs` `enum Effect` | Added/removed capabilities |
| **Design** -- Capabilities | Type aliases (`type Http = ...`) | `app.rs` top section | Added/removed aliases |
| **Design** -- Capabilities | Crate dependencies | `shared/Cargo.toml` | Added/removed deps |
| **Design** -- Capabilities | Custom capability modules | `shared/src/*.rs`, `lib.rs` | Added/removed modules |
| **Design** -- Capabilities | Internal Event variants | `app.rs` `enum Event` | Callback variants for added/removed capabilities |
| **Design** -- API Contracts (endpoints) | HTTP call sites in `update()` | `app.rs` `fn update()` | Changed URLs, methods |
| **Design** -- API Contracts (shapes) | Request/response body structs | `app.rs` domain types | Changed fields |
| **Spec** -- Scenario conditions | Validation logic in `update()` | `app.rs` `fn update()` | Changed guards, conditions |
| **Spec** -- Scenario conditions | Helper functions | `app.rs` free functions | Changed conflict resolution, sync logic |

> **Note:** Per-page view struct fields align with `composition.yaml` field bindings via `design.md`. The core-writer reads `design.md`, not `composition.yaml` — layout is a shell concern. The composition artifact declares which data fields each screen needs (via `bind` keys), the design brief formalizes them into per-page view structs, and the core-writer generates the Rust types from the design.

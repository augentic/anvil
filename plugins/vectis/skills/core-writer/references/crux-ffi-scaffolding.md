# Crux FFI Scaffolding (0.17+ API)

The FFI layer bridges the Crux core to platform shells (iOS via UniFFI, Web via wasm-bindgen).
In 0.17+ this is implemented as a `CoreFFI` struct with feature-gated attributes.

## `shared/src/ffi.rs`

This file is identical across all Crux apps except for the `Bridge<AppType>` generic parameter.
Copy this template and replace `MyApp` with your app struct name.

```rust
use crux_core::{
    Core,
    bridge::{Bridge, EffectId},
};

use crate::MyApp;

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "wasm_bindgen", wasm_bindgen::prelude::wasm_bindgen)]
pub struct CoreFFI {
    core: Bridge<MyApp>,
}

impl Default for CoreFFI {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
#[cfg_attr(feature = "wasm_bindgen", wasm_bindgen::prelude::wasm_bindgen)]
impl CoreFFI {
    #[cfg_attr(feature = "uniffi", uniffi::constructor)]
    #[cfg_attr(
        feature = "wasm_bindgen",
        wasm_bindgen::prelude::wasm_bindgen(constructor)
    )]
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: Bridge::new(Core::new()),
        }
    }

    /// Send an event to the app and return the serialized effects.
    /// # Panics
    /// If the event cannot be deserialized.
    #[must_use]
    pub fn update(&self, data: &[u8]) -> Vec<u8> {
        let mut effects = Vec::new();
        match self.core.update(data, &mut effects) {
            Ok(()) => effects,
            Err(e) => panic!("{e}"),
        }
    }

    /// Resolve an effect with a response and return any new serialized effects.
    /// # Panics
    /// If the data cannot be deserialized or the `effect_id` is invalid.
    #[must_use]
    pub fn resolve(&self, id: u32, data: &[u8]) -> Vec<u8> {
        let mut effects = Vec::new();
        match self.core.resolve(EffectId(id), data, &mut effects) {
            Ok(()) => effects,
            Err(e) => panic!("{e}"),
        }
    }

    /// Get the current `ViewModel` as serialized bytes.
    /// # Panics
    /// If the view cannot be serialized.
    #[must_use]
    pub fn view(&self) -> Vec<u8> {
        let mut view_model = Vec::new();
        match self.core.view(&mut view_model) {
            Ok(()) => view_model,
            Err(e) => panic!("{e}"),
        }
    }
}
```

## `shared/src/lib.rs`

Wire the FFI module (conditionally compiled) and set up UniFFI scaffolding:

```rust
mod app;
#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
mod ffi;

pub use app::*;
pub use crux_core::Core;

#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
pub use ffi::CoreFFI;

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();
```

If you have custom capability modules, add them here:

```rust
mod app;
#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
mod ffi;
pub mod sse;

pub use app::*;
pub use crux_core::Core;

#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
pub use ffi::CoreFFI;

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();
```

## Key Points

### No `.udl` file

The 0.17+ API uses `uniffi::setup_scaffolding!()` and `#[uniffi::export]` attributes
instead of a `.udl` interface definition file. Do not create a `.udl` file.

### No `LazyLock` static

The old pattern used a global `static CORE: LazyLock<Bridge<App>>`. The new pattern
creates `CoreFFI` instances that each own their `Bridge`. Do not use `LazyLock`.

### Feature gates

All UniFFI and wasm-bindgen code is behind feature flags:

- `feature = "uniffi"` -- for native iOS/Android via UniFFI
- `feature = "wasm_bindgen"` -- for Web via wasm-bindgen

This means the shared library compiles cleanly as a plain Rust library when
neither feature is enabled (e.g., during `cargo test`).

### `Bridge` vs `Core`

- `Core<MyApp>` is the Crux core that runs the app.
- `Bridge<MyApp>` wraps `Core` and handles serialization/deserialization of
  events, effects, and view models for FFI transport.
- Always use `Bridge` in `CoreFFI`, never `Core` directly.

### The three FFI methods

| Method | Shell calls it when... | Input | Output |
|--------|------------------------|-------|--------|
| `update(data)` | User interacts with UI | Serialized `Event` | Serialized effect requests |
| `resolve(id, data)` | Shell completes a side-effect | Effect ID + serialized response | Serialized new effect requests |
| `view()` | Shell needs current UI state | None | Serialized `ViewModel` |

### `EffectId`

Each effect request has a unique `EffectId(u32)` assigned by the bridge.
The shell uses this ID to route responses back to the correct pending effect.
Import it from `crux_core::bridge::EffectId`.

# Crux FFI Scaffolding (0.17+ API)

The FFI layer bridges the Crux core to platform shells (iOS via UniFFI, Web via wasm-bindgen).
In 0.17+ this is implemented as a `CoreFFI` struct with feature-gated attributes.

## `shared/src/ffi.rs`

This file is identical across all Crux apps except for the `Bridge<AppType>` generic
parameter and the `use crate::MyApp` import. Copy this template and replace `MyApp`
with your app struct name.

```rust
use crux_core::{
    Core,
    bridge::{Bridge, BridgeError, EffectId, FfiFormat},
};

use crate::MyApp;

/// FFI error type surfaced to shell platforms.
///
/// UniFFI maps this to a thrown Swift/Kotlin error.
/// wasm-bindgen maps this to a JavaScript exception.
#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Error))]
#[cfg_attr(feature = "uniffi", uniffi(flat_error))]
pub enum CoreError {
    #[error("{msg}")]
    Bridge { msg: String },
}

impl<F: FfiFormat> From<BridgeError<F>> for CoreError {
    fn from(e: BridgeError<F>) -> Self {
        Self::Bridge {
            msg: e.to_string(),
        }
    }
}

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
    ///
    /// # Errors
    ///
    /// Returns `CoreError` if the event cannot be deserialized.
    pub fn update(&self, data: &[u8]) -> Result<Vec<u8>, CoreError> {
        let mut effects = Vec::new();
        self.core.update(data, &mut effects)?;
        Ok(effects)
    }

    /// Resolve an effect with a response and return any new serialized effects.
    ///
    /// # Errors
    ///
    /// Returns `CoreError` if the data cannot be deserialized or the effect ID
    /// is invalid.
    pub fn resolve(&self, id: u32, data: &[u8]) -> Result<Vec<u8>, CoreError> {
        let mut effects = Vec::new();
        self.core.resolve(EffectId(id), data, &mut effects)?;
        Ok(effects)
    }

    /// Get the current `ViewModel` as serialized bytes.
    ///
    /// # Errors
    ///
    /// Returns `CoreError` if the view model cannot be serialized.
    pub fn view(&self) -> Result<Vec<u8>, CoreError> {
        let mut view_model = Vec::new();
        self.core.view(&mut view_model)?;
        Ok(view_model)
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
| `update(data)` | User interacts with UI | Serialized `Event` | `Result<Vec<u8>, CoreError>` -- serialized effect requests |
| `resolve(id, data)` | Shell completes a side-effect | Effect ID + serialized response | `Result<Vec<u8>, CoreError>` -- serialized new effect requests |
| `view()` | Shell needs current UI state | None | `Result<Vec<u8>, CoreError>` -- serialized `ViewModel` |

All three methods return `Result` so that serialization or deserialization
failures surface as typed errors in the shell rather than crashing the process.
UniFFI maps `Result<T, CoreError>` to Swift `throws` / Kotlin `throws`.
wasm-bindgen maps it to a JavaScript exception.

### `CoreError`

`CoreError` is a simple wrapper around `BridgeError` that works across FFI
boundaries. It uses `thiserror::Error` for the `Display` impl and is
feature-gated with `uniffi::Error` (using `flat_error` so UniFFI uses the
`Display` string as the error message). The `From<BridgeError<F>>` impl
enables `?` propagation from Bridge methods.

Do **not** use `panic!` in FFI methods. A panic in the static library traps
the host process (iOS app crash, browser tab crash) with no recovery path.

### `EffectId`

Each effect request has a unique `EffectId(u32)` assigned by the bridge.
The shell uses this ID to route responses back to the correct pending effect.
Import it from `crux_core::bridge::EffectId`.

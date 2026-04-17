use crux_core::{
    Core,
    bridge::{Bridge, BridgeError, EffectId, FfiFormat},
};

use crate::__APP_STRUCT__;

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
    core: Bridge<__APP_STRUCT__>,
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

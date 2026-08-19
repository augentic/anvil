//! The mock source adapter as a Wasm component: the journey fixture.
//! One export serves every identity; behaviour keys off adapter-id.
#![cfg(target_arch = "wasm32")]

mod source_adapter;

emery_adapter::source!(source_adapter::Adapter);

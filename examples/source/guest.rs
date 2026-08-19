//! The mock source adapter as a Wasm component: the journey fixture.
//! One export serves every identity; behaviour keys off adapter-id.
#![cfg(target_arch = "wasm32")]

extern crate adapter as sdk;

mod adapter;

sdk::source!(adapter::Adapter);

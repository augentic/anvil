//! Component export
//!
//! Turns a [`crate::SourceAdapter`] implementation into the `source-adapter`
//! wasm world the engine loads. An adapter crate invokes
//! [`crate::source!`] once and gains a complete component export without
//! touching the generated bindings.
//!
//! This is the only wasm-specific code an adapter carries, which keeps the
//! rest of its logic portable and testable natively.

pub use emery_source::wire::*;

/// Maps adapter metadata to its WIT record.
#[must_use]
pub fn dispatch_metadata<A: crate::SourceAdapter>() -> AdapterMetadata {
    A::metadata().into()
}

/// Dispatches extract through adapter `A`.
///
/// # Errors
///
/// Returns the adapter's extract error.
pub async fn dispatch_extract<A: crate::SourceAdapter>(
    id: AdapterId, input: Input,
) -> Result<Evidence, Error> {
    let input = crate::types::SourceInput::from(input);
    let ctx = crate::types::Context::guest(&id).with_docs(A::docs());
    let ctx = match &input.content {
        crate::types::SourceContent::Workspace(view) => ctx.lending(view.root.clone()),
        crate::types::SourceContent::Value(_) => ctx.without_lend(),
    };

    A::extract(&crate::WasiModel, &ctx, &input).await.map(Into::into).map_err(Into::into)
}

/// Wires a [`crate::SourceAdapter`] into component exports.
///
/// ```ignore
/// emery_adapter::source!(crate::Captures);
/// ```
#[macro_export]
macro_rules! source {
    ($adapter:ty) => {
        struct Adapter;
        $crate::source::export!(Adapter with_types_in $crate::source);

        impl $crate::source::Guest for Adapter {
            fn metadata(
                _id: $crate::source::AdapterId,
            ) -> $crate::source::AdapterMetadata {
                $crate::source::dispatch_metadata::<$adapter>()
            }

            async fn extract(
                id: $crate::source::AdapterId,
                input: $crate::source::Input,
            ) -> Result<$crate::source::Evidence, $crate::source::Error> {
                $crate::source::dispatch_extract::<$adapter>(id, input).await
            }
        }
    };
}

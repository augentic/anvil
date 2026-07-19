//! A Wasm-free Specify CLI over statically linked mock adapters.
//! (wasm32 builds compile an empty stub so `--examples` passes.)

#[cfg(not(target_arch = "wasm32"))]
mod model;

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::process::ExitCode;

#[cfg(not(target_arch = "wasm32"))]
use native::{Catalog, DynModel, ExecutionPaths};

#[cfg(not(target_arch = "wasm32"))]
use crate::model::DevModel;

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn run() -> anyhow::Result<ExitCode> {
    let root = PathBuf::from(".").canonicalize()?;
    let catalog = Catalog::builder().source::<mock::Adapter>().target::<mock::Adapter>().build()?;
    let paths = ExecutionPaths::operator(root.clone());
    let model = DynModel::new(DevModel::new(&root));

    Ok(native::command::run(paths, model, catalog, std::env::args().collect()).await)
}

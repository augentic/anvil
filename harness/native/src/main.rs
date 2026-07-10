//! `specify-dev` — the Rust-native shim binary.
//!
//! Two modes over the same handler layer the wasm guest serves, each
//! owned by a symmetric transport module:
//!
//! - **CLI mode** (default, [`command`]): the shared typed command
//!   router against a `NativeProvider`, plus an ephemeral MCP shelf.
//! - **`serve` mode** ([`http`]): the shared typed HTTP router merged
//!   with the `/mcp/<name>` shelves on one `TcpListener`.

mod command;
mod http;

use std::process::ExitCode;

use specify_dev::provider;
use workflow::adapter;

#[tokio::main]
async fn main() -> ExitCode {
    // Adapter describe dispatch calls the linked adapter crates'
    // `describe()` directly, so the resolvers work without a wasm
    // runtime.
    adapter::metadata::register(provider::metadata);

    let argv: Vec<String> = std::env::args().collect();
    if argv.get(1).map(String::as_str) == Some("serve") {
        match http::serve(&argv[1..]).await {
            Ok(code) => code,
            Err(err) => {
                eprintln!("specify-dev: {err:#}");
                ExitCode::FAILURE
            }
        }
    } else {
        ExitCode::from(command::run(argv).await)
    }
}

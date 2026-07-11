//! `specify-dev` — the Rust-native shim binary.
//!
//! Two modes over the same handler layer the wasm guest serves, each
//! owned by a symmetric transport module:
//!
//! - **CLI mode** (default, [`command`]): the shared typed command
//!   router against the native provider, plus an ephemeral MCP shelf.
//!   A leading shim-global `--project-dir <path>` anchors the provider
//!   (and the model's lent workspace) at another project root.
//! - **`serve` mode** ([`http`]): the shared typed HTTP router merged
//!   with the `/mcp/<name>` shelves on one `TcpListener`; carries its
//!   own `--project-dir` flag.

mod command;
mod http;

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
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

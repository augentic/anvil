//! WASI command export over the shared typed Specify router.

use omnia_guest::api::invoke::Invoker;
use omnia_guest::wasip3;

use crate::provider::Provider;

struct CliGuest;
wasip3::cli::command::export!(CliGuest);

impl wasip3::exports::cli::run::Guest for CliGuest {
    async fn run() -> Result<(), ()> {
        let invoker = Invoker::new("specify", Provider);
        let router = argv::router::router(invoker, |_| Ok(())).map_err(|_error| ())?;
        omnia_guest::api::command::execute_wasi(&router).await
    }
}

//! Native command entry over the shared typed Specify router.

use std::path::PathBuf;

use omnia_guest::api::invoke::Invoker;
use specify_dev::mcp;
use specify_dev::model::DevModel;
use specify_dev::provider::NativeProvider;
use tokio::net::TcpListener;

/// Parse and execute one native command invocation.
pub async fn run(argv: Vec<String>) -> u8 {
    let root = PathBuf::from(".");
    let model = match DevModel::from_env(&root) {
        Ok(model) => model,
        Err(error) => {
            eprintln!("error: {error:#}");
            return 1;
        }
    };
    let mut provider = NativeProvider::new(root, model);
    if let Some(base) = shelves().await {
        provider = provider.mcp_base(base);
    }
    let router = match argv::router::router(Invoker::new("specify", provider), |_| Ok(())) {
        Ok(router) => router,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    let response = router.execute(argv).await;
    if response.write_to(&mut std::io::stdout().lock(), &mut std::io::stderr().lock()).is_err() {
        return 1;
    }
    response.exit
}

async fn shelves() -> Option<String> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.ok()?;
    let base = format!("http://127.0.0.1:{}", listener.local_addr().ok()?.port());
    tokio::spawn(async move {
        drop(axum::serve(listener, mcp::router()).await);
    });
    Some(base)
}

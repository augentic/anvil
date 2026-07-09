//! MCP HTTP route test: the `/mcp/echo` route serves the echo guest's
//! references.
//!
//! Mirrors omnia's `wasi-http` routing seam test — deployment from a manifest,
//! `wasi:http` host linked, requests driven through the testkit's in-process
//! HTTP driver (no TCP socket) — and speaks MCP Streamable HTTP: one JSON-RPC
//! message per POST.

#![cfg(not(target_arch = "wasm32"))]

use std::sync::Arc;

use anyhow::{Context as _, Result};
use omnia::{Backends as _, DeploymentBuilder, MountRegistry, Runtime, StoreCtx};
use omnia_testkit::http;
use omnia_wasi_http::WasiHttp;
use omnia_wasi_model::WasiModel;
use serde_json::{Value, json};

use crate::common::Bundle;

// Assemble the skeleton deployment into a runtime the in-process HTTP driver
// can serve requests through.
async fn runtime() -> Result<Runtime<Bundle>> {
    let manifest = crate::common::skeleton_manifest("source:echo")?;
    let mut deployment = DeploymentBuilder::new()
        .config(manifest.path().to_path_buf())
        .build::<StoreCtx<Bundle>>()
        .await
        .context("building deployment")?;
    deployment.host::<WasiHttp, Bundle>().context("linking http host")?;
    deployment.host::<WasiModel, Bundle>().context("linking model host")?;
    let registry = deployment.into_registry().context("assembling registry")?;

    Ok(Runtime::from_parts(
        Arc::new(registry),
        Vec::new(),
        Arc::new(MountRegistry::default()),
        Bundle::connect().await.context("connecting backends")?,
    ))
}

// POST one JSON-RPC message to /mcp/echo and parse the reply.
async fn post(runtime: &Runtime<Bundle>, message: &Value) -> Result<Value> {
    let response = http::post_json(runtime, "/mcp/echo", message.to_string()).await?;
    assert!(response.status().is_success(), "MCP POST replies 2xx: {}", response.status());
    serde_json::from_slice(response.body()).context("MCP reply is JSON")
}

// The route serves an MCP references: initialize identifies the echo server,
// resources/list carries the echo-reference doc, and the read_reference tool
// returns its body.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn references() -> Result<()> {
    let runtime = runtime().await?;

    let init = post(
        &runtime,
        &json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2025-06-18" }
        }),
    )
    .await?;
    assert_eq!(init["result"]["serverInfo"]["name"], "specify-echo-references");

    let resources =
        post(&runtime, &json!({ "jsonrpc": "2.0", "id": 2, "method": "resources/list" })).await?;
    assert_eq!(resources["result"]["resources"][0]["name"], "echo-reference");
    assert_eq!(resources["result"]["resources"][0]["uri"], "doc://echo-reference");

    let tool = post(
        &runtime,
        &json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": { "name": "read_reference" }
        }),
    )
    .await?;
    let text = tool["result"]["content"][0]["text"].as_str().unwrap_or_default();
    assert!(text.contains("Echo Reference"), "tool returned the reference body: {tool}");

    Ok(())
}

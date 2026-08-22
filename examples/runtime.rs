//! The journey host: the shipped runtime shape with the mock source
//! component as the one adapter guest and `WasiModel` answering from
//! a script directory instead of the Cursor backend.

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use std::future::Future;
        use std::sync::Arc;

        use anyhow::Context as _;
        use omnia_filesystem::Client as Filesystem;
        use omnia_testkit::model::Scripted;
        use omnia_wasi_blobstore::{Container, WasiBlobstore, WasiBlobstoreCtx};
        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_keyvalue::{Bucket, WasiKeyValue, WasiKeyValueCtx};
        use omnia_wasi_model::{Answer, FutureResult, Request, ToolHost, WasiModel, WasiModelCtx};
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

        omnia::runtime!({
            mode: command,
            guests: [
                {
                    id: "emery",
                    source: include_bytes!(concat!(env!("OUT_DIR"), "/emery.cwasm")),
                },
                {
                    id: "source:source",
                    source: concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/target/wasm32-wasip2/release/examples/source.wasm",
                    ),
                    routes: {http: ["/mcp/source/source"]},
                },
            ],
            mounts: [
                { name: ".", path: ".", writable: true },
            ],
            dispatch: ["emery:adapter/source@0.1.0"],
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiModel: ScriptedModel,
                WasiKeyValue: ProjectStore,
                WasiBlobstore: ProjectStore,
            }
        });

        // The shipped binary's storage binding, reproduced for the
        // journey host: a durable filesystem store rooted at `.emery`
        // under the invocation directory, engine blob containers
        // created at connect (see `src/main.rs`).
        const STORE_ROOT: &str = ".emery";

        #[derive(Clone, Debug)]
        struct ProjectStore(Filesystem);

        impl omnia::Backend for ProjectStore {
            type ConnectOptions = omnia::NoOptions;

            async fn connect_with(_options: omnia::NoOptions) -> anyhow::Result<Self> {
                let client = Filesystem::open(STORE_ROOT)?;
                for container in [
                    emery_engine::home::SPEC_CONTAINER,
                    emery_engine::handler::ADAPTERS_CONTAINER,
                    emery_engine::handler::STORE_CONTAINER,
                ] {
                    drop(client.create_container(container.to_string()).await?);
                }
                Ok(Self(client))
            }
        }

        impl WasiKeyValueCtx for ProjectStore {
            fn open_bucket(&self, identifier: String) -> FutureResult<Arc<dyn Bucket>> {
                self.0.open_bucket(identifier)
            }
        }

        impl WasiBlobstoreCtx for ProjectStore {
            fn create_container(&self, name: String) -> FutureResult<Arc<dyn Container>> {
                self.0.create_container(name)
            }

            fn get_container(&self, name: String) -> FutureResult<Arc<dyn Container>> {
                self.0.get_container(name)
            }

            fn delete_container(&self, name: String) -> FutureResult<()> {
                self.0.delete_container(name)
            }

            fn container_exists(&self, name: String) -> FutureResult<bool> {
                self.0.container_exists(name)
            }
        }

        // script directory: each file is one model answer.
        const SCRIPT_ENV: &str = "EMERY_JOURNEY_SCRIPT";

        fn connect() -> anyhow::Result<ScriptedModel> {
            let dir = std::env::var(SCRIPT_ENV)
                .with_context(|| format!("{SCRIPT_ENV} must name the model script directory"))?;
            let mut files: Vec<_> = std::fs::read_dir(&dir)
                .with_context(|| format!("reading the model script directory `{dir}`"))?
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .collect();
            files.sort();
            let answers: Vec<String> = files
                .iter()
                .map(std::fs::read_to_string)
                .collect::<Result<_, _>>()
                .context("reading a script answer")?;
            anyhow::ensure!(!answers.is_empty(), "the script directory `{dir}` carries no answers");
            Ok(ScriptedModel(Scripted::answers(answers)))
        }

        // The scripted `wasi:model` backend behind the unchanged seam.
        #[derive(Clone, Debug)]
        struct ScriptedModel(Scripted);

        impl omnia::Backend for ScriptedModel {
            type ConnectOptions = omnia::NoOptions;

            fn connect_with(
                _options: omnia::NoOptions,
            ) -> impl Future<Output = anyhow::Result<Self>> {
                std::future::ready(connect())
            }
        }

        impl WasiModelCtx for ScriptedModel {
            fn complete(
                &self, request: Request, tool_host: Arc<dyn ToolHost>,
            ) -> FutureResult<Answer> {
                self.0.complete(request, tool_host)
            }
        }
    } else {
        fn main() {}
    }
}

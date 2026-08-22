//! Journey host using the mock source component and scripted model answers.

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

        // Match the shipped binary's durable, invocation-relative storage.
        const STORE_ROOT: &str = ".emery";

        #[derive(Clone, Debug)]
        struct ProjectStore(Filesystem);

        impl omnia::Backend for ProjectStore {
            type ConnectOptions = omnia::NoOptions;

            fn connect_with(
                _options: omnia::NoOptions,
            ) -> impl Future<Output = anyhow::Result<Self>> {
                std::future::ready(Filesystem::open(STORE_ROOT).map(Self))
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

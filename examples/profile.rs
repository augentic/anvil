//! Project-id-keyed storage profile over Omnia's in-memory hosts.
//!
//! `EMERY_PROJECT_ID` scopes every bucket and container; see
//! `docs/reference/deployment-profiles.md` for remote-backed deployments.

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use std::sync::Arc;

        use anyhow::Context as _;
        use omnia::{Backend, FromEnv, FutureResult};
        use omnia_cursor::Client as Cursor;
        use omnia_wasi_blobstore::{BlobstoreDefault, Container, WasiBlobstore, WasiBlobstoreCtx};
        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_keyvalue::{Bucket, KeyValueDefault, WasiKeyValue, WasiKeyValueCtx};
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

        omnia::runtime!({
            mode: command,
            guests: [
                {
                    id: "emery",
                    source: include_bytes!(concat!(env!("OUT_DIR"), "/emery.cwasm")),
                    routes: {http: ["/mcp/emery/spec"]},
                }
            ],
            mounts: [
                { name: ".", path: "." },
            ],
            dispatch: ["emery:adapter/source@0.1.0"],
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiModel: Cursor,
                WasiKeyValue: ProjectStore,
                WasiBlobstore: ProjectStore,
            }
        });

        /// Storage binding scoping one shared backing per project id.
        #[derive(Clone, Debug)]
        struct ProjectStore {
            project: String,
            keyvalue: KeyValueDefault,
            blobstore: BlobstoreDefault,
        }

        impl ProjectStore {
            fn scoped(&self, name: &str) -> String {
                format!("{}/{name}", self.project)
            }
        }

        /// Connection options naming the project this process serves.
        #[derive(Clone, Debug)]
        struct ProjectOptions {
            project: String,
        }

        impl FromEnv for ProjectOptions {
            fn load_env() -> anyhow::Result<Self> {
                let project = std::env::var("EMERY_PROJECT_ID")
                    .context("EMERY_PROJECT_ID must name the project to scope storage under")?;
                anyhow::ensure!(!project.is_empty(), "EMERY_PROJECT_ID must not be empty");
                Ok(Self { project })
            }
        }

        impl Backend for ProjectStore {
            type ConnectOptions = ProjectOptions;

            async fn connect_with(options: ProjectOptions) -> anyhow::Result<Self> {
                Ok(Self {
                    project: options.project,
                    keyvalue: KeyValueDefault::connect().await?,
                    blobstore: BlobstoreDefault::connect().await?,
                })
            }
        }

        impl WasiKeyValueCtx for ProjectStore {
            fn open_bucket(&self, identifier: String) -> FutureResult<Arc<dyn Bucket>> {
                self.keyvalue.open_bucket(self.scoped(&identifier))
            }
        }

        impl WasiBlobstoreCtx for ProjectStore {
            fn create_container(&self, name: String) -> FutureResult<Arc<dyn Container>> {
                self.blobstore.create_container(self.scoped(&name))
            }

            fn get_container(&self, name: String) -> FutureResult<Arc<dyn Container>> {
                self.blobstore.get_container(self.scoped(&name))
            }

            fn delete_container(&self, name: String) -> FutureResult<()> {
                self.blobstore.delete_container(self.scoped(&name))
            }

            fn container_exists(&self, name: String) -> FutureResult<bool> {
                self.blobstore.container_exists(self.scoped(&name))
            }
        }
    } else {
        fn main() {}
    }
}

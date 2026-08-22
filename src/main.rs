//! The shipped `emery` executable: one `omnia::runtime!` invocation.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        fn main() {}
    } else {
        use std::sync::Arc;

        use omnia::FutureResult;
        use omnia_cursor::Client as Cursor;
        use omnia_filesystem::Client as Filesystem;
        use omnia_wasi_blobstore::{Container, WasiBlobstore, WasiBlobstoreCtx};
        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_keyvalue::{Bucket, WasiKeyValue, WasiKeyValueCtx};
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

        omnia::runtime!({
            mode: command,
            guests: [
                {
                    id: "emery",
                    source: include_bytes!(concat!(env!("OUT_DIR"), "/emery.cwasm")),
                    routes: {http: ["/mcp/source/source"]},
                }
            ],
            mounts: [
                { name: ".", path: ".", writable: true },
                { name: emery_engine::handler::CACHE_MOUNT, path: cache_dir(), writable: true },
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

        fn cache_dir() -> &'static str {
            drop(std::fs::create_dir_all(".emery-cache"));
            ".emery-cache"
        }

        // The engine-state root under the invocation directory: the
        // filesystem backend serves `wasi:keyvalue` and `wasi:blobstore`
        // from disjoint subtrees (`keyvalue/`, `blobstore/`) beneath it.
        const STORE_ROOT: &str = ".emery";

        /// The default local binding of the engine's storage
        /// capabilities: a durable filesystem store rooted at
        /// [`STORE_ROOT`], with the engine's blob containers created at
        /// connect so the guest's `get-container` opens never race a
        /// first write. Alternative bindings are deployment profiles,
        /// not engine changes.
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
    }
}

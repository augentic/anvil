//! The shipped `emery` executable: one `omnia::runtime!` invocation.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        fn main() {}
    } else {
        use std::future::Future;
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

        // The engine-state root under the invocation directory: the
        // filesystem backend serves `wasi:keyvalue` and `wasi:blobstore`
        // from disjoint subtrees (`keyvalue/`, `blobstore/`) beneath it.
        const STORE_ROOT: &str = ".emery";

        /// The default local binding of the engine's storage
        /// capabilities: a durable filesystem store rooted at
        /// [`STORE_ROOT`] rather than the stock `FILESYSTEM_ROOT`
        /// environment contract — the one policy this newtype pins.
        /// Containers need no bootstrap: the backend's `get-container`
        /// is an ensure. Alternative bindings are deployment profiles,
        /// not engine changes.
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
    }
}

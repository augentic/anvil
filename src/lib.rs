//! The emery guest (wasm32) — the deployment's only `wasi:cli/run`
//! exporter. Native deployment policy lives inline in `src/main.rs`;
//! this library carries nothing on native targets.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use emery_engine::storage::Disk;
        use emery_transport::{command, http};
        use omnia_guest::api::invoke::Invoker;
        use omnia_guest::{BlobStore, CasError, ContainerMetadata, ObjectMetadata, StateStore};

        // Bare provider over the WASI capability defaults
        struct Provider;
        impl omnia_guest::Model for Provider {}
        impl emery_adapter::Source for Provider {}

        // Step-1 storage binding (design/portable-storage.md): engine
        // state stays on the filesystem preopens, so every method
        // overrides the `wasi:keyvalue` / `wasi:blobstore` default
        // body with the `Disk` delegate — the WASI imports are never
        // instantiated. Step 2 deletes these impl bodies to take the
        // defaults.
        impl StateStore for Provider {
            async fn get(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
                StateStore::get(&Disk::deployed(), key).await
            }

            async fn set(
                &self, key: &str, value: &[u8], ttl_secs: Option<u64>,
            ) -> anyhow::Result<Option<Vec<u8>>> {
                StateStore::set(&Disk::deployed(), key, value, ttl_secs).await
            }

            async fn delete(&self, key: &str) -> anyhow::Result<()> {
                StateStore::delete(&Disk::deployed(), key).await
            }

            async fn cas(
                &self, key: &str, expected: Option<&[u8]>, value: &[u8],
            ) -> Result<(), CasError> {
                StateStore::cas(&Disk::deployed(), key, expected, value).await
            }

            async fn increment(&self, key: &str, delta: i64) -> anyhow::Result<i64> {
                StateStore::increment(&Disk::deployed(), key, delta).await
            }
        }

        impl BlobStore for Provider {
            async fn get(&self, container: &str, name: &str) -> anyhow::Result<Option<Vec<u8>>> {
                BlobStore::get(&Disk::deployed(), container, name).await
            }

            async fn put(&self, container: &str, name: &str, data: &[u8]) -> anyhow::Result<()> {
                BlobStore::put(&Disk::deployed(), container, name, data).await
            }

            async fn delete(&self, container: &str, name: &str) -> anyhow::Result<()> {
                BlobStore::delete(&Disk::deployed(), container, name).await
            }

            async fn has(&self, container: &str, name: &str) -> anyhow::Result<bool> {
                BlobStore::has(&Disk::deployed(), container, name).await
            }

            async fn list(&self, container: &str) -> anyhow::Result<Vec<String>> {
                BlobStore::list(&Disk::deployed(), container).await
            }

            async fn get_range(
                &self, container: &str, name: &str, start: u64, end: u64,
            ) -> anyhow::Result<Vec<u8>> {
                BlobStore::get_range(&Disk::deployed(), container, name, start, end).await
            }

            async fn object_info(
                &self, container: &str, name: &str,
            ) -> anyhow::Result<ObjectMetadata> {
                BlobStore::object_info(&Disk::deployed(), container, name).await
            }

            async fn delete_objects(
                &self, container: &str, names: &[String],
            ) -> anyhow::Result<()> {
                BlobStore::delete_objects(&Disk::deployed(), container, names).await
            }

            async fn clear(&self, container: &str) -> anyhow::Result<()> {
                BlobStore::clear(&Disk::deployed(), container).await
            }

            async fn create_container(&self, name: &str) -> anyhow::Result<()> {
                BlobStore::create_container(&Disk::deployed(), name).await
            }

            async fn delete_container(&self, name: &str) -> anyhow::Result<()> {
                BlobStore::delete_container(&Disk::deployed(), name).await
            }

            async fn container_exists(&self, name: &str) -> anyhow::Result<bool> {
                BlobStore::container_exists(&Disk::deployed(), name).await
            }

            async fn container_info(&self, container: &str) -> anyhow::Result<ContainerMetadata> {
                BlobStore::container_info(&Disk::deployed(), container).await
            }

            async fn copy_object(
                &self, src_container: &str, src_name: &str, dest_container: &str, dest_name: &str,
            ) -> anyhow::Result<()> {
                BlobStore::copy_object(&Disk::deployed(), src_container, src_name, dest_container, dest_name)
                    .await
            }

            async fn move_object(
                &self, src_container: &str, src_name: &str, dest_container: &str, dest_name: &str,
            ) -> anyhow::Result<()> {
                BlobStore::move_object(&Disk::deployed(), src_container, src_name, dest_container, dest_name)
                    .await
            }
        }

        struct Cli;
        wasip3::cli::command::export!(Cli);

        impl wasip3::exports::cli::run::Guest for Cli {
            async fn run() -> Result<(), ()> {
                let router = command::router(Invoker::new("emery", Provider)).map_err(drop)?;
                omnia_guest::api::command::execute_wasi(&router).await
            }
        }

        struct Http;
        wasip3::http::service::export!(Http);

        impl wasip3::exports::http::handler::Guest for Http {
            async fn handle(
                request: wasip3::http::types::Request,
            ) -> Result<wasip3::http::types::Response, wasip3::http::types::ErrorCode> {
                omnia_wasi_http::serve(http::refusal(), request).await
            }
        }
    }
}

//! Source-adapter resolution: local-component mirroring into the
//! project cache and the adapter compatibility-floor gate.

mod selector;

use std::path::Path;

use emery_adapter::Source;
use emery_error::Error;
use omnia_guest::BlobStore;
pub use selector::AdapterSelector;

use crate::handler::preopen_path;
use crate::storage;

/// Blobstore container of the project component cache.
pub const ADAPTERS_CONTAINER: &str = "adapters";

// Mirrored component object name.
fn object(name: &str) -> String {
    format!("{name}.wasm")
}

/// Resolves a selector to its routed dispatch id
/// (`source:<name>[@<version>]`).
///
/// A local component mirrors into the project cache on the first
/// `specify` that names it; every resolution enforces the adapter's
/// declared `emery` compatibility floor.
///
/// # Errors
///
/// Returns selector, mirroring, floor, or storage failures.
pub async fn source<P: Source + BlobStore>(
    provider: &P, selector: &AdapterSelector,
) -> Result<String, Error> {
    if let AdapterSelector::Component { path } = selector {
        mirror(path, provider).await?;
    }
    let name = selector.name()?;
    let id = selector
        .version()
        .map_or_else(|| format!("source:{name}"), |version| format!("source:{name}@{version}"));
    let metadata = provider.metadata(&id);
    let floor = parse_floor(metadata.emery_floor.as_deref(), &name, &id)?;
    check_floor(floor.as_ref(), env!("CARGO_PKG_VERSION"), &name, &id)?;
    Ok(id)
}

// An existing mirror keeps the selector resolvable after the operator
// deletes the source file; a present file re-seeds the cache entry.
async fn mirror<B: BlobStore>(path: &Path, blobs: &B) -> Result<(), Error> {
    let relative = preopen_path(path, "<adapter>")?;
    if !relative.is_file() {
        let cached = match selector::name_from_component(&relative) {
            Ok(name) => blobs
                .has(ADAPTERS_CONTAINER, &object(&name))
                .await
                .map_err(|err| storage::failed("probing the component cache", &err))?,
            Err(_) => false,
        };
        if cached {
            return Ok(());
        }
    }
    seed(path, &relative, blobs).await
}

// Re-seeding replaces the entry; world validation stays a dispatch concern.
async fn seed<B: BlobStore>(original: &Path, relative: &Path, blobs: &B) -> Result<(), Error> {
    if !relative.is_file() || relative.extension().is_none_or(|ext| ext != "wasm") {
        return Err(Error::Diag {
            code: "adapter-component-missing",
            detail: format!(
                "adapter `{}` did not resolve to a `.wasm` component file at {} (an adapter \
                 is a single WebAssembly component)",
                original.display(),
                relative.display()
            ),
        });
    }
    let name = selector::name_from_component(relative)?;
    // Source reads use the workspace; mirrors use the storage capability.
    let bytes = std::fs::read(relative)?;
    blobs
        .put(ADAPTERS_CONTAINER, &object(&name), &bytes)
        .await
        .map_err(|err| storage::failed("mirroring the component into the cache", &err))
}

// A missing floor admits; a malformed floor refuses typed.
fn parse_floor(
    floor: Option<&str>, name: &str, id: &str,
) -> Result<Option<semver::Version>, Error> {
    let Some(floor) = floor else {
        return Ok(None);
    };
    semver::Version::parse(floor).map(Some).map_err(|err| {
        Error::validation_failed(
            "adapter-floor-malformed",
            "an adapter's metadata answer declares a semver `emery-floor`",
            format!(
                "adapter `{name}` ({id}) declares `emery-floor: {floor}`, which is not an \
                 exact semver: {err}"
            ),
        )
    })
}

// An unparseable running version is permissive to preserve recovery.
fn check_floor(
    floor: Option<&semver::Version>, current: &str, name: &str, id: &str,
) -> Result<(), Error> {
    let Some(floor) = floor else {
        return Ok(());
    };
    let Ok(current_version) = semver::Version::parse(current) else {
        return Ok(());
    };
    if current_version < *floor {
        return Err(Error::AdapterCliTooOld {
            adapter: format!("{name} ({id})"),
            required: floor.to_string(),
            found: current.to_string(),
        });
    }
    Ok(())
}

// Keep (CLI-unreachable defensive branch): production `current` is the
// binary's own always-parseable `env!("CARGO_PKG_VERSION")`, so no CLI
// input can reach the permissive unparseable-version arm.
#[cfg(test)]
mod tests {
    use super::check_floor;

    #[test]
    fn unparseable_permissive() {
        let floor = semver::Version::new(2, 0, 0);

        check_floor(Some(&floor), "not-a-version", "demo-source", "source:demo-source")
            .expect("an unparseable running version must not brick resolution");
    }
}

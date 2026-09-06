//! Source-adapter resolution: loader-backed local components and
//! registry packages, plus the adapter compatibility-floor gate.

mod selector;

use std::path::Path;

use emery_source::Source;
use omnia_guest::plugins::{Digest, Error as LoadError, Location, PluginRef};
use omnia_guest::{Error, Plugins, bad_request, not_found};
pub use selector::AdapterSelector;

use crate::preopen::preopen_path;

/// Resolves a selector to its routed dispatch id, loading a local
/// component or registry package through the deployment's loader and
/// enforcing the adapter's `emery` compatibility floor.
///
/// # Errors
///
/// Returns selector, load, or floor failures.
pub async fn source<P: Source + Plugins>(
    provider: &P, selector: &AdapterSelector, pin: Option<&Digest>, registry: Option<&str>,
) -> Result<Resolved, Error> {
    let name = selector.name()?;
    let resolved = match selector {
        AdapterSelector::Package {
            namespace,
            name,
            version,
        } => {
            let request = PluginRef::builder()
                .package(format!("{namespace}:{name}@{version}"))
                .location(Location::Registry(registry.map(ToOwned::to_owned)))
                .maybe_digest(pin.cloned())
                .build();
            let plugin = Plugins::load(provider, &request).await?;
            Resolved {
                id: plugin.id().to_owned(),
                digest: Some(plugin.digest().clone()),
            }
        }
        AdapterSelector::Component { path } => {
            let id = format!("source:{name}");
            let digest = load(provider, &id, path, pin).await?;
            Resolved {
                id,
                digest: Some(digest),
            }
        }
        AdapterSelector::Bare { .. } => Resolved {
            id: format!("source:{name}"),
            digest: None,
        },
    };

    let metadata = Source::metadata(provider, &resolved.id);
    let floor = parse_floor(metadata.emery_floor.as_deref(), &name, &resolved.id)?;
    check_floor(floor.as_ref(), env!("CARGO_PKG_VERSION"), &name, &resolved.id)?;

    Ok(resolved)
}

/// One resolved source binding: the routed dispatch id plus, for a
/// loader-loaded adapter, its resolved content digest.
#[derive(Debug, Clone)]
pub struct Resolved {
    /// Routed dispatch id: the package reference for a registry
    /// package, `source:<name>` otherwise.
    pub id: String,
    /// Resolved sha256 digest of the loaded component bytes.
    pub digest: Option<Digest>,
}

impl Resolved {
    // Refuses a pin that disagrees with an already-loaded guest.
    pub(crate) fn pin_agrees(&self, pin: Option<&Digest>) -> Result<(), Error> {
        match (pin, self.digest.as_ref()) {
            (Some(pin), Some(held)) if pin != held => Err(LoadError::AlreadyActive(format!(
                "package `{}` is already active with digest {held}, which is not the requested pin",
                self.id
            ))
            .into()),
            _ => Ok(()),
        }
    }
}

// The loader reads the file fresh — nothing is mirrored, so a deleted
// file refuses on the next run. The engine keeps only the operator-typo
// gate: a missing or non-component path refuses typed before any load.
async fn load<P: Plugins>(
    provider: &P, id: &str, path: &Path, pin: Option<&Digest>,
) -> Result<Digest, Error> {
    let relative = preopen_path(path)?;
    if !relative.is_file() || relative.extension().is_none_or(|ext| ext != "wasm") {
        return Err(not_found!(
            "adapter `{}` did not resolve to a `.wasm` component file at {} (an adapter is a \
             single WebAssembly component)",
            path.display(),
            relative.display()
        ));
    }

    let request = PluginRef::builder()
        .package(id)
        .location(Location::Path(relative.display().to_string()))
        .maybe_digest(pin.cloned())
        .build();
    let plugin = Plugins::load(provider, &request).await?;

    Ok(plugin.digest().clone())
}

// A missing floor admits; a malformed floor refuses typed.
fn parse_floor(
    floor: Option<&str>, name: &str, id: &str,
) -> Result<Option<semver::Version>, Error> {
    let Some(floor) = floor else {
        return Ok(None);
    };
    semver::Version::parse(floor).map(Some).map_err(|err| {
        bad_request!(
            "an adapter's metadata answer declares a semver `emery-floor`: adapter `{name}` \
             ({id}) declares `emery-floor: {floor}`, which is not an exact semver: {err}",
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
        return Err(Error::BadRequest {
            code: "adapter-cli-too-old".into(),
            description: format!(
                "emery version {current} is older than the floor {floor} required by adapter \
                 {name} ({id}); a newer emery is required"
            ),
        });
    }
    Ok(())
}

// Keep (entry-point-unreachable defensive branch): production `current`
// is the binary's own always-parseable `env!("CARGO_PKG_VERSION")`, so
// no operator input can reach the permissive unparseable-version arm.
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

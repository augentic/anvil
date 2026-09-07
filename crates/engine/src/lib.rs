//! Emery's specification-generation engine.

mod extract;
mod plugin;
pub mod show;
pub mod sources;
mod spec;
pub mod specify;
mod store;
mod synthesise;

use std::path::{Component, Path, PathBuf};

use emery_source::Source;
use omnia_guest::{BlobStore, Error, Model, Plugins, StateStore, bad_request};
pub use plugin::AdapterRef;
pub use store::{CONTAINER, CURRENT};

/// Normalizes an operator path inside the `.` project preopen.
///
/// # Errors
///
/// Returns a `BadRequest` for an absolute path or a relative path that
/// escapes above the project root.
pub fn preopen_path(path: &Path) -> Result<PathBuf, Error> {
    if path.is_absolute() {
        return Err(bad_request!("path `{}` must be relative to the project root", path.display()));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir if normalized.pop() => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(bad_request!(
                    "path `{}` must be relative to the project root",
                    path.display()
                ));
            }
        }
    }

    Ok(if normalized.as_os_str().is_empty() { PathBuf::from(".") } else { normalized })
}

/// The capability set every operation can be dispatched over, as one
/// bound for the transports that bind a provider.
pub trait Provider:
    Model + Source + StateStore + BlobStore + Plugins + Send + Sync + 'static
{
}

impl<P: Model + Source + StateStore + BlobStore + Plugins + Send + Sync + 'static> Provider for P {}

// Generated from the link-checked synthesis corpus at build time.
mod prose {
    emery_prose::registry!();
}

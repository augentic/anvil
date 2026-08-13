//! Host-supplied adapter catalog, recognition matching, and source keys.
//!
//! The kernel consumes a [`Catalog`]; hosts compile or replace the
//! table at composition time. `intent` is explicit-only.

mod key;
mod pin;
mod profile;

use std::path::Path;

use error::Error;
pub use key::{Row, assign, identity};
pub use pin::Pin;
pub use profile::Profile;

use crate::Platform;
use crate::adapter::PlatformsCapability;
use crate::binding::Origin;

/// Reserved source-adapter name and source key.
pub const INTENT: &str = "intent";

/// Compiled first-party adapter version (declared starting value).
const VERSION: semver::Version = semver::Version::new(0, 12, 0);

/// Host-supplied bounded adapter inventory (RFC-88 D6).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Catalog {
    /// Source adapters and their recognition rules.
    pub sources: Vec<Source>,
    /// Target adapters and their platform constraints.
    pub targets: Vec<Target>,
}

/// One source-adapter catalog row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    /// Exact package pin recorded when this adapter is selected.
    pub pin: Pin,
    /// How this adapter is chosen.
    pub recognition: Recognition,
}

/// How a source adapter may be selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recognition {
    /// Never fingerprinted; the handoff must pin it (`intent`).
    Explicit,
    /// Deterministic probes over a staged source value.
    Profile(Profile),
}

/// One target-adapter catalog row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    /// Exact package pin recorded when this adapter is selected.
    pub pin: Pin,
    /// Platform constraints from the adapter's metadata answer.
    pub platforms: Option<PlatformsCapability>,
}

/// Adapter hint for [`select`]: reuse a pin or fingerprint a tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hint<'a> {
    /// Exact pin already carried by the handoff.
    Pin(&'a str),
    /// Newly focused source; fingerprint `root`.
    Open(&'a Path),
}

impl Catalog {
    /// Validate unique names, axis uniqueness, and `intent` explicitness.
    ///
    /// # Errors
    ///
    /// `adapter-catalog-invalid` when a name repeats or `intent` is
    /// fingerprinted.
    pub fn new(sources: Vec<Source>, targets: Vec<Target>) -> Result<Self, Error> {
        let catalog = Self { sources, targets };
        catalog.check()?;
        Ok(catalog)
    }

    /// Compiled first-party catalog: `intent` plus fingerprintable
    /// sources and the three target adapters.
    ///
    /// # Panics
    ///
    /// Never: the compiled table is statically valid.
    #[must_use]
    pub fn first_party() -> Self {
        Self::new(first_party_sources(), first_party_targets())
            .expect("the first-party catalog is statically valid")
    }

    fn check(&self) -> Result<(), Error> {
        let mut names = std::collections::BTreeSet::new();
        for source in &self.sources {
            if !names.insert(source.pin.name.as_str()) {
                return Err(invalid(format!("duplicate adapter name `{}`", source.pin.name)));
            }
            if source.pin.name == INTENT && !matches!(source.recognition, Recognition::Explicit) {
                return Err(invalid("adapter `intent` is explicit-only".into()));
            }
            if source.pin.name != INTENT && matches!(source.recognition, Recognition::Explicit) {
                return Err(invalid(format!(
                    "source `{}` must carry a recognition profile",
                    source.pin.name
                )));
            }
        }
        for target in &self.targets {
            if !names.insert(target.pin.name.as_str()) {
                return Err(invalid(format!("duplicate adapter name `{}`", target.pin.name)));
            }
        }
        Ok(())
    }
}

/// Select an exact source pin: reuse a handoff pin, or fingerprint.
///
/// # Errors
///
/// `adapter-unversioned` for a bare name or local component;
/// `source-intent-locator` when `intent` carries a locator;
/// `source-adapter-no-match` / `source-adapter-ambiguous` on
/// fingerprint failure.
pub fn select(catalog: &Catalog, hint: Hint<'_>, origin: &Origin) -> Result<Pin, Error> {
    match hint {
        Hint::Pin(raw) => reuse(raw, origin),
        Hint::Open(root) => fingerprint(catalog, root, origin),
    }
}

fn reuse(raw: &str, origin: &Origin) -> Result<Pin, Error> {
    let pin = Pin::parse(raw)?;
    if pin.name == INTENT && matches!(origin, Origin::Location(_)) {
        return Err(key::intent_locator());
    }
    Ok(pin)
}

fn fingerprint(catalog: &Catalog, root: &Path, origin: &Origin) -> Result<Pin, Error> {
    if matches!(origin, Origin::Value(_)) {
        return Err(no_match(root));
    }
    let mut hits = Vec::new();
    for source in &catalog.sources {
        let Recognition::Profile(profile) = &source.recognition else {
            continue;
        };
        if profile.matches(root)? {
            hits.push(source);
        }
    }
    match hits.as_slice() {
        [source] => Ok(source.pin.clone()),
        [] => Err(no_match(root)),
        _ => {
            let names: Vec<&str> = hits.iter().map(|source| source.pin.name.as_str()).collect();
            Err(Error::Diag {
                code: "source-adapter-ambiguous",
                detail: format!(
                    "source at `{}` matches multiple adapters [{}]; pin one adapter on the reviewed handoff",
                    root.display(),
                    names.join(", ")
                ),
            })
        }
    }
}

fn no_match(root: &Path) -> Error {
    Error::Diag {
        code: "source-adapter-no-match",
        detail: format!(
            "source at `{}` matches no recognition profile; pin an adapter on the reviewed handoff",
            root.display()
        ),
    }
}

const fn invalid(detail: String) -> Error {
    Error::Diag {
        code: "adapter-catalog-invalid",
        detail,
    }
}

fn first_party_sources() -> Vec<Source> {
    vec![
        Source {
            pin: Pin::emery(INTENT, VERSION),
            recognition: Recognition::Explicit,
        },
        Source {
            pin: Pin::emery("typescript", VERSION),
            recognition: Recognition::Profile(Profile {
                paths: vec!["package.json".into(), "tsconfig.json".into(), "jsconfig.json".into()],
                extensions: vec![
                    "ts".into(),
                    "tsx".into(),
                    "js".into(),
                    "jsx".into(),
                    "mts".into(),
                    "cts".into(),
                    "mjs".into(),
                    "cjs".into(),
                ],
            }),
        },
        Source {
            pin: Pin::emery("documentation", VERSION),
            recognition: Recognition::Profile(Profile {
                paths: Vec::new(),
                extensions: vec!["md".into(), "mdx".into(), "rst".into(), "adoc".into()],
            }),
        },
        Source {
            pin: Pin::emery("screenshots", VERSION),
            recognition: Recognition::Profile(Profile {
                paths: Vec::new(),
                extensions: vec![
                    "png".into(),
                    "jpg".into(),
                    "jpeg".into(),
                    "webp".into(),
                    "gif".into(),
                ],
            }),
        },
        Source {
            pin: Pin::emery("captures", VERSION),
            recognition: Recognition::Profile(Profile {
                paths: vec!["tests/data/replays".into()],
                extensions: Vec::new(),
            }),
        },
    ]
}

fn first_party_targets() -> Vec<Target> {
    vec![
        Target {
            pin: Pin::emery("omnia", VERSION),
            platforms: None,
        },
        Target {
            pin: Pin::emery("vectis", VERSION),
            platforms: Some(PlatformsCapability {
                required: true,
                allowed: vec![
                    Platform::Core,
                    Platform::Ios,
                    Platform::Android,
                    Platform::Web,
                    Platform::Desktop,
                ],
                default: vec![Platform::Core, Platform::Ios, Platform::Android],
            }),
        },
        Target {
            pin: Pin::emery("contracts", VERSION),
            platforms: None,
        },
    ]
}

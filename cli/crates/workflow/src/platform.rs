//! Closed platform enum — the set of target platforms a project may
//! declare in `project.yaml`.

mod bootstrap;
mod detect;

pub use bootstrap::{BootstrapContext, bootstrap_context, bootstrap_context_from_missing};
pub use detect::vectis_missing_platforms;
use serde::{Deserialize, Serialize};

/// Target platform for a Specify project.
///
/// `Core` is the shared Rust business-logic crate; every project that
/// declares platforms must include it. The shell variants (`Ios`,
/// `Android`, `Web`, `Desktop`) represent native presentation layers.
///
/// Only `Ios` and `Android` have scaffold/build/verify support today;
/// `Web` and `Desktop` are type-system placeholders signalling future
/// functionality.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Platform {
    /// Shared Rust business-logic crate (mandatory in every platform set).
    Core,
    /// iOS native shell (Swift + UIKit/SwiftUI).
    Ios,
    /// Android native shell (Kotlin + Compose/Views).
    Android,
    /// Web shell (future — accepted but no build/scaffold support yet).
    Web,
    /// Desktop shell (future — accepted but no build/scaffold support yet).
    Desktop,
}

/// Parse a comma-separated platform string into a sorted, deduplicated
/// `Vec<Platform>`. Returns an error naming the first unknown token.
///
/// # Errors
///
/// Returns a human-readable `String` when any token is not a valid
/// [`Platform`] variant.
pub fn parse_platforms_csv(csv: &str) -> Result<Vec<Platform>, String> {
    let mut platforms: Vec<Platform> = csv
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|token| {
            token.parse::<Platform>().map_err(|_err| {
                format!(
                    "unknown platform `{token}`; expected one of: core, ios, android, web, desktop"
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    platforms.sort();
    platforms.dedup();
    Ok(platforms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_names_coherent() {
        // The strum-derived `Display` / `FromStr` (`serialize_all =
        // "kebab-case"`) must not drift from the serde
        // `#[serde(rename_all = "kebab-case")]` wire name.
        for platform in
            [Platform::Core, Platform::Ios, Platform::Android, Platform::Web, Platform::Desktop]
        {
            let name = platform.to_string();
            assert_eq!(name.parse::<Platform>().unwrap(), platform, "FromStr(Display) round trip");
            let yaml = serde_saphyr::to_string(&platform).unwrap();
            assert_eq!(yaml.trim(), name, "serde wire name must match Display");
        }
    }

    #[test]
    fn parse_csv_edge_cases() {
        // Whitespace and empty tokens are tolerated; duplicates collapse;
        // output is sorted with `Core` first; the first unknown token is
        // named in the error.
        let platforms = parse_platforms_csv(" android , core ,ios,core,").unwrap();
        assert_eq!(platforms, vec![Platform::Core, Platform::Ios, Platform::Android]);

        let err = parse_platforms_csv("core,windows").unwrap_err();
        assert!(err.contains("windows"), "error should name the bad token: {err}");
    }
}

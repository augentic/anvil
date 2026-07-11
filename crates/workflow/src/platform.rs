//! Closed platform enum — the set of target platforms a project may
//! declare in `project.yaml`.

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
/// ```
/// use workflow::platform::{Platform, parse_platforms_csv};
///
/// let set = parse_platforms_csv("ios, core, ios").unwrap();
/// assert_eq!(set, [Platform::Core, Platform::Ios]);
/// assert!(parse_platforms_csv("core,vision-os").is_err());
/// ```
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

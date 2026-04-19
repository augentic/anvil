//! Embedded chunk-3b iOS templates plus their source→target path mapping.
//!
//! The slice mirrors `templates/vectis/ios/MANIFEST.md` § Path mapping. When
//! adding a new iOS template:
//!
//! 1. Drop the file under `templates/vectis/ios/` with a flat name.
//! 2. Append a row here with the matching target path (placeholders allowed
//!    in directory and file-name positions -- see notes below).
//! 3. Update the manifest's path-mapping table and self-check diff.
//!
//! Unlike the core registry, target paths embed `__APP_NAME__` in directory
//! and file-name positions (e.g. `iOS/__APP_NAME__/__APP_NAME__App.swift`).
//! The scaffolder applies placeholder substitution to the constructed target
//! path string before writing -- the on-disk template layout itself stays
//! flat regardless.

/// One template entry: source filename (matches `include_str!`) and the
/// path the engine should write to under the project directory. The target
/// path may contain `__APP_NAME__` and `__APP_NAME_LOWER__` placeholders --
/// they are substituted at scaffold time, the same way file contents are.
pub struct IosTemplate {
    pub target: &'static str,
    pub contents: &'static str,
}

/// Embedded iOS registry. Order is the order files are written, which is
/// also the order the JSON output reports them in (matches the RFC's example
/// `vectis init` output for iOS shells).
pub const TEMPLATES: &[IosTemplate] = &[
    IosTemplate {
        target: "iOS/project.yml",
        contents: include_str!("../../../../templates/vectis/ios/project.yml"),
    },
    IosTemplate {
        target: "iOS/Makefile",
        contents: include_str!("../../../../templates/vectis/ios/Makefile"),
    },
    IosTemplate {
        target: "iOS/__APP_NAME__/__APP_NAME__App.swift",
        contents: include_str!("../../../../templates/vectis/ios/App.swift"),
    },
    IosTemplate {
        target: "iOS/__APP_NAME__/Core.swift",
        contents: include_str!("../../../../templates/vectis/ios/Core.swift"),
    },
    IosTemplate {
        target: "iOS/__APP_NAME__/ContentView.swift",
        contents: include_str!("../../../../templates/vectis/ios/ContentView.swift"),
    },
    IosTemplate {
        target: "iOS/__APP_NAME__/Views/LoadingScreen.swift",
        contents: include_str!("../../../../templates/vectis/ios/LoadingScreen.swift"),
    },
    IosTemplate {
        target: "iOS/__APP_NAME__/Views/HomeScreen.swift",
        contents: include_str!("../../../../templates/vectis/ios/HomeScreen.swift"),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_matches_rfc_ios_file_count() {
        // RFC-6 § File Manifests § iOS Assembly enumerates 7 files.
        assert_eq!(TEMPLATES.len(), 7);
    }

    #[test]
    fn registry_targets_are_unique() {
        let mut targets: Vec<&str> = TEMPLATES.iter().map(|t| t.target).collect();
        targets.sort_unstable();
        let len_before = targets.len();
        targets.dedup();
        assert_eq!(
            targets.len(),
            len_before,
            "duplicate target paths in iOS registry"
        );
    }

    #[test]
    fn every_target_is_under_ios_dir() {
        for entry in TEMPLATES {
            assert!(
                entry.target.starts_with("iOS/"),
                "target {:?} is not under iOS/",
                entry.target
            );
        }
    }
}

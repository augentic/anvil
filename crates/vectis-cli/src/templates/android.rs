//! Embedded chunk-3c Android templates plus their source→target path mapping.
//!
//! The slice mirrors `templates/vectis/android/MANIFEST.md` § Path mapping.
//! When adding a new Android template:
//!
//! 1. Drop the file under `templates/vectis/android/` with a flat name.
//! 2. Append a row here with the matching target path (placeholders allowed
//!    in directory and file-name positions -- see notes below).
//! 3. Update the manifest's path-mapping table and self-check diff.
//!
//! Target paths embed `__APP_NAME__` and `__ANDROID_PACKAGE_PATH__` in
//! directory and file-name positions (e.g.
//! `Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/__APP_NAME__Application.kt`).
//! The scaffolder applies path-segment substitution via
//! [`crate::templates::substitute_path_with`] before opening each file --
//! the on-disk template layout itself stays flat regardless.
//!
//! Whole-file conditionals (only `network-security-config.xml` today) live
//! on [`AndroidTemplate::include_when`]. The scaffolder evaluates this
//! against the active capability set and skips the entry entirely when the
//! predicate fails. CAP markers *inside* a file are still handled by the
//! shared marker engine; the include predicate is for the case where the
//! whole file's existence is conditional.

use crate::templates::Capability;

/// One Android template entry: source filename (matches `include_str!`),
/// the target path (with optional placeholders), and an inclusion
/// predicate that decides whether this entry is rendered for the active
/// capability set.
pub struct AndroidTemplate {
    pub target: &'static str,
    pub contents: &'static str,
    /// Whole-file inclusion predicate. `Always` means the entry is rendered
    /// for every cap selection; the `AnyOf` variant skips the entry unless
    /// at least one of the listed caps is present. CAP-marker conditionals
    /// inside the file are independent of this predicate.
    pub include_when: IncludeWhen,
}

/// Whole-file inclusion predicate (chunk 3c MANIFEST § Cap-marker reference
/// -- "whole-file conditional" callout).
#[derive(Debug, Clone, Copy)]
pub enum IncludeWhen {
    /// File is rendered regardless of `--caps`. Most templates fall here.
    Always,
    /// File is rendered iff at least one of the listed caps is present in
    /// `caps`. Used for `network-security-config.xml`, which is only
    /// referenced from `AndroidManifest.xml` when HTTP or SSE is on (and
    /// would be a dead resource otherwise).
    AnyOf(&'static [Capability]),
}

impl IncludeWhen {
    /// Should this entry be rendered for the given cap selection?
    pub fn should_include(self, caps: &[Capability]) -> bool {
        match self {
            IncludeWhen::Always => true,
            IncludeWhen::AnyOf(needed) => needed.iter().any(|c| caps.contains(c)),
        }
    }
}

/// Embedded Android registry. Order is the order files are written, which
/// is also the order the JSON output reports them in (matches the chunk-3c
/// MANIFEST § Path mapping order).
pub const TEMPLATES: &[AndroidTemplate] = &[
    AndroidTemplate {
        target: "Android/Makefile",
        contents: include_str!("../../../../templates/vectis/android/Makefile"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/.gitignore",
        contents: include_str!("../../../../templates/vectis/android/gitignore"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/build.gradle.kts",
        contents: include_str!("../../../../templates/vectis/android/root-build.gradle.kts"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/settings.gradle.kts",
        contents: include_str!("../../../../templates/vectis/android/settings.gradle.kts"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/gradle.properties",
        contents: include_str!("../../../../templates/vectis/android/gradle.properties"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/gradle/libs.versions.toml",
        contents: include_str!("../../../../templates/vectis/android/libs.versions.toml"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/app/build.gradle.kts",
        contents: include_str!("../../../../templates/vectis/android/app-build.gradle.kts"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/shared/build.gradle.kts",
        contents: include_str!("../../../../templates/vectis/android/shared-build.gradle.kts"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/app/src/main/AndroidManifest.xml",
        contents: include_str!("../../../../templates/vectis/android/AndroidManifest.xml"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/app/src/main/res/values/themes.xml",
        contents: include_str!("../../../../templates/vectis/android/themes.xml"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/app/src/main/res/xml/network_security_config.xml",
        contents: include_str!(
            "../../../../templates/vectis/android/network-security-config.xml"
        ),
        // Only meaningful when HTTP or SSE is on (the manifest only
        // references `@xml/network_security_config` from inside its own
        // `<<<CAP:http` block). Leaving the file in render-only / kv-only
        // builds yields a dead resource; dropping the file keeps the
        // resource set tight.
        include_when: IncludeWhen::AnyOf(&[Capability::Http, Capability::Sse]),
    },
    AndroidTemplate {
        target: "Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/__APP_NAME__Application.kt",
        contents: include_str!("../../../../templates/vectis/android/Application.kt"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/MainActivity.kt",
        contents: include_str!("../../../../templates/vectis/android/MainActivity.kt"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/core/Core.kt",
        contents: include_str!("../../../../templates/vectis/android/Core.kt"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/ui/screens/LoadingScreen.kt",
        contents: include_str!("../../../../templates/vectis/android/LoadingScreen.kt"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/ui/screens/HomeScreen.kt",
        contents: include_str!("../../../../templates/vectis/android/HomeScreen.kt"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/ui/theme/Color.kt",
        contents: include_str!("../../../../templates/vectis/android/Color.kt"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/ui/theme/Theme.kt",
        contents: include_str!("../../../../templates/vectis/android/Theme.kt"),
        include_when: IncludeWhen::Always,
    },
    AndroidTemplate {
        target: "Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/ui/theme/Type.kt",
        contents: include_str!("../../../../templates/vectis/android/Type.kt"),
        include_when: IncludeWhen::Always,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_matches_rfc_android_file_count() {
        // RFC-6 § File Manifests § Android Assembly enumerates 19 files.
        assert_eq!(TEMPLATES.len(), 19);
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
            "duplicate target paths in Android registry"
        );
    }

    #[test]
    fn every_target_is_under_android_dir() {
        for entry in TEMPLATES {
            assert!(
                entry.target.starts_with("Android/"),
                "target {:?} is not under Android/",
                entry.target
            );
        }
    }

    #[test]
    fn network_security_config_is_http_or_sse_conditional() {
        let entry = TEMPLATES
            .iter()
            .find(|t| t.target.ends_with("network_security_config.xml"))
            .expect("network_security_config.xml must be in registry");
        match entry.include_when {
            IncludeWhen::AnyOf(caps) => {
                assert!(caps.contains(&Capability::Http));
                assert!(caps.contains(&Capability::Sse));
            }
            IncludeWhen::Always => {
                panic!("network_security_config.xml must be cap-conditional, not Always")
            }
        }
    }

    #[test]
    fn include_when_any_of_skips_when_no_overlap() {
        let pred = IncludeWhen::AnyOf(&[Capability::Http, Capability::Sse]);
        assert!(!pred.should_include(&[]));
        assert!(!pred.should_include(&[Capability::Kv]));
        assert!(pred.should_include(&[Capability::Http]));
        assert!(pred.should_include(&[Capability::Sse]));
        assert!(pred.should_include(&[Capability::Kv, Capability::Http]));
    }

    #[test]
    fn include_when_always_is_unconditional() {
        let pred = IncludeWhen::Always;
        assert!(pred.should_include(&[]));
        assert!(pred.should_include(&[Capability::Http]));
    }
}

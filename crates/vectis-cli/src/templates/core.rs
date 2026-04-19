//! Embedded chunk-3a core templates plus their source→target path mapping.
//!
//! The slice mirrors `templates/vectis/core/MANIFEST.md` § Path mapping. When
//! adding a new core template:
//!
//! 1. Drop the file under `templates/vectis/core/` with a flat name.
//! 2. Append a row here with the matching target path.
//! 3. Update the manifest's path-mapping table and self-check diff.
//!
//! Target paths are static strings -- no `__APP_NAME__` substitution today
//! (chunk 3a deliberately put every per-app placeholder in file *contents*,
//! not directory names). iOS / Android (chunks 7 / 8) need path-segment
//! substitution and add it on their own registries.

/// One template entry: source filename (matches `include_str!`) and the
/// path the engine should write to under the project directory.
pub struct CoreTemplate {
    pub target: &'static str,
    pub contents: &'static str,
}

/// Embedded core registry. Order is the order files are written, which is
/// also the order the JSON output reports them in (matches the RFC's example
/// `vectis init` output).
pub const TEMPLATES: &[CoreTemplate] = &[
    CoreTemplate {
        target: "Cargo.toml",
        contents: include_str!("../../../../templates/vectis/core/workspace-cargo.toml"),
    },
    CoreTemplate {
        target: "clippy.toml",
        contents: include_str!("../../../../templates/vectis/core/clippy.toml"),
    },
    CoreTemplate {
        target: "rust-toolchain.toml",
        contents: include_str!("../../../../templates/vectis/core/rust-toolchain.toml"),
    },
    CoreTemplate {
        target: ".gitignore",
        contents: include_str!("../../../../templates/vectis/core/gitignore"),
    },
    CoreTemplate {
        target: "shared/Cargo.toml",
        contents: include_str!("../../../../templates/vectis/core/shared-cargo.toml"),
    },
    CoreTemplate {
        target: "shared/src/lib.rs",
        contents: include_str!("../../../../templates/vectis/core/lib.rs"),
    },
    CoreTemplate {
        target: "shared/src/app.rs",
        contents: include_str!("../../../../templates/vectis/core/app.rs"),
    },
    CoreTemplate {
        target: "shared/src/ffi.rs",
        contents: include_str!("../../../../templates/vectis/core/ffi.rs"),
    },
    CoreTemplate {
        target: "shared/src/bin/codegen.rs",
        contents: include_str!("../../../../templates/vectis/core/codegen.rs"),
    },
    CoreTemplate {
        target: "deny.toml",
        contents: include_str!("../../../../templates/vectis/core/deny.toml"),
    },
    CoreTemplate {
        target: "supply-chain/config.toml",
        contents: include_str!("../../../../templates/vectis/core/supply-chain-config.toml"),
    },
    CoreTemplate {
        target: "supply-chain/audits.toml",
        contents: include_str!("../../../../templates/vectis/core/supply-chain-audits.toml"),
    },
    CoreTemplate {
        target: "supply-chain/imports.lock",
        contents: include_str!("../../../../templates/vectis/core/supply-chain-imports.lock"),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_matches_rfc_core_file_count() {
        // RFC-6 § File Manifests § Core Assembly enumerates 13 files.
        assert_eq!(TEMPLATES.len(), 13);
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
            "duplicate target paths in core registry"
        );
    }
}

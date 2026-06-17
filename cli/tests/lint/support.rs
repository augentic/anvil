//! Shared fixture scaffold for the `specify lint framework` suites
//! (`framework.rs`, `framework_json.rs`).

use std::fs;
use std::path::Path;

/// Write the minimal directory and file scaffold that
/// `Context::from_framework_root` requires *and* that silences every
/// non-codex authoring check on an otherwise empty tree.
///
/// Specifically the scaffold:
///
/// - Creates `plugins/`, `adapters/{sources,targets,shared}/` so the
///   path passes `is_framework_root`.
/// - Writes a structurally-valid `.cursor-plugin/marketplace.json`
///   carrying a single synthetic `test` plugin entry so the
///   `plugins.marketplace-drift` schema (`minItems: 1`) is satisfied
///   without dragging real plugin content into the tree.
/// - Writes the matching `plugins/test/.cursor-plugin/plugin.json`
///   plus an empty `plugins/test/skills/` directory so the
///   `marketplace` framework tool finds the manifest the marketplace
///   declares.
/// - Writes `docs/standards/skill-authoring.md` containing the literal
///   `512` (description cap) and `200` (body cap) tokens so
///   `prose.numeric-cap-exceeded` short-circuits (the description cap is
///   cross-checked against the embedded `skill.schema.json`).
/// - Writes `docs/reference/review-team-protocol.md` so the
///   `agent-teams.missing-canonical` predicate has a canonical doc
///   to hash against; per-target `references/agent-teams.md` files
///   are never created so the per-adapter overlay arm short-circuits.
pub fn scaffold_framework(root: &Path) {
    for rel in [
        "adapters/sources",
        "adapters/targets",
        "adapters/shared",
        "plugins",
        "plugins/test/skills",
    ] {
        fs::create_dir_all(root.join(rel)).expect("scaffold dir");
    }

    let marketplace = root.join(".cursor-plugin").join("marketplace.json");
    fs::create_dir_all(marketplace.parent().expect("marketplace parent"))
        .expect("mkdir .cursor-plugin");
    fs::write(
        &marketplace,
        r#"{
  "name": "test",
  "owner": { "name": "Test Owner", "email": "test@example.com" },
  "metadata": {
    "description": "Synthetic marketplace for specify lint framework tests.",
    "version": "0.0.0",
    "pluginRoot": "plugins"
  },
  "plugins": [
    {
      "name": "test",
      "source": "test",
      "description": "Synthetic plugin used by specify lint framework tests."
    }
  ]
}
"#,
    )
    .expect("marketplace.json");

    let plugin_manifest =
        root.join("plugins").join("test").join(".cursor-plugin").join("plugin.json");
    fs::create_dir_all(plugin_manifest.parent().expect("plugin manifest parent"))
        .expect("mkdir plugins/test/.cursor-plugin");
    fs::write(
        &plugin_manifest,
        r#"{
  "name": "test",
  "displayName": "Test Plugin",
  "description": "Synthetic plugin used by specify lint framework tests.",
  "version": "0.0.0"
}
"#,
    )
    .expect("plugins/test/.cursor-plugin/plugin.json");

    let standards = root.join("docs").join("standards").join("skill-authoring.md");
    fs::create_dir_all(standards.parent().expect("standards parent"))
        .expect("mkdir docs/standards");
    fs::write(
        &standards,
        "# Skill authoring (synthetic)\n\nDescription cap: 512 characters. Body cap: 200 lines.\n",
    )
    .expect("skill-authoring.md");

    let canonical = root.join("docs").join("reference").join("review-team-protocol.md");
    fs::create_dir_all(canonical.parent().expect("canonical parent"))
        .expect("mkdir docs/reference");
    fs::write(&canonical, "# Review Team Protocol\n\nSynthetic stub for tests.\n")
        .expect("review-team-protocol.md");
}

//! Template engine -- placeholder substitution and capability-conditional
//! sections, plus per-assembly template registries.
//!
//! Chunk 5 landed the engine (placeholder substitution + the include-when-
//! selected evaluator) and the core registry. Chunk 6 wires the
//! `Capability` enum through `init::run` so every variant is now
//! actively constructed. iOS / Android registries follow in chunks 7 / 8
//! (same engine, different `TEMPLATES` slice).

pub mod android;
pub mod core;
pub mod ios;

/// A single capability the user can enable via `--caps`.
///
/// The variants mirror the chunk-3a/3b/3c CAP markers and the RFC's
/// enumerated values. Chunk 6 wires the CLI flag through `init::run` so
/// every variant is now actively constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    Http,
    Kv,
    Time,
    Platform,
    Sse,
}

impl Capability {
    /// Marker tag as it appears in the templates (e.g. `<<<CAP:http`).
    pub fn marker_tag(self) -> &'static str {
        match self {
            Capability::Http => "http",
            Capability::Kv => "kv",
            Capability::Time => "time",
            Capability::Platform => "platform",
            Capability::Sse => "sse",
        }
    }

    /// Parse the user-facing tag (as accepted on `--caps`) into a
    /// `Capability`. Returns `None` for unknown tags so the caller can
    /// produce a structured error referencing the offending value.
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "http" => Some(Capability::Http),
            "kv" => Some(Capability::Kv),
            "time" => Some(Capability::Time),
            "platform" => Some(Capability::Platform),
            "sse" => Some(Capability::Sse),
            _ => None,
        }
    }
}

/// Placeholder values supplied per `vectis init` invocation.
///
/// Field names match the chunk-3a/3b/3c MANIFEST placeholder tables. Values
/// are derived from the CLI args + resolved version pins; the engine treats
/// each `__FOO__` as a literal string substitution.
#[derive(Debug, Clone)]
pub struct Params {
    pub app_name: String,
    pub app_struct: String,
    pub app_name_lower: String,
    pub android_package: String,
    pub crux_core_version: String,
    pub crux_http_version: String,
    pub crux_kv_version: String,
    pub crux_time_version: String,
    pub crux_platform_version: String,
    pub facet_version: String,
    pub serde_version: String,
    pub uniffi_version: String,
    // Android-only placeholders (chunk 3c MANIFEST § Placeholder reference).
    // These are substituted into `libs.versions.toml` and -- in the case of
    // `__ANDROID_NDK_VERSION__` -- `shared-build.gradle.kts`. They have no
    // effect on core / iOS templates because no core/iOS template references
    // them, but the engine substitutes them unconditionally for simplicity.
    pub agp_version: String,
    pub kotlin_version: String,
    pub compose_bom_version: String,
    pub ktor_version: String,
    pub koin_version: String,
    pub android_ndk_version: String,
}

/// Render a template string with placeholder substitution and
/// capability-marker handling.
///
/// Marker semantics (chunk-3a MANIFEST § Cap-marker reference):
///
/// - `<<<CAP:foo` opens a region and `CAP:foo>>>` closes it; each marker
///   sits on its own line (no inline markers, no nesting).
/// - When `foo` is absent from `caps`, the entire region (markers and
///   content inclusive, plus the trailing newline of each marker line) is
///   removed. This is the chunk-5 default for *every* cap because chunk 5
///   only renders the render-only baseline.
/// - When `foo` is present in `caps`, only the marker lines themselves are
///   removed -- content is preserved verbatim including surrounding
///   indentation. Chunk 6 starts exercising this branch.
///
/// Placeholder substitution runs after marker handling so any
/// capability-version placeholders inside dropped regions are never
/// emitted.
pub fn render(template: &str, params: &Params, caps: &[Capability]) -> String {
    let stripped = process_caps(template, caps);
    substitute_placeholders(&stripped, params)
}

/// Substitute placeholders that may appear in a target *path* (rather than
/// a file's contents).
///
/// Today the path-segment subset used by iOS templates is `__APP_NAME__` /
/// `__APP_NAME_LOWER__`; Android additionally needs `__ANDROID_PACKAGE_PATH__`
/// for the `Android/app/src/main/java/<pkg-path>/...` segment. Chunk 8
/// extends this via [`substitute_path_with`] -- the package path is derived
/// at file-write time (`.` -> `/` translation), not stored on `Params`, so
/// each shell handles its own derivation and passes it in here.
///
/// Order matters across the chain: longer-name placeholders that are strict
/// superstrings of shorter ones must come first. `__APP_NAME_LOWER__` is a
/// superstring of `__APP_NAME__` and `__ANDROID_PACKAGE_PATH__` is a
/// superstring of `__ANDROID_PACKAGE__` (which itself never appears in path
/// positions today, but the rule keeps the chain robust to future
/// additions).
pub fn substitute_path(target: &str, params: &Params) -> String {
    substitute_path_with(target, params, None)
}

/// Path-segment substitution including `__ANDROID_PACKAGE_PATH__` when the
/// caller supplies it. iOS callers pass `None`; Android callers compute the
/// package path (`.` -> `/`) and pass `Some(...)` so the engine can splice
/// it into `Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/...` targets.
pub fn substitute_path_with(
    target: &str,
    params: &Params,
    android_package_path: Option<&str>,
) -> String {
    let mut out = target.to_string();
    if let Some(pkg_path) = android_package_path {
        out = out.replace("__ANDROID_PACKAGE_PATH__", pkg_path);
    }
    out.replace("__APP_NAME_LOWER__", &params.app_name_lower)
        .replace("__APP_NAME__", &params.app_name)
}

/// Apply substitutions for every placeholder field on `Params`.
///
/// Substitution is a literal `replace()` -- the `__FOO__` delimiter
/// (double-underscore + UPPER_SNAKE_CASE) was chosen specifically because
/// it cannot collide with Rust `{}` format strings, Swift `\()`
/// interpolation, or Kotlin `${}` templates that appear in the generated
/// source.
fn substitute_placeholders(input: &str, params: &Params) -> String {
    input
        .replace("__APP_NAME_LOWER__", &params.app_name_lower)
        .replace("__APP_NAME__", &params.app_name)
        .replace("__APP_STRUCT__", &params.app_struct)
        .replace("__ANDROID_PACKAGE__", &params.android_package)
        .replace("__CRUX_CORE_VERSION__", &params.crux_core_version)
        .replace("__CRUX_HTTP_VERSION__", &params.crux_http_version)
        .replace("__CRUX_KV_VERSION__", &params.crux_kv_version)
        .replace("__CRUX_TIME_VERSION__", &params.crux_time_version)
        .replace("__CRUX_PLATFORM_VERSION__", &params.crux_platform_version)
        .replace("__FACET_VERSION__", &params.facet_version)
        .replace("__SERDE_VERSION__", &params.serde_version)
        .replace("__UNIFFI_VERSION__", &params.uniffi_version)
        .replace("__AGP_VERSION__", &params.agp_version)
        .replace("__KOTLIN_VERSION__", &params.kotlin_version)
        .replace("__COMPOSE_BOM_VERSION__", &params.compose_bom_version)
        .replace("__KTOR_VERSION__", &params.ktor_version)
        .replace("__KOIN_VERSION__", &params.koin_version)
        .replace("__ANDROID_NDK_VERSION__", &params.android_ndk_version)
}

/// Walk a template line-by-line resolving CAP markers.
///
/// The implementation deliberately scans the full input each pass rather
/// than holding open/close-line indices, so unmatched markers surface as
/// preserved text in the output (caught by `cargo check` on the rendered
/// project) rather than as a silent partial drop. Performance is fine for
/// templates of this size (the largest core file is ~3KB).
fn process_caps(input: &str, caps: &[Capability]) -> String {
    // Capture the input's line ending so we can emit identical output for
    // CRLF templates. The `lines()` iterator already strips both `\n` and
    // `\r\n`, so we re-attach the original separator.
    let newline = if input.contains("\r\n") { "\r\n" } else { "\n" };
    let trailing_newline = input.ends_with('\n') || input.ends_with("\r\n");

    let mut out = String::with_capacity(input.len());
    let mut active: Option<&str> = None;
    let mut wrote_first = false;

    for line in input.lines() {
        let trimmed = line.trim();

        // Open marker: `<<<CAP:<tag>` with no other content on the line.
        if let Some(tag) = trimmed.strip_prefix("<<<CAP:") {
            // The tag is the entire trailing content -- we already trimmed.
            // Reject inline-content openers (e.g. `<<<CAP:http extra`) by
            // checking for whitespace; templates today never produce them
            // but a malformed template should fail loudly.
            if tag.is_empty() || tag.contains(char::is_whitespace) {
                if wrote_first {
                    out.push_str(newline);
                }
                out.push_str(line);
                wrote_first = true;
                continue;
            }
            if active.is_some() {
                // Nested opener -- treat as data so the malformed template
                // is visible in the rendered output. Real templates do
                // not nest (chunk-3a MANIFEST § Notes).
                if wrote_first {
                    out.push_str(newline);
                }
                out.push_str(line);
                wrote_first = true;
                continue;
            }
            active = Some(static_tag(tag));
            continue;
        }

        // Close marker: `CAP:<tag>>>>` (note the three trailing `>`).
        // A mismatched close (no active opener, or tag mismatch) falls
        // through to the regular emit-or-skip path below so the
        // malformed marker shows up verbatim in the rendered file -- the
        // downstream compiler then fails loudly instead of us silently
        // closing the wrong region.
        if let Some(rest) = trimmed.strip_prefix("CAP:")
            && let Some(tag) = rest.strip_suffix(">>>")
            && let Some(open_tag) = active
            && tag == open_tag
        {
            active = None;
            continue;
        }

        if let Some(open_tag) = active {
            // Inside a marker: include or drop based on caps membership.
            if cap_selected(open_tag, caps) {
                if wrote_first {
                    out.push_str(newline);
                }
                out.push_str(line);
                wrote_first = true;
            }
        } else {
            // Outside any marker: always emit.
            if wrote_first {
                out.push_str(newline);
            }
            out.push_str(line);
            wrote_first = true;
        }
    }

    if trailing_newline {
        out.push_str(newline);
    }

    out
}

/// Look up a tag string against the canonical `Capability` enum and
/// return whether the user enabled it.
fn cap_selected(tag: &str, caps: &[Capability]) -> bool {
    caps.iter().any(|c| c.marker_tag() == tag)
}

/// Map a runtime tag string back to a `'static str` for storage in
/// `active`. Unknown tags fall through to `"__unknown__"` so an
/// unmatched closer can never accidentally close the wrong region.
fn static_tag(tag: &str) -> &'static str {
    match tag {
        "http" => "http",
        "kv" => "kv",
        "time" => "time",
        "platform" => "platform",
        "sse" => "sse",
        _ => "__unknown__",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_params() -> Params {
        Params {
            app_name: "Counter".into(),
            app_struct: "Counter".into(),
            app_name_lower: "counter".into(),
            android_package: "com.vectis.counter".into(),
            crux_core_version: "0.17.0".into(),
            crux_http_version: "0.16.0".into(),
            crux_kv_version: "0.11.0".into(),
            crux_time_version: "0.15.0".into(),
            crux_platform_version: "0.8.0".into(),
            facet_version: "=0.31".into(),
            serde_version: "1.0".into(),
            uniffi_version: "=0.29.4".into(),
            agp_version: "8.13.2".into(),
            kotlin_version: "2.3.0".into(),
            compose_bom_version: "2026.01.01".into(),
            ktor_version: "3.4.0".into(),
            koin_version: "4.1.1".into(),
            android_ndk_version: "27.0.12077973".into(),
        }
    }

    #[test]
    fn substitutes_known_placeholders_only() {
        let input = "name=__APP_NAME__ struct=__APP_STRUCT__ unknown=__NOT_A_PLACEHOLDER__";
        let out = substitute_placeholders(input, &sample_params());
        assert_eq!(
            out,
            "name=Counter struct=Counter unknown=__NOT_A_PLACEHOLDER__"
        );
    }

    #[test]
    fn substitutes_app_name_lower_before_app_name() {
        // Order matters: __APP_NAME_LOWER__ is a superstring of __APP_NAME__
        // (with `_LOWER__` suffix). If we substituted __APP_NAME__ first,
        // we'd corrupt __APP_NAME_LOWER__ into `Counter_LOWER__`.
        let input = "id=__APP_NAME_LOWER__ name=__APP_NAME__";
        let out = substitute_placeholders(input, &sample_params());
        assert_eq!(out, "id=counter name=Counter");
    }

    #[test]
    fn render_only_strips_every_cap_block() {
        let input = "before\n<<<CAP:http\nhttp_only\nCAP:http>>>\n<<<CAP:kv\nkv_only\nCAP:kv>>>\nafter\n";
        let out = render(input, &sample_params(), &[]);
        assert_eq!(out, "before\nafter\n");
    }

    #[test]
    fn cap_selected_keeps_inner_content_only() {
        let input = "before\n<<<CAP:http\nhttp_only\nCAP:http>>>\nafter\n";
        let out = render(input, &sample_params(), &[Capability::Http]);
        assert_eq!(out, "before\nhttp_only\nafter\n");
    }

    #[test]
    fn cap_marker_lines_are_dropped_when_selected() {
        // Prove the marker lines themselves never make it into output even
        // when the cap is on -- otherwise downstream parsers (rustc, swiftc,
        // gradle) would see literal `<<<CAP:http` text.
        let input = "<<<CAP:http\nx\nCAP:http>>>\n";
        let out = render(input, &sample_params(), &[Capability::Http]);
        assert_eq!(out, "x\n");
    }

    #[test]
    fn preserves_indentation_inside_markers() {
        let input = "[features]\ncodegen = [\n    <<<CAP:http\n    \"crux_http/facet_typegen\",\n    CAP:http>>>\n]\n";
        let out = render(input, &sample_params(), &[Capability::Http]);
        assert_eq!(
            out,
            "[features]\ncodegen = [\n    \"crux_http/facet_typegen\",\n]\n"
        );
    }

    #[test]
    fn render_substitutes_capability_versions_only_when_kept() {
        // Render-only: __CRUX_HTTP_VERSION__ is inside a stripped block, so
        // the substitution never matters. Run it anyway to prove the engine
        // doesn't accidentally leak the placeholder when it would otherwise
        // be dropped.
        let input = "always=__CRUX_CORE_VERSION__\n<<<CAP:http\ncrux_http=\"__CRUX_HTTP_VERSION__\"\nCAP:http>>>\n";
        let render_only = render(input, &sample_params(), &[]);
        assert_eq!(render_only, "always=0.17.0\n");

        let with_http = render(input, &sample_params(), &[Capability::Http]);
        assert_eq!(with_http, "always=0.17.0\ncrux_http=\"0.16.0\"\n");
    }

    #[test]
    fn preserves_files_with_no_trailing_newline() {
        let input = "no-trailer";
        let out = render(input, &sample_params(), &[]);
        assert_eq!(out, "no-trailer");
    }

    #[test]
    fn render_is_idempotent_for_files_without_markers_or_placeholders() {
        let input = "static content\nline 2\n";
        assert_eq!(render(input, &sample_params(), &[]), input);
    }

    #[test]
    fn substitute_path_handles_app_name_in_dir_and_filename_positions() {
        let target = "iOS/__APP_NAME__/__APP_NAME__App.swift";
        let out = substitute_path(target, &sample_params());
        assert_eq!(out, "iOS/Counter/CounterApp.swift");
    }

    #[test]
    fn substitute_path_handles_app_name_lower_before_app_name() {
        // `__APP_NAME_LOWER__` is a superstring of `__APP_NAME__`.
        let target = "iOS/__APP_NAME__/__APP_NAME_LOWER__/x.swift";
        let out = substitute_path(target, &sample_params());
        assert_eq!(out, "iOS/Counter/counter/x.swift");
    }

    #[test]
    fn substitute_path_leaves_static_paths_alone() {
        let target = "iOS/project.yml";
        let out = substitute_path(target, &sample_params());
        assert_eq!(out, "iOS/project.yml");
    }

    #[test]
    fn substitute_path_with_android_package_path_substitutes_pkg_path() {
        let target =
            "Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/__APP_NAME__Application.kt";
        let out = substitute_path_with(target, &sample_params(), Some("com/vectis/counter"));
        assert_eq!(
            out,
            "Android/app/src/main/java/com/vectis/counter/CounterApplication.kt"
        );
    }

    #[test]
    fn substitute_path_with_no_android_package_path_leaves_placeholder_alone() {
        let target =
            "Android/app/src/main/java/__ANDROID_PACKAGE_PATH__/__APP_NAME__Application.kt";
        let out = substitute_path(target, &sample_params());
        // iOS callers pass `None`; the engine must not substitute the
        // package-path placeholder. Leaving it visible here means an
        // accidental Android-target row in the iOS registry would surface
        // as a malformed write path rather than a silent corruption.
        assert!(out.contains("__ANDROID_PACKAGE_PATH__"));
    }

    #[test]
    fn substitute_placeholders_substitutes_android_version_placeholders() {
        let input = "agp=__AGP_VERSION__ kotlin=__KOTLIN_VERSION__ \
                     compose=__COMPOSE_BOM_VERSION__ ktor=__KTOR_VERSION__ \
                     koin=__KOIN_VERSION__ ndk=__ANDROID_NDK_VERSION__";
        let out = substitute_placeholders(input, &sample_params());
        assert_eq!(
            out,
            "agp=8.13.2 kotlin=2.3.0 compose=2026.01.01 ktor=3.4.0 \
             koin=4.1.1 ndk=27.0.12077973"
        );
    }
}

//! `vectis init` -- scaffold a new Crux project.
//!
//! Chunk 5 landed the render-only core scaffold. Chunk 6 wires the
//! `--caps` flag through the engine so any combination of `http`, `kv`,
//! `time`, `platform`, `sse` is honoured. Chunks 7 / 8 will replace the
//! `--shells` guard with iOS / Android scaffolding; today, requesting a
//! shell platform passes the prereq check (so the user gets an accurate
//! "your toolchain is incomplete" report) but is rejected with a
//! structured `InvalidProject` error before any files are written.

pub mod core;

use std::path::PathBuf;

use crate::{
    CommandOutcome, InitArgs,
    error::VectisError,
    prerequisites::{self, AssemblyKind},
    templates::Capability,
    versions::Versions,
};

pub fn run(args: &InitArgs) -> Result<CommandOutcome, VectisError> {
    let mut assemblies = vec![AssemblyKind::Core];
    let shells = parse_shells(args.shells.as_deref())?;
    for shell in &shells {
        assemblies.push(*shell);
    }

    prerequisites::check(&assemblies)?;

    // Resolve version pins up-front so a bad `--version-file` is reported
    // before we touch the filesystem (chunk 4 wired this in for the smoke
    // test; chunk 5 starts actually consuming the resolved struct).
    let project_dir = resolve_project_dir(args.dir.as_deref())?;
    let versions = Versions::resolve(&project_dir, args.version_file.as_deref())?;

    let caps = parse_caps(args.caps.as_deref())?;

    let android_package = args
        .android_package
        .clone()
        .unwrap_or_else(|| core::default_android_package(&args.app_name));

    let core_result = core::scaffold(
        &project_dir,
        &args.app_name,
        &android_package,
        &versions,
        &caps,
    )?;

    // Chunk 5 only scaffolds core. iOS / Android shells listed via
    // `--shells` are accepted by the prereq check (so the user gets an
    // accurate "your toolchain is incomplete" report against the platforms
    // they asked for) but the actual scaffold is not yet implemented. We
    // surface this as a structured error rather than silently dropping the
    // request.
    if !shells.is_empty() {
        return Err(VectisError::InvalidProject {
            message: format!(
                "--shells {} requested but iOS/Android scaffolding lands in chunks 7/8; rerun without --shells to scaffold core-only",
                shells.iter().map(|a| a.tag()).collect::<Vec<_>>().join(",")
            ),
        });
    }

    let value = serde_json::json!({
        "app_name": args.app_name,
        "app_struct": args.app_name,
        "project_dir": project_dir.display().to_string(),
        "assemblies": {
            "core": {
                "status": "created",
                "files": core_result.files,
            }
        },
        "capabilities": caps.iter().map(|c| c.marker_tag()).collect::<Vec<_>>(),
        "shells": serde_json::Value::Array(vec![]),
    });

    Ok(CommandOutcome::Success(value))
}

/// Parse the `--caps` flag into the canonical `Capability` set.
///
/// Accepts a comma-separated list. Empty entries (including `--caps ""`)
/// are tolerated so build orchestration that always passes the flag does
/// not break. Unknown tags produce an `InvalidProject` error pointing at
/// the offending value and the canonical accepted set.
///
/// Duplicate entries are deduplicated in input order so the rendered
/// output is stable regardless of how the user spells the list.
fn parse_caps(raw: Option<&str>) -> Result<Vec<Capability>, VectisError> {
    let mut out: Vec<Capability> = Vec::new();
    let Some(raw) = raw else { return Ok(out) };
    for tag in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let cap = Capability::from_tag(tag).ok_or_else(|| VectisError::InvalidProject {
            message: format!(
                "unknown capability: {tag:?} (expected one of: http, kv, time, platform, sse)"
            ),
        })?;
        if !out.contains(&cap) {
            out.push(cap);
        }
    }
    Ok(out)
}

fn parse_shells(raw: Option<&str>) -> Result<Vec<AssemblyKind>, VectisError> {
    let mut out = Vec::new();
    let Some(raw) = raw else { return Ok(out) };
    for shell in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        match shell {
            "ios" => out.push(AssemblyKind::Ios),
            "android" => out.push(AssemblyKind::Android),
            other => {
                return Err(VectisError::InvalidProject {
                    message: format!(
                        "unknown shell platform: {other:?} (expected one of: ios, android)"
                    ),
                });
            }
        }
    }
    Ok(out)
}

fn resolve_project_dir(dir: Option<&std::path::Path>) -> Result<PathBuf, VectisError> {
    match dir {
        Some(p) => Ok(p.to_path_buf()),
        None => std::env::current_dir().map_err(VectisError::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_caps_none_yields_empty_render_only_set() {
        assert!(parse_caps(None).unwrap().is_empty());
    }

    #[test]
    fn parse_caps_empty_string_is_render_only() {
        // `--caps ""` and `--caps " , "` must both behave like "no caps"
        // so callers (CI, scripts) can pass the flag unconditionally.
        assert!(parse_caps(Some("")).unwrap().is_empty());
        assert!(parse_caps(Some(" , ,")).unwrap().is_empty());
    }

    #[test]
    fn parse_caps_accepts_full_matrix_in_order() {
        let caps = parse_caps(Some("http,kv,time,platform,sse")).unwrap();
        assert_eq!(
            caps,
            vec![
                Capability::Http,
                Capability::Kv,
                Capability::Time,
                Capability::Platform,
                Capability::Sse,
            ]
        );
    }

    #[test]
    fn parse_caps_trims_whitespace_around_each_token() {
        let caps = parse_caps(Some("  http , kv ")).unwrap();
        assert_eq!(caps, vec![Capability::Http, Capability::Kv]);
    }

    #[test]
    fn parse_caps_dedupes_in_input_order() {
        let caps = parse_caps(Some("kv,http,kv,http,time")).unwrap();
        assert_eq!(
            caps,
            vec![Capability::Kv, Capability::Http, Capability::Time]
        );
    }

    #[test]
    fn parse_caps_rejects_unknown_token() {
        let err = parse_caps(Some("http,bogus")).expect_err("unknown cap must error");
        match err {
            VectisError::InvalidProject { message } => {
                assert!(message.contains("\"bogus\""), "{message}");
                assert!(message.contains("http"), "{message}");
                assert!(message.contains("sse"), "{message}");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
}

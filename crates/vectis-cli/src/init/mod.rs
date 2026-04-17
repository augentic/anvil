//! `vectis init` -- scaffold a new Crux project.
//!
//! Chunk 5 lands the render-only core scaffold. Chunks 6 / 7 / 8 extend the
//! flow with `--caps`, iOS shells, and Android shells respectively; chunk 5
//! deliberately accepts only an empty `--caps` and an empty `--shells` (any
//! shell-platform list still passes prereqs but the scaffold itself is
//! core-only -- the iOS / Android paths land in their own chunks).

pub mod core;

use std::path::PathBuf;

use crate::{
    CommandOutcome, InitArgs,
    error::VectisError,
    prerequisites::{self, AssemblyKind},
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

    // Chunk 5 is render-only: caps are always empty and any non-empty
    // --caps flag is rejected as unimplemented to avoid producing a
    // half-baked project. Chunk 6 swaps this for a real comma-split parser
    // shared with --shells (the helper lives in `init::core` once both
    // call sites need it).
    if let Some(caps) = &args.caps {
        let any = caps
            .split(',')
            .map(str::trim)
            .any(|s| !s.is_empty());
        if any {
            return Err(VectisError::InvalidProject {
                message: "--caps is not yet implemented (chunk 6); rerun without --caps to scaffold the render-only baseline".into(),
            });
        }
    }

    let android_package = args
        .android_package
        .clone()
        .unwrap_or_else(|| core::default_android_package(&args.app_name));

    let core_result = core::scaffold(
        &project_dir,
        &args.app_name,
        &android_package,
        &versions,
        &[],
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
        "capabilities": serde_json::Value::Array(vec![]),
        "shells": serde_json::Value::Array(vec![]),
    });

    Ok(CommandOutcome::Success(value))
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

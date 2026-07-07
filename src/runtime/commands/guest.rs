//! Blind argv forwarding: the composed-deployment leg of the binary.
//!
//! Everything the first-token triage does not recognise as a native
//! provisioning verb lands here *unparsed* — workflow verbs,
//! `--help`, `--version`, bare invocations — and runs in the workflow
//! guest against the composed deployment (workflow guest + adapter
//! guests + the spawning `cursor-agent` model backend) through
//! `specify_runtime::drive`, which spawns the generic host layer
//! (`specify-host run --config <manifest> -- <argv>`, RFC-65 move 2)
//! with inherited stdio and the exit code passed through to the
//! process exit. The guest's clap tree owns parsing, help, version,
//! and usage errors (exit 2 travels back verbatim). This module owns
//! only the manifest choice the guest leg runs against — an
//! `omnia.toml` at the project root wins wholesale (the developer
//! posture); absent one, the generated deployment manifest is
//! regenerated in the per-project cache (`commands::deploy` — the one
//! manifest-producing code path) and driven directly — plus the two
//! host-side guards that need a peek at the global flags: the
//! `--plan-dir` refusal and the failure-envelope format. See
//! DECISIONS.md §"One `specify` binary".

use std::fs;
use std::path::{Path, PathBuf};

use specify_error::Error;

use crate::runtime::cli::Format;
use crate::runtime::commands::deploy;
use crate::runtime::output::{Exit, report};

/// Operator-provided deployment manifest at the project root; when
/// present it replaces the generated manifest wholesale.
const MANIFEST_FILENAME: &str = "omnia.toml";

/// Wire code for a failure ahead of or around the host spawn: the
/// `specify-host` binary missing beside the executable, the spawn
/// itself failing, or the host dying to a signal. A failure *inside*
/// the spawned host — deployment assembly, backend connect
/// (`cursor-agent` missing from `PATH`) — surfaces on the host's own
/// stderr and its exit code passes through, exactly like a failure
/// inside the guest. Adapter-resolution failures during manifest
/// regeneration keep their own typed codes (`adapter-not-found`,
/// `adapter-not-installed`, `adapter-digest-mismatch`, …) so the
/// operator sees the same diagnostics the native resolvers raise.
const GUEST_RUNTIME_FAILED: &str = "guest-runtime-failed";

/// Forward the process argv unparsed to the workflow guest and map
/// the outcome onto the process exit: guest exit codes pass through
/// verbatim; host-side failures render an error envelope in the
/// sniffed `--format`.
pub fn forward() -> Exit {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let format = sniff_format(&args);
    match dispatch(&args) {
        Ok(0) => Exit::Success,
        Ok(code) => Exit::Code(code),
        Err(err) => report(format, &err),
    }
}

/// Resolve the deployment manifest (project-root `omnia.toml` or the
/// generated manifest, regenerated fresh for this drive), then block
/// on the composed run, forwarding argv verbatim (the runtime core
/// supplies the guest program name) so the guest's clap sees exactly
/// what the operator typed. Regeneration is cheap by construction —
/// filesystem probes, digest-cached describe answers, and one
/// content-digest read per pinned entry (the D4 + committed-lock
/// verification, RFC-65 AC8), never a fetch — so the manifest is
/// always fresh and lock-verified at drive time without a staleness
/// heuristic.
fn dispatch(args: &[String]) -> Result<u8, Error> {
    let project_dir = std::env::current_dir()
        .map_err(|err| failed(format!("resolving the working directory: {err}")))?;
    check_plan_dir(sniff_plan_dir(args).as_deref(), &project_dir)?;

    let committed = project_dir.join(MANIFEST_FILENAME);
    if committed.is_file() {
        return drive(&committed, args.to_vec());
    }

    let generated = match deploy::regenerate(&project_dir) {
        Ok(path) => path,
        // Operator help stays whole on a degraded deployment: grammar
        // rendering needs only the workflow guest, so an evicted store
        // entry or lock drift must not block `--help` / `--version` /
        // `completions`. Every other verb keeps the typed discovery
        // failure (`adapter-not-installed`, `adapter-digest-mismatch`).
        // The discovery failure is the primary error when the
        // fallback also fails — the core-only miss is a symptom of
        // the same degraded state.
        Err(err) if grammar_only(args) => {
            deploy::regenerate_core_only(&project_dir).map_err(|_fallback| err)?
        }
        Err(err) => return Err(err),
    };
    drive(&generated, args.to_vec())
}

/// Whether the argv can be answered by the clap grammar alone —
/// `--help` / `-h` / `--version` / `-V` anywhere, or the
/// `completions` verb — so a core-only deployment suffices.
fn grammar_only(args: &[String]) -> bool {
    args.iter().any(|token| matches!(token.as_str(), "--help" | "-h" | "--version" | "-V"))
        || args.first().is_some_and(|token| token == "completions")
}

/// The failure-envelope format for host-side errors, sniffed from the
/// global `--format` flag (space- or `=`-separated) with the
/// `SPECIFY_FORMAT` env fallback — the same resolution clap performs
/// in-guest, applied here without parsing the workflow argv. Only
/// host-side failures read it; the guest re-resolves the flag itself.
fn sniff_format(args: &[String]) -> Format {
    sniff_flag(args, "--format").or_else(|| std::env::var("SPECIFY_FORMAT").ok()).map_or(
        Format::Text,
        |value| {
            if value == "json" { Format::Json } else { Format::Text }
        },
    )
}

/// The `--plan-dir` global (space- or `=`-separated) with the
/// `SPECIFY_PLAN_DIR` env fallback, sniffed for the native refusal
/// guard below without parsing the workflow argv.
fn sniff_plan_dir(args: &[String]) -> Option<PathBuf> {
    sniff_flag(args, "--plan-dir")
        .or_else(|| std::env::var("SPECIFY_PLAN_DIR").ok())
        .map(PathBuf::from)
}

/// The last value of `<flag> <value>` / `<flag>=<value>` in `args`
/// (clap's last-one-wins semantics for a repeated global flag).
fn sniff_flag(args: &[String], flag: &str) -> Option<String> {
    let mut found = None;
    let mut args = args.iter();
    while let Some(token) = args.next() {
        if token == flag {
            found = args.next().cloned();
        } else if let Some(value) = token.strip_prefix(flag).and_then(|rest| rest.strip_prefix('='))
        {
            found = Some(value.to_string());
        }
    }
    found
}

/// Refuse a `--plan-dir` (or `SPECIFY_PLAN_DIR`) pointing anywhere but
/// the working directory: the guest anchors plan artifacts at the `"."`
/// preopen, so any other plan root would be silently ignored in-guest.
/// A value that resolves to the working directory itself is a no-op and
/// passes.
fn check_plan_dir(plan_dir: Option<&Path>, project_dir: &Path) -> Result<(), Error> {
    let Some(dir) = plan_dir else {
        return Ok(());
    };
    let same = fs::canonicalize(dir)
        .and_then(|requested| fs::canonicalize(project_dir).map(|cwd| requested == cwd))
        .unwrap_or(false);
    if same {
        return Ok(());
    }
    Err(Error::Argument {
        flag: "--plan-dir",
        detail: format!(
            "`--plan-dir` is native-only: forwarded verbs anchor plan artifacts at the working \
             directory, so {} would be ignored; run from the plan root instead",
            dir.display()
        ),
    })
}

fn drive(manifest: &Path, args: Vec<String>) -> Result<u8, Error> {
    specify_runtime::drive(manifest, args).map_err(|err| failed(format!("{err:#}")))
}

const fn failed(detail: String) -> Error {
    Error::Diag {
        code: GUEST_RUNTIME_FAILED,
        detail,
    }
}

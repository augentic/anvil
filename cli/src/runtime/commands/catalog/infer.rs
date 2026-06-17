//! `specify catalog infer` handler — the host orchestration around the
//! deterministic `vectis infer` tool.
//!
//! Two phases, mirroring the `specify slice build --phase prepare|finalize`
//! idiom:
//!
//! - `report` (read-only) dispatches `vectis infer` against the
//!   composition baseline and prints its **name-free** cluster report.
//!   It writes nothing.
//! - `bind` consumes a skill-authored `{ fingerprint → slug }` bindings
//!   file, reconciles it against the existing catalog under the
//!   no-overwrite + one-skeleton-per-slug guards, and writes
//!   `components.yaml` (or prints the diff under `--dry-run`).
//!
//! The host **invents no names**: `bind` is deterministic bookkeeping
//! over names the build skill or operator parts
//! supply. The collision-suffix logic operates purely on the
//! fingerprint strings already present in the bindings, so no skeleton
//! logic crosses back into the host — the single normalizer stays
//! tool-side.
//!
//! **Operator parts.** When `.specify/design-system/parts.yaml`
//! exists, both phases forward it to the tool with `--parts`. A part's
//! `group` fragment is fingerprinted (tool-side, at read time) and
//! registered as a pinned binding carrying two authorities: **naming**
//! (a matched-pin cluster echoes the operator slug in `bound-slug`, so
//! `report`'s catalog echo and the build skill both leave it untouched)
//! and **promotion** (a matched pin clusters below `--min-occurrences`).
//! `bind` projects each **matched** pin into `components.yaml` as a
//! `status: confirmed` entry, re-derived from `parts.yaml` every run
//! (so re-runs are no-ops), and surfaces the non-blocking `part-unmatched`
//! report for pins that matched nothing — informational only, never an
//! abort or a merge precondition.
//!
//! **Run-to-run binding stability.** `bind` persists each
//! `fingerprint → slug` binding on the catalog entry (the `fingerprint`
//! field), and `report` reverse-maps the catalog by fingerprint to fill
//! each already-named cluster's `bound-slug`. So once the skill names a
//! fingerprint, every later `report` echoes that slug and the skill
//! leaves the cluster untouched — naming never thrashes the catalog.
//! `report` only fills a `bound-slug` the tool left `null`; it never
//! clobbers a tool-emitted binding (e.g. an operator-pin echo).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::Path;

use serde::Deserialize;
use serde_json::{Value, json};
use specify_error::{Error, Result};
use specify_workflow::design_system::{ComponentsCatalog, Parts};

use super::cli::InferPhase;
use crate::runtime::commands::extension;
use crate::runtime::context::Ctx;

/// Length of the fingerprint prefix appended when two distinct
/// fingerprints are bound to the same bare slug ("first-writer-wins
/// … suffixed `slug-<fp-prefix>`"). Eight hex characters keep the
/// suffix readable while collisions stay astronomically unlikely.
const FP_PREFIX_LEN: usize = 8;

/// Extension name the composition inference subcommand lives under.
const VECTIS_TOOL: &str = "vectis";

/// Composition baseline path relative to the project root.
const COMPOSITION_REL: &str = ".specify/specs/composition.yaml";

/// Screenshot stage-6 candidate-cache directory relative to the project
/// root. When present, `report` feeds it to the tool so
/// cached skeletons cluster alongside baseline groups.
const CANDIDATE_CACHE_REL: &str = ".specify/.cache/component-candidates";

pub fn run(
    ctx: &Ctx, phase: InferPhase, min_occurrences: Option<u32>, bindings: Option<&Path>,
    dry_run: bool,
) -> Result<()> {
    match phase {
        InferPhase::Report => report(ctx, min_occurrences),
        InferPhase::Bind => bind(ctx, bindings, dry_run),
    }
}

/// `--phase report`: dispatch `vectis infer` and print the name-free
/// cluster report. An absent baseline emits an empty report and runs no
/// tool ("absent catalog = no factoring") — but still lists every
/// operator part as `part-unmatched`, since nothing can match without a
/// baseline yet.
fn report(ctx: &Ctx, min_occurrences: Option<u32>) -> Result<()> {
    match dispatch_infer(ctx, min_occurrences)? {
        None => emit_report(ctx, &empty_report(&all_part_slugs(ctx)?)),
        Some(mut report) => {
            populate_bound_slugs(ctx, &mut report)?;
            emit_report(ctx, &report)
        }
    }
}

/// Dispatch the deterministic `vectis infer` tool against the
/// composition baseline, folding in the candidate cache and operator
/// `parts.yaml` when each is present. Returns `Ok(None)` when no
/// baseline exists (the tool requires one, and an absent baseline means
/// nothing to cluster). `parts.yaml` is schema-validated before being
/// forwarded (schema-validated on read).
fn dispatch_infer(ctx: &Ctx, min_occurrences: Option<u32>) -> Result<Option<Value>> {
    let composition = ctx.project_dir.join(COMPOSITION_REL);
    if !composition.is_file() {
        return Ok(None);
    }

    let mut args =
        vec!["infer".to_string(), "--composition".to_string(), composition.display().to_string()];
    let candidate_cache = ctx.project_dir.join(CANDIDATE_CACHE_REL);
    if candidate_cache.is_dir() {
        args.push("--candidate-cache".to_string());
        args.push(candidate_cache.display().to_string());
    }
    let parts_path = Parts::path_in(&ctx.project_dir);
    if parts_path.is_file() {
        // Validate on read; a malformed parts file fails here rather
        // than being silently dropped by the best-effort tool reader.
        Parts::load(&ctx.project_dir)?;
        args.push("--parts".to_string());
        args.push(parts_path.display().to_string());
    }
    if let Some(n) = min_occurrences {
        args.push("--min-occurrences".to_string());
        args.push(n.to_string());
    }

    let captured = extension::run_captured(ctx, VECTIS_TOOL, args)?;
    if captured.exit_code != 0 {
        let stderr = String::from_utf8_lossy(&captured.stderr);
        let stdout = String::from_utf8_lossy(&captured.stdout);
        return Err(Error::Diag {
            code: "catalog-infer-tool-failed",
            detail: format!(
                "vectis infer exited with code {}: {}",
                captured.exit_code,
                if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() }
            ),
        });
    }

    let report: Value = serde_json::from_slice(&captured.stdout).map_err(|err| Error::Diag {
        code: "catalog-infer-report-malformed",
        detail: format!("vectis infer report is not valid JSON: {err}"),
    })?;
    Ok(Some(report))
}

/// Every operator part slug, in sorted order — the `part-unmatched` set
/// for the no-baseline case (nothing can match yet). Returns an empty
/// vector when no `parts.yaml` exists. Loading validates the file.
fn all_part_slugs(ctx: &Ctx) -> Result<Vec<String>> {
    Ok(Parts::load(&ctx.project_dir)?
        .map(|parts| parts.parts.into_keys().collect())
        .unwrap_or_default())
}

/// Fill each cluster's `bound-slug` from the existing catalog's
/// `fingerprint → slug` index (run-to-run stability). Only a
/// `null`/absent `bound-slug` is filled — a slug the tool already bound
/// (e.g. an operator-pin echo) is never overwritten.
fn populate_bound_slugs(ctx: &Ctx, report: &mut Value) -> Result<()> {
    let Some(catalog) = ComponentsCatalog::load(&ctx.project_dir)? else {
        return Ok(());
    };
    let index = catalog.fingerprint_index();
    let Some(clusters) = report.get_mut("clusters").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    for cluster in clusters {
        if cluster.get("bound-slug").is_some_and(|v| !v.is_null()) {
            continue;
        }
        let Some(fingerprint) = cluster.get("fingerprint").and_then(Value::as_str) else {
            continue;
        };
        if let Some(slug) = index.get(fingerprint) {
            cluster["bound-slug"] = Value::String((*slug).to_string());
        }
    }
    Ok(())
}

/// `--phase bind`: reconcile skill-authored bindings **and** matched
/// operator-part projections into the catalog under the bind guards,
/// then write it (or print the diff under `--dry-run`).
///
/// Operator parts win naming: a matched pin is a first-writer for its
/// fingerprint (so a skill binding handed the same name under a
/// *different* fingerprint is suffixed by the slug-uniqueness guard), and
/// a skill binding for a *pinned* fingerprint is dropped (the operator
/// already named it). Matched pins are re-derived from `parts.yaml`
/// every run, so re-binding is a no-op; unmatched pins surface in
/// the diff as `part-unmatched`.
fn bind(ctx: &Ctx, bindings: Option<&Path>, dry_run: bool) -> Result<()> {
    let parts = part_projections(ctx)?;
    let skill = bindings.map(load_bindings).transpose()?;

    if skill.is_none() && !Parts::path_in(&ctx.project_dir).is_file() {
        return Err(Error::Argument {
            flag: "--bindings",
            detail: "`specify catalog infer --phase bind` requires --bindings <path> or a \
                     parts.yaml input"
                .to_string(),
        });
    }

    let desired = combine_desired(parts.projections, skill);

    let mut catalog =
        ComponentsCatalog::load(&ctx.project_dir)?.unwrap_or_else(ComponentsCatalog::empty);
    let before: Vec<String> = catalog.components.keys().cloned().collect();

    for binding in resolve_slugs(desired) {
        catalog.upsert_bound(&binding.slug, &binding.fingerprint, binding.description);
    }

    let added: Vec<String> =
        catalog.components.keys().filter(|slug| !before.contains(slug)).cloned().collect();

    if dry_run {
        return emit_bind_diff(ctx, &added, &parts.unmatched, true);
    }

    // Create the file only when there is something to record
    // ("absent catalog = no factoring"); an empty bindings file against
    // an absent catalog writes nothing.
    if !catalog.components.is_empty() {
        catalog.save(&ctx.project_dir)?;
    }
    emit_bind_diff(ctx, &added, &parts.unmatched, false)
}

/// The matched + unmatched outcome of folding `parts.yaml` through the
/// tool.
#[derive(Default)]
struct PartsOutcome {
    /// Matched pins to project as `confirmed` catalog entries.
    projections: Vec<DesiredBinding>,
    /// Sorted slugs of pins that matched no baseline/cache group.
    unmatched: Vec<String>,
}

/// Resolve operator parts against the current baseline by dispatching
/// the tool with `--parts` and reading back the matched-pin clusters
/// (those carrying `pinned: true` + a `bound-slug`) and the
/// `unmatched-parts` list. Returns an empty outcome when no `parts.yaml`
/// exists; when a parts file exists but no baseline does yet, every part
/// is unmatched.
fn part_projections(ctx: &Ctx) -> Result<PartsOutcome> {
    let Some(parts) = Parts::load(&ctx.project_dir)? else {
        return Ok(PartsOutcome::default());
    };
    let Some(report) = dispatch_infer(ctx, None)? else {
        return Ok(PartsOutcome {
            projections: Vec::new(),
            unmatched: parts.parts.into_keys().collect(),
        });
    };

    let mut projections: Vec<DesiredBinding> = Vec::new();
    if let Some(clusters) = report.get("clusters").and_then(Value::as_array) {
        for cluster in clusters {
            if cluster.get("pinned").and_then(Value::as_bool) != Some(true) {
                continue;
            }
            let (Some(fingerprint), Some(slug)) = (
                cluster.get("fingerprint").and_then(Value::as_str),
                cluster.get("bound-slug").and_then(Value::as_str),
            ) else {
                continue;
            };
            projections.push(DesiredBinding {
                slug: slug.to_string(),
                fingerprint: fingerprint.to_string(),
                description: parts.description_of(slug).map(str::to_string),
                pinned: true,
            });
        }
    }

    let unmatched: Vec<String> = report
        .get("unmatched-parts")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default();

    Ok(PartsOutcome {
        projections,
        unmatched,
    })
}

/// Merge matched part projections (first-writers) with the skill
/// bindings into one desired-binding list. A skill binding for a
/// fingerprint already pinned by an operator part is dropped — the
/// operator owns that fingerprint's name.
fn combine_desired(
    projections: Vec<DesiredBinding>, skill: Option<BindingsFile>,
) -> Vec<DesiredBinding> {
    let pinned_fps: BTreeSet<String> = projections.iter().map(|p| p.fingerprint.clone()).collect();
    let mut desired = projections;
    if let Some(skill) = skill {
        for (fingerprint, value) in skill.bindings {
            if pinned_fps.contains(&fingerprint) {
                continue;
            }
            desired.push(DesiredBinding {
                slug: value.slug().to_string(),
                fingerprint,
                description: value.description(),
                pinned: false,
            });
        }
    }
    desired
}

/// A single skill-authored binding value: either a bare slug string or
/// an object carrying an optional description.
#[derive(Deserialize)]
#[serde(untagged)]
enum BindingValue {
    Slug(String),
    Detailed { slug: String, description: Option<String> },
}

impl BindingValue {
    fn slug(&self) -> &str {
        match self {
            Self::Slug(slug) | Self::Detailed { slug, .. } => slug,
        }
    }

    fn description(&self) -> Option<String> {
        match self {
            Self::Slug(_) => None,
            Self::Detailed { description, .. } => description.clone(),
        }
    }
}

/// The `{ fingerprint → slug }` map the build skill or a
/// future projection authors. An optional top-level `version` field is
/// tolerated (and ignored) — serde drops unknown keys by default.
#[derive(Deserialize)]
struct BindingsFile {
    bindings: BTreeMap<String, BindingValue>,
}

fn load_bindings(path: &Path) -> Result<BindingsFile> {
    let content = std::fs::read_to_string(path).map_err(|source| Error::Filesystem {
        op: "read",
        path: path.to_path_buf(),
        source,
    })?;
    let file: BindingsFile = serde_saphyr::from_str(&content).map_err(|err| {
        Error::validation_failed(
            "catalog-bindings-malformed",
            "bindings file is a `{ version?, bindings: { <fingerprint>: <slug> } }` map",
            format!("{}: parse failed: {err}", path.display()),
        )
    })?;
    validate_bindings(&file, path)?;
    Ok(file)
}

/// Reject a bindings file whose keys are not 64-char lowercase-hex
/// fingerprints or whose slugs are not kebab-case, *before* `bind`
/// writes anything. `ComponentsCatalog::load` schema-validates the
/// catalog on read (kebab slug keys, `^[0-9a-f]{64}$` fingerprints), so
/// without this guard `bind` could persist a `components.yaml` that
/// every subsequent run fails to load.
fn validate_bindings(file: &BindingsFile, path: &Path) -> Result<()> {
    for (fingerprint, value) in &file.bindings {
        if !is_hex_fingerprint(fingerprint) {
            return Err(Error::validation_failed(
                "catalog-bindings-malformed",
                "each binding key must be a 64-character lowercase-hex fingerprint",
                format!("{}: invalid fingerprint key `{fingerprint}`", path.display()),
            ));
        }
        let slug = value.slug();
        if !specify_error::is_kebab_leading_alpha(slug) {
            return Err(Error::validation_failed(
                "catalog-bindings-malformed",
                "each binding slug must be kebab-case (`^[a-z][a-z0-9]*(-[a-z0-9]+)*$`)",
                format!(
                    "{}: invalid slug `{slug}` for fingerprint `{fingerprint}`",
                    path.display()
                ),
            ));
        }
    }
    Ok(())
}

/// Whether `s` is a 64-character lowercase SHA-256 hex digest, matching
/// the components-catalog schema's `^[0-9a-f]{64}$` fingerprint pattern.
fn is_hex_fingerprint(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// A desired binding before de-collision: the requested bare slug, the
/// fingerprint it anchors to, an optional description, and whether it
/// comes from an operator part (`pinned` = first-writer priority).
struct DesiredBinding {
    slug: String,
    fingerprint: String,
    description: Option<String>,
    pinned: bool,
}

/// A fully resolved binding ready to record: the final (de-collided)
/// slug, the fingerprint it anchors to, and its optional description.
struct ResolvedBinding {
    slug: String,
    fingerprint: String,
    description: Option<String>,
}

/// Resolve desired bindings into final bindings, applying the
/// one-skeleton-per-slug uniqueness guard: when distinct fingerprints
/// want the same bare slug, one keeps it (first-writer-wins) and every
/// later fingerprint is suffixed `slug-<fp-prefix>` — deterministic and
/// fingerprint-derived, never ordinal, so resolution is stable across
/// runs. **Operator parts win the bare slug**: within a slug group
/// a `pinned` binding sorts ahead of skill bindings regardless of
/// fingerprint order; ties break lexicographically by fingerprint. The
/// fingerprint travels through to the catalog so a later `report` run can
/// echo the bound slug (run-to-run stability).
fn resolve_slugs(desired: Vec<DesiredBinding>) -> Vec<ResolvedBinding> {
    // Group by desired bare slug, deterministically.
    let mut by_slug: BTreeMap<String, Vec<DesiredBinding>> = BTreeMap::new();
    for binding in desired {
        by_slug.entry(binding.slug.clone()).or_default().push(binding);
    }

    let mut resolved: Vec<ResolvedBinding> = Vec::new();
    for (slug, mut group) in by_slug {
        // Pinned (operator) first, then lexicographic fingerprint.
        group.sort_by(|a, b| {
            b.pinned.cmp(&a.pinned).then_with(|| a.fingerprint.cmp(&b.fingerprint))
        });
        for (index, binding) in group.into_iter().enumerate() {
            let final_slug = if index == 0 {
                slug.clone()
            } else {
                let prefix: String = binding.fingerprint.chars().take(FP_PREFIX_LEN).collect();
                format!("{slug}-{prefix}")
            };
            resolved.push(ResolvedBinding {
                slug: final_slug,
                fingerprint: binding.fingerprint,
                description: binding.description,
            });
        }
    }
    resolved
}

fn empty_report(unmatched_parts: &[String]) -> Value {
    json!({ "version": 1, "clusters": [], "unmatched-parts": unmatched_parts })
}

fn emit_report(ctx: &Ctx, report: &Value) -> Result<()> {
    ctx.write(report, write_report_text)
}

fn write_report_text(w: &mut dyn Write, report: &Value) -> std::io::Result<()> {
    let clusters = report.get("clusters").and_then(Value::as_array);
    let count = clusters.map_or(0, Vec::len);
    writeln!(w, "clusters: {count}")?;
    if let Some(clusters) = clusters {
        for cluster in clusters {
            let fp = cluster.get("fingerprint").and_then(Value::as_str).unwrap_or("<none>");
            let occ = cluster.get("occurrences").and_then(Value::as_u64).unwrap_or(0);
            let bound = cluster.get("bound-slug").and_then(Value::as_str).unwrap_or("<unbound>");
            writeln!(w, "  - {fp} (occurrences: {occ}, bound: {bound})")?;
        }
    }
    if let Some(unmatched) = report.get("unmatched-parts").and_then(Value::as_array)
        && !unmatched.is_empty()
    {
        writeln!(w, "unmatched parts: {}", unmatched.len())?;
        for part in unmatched {
            if let Some(slug) = part.as_str() {
                writeln!(w, "  - part-unmatched: {slug}")?;
            }
        }
    }
    Ok(())
}

fn emit_bind_diff(
    ctx: &Ctx, added: &[String], unmatched_parts: &[String], dry_run: bool,
) -> Result<()> {
    let body = json!({ "added": added, "unmatched-parts": unmatched_parts, "dry-run": dry_run });
    ctx.write(&body, |w, _| write_bind_text(w, added, unmatched_parts, dry_run))
}

fn write_bind_text(
    w: &mut dyn Write, added: &[String], unmatched_parts: &[String], dry_run: bool,
) -> std::io::Result<()> {
    let verb = if dry_run { "would add" } else { "added" };
    if added.is_empty() {
        writeln!(w, "catalog unchanged (0 components {verb})")?;
    } else {
        writeln!(w, "{} component(s) {verb}:", added.len())?;
        for slug in added {
            writeln!(w, "  + {slug}: confirmed")?;
        }
    }
    if !unmatched_parts.is_empty() {
        writeln!(w, "unmatched parts: {}", unmatched_parts.len())?;
        for slug in unmatched_parts {
            writeln!(w, "  - part-unmatched: {slug}")?;
        }
    }
    Ok(())
}

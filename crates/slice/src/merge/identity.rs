//! Wave-commit requirement identity finalization.
//!
//! Merge rewrites slice specs, `model.yaml`, and `tasks.md` in place
//! before the delta fold so the engine sees final baseline `REQ` ids.

use std::collections::BTreeMap;
use std::path::Path;

use diagnostics::digest::sha256_hex;
use error::Error;
use project::journal::IdentityMap;

use crate::model::SliceModel;
use crate::synthesis::baseline::BaselineIndex;

/// Finalize slice-local requirement ids into baseline numbers.
///
/// Returns the identity maps recorded on `target.merge.wave-committed`.
/// When `model.yaml` is absent (no synthesized requirements), returns
/// an empty map list without touching the tree.
///
/// # Errors
///
/// `merge-base-drifted` when a `MODIFIED` row's recorded baseline-body
/// digest no longer matches the live baseline; filesystem / YAML
/// failures while rewriting artifacts.
pub fn finalize(specs_dir: &Path, slice_dir: &Path) -> Result<Vec<IdentityMap>, Error> {
    let model_path = slice_dir.join("model.yaml");
    if !model_path.is_file() {
        return Ok(Vec::new());
    }
    let mut model = SliceModel::load(&model_path)?;
    if model.requirements.is_empty() {
        return Ok(Vec::new());
    }

    let baseline = BaselineIndex::build(specs_dir)?;
    let maps = assign_maps(&model, &baseline)?;
    if maps.is_empty() {
        return Ok(Vec::new());
    }

    let by_local: BTreeMap<&str, &str> =
        maps.iter().map(|m| (m.local.as_str(), m.baseline.as_str())).collect();

    for req in &mut model.requirements {
        if let Some(local) = req.id.as_deref()
            && let Some(baseline_id) = by_local.get(local)
        {
            req.id = Some((*baseline_id).to_string());
        }
    }
    for task in &mut model.tasks {
        for satisfied in &mut task.satisfies {
            if let Some(baseline_id) = by_local.get(satisfied.as_str()) {
                *satisfied = (*baseline_id).to_string();
            }
        }
    }

    rewrite_spec_ids(slice_dir, &by_local)?;
    rewrite_tasks_ids(slice_dir, &by_local)?;
    artifacts::atomic::yaml_write(&model_path, &model)?;

    Ok(maps)
}

fn assign_maps(model: &SliceModel, baseline: &BaselineIndex) -> Result<Vec<IdentityMap>, Error> {
    let mut next = next_baseline_num(baseline);
    let mut maps = Vec::with_capacity(model.requirements.len());
    for req in &model.requirements {
        let local = req.id.as_deref().ok_or_else(|| {
            Error::validation_failed(
                "merge-identity-missing-id",
                "every model requirement carries a slice-local id at wave commit",
                "a requirement is missing id; re-run the refine phase (`emery plan execute`) \
                 before merging",
            )
        })?;
        let baseline_id = if let Some(baseline_id) = req.baseline_id.as_deref() {
            reject_drifted(req, baseline_id, baseline)?;
            baseline_id.to_string()
        } else {
            let id = format!("REQ-{next:03}");
            next = next.saturating_add(1);
            id
        };
        maps.push(IdentityMap {
            local: local.to_string(),
            baseline: baseline_id,
        });
    }
    Ok(maps)
}

fn reject_drifted(
    req: &crate::model::ModelRequirement, baseline_id: &str, baseline: &BaselineIndex,
) -> Result<(), Error> {
    let domain = req.domain_or_default();
    let Some(expected) = req.baseline_digest.as_deref() else {
        return Err(Error::validation_failed(
            "merge-base-drifted",
            "MODIFIED requirements record a baseline-body digest at synthesize",
            format!(
                "requirement `{baseline_id}` has baseline-id but no baseline-digest; \
                 re-run the refine phase (`emery plan execute`) before merging"
            ),
        ));
    };
    let Some(body) = baseline.body(domain, baseline_id) else {
        return Err(Error::validation_failed(
            "merge-base-drifted",
            "MODIFIED requirements name a live baseline body",
            format!(
                "baseline requirement `{baseline_id}` is absent from domain `{domain}`; \
                 the baseline moved after refine"
            ),
        ));
    };
    let actual = format!("sha256:{}", sha256_hex(body.as_bytes()));
    if actual != expected {
        return Err(Error::validation_failed(
            "merge-base-drifted",
            "MODIFIED baseline body still matches the digest recorded at refine",
            format!(
                "baseline requirement `{baseline_id}` in domain `{domain}` drifted \
                 (recorded `{expected}`, live `{actual}`)"
            ),
        ));
    }
    Ok(())
}

fn next_baseline_num(baseline: &BaselineIndex) -> u32 {
    baseline.domains().map(|(_, domain)| domain.max_req_num).max().unwrap_or(0).saturating_add(1)
}

fn rewrite_spec_ids(slice_dir: &Path, by_local: &BTreeMap<&str, &str>) -> Result<(), Error> {
    let specs = slice_dir.join("specs");
    if !specs.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(&specs).map_err(|source| Error::Filesystem {
        op: "read_dir",
        path: specs.clone(),
        source,
    })? {
        let entry = entry.map_err(|source| Error::Filesystem {
            op: "read_dir",
            path: specs.clone(),
            source,
        })?;
        let spec = entry.path().join("spec.md");
        if !spec.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&spec).map_err(|source| Error::Filesystem {
            op: "read",
            path: spec.clone(),
            source,
        })?;
        let rewritten = rewrite_id_lines(&text, by_local);
        if rewritten != text {
            artifacts::atomic::bytes_write(&spec, rewritten.as_bytes())?;
        }
    }
    Ok(())
}

fn rewrite_tasks_ids(slice_dir: &Path, by_local: &BTreeMap<&str, &str>) -> Result<(), Error> {
    let path = slice_dir.join("tasks.md");
    if !path.is_file() {
        return Ok(());
    }
    let text = std::fs::read_to_string(&path).map_err(|source| Error::Filesystem {
        op: "read",
        path: path.clone(),
        source,
    })?;
    let rewritten = rewrite_req_tokens(&text, by_local);
    if rewritten != text {
        artifacts::atomic::bytes_write(&path, rewritten.as_bytes())?;
    }
    Ok(())
}

fn rewrite_id_lines(text: &str, by_local: &BTreeMap<&str, &str>) -> String {
    let mut out = String::with_capacity(text.len());
    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if let Some(id) = line.strip_prefix("ID: ")
            && let Some(baseline) = by_local.get(id.trim())
        {
            out.push_str("ID: ");
            out.push_str(baseline);
        } else {
            out.push_str(line);
        }
    }
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn rewrite_req_tokens(text: &str, by_local: &BTreeMap<&str, &str>) -> String {
    let mut out = text.to_string();
    for (local, baseline) in by_local {
        out = replace_req_token(&out, local, baseline);
    }
    out
}

/// Replace `from` when not followed by another digit (so `REQ-001` does
/// not clobber a hypothetical `REQ-0010`).
fn replace_req_token(text: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.find(from) {
        out.push_str(&rest[..idx]);
        let after = &rest[idx + from.len()..];
        let boundary_ok = after.chars().next().is_none_or(|c| !c.is_ascii_digit());
        if boundary_ok {
            out.push_str(to);
        } else {
            out.push_str(from);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

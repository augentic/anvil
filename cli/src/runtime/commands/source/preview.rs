//! `specify source preview` handler — workflow-free source adapter
//! execution scaffolding (`specify source preview` contract).
//!
//! Validates `--source`, then runs the shared [`prep`] seam (adapter
//! resolution, brief directory, the four-root sandbox layout, and
//! `evidence/` scaffolding under `--out`) and emits a summary of
//! adapter info, brief paths, and the source binding. The agent then
//! executes the briefs against the prepared environment.
//!
//! Preview is the workflow-free dry run: it consumes [`prep`] but adds
//! none of the workflow-integrated behaviour (sandbox dispatch, cache,
//! journal events, `discovery.md` merge / Evidence persist) the
//! `survey` / `extract` runners layer on the same seam.
//! No `.specify/` writes, no journal events; output lives entirely
//! under `--out` (default `.specify-preview/`). Because preview is
//! slice-less, it preps under the [`prep::SourceOp::Survey`] keying;
//! the resulting scratch path is data only and is never created.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use specify_error::{Error, Result};
use specify_workflow::adapter::SourceOperation;

use crate::runtime::cli::Format;
use crate::runtime::commands::source::prep;
use crate::runtime::output;

const DEFAULT_OUT_DIR: &str = ".specify-preview";

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct BriefEntry {
    operation: SourceOperation,
    path: PathBuf,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct PreviewBody {
    adapter: String,
    version: semver::Version,
    source: PathBuf,
    out: PathBuf,
    evidence_dir: PathBuf,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    leads: Vec<String>,
    briefs: Vec<BriefEntry>,
}

pub fn preview(
    format: Format, adapter_name: &str, source: &Path, leads: &[String], out: Option<&Path>,
    project_dir: &Path,
) -> Result<()> {
    if !source.exists() {
        return Err(Error::Argument {
            flag: "--source",
            detail: format!("path does not exist: {}", source.display()),
        });
    }

    let out_dir = out.map_or_else(|| PathBuf::from(DEFAULT_OUT_DIR), Path::to_path_buf);

    let prepared = prep::prepare(&prep::PrepRequest {
        adapter: adapter_name,
        version: None,
        project_dir,
        op: prep::SourceOp::Survey,
        source: Some(source),
        leads,
        evidence_root: Some(&out_dir),
    })?;

    let briefs: Vec<BriefEntry> = prepared
        .manifest
        .briefs
        .iter()
        .map(|(op, relative)| BriefEntry {
            operation: *op,
            path: prepared.adapter_dir.join(relative),
        })
        .collect();

    let Some(evidence_dir) = prepared.evidence_dir else {
        return Err(Error::Diag {
            code: "source-preview-dir-missing",
            detail: "preview prep did not scaffold the evidence/ directory \
                (evidence_root was None)"
                .to_string(),
        });
    };

    let body = PreviewBody {
        adapter: prepared.manifest.name,
        version: prepared.manifest.version,
        source: source.to_path_buf(),
        out: out_dir,
        evidence_dir,
        leads: prepared.leads,
        briefs,
    };

    output::emit(&mut std::io::stdout().lock(), format, &body, write_preview_text)?;
    Ok(())
}

fn write_preview_text(w: &mut dyn Write, body: &PreviewBody) -> std::io::Result<()> {
    writeln!(w, "adapter: {} {}", body.adapter, body.version)?;
    writeln!(w, "source: {}", body.source.display())?;
    writeln!(w, "out: {}", body.out.display())?;
    writeln!(w, "evidence: {}", body.evidence_dir.display())?;
    if !body.leads.is_empty() {
        writeln!(w, "leads: {}", body.leads.join(", "))?;
    }
    writeln!(w, "briefs:")?;
    for brief in &body.briefs {
        writeln!(w, "  {}: {}", brief.operation, brief.path.display())?;
    }
    Ok(())
}

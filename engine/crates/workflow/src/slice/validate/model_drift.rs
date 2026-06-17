//! Typed-model drift gates over `model.yaml`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value as JsonValue;
use specify_diagnostics::{Artifact, Diagnostic};
use specify_error::{Error, Result};
use specify_model::evidence::ClaimKind;
use specify_model::spec::provenance::{self, ParsedSpec, RequirementStatus};
use specify_model::spec::{is_req_id, is_task_id};

use crate::change::Plan;
use crate::schema::EvidenceDoc;
use crate::slice::expected_provenance_lines;
use crate::slice::model::{SliceModel, validate_model_doc};

/// Emit the drift-validation findings over the slice's
/// `model.yaml`.
///
/// Skipped silently when the file is absent — every
/// synthesized slice carries it, but `slice validate` runs on
/// pre-synthesis (`refining`) slices too, so absence is not a defect
/// here (it is enforced at synthesize time and by
/// `slice provenance` / `slice model show`).
///
/// The schema gate (`slice-model-schema`) and the typed model-derived
/// gates are evaluated independently from the same raw document: the
/// embedded `model.schema.json` overlaps several of the structural
/// checks (e.g. it pins the `REQ` / `TASK` id patterns), so collecting
/// both surfaces every disagreement in one pass. The model-derived
/// checks short-circuit only when the document cannot deserialise into
/// the typed view at all — then the schema finding already explains
/// why, and there is nothing typed left to inspect.
///
/// `plan_path` resolves the target-drift gate; when it does not exist
/// that gate no-ops.
///
/// `evidence` is the parsed Evidence document set [`pre_adapter_gates`](super::pre_adapter_gates)
/// already read and schema-validated; the source-orphan and
/// claim-kind-mismatch gates derive their facts from it rather than
/// re-reading `evidence/*.yaml`.
///
/// # Errors
///
/// Returns [`Error::Filesystem`] when the model or a spec file cannot be
/// read, or a YAML parse error when `model.yaml` is malformed.
pub(super) fn model_drift_findings(
    slice_dir: &Path, plan_path: &Path, slice_name: &str, evidence: &[EvidenceDoc],
) -> Result<Vec<Diagnostic>> {
    let model_path = slice_dir.join("model.yaml");
    if !model_path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&model_path).map_err(|source| Error::Filesystem {
        op: "read",
        path: model_path.clone(),
        source,
    })?;
    let value: JsonValue = serde_saphyr::from_str(&raw)?;

    let mut findings = Vec::new();
    if let Err(Error::Validation { detail, .. }) = validate_model_doc(&value) {
        findings.push(model_drift(
            "slice-model-schema",
            "model.yaml conforms to schemas/slice/model.schema.json",
            detail,
        ));
    }
    let Ok(model) = serde_saphyr::from_str::<SliceModel>(&raw) else {
        return Ok(findings);
    };

    let facts = EvidenceFacts::from_docs(evidence);
    findings.extend(provenance_stale_findings(slice_dir, &model)?);
    findings.extend(target_drift_findings(plan_path, &model, slice_name)?);
    findings.extend(source_orphan_findings(&model, &facts));
    findings.extend(cross_ref_orphan_findings(&model));
    findings.extend(claim_kind_mismatch_findings(&model, &facts));
    findings.extend(id_grammar_findings(&model));
    Ok(findings)
}

fn model_drift(code: &'static str, rule: &'static str, detail: String) -> Diagnostic {
    Diagnostic::violation(code, rule, detail, Artifact::Specs, None)
}

/// `slice-spec-provenance-stale` — compare each model requirement's
/// kernel-owned `id` / `sources` / `status` against the matching
/// requirement parsed from the on-disk `specs/<domain>/spec.md`. A
/// disagreement (or an absent rendered requirement) means an operator
/// hand-edited a kernel-rendered provenance line without
/// re-synthesising.
fn provenance_stale_findings(slice_dir: &Path, model: &SliceModel) -> Result<Vec<Diagnostic>> {
    const RULE: &str = "spec.md provenance lines agree with model.yaml";
    let mut parsed_domains: BTreeMap<String, Option<ParsedSpec>> = BTreeMap::new();
    let mut findings = Vec::new();
    for exp in expected_provenance_lines(model) {
        if exp.id.is_empty() {
            continue;
        }
        if !parsed_domains.contains_key(&exp.domain) {
            let path = slice_dir.join("specs").join(&exp.domain).join("spec.md");
            let parsed = match std::fs::read_to_string(&path) {
                Ok(text) => Some(provenance::parse_spec_md(&text)),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
                Err(source) => {
                    return Err(Error::Filesystem {
                        op: "read",
                        path,
                        source,
                    });
                }
            };
            parsed_domains.insert(exp.domain.clone(), parsed);
        }
        let Some(parsed) = parsed_domains.get(&exp.domain).and_then(Option::as_ref) else {
            findings.push(model_drift(
                "slice-spec-provenance-stale",
                RULE,
                format!(
                    "model requirement `{}` has no rendered `specs/{}/spec.md`",
                    exp.id, exp.domain
                ),
            ));
            continue;
        };
        let Some(req) = parsed.requirements.iter().find(|r| r.id == exp.id) else {
            findings.push(model_drift(
                "slice-spec-provenance-stale",
                RULE,
                format!(
                    "model requirement `{}` is absent from `specs/{}/spec.md`",
                    exp.id, exp.domain
                ),
            ));
            continue;
        };
        if req.sources != exp.sources {
            findings.push(model_drift(
                "slice-spec-provenance-stale",
                RULE,
                format!(
                    "requirement `{}` `Sources:` in `specs/{}/spec.md` ({}) disagrees with \
                     model.yaml ({})",
                    exp.id,
                    exp.domain,
                    render_sources(&req.sources),
                    render_sources(&exp.sources),
                ),
            ));
        }
        if req.status != exp.status {
            findings.push(model_drift(
                "slice-spec-provenance-stale",
                RULE,
                format!(
                    "requirement `{}` `Status:` in `specs/{}/spec.md` ({}) disagrees with \
                     model.yaml ({})",
                    exp.id,
                    exp.domain,
                    render_status(req.status),
                    render_status(exp.status),
                ),
            ));
        }
    }
    Ok(findings)
}

fn render_sources(sources: &[String]) -> String {
    if sources.is_empty() { "<none>".to_string() } else { sources.join(", ") }
}

fn render_status(status: Option<RequirementStatus>) -> String {
    status.map_or_else(|| "<none>".to_string(), |s| s.to_string())
}

/// `slice-model-target-drift` — the persisted `model.yaml.project` must
/// agree with the slice's `plan.yaml` entry `project`. Only flagged
/// when both carry an explicit value and they differ; an omitted plan
/// `project` resolves to the sole topology project, so it cannot
/// disagree. Skipped when no `plan.yaml` exists or the slice has no
/// matching entry. `target` is never persisted, so there is no
/// target-vs-resolved-target half.
fn target_drift_findings(
    plan_path: &Path, model: &SliceModel, name: &str,
) -> Result<Vec<Diagnostic>> {
    if !plan_path.exists() {
        return Ok(Vec::new());
    }
    let plan = Plan::load(plan_path)?;
    let Some(entry) = plan.entries.iter().find(|e| e.name == name) else {
        return Ok(Vec::new());
    };
    match (model.project.as_deref(), entry.project.as_deref()) {
        (Some(model_project), Some(plan_project)) if model_project != plan_project => {
            Ok(vec![Diagnostic::violation(
                "slice-model-target-drift",
                "model.yaml `project` agrees with the slice's plan entry",
                format!(
                    "model.yaml `project: {model_project}` disagrees with plan.yaml slice \
                     `{name}` `project: {plan_project}`"
                ),
                Artifact::Plan,
                None,
            )])
        }
        _ => Ok(Vec::new()),
    }
}

/// `slice-model-source-orphan` — every contributing claim must trace to
/// a real `(source, id)` in the slice's Evidence: the `source` key must
/// own an `evidence/<source>.yaml`, and that file must carry a claim
/// with the cited `id`.
fn source_orphan_findings(model: &SliceModel, evidence: &EvidenceFacts) -> Vec<Diagnostic> {
    const RULE: &str = "every claim traces to a real Evidence `(source, id)`";
    let mut findings = Vec::new();
    for claim in model.requirements.iter().flat_map(|req| &req.claims) {
        if !evidence.sources.contains(&claim.source) {
            findings.push(model_drift(
                "slice-model-source-orphan",
                RULE,
                format!(
                    "claim `{}:{}` references source key `{}`, which has no `evidence/{}.yaml`",
                    claim.source, claim.id, claim.source, claim.source
                ),
            ));
        } else if !evidence.claim_kinds.contains_key(&(claim.source.clone(), claim.id.clone())) {
            findings.push(model_drift(
                "slice-model-source-orphan",
                RULE,
                format!(
                    "claim `{}:{}` references an Evidence claim id absent from `evidence/{}.yaml`",
                    claim.source, claim.id, claim.source
                ),
            ));
        }
    }
    findings
}

/// `slice-model-cross-ref-orphan` — every `tasks[].satisfies[]`
/// reference must name an existing `requirements[].id`.
fn cross_ref_orphan_findings(model: &SliceModel) -> Vec<Diagnostic> {
    const RULE: &str = "every `satisfies[]` reference names an existing requirement";
    let req_ids: BTreeSet<&str> =
        model.requirements.iter().filter_map(|req| req.id.as_deref()).collect();
    let mut findings = Vec::new();
    for task in &model.tasks {
        for req_ref in &task.satisfies {
            if !req_ids.contains(req_ref.as_str()) {
                findings.push(model_drift(
                    "slice-model-cross-ref-orphan",
                    RULE,
                    format!(
                        "task `{}` `satisfies` references `{}`, which is not a `requirements[].id`",
                        task.id, req_ref
                    ),
                ));
            }
        }
    }
    findings
}

/// `slice-model-claim-kind-mismatch` — a claim's `kind` in
/// `model.yaml` must equal the `kind` recorded on the matching Evidence
/// claim. Claims with no matching Evidence `(source, id)` are left to
/// [`source_orphan_findings`].
fn claim_kind_mismatch_findings(model: &SliceModel, evidence: &EvidenceFacts) -> Vec<Diagnostic> {
    const RULE: &str = "claim `kind` agrees with the Evidence claim it traces to";
    let mut findings = Vec::new();
    for claim in model.requirements.iter().flat_map(|req| &req.claims) {
        let key = (claim.source.clone(), claim.id.clone());
        if let Some(evidence_kind) = evidence.claim_kinds.get(&key)
            && *evidence_kind != claim.kind
        {
            findings.push(model_drift(
                "slice-model-claim-kind-mismatch",
                RULE,
                format!(
                    "claim `{}:{}` has `kind: {}` in model.yaml but `kind: {}` in \
                     `evidence/{}.yaml`",
                    claim.source, claim.id, claim.kind, evidence_kind, claim.source
                ),
            ));
        }
    }
    findings
}

/// `slice-model-id-grammar` — `requirements[].id` matches `^REQ-[0-9]{3}$`,
/// `tasks[].id` and `depends-on[]` match `^TASK-[0-9]{3}$`, and
/// `satisfies[]` references match `^REQ-[0-9]{3}$`.
fn id_grammar_findings(model: &SliceModel) -> Vec<Diagnostic> {
    let mut findings = Vec::new();
    for id in model.requirements.iter().filter_map(|req| req.id.as_deref()) {
        if !is_req_id(id) {
            findings.push(id_grammar_finding(format!(
                "requirement id `{id}` does not match `^REQ-[0-9]{{3}}$`"
            )));
        }
    }
    for task in &model.tasks {
        if !is_task_id(&task.id) {
            findings.push(id_grammar_finding(format!(
                "task id `{}` does not match `^TASK-[0-9]{{3}}$`",
                task.id
            )));
        }
        for dep in &task.depends_on {
            if !is_task_id(dep) {
                findings.push(id_grammar_finding(format!(
                    "task `{}` `depends-on` entry `{}` does not match `^TASK-[0-9]{{3}}$`",
                    task.id, dep
                )));
            }
        }
        for req_ref in &task.satisfies {
            if !is_req_id(req_ref) {
                findings.push(id_grammar_finding(format!(
                    "task `{}` `satisfies` entry `{}` does not match `^REQ-[0-9]{{3}}$`",
                    task.id, req_ref
                )));
            }
        }
    }
    findings
}

fn id_grammar_finding(detail: String) -> Diagnostic {
    model_drift(
        "slice-model-id-grammar",
        "`REQ` / `TASK` ids match their closed three-digit grammar",
        detail,
    )
}

/// Per-slice Evidence facts the model-drift checks read: the set of
/// source keys (one per `evidence/*.yaml`) and the `(source, id)` →
/// [`ClaimKind`] map for the source-orphan and kind-mismatch checks.
struct EvidenceFacts {
    sources: BTreeSet<String>,
    claim_kinds: BTreeMap<(String, String), ClaimKind>,
}

impl EvidenceFacts {
    /// Derive the facts from the Evidence documents [`pre_adapter_gates`](super::pre_adapter_gates)
    /// already read and schema-validated, so the file is never read or
    /// parsed a second time. The schema pass runs first and
    /// short-circuits on any read/parse failure, so every `doc.value`
    /// here is a successfully parsed document — the equivalent of the
    /// previous re-read, minus the redundant I/O.
    fn from_docs(docs: &[EvidenceDoc]) -> Self {
        let mut sources = BTreeSet::new();
        let mut claim_kinds = BTreeMap::new();
        for doc in docs {
            let source =
                doc.path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
            sources.insert(source.clone());
            let Some(claims) = doc.value.get("claims").and_then(JsonValue::as_array) else {
                continue;
            };
            for claim in claims {
                let Some(id) = claim.get("id").and_then(JsonValue::as_str) else {
                    continue;
                };
                let Some(kind) = claim
                    .get("kind")
                    .and_then(JsonValue::as_str)
                    .and_then(|raw| raw.parse::<ClaimKind>().ok())
                else {
                    continue;
                };
                claim_kinds.insert((source.clone(), id.to_string()), kind);
            }
        }
        Self { sources, claim_kinds }
    }
}

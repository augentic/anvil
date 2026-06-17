//! `kind: schema` evaluator per `kind: schema` evaluator contract.
//!
//! `hint.value` selects one of two shapes:
//!
//! - **Registered schema id** (bare token; e.g. `codex-rule`) — looks
//!   up an embedded schema [`specify_schema`] ships with the CLI.
//! - **Project-relative `$ref`** (`./...` or `../...`) — reads the
//!   schema file from the project tree, after refusing paths that
//!   escape `project_dir` via `..`.
//!
//! `http(s)://` references are rejected so `specify lint` runs
//! offline and reproducibly.
//!
//! Per-file targeting follows the v1 default — the hint applies to
//! the parsed body of the candidate file. JSON / YAML / TOML files
//! are parsed; markdown files fall back to their extracted
//! frontmatter via [`crate::lint::WorkspaceModel::frontmatter`].
//! The `target: frontmatter` extension from the contract is reserved
//! — the closed [`crate::rules::RuleHint`] shape carries no
//! `target` field, so v1 cannot opt into it.
//!
//! Each `iter_errors` entry maps to one [`specify_diagnostics::Diagnostic`]
//! with `Structured` evidence carrying the failing JSON-pointer.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use jsonschema::Validator;
use serde_json::Value;
use specify_diagnostics::{Diagnostic, FindingEvidence, FindingLocation};
use specify_schema::{
    ADAPTER_JSON_SCHEMA, COMPONENTS_JSON_SCHEMA, DIAGNOSTIC_JSON_SCHEMA,
    DIAGNOSTIC_REPORT_JSON_SCHEMA, EVIDENCE_JSON_SCHEMA, FRAMEWORK_JSON_SCHEMA, PLAN_JSON_SCHEMA,
    PROVENANCE_JSON_SCHEMA, RESOLVED_RULES_JSON_SCHEMA, RULE_JSON_SCHEMA, SCENARIO_JSON_SCHEMA,
    SKILL_JSON_SCHEMA, WORKSPACE_MODEL_JSON_SCHEMA, compile_schema,
};

use super::{HintError, make_finding};
use crate::lint::WorkspaceModel;
use crate::rules::{ResolvedRule, RuleHint};

static REGISTERED_SCHEMAS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        ("adapter", ADAPTER_JSON_SCHEMA),
        ("rule", RULE_JSON_SCHEMA),
        ("skill", SKILL_JSON_SCHEMA),
        ("resolved-rules", RESOLVED_RULES_JSON_SCHEMA),
        ("review-finding", DIAGNOSTIC_JSON_SCHEMA),
        ("review-result", DIAGNOSTIC_REPORT_JSON_SCHEMA),
        ("workspace-model", WORKSPACE_MODEL_JSON_SCHEMA),
        ("scenario", SCENARIO_JSON_SCHEMA),
        ("framework", FRAMEWORK_JSON_SCHEMA),
        ("plan", PLAN_JSON_SCHEMA),
        ("evidence", EVIDENCE_JSON_SCHEMA),
        ("provenance", PROVENANCE_JSON_SCHEMA),
        ("components", COMPONENTS_JSON_SCHEMA),
    ])
});

/// Run-scoped memo for `kind: schema` evaluation.
///
/// Built once per `specify lint` / `specify lint framework` invocation in
/// [`super::evaluate_rules`] and threaded by `&mut` into every per-rule
/// `evaluate_with_cache` call, so a schema referenced by N rules compiles
/// once per run instead of once per rule. Two maps back the two results
/// the previous code recomputed on every evaluation:
///
/// - `validators` — the compiled [`Validator`], keyed by the registered
///   schema id (e.g. `rule`) or, for a project-relative `$ref`, its
///   resolved absolute path (so aliased refs share one validator). The
///   two key namespaces never collide: a project key is always an
///   absolute path under `project_dir`, never a bare registered token.
/// - `resolved_paths` — the normalised project file path keyed by the
///   `$ref` verbatim, so the `..`-escape normalisation runs once per ref.
///
/// Run scope (rather than a process-global `LazyLock`) is what keeps
/// project file schemas honest: a long-lived host that lints several
/// trees — or the in-process integration suite that lints many temp
/// dirs — never serves a validator compiled from a stale or sibling
/// tree. Registered ids are `&'static`, so their identity is stable
/// regardless; sharing one run-scoped map keeps both reference shapes on
/// a single mechanism. A plain `&mut` map (not a lock) sidesteps any
/// guard-lifetime concern.
#[derive(Default)]
pub(crate) struct SchemaCache {
    validators: HashMap<String, Arc<Validator>>,
    resolved_paths: HashMap<String, PathBuf>,
}

impl SchemaCache {
    /// Resolve a project-relative `$ref` to its normalised absolute path,
    /// memoised by the `$ref` verbatim so the escape check runs once.
    fn resolve_project_path(&mut self, project_dir: &Path, raw: &str) -> Result<PathBuf, String> {
        if let Some(path) = self.resolved_paths.get(raw) {
            return Ok(path.clone());
        }
        let resolved = resolve_project_relative(project_dir, raw)?;
        self.resolved_paths.insert(raw.to_owned(), resolved.clone());
        Ok(resolved)
    }
}

/// Fact-family selector that validates the whole `scenarios` fact
/// family directly rather than per-candidate file. Naming the fact
/// family is mechanism; the schema it resolves to is registered.
const SCENARIO_FACT_FAMILY: &str = "scenario";

pub(crate) fn evaluate(
    rule: &ResolvedRule, hint: &RuleHint, candidates: &[PathBuf], project_dir: &Path,
    model: &WorkspaceModel, next_id: &mut u64, cache: &mut SchemaCache,
) -> Result<Vec<Diagnostic>, HintError> {
    let validator = compile_schema_for_hint(rule, hint, project_dir, cache)?;

    // The `scenario` selector validates the dedicated scenario fact
    // family whole-tree: scenario files are kept out of `model.files`,
    // so the candidate set can never select them and the `candidates`
    // argument is intentionally unused here.
    if hint.value.trim() == SCENARIO_FACT_FAMILY {
        return Ok(evaluate_scenarios(rule, &validator, model, next_id));
    }

    let mut out: Vec<Diagnostic> = Vec::new();
    for candidate in candidates {
        let candidate_str = candidate.to_string_lossy().into_owned();
        let Some(instance) =
            load_candidate_instance(model, project_dir, candidate, &candidate_str)?
        else {
            continue;
        };
        for error in validator.iter_errors(&instance) {
            let pointer = error.instance_path().to_string();
            let evidence = FindingEvidence::Structured {
                summary: error.to_string(),
                data: serde_json::json!({ "json_pointer": pointer }),
                locations: None,
            };
            let location = FindingLocation {
                path: candidate_str.clone(),
                line: None,
                column: None,
                end_line: None,
                end_column: None,
            };
            let title = format!("{}: schema validation failed", rule.title);
            let finding = make_finding(rule, *next_id, title, Some(location), evidence);
            *next_id += 1;
            out.push(finding);
        }
    }
    Ok(out)
}

/// Validate every `scenario` fact's parsed frontmatter against the
/// scenario schema, emitting one finding per schema error with the
/// scenario file as the finding location (mirrors the per-candidate
/// loop, but driven by the fact family rather than candidate files).
fn evaluate_scenarios(
    rule: &ResolvedRule, validator: &Validator, model: &WorkspaceModel, next_id: &mut u64,
) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();
    for scenario in &model.scenarios {
        let instance = Value::Object(scenario.fields.clone());
        for error in validator.iter_errors(&instance) {
            let pointer = error.instance_path().to_string();
            let evidence = FindingEvidence::Structured {
                summary: error.to_string(),
                data: serde_json::json!({ "json_pointer": pointer }),
                locations: None,
            };
            let location = FindingLocation {
                path: scenario.path.clone(),
                line: None,
                column: None,
                end_line: None,
                end_column: None,
            };
            let title = format!("{}: schema validation failed", rule.title);
            let finding = make_finding(rule, *next_id, title, Some(location), evidence);
            *next_id += 1;
            out.push(finding);
        }
    }
    out
}

fn compile_schema_for_hint(
    rule: &ResolvedRule, hint: &RuleHint, project_dir: &Path, cache: &mut SchemaCache,
) -> Result<Arc<Validator>, HintError> {
    let raw = hint.value.trim();
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return Err(HintError::SchemaResolve {
            rule_id: rule.rule_id.clone(),
            schema_ref: raw.to_owned(),
            reason: "external http(s) references are not allowed; specify lint must run offline"
                .to_owned(),
        });
    }
    if raw.starts_with("./") || raw.starts_with("../") {
        let resolved = cache.resolve_project_path(project_dir, raw).map_err(|reason| {
            HintError::SchemaResolve {
                rule_id: rule.rule_id.clone(),
                schema_ref: raw.to_owned(),
                reason,
            }
        })?;
        let key = resolved.to_string_lossy().into_owned();
        if let Some(validator) = cache.validators.get(&key) {
            return Ok(Arc::clone(validator));
        }
        let body = std::fs::read_to_string(&resolved).map_err(|err| HintError::Filesystem {
            op: "read-schema",
            path: resolved.clone(),
            source: err,
        })?;
        let validator =
            Arc::new(compile_schema(&body).map_err(|err| HintError::SchemaCompile {
                rule_id: rule.rule_id.clone(),
                schema_ref: raw.to_owned(),
                detail: err.to_string(),
            })?);
        cache.validators.insert(key, Arc::clone(&validator));
        return Ok(validator);
    }
    if let Some(validator) = cache.validators.get(raw) {
        return Ok(Arc::clone(validator));
    }
    let registered = REGISTERED_SCHEMAS.get(raw).ok_or_else(|| HintError::SchemaResolve {
        rule_id: rule.rule_id.clone(),
        schema_ref: raw.to_owned(),
        reason: "unknown registered schema id".to_owned(),
    })?;
    let validator =
        Arc::new(compile_schema(registered).map_err(|err| HintError::SchemaCompile {
            rule_id: rule.rule_id.clone(),
            schema_ref: raw.to_owned(),
            detail: err.to_string(),
        })?);
    cache.validators.insert(raw.to_owned(), Arc::clone(&validator));
    Ok(validator)
}

fn resolve_project_relative(project_dir: &Path, raw: &str) -> Result<PathBuf, String> {
    let candidate = project_dir.join(raw);
    let normalised = normalise(&candidate);
    if !normalised.starts_with(project_dir) {
        return Err("schema reference escapes project directory".to_owned());
    }
    Ok(normalised)
}

fn normalise(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn load_candidate_instance(
    model: &WorkspaceModel, project_dir: &Path, candidate: &Path, candidate_str: &str,
) -> Result<Option<Value>, HintError> {
    let extension = candidate_str.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase());
    match extension.as_deref() {
        Some("json") => {
            let Some(body) = read_text(project_dir, candidate)? else {
                return Ok(None);
            };
            Ok(serde_json::from_str(&body).ok())
        }
        Some("yaml" | "yml") => {
            let Some(body) = read_text(project_dir, candidate)? else {
                return Ok(None);
            };
            Ok(serde_saphyr::from_str(&body).ok())
        }
        Some("toml") => {
            let Some(body) = read_text(project_dir, candidate)? else {
                return Ok(None);
            };
            let Ok(parsed) = toml::from_str::<toml::Value>(&body) else {
                return Ok(Some(Value::Object(serde_json::Map::new())));
            };
            Ok(serde_json::to_value(parsed).ok())
        }
        Some("md") => Ok(model
            .frontmatter
            .iter()
            .find(|fm| fm.path == candidate_str)
            .map(|fm| Value::Object(fm.fields.clone()))),
        _ => Ok(None),
    }
}

fn read_text(project_dir: &Path, candidate: &Path) -> Result<Option<String>, HintError> {
    let absolute = project_dir.join(candidate);
    match std::fs::read_to_string(&absolute) {
        Ok(body) => Ok(Some(body)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(HintError::Filesystem {
            op: "read",
            path: absolute,
            source: err,
        }),
    }
}

#[cfg(test)]
mod unit {
    use std::fs;

    use serde_json::json;

    use super::*;
    use crate::lint::Frontmatter;
    use crate::lint::eval::testkit::{candidates, empty_model, hint, rule};
    use crate::rules::HintKind;

    /// A schema file requiring a string `name` property.
    const NAME_SCHEMA: &str = r#"{
        "type": "object",
        "required": ["name"],
        "properties": { "name": { "type": "string" } }
    }"#;

    fn run(
        hint_value: &str, cands: &[PathBuf], project_dir: &Path, model: &WorkspaceModel,
    ) -> Result<Vec<Diagnostic>, HintError> {
        let hint = hint(HintKind::Schema, hint_value);
        let mut cache = SchemaCache::default();
        evaluate(&rule(), &hint, cands, project_dir, model, &mut 1, &mut cache)
    }

    #[test]
    fn http_refs_rejected() {
        let model = empty_model();
        let result = run("https://example.com/schema.json", &[], Path::new("/tmp"), &model);
        assert!(matches!(result, Err(HintError::SchemaResolve { .. })));
    }

    #[test]
    fn unknown_registered_id_rejected() {
        let model = empty_model();
        let result = run("no-such-schema", &[], Path::new("/tmp"), &model);
        assert!(matches!(result, Err(HintError::SchemaResolve { .. })));
    }

    #[test]
    fn escaping_ref_rejected() {
        let tmp = tempfile::tempdir().expect("tmp");
        let model = empty_model();
        let result = run("../outside.schema.json", &[], tmp.path(), &model);
        assert!(matches!(result, Err(HintError::SchemaResolve { .. })));
    }

    #[test]
    fn project_ref_validates_json_candidates() {
        let tmp = tempfile::tempdir().expect("tmp");
        fs::write(tmp.path().join("name.schema.json"), NAME_SCHEMA).expect("schema");
        fs::write(tmp.path().join("good.json"), r#"{ "name": "x" }"#).expect("good");
        fs::write(tmp.path().join("bad.json"), r#"{ "other": 1 }"#).expect("bad");
        let model = empty_model();
        let out =
            run("./name.schema.json", &candidates(&["good.json", "bad.json"]), tmp.path(), &model)
                .expect("evaluate");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].location.as_ref().map(|l| l.path.as_str()), Some("bad.json"));
    }

    #[test]
    fn markdown_candidates_validate_frontmatter() {
        let tmp = tempfile::tempdir().expect("tmp");
        fs::write(tmp.path().join("name.schema.json"), NAME_SCHEMA).expect("schema");
        let mut model = empty_model();
        let mut bad_fields = serde_json::Map::new();
        bad_fields.insert("other".to_string(), json!(1));
        let mut good_fields = serde_json::Map::new();
        good_fields.insert("name".to_string(), json!("x"));
        model.frontmatter = vec![
            Frontmatter {
                path: "good.md".to_string(),
                schema_id: None,
                fields: good_fields,
            },
            Frontmatter {
                path: "bad.md".to_string(),
                schema_id: None,
                fields: bad_fields,
            },
        ];
        let out =
            run("./name.schema.json", &candidates(&["good.md", "bad.md"]), tmp.path(), &model)
                .expect("evaluate");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].location.as_ref().map(|l| l.path.as_str()), Some("bad.md"));
    }

    #[test]
    fn scenario_family_validated_whole_tree() {
        let mut model = empty_model();
        model.scenarios = vec![crate::lint::Scenario {
            path: "evals/scenarios/empty.md".to_string(),
            id: None,
            stages: vec![],
            expected_artifacts: vec![],
            body_id: None,
            fields: serde_json::Map::new(),
        }];
        // The scenario selector ignores the candidate set entirely.
        let out = run("scenario", &[], Path::new("/tmp"), &model).expect("evaluate");
        assert!(!out.is_empty(), "an empty frontmatter map must fail the scenario schema");
        assert!(
            out.iter()
                .all(|f| f.location.as_ref().is_some_and(|l| l.path == "evals/scenarios/empty.md"))
        );
    }
}

//! JSON-Schema validation helpers shared across crates.
//!
//! [`compile_schema`] parses and compiles an embedded schema string.
//! [`validate_value`] runs a compiled validator over a `serde_json`
//! instance and returns the unified [`ValidationSummary`] shape
//! callers fold into the appropriate [`Error`] variant.
//! [`validate_serialisable`] serialises any [`Serialize`] value and
//! runs the same check; [`read_yaml_as_json`] is the YAML-to-JSON
//! bridge used by file-driven validators.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, LazyLock, RwLock};

use jsonschema::Validator;
use jsonschema::error::{ValidationError, ValidationErrorKind};
use serde::Serialize;
use serde_json::Value as JsonValue;
use specify_error::{Error, Result};

/// Outcome of a single schema-validation check.
///
/// The schema layer is operational: it only ever decides `Pass` /
/// `Fail` deterministically. The richer agent-judgment axis lives on
/// the [`Diagnostic`](https://docs.rs/specify-diagnostics) currency the
/// user-facing `validate` surface emits, not here.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValidationStatus {
    /// Rule passed.
    Pass,
    /// Rule failed.
    Fail,
}

/// Compact result of one schema-validation check.
///
/// Owned by `specify-schema` (the operational schema layer) rather than
/// the error leaf: `Error::Validation` is now payload-free, so the
/// outcome rows live with the validator that produces them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ValidationSummary {
    /// Outcome of this validation check.
    pub status: ValidationStatus,
    /// Stable rule identifier (e.g. `plan-schema`).
    pub rule_id: String,
    /// Human-readable rule description.
    pub rule: String,
    /// Populated for `fail`; `None` for `pass`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Read `path` as UTF-8 YAML and reinterpret the document as a
/// [`serde_json::Value`] for schema validation. Returns a free-form
/// error string so callers can attach their own provenance.
///
/// # Errors
///
/// Returns the underlying I/O or YAML parse message; the caller wraps
/// it into an [`Error`] variant whose `detail` carries the originating
/// path.
pub fn read_yaml_as_json(path: &Path) -> std::result::Result<JsonValue, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("read failed: {err}"))?;
    serde_saphyr::from_str(&raw).map_err(|err| format!("YAML parse failed: {err}"))
}

/// Serialise `value` to JSON and validate against `schema_source`.
///
/// Returns `Ok(())` on a clean validation; otherwise an
/// [`Error::Validation`] whose [`ValidationSummary`] entries carry
/// `rule_id` and `rule`.
///
/// # Errors
///
/// - [`Error::Diag`] when `value` is not JSON-serialisable.
/// - [`Error::Validation`] when the instance fails the schema.
pub fn validate_serialisable<T: Serialize>(
    value: &T, schema_source: &str, rule_id: &str, rule: &str, serialise_code: &'static str,
    serialise_label: &str,
) -> Result<(), Error> {
    let instance = serde_json::to_value(value).map_err(|err| Error::Diag {
        code: serialise_code,
        detail: format!(
            "failed to serialise {serialise_label} to JSON for schema validation: {err}"
        ),
    })?;
    let failures: Vec<ValidationSummary> = validate_value(&instance, schema_source, rule_id, rule)
        .into_iter()
        .filter(|summary| summary.status == ValidationStatus::Fail)
        .collect();
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Error::Validation {
            code: rule_id.to_string().into(),
            detail: join_details(&failures),
        })
    }
}

/// Join the `detail` strings of a failure list into a single
/// payload-free [`Error::Validation`] message.
#[must_use]
pub fn join_details(failures: &[ValidationSummary]) -> String {
    failures.iter().filter_map(|summary| summary.detail.clone()).collect::<Vec<_>>().join("; ")
}

/// Validate `instance` against the embedded JSON Schema `schema_source`.
///
/// Returns one `Pass`-status [`ValidationSummary`] entry on a clean
/// validation, one `Fail` entry with the joined error list on a schema
/// mismatch, or a single `Fail` carrying the meta-failure reason if the
/// embedded schema itself cannot be parsed or compiled. Callers wrap
/// the resulting vector into the [`Error`] variant that suits their
/// exit-code policy: structural manifest checks fold failures into
/// [`Error::Diag`] (exit 1); plan / evidence checks fold into
/// [`Error::Validation`] (exit 2).
#[must_use]
pub fn validate_value(
    instance: &JsonValue, schema_source: &str, rule_id: &str, rule: &str,
) -> Vec<ValidationSummary> {
    match compile_schema(schema_source) {
        Ok(validator) => summarise(&validator, instance, rule_id, rule),
        Err(err) => vec![meta_failure(rule_id, rule, &err.to_string())],
    }
}

/// Cache-backed twin of [`validate_value`] for the embedded `&'static`
/// schema constants on hot validation paths (per-`evidence/*.yaml`,
/// per-lead, per-manifest, plan/proposal/build-request).
///
/// The compiled [`Validator`] is built once per process per schema and
/// reused on every subsequent call; behaviour is byte-identical to
/// [`validate_value`], only the validator is no longer recompiled.
/// Requiring `&'static str` is what makes the pointer-identity cache key
/// sound: a `'static` schema's backing bytes never move or get freed, so
/// its address is stable and collision-free for the program's lifetime
/// (distinct schema sources have distinct addresses; the compiler may
/// merge byte-identical constants, which is harmless).
///
/// `$ref`-bearing schemas (synthesis, build-report) are *not* served
/// here — they need a [`jsonschema::Registry`] and keep their dedicated
/// `LazyLock<Validator>` statics in the workflow layer.
#[must_use]
pub fn validate_value_cached(
    instance: &JsonValue, schema_source: &'static str, rule_id: &str, rule: &str,
) -> Vec<ValidationSummary> {
    match cached_validator(schema_source) {
        Ok(validator) => summarise(&validator, instance, rule_id, rule),
        Err(err) => vec![meta_failure(rule_id, rule, &err.to_string())],
    }
}

/// Process-lifetime cache of compiled validators keyed by the
/// `&'static str` schema source's address.
///
/// An [`RwLock`] guards a pointer-keyed map of [`Arc<Validator>`]: reads
/// (the steady state after warmup) take the shared lock; a miss upgrades
/// to the write lock, double-checks, then compiles under the lock so a
/// schema is compiled at most once even under concurrent first use.
static VALIDATOR_CACHE: LazyLock<RwLock<HashMap<usize, Arc<Validator>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Fetch (or compile-and-insert) the cached validator for a `&'static`
/// schema, the shared schema-cache entry point for every cache-backed
/// validator (the embedded-`&'static`-schema callers).
///
/// # Errors
///
/// - [`Error::Diag`] (`schema-cache-poisoned`) when the cache lock is
///   poisoned by a panic on another thread. Recoverable rather than a
///   propagated panic so a poisoned cache never takes down the CLI.
/// - Propagates the [`compile_schema`] meta-failure ([`Error::Diag`])
///   when the embedded schema cannot be parsed or compiled —
///   unreachable in production (it signals a corrupt binary) but kept
///   honest rather than `expect`ed.
pub fn cached_validator(schema_source: &'static str) -> Result<Arc<Validator>> {
    let key = schema_source.as_ptr() as usize;
    if let Some(validator) = cache_read()?.get(&key) {
        return Ok(Arc::clone(validator));
    }
    let mut cache = cache_write()?;
    if let Some(validator) = cache.get(&key) {
        return Ok(Arc::clone(validator));
    }
    let validator = Arc::new(compile_schema(schema_source)?);
    cache.insert(key, Arc::clone(&validator));
    drop(cache);
    Ok(validator)
}

/// Poison-mapped read guard onto [`VALIDATOR_CACHE`].
fn cache_read() -> Result<std::sync::RwLockReadGuard<'static, HashMap<usize, Arc<Validator>>>> {
    VALIDATOR_CACHE.read().map_err(|_poison| poisoned())
}

/// Poison-mapped write guard onto [`VALIDATOR_CACHE`].
fn cache_write() -> Result<std::sync::RwLockWriteGuard<'static, HashMap<usize, Arc<Validator>>>> {
    VALIDATOR_CACHE.write().map_err(|_poison| poisoned())
}

fn poisoned() -> Error {
    Error::Diag {
        code: "schema-cache-poisoned",
        detail: "compiled-validator cache lock was poisoned by a prior panic".to_string(),
    }
}

/// Run a compiled `validator` over `instance` and fold its errors into
/// the single-entry [`ValidationSummary`] vector both entry points return.
fn summarise(
    validator: &Validator, instance: &JsonValue, rule_id: &str, rule: &str,
) -> Vec<ValidationSummary> {
    let errors: Vec<String> =
        validator.iter_errors(instance).map(|err| validation_error_detail(&err)).collect();
    if errors.is_empty() {
        vec![ValidationSummary {
            status: ValidationStatus::Pass,
            rule_id: rule_id.into(),
            rule: rule.into(),
            detail: None,
        }]
    } else {
        vec![ValidationSummary {
            status: ValidationStatus::Fail,
            rule_id: rule_id.into(),
            rule: rule.into(),
            detail: Some(errors.join("; ")),
        }]
    }
}

/// The `Fail` summary emitted when the embedded schema itself cannot be
/// parsed or compiled (a corrupt-binary meta-failure).
fn meta_failure(rule_id: &str, rule: &str, detail: &str) -> ValidationSummary {
    ValidationSummary {
        status: ValidationStatus::Fail,
        rule_id: rule_id.into(),
        rule: rule.into(),
        detail: Some(detail.to_string()),
    }
}

/// Parse and compile an embedded JSON Schema string.
///
/// # Errors
///
/// Returns [`Error::Diag`] with `schema-meta-loadable` if the schema
/// source is not valid JSON, or `schema-meta-compilable` if the JSON
/// is not a valid JSON Schema.
pub fn compile_schema(schema_source: &str) -> Result<Validator> {
    let schema: JsonValue = serde_json::from_str(schema_source).map_err(|err| Error::Diag {
        code: "schema-meta-loadable",
        detail: format!("embedded JSON Schema does not parse as JSON: {err}"),
    })?;
    jsonschema::validator_for(&schema).map_err(|err| Error::Diag {
        code: "schema-meta-compilable",
        detail: format!("embedded JSON Schema does not compile: {err}"),
    })
}

pub(crate) fn validation_error_detail(err: &ValidationError<'_>) -> String {
    let path = match err.kind() {
        ValidationErrorKind::AdditionalProperties { unexpected } if unexpected.len() == 1 => {
            child_pointer(&err.instance_path().to_string(), &unexpected[0])
        }
        _ => err.instance_path().to_string(),
    };
    format!("{path}: {err}")
}

pub(crate) fn child_pointer(parent: &str, property: &str) -> String {
    let property = property.replace('~', "~0").replace('/', "~1");
    if parent.is_empty() { format!("/{property}") } else { format!("{parent}/{property}") }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::json;
    use specify_error::Error;

    use super::{
        ValidationStatus, cached_validator, child_pointer, compile_schema, join_details,
        validate_serialisable, validate_value, validate_value_cached,
    };

    const OBJECT_SCHEMA: &str = r#"{
        "type": "object",
        "additionalProperties": false,
        "required": ["name"],
        "properties": { "name": { "type": "string" } }
    }"#;

    #[test]
    fn compile_schema_rejects_non_json_source() {
        let err = compile_schema("{ not json").expect_err("garbage source fails to parse");
        match err {
            Error::Diag { code, .. } => assert_eq!(code, "schema-meta-loadable"),
            other => panic!("expected schema-meta-loadable Diag, got {other:?}"),
        }
    }

    #[test]
    fn rejects_non_schema_json() {
        let err = compile_schema(r#"{ "type": "frobnicate" }"#)
            .expect_err("unknown type keyword fails to compile");
        match err {
            Error::Diag { code, .. } => assert_eq!(code, "schema-meta-compilable"),
            other => panic!("expected schema-meta-compilable Diag, got {other:?}"),
        }
    }

    #[test]
    fn passes_conforming_instance() {
        let summaries =
            validate_value(&json!({ "name": "ok" }), OBJECT_SCHEMA, "object", "object shape");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].status, ValidationStatus::Pass);
        assert!(summaries[0].detail.is_none(), "pass carries no detail");
    }

    #[test]
    fn reports_additional_property() {
        let summaries = validate_value(
            &json!({ "name": "ok", "extra": 1 }),
            OBJECT_SCHEMA,
            "object",
            "object shape",
        );
        assert_eq!(summaries[0].status, ValidationStatus::Fail);
        let detail = summaries[0].detail.as_deref().expect("fail carries detail");
        assert!(detail.starts_with("/extra:"), "pointer names the offending key, got {detail:?}");
    }

    #[test]
    fn surfaces_meta_failure() {
        let summaries = validate_value(&json!({}), "{ not json", "bad", "bad schema");
        assert_eq!(summaries[0].status, ValidationStatus::Fail);
    }

    #[derive(Serialize)]
    struct Doc {
        name: String,
    }

    #[derive(Serialize)]
    struct Wrong {
        name: u32,
    }

    #[test]
    fn serialisable_ok_and_err() {
        validate_serialisable(
            &Doc { name: "ok".into() },
            OBJECT_SCHEMA,
            "object",
            "object shape",
            "doc-serialise",
            "doc",
        )
        .expect("conforming value passes");

        let err = validate_serialisable(
            &Wrong { name: 7 },
            OBJECT_SCHEMA,
            "object",
            "object shape",
            "doc-serialise",
            "doc",
        )
        .expect_err("type mismatch fails");
        match err {
            Error::Validation { code, .. } => assert_eq!(code, "object"),
            other => panic!("expected Validation error, got {other:?}"),
        }
    }

    #[test]
    fn join_details_only_fail_details() {
        let summaries = validate_value(&json!({}), OBJECT_SCHEMA, "object", "object shape");
        let joined = join_details(&summaries);
        assert!(joined.contains("name"), "missing required field appears in joined detail");
    }

    #[test]
    fn cached_validator_compiles_once_per_schema() {
        let first = cached_validator(OBJECT_SCHEMA).expect("first compile succeeds");
        let second = cached_validator(OBJECT_SCHEMA).expect("cache hit on second call");
        assert!(
            std::sync::Arc::ptr_eq(&first, &second),
            "the same `&'static` schema yields one cached validator, not a recompile"
        );
    }

    #[test]
    fn cached_matches_uncached_behaviour() {
        let pass = validate_value_cached(
            &json!({ "name": "ok" }),
            OBJECT_SCHEMA,
            "object",
            "object shape",
        );
        assert_eq!(
            pass,
            validate_value(&json!({ "name": "ok" }), OBJECT_SCHEMA, "object", "object shape")
        );
        assert_eq!(pass[0].status, ValidationStatus::Pass);

        let fail = validate_value_cached(
            &json!({ "name": "ok", "extra": 1 }),
            OBJECT_SCHEMA,
            "object",
            "object shape",
        );
        assert_eq!(
            fail,
            validate_value(
                &json!({ "name": "ok", "extra": 1 }),
                OBJECT_SCHEMA,
                "object",
                "object shape"
            )
        );
        assert_eq!(fail[0].status, ValidationStatus::Fail);
    }

    #[test]
    fn child_pointer_escapes_metachars() {
        assert_eq!(child_pointer("", "a"), "/a");
        assert_eq!(child_pointer("/parent", "child"), "/parent/child");
        assert_eq!(child_pointer("", "a/b~c"), "/a~1b~0c", "/ -> ~1 and ~ -> ~0");
    }
}

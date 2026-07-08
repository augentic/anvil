//! Derived judgment answer schemas.
//!
//! Every adapter judgment operation issues `omnia:model/completion.create`
//! with `format: schema(...)` so the host gate validates the answer before
//! the guest sees it. The three payload documents here are never
//! hand-written: each derives from an embedded canonical schema — the
//! canonical document stays the source of truth — by stripping the
//! call-scoped envelope fields the caller already knows and inlining
//! cross-file `$ref`s so one self-contained document rides the model call.
//! The generated copies under `schemas/answers/` are parity-gated against
//! this derivation by `crates/schema/tests/answers.rs`.

use serde_json::{Value, json};

use crate::constants::{
    BUILD_REPORT_JSON_SCHEMA, DIAGNOSTIC_JSON_SCHEMA, EVIDENCE_JSON_SCHEMA, LEAD_JSON_SCHEMA,
    PROPOSAL_JSON_SCHEMA, SLICE_MODEL_JSON_SCHEMA, SYNTHESIS_JSON_SCHEMA,
};

const ANSWERS_ID_BASE: &str = "https://github.com/augentic/specify/schemas/answers";

/// The `survey` answer: `{ "leads": [ ... ] }`, each item the canonical
/// lead shape minus the envelope `source` key (the caller attributes
/// every lead a survey produces to the surveyed source itself).
#[must_use]
pub fn leads() -> Value {
    let mut lead = parse(LEAD_JSON_SCHEMA);
    strip_envelope(&mut lead, &["source"]);
    let defs = take(&mut lead, "$defs");
    for key in ["$schema", "$id", "title"] {
        take(&mut lead, key);
    }

    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": format!("{ANSWERS_ID_BASE}/leads.schema.json"),
        "title": "Specify survey answer",
        "description": "Generated judgment-answer schema — derived from \
            schemas/discovery/lead.schema.json by specify-schema's answer \
            derivation; do not edit. Validates the schema-gated answer to a \
            source adapter's survey operation: an object carrying `leads[]`, \
            each item the canonical lead shape minus the envelope `source` \
            key (the caller attributes every lead to the surveyed source).",
        "type": "object",
        "additionalProperties": false,
        "required": ["leads"],
        "properties": {
            "leads": {
                "type": "array",
                "description": "Every lead the survey surfaced, in source order.",
                "items": lead,
            }
        },
        "$defs": defs,
    })
}

/// The `extract` answer: the canonical Evidence shape minus the envelope
/// `lead` key (the extract call names the lead, so the answer never
/// repeats it).
#[must_use]
pub fn evidence() -> Value {
    let mut evidence = parse(EVIDENCE_JSON_SCHEMA);
    strip_envelope(&mut evidence, &["lead"]);
    set(&mut evidence, "$id", json!(format!("{ANSWERS_ID_BASE}/evidence.schema.json")));
    set(&mut evidence, "title", json!("Specify extract answer"));
    set(
        &mut evidence,
        "description",
        json!(
            "Generated judgment-answer schema — derived from \
             schemas/evidence.schema.json by specify-schema's answer \
             derivation; do not edit. Validates the schema-gated answer to a \
             source adapter's extract operation: the canonical Evidence shape \
             minus the envelope `lead` key (the extract call names the lead)."
        ),
    );
    evidence
}

/// The `build` / `merge` answer.
///
/// The canonical build-report shape minus the envelope keys (`version`,
/// `slice`, `target`) the caller already knows, with the cross-file
/// diagnostic `$ref` inlined so the document is self-contained.
///
/// # Panics
///
/// Panics when the embedded canonical schemas drop a shape this
/// derivation depends on — a compile-adjacent invariant, not a runtime
/// input condition.
#[must_use]
pub fn report() -> Value {
    let mut report = parse(BUILD_REPORT_JSON_SCHEMA);
    strip_envelope(&mut report, &["version", "slice", "target"]);
    set(&mut report, "$id", json!(format!("{ANSWERS_ID_BASE}/report.schema.json")));
    set(&mut report, "title", json!("Specify build/merge answer"));
    set(
        &mut report,
        "description",
        json!(
            "Generated judgment-answer schema — derived from \
             schemas/target/build-report.schema.json (with \
             schemas/diagnostics/diagnostic.schema.json inlined) by \
             specify-schema's answer derivation; do not edit. Validates the \
             schema-gated answer to a target adapter's build or merge \
             operation: the canonical report shape minus the envelope keys \
             (`version`, `slice`, `target`) the caller already knows."
        ),
    );

    // The findings[] items `$ref` points at the sibling diagnostic file;
    // a self-contained payload inlines it: the diagnostic's own `$defs`
    // hoist to the root under a `diagnostic-` prefix (no collisions with
    // the report's defs), its internal refs are rewritten to match, and
    // the body lands at `#/$defs/diagnostic`.
    let mut diagnostic = parse(DIAGNOSTIC_JSON_SCHEMA);
    for key in ["$schema", "$id"] {
        take(&mut diagnostic, key);
    }
    let mut hoisted = take(&mut diagnostic, "$defs");
    rewrite_local_refs(&mut diagnostic, "diagnostic-");
    rewrite_local_refs(&mut hoisted, "diagnostic-");

    let defs = report
        .get_mut("$defs")
        .and_then(Value::as_object_mut)
        .expect("build-report schema carries $defs");
    defs.insert("diagnostic".to_string(), diagnostic);
    if let Value::Object(entries) = hoisted {
        for (name, def) in entries {
            defs.insert(format!("diagnostic-{name}"), def);
        }
    }
    set_pointer(&mut report, "/properties/findings/items", json!({ "$ref": "#/$defs/diagnostic" }));
    report
}

/// The plan-time lead-reconciliation answer.
///
/// The canonical proposal envelope narrowed to its `kind: response`
/// arm, with the request-only `$defs` trimmed away so the document
/// riding the model call carries only the response vocabulary.
///
/// The `version` / `kind` envelope consts are deliberately kept (not
/// stripped like the survey/extract envelope keys): the deterministic
/// tail validates the raw answer against the full canonical envelope
/// (`validate_proposal_json`), so an answer carrying the consts flows
/// through unchanged.
///
/// # Panics
///
/// Panics when the embedded canonical schema drops a shape this
/// derivation depends on — a compile-adjacent invariant, not a runtime
/// input condition.
#[must_use]
pub fn proposal() -> Value {
    let mut envelope = parse(PROPOSAL_JSON_SCHEMA);
    let mut defs = take(&mut envelope, "$defs");
    let mut response = take(&mut defs, "response");
    // The request arm and its exclusive vocabulary never ride the call.
    for request_only in [
        "request",
        "leadCatalogEntry",
        "projectRef",
        "surfaceDomain",
        "decision",
        "platform",
        "targetRef",
    ] {
        take(&mut defs, request_only);
    }
    // A def added to the canonical schema must be classified here —
    // response-reachable stays, request-only joins the trim list.
    let residual: Vec<&String> = defs.as_object().expect("$defs is an object").keys().collect();
    assert_eq!(
        residual,
        ["disagreement", "gateProse", "kebabName", "responseMember", "responseSlice"],
        "unclassified canonical $defs entry; extend the request-only trim list or accept it \
         into the answer vocabulary"
    );

    let body = response.as_object_mut().expect("response def is an object");
    // Gate 1 prose is optional on the canonical envelope but mandatory
    // on the judgment answer: the collapsed `plan author` orchestration
    // persists the prose into `change.md` / `discovery.md`, so an
    // answer without it is incomplete.
    let required = body.get_mut("required").and_then(Value::as_array_mut).expect("required array");
    assert!(!required.iter().any(|entry| entry == "gate"), "gate must be canonically optional");
    required.push(json!("gate"));
    body.insert("$schema".to_string(), json!("https://json-schema.org/draft/2020-12/schema"));
    body.insert("$id".to_string(), json!(format!("{ANSWERS_ID_BASE}/proposal.schema.json")));
    body.insert("title".to_string(), json!("Specify plan reconciliation answer"));
    body.insert(
        "description".to_string(),
        json!(
            "Generated judgment-answer schema — derived from \
             schemas/discovery/proposal.schema.json by specify-schema's answer \
             derivation; do not edit. Validates the schema-gated answer to the \
             plan-time lead-reconciliation judgment: the canonical `kind: \
             response` envelope arm the guest `plan author` orchestration \
             consumes, with the request-only definitions trimmed."
        ),
    );
    body.insert("$defs".to_string(), defs);
    response
}

/// The slice synthesis answer.
///
/// The canonical synthesis-response envelope with the cross-file
/// `model.schema.json` `$ref` inlined so one self-contained document
/// rides the model call.
///
/// As with [`proposal`], the envelope fields (`version` / `kind` /
/// `slice`) are kept: the deterministic tail validates the raw answer
/// against the full canonical envelope (`validate_synthesis_json`).
///
/// # Panics
///
/// Panics when the embedded canonical schemas drop a shape this
/// derivation depends on — a compile-adjacent invariant, not a runtime
/// input condition.
#[must_use]
pub fn synthesis() -> Value {
    let mut synthesis = parse(SYNTHESIS_JSON_SCHEMA);
    set(&mut synthesis, "$id", json!(format!("{ANSWERS_ID_BASE}/synthesis.schema.json")));
    set(&mut synthesis, "title", json!("Specify slice synthesis answer"));
    set(
        &mut synthesis,
        "description",
        json!(
            "Generated judgment-answer schema — derived from \
             schemas/slice/synthesis.schema.json (with \
             schemas/slice/model.schema.json inlined) by specify-schema's \
             answer derivation; do not edit. Validates the schema-gated \
             answer to the slice-synthesis judgment: the canonical `kind: \
             response` envelope the synthesis kernel projects and persists."
        ),
    );

    // The `model` property `$ref`s the sibling model.schema.json; a
    // self-contained payload inlines it: the model schema's own `$defs`
    // hoist to the root under a `model-` prefix, its internal refs are
    // rewritten to match, and the body lands at `#/$defs/model`.
    let mut model = parse(SLICE_MODEL_JSON_SCHEMA);
    for key in ["$schema", "$id"] {
        take(&mut model, key);
    }
    let mut hoisted = take(&mut model, "$defs");
    rewrite_local_refs(&mut model, "model-");
    rewrite_local_refs(&mut hoisted, "model-");

    let defs = synthesis
        .get_mut("$defs")
        .and_then(Value::as_object_mut)
        .expect("synthesis schema carries $defs");
    defs.insert("model".to_string(), model);
    if let Value::Object(entries) = hoisted {
        for (name, def) in entries {
            defs.insert(format!("model-{name}"), def);
        }
    }
    set_pointer(&mut synthesis, "/properties/model", json!({ "$ref": "#/$defs/model" }));
    synthesis
}

/// Render an answer schema the way the generated `schemas/answers/`
/// files are committed: pretty-printed with a trailing newline.
///
/// # Panics
///
/// Panics when the value is not JSON-serialisable — impossible for the
/// derivations above.
#[must_use]
pub fn render(schema: &Value) -> String {
    let mut rendered = serde_json::to_string_pretty(schema).expect("answer schema serialises");
    rendered.push('\n');
    rendered
}

fn parse(source: &str) -> Value {
    serde_json::from_str(source).expect("embedded canonical schema parses")
}

// Strip call-scoped envelope fields from an object schema: each name
// leaves `required` and its property schema becomes `false`, which
// rejects a document that repeats the field even when the canonical
// schema is `additionalProperties: true` (evidence claims stay open).
// The canonical schema must carry the name in both places, so a rename
// there breaks the derivation loudly.
fn strip_envelope(schema: &mut Value, names: &[&str]) {
    for name in names {
        let replaced = schema
            .get_mut("properties")
            .and_then(Value::as_object_mut)
            .and_then(|properties| properties.insert((*name).to_string(), Value::Bool(false)));
        assert!(replaced.is_some(), "canonical schema carries envelope property `{name}`");

        let required =
            schema.get_mut("required").and_then(Value::as_array_mut).expect("required array");
        let position = required
            .iter()
            .position(|entry| entry.as_str() == Some(*name))
            .unwrap_or_else(|| panic!("canonical schema requires envelope property `{name}`"));
        required.remove(position);
    }
}

// Rewrite every local `"$ref": "#/$defs/<name>"` to `#/$defs/<prefix><name>`.
fn rewrite_local_refs(value: &mut Value, prefix: &str) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(reference)) = map.get_mut("$ref")
                && let Some(name) = reference.strip_prefix("#/$defs/")
            {
                *reference = format!("#/$defs/{prefix}{name}");
            }
            for entry in map.values_mut() {
                rewrite_local_refs(entry, prefix);
            }
        }
        Value::Array(items) => {
            for item in items {
                rewrite_local_refs(item, prefix);
            }
        }
        _ => {}
    }
}

fn take(schema: &mut Value, key: &str) -> Value {
    schema
        .as_object_mut()
        .and_then(|map| map.remove(key))
        .unwrap_or_else(|| panic!("canonical schema carries `{key}`"))
}

fn set(schema: &mut Value, key: &str, value: Value) {
    let replaced =
        schema.as_object_mut().expect("schema is an object").insert(key.to_string(), value);
    assert!(replaced.is_some(), "canonical schema carries `{key}` to replace");
}

fn set_pointer(schema: &mut Value, pointer: &str, value: Value) {
    let slot = schema
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("canonical schema resolves `{pointer}`"));
    *slot = value;
}

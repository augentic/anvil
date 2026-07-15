//! Generated judgment answer schemas.
//!
//! Every adapter judgment operation issues `omnia:model/completion.create`
//! with `format: schema(...)` so the host gate validates the answer before
//! the guest sees it. The documents here are never hand-written: each is
//! generated (via `schemars`) from the Rust wire type that deserialises
//! the answer, so the types stay the single source of truth. Safely
//! expressible semantic constraints (the kebab id grammars, the
//! per-kind claim id requirement) are patched onto the generated shape
//! so the host gate rejects them early; deterministic tails remain the
//! authority and additionally cover what a schema cannot express
//! (trim-aware synopsis checks, cross-field domain checks).
//!
//! The committed copies under `crates/project/answers/` (and
//! `crates/slice/answers/` for the synthesis leg) are parity-gated
//! against this generation by `crates/project/tests/answers.rs` and
//! `crates/slice/tests/answers.rs`; adapters in `augentic/specify-adapters`
//! vendor the `leads` / `evidence` / `report` documents.

use schemars::JsonSchema;
use serde_json::{Value, json};

use crate::seam::wire::{BuildOutput, BuildStatus, UiSurface};
use crate::seam::{Evidence, Lead};

/// `$id` base for the generated answer documents.
const ANSWERS_ID_BASE: &str = "https://github.com/augentic/specify/answers";

/// Kebab slug grammar for lead ids and topic slugs, mirroring
/// `artifacts::evidence::is_kebab` in the deterministic tail.
const KEBAB_PATTERN: &str = "^[a-z0-9]+(-[a-z0-9]+)*$";

/// Dotted-kebab claim-id grammar, mirroring
/// `artifacts::evidence::claim::is_dotted_kebab` in the deterministic
/// tail.
const DOTTED_KEBAB_PATTERN: &str = "^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$";

/// The `survey` answer envelope: `{ "leads": [ ... ] }`, each item a
/// [`Lead`] (the discovery lead minus the envelope `source` key — the
/// caller attributes every lead a survey produces to the surveyed
/// source itself).
#[derive(JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[expect(dead_code, reason = "fields exist only for schemars schema generation")]
struct LeadsAnswer {
    /// Every lead the survey surfaced, in source order.
    leads: Vec<Lead>,
}

/// The `build` / `merge` answer: the report minus the envelope keys
/// (`version`, `slice`, `target`) the caller already knows and stamps
/// when widening onto the canonical report.
#[derive(JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[expect(dead_code, reason = "fields exist only for schemars schema generation")]
struct ReportAnswer {
    /// Operation outcome as judged by the model.
    status: BuildStatus,
    /// Full structured diagnostics; default `[]`.
    #[serde(default)]
    findings: Vec<diagnostics::Diagnostic>,
    /// Per-platform build outputs; default `[]`.
    #[serde(default)]
    outputs: Vec<BuildOutput>,
    /// Optional UI-surface signal.
    #[serde(default)]
    ui_surface: Option<UiSurface>,
}

/// Generate the root answer schema for `T`, stamping the `$id`,
/// `title`, and `description` metadata.
///
/// # Panics
///
/// Panics when the generated schema is not a JSON object — impossible
/// for the derived `JsonSchema` implementations.
#[must_use]
pub fn root_schema<T: JsonSchema>(file: &str, title: &str, description: &str) -> Value {
    let schema = schemars::schema_for!(T);
    let mut value = schema.to_value();
    let object = value.as_object_mut().expect("generated answer schema is an object");
    object.insert("$id".to_string(), json!(format!("{ANSWERS_ID_BASE}/{file}")));
    object.insert("title".to_string(), json!(title));
    object.insert("description".to_string(), json!(description));
    value
}

/// The `survey` answer schema.
///
/// The generated shape is patched with the kebab grammar on lead ids
/// and topic slugs, so the host gate rejects malformed slugs before
/// the deterministic tail re-checks them.
///
/// # Panics
///
/// Panics when the generated schema drops the patched properties — a
/// compile-adjacent invariant, not a runtime input condition.
#[must_use]
pub fn leads() -> Value {
    let mut schema = root_schema::<LeadsAnswer>(
        "leads.schema.json",
        "Specify survey answer",
        "Generated judgment-answer schema — generated from the Rust wire types by \
         project::answers; do not edit. Validates the schema-gated answer to a source \
         adapter's survey operation: an object carrying `leads[]`, each item the lead shape \
         minus the envelope `source` key (the caller attributes every lead to the surveyed \
         source). Lead ids and topic slugs carry the kebab-case grammar in-schema and are \
         re-checked deterministically after the gate, alongside the trim-aware synopsis \
         check the schema cannot express.",
    );
    set_pattern(&mut schema, "/$defs/Lead/properties/lead", KEBAB_PATTERN);
    set_pattern(&mut schema, "/$defs/Lead/properties/topics/items", KEBAB_PATTERN);
    schema
}

/// The `extract` answer schema: the Evidence shape minus the envelope
/// `lead` key (the extract call names the lead, so the answer never
/// repeats it).
///
/// The generated shape is patched with the dotted-kebab grammar on
/// claim ids and a conditional `required` making the id mandatory on
/// `requirement` / `criterion` / `example` claims, so the host gate
/// rejects those defects before the deterministic tail re-checks them.
///
/// # Panics
///
/// Panics when the generated schema drops the patched properties — a
/// compile-adjacent invariant, not a runtime input condition.
#[must_use]
pub fn evidence() -> Value {
    let mut schema = root_schema::<Evidence>(
        "evidence.schema.json",
        "Specify extract answer",
        "Generated judgment-answer schema — generated from the Rust wire types by \
         project::answers; do not edit. Validates the schema-gated answer to a source \
         adapter's extract operation: the Evidence shape minus the envelope `lead` key (the \
         extract call names the lead). Per-kind claim body fields are intentionally open; \
         claim ids carry the dotted-kebab grammar in-schema and `requirement` / `criterion` \
         / `example` claims conditionally require one — both re-checked deterministically \
         after the gate.",
    );
    set_pattern(&mut schema, "/$defs/Claim/properties/id", DOTTED_KEBAB_PATTERN);
    let claim = schema
        .pointer_mut("/$defs/Claim")
        .and_then(Value::as_object_mut)
        .expect("evidence answer schema carries the Claim definition");
    claim.insert(
        "if".to_string(),
        json!({ "properties": { "kind": { "enum": ["requirement", "criterion", "example"] } } }),
    );
    claim.insert(
        "then".to_string(),
        json!({
            "properties": { "id": { "pattern": DOTTED_KEBAB_PATTERN, "type": "string" } },
            "required": ["id"]
        }),
    );
    schema
}

/// The `build` / `merge` answer schema.
#[must_use]
pub fn report() -> Value {
    root_schema::<ReportAnswer>(
        "report.schema.json",
        "Specify build/merge answer",
        "Generated judgment-answer schema — generated from the Rust wire types by \
         project::answers; do not edit. Validates the schema-gated answer to a target \
         adapter's build or merge operation: the report shape minus the envelope keys \
         (`version`, `slice`, `target`) the caller already knows.",
    )
}

/// The plan-time lead-reconciliation answer schema.
///
/// Generated from [`crate::plan::ProposalResponse`], with two
/// answer-side adjustments the shared type cannot carry: `gate` is made
/// required (optional on the persisted envelope, mandatory on the
/// judgment answer — the collapsed `plan author` orchestration persists
/// it into `change.md` / `discovery.md`), and `kind` is pinned to the
/// `response` literal (the type's kind enum also covers the request
/// envelope).
///
/// # Panics
///
/// Panics when the generated schema drops the shape these adjustments
/// depend on — a compile-adjacent invariant, not a runtime input
/// condition.
#[must_use]
pub fn proposal() -> Value {
    let mut schema = root_schema::<crate::plan::ProposalResponse>(
        "proposal.schema.json",
        "Specify plan reconciliation answer",
        "Generated judgment-answer schema — generated from the Rust wire types by \
         project::answers; do not edit. Validates the schema-gated answer to the plan-time \
         lead-reconciliation judgment: the `kind: response` envelope the guest `plan author` \
         orchestration consumes. The deterministic tail re-checks the envelope kind and runs \
         the projection kernel.",
    );
    let required = schema
        .get_mut("required")
        .and_then(Value::as_array_mut)
        .expect("proposal answer schema carries a required array");
    assert!(!required.iter().any(|entry| entry == "gate"), "gate must be optional on the type");
    required.push(json!("gate"));
    let kind = schema
        .pointer_mut("/properties/kind")
        .expect("proposal answer schema carries a kind property");
    *kind = json!({ "const": "response" });
    schema
}

// Stamp a `pattern` constraint onto the string schema at `pointer`.
fn set_pattern(schema: &mut Value, pointer: &str, pattern: &str) {
    let target = schema
        .pointer_mut(pointer)
        .and_then(Value::as_object_mut)
        .expect("answer schema carries the patched property");
    target.insert("pattern".to_string(), json!(pattern));
}

/// Render an answer schema the way the committed golden files are
/// written: object keys sorted, pretty-printed, trailing newline.
///
/// The explicit sort keeps the byte shape independent of `serde_json`'s
/// `preserve_order` feature, which downstream dependencies may toggle.
///
/// # Panics
///
/// Panics when the value is not JSON-serialisable — impossible for the
/// generated schemas above.
#[must_use]
pub fn render(schema: &Value) -> String {
    let mut rendered =
        serde_json::to_string_pretty(&sort_keys(schema)).expect("answer schema serialises");
    rendered.push('\n');
    rendered
}

// Rebuild the value with object keys in sorted order, recursively.
fn sort_keys(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by_key(|(key, _)| *key);
            Value::Object(
                entries.into_iter().map(|(key, entry)| (key.clone(), sort_keys(entry))).collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(sort_keys).collect()),
        other => other.clone(),
    }
}

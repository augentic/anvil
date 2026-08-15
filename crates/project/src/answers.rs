//! Generated judgment answer schemas.
//!
//! Generated via `schemars` from the wire types that parse each answer;
//! deterministic tails stay authoritative for what schemas cannot express.

use schemars::JsonSchema;
use serde_json::{Value, json};

use crate::seam::wire::{
    BuildOutput, BuildStatus, PhaseOutcome, PhaseSource, PhaseWrite, UiSurface,
};
use crate::seam::{Evidence, Lead};

/// `$id` base for the generated answer documents.
const ANSWERS_ID_BASE: &str = "https://github.com/augentic/emery/answers";

/// Kebab slug grammar for lead ids and topic slugs, mirroring
/// `artifacts::evidence::is_kebab` in the deterministic tail.
const KEBAB_PATTERN: &str = "^[a-z0-9]+(-[a-z0-9]+)*$";

/// Dotted-kebab claim-id grammar, mirroring
/// `artifacts::evidence::claim::is_dotted_kebab` in the deterministic
/// tail.
const DOTTED_KEBAB_PATTERN: &str = "^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$";

/// The `survey` answer envelope: `{ "leads": [ ... ], "children": [ ... ] }`,
/// each item a [`Lead`] (the catalog lead minus the envelope `source` key).
#[derive(JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[expect(dead_code, reason = "fields exist only for schemars schema generation")]
struct LeadsAnswer {
    /// Top-level leads from an unfocused survey, in source order.
    #[serde(default)]
    leads: Vec<Lead>,
    /// Stable child leads under a focused parent.
    #[serde(default)]
    children: Vec<Lead>,
}

/// The `merge` answer: the report minus the envelope keys
/// (`version`, `slice`, `target`) the caller already knows and stamps
/// when widening onto the canonical report. The coverage claim rides
/// the build phase answer, never the merge return.
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

/// The `build` / `repair` / `verify` / `review` phase answer: the
/// RFC-90 phase report minus the adapter-attached `next-continuation`
/// (opaque session bytes never come from the model answer).
#[derive(JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[expect(dead_code, reason = "fields exist only for schemars schema generation")]
struct PhaseReportAnswer {
    /// Adapter-selected phase outcome.
    outcome: PhaseOutcome,
    /// Required report-level assurance claim.
    source: PhaseSource,
    /// Full structured diagnostics; default `[]`.
    #[serde(default)]
    findings: Vec<diagnostics::Diagnostic>,
    /// Candidate per-platform build outputs (`build` only); default `[]`.
    #[serde(default)]
    outputs: Vec<BuildOutput>,
    /// Optional UI-surface signal (`build` only).
    #[serde(default)]
    ui_surface: Option<UiSurface>,
    /// Slice-local requirement ids (`REQ-NNN`) the phase claims to
    /// have implemented (`build` only); default `[]`. Must never name
    /// a requirement from the build request's `deferred[]` exclusion
    /// set (RFC-86a D4).
    #[serde(default)]
    covered: Vec<String>,
    /// Audit-evidence writes performed by the phase; default `[]`.
    #[serde(default)]
    written: Vec<PhaseWrite>,
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
        "Emery survey answer",
        "Generated judgment-answer schema — generated from the Rust wire types by \
         project::answers; do not edit. Validates the schema-gated answer to a source \
         adapter's survey operation: an object carrying `leads[]` (unfocused) and \
         `children[]` (focused), each item the lead shape minus the envelope `source` \
         key (the caller attributes every lead to the surveyed source). Lead ids and \
         topic slugs carry the kebab-case grammar in-schema and are re-checked \
         deterministically after the gate, alongside the trim-aware synopsis check \
         the schema cannot express.",
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
        "Emery extract answer",
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

/// The `merge` answer schema.
#[must_use]
pub fn report() -> Value {
    root_schema::<ReportAnswer>(
        "report.schema.json",
        "Emery merge answer",
        "Generated judgment-answer schema — generated from the Rust wire types by \
         project::answers; do not edit. Validates the schema-gated answer to a target \
         adapter's merge operation: the report shape minus the envelope keys \
         (`version`, `slice`, `target`) the caller already knows.",
    )
}

/// The RFC-90 phase answer schema gating `build` / `repair` /
/// `verify` / `review` replies.
///
/// The generated shape drops `id` and `fingerprint` from the
/// diagnostic's required set: the engine renumbers report-local ids
/// and verifies-or-recomputes fingerprints on every accepted phase
/// report, so the model never has to mint either.
///
/// # Panics
///
/// Panics when the generated schema drops the patched Diagnostic
/// definition — a compile-adjacent invariant, not a runtime input
/// condition.
#[must_use]
pub fn phase_report() -> Value {
    let mut schema = root_schema::<PhaseReportAnswer>(
        "phase-report.schema.json",
        "Emery build-phase answer",
        "Generated judgment-answer schema — generated from the Rust wire types by \
         project::answers; do not edit. Validates the schema-gated answer to a target \
         adapter's build, repair, verify, or review operation (RFC-90): one typed phase \
         report. Blocking findings and dispatch errors determine failure — there is no \
         adapter-selected success/failure. Finding `id` and `fingerprint` are optional on \
         the answer; the engine renumbers and recomputes both.",
    );
    let required = schema
        .pointer_mut("/$defs/Diagnostic/required")
        .and_then(Value::as_array_mut)
        .expect("phase answer schema carries the Diagnostic required set");
    required.retain(|entry| entry != "id" && entry != "fingerprint");
    schema
}

/// The plan-time lead-reconciliation answer schema.
///
/// Generated from [`crate::plan::ProposalResponse`], with two
/// answer-side adjustments the shared type cannot carry: `gate` is made
/// required (optional on the persisted envelope, mandatory on the
/// judgment answer — the collapsed `plan author` orchestration persists
/// it into `change.md`), and `kind` is pinned to the
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
        "Emery plan reconciliation answer",
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

/// The publication-set record schema (RFC-95 D7/D8).
///
/// Not a judgment answer: plan-backed records are projected by
/// `emery plan archive`, and external producers author the same shape
/// against this document.
#[must_use]
pub fn publication() -> Value {
    root_schema::<crate::plan::publication::Record>(
        "publication.schema.json",
        "Emery publication-set record",
        "Generated wire schema — generated from the Rust wire types by \
         project::answers; do not edit. The RFC-95 publication-set record: members \
         with repository, branch, pull request, base, merge commit, publication \
         state, and derived order, plus the whole-set verification verdict and the \
         stable failing-member list. Plan-backed records are projected at \
         `emery plan archive`; external records validate against the same shape.",
    )
}

/// The decomposition partition answer schema (`split | leaf`).
///
/// Generated from [`crate::plan::PartitionResponse`], with `kind`
/// pinned to the `split` / `leaf` enum the type already carries.
#[must_use]
pub fn partition() -> Value {
    root_schema::<crate::plan::PartitionResponse>(
        "partition.schema.json",
        "Emery decomposition partition answer",
        "Generated judgment-answer schema — generated from the Rust wire types by \
         project::answers; do not edit. Validates the schema-gated `split | leaf` \
         partition of one open delivery domain. The deterministic tail applies the \
         answer to a tentative tree and runs Decomposition::check.",
    )
}

/// The bounded boundary-review answer schema.
///
/// Generated from [`crate::plan::BoundaryReview`].
#[must_use]
pub fn boundary_review() -> Value {
    root_schema::<crate::plan::BoundaryReview>(
        "boundary-review.schema.json",
        "Emery decomposition boundary-review answer",
        "Generated judgment-answer schema — generated from the Rust wire types by \
         project::answers; do not edit. Validates the schema-gated boundary review \
         after a provisional complexity score exceeds the slice-split threshold: \
         `close | focus | unready`.",
    )
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

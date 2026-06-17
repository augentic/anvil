//! `SliceMetadata` / `Outcome` serde round-trips, plus the top-level
//! `--help` axis-verb surface.

use crate::support::*;

#[test]
fn metadata_without_outcome_still_parses() {
    use specify_workflow::slice::SliceMetadata;
    // A freshly-created slice writes `metadata.yaml` with no `outcome`
    // key (omitted via `skip_serializing_if`) — byte-for-byte the
    // back-compat shape of metadata that predates the field. Drive
    // creation through `slice create` rather than hand-writing the file
    // (testing.md:45), then assert `SliceMetadata::load` leaves `outcome`
    // as None.
    let project = Project::init();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "create", "my-slice"])
        .assert()
        .success();
    let slice_dir = project.slices_dir().join("my-slice");
    let meta = SliceMetadata::load(&slice_dir).expect("freshly-created metadata parses");
    assert!(meta.outcome.is_none(), "metadata without an outcome field must load as None");
}

#[test]
fn phase_outcome_round_trips_serde() {
    use specify_workflow::slice::Outcome;
    // Construction via struct literal would require crossing the
    // `#[non_exhaustive]` boundary on `Outcome`; round-trip through
    // YAML instead so the wire shape is what's exercised.
    for kind in ["success", "failure", "deferred"] {
        for phase in ["shape", "build", "merge"] {
            let yaml = format!(
                "phase: {phase}\noutcome: {kind}\nat: \"2024-08-01T10:00:00Z\"\nsummary: some summary\n"
            );
            let parsed: Outcome = serde_saphyr::from_str(&yaml).expect("parse");
            let reserialised = serde_saphyr::to_string(&parsed).expect("serialize");
            let reparsed: Outcome = serde_saphyr::from_str(&reserialised).expect("reparse");
            assert_eq!(parsed, reparsed, "round-trip failed for yaml:\n{yaml}");
        }
    }
}

// ---- Top-level verb surface carries source/target axis verbs ----

#[test]
fn help_lists_axis_verbs() {
    // Inventory asserted via the contract dump, not clap help wording:
    // the axis verbs must be present and the retired `change` verb must
    // stay gone.
    let verbs = crate::common::contract_dump_verbs(&[]);
    for verb in ["slice", "source", "target"] {
        assert!(verbs.iter().any(|v| v == verb), "top level must declare `{verb}`: {verbs:?}");
    }
    let retired = "change";
    assert!(
        !verbs.iter().any(|v| v == retired),
        "retired verb `{retired}` must not resurface: {verbs:?}"
    );
}

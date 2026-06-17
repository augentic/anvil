use specify_model::evidence::ClaimKind;

use super::*;
use crate::journal::test_timestamp;

fn sample() -> ProvenanceIndex {
    ProvenanceIndex {
        version: 1,
        slice: "identity-user-registration".to_string(),
        generated_at: test_timestamp("2026-05-22T13:15:00Z"),
        generator: "specify@2.1.0".to_string(),
        requirements: vec![
            ProvenanceRequirement {
                id: "REQ-001".to_string(),
                status: RequirementStatus::Agreed,
                sources: vec!["identity-design-notes".to_string(), "runtime".to_string()],
                contributing_claims: vec![
                    ContributingClaim {
                        source: "identity-design-notes".to_string(),
                        id: "password-reset.request".to_string(),
                        kind: ClaimKind::Requirement,
                        value: None,
                        path: None,
                        winner: None,
                    },
                    ContributingClaim {
                        source: "runtime".to_string(),
                        id: "users.register.happy-path".to_string(),
                        kind: ClaimKind::Example,
                        value: None,
                        path: None,
                        winner: None,
                    },
                ],
                resolution: ProvenanceResolution::SingleValueAgreement,
                resolution_trace: None,
            },
            ProvenanceRequirement {
                id: "REQ-007".to_string(),
                status: RequirementStatus::Divergence,
                sources: vec!["identity-design-notes".to_string(), "legacy-monolith".to_string()],
                contributing_claims: vec![
                    ContributingClaim {
                        source: "identity-design-notes".to_string(),
                        id: "password-reset.expiry".to_string(),
                        kind: ClaimKind::Criterion,
                        value: Some("Reset links expire after 30 minutes.".to_string()),
                        path: Some("docs/account.md#L7".to_string()),
                        winner: Some(true),
                    },
                    ContributingClaim {
                        source: "legacy-monolith".to_string(),
                        id: "password-reset.expiry".to_string(),
                        kind: ClaimKind::Criterion,
                        value: Some("expiresAt = createdAt + 24h".to_string()),
                        path: Some("src/users/reset.ts#L42".to_string()),
                        winner: Some(false),
                    },
                ],
                resolution: ProvenanceResolution::PerSliceOverride,
                resolution_trace: Some(ResolutionTrace {
                    step: "per-slice-authority-override".to_string(),
                    r#override: Some(serde_json::json!({
                        "criterion": "identity-design-notes",
                    })),
                    winner: Some("identity-design-notes".to_string()),
                }),
            },
        ],
    }
}

#[test]
fn round_trips_through_yaml() {
    let original = sample();
    let yaml = serde_saphyr::to_string(&original).expect("serialise");
    assert!(yaml.contains("generated-at: 2026-05-22T13:15:00Z"));
    assert!(yaml.contains("contributing-claims:"));
    assert!(yaml.contains("resolution: per-slice-override"));
    let reparsed: ProvenanceIndex = serde_saphyr::from_str(&yaml).expect("reparse");
    assert_eq!(original, reparsed);
}

#[test]
fn validates_against_embedded_schema() {
    sample()
        .validate()
        .expect("sample provenance projection must validate against the embedded schema");
}

#[test]
fn resolution_round_trips_kebab_case() {
    for (variant, wire) in [
        (ProvenanceResolution::SingleSource, "single-source"),
        (ProvenanceResolution::SingleValueAgreement, "single-value-agreement"),
        (ProvenanceResolution::AuthorityResolved, "authority-resolved"),
        (ProvenanceResolution::PerSliceOverride, "per-slice-override"),
        (ProvenanceResolution::UnknownNoEvidence, "unknown-no-evidence"),
        (ProvenanceResolution::TiedConflict, "tied-conflict"),
    ] {
        assert_eq!(serde_json::to_string(&variant).expect("serialise"), format!("\"{wire}\""));
    }
}

#[test]
fn rejects_unknown_top_level_fields() {
    let yaml = r"version: 1
slice: x
generated-at: 2026-05-22T13:15:00Z
generator: specify@2.1.0
requirements: []
rogue: true
";
    let err = serde_saphyr::from_str::<ProvenanceIndex>(yaml)
        .expect_err("deny_unknown_fields must reject rogue");
    assert!(err.to_string().contains("rogue"), "expected error to name rogue, got: {err}");
}

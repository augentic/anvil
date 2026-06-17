use super::*;

fn claim(source: &str, id: &str, kind: ClaimKind) -> ClaimRef {
    ClaimRef {
        source: source.to_string(),
        id: id.to_string(),
        kind,
    }
}

fn authority(pairs: &[(&str, AuthorityClass)]) -> BTreeMap<String, AuthorityClass> {
    pairs.iter().map(|(source, class)| ((*source).to_string(), *class)).collect()
}

fn overrides(pairs: &[(ClaimKind, &str)]) -> BTreeMap<ClaimKind, String> {
    pairs.iter().map(|(kind, source)| (*kind, (*source).to_string())).collect()
}

// -- Status derivation table -------

#[test]
fn zero_claims_are_unknown_no_evidence() {
    let resolved = resolve(&[], &authority(&[]), &overrides(&[]), None);
    assert_eq!(resolved.status, RequirementStatus::Unknown);
    assert_eq!(resolved.resolution, ProvenanceResolution::UnknownNoEvidence);
    assert!(resolved.winners.is_empty());
}

#[test]
fn one_claim_is_agreed_single_source() {
    let claims = [claim("docs", "reset.request", ClaimKind::Requirement)];
    let resolved = resolve(
        &claims,
        &authority(&[("docs", AuthorityClass::Documentation)]),
        &overrides(&[]),
        None,
    );
    assert_eq!(resolved.status, RequirementStatus::Agreed);
    assert_eq!(resolved.resolution, ProvenanceResolution::SingleSource);
    assert_eq!(resolved.winners, vec![None]);
}

#[test]
fn multi_agreed_is_single_value_agreement() {
    let claims = [
        claim("docs", "reset.request", ClaimKind::Requirement),
        claim("legacy", "users.reset.request", ClaimKind::Example),
    ];
    let resolved = resolve(
        &claims,
        &authority(&[
            ("docs", AuthorityClass::Documentation),
            ("legacy", AuthorityClass::Behaviour),
        ]),
        &overrides(&[]),
        Some(Agreement::Agreed),
    );
    assert_eq!(resolved.status, RequirementStatus::Agreed);
    assert_eq!(resolved.resolution, ProvenanceResolution::SingleValueAgreement);
    assert_eq!(resolved.winners, vec![None, None]);
}

#[test]
fn multi_disagreed_unique_top_is_divergence() {
    let claims = [
        claim("docs", "reset.expiry", ClaimKind::Criterion),
        claim("legacy", "reset.expiry", ClaimKind::Example),
    ];
    let resolved = resolve(
        &claims,
        &authority(&[
            ("docs", AuthorityClass::Documentation),
            ("legacy", AuthorityClass::Behaviour),
        ]),
        &overrides(&[]),
        Some(Agreement::Disagreed),
    );
    assert_eq!(resolved.status, RequirementStatus::Divergence);
    assert_eq!(resolved.resolution, ProvenanceResolution::AuthorityResolved);
    assert_eq!(resolved.winners, vec![Some(true), Some(false)]);
}

#[test]
fn multi_disagreed_tied_top_is_conflict() {
    let claims = [
        claim("docs-a", "reset.expiry", ClaimKind::Criterion),
        claim("docs-b", "reset.expiry", ClaimKind::Criterion),
    ];
    let resolved = resolve(
        &claims,
        &authority(&[
            ("docs-a", AuthorityClass::Documentation),
            ("docs-b", AuthorityClass::Documentation),
        ]),
        &overrides(&[]),
        Some(Agreement::Disagreed),
    );
    assert_eq!(resolved.status, RequirementStatus::Conflict);
    assert_eq!(resolved.resolution, ProvenanceResolution::TiedConflict);
    assert_eq!(resolved.winners, vec![None, None]);
}

// -- Resolution order -----------

#[test]
fn step_1_per_slice_override_wins() {
    // `runtime` is the lowest class but the override forces it to
    // win the `example` kind outright.
    let claims = [
        claim("docs", "reset.expiry", ClaimKind::Criterion),
        claim("runtime", "reset.expiry", ClaimKind::Example),
    ];
    let resolved = resolve(
        &claims,
        &authority(&[
            ("docs", AuthorityClass::Documentation),
            ("runtime", AuthorityClass::Behaviour),
        ]),
        &overrides(&[(ClaimKind::Example, "runtime")]),
        Some(Agreement::Disagreed),
    );
    assert_eq!(resolved.status, RequirementStatus::Divergence);
    assert_eq!(resolved.resolution, ProvenanceResolution::PerSliceOverride);
    assert_eq!(resolved.winners, vec![Some(false), Some(true)]);
}

#[test]
fn step_2_document_authority_wins() {
    let claims = [
        claim("docs", "reset.expiry", ClaimKind::Criterion),
        claim("runtime", "reset.expiry", ClaimKind::Example),
    ];
    let resolved = resolve(
        &claims,
        &authority(&[
            ("docs", AuthorityClass::Documentation),
            ("runtime", AuthorityClass::Behaviour),
        ]),
        &overrides(&[]),
        Some(Agreement::Disagreed),
    );
    assert_eq!(resolved.status, RequirementStatus::Divergence);
    assert_eq!(resolved.resolution, ProvenanceResolution::AuthorityResolved);
    assert_eq!(resolved.winners, vec![Some(true), Some(false)]);
}

#[test]
fn step_3_default_ordering_breaks_tie() {
    // `intent > documentation` decides when no override fires.
    let claims = [
        claim("brief", "reset.expiry", ClaimKind::Intent),
        claim("docs", "reset.expiry", ClaimKind::Criterion),
    ];
    let resolved = resolve(
        &claims,
        &authority(&[("brief", AuthorityClass::Intent), ("docs", AuthorityClass::Documentation)]),
        &overrides(&[]),
        Some(Agreement::Disagreed),
    );
    assert_eq!(resolved.status, RequirementStatus::Divergence);
    assert_eq!(resolved.resolution, ProvenanceResolution::AuthorityResolved);
    assert_eq!(resolved.winners, vec![Some(true), Some(false)]);
}

#[test]
fn resolution_order_step_4_tie_is_conflict() {
    let claims = [
        claim("docs-a", "reset.expiry", ClaimKind::Criterion),
        claim("docs-b", "reset.expiry", ClaimKind::Criterion),
    ];
    let resolved = resolve(
        &claims,
        &authority(&[
            ("docs-a", AuthorityClass::Documentation),
            ("docs-b", AuthorityClass::Documentation),
        ]),
        &overrides(&[]),
        Some(Agreement::Disagreed),
    );
    assert_eq!(resolved.status, RequirementStatus::Conflict);
    assert_eq!(resolved.resolution, ProvenanceResolution::TiedConflict);
}

// -- Mixed-kind requirements ----

#[test]
fn mixed_kinds_picks_winner() {
    // A `criterion` (documentation) outranks an `example`
    // (behaviour) by the default ordering, no override.
    let claims = [
        claim("docs", "reset.expiry", ClaimKind::Criterion),
        claim("legacy", "reset.expiry", ClaimKind::Example),
    ];
    let resolved = resolve(
        &claims,
        &authority(&[
            ("docs", AuthorityClass::Documentation),
            ("legacy", AuthorityClass::Behaviour),
        ]),
        &overrides(&[]),
        Some(Agreement::Disagreed),
    );
    assert_eq!(resolved.status, RequirementStatus::Divergence);
    assert_eq!(resolved.resolution, ProvenanceResolution::AuthorityResolved);
    assert_eq!(resolved.winners, vec![Some(true), Some(false)]);
}

#[test]
fn mixed_kinds_override_flips_winner() {
    // The default ordering would pick `docs`, but an override on the
    // `example` kind forces `legacy` to win.
    let claims = [
        claim("docs", "reset.expiry", ClaimKind::Criterion),
        claim("legacy", "reset.expiry", ClaimKind::Example),
    ];
    let resolved = resolve(
        &claims,
        &authority(&[
            ("docs", AuthorityClass::Documentation),
            ("legacy", AuthorityClass::Behaviour),
        ]),
        &overrides(&[(ClaimKind::Example, "legacy")]),
        Some(Agreement::Disagreed),
    );
    assert_eq!(resolved.status, RequirementStatus::Divergence);
    assert_eq!(resolved.resolution, ProvenanceResolution::PerSliceOverride);
    assert_eq!(resolved.winners, vec![Some(false), Some(true)]);
}

#[test]
fn override_for_absent_source_does_not_fire() {
    // The override names `docs` for `criterion`, but no `docs`
    // criterion claim exists, so it falls through to authority.
    let claims = [
        claim("brief", "reset.expiry", ClaimKind::Intent),
        claim("legacy", "reset.expiry", ClaimKind::Example),
    ];
    let resolved = resolve(
        &claims,
        &authority(&[("brief", AuthorityClass::Intent), ("legacy", AuthorityClass::Behaviour)]),
        &overrides(&[(ClaimKind::Criterion, "docs")]),
        Some(Agreement::Disagreed),
    );
    assert_eq!(resolved.status, RequirementStatus::Divergence);
    assert_eq!(resolved.resolution, ProvenanceResolution::AuthorityResolved);
    assert_eq!(resolved.winners, vec![Some(true), Some(false)]);
}

#[test]
fn override_only_fires_for_matching_kind() {
    // The override is keyed on `criterion`; `docs`'s `excerpt` claim
    // is not promoted, so behaviour-class authority is unchanged.
    let claims = [
        claim("docs", "reset.expiry", ClaimKind::Excerpt),
        claim("brief", "reset.expiry", ClaimKind::Intent),
    ];
    let resolved = resolve(
        &claims,
        &authority(&[("docs", AuthorityClass::Documentation), ("brief", AuthorityClass::Intent)]),
        &overrides(&[(ClaimKind::Criterion, "docs")]),
        Some(Agreement::Disagreed),
    );
    assert_eq!(resolved.status, RequirementStatus::Divergence);
    assert_eq!(resolved.resolution, ProvenanceResolution::AuthorityResolved);
    // `brief` (intent) wins; the override never fired.
    assert_eq!(resolved.winners, vec![Some(false), Some(true)]);
}

#[test]
fn agreement_round_trips_kebab_case() {
    for (variant, wire) in [(Agreement::Agreed, "agreed"), (Agreement::Disagreed, "disagreed")] {
        let json = serde_json::to_string(&variant).expect("serialise");
        assert_eq!(json, format!("\"{wire}\""));
        let reparsed: Agreement = serde_json::from_str(&json).expect("reparse");
        assert_eq!(variant, reparsed);
    }
}

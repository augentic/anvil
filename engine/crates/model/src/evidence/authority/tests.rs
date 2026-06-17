use super::*;

#[test]
fn authority_class_round_trips_kebab_case() {
    for (variant, wire) in [
        (AuthorityClass::Intent, "intent"),
        (AuthorityClass::Documentation, "documentation"),
        (AuthorityClass::Behaviour, "behaviour"),
    ] {
        let json = serde_json::to_string(&variant).expect("serialise");
        assert_eq!(json, format!("\"{wire}\""));
        let reparsed: AuthorityClass = serde_json::from_str(&json).expect("reparse");
        assert_eq!(variant, reparsed);
    }
}

#[test]
fn claim_kind_round_trips_kebab_case() {
    let json = serde_json::to_string(&ClaimKind::Example).expect("serialise");
    assert_eq!(json, "\"example\"");
    let reparsed: ClaimKind = serde_json::from_str(&json).expect("reparse");
    assert_eq!(reparsed, ClaimKind::Example);
}

#[test]
fn claim_kind_from_str_round_trips() {
    for variant in [
        ClaimKind::Intent,
        ClaimKind::Requirement,
        ClaimKind::Criterion,
        ClaimKind::Decision,
        ClaimKind::Section,
        ClaimKind::Diagram,
        ClaimKind::Contract,
        ClaimKind::Example,
        ClaimKind::Excerpt,
        ClaimKind::Type,
        ClaimKind::Call,
        ClaimKind::Region,
        ClaimKind::Container,
        ClaimKind::Leaf,
    ] {
        let wire = variant.to_string();
        let parsed: ClaimKind = wire.parse().expect("round-trip");
        assert_eq!(parsed, variant, "ClaimKind round-trip failed for {wire}");
    }
}

#[test]
fn claim_kind_from_str_rejects_unknown() {
    let err = "bogus".parse::<ClaimKind>().expect_err("must reject unknown");
    assert!(err.contains("bogus"), "error must mention input, got: {err}");
}

//! Authority-class wire and resolution matrix (`artifacts::evidence::authority`).

use artifacts::evidence::authority::*;

#[test]
fn enum_wire_matrix() {
    for (variant, wire) in [
        (AuthorityClass::Intent, "intent"),
        (AuthorityClass::Documentation, "documentation"),
        (AuthorityClass::Behaviour, "behaviour"),
    ] {
        let json = serde_json::to_string(&variant).expect("serialise");
        assert_eq!(json, format!("\"{wire}\""));
        let reparsed: AuthorityClass = serde_json::from_str(&json).expect("reparse");
        assert_eq!(variant, reparsed);
        assert_eq!(variant.to_string(), wire);
    }

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
        let json = serde_json::to_string(&variant).expect("serialise");
        assert_eq!(json, format!("\"{wire}\""));
        assert_eq!(serde_json::from_str::<ClaimKind>(&json).expect("serde reparse"), variant);
        let parsed: ClaimKind = wire.parse().expect("round-trip");
        assert_eq!(parsed, variant, "ClaimKind round-trip failed for {wire}");
    }
}

#[test]
fn claim_kind_rejects_unknown() {
    let err = "bogus".parse::<ClaimKind>().expect_err("must reject unknown");
    assert!(err.contains("bogus"), "error must mention input, got: {err}");
}

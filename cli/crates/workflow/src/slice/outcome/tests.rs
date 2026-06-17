use super::*;

const ALL: [Kind; 3] = [Kind::Success, Kind::Failure, Kind::Deferred];

#[test]
fn display_matches_serde() {
    // The strum `Display` and the serde wire scalar must agree for every
    // variant — both are the kebab-case lowercase name.
    for (kind, wire) in
        [(Kind::Success, "success"), (Kind::Failure, "failure"), (Kind::Deferred, "deferred")]
    {
        assert_eq!(kind.to_string(), wire);
        assert_eq!(serde_json::to_string(&kind).expect("serialise"), format!("\"{wire}\""));
    }
}

#[test]
fn round_trips_through_json() {
    for kind in ALL {
        let json = serde_json::to_string(&kind).expect("serialise");
        let back: Kind = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, kind, "{kind} must survive a JSON round trip");
    }
}

#[test]
fn round_trips_through_yaml() {
    // `metadata.yaml` is the on-disk home; confirm the saphyr path
    // matches the JSON path so a serializer swap can't drift the wire.
    for kind in ALL {
        let yaml = serde_saphyr::to_string(&kind).expect("serialise yaml");
        let back: Kind = serde_saphyr::from_str(&yaml).expect("deserialise yaml");
        assert_eq!(back, kind);
    }
}

#[test]
fn unknown_variant_errors() {
    // An unknown outcome (e.g. a `skipped` value outside the closed
    // set) must fail the closed enum rather than silently default.
    serde_json::from_str::<Kind>("\"skipped\"")
        .expect_err("unknown outcome kind must not deserialise");
}

#[test]
fn rejects_pascal_case_on_wire() {
    // Guard the kebab-case contract: the `Pascal` spelling is not an
    // accepted alias.
    serde_json::from_str::<Kind>("\"Success\"").expect_err("PascalCase is not a valid wire form");
}

use super::{PlanName, SliceName};

#[test]
fn slice_name_serialises_transparently() {
    let name = SliceName::new("user-registration");
    let json = serde_json::to_string(&name).expect("serialise");
    assert_eq!(json, "\"user-registration\"");
    let round: SliceName = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(round, name);
}

#[test]
fn plan_name_deserialises_from_bare_string() {
    let name: PlanName = serde_json::from_str("\"identity-rollout\"").expect("deserialise");
    assert_eq!(name.as_str(), "identity-rollout");
}

#[test]
fn deref_exposes_str_api() {
    let name = SliceName::new("fix-typo");
    assert!(name.starts_with("fix"));
    assert_eq!(name.len(), 8);
}

#[test]
fn equality_against_bare_strings() {
    let name = PlanName::new("rollout");
    assert_eq!(name, "rollout");
    assert_eq!(name, String::from("rollout"));
}

#[test]
fn borrow_enables_str_keyed_lookup() {
    use std::collections::HashMap;

    let mut map: HashMap<SliceName, u8> = HashMap::new();
    map.insert(SliceName::new("alpha"), 1);
    assert_eq!(map.get("alpha"), Some(&1));
}

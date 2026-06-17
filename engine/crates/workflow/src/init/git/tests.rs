use super::*;

#[test]
fn sparse_checkout_uses_adapter_parent() {
    assert_eq!(sparse_checkout_path("adapters/omnia"), "adapters");
    assert_eq!(sparse_checkout_path("schemas/omnia"), "schemas");
    assert_eq!(sparse_checkout_path("omnia"), "omnia");
}

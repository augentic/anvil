//! Adapter identity tests.

use std::str::FromStr;

use emery_source::{AdapterIdentity, IdentityError};

#[test]
fn from_str_name_version() {
    let identity = AdapterIdentity::from_str("source@0.1.0").expect("valid identity");
    assert_eq!(identity.name, "source");
    assert_eq!(identity.version, "0.1.0");
}

#[test]
fn rejects_malformed() {
    for value in ["source", "source@", "@0.1.0", "source@0.1.0@beta", ""] {
        assert_eq!(AdapterIdentity::from_str(value), Err(IdentityError), "{value}");
    }
}

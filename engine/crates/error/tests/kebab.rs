//! Kebab-case validators: is_kebab and is_kebab_leading_alpha.

use specify_error::{is_kebab, is_kebab_leading_alpha};

#[test]
fn is_kebab_accepts_and_rejects() {
    for ok in ["a", "abc", "alpha-gateway", "x-1", "a1-b2"] {
        assert!(is_kebab(ok), "expected `{ok}` to pass");
    }
    for bad in ["", "-a", "a-", "a--b", "A", "alpha_gateway", "alpha gateway"] {
        assert!(!is_kebab(bad), "expected `{bad}` to fail");
    }
}

#[test]
fn is_kebab_leading_alpha_rejects() {
    for ok in ["a", "tab-bar", "x-1"] {
        assert!(is_kebab_leading_alpha(ok), "expected `{ok}` to pass");
    }
    for bad in ["", "1a", "9-lives", "-a", "a--b", "A"] {
        assert!(!is_kebab_leading_alpha(bad), "expected `{bad}` to fail");
    }
}

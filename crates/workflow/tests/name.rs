//! Identifier grammar tests.

use workflow::name::{is_kebab, is_kebab_leading_alpha};

#[test]
fn kebab_grammar() {
    for valid in ["a", "abc", "alpha-gateway", "x-1", "a1-b2"] {
        assert!(is_kebab(valid), "expected `{valid}` to pass");
    }
    for invalid in ["", "-a", "a-", "a--b", "A", "alpha_gateway", "alpha gateway"] {
        assert!(!is_kebab(invalid), "expected `{invalid}` to fail");
    }

    for valid in ["a", "tab-bar", "x-1"] {
        assert!(is_kebab_leading_alpha(valid), "expected `{valid}` to pass");
    }
    for invalid in ["", "1a", "9-lives", "-a", "a--b", "A"] {
        assert!(!is_kebab_leading_alpha(invalid), "expected `{invalid}` to fail");
    }
}

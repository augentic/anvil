//! The WIRE_CODES table: sorted, unique, kebab-case.

use specify_error::codes::WIRE_CODES;

#[test]
fn sorted_unique_kebab() {
    for window in WIRE_CODES.windows(2) {
        assert!(
            window[0] < window[1],
            "WIRE_CODES must stay sorted and deduplicated: `{}` >= `{}`",
            window[0],
            window[1]
        );
    }
    for code in WIRE_CODES {
        assert!(specify_error::is_kebab(code), "wire code `{code}` must be kebab-case");
    }
}

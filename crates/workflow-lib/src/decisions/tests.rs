use super::*;

// The `promote` kernel's behaviour (id assignment, supersede flips, the
// orphan abort) lives in `crates/workflow/tests/decisions.rs`, which
// drives the public entry point over a real tree. Only the private
// `dec_number` / `is_dec_ref` parser stays here — no public input reaches it.

#[test]
fn dec_number_parses() {
    assert_eq!(dec_number("DEC-0007"), Some(7));
    assert_eq!(dec_number("DEC-12"), Some(12));
    assert_eq!(dec_number("DEC-"), None);
    assert_eq!(dec_number("REQ-001"), None);
    assert!(is_dec_ref("DEC-0001"));
    assert!(!is_dec_ref("some-slug"));
}

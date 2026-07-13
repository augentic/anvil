//! The fixture core's request-validation branch: an extract for a
//! lead the source never surveyed is a typed `invalid-request` — the
//! one seam failure the workflow orchestrations can never produce
//! (they only extract surveyed leads), so it is proven here at the
//! crate boundary.

use fixtures::{Error, Lead, extract};

#[test]
fn unknown_lead_invalid() {
    let lead = Lead {
        lead: "never-surveyed".to_string(),
        synopsis: "A lead this source does not know.".to_string(),
        topics: Vec::new(),
    };
    let err = extract("source:fixture", &lead).expect_err("unknown lead refused");
    assert!(matches!(err, Error::InvalidRequest(_)), "{err:?}");
    assert!(err.to_string().contains("never-surveyed"), "{err}");
}

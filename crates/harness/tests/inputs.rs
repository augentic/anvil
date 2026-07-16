//! Strict-subset refusals of the shared `trial.env` parser.
//! Checked-in-definition and shell parity live with each wrapper's
//! own committed data.

use harness::inputs::TrialInputs;

const VALID: &str = "TRIAL_PROJECT_NAME=\"orders\"\nTRIAL_CHANGE=\"orders\"\n\
                     TRIAL_SOURCE=\"docs=documentation:docs\"\nTRIAL_INTENT=\"do it\"\n";

#[test]
fn valid_parses() {
    TrialInputs::parse(VALID).expect("documented shape parses");
}

#[test]
fn unquoted_value() {
    let err =
        TrialInputs::parse(&VALID.replace("\"orders\"\nTRIAL_CHANGE", "orders\nTRIAL_CHANGE"))
            .expect_err("unquoted values refuse");
    assert!(format!("{err:#}").contains("double-quoted"), "{err:#}");
}

#[test]
fn expansion_refused() {
    let err = TrialInputs::parse(&VALID.replace("do it", "do $HOME"))
        .expect_err("shell expansion characters refuse");
    assert!(format!("{err:#}").contains("shell would expand"), "{err:#}");
}

#[test]
fn missing_key() {
    let body = VALID.replace("TRIAL_INTENT=\"do it\"\n", "");
    let err = TrialInputs::parse(&body).expect_err("a missing key refuses");
    assert!(format!("{err:#}").contains("TRIAL_INTENT"), "{err:#}");
}

#[test]
fn unknown_key() {
    let body = format!("{VALID}TRIAL_SURPRISE=\"x\"\n");
    let err = TrialInputs::parse(&body).expect_err("an unknown key refuses");
    assert!(format!("{err:#}").contains("TRIAL_SURPRISE"), "{err:#}");
}

#[test]
fn duplicate_key() {
    let body = format!("{VALID}TRIAL_CHANGE=\"again\"\n");
    let err = TrialInputs::parse(&body).expect_err("a duplicate key refuses");
    assert!(format!("{err:#}").contains("duplicate"), "{err:#}");
}

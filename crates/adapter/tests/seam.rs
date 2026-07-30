//! The seam vocabulary's own behavior: severity blocking classes, input
//! prompt labels, and the model-to-seam error mapping.

use adapter::Error as ModelError;
use adapter::seam::{Error, Input, Payload, Severity};

#[test]
fn blocking_severities() {
    assert!(Severity::Critical.blocking());
    assert!(Severity::Important.blocking());
    assert!(!Severity::Suggestion.blocking());
    assert!(!Severity::Optional.blocking());
}

#[test]
fn input_labels() {
    let payload = |path: &str| Payload::Path(path.to_string());
    let inputs = [
        (Input::Proposal(payload("p")), "proposal"),
        (Input::Design(payload("d")), "design"),
        (Input::Tasks(payload("t")), "tasks"),
        (Input::Spec(payload("s")), "spec"),
        (Input::Other(payload("o")), "other"),
    ];
    for (input, label) in &inputs {
        assert_eq!(input.label(), *label);
        assert_eq!(input.path(), Some(&label[..1]), "path survives the label projection");
        assert_eq!(input.body(), None, "lent deployments carry no inlined body");
    }
}

#[test]
fn error_mapping() {
    assert_eq!(
        Error::from(ModelError::InvalidRequest("empty".to_string())),
        Error::InvalidRequest("empty".to_string())
    );
    assert_eq!(
        Error::from(ModelError::BudgetExhausted("iterations".to_string())),
        Error::Internal("budget exhausted: iterations".to_string())
    );
}

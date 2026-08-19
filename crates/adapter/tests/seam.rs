//! The seam vocabulary's own behavior: the model-to-seam error mapping.

use emery_adapter::Error as ModelError;
use emery_adapter::seam::Error;

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

//! Contract error mapping
//!
//! How a model failure becomes an adapter error: a malformed request stays a
//! request error, and everything else becomes an internal failure. The engine
//! branches on this distinction, so it is pinned at the contract.

use emery_source::types::Error;
use omnia_guest::model::Error as ModelError;

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

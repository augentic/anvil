use std::path::PathBuf;

/// A single validation finding from a check predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule_id: &'static str,
    pub message: String,
    pub location: Option<Location>,
}

/// File location for a finding (1-based line, optional column).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub path: PathBuf,
    pub line: usize,
    pub column: Option<usize>,
}

/// A check predicate that scans the framework repo and returns findings.
pub trait Check {
    fn run(&self, ctx: &crate::context::Context) -> Vec<Finding>;
}

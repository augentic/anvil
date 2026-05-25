/// Exit codes aligned with `specify-cli` (`src/output.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Exit {
    Success,
    GenericFailure,
    ValidationFailed,
}

impl Exit {
    pub const fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::GenericFailure => 1,
            Self::ValidationFailed => 2,
        }
    }
}

/// Map findings and infrastructure errors to process exit codes.
pub fn exit_from_result(result: Result<(), crate::error::ToolingError>, findings: usize) -> Exit {
    match result {
        Ok(()) if findings == 0 => Exit::Success,
        Ok(()) => Exit::ValidationFailed,
        Err(crate::error::ToolingError::Validation(_)) => Exit::ValidationFailed,
        Err(crate::error::ToolingError::Infrastructure(_)) => Exit::GenericFailure,
    }
}

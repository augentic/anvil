use std::fmt;

/// Infrastructure or validation failures surfaced by the tooling binary.
#[derive(Debug)]
pub enum ToolingError {
    Infrastructure(String),
    Validation(String),
}

impl fmt::Display for ToolingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Infrastructure(message) => write!(f, "{message}"),
            Self::Validation(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ToolingError {}

impl From<std::io::Error> for ToolingError {
    fn from(source: std::io::Error) -> Self {
        Self::Infrastructure(source.to_string())
    }
}

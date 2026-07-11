//! Shallow root-marker detection for generated context guidance.
//! Public surface: [`Detection`] (the per-language summary folded into
//! AGENTS.md) plus the [`detect_root_markers`] orchestrator.

mod markers;
mod runtimes;

pub use runtimes::detect_root_markers;

const NOT_DETECTED: &str = "not detected";

/// Everything the renderer needs from root-marker detection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Detection {
    /// Detected language runtimes, sorted by label.
    pub runtimes: Vec<RuntimeDetection>,
    /// Detected test commands.
    pub tests: Vec<CommandDetection>,
    /// Detected lint commands and CI workflows.
    pub linting: Vec<LintDetection>,
    /// Unreadable or malformed marker files, sorted by path.
    pub warnings: Vec<DetectionWarning>,
    /// Repo-relative marker paths that fed the detection, for fingerprinting.
    pub input_paths: Vec<String>,
}

impl Detection {
    /// `## Runtime` bullet lines, or the "not detected" placeholder.
    #[must_use]
    pub fn runtime_bullets(&self) -> Vec<String> {
        if self.runtimes.is_empty() {
            return vec![NOT_DETECTED.to_string()];
        }
        self.runtimes.iter().map(RuntimeDetection::bullet).collect()
    }

    /// `## Tests` bullet lines, or the "not detected" placeholder.
    #[must_use]
    pub fn test_bullets(&self) -> Vec<String> {
        if self.tests.is_empty() {
            return vec![NOT_DETECTED.to_string()];
        }
        self.tests.iter().map(CommandDetection::bullet).collect()
    }

    /// `## Linting` bullet lines, or the "not detected" placeholder.
    #[must_use]
    pub fn lint_bullets(&self) -> Vec<String> {
        if self.linting.is_empty() {
            return vec![NOT_DETECTED.to_string()];
        }
        self.linting.iter().map(LintDetection::bullet).collect()
    }
}

/// A marker file that existed but could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectionWarning {
    /// Repo-relative path of the offending marker.
    pub path: String,
    /// Human-readable parse failure.
    pub message: String,
}

/// One detected language runtime (`Rust`, `Go 1.22`, …).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDetection {
    id: &'static str,
    label: String,
}

impl RuntimeDetection {
    const fn new(id: &'static str, label: String) -> Self {
        Self { id, label }
    }

    fn bullet(&self) -> String {
        format!("detected: {}.", self.label)
    }
}

/// One detected tool invocation (`cargo test`, `npm test`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandDetection {
    id: &'static str,
    command: &'static str,
}

impl CommandDetection {
    const fn new(id: &'static str, command: &'static str) -> Self {
        Self { id, command }
    }

    fn bullet(&self) -> String {
        format!("detected: `{}`.", self.command)
    }
}

/// One detected linting surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintDetection {
    /// A lint tool invocation (`cargo clippy`, `eslint`, …).
    Command(CommandDetection),
    /// A GitHub Actions workflow file, by name.
    Workflow(String),
}

impl LintDetection {
    const fn id(&self) -> &str {
        match self {
            Self::Command(command) => command.id,
            Self::Workflow(_name) => "github-actions",
        }
    }

    fn bullet(&self) -> String {
        match self {
            Self::Command(command) => command.bullet(),
            Self::Workflow(name) => format!("detected: GitHub Actions workflow `{name}`."),
        }
    }
}

// Detection ordering and the corrupt-marker warning path are exercised
// through the public API in `crates/workflow/tests/agents_detect.rs`; the
// private per-marker grammars keep their unit matrices in
// `detect/markers.rs`.

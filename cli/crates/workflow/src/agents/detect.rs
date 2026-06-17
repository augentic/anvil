//! Shallow root-marker detection for generated context guidance.
//! Public surface: [`Detection`] (the per-language summary folded into
//! AGENTS.md) plus the [`detect_root_markers`] orchestrator.

mod markers;
mod runtimes;

pub use runtimes::detect_root_markers;

const NOT_DETECTED: &str = "not detected";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Detection {
    pub runtimes: Vec<RuntimeDetection>,
    pub tests: Vec<CommandDetection>,
    pub linting: Vec<LintDetection>,
    pub warnings: Vec<DetectionWarning>,
    pub input_paths: Vec<String>,
}

impl Detection {
    pub fn runtime_bullets(&self) -> Vec<String> {
        if self.runtimes.is_empty() {
            return vec![NOT_DETECTED.to_string()];
        }
        self.runtimes.iter().map(RuntimeDetection::bullet).collect()
    }

    pub fn test_bullets(&self) -> Vec<String> {
        if self.tests.is_empty() {
            return vec![NOT_DETECTED.to_string()];
        }
        self.tests.iter().map(CommandDetection::bullet).collect()
    }

    pub fn lint_bullets(&self) -> Vec<String> {
        if self.linting.is_empty() {
            return vec![NOT_DETECTED.to_string()];
        }
        self.linting.iter().map(LintDetection::bullet).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectionWarning {
    pub path: String,
    pub message: String,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintDetection {
    Command(CommandDetection),
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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn mixed_runtime_order_is_deterministic() {
        let tmp = tempdir().expect("tempdir");
        fs::write(tmp.path().join("package.json"), r#"{"engines":{"node":">=20"}}"#)
            .expect("package");
        fs::write(tmp.path().join("go.mod"), "module demo\n\ngo 1.22\n").expect("go");
        fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"demo\"\n").expect("cargo");

        let detection = detect_root_markers(tmp.path());

        let labels: Vec<String> =
            detection.runtimes.iter().map(|runtime| runtime.label.clone()).collect();
        assert_eq!(
            labels,
            vec![
                "Go 1.22".to_string(),
                "Node.js (engines.node `>=20`)".to_string(),
                "Rust".to_string(),
            ]
        );
    }

    #[test]
    fn corrupt_markers_warn_not_detect() {
        let tmp = tempdir().expect("tempdir");
        fs::write(tmp.path().join("Cargo.toml"), "package = [").expect("cargo");
        fs::write(tmp.path().join("package.json"), "{").expect("package");
        fs::create_dir_all(tmp.path().join(".github/workflows")).expect("workflows dir");
        fs::write(tmp.path().join(".github/workflows/ci.yaml"), "name: [").expect("workflow");

        let detection = detect_root_markers(tmp.path());

        assert!(detection.runtimes.is_empty());
        assert_eq!(
            detection.warnings.iter().map(|warning| warning.path.as_str()).collect::<Vec<_>>(),
            vec![".github/workflows/ci.yaml", "Cargo.toml", "package.json"]
        );
    }
}

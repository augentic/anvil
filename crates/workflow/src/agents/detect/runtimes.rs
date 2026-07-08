//! Per-language detection passes assembled into a [`Detection`]. Each
//! `detect_*` method on [`Detector`] inspects one root marker;
//! [`detect_root_markers`] orchestrates the passes with byte-stable output.

use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use serde_json::Value;

use super::markers::{
    TomlMarker, parse_go_version, parse_make_targets, relative_marker_path, strip_json_comments,
};
use super::{CommandDetection, Detection, DetectionWarning, LintDetection, RuntimeDetection};

pub fn detect_root_markers(project_dir: &Path) -> Detection {
    let mut detector = Detector::new(project_dir);
    detector.detect_rust();
    detector.detect_node();
    detector.detect_python();
    detector.detect_go();
    detector.detect_deno();
    detector.detect_make();
    detector.detect_github_actions();
    detector.sort();
    detector.detection
}

struct Detector<'a> {
    project_dir: &'a Path,
    detection: Detection,
}

impl<'a> Detector<'a> {
    const fn new(project_dir: &'a Path) -> Self {
        Self {
            project_dir,
            detection: Detection {
                runtimes: Vec::new(),
                tests: Vec::new(),
                linting: Vec::new(),
                warnings: Vec::new(),
                input_paths: Vec::new(),
            },
        }
    }

    fn detect_rust(&mut self) {
        let cargo_path = self.project_dir.join("Cargo.toml");
        if !cargo_path.is_file() {
            return;
        }
        if self.parse_toml_marker(&cargo_path).is_none() {
            return;
        }

        let toolchain = self
            .parse_toml_marker(&self.project_dir.join("rust-toolchain.toml"))
            .and_then(|marker| marker.value(["toolchain"], "channel").map(ToOwned::to_owned));
        let label = match toolchain {
            Some(channel) if !channel.is_empty() => format!("Rust (toolchain `{channel}`)"),
            _ => "Rust".to_string(),
        };
        self.detection.runtimes.push(RuntimeDetection::new("rust", label));
        self.detection.tests.push(CommandDetection::new("rust", "cargo test"));

        let clippy_path = self.project_dir.join("clippy.toml");
        if clippy_path.is_file() && self.parse_toml_marker(&clippy_path).is_some() {
            self.detection
                .linting
                .push(LintDetection::Command(CommandDetection::new("rust-clippy", "cargo clippy")));
        }
    }

    fn detect_node(&mut self) {
        let package_path = self.project_dir.join("package.json");
        let Some(package) = self.parse_json_marker(&package_path) else {
            return;
        };

        let engine = package
            .get("engines")
            .and_then(|engines| engines.get("node"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty());
        let label = engine.map_or_else(
            || "Node.js".to_string(),
            |version| format!("Node.js (engines.node `{version}`)"),
        );
        self.detection.runtimes.push(RuntimeDetection::new("node", label));

        let scripts = package.get("scripts").and_then(Value::as_object);
        if scripts.and_then(|scripts| scripts.get("test")).and_then(Value::as_str).is_some() {
            self.detection.tests.push(CommandDetection::new("node", "npm test"));
        }
        if scripts.and_then(|scripts| scripts.get("lint")).and_then(Value::as_str).is_some() {
            self.detection
                .linting
                .push(LintDetection::Command(CommandDetection::new("node-lint", "npm run lint")));
        }
    }

    fn detect_python(&mut self) {
        let pyproject_path = self.project_dir.join("pyproject.toml");
        let requirements_path = self.project_dir.join("requirements.txt");
        let pyproject_detected =
            pyproject_path.is_file() && self.parse_toml_marker(&pyproject_path).is_some();
        if pyproject_detected {
            self.detection
                .runtimes
                .push(RuntimeDetection::new("python", "Python (pyproject.toml)".to_string()));
        } else if requirements_path.is_file() {
            if self.read_marker(&requirements_path).is_none() {
                return;
            }
            self.detection
                .runtimes
                .push(RuntimeDetection::new("python", "Python (requirements.txt)".to_string()));
        }

        let ruff_path = self.project_dir.join("ruff.toml");
        if ruff_path.is_file() && self.parse_toml_marker(&ruff_path).is_some() {
            self.detection
                .linting
                .push(LintDetection::Command(CommandDetection::new("ruff", "ruff check")));
        }
    }

    fn detect_go(&mut self) {
        let go_mod_path = self.project_dir.join("go.mod");
        if !go_mod_path.is_file() {
            return;
        }
        let Some(contents) = self.read_marker(&go_mod_path) else {
            return;
        };
        let version = parse_go_version(&contents);
        let label = version.map_or_else(|| "Go".to_string(), |version| format!("Go {version}"));
        self.detection.runtimes.push(RuntimeDetection::new("go", label));
        self.detection.tests.push(CommandDetection::new("go", "go test ./..."));
    }

    fn detect_deno(&mut self) {
        let deno_path = ["deno.json", "deno.jsonc"]
            .iter()
            .map(|name| self.project_dir.join(name))
            .find(|path| path.is_file());
        let Some(deno_path) = deno_path else {
            return;
        };
        let Some(config) = self.parse_deno_marker(&deno_path) else {
            return;
        };

        self.detection.runtimes.push(RuntimeDetection::new("deno", "Deno".to_string()));
        let tasks = config.get("tasks").and_then(Value::as_object);
        if tasks.and_then(|tasks| tasks.get("test")).and_then(Value::as_str).is_some() {
            self.detection.tests.push(CommandDetection::new("deno", "deno task test"));
        }
        if tasks.and_then(|tasks| tasks.get("lint")).and_then(Value::as_str).is_some() {
            self.detection.linting.push(LintDetection::Command(CommandDetection::new(
                "deno-lint-task",
                "deno task lint",
            )));
        } else if config.get("lint").is_some() {
            self.detection
                .linting
                .push(LintDetection::Command(CommandDetection::new("deno-lint", "deno lint")));
        }
    }

    fn detect_make(&mut self) {
        let makefile_path = self.project_dir.join("Makefile");
        if !makefile_path.is_file() {
            return;
        }
        let Some(contents) = self.read_marker(&makefile_path) else {
            return;
        };
        let targets = parse_make_targets(&contents);
        if targets.has_test {
            self.detection.tests.push(CommandDetection::new("make-test", "make test"));
        }
        if targets.has_checks {
            self.detection
                .linting
                .push(LintDetection::Command(CommandDetection::new("make-checks", "make checks")));
        }
    }

    fn detect_github_actions(&mut self) {
        let workflows_dir = self.project_dir.join(".github").join("workflows");
        if !workflows_dir.is_dir() {
            return;
        }
        let mut workflows = Vec::new();
        let Ok(entries) = fs::read_dir(workflows_dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension() == Some(OsStr::new("yaml")) {
                workflows.push(path);
            }
        }
        workflows.sort();
        let Some(first_workflow) = workflows.first() else {
            return;
        };
        let Some(value) = self.parse_yaml_marker(first_workflow) else {
            return;
        };
        let Some(name) =
            value.get("name").and_then(Value::as_str).filter(|value| !value.is_empty())
        else {
            return;
        };
        self.detection.linting.push(LintDetection::Workflow(name.to_string()));
    }

    fn parse_json_marker(&mut self, path: &Path) -> Option<Value> {
        if !path.is_file() {
            return None;
        }
        let contents = self.read_marker(path)?;
        match serde_json::from_str(&contents) {
            Ok(value) => Some(value),
            Err(err) => {
                self.warn(path, format!("failed to parse JSON marker: {err}"));
                None
            }
        }
    }

    fn parse_deno_marker(&mut self, path: &Path) -> Option<Value> {
        let contents = self.read_marker(path)?;
        let json = if path.file_name() == Some(OsStr::new("deno.jsonc")) {
            strip_json_comments(&contents)
        } else {
            contents
        };
        match serde_json::from_str(&json) {
            Ok(value) => Some(value),
            Err(err) => {
                self.warn(path, format!("failed to parse JSON marker: {err}"));
                None
            }
        }
    }

    fn parse_yaml_marker(&mut self, path: &Path) -> Option<Value> {
        let contents = self.read_marker(path)?;
        match serde_saphyr::from_str(&contents) {
            Ok(value) => Some(value),
            Err(err) => {
                self.warn(path, format!("failed to parse YAML marker: {err}"));
                None
            }
        }
    }

    fn parse_toml_marker(&mut self, path: &Path) -> Option<TomlMarker> {
        if !path.is_file() {
            return None;
        }
        let contents = self.read_marker(path)?;
        match TomlMarker::parse(&contents) {
            Ok(marker) => Some(marker),
            Err(err) => {
                self.warn(path, format!("failed to parse TOML marker: {err}"));
                None
            }
        }
    }

    fn read_marker(&mut self, path: &Path) -> Option<String> {
        match fs::read_to_string(path) {
            Ok(contents) => {
                self.detection.input_paths.push(relative_marker_path(self.project_dir, path));
                Some(contents)
            }
            Err(err) => {
                self.warn(path, format!("failed to read marker: {err}"));
                None
            }
        }
    }

    fn warn(&mut self, path: &Path, message: String) {
        self.detection.warnings.push(DetectionWarning {
            path: relative_marker_path(self.project_dir, path),
            message,
        });
    }

    fn sort(&mut self) {
        self.detection.runtimes.sort_by(|left, right| left.id.cmp(right.id));
        self.detection.runtimes.dedup_by(|left, right| left.id == right.id);
        self.detection.tests.sort_by(|left, right| left.id.cmp(right.id));
        self.detection.tests.dedup_by(|left, right| left.id == right.id);
        self.detection.linting.sort_by(|left, right| left.id().cmp(right.id()));
        self.detection.linting.dedup_by(|left, right| left.id() == right.id());
        self.detection.warnings.sort_by(|left, right| left.path.cmp(&right.path));
        self.detection.input_paths.sort();
        self.detection.input_paths.dedup();
    }
}

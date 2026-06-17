//! `kind: tool` evaluator per `kind: tool` evaluator contract.
//!
//! Hint `value` is the declared tool name. The runner trait the
//! evaluator is plumbed with — [`ToolRunner`] — defers WASI host
//! wiring to the CLI layer so the standards crate stays free of a
//! `wasmtime` / `specify-tool` crate dependency. The CLI implementation
//! lives in `specify lint` (S9); this module only consumes the
//! abstract trait surface.
//!
//! The tool runs once per candidate file. The candidate's
//! project-relative path is the first positional argument; when the
//! rule's `kind: tool` hint carries a `config:` block, its JSON
//! serialisation is forwarded as a second positional argument so the
//! tool reads its policy (caps, allow-lists, grammars) from the rule
//! file — the engine relays the value, it never interprets it (the
//! no-embedded-policy invariant). The closed `{artifact}` /
//! `{project_dir}` / `{rule_id}` placeholder set named in the contract
//! cannot be expanded in v1 because the closed [`crate::rules::RuleHint`]
//! shape carries no `args:` field; extending the hint shape is the rules
//! schema's responsibility, not this evaluator's.
//!
//! Per `kind: tool` evaluator contract:
//!
//! - Tools the project did not declare emit a single
//!   `tool.undeclared` finding (severity `important`).
//! - Successful runs whose stdout is the `DiagnosticReport`
//!   envelope OR a single `Diagnostic` body fold the tool's
//!   findings straight into the scan result; the umbrella
//!   re-stamps `id` and `fingerprint` after applying the §"Evidence
//!   cap" truncation.
//! - Non-zero exit with no findings emits one
//!   `tool.invocation-failed` finding with the rule's severity and
//!   the (truncated) stderr in `Snippet` evidence.
//! - Runner-level invocation failures (e.g. WASI host could not
//!   start the tool) propagate as
//!   [`super::HintError::ToolInvocation`] for the caller to map to
//!   the lint exit mapping exit-code table.

use std::path::{Path, PathBuf};

use specify_diagnostics::{
    Diagnostic, DiagnosticReport, FindingEvidence, FindingLocation, Severity,
};
use thiserror::Error;

use super::{HintError, SyntheticFinding, make_synthetic_finding, restamp_finding};
use crate::rules::{ResolvedRule, RuleHint};

const STDERR_MAX_BYTES: usize = 8 * 1024;

/// Trait the umbrella plumbs through [`super::EvalEnv`] into rule
/// evaluation (see [`super::evaluate_rules`]) so the WASI runtime stays
/// out of the standards crate's dep graph.
///
/// `specify lint` (S9) supplies a `wasmtime`-backed implementation;
/// integration tests in this crate supply a fake.
pub trait ToolRunner {
    /// Invoke the named tool with `args` against `project_dir`.
    ///
    /// # Errors
    ///
    /// Returns [`ToolRunError`] when the runtime cannot deliver the
    /// tool's stdout / stderr / exit-code triple (the tool itself
    /// could not be started, the WASI sandbox refused the
    /// permission set, etc.). A successful invocation that simply
    /// exited non-zero is reported as `Ok(ToolOutput { exit_code, … })`.
    fn run(
        &self, tool_name: &str, args: &[String], project_dir: &Path,
    ) -> Result<ToolOutput, ToolRunError>;
    /// Return `true` when `tool_name` is declared by the project's
    /// `tools.yaml` (or an adapter-declared tool the project has
    /// granted `review` capability).
    fn is_declared(&self, tool_name: &str) -> bool;
    /// Invoke the named tool and return its findings as typed
    /// [`Diagnostic`] values.
    ///
    /// The default implementation runs [`Self::run`] and parses the
    /// `DiagnosticReport` envelope (or single-`Diagnostic` body) off
    /// stdout — the wire contract WASI tools print. In-process
    /// runners override this to hand back typed findings directly and
    /// skip the JSON serialise→reparse round-trip.
    ///
    /// # Errors
    ///
    /// Returns [`ToolRunError`] under the same conditions as
    /// [`Self::run`].
    fn run_diagnostics(
        &self, tool_name: &str, args: &[String], project_dir: &Path,
    ) -> Result<ToolDiagnostics, ToolRunError> {
        let output = self.run(tool_name, args, project_dir)?;
        Ok(ToolDiagnostics {
            findings: parse_tool_findings(&output),
            stderr: output.stderr,
            exit_code: output.exit_code,
        })
    }
}

/// A [`ToolRunner`] that declares no WASI tools.
///
/// The framework surface uses it because every framework `kind: tool`
/// rule resolves to an in-process `framework_tools` checker before the
/// runner is consulted; the trait is required only by the project-side
/// WASI path.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopToolRunner;

impl ToolRunner for NoopToolRunner {
    fn run(
        &self, tool_name: &str, _args: &[String], _project_dir: &Path,
    ) -> Result<ToolOutput, ToolRunError> {
        Err(ToolRunError::Runtime(format!("tool {tool_name} is not a declared WASI tool")))
    }

    fn is_declared(&self, _tool_name: &str) -> bool {
        false
    }
}

/// Typed result of one [`ToolRunner::run_diagnostics`] invocation:
/// the parsed findings plus the stderr / exit-code pair the evaluator
/// needs to synthesise `tool.invocation-failed`.
#[derive(Debug, Clone)]
pub struct ToolDiagnostics {
    /// Findings the tool reported (empty when the tool found nothing
    /// or its output did not parse).
    pub findings: Vec<Diagnostic>,
    /// Verbatim stderr bytes.
    pub stderr: Vec<u8>,
    /// Process exit code (0 on success).
    pub exit_code: i32,
}

/// Captured stdout / stderr / exit code from one tool invocation.
#[derive(Debug, Clone)]
pub struct ToolOutput {
    /// Verbatim stdout bytes.
    pub stdout: Vec<u8>,
    /// Verbatim stderr bytes.
    pub stderr: Vec<u8>,
    /// Process exit code (0 on success).
    pub exit_code: i32,
}

/// Closed runtime failure mode for [`ToolRunner::run`].
#[derive(Debug, Error)]
pub enum ToolRunError {
    /// Runtime could not deliver the tool's stdout / stderr / exit
    /// triple.
    #[error("tool runtime failure: {0}")]
    Runtime(String),
}

pub(crate) fn evaluate(
    rule: &ResolvedRule, hint: &RuleHint, candidates: &[PathBuf], project_dir: &Path,
    runner: &dyn ToolRunner, next_id: &mut u64,
) -> Result<Vec<Diagnostic>, HintError> {
    // First-party framework checkers resolve in-process: no
    // `ToolRunner` hop, no JSON round-trip. The trait survives for the
    // genuine project-side WASI path only.
    if crate::lint::framework_tools::is_framework_checker(&hint.value) {
        return Ok(evaluate_in_process(hint, candidates, project_dir, next_id));
    }
    if !runner.is_declared(&hint.value) {
        let finding = build_undeclared(rule, hint, *next_id);
        *next_id += 1;
        return Ok(vec![finding]);
    }
    let mut out: Vec<Diagnostic> = Vec::new();
    for candidate in candidates {
        let args = tool_args(candidate, hint);
        let result = runner.run_diagnostics(&hint.value, &args, project_dir).map_err(|err| {
            HintError::ToolInvocation {
                rule_id: rule.rule_id.clone(),
                tool: hint.value.clone(),
                detail: err.to_string(),
            }
        })?;
        if result.findings.is_empty() && result.exit_code != 0 {
            let finding = build_invocation_failed(rule, hint, *next_id, candidate, &result.stderr);
            *next_id += 1;
            out.push(finding);
            continue;
        }
        for mut finding in result.findings {
            restamp_finding(&mut finding, *next_id);
            *next_id += 1;
            out.push(finding);
        }
    }
    Ok(out)
}

/// Run an in-process framework checker once per candidate, folding its
/// typed findings with `id` / `fingerprint` re-stamped monotonically
/// from `next_id` — the same fold the WASI path applies, minus the
/// runner hop and JSON round-trip.
fn evaluate_in_process(
    hint: &RuleHint, candidates: &[PathBuf], project_dir: &Path, next_id: &mut u64,
) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();
    for candidate in candidates {
        let args = tool_args(candidate, hint);
        let Some(findings) =
            crate::lint::framework_tools::run_checker(&hint.value, project_dir, &args)
        else {
            continue;
        };
        for mut finding in findings {
            restamp_finding(&mut finding, *next_id);
            *next_id += 1;
            out.push(finding);
        }
    }
    out
}

/// Build the positional args for one tool invocation: the candidate's
/// project-relative path, plus — when the rule declares one — its
/// `config:` serialised as JSON.
///
/// Forwarding `config` is how a referenced tool reads its policy
/// (caps, allow-lists, grammars) from the rule file rather than baking
/// a rule-specific literal into the tool source (the no-embedded-policy
/// invariant). The shape stays generic — the engine never interprets the
/// payload; it relays the rule-owned value to the tool that consumes it.
fn tool_args(candidate: &Path, hint: &RuleHint) -> Vec<String> {
    let mut args = vec![candidate.to_string_lossy().into_owned()];
    if let Some(config) = &hint.config
        && let Ok(serialised) = serde_json::to_string(config)
    {
        args.push(serialised);
    }
    args
}

fn parse_tool_findings(output: &ToolOutput) -> Vec<Diagnostic> {
    if output.stdout.is_empty() {
        return Vec::new();
    }
    if let Ok(envelope) = serde_json::from_slice::<DiagnosticReport>(&output.stdout) {
        return envelope.findings;
    }
    if let Ok(single) = serde_json::from_slice::<Diagnostic>(&output.stdout) {
        return vec![single];
    }
    Vec::new()
}

fn build_undeclared(rule: &ResolvedRule, hint: &RuleHint, id_num: u64) -> Diagnostic {
    let evidence = FindingEvidence::Snippet {
        value: format!("tool {tool} not declared in the project's project.yaml", tool = hint.value),
    };
    make_synthetic_finding(SyntheticFinding {
        id_num,
        rule_id: "tool.undeclared",
        title: format!("Extension {} is not declared by the project", hint.value),
        severity: Severity::Important,
        location: None,
        evidence,
        impact: format!(
            "Rule {rule} cannot run; declared-tool gating refused the invocation.",
            rule = rule.rule_id
        ),
        remediation: format!(
            "Declare {tool} in project.yaml or remove the hint.",
            tool = hint.value
        ),
        target_adapter: None,
    })
}

fn build_invocation_failed(
    rule: &ResolvedRule, hint: &RuleHint, id_num: u64, candidate: &Path, stderr: &[u8],
) -> Diagnostic {
    let snippet = clip_stderr(stderr);
    let evidence = FindingEvidence::Snippet { value: snippet };
    let location = FindingLocation {
        path: candidate.to_string_lossy().into_owned(),
        line: None,
        column: None,
        end_line: None,
        end_column: None,
    };
    make_synthetic_finding(SyntheticFinding {
        id_num,
        rule_id: "tool.invocation-failed",
        title: format!("Extension {} exited non-zero on {}", hint.value, candidate.display()),
        severity: rule.severity,
        location: Some(location),
        evidence,
        impact: format!(
            "Rule {rule} could not be evaluated; the declared tool exited non-zero.",
            rule = rule.rule_id
        ),
        remediation: format!("Inspect the tool stderr above and rerun {tool}.", tool = hint.value),
        target_adapter: None,
    })
}

fn clip_stderr(stderr: &[u8]) -> String {
    if stderr.len() <= STDERR_MAX_BYTES {
        return String::from_utf8_lossy(stderr).into_owned();
    }
    let mut out = String::from_utf8_lossy(&stderr[..STDERR_MAX_BYTES]).into_owned();
    out.push_str("…[truncated]");
    out
}

#[cfg(test)]
mod unit {
    use std::sync::Mutex;

    use serde_json::json;

    use super::*;
    use crate::lint::eval::testkit::{candidates, hint, hint_with_config, rule};
    use crate::rules::HintKind;

    /// Canned-output runner that records the args of every invocation.
    struct FakeRunner {
        declared: bool,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        exit_code: i32,
        calls: Mutex<Vec<Vec<String>>>,
    }

    impl FakeRunner {
        fn new(stdout: &str, exit_code: i32) -> Self {
            Self {
                declared: true,
                stdout: stdout.as_bytes().to_vec(),
                stderr: vec![],
                exit_code,
                calls: Mutex::new(vec![]),
            }
        }
    }

    impl ToolRunner for FakeRunner {
        fn run(
            &self, _tool_name: &str, args: &[String], _project_dir: &Path,
        ) -> Result<ToolOutput, ToolRunError> {
            self.calls.lock().expect("calls lock").push(args.to_vec());
            Ok(ToolOutput {
                stdout: self.stdout.clone(),
                stderr: self.stderr.clone(),
                exit_code: self.exit_code,
            })
        }

        fn is_declared(&self, _tool_name: &str) -> bool {
            self.declared
        }
    }

    struct FailingRunner;

    impl ToolRunner for FailingRunner {
        fn run(
            &self, _tool_name: &str, _args: &[String], _project_dir: &Path,
        ) -> Result<ToolOutput, ToolRunError> {
            Err(ToolRunError::Runtime("host refused".to_string()))
        }

        fn is_declared(&self, _tool_name: &str) -> bool {
            true
        }
    }

    fn single_finding_json() -> String {
        serde_json::to_string(&json!({
            "id": "FIND-9999",
            "rule-id": "demo.rule",
            "source": "tool",
            "kind": "violation",
            "severity": "important",
            "title": "demo finding",
            "evidence": { "type": "snippet", "value": "x" },
            "fingerprint": "0".repeat(64),
        }))
        .expect("finding json")
    }

    #[test]
    fn undeclared_tool_emits_single_finding() {
        let runner = FakeRunner {
            declared: false,
            ..FakeRunner::new("", 0)
        };
        let hint = hint(HintKind::Tool, "ghost");
        let out =
            evaluate(&rule(), &hint, &candidates(&["a.md"]), Path::new("/tmp"), &runner, &mut 1)
                .expect("evaluate");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rule_id.as_deref(), Some("tool.undeclared"));
        assert!(runner.calls.lock().expect("calls lock").is_empty(), "undeclared tools never run");
    }

    #[test]
    fn findings_folded_and_restamped() {
        let runner = FakeRunner::new(&single_finding_json(), 1);
        let hint = hint(HintKind::Tool, "demo");
        let out = evaluate(
            &rule(),
            &hint,
            &candidates(&["a.md", "b.md"]),
            Path::new("/tmp"),
            &runner,
            &mut 7,
        )
        .expect("evaluate");
        assert_eq!(out.len(), 2);
        // Ids are re-stamped monotonically from the seed, not taken
        // from the tool's wire payload.
        assert_eq!(out[0].id, "FIND-0007");
        assert_eq!(out[1].id, "FIND-0008");
    }

    #[test]
    fn nonzero_exit_without_findings_is_invocation_failed() {
        let runner = FakeRunner {
            stderr: b"boom".to_vec(),
            ..FakeRunner::new("", 3)
        };
        let hint = hint(HintKind::Tool, "demo");
        let out =
            evaluate(&rule(), &hint, &candidates(&["a.md"]), Path::new("/tmp"), &runner, &mut 1)
                .expect("evaluate");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rule_id.as_deref(), Some("tool.invocation-failed"));
    }

    #[test]
    fn runtime_failure_propagates() {
        let hint = hint(HintKind::Tool, "demo");
        let result = evaluate(
            &rule(),
            &hint,
            &candidates(&["a.md"]),
            Path::new("/tmp"),
            &FailingRunner,
            &mut 1,
        );
        assert!(matches!(result, Err(HintError::ToolInvocation { .. })));
    }

    #[test]
    fn config_forwarded_as_second_arg() {
        let runner = FakeRunner::new("", 0);
        let hint = hint_with_config(HintKind::Tool, "demo", Some(json!({ "max": 3 })));
        evaluate(&rule(), &hint, &candidates(&["a.md"]), Path::new("/tmp"), &runner, &mut 1)
            .expect("evaluate");
        let calls = runner.calls.lock().expect("calls lock").clone();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], vec!["a.md".to_string(), r#"{"max":3}"#.to_string()]);
    }
}

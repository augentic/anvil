//! Minimal WASI extension for engine host-dispatch integration tests.
//!
//! Implements deterministic `infer`, `prepare build`, and `schema` subcommands
//! so the specify engine can exercise adapter-agnostic wiring without the
//! real vectis component.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Default cluster fingerprint returned when `--parts` is absent.
const DEFAULT_FP: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
/// Pinned-cluster fingerprint returned when `--parts` is present.
const PINNED_FP: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
/// Unpinned cluster fingerprint for slug-collision scenarios with `--parts`.
const UNPINNED_FP: &str =
    "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let (json, code) = dispatch(&args);
    println!("{json}");
    ExitCode::from(code)
}

fn dispatch(args: &[String]) -> (String, u8) {
    match args.first().map(String::as_str) {
        Some("infer") => infer(args),
        Some("prepare") => prepare(args),
        Some("schema") => schema(args),
        _ => (
            r#"{"error":"unknown-command","message":"expected infer, prepare, or schema"}"#
                .to_string(),
            2,
        ),
    }
}

fn infer(args: &[String]) -> (String, u8) {
    let mut composition: Option<PathBuf> = None;
    let mut parts: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--composition" if i + 1 < args.len() => {
                composition = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--parts" if i + 1 < args.len() => {
                parts = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--candidate-cache" | "--min-occurrences" => i += 2,
            _ => i += 1,
        }
    }

    if composition
        .as_ref()
        .is_some_and(|p| p.file_name().is_some_and(|n| n == "empty.yaml"))
    {
        return (empty_report(), 0);
    }

    if let Some(parts_path) = parts {
        let slug = first_part_slug(&parts_path).unwrap_or_else(|| "primary-nav".to_string());
        return (parts_report(&slug), 0);
    }

    (default_report(), 0)
}

fn prepare(args: &[String]) -> (String, u8) {
    if args.get(1).map(String::as_str) != Some("build") {
        return (
            r#"{"error":"unknown-prepare","message":"expected prepare build <slice>"}"#.to_string(),
            2,
        );
    }
    let body = r#"{"bootstrap_app_icon":{"findings":[{"id":"plan-bootstrap-app-icon-missing"}]}}"#;
    (body.to_string(), 1)
}

fn schema(args: &[String]) -> (String, u8) {
    match args.get(1).map(String::as_str) {
        Some("tokens") => (
            r#"{"$id":"https://example.com/dispatch-fixture/tokens"}"#.to_string(),
            0,
        ),
        Some(name) => {
            let body = format!(
                r#"{{"error":"unknown-schema","message":"unknown schema: {name:?}","exit-code":2}}"#
            );
            (body, 2)
        }
        None => (r#"{"error":"unknown-schema","exit-code":2}"#.to_string(), 2),
    }
}

fn default_report() -> String {
    format!(
        r#"{{
  "version": 1,
  "clusters": [{{
    "fingerprint": "{DEFAULT_FP}",
    "occurrences": 2,
    "screens": ["home", "search"],
    "bound-slug": null,
    "evidence": {{ "region": "footer" }}
  }}],
  "unmatched-parts": []
}}"#
    )
}

fn empty_report() -> String {
    r#"{"version":1,"clusters":[],"unmatched-parts":[]}"#.to_string()
}

fn parts_report(slug: &str) -> String {
    format!(
        r#"{{
  "version": 1,
  "clusters": [
    {{
      "fingerprint": "{PINNED_FP}",
      "occurrences": 1,
      "screens": ["home"],
      "bound-slug": "{slug}",
      "pinned": true,
      "evidence": {{ "region": "footer" }}
    }},
    {{
      "fingerprint": "{UNPINNED_FP}",
      "occurrences": 2,
      "screens": ["home", "search"],
      "bound-slug": null,
      "evidence": {{ "region": "body" }}
    }}
  ],
  "unmatched-parts": []
}}"#
    )
}

fn first_part_slug(parts_path: &Path) -> Option<String> {
    let content = fs::read_to_string(parts_path).ok()?;
    let mut after_parts = false;
    for line in content.lines() {
        if line.trim() == "parts:" {
            after_parts = true;
            continue;
        }
        if after_parts {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            return trimmed.strip_suffix(':').map(str::to_string);
        }
    }
    None
}

//! `specify validate` -- check project `OpenSpec` configuration and structure.

use anyhow::Result;
use console::style;

use crate::core::change::Change;
use crate::core::config::ProjectConfig;
use crate::core::delta;
use crate::core::graph::ArtifactGraph;
use crate::core::paths::ProjectDir;
use crate::core::schema::Schema;

/// Diagnostic severity.
#[derive(Debug, Clone, Copy)]
enum Level {
    Error,
    Warn,
}

/// A single validation finding.
struct Finding {
    level: Level,
    message: String,
}

/// Run the validate command.
///
/// # Errors
///
/// Returns an error if the current directory cannot be determined.
pub fn run() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let mut findings: Vec<Finding> = Vec::new();

    let Ok(project) = ProjectDir::discover(&cwd) else {
        println!(
            "\n  {} No specify/ directory found. Run {} first.\n",
            style("✗").red().bold(),
            style("specify init").yellow()
        );
        return Ok(());
    };

    check_config(&project, &mut findings);
    check_schema(&project, &mut findings);
    check_graph(&project, &mut findings);
    check_changes(&project, &mut findings);

    print_findings(&findings);
    Ok(())
}

fn check_config(project: &ProjectDir, findings: &mut Vec<Finding>) {
    let config_path = project.config_file();
    if !config_path.is_file() {
        findings.push(Finding {
            level: Level::Error,
            message: format!("config.yaml not found at {}", config_path.display()),
        });
        return;
    }

    match ProjectConfig::load(&config_path) {
        Ok(config) => {
            if config.schema.is_empty() {
                findings.push(Finding {
                    level: Level::Error,
                    message: "config.yaml: 'schema' field is empty".to_string(),
                });
            }
            if config.context.trim().is_empty() {
                findings.push(Finding {
                    level: Level::Warn,
                    message: "config.yaml: 'context' field is empty (recommended for quality)"
                        .to_string(),
                });
            }
        }
        Err(e) => {
            findings.push(Finding {
                level: Level::Error,
                message: format!("config.yaml: parse error: {e}"),
            });
        }
    }
}

fn check_schema(project: &ProjectDir, findings: &mut Vec<Finding>) {
    let Ok(config) = ProjectConfig::load(&project.config_file()) else {
        return;
    };

    let schema_dir = project.schema_dir(&config.schema);
    if !schema_dir.is_dir() {
        findings.push(Finding {
            level: Level::Error,
            message: format!("schema directory not found: schemas/{}/", config.schema),
        });
        return;
    }

    let schema_yaml_path = schema_dir.join("schema.yaml");
    if !schema_yaml_path.is_file() {
        findings.push(Finding {
            level: Level::Error,
            message: format!("schemas/{}/schema.yaml not found", config.schema),
        });
        return;
    }

    let Some(schema) =
        std::fs::read(&schema_yaml_path).ok().and_then(|b| Schema::from_yaml(&b).ok())
    else {
        findings.push(Finding {
            level: Level::Error,
            message: format!("schemas/{}/schema.yaml: parse error", config.schema),
        });
        return;
    };

    let templates_dir = schema_dir.join("templates");
    for template_name in schema.template_names() {
        let path = templates_dir.join(template_name);
        if !path.is_file() {
            findings.push(Finding {
                level: Level::Error,
                message: format!(
                    "template not found: schemas/{}/templates/{template_name}",
                    config.schema
                ),
            });
        }
    }
}

/// Validate that the artifact graph is acyclic and all references resolve.
fn check_graph(project: &ProjectDir, findings: &mut Vec<Finding>) {
    let Ok(config) = ProjectConfig::load(&project.config_file()) else {
        return;
    };
    let Ok(schema) = Schema::load(project, &config.schema) else {
        return;
    };

    if let Err(e) = ArtifactGraph::from_schema(&schema) {
        findings.push(Finding {
            level: Level::Error,
            message: format!("artifact graph: {e}"),
        });
    }
}

fn check_changes(project: &ProjectDir, findings: &mut Vec<Finding>) {
    let Ok(config) = ProjectConfig::load(&project.config_file()) else {
        return;
    };
    let Ok(schema) = Schema::load(project, &config.schema) else {
        return;
    };
    let Ok(graph) = ArtifactGraph::from_schema(&schema) else {
        return;
    };

    let Ok(changes) = Change::discover_active(project) else {
        return;
    };

    for change in &changes {
        check_change_metadata(change, findings);
        check_change_artifacts(change, &graph, findings);
        check_change_delta_specs(change, findings);
    }
}

fn check_change_metadata(change: &Change, findings: &mut Vec<Finding>) {
    if change.metadata.schema.is_empty() {
        findings.push(Finding {
            level: Level::Warn,
            message: format!("change '{}': .metadata.yaml has empty schema field", change.name),
        });
    }
    if change.metadata.created_at.is_empty() {
        findings.push(Finding {
            level: Level::Warn,
            message: format!("change '{}': .metadata.yaml has empty created_at field", change.name),
        });
    }
}

fn check_change_artifacts(change: &Change, graph: &ArtifactGraph, findings: &mut Vec<Finding>) {
    for id in graph.artifact_ids() {
        let Some(pattern) = graph.generates(id) else {
            continue;
        };
        if pattern.contains("**") {
            continue;
        }
        let file_path = change.path.join(pattern);
        if !file_path.is_file() {
            findings.push(Finding {
                level: Level::Warn,
                message: format!("change '{}': missing artifact '{pattern}'", change.name),
            });
        }
    }
}

/// Validate delta spec structure if spec files exist.
fn check_change_delta_specs(change: &Change, findings: &mut Vec<Finding>) {
    let specs_dir = change.path.join("specs");
    if !specs_dir.is_dir() {
        return;
    }

    let Ok(entries) = std::fs::read_dir(&specs_dir) else {
        return;
    };

    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let capability = entry.file_name().to_string_lossy().to_string();
        let spec_path = entry.path().join("spec.md");
        if !spec_path.is_file() {
            continue;
        }

        let Ok(content) = std::fs::read_to_string(&spec_path) else {
            continue;
        };

        if let Err(e) = delta::parse_sections(&content) {
            findings.push(Finding {
                level: Level::Error,
                message: format!("change '{}': specs/{capability}/spec.md: {e}", change.name,),
            });
        }
    }
}

fn print_findings(findings: &[Finding]) {
    let errors = findings.iter().filter(|f| matches!(f.level, Level::Error)).count();
    let warnings = findings.iter().filter(|f| matches!(f.level, Level::Warn)).count();

    println!();
    if findings.is_empty() {
        println!("  {} OpenSpec configuration is valid.\n", style("✓").green().bold());
        return;
    }

    for finding in findings {
        let (icon, colored) = match finding.level {
            Level::Error => (style("✗").red().bold(), style(&finding.message).red()),
            Level::Warn => (style("⚠").yellow().bold(), style(&finding.message).yellow()),
        };
        println!("  {icon} {colored}");
    }

    println!();
    if errors > 0 {
        println!("  {} error(s), {} warning(s)\n", style(errors).red().bold(), warnings);
    } else {
        println!("  {} warning(s), no errors\n", style(warnings).yellow().bold());
    }
}

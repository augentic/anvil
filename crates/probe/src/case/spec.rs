//! The `case.toml` shapes and their parse / validation surface: the
//! closed [`Case`] kinds, per-shape field validation, and the case-id
//! listing over a `cases/` root.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context as _, Result, bail, ensure};
use serde::Deserialize;

/// One eval case, parsed from `case.toml` by its `kind` tag.
#[derive(Debug)]
pub enum Case {
    /// A source-to-target workflow over the operator verbs.
    Workflow(Workflow),
    /// One build phase against a committed refined fixture.
    Build(Build),
}

// The closed `kind` tag; parsed first so each shape can carry
// `deny_unknown_fields` (serde's internal tagging cannot).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum Kind {
    Workflow,
    Build,
}

/// A workflow case: `plan author --from --wave` always runs; `until`
/// selects how far past authoring the run continues.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Workflow {
    /// Target adapter passed to `emery init` when the case is in-place
    /// (no supplied definition home). Detached cases init each handoff
    /// target tree instead.
    #[serde(default)]
    pub target: String,
    /// Change name passed to `plan author`.
    pub change: String,
    /// Wave id passed to `plan author --wave`. Defaults to `deliver`.
    #[serde(default)]
    pub wave: Option<String>,
    /// Definition home relative to `case.toml`. Absent means the
    /// sibling `definition/` directory when it exists, else a mint
    /// from `intent` / `[sources]`.
    #[serde(default)]
    pub definition: Option<PathBuf>,
    /// Optional operator intent used to mint a degenerate handoff
    /// when no definition home is supplied.
    #[serde(default)]
    pub intent: Option<String>,
    /// Source bindings used to mint a handoff when no definition home
    /// is supplied (`key = "adapter:value:…"` or `key = "adapter:path"`).
    #[serde(default)]
    pub sources: BTreeMap<String, String>,
    /// Tree copied into the fresh sandbox, relative to `case.toml`;
    /// absent means the sibling `fixture/` directory (when present).
    #[serde(default)]
    pub fixture: Option<PathBuf>,
    /// Upstream tree shallow-cloned on miss into the sibling
    /// `fixture/` cache; mutually exclusive with `fixture`.
    #[serde(default)]
    pub clone: Option<CloneSpec>,
    /// Default stop rung; `--until` overrides per run.
    #[serde(default)]
    pub until: WorkflowUntil,
}

/// One `git clone --depth 1` populating the sibling `fixture/` cache.
///
/// For source trees that cannot ship as committed fixtures (e.g. an
/// `UNLICENSED` upstream): the case directory carries a `.gitignore`
/// over `fixture/`, so the tree never enters the repository. The
/// clone happens once, on miss, with `.git` stripped; every run then
/// copies the cached tree into the sandbox like any other fixture.
/// Refresh the snapshot by deleting the cached tree.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct CloneSpec {
    /// Git URL passed verbatim to `git clone`.
    pub url: String,
    /// Sandbox-relative destination directory.
    pub dest: PathBuf,
}

/// A build case: the fixture carries the exact refined state the
/// build phase consumes, including valid project and slice metadata —
/// the runner never stamps lifecycle state.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Build {
    /// The refined slice the build phase runs for.
    pub slice: String,
    /// Tree copied into the fresh sandbox, relative to `case.toml`;
    /// absent means the sibling `fixture/` directory (when present).
    #[serde(default)]
    pub fixture: Option<PathBuf>,
    /// Sandbox-relative paths that must hold a file after the build.
    pub expect: Vec<String>,
}

/// How far a [`Workflow`] case runs.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowUntil {
    /// Stop after `plan author`, leaving every entry `pending`.
    Plan,
    /// Author, then drain `plan refine`.
    Refine,
    /// Author, refine, then run the genuine drained `plan execute`.
    #[default]
    Execute,
    /// Execute, then `plan archive`.
    Finalize,
}

impl WorkflowUntil {
    /// Kebab-case rung label for run output and span attributes.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Refine => "refine",
            Self::Execute => "execute",
            Self::Finalize => "finalize",
        }
    }
}

impl Case {
    // Kebab-case kind label for run output and span attributes.
    pub(super) const fn label(&self) -> &'static str {
        match self {
            Self::Workflow(_) => "workflow",
            Self::Build(_) => "build",
        }
    }
}

/// Parse and validate one `case.toml` body.
///
/// # Errors
///
/// Returns a missing or unknown `kind`, per-shape parse failures
/// (including unknown keys), and every shape validation failure.
pub fn parse(body: &str) -> Result<Case> {
    let mut table: toml::Table = toml::from_str(body).context("parsing case.toml")?;
    let kind = table
        .remove("kind")
        .context("case.toml requires `kind = \"workflow\"` or `kind = \"build\"`")?;
    let kind: Kind = kind.try_into().context("unknown case `kind`")?;
    let case = match kind {
        Kind::Workflow => {
            Case::Workflow(toml::Value::Table(table).try_into().context("workflow case")?)
        }
        Kind::Build => Case::Build(toml::Value::Table(table).try_into().context("build case")?),
    };
    validate(&case)?;
    Ok(case)
}

/// Parse and validate the case at `<root>/<id>/case.toml`.
///
/// # Errors
///
/// Returns a malformed id, a missing `case.toml` (naming the known
/// cases), and every [`parse`] failure.
pub fn load(root: &Path, id: &str) -> Result<Case> {
    let mut components = Path::new(id).components();
    ensure!(
        matches!((components.next(), components.next()), (Some(Component::Normal(_)), None)),
        "case ids are flat directory names"
    );
    let path = root.join(id).join("case.toml");
    if !path.is_file() {
        bail!(
            "no case.toml at {}; known cases: {}",
            path.display(),
            ids(root).unwrap_or_default().join(", ")
        );
    }
    parse(&fs::read_to_string(&path)?).with_context(|| format!("parsing {}", path.display()))
}

fn validate(case: &Case) -> Result<()> {
    match case {
        Case::Workflow(workflow) => {
            ensure!(!workflow.change.trim().is_empty(), "empty change name");
            if let Some(definition) = &workflow.definition {
                validate_entry(&definition.to_string_lossy()).context("definition")?;
            }
            ensure!(
                workflow.definition.is_some()
                    || workflow.intent.is_some()
                    || !workflow.sources.is_empty(),
                "a workflow case requires `definition`, `intent`, or at least one `[sources]` binding"
            );
            if let Some(clone) = &workflow.clone {
                ensure!(
                    workflow.fixture.is_none(),
                    "`fixture` and `clone` are mutually exclusive — `clone` populates \
                     the sibling `fixture/` itself"
                );
                ensure!(!clone.url.trim().is_empty(), "empty clone url");
                validate_entry(&clone.dest.to_string_lossy()).context("clone `dest`")?;
            }
        }
        Case::Build(build) => {
            ensure!(!build.slice.trim().is_empty(), "empty slice name");
            ensure!(
                !build.expect.is_empty(),
                "build cases must declare at least one `expect` artifact — a success \
                 report that produced nothing would otherwise pass as a silent no-op"
            );
            for rel in &build.expect {
                validate_entry(rel).with_context(|| format!("expect entry `{rel}`"))?;
            }
        }
    }
    Ok(())
}

pub(super) fn validate_entry(rel: &str) -> Result<()> {
    ensure!(!rel.trim().is_empty(), "empty expect entry");
    let path = Path::new(rel);
    ensure!(path.is_relative(), "absolute paths are not allowed");
    ensure!(
        path.components().all(|component| matches!(component, Component::Normal(_))),
        "path components must be plain names (no `..` or `.`)"
    );
    Ok(())
}

pub(super) fn list(root: &Path) -> Result<()> {
    let ids = ids(root)?;
    ensure!(!ids.is_empty(), "no cases under {}", root.display());
    println!("cases (run with `eval <id>`):");
    for id in ids {
        println!("  {id}");
    }
    Ok(())
}

fn ids(root: &Path) -> Result<Vec<String>> {
    let entries = fs::read_dir(root).with_context(|| format!("reading {}", root.display()))?;
    let mut ids: Vec<String> = entries
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.join("case.toml").is_file())
        .filter_map(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
        .collect();
    ids.sort();
    Ok(ids)
}

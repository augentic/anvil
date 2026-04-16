# RFC-1: `specify` CLI

> Status: Draft · Depends: — · Enables: [RFC-2](rfc-2-migration.md), [RFC-3](rfc-3-multi-repo.md), [RFC-4](rfc-4-dsl.md)

## Abstract

Replace prose-interpreted deterministic operations (validation, task parsing, artifact structure checking) with a Rust CLI binary (`specify`) that returns structured JSON and exit codes. The agent retains judgment; the CLI enforces correctness.

## Motivation

Every precision-critical operation — validation, task parsing, artifact structure checking — is currently performed by the LLM interpreting prose rules. This produces unreliable results for operations that are fundamentally structured decision trees.

The CLI is the foundation everything else builds on. Migration commands ([RFC-2](rfc-2-migration.md)), multi-repo coordination ([RFC-3](rfc-3-multi-repo.md)), and skill validation ([RFC-4](rfc-4-dsl.md)) all require a binary that understands `.specify/` structure, spec format, and schema rules. Building the CLI first means every subsequent RFC extends an existing tool rather than creating a new one.

## Design Principles

| Use CLI (`specify ...`) when:                 | Use agent judgment when:                    |
| --------------------------------------------- | ------------------------------------------- |
| The operation must be idempotent              | The response depends on context             |
| The output is structured (JSON, exit codes)   | The output is natural language              |
| Correctness is verifiable (schema validation) | Correctness requires semantic understanding |
| The operation is repeated across many skills  | The operation is unique to one skill        |
| Failure modes are enumerable                  | Failure modes are open-ended                |

The `specify` CLI gives a clean abstraction boundary. Instead of skills containing scattered shell commands, they can use `specify` subcommands that return structured output. The principle: **the CLI owns Specify operations; external tool invocation stays with the agent.**

A good litmus test: "Would this command need to understand `.specify/` directory structure or spec format?" If yes, it belongs in the CLI. If no (like running `cargo test`), it stays as a direct shell command in the skill.

## Detailed Design

### Priority Order

#### Phase 1: Core CLI

1. **Cargo workspace scaffold** — workspace manifest, `specify-cli`, `specify-core`, `specify-check` crates, CI integration
2. **`specify validate`** — the Pass/Fail/Deferred validation engine; replaces ~40 lines of prose validation in the build skill
3. **`specify merge`** — deterministic delta-merge replacing `merge-specs.py`
4. **`specify init`** — project initialization replacing scattered mkdir/write logic
5. **Migrate `init`, `merge`, and `build` skills** to use CLI commands
6. **`specify task`** subcommands — deterministic task parsing and progress tracking
7. **`specify check`** — port `checks.ts` into `specify-check` crate (runs alongside `checks.ts` during migration, replaces it once complete)

The first four items establish a working binary with immediate value. Items 5–6 close the loop on the core workflow. Item 7 is a natural migration that happens incrementally — each check ported from TypeScript to Rust is removed from `checks.ts` until the script is empty.

#### Phase 2: Migration extensions ([RFC-2](rfc-2-migration.md))

8. **`specify migrate init`** — scaffold `migration.yaml` from a legacy codebase scan
9. **`specify migrate next`** — select the next pending slice from the manifest (respecting `depends_on`)
10. **`specify migrate status`** — track slice-level migration progress across iterations
11. **Slice recommender** — analyse legacy dependency graph and suggest migration ordering
12. **Behavioural diff** — compare legacy fixture output against new implementation output

These build on the existing `/spec:extract`, `wiretapper`, `replay-writer`, and core `/spec:*` skills. See [RFC-2](rfc-2-migration.md) for the full design.

#### Phase 3: Federation extensions ([RFC-3](rfc-3-multi-repo.md))

13. **Federation config** and `specify federation sync` for multi-repo
14. **Cross-repo spec references** and `specify federation validate`

See [RFC-3](rfc-3-multi-repo.md) for the full design.

### Impact on Existing Skills

| Skill    | Current agent-interpreted logic                           | Moves to CLI                                 |
| -------- | --------------------------------------------------------- | -------------------------------------------- |
| `init`   | mkdir, file creation, schema resolution, cache population | `specify init`                               |
| `define` | Schema resolution, metadata writes, overlap detection     | `specify schema resolve`, `specify status`   |
| `build`  | Artifact validation, task progress tracking               | `specify validate`, `specify task next/mark` |
| `merge`  | merge-specs.py invocation, coherence check, archive move  | `specify merge`                              |
| `verify` | Spec parsing, requirement extraction                      | `specify diff`                               |
| `status` | Metadata + task parsing                                   | `specify status`                             |

### Workspace Layout

The CLI lives at the repo root as a Cargo workspace. This keeps it alongside the plugins and schemas it operates on — important because `specify check` needs to validate the repo's own schema files and skills, and integration tests can reference the real `schemas/` directory.

```
specify/                              # repo root (already exists)
├── Cargo.toml                        # workspace manifest
├── Cargo.lock
├── crates/
│   ├── specify-cli/                  # binary crate — thin dispatch layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   ├── specify-core/                 # library crate — all domain logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs             # config.yaml parsing + resolution
│   │       ├── schema.rs             # schema.yaml parsing + composition
│   │       ├── metadata.rs           # .metadata.yaml lifecycle
│   │       ├── spec.rs               # spec format parsing (requirement blocks, deltas)
│   │       ├── task.rs               # task parsing (checkboxes, skill directives)
│   │       ├── blueprint.rs          # blueprint DAG, dependency ordering
│   │       ├── validate.rs           # artifact validation engine
│   │       ├── merge.rs              # deterministic delta-merge (replaces merge-specs.py)
│   │       ├── init.rs               # project initialization logic
│   │       ├── drift.rs              # spec-vs-code drift detection scaffolding
│   │       ├── federation.rs         # multi-repo coordination (RFC-3, stubbed)
│   │       └── error.rs              # unified error types
│   └── specify-check/               # framework validation (replaces checks.ts)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── links.rs              # markdown link resolution
│           ├── schema_integrity.rs   # schema.yaml validation against JSON Schema
│           ├── skill_frontmatter.rs  # SKILL.md frontmatter validation
│           ├── skill_references.rs   # skill reference/symlink resolution
│           ├── skill_variables.rs    # variable consistency
│           ├── skill_directives.rs   # <!-- skill: plugin:skill --> validation
│           ├── marketplace.rs        # marketplace.json vs plugin.json consistency
│           └── plugins_doc.rs        # docs/plugins.md inventory check
├── plugins/                          # existing — unchanged
├── schemas/                          # existing — unchanged
├── scripts/                          # existing — checks.ts stays during migration
└── Makefile                          # updated with new targets
```

### Why Three Crates, Not One

**`specify-core`** is the library. It has no CLI concerns — no argument parsing, no terminal formatting, no exit codes. It returns `Result<T, SpecifyError>` from every public function. This matters because:

1. Skills that invoke the CLI get structured output (JSON). But the logic may also be called from other contexts — a future LSP for schema validation in editors, a WASM build for browser-based tooling, or integration tests that call the library directly.
2. The merge logic, spec parser, and validator are independently testable without spawning processes.

**`specify-cli`** is the binary. It owns argument parsing (via `clap`), output formatting (JSON vs human-readable), exit codes, and I/O. It's a thin dispatch layer — each subcommand is ~20 lines that parse args, call a `specify-core` function, format the result, and set the exit code.

**`specify-check`** is the framework-repo linter. It replaces `checks.ts` over time but serves a different audience than `specify-core`. `specify-core` validates *consumer projects* (artifact correctness at runtime). `specify-check` validates *this repo* (skill integrity, schema consistency, marketplace alignment at CI time). The overlap is small: both parse `schema.yaml`, so they share the `specify-core::schema` module. But the check logic (symlink resolution, SKILL.md frontmatter, docs inventory) is repo-specific and doesn't belong in the runtime library.

### Module Design: `specify-core`

#### `error.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not initialized: .specify/config.yaml not found")]
    NotInitialized,

    #[error("schema resolution failed: {0}")]
    SchemaResolution(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("validation failed: {count} errors")]
    Validation { count: usize, results: Vec<ValidationResult> },

    #[error("merge failed: {0}")]
    Merge(String),

    #[error("lifecycle error: expected {expected}, found {found}")]
    Lifecycle { expected: String, found: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
}
```

A single error type with structured variants means the CLI can pattern-match on the variant to decide exit codes and output format, and the library never touches `std::process::exit`.

#### `config.rs`

```rust
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectConfig {
    pub schema: String,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub overrides: BTreeMap<String, String>,
}

impl ProjectConfig {
    pub fn load(project_dir: &Path) -> Result<Self, Error>;
    pub fn specify_dir(project_dir: &Path) -> PathBuf;
    pub fn changes_dir(project_dir: &Path) -> PathBuf;
    pub fn specs_dir(project_dir: &Path) -> PathBuf;
    pub fn cache_dir(project_dir: &Path) -> PathBuf;
}
```

Straightforward serde deserialization. The path helpers centralise the `.specify/changes/`, `.specify/specs/`, `.specify/.cache/` conventions that are currently scattered across every skill.

#### `schema.rs`

The most important module — it encodes the resolution algorithm from `schema-resolution.md`.

```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct Schema {
    pub name: String,
    pub version: u32,
    pub description: String,
    pub extends: Option<String>,
    pub terminology: Terminology,
    pub blueprints: Vec<Blueprint>,
    #[serde(default)]
    pub validation: BTreeMap<String, bool>,
    pub build: BuildConfig,
    #[serde(default)]
    pub defaults: Option<Defaults>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Terminology {
    pub deliverable: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Blueprint {
    pub id: String,
    pub generates: String,
    pub description: String,
    pub instructions: String,
    pub requires: Vec<String>,
    #[serde(default)]
    pub validate: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BuildConfig {
    pub requires: Vec<String>,
    pub tracks: String,
    pub instructions: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Defaults {
    pub context: Option<String>,
    #[serde(default)]
    pub rules: BTreeMap<String, String>,
}

pub struct ResolvedSchema {
    pub schema: Schema,
    pub root_dir: PathBuf,
    pub source: SchemaSource,
}

pub enum SchemaSource {
    Local(PathBuf),
    Cached(PathBuf),
}

impl Schema {
    /// Full resolution: parse schema value, resolve local/cache,
    /// handle composition via `extends`.
    pub fn resolve(
        schema_value: &str,
        project_dir: &Path,
    ) -> Result<ResolvedSchema, Error>;

    /// Validate schema.yaml structure against the embedded JSON Schema.
    pub fn validate_structure(&self) -> Vec<ValidationResult>;

    /// Topologically sort blueprints by `requires` dependencies.
    pub fn blueprint_order(&self) -> Result<Vec<&Blueprint>, Error>;

    /// Merge a child schema on top of a parent (composition).
    pub fn merge(parent: Schema, child: Schema) -> Schema;
}
```

Note the absence of any HTTP fetching — the `resolve` function handles local and cache paths. Remote fetching (the WebFetch step in the current skill) remains the agent's responsibility. The CLI's `specify schema resolve` subcommand outputs the resolved path so the skill knows where to find files, but the agent does the HTTP fetch if the cache is stale. This keeps the CLI dependency-free for networking and avoids duplicating the agent's authenticated GitHub access.

#### `spec.rs`

Replaces `merge-specs.py`'s parser in Rust.

```rust
pub struct RequirementBlock {
    pub heading: String,
    pub name: String,
    pub id: String,
    pub body: String,
    pub scenarios: Vec<Scenario>,
}

pub struct Scenario {
    pub name: String,
    pub body: String,
}

pub struct ParsedSpec {
    pub preamble: String,
    pub requirements: Vec<RequirementBlock>,
}

pub struct DeltaSpec {
    pub renamed: Vec<RenameEntry>,
    pub removed: Vec<RequirementBlock>,
    pub modified: Vec<RequirementBlock>,
    pub added: Vec<RequirementBlock>,
}

pub fn parse_baseline(text: &str) -> ParsedSpec;
pub fn parse_delta(text: &str) -> DeltaSpec;
pub fn has_delta_headers(text: &str) -> bool;
```

The heading conventions are constants, matching `spec-format.md`:

```rust
pub const REQUIREMENT_HEADING: &str = "### Requirement:";
pub const REQUIREMENT_ID_PREFIX: &str = "ID:";
pub const REQUIREMENT_ID_PATTERN: &str = r"^REQ-[0-9]{3}$";
pub const SCENARIO_HEADING: &str = "#### Scenario:";
pub const DELTA_ADDED: &str = "## ADDED Requirements";
pub const DELTA_MODIFIED: &str = "## MODIFIED Requirements";
pub const DELTA_REMOVED: &str = "## REMOVED Requirements";
pub const DELTA_RENAMED: &str = "## RENAMED Requirements";
```

These are hard-coded rather than configurable because `spec-format.md` explicitly says "These are not configurable per-schema."

#### `merge.rs`

```rust
pub struct MergeResult {
    pub output: String,
    pub operations: Vec<MergeOperation>,
}

pub enum MergeOperation {
    Renamed { id: String, old_name: String, new_name: String },
    Removed { id: String, name: String },
    Modified { id: String, name: String },
    Added { id: String, name: String },
    CreatedBaseline { requirement_count: usize },
}

/// Merge a delta spec into a baseline. If baseline is None, creates
/// a new baseline from the delta's ADDED section.
pub fn merge(
    baseline: Option<&str>,
    delta: &str,
) -> Result<MergeResult, Error>;

/// Post-merge coherence validation.
pub fn validate_baseline(
    baseline: &str,
    design: Option<&str>,
) -> Vec<ValidationResult>;

/// Atomic multi-capability merge. Takes a change directory and merges
/// all capabilities, rolling back on error.
pub fn merge_change(
    change_dir: &Path,
    specs_dir: &Path,
) -> Result<Vec<(String, MergeResult)>, Error>;
```

The merge algorithm is a direct port of `merge-specs.py` with two improvements:

1. **Structured output.** Instead of writing to stdout, it returns `MergeResult` with the merged text and a log of operations. The CLI formats this as JSON for skills or as human-readable text for direct invocation.
2. **Atomic multi-capability merge.** The current skill runs `merge-specs.py` once per capability. The library function `merge_change` takes a change directory and merges all capabilities, rolling back on error.

#### `task.rs`

```rust
pub struct Task {
    pub group: String,
    pub number: String,
    pub description: String,
    pub complete: bool,
    pub skill_directive: Option<SkillDirective>,
}

pub struct SkillDirective {
    pub plugin: String,
    pub skill: String,
}

pub struct TaskProgress {
    pub total: usize,
    pub complete: usize,
    pub tasks: Vec<Task>,
}

pub fn parse_tasks(content: &str) -> TaskProgress;

pub fn mark_complete(
    content: &str,
    task_number: &str,
) -> Result<String, Error>;

pub fn next_pending(tasks: &TaskProgress) -> Option<&Task>;
```

#### `validate.rs`

The `validate` rules in `schema.yaml` are human-readable strings. The CLI handles the *structural* ones deterministically and flags the *semantic* ones for the agent. See [RFC-1-A: Deferred Validation](rfc-1a-validation.md) for the full classification design.

```rust
pub enum ValidationResult {
    Pass { rule: String },
    Fail { rule: String, detail: String },
    Deferred { rule: String, reason: String },
}

pub struct ValidationReport {
    pub blueprint_results: BTreeMap<String, Vec<ValidationResult>>,
    pub cross_checks: Vec<ValidationResult>,
    pub passed: bool,
}

/// Run all deterministic validations for a change.
pub fn validate_change(
    change_dir: &Path,
    schema: &Schema,
) -> ValidationReport;
```

The key design decision: rules that the CLI can check deterministically (heading structure, ID format, checkbox format, section existence) produce `Pass` or `Fail`. Rules that require semantic judgment (like "Uses SHALL/MUST language for normative requirements") produce `Deferred` with an explanation. The skill prose only needs to handle deferred rules.

Built-in structural validators:

```rust
fn has_section(content: &str, heading: &str) -> bool;
fn has_content_after_heading(content: &str, heading: &str) -> bool;
fn all_requirements_have_scenarios(spec: &ParsedSpec) -> bool;
fn all_requirements_have_ids(spec: &ParsedSpec) -> bool;
fn ids_match_pattern(spec: &ParsedSpec, pattern: &str) -> bool;
fn all_tasks_use_checkbox(tasks: &TaskProgress) -> bool;
fn tasks_grouped_under_headings(content: &str) -> bool;
fn proposal_deliverables_have_specs(
    proposal: &str, specs_dir: &Path, term: &str,
) -> bool;
fn design_references_exist(design: &str, specs_dir: &Path) -> bool;
```

#### `metadata.rs`

```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct ChangeMetadata {
    pub schema: String,
    pub status: LifecycleStatus,
    pub created_at: Option<String>,
    pub defined_at: Option<String>,
    pub build_started_at: Option<String>,
    pub completed_at: Option<String>,
    pub touched_specs: Vec<TouchedSpec>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LifecycleStatus {
    Defining,
    Defined,
    Building,
    Complete,
    Merged,
    Dropped,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TouchedSpec {
    pub name: String,
    #[serde(rename = "type")]
    pub spec_type: SpecType,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SpecType {
    New,
    Modified,
}
```

The `LifecycleStatus` enum eliminates the recurring guardrail in every skill: "Valid lifecycle status values are: `defining`, `defined`, `building`, `complete`, `merged`, `dropped`." The CLI enforces this at the type level.

A status transition function prevents invalid moves:

```rust
impl LifecycleStatus {
    pub fn can_transition_to(&self, target: &Self) -> bool {
        matches!(
            (self, target),
            (Defining, Defined)
                | (Defined, Building)
                | (Building, Complete)
                | (Complete, Merged)
                | (Defining | Defined | Building | Complete, Dropped)
        )
    }

    pub fn transition(
        &self,
        target: LifecycleStatus,
    ) -> Result<LifecycleStatus, Error> {
        if self.can_transition_to(&target) {
            Ok(target)
        } else {
            Err(Error::Lifecycle {
                expected: format!("valid transition from {self:?}"),
                found: format!("{target:?}"),
            })
        }
    }
}
```

#### `init.rs`

```rust
pub struct InitResult {
    pub config_path: PathBuf,
    pub schema_name: String,
    pub cache_populated: bool,
    pub directories_created: Vec<PathBuf>,
}

pub fn init(
    project_dir: &Path,
    schema_value: &str,
    schema_source_dir: &Path,
    context: Option<&str>,
) -> Result<InitResult, Error>;
```

The `init` function handles the mechanical parts (directory creation, config template, cache population, gitignore) and returns what it did so the skill can report to the user. The agent still handles the interactive parts (asking which schema, confirming reinitialize).

#### `drift.rs` (RFC-2/0003, initially stubbed)

```rust
pub struct DriftEntry {
    pub requirement_id: String,
    pub requirement_name: String,
    pub status: DriftStatus,
    pub detail: Option<String>,
}

pub enum DriftStatus {
    Covered,
    Drifted,
    Missing,
    Unspecified,
}

pub fn baseline_inventory(
    specs_dir: &Path,
) -> Result<Vec<(String, Vec<RequirementBlock>)>, Error>;
```

#### `federation.rs` (RFC-3, stubbed)

```rust
pub struct PeerRepo {
    pub name: String,
    pub repo: String,
    pub specs_path: String,
}

pub fn parse_federation_config(
    config: &ProjectConfig,
) -> Vec<PeerRepo>;
```

### CLI Subcommands (`specify-cli`)

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "specify",
    version,
    about = "Specify CLI — deterministic operations for spec-driven development"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format
    #[arg(long, default_value = "text", global = true)]
    format: OutputFormat,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize .specify/ in a project
    Init {
        /// Schema name or URL
        schema: String,
        /// Schema source directory (pre-resolved)
        #[arg(long)]
        schema_dir: PathBuf,
        /// Project context
        #[arg(long)]
        context: Option<String>,
    },

    /// Validate change artifacts against schema rules
    Validate {
        /// Change directory (.specify/changes/<name>)
        change_dir: PathBuf,
    },

    /// Merge all delta specs for a change into baseline
    Merge {
        /// Change directory
        change_dir: PathBuf,
        /// Archive after merge
        #[arg(long, default_value = "true")]
        archive: bool,
    },

    /// Show change status and task progress
    Status {
        /// Specific change name (optional)
        change: Option<String>,
    },

    /// Task operations
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Schema operations
    Schema {
        #[command(subcommand)]
        action: SchemaAction,
    },

    /// Validate the specify framework repo itself
    Check {
        /// Repository root
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
}

#[derive(Subcommand)]
enum TaskAction {
    /// Show the next pending task
    Next { change_dir: PathBuf },
    /// Mark a task complete
    Mark { change_dir: PathBuf, task_number: String },
    /// List all tasks with status
    List { change_dir: PathBuf },
}

#[derive(Subcommand)]
enum SchemaAction {
    /// Resolve a schema value to a directory path
    Resolve {
        schema_value: String,
        #[arg(long, default_value = ".")]
        project_dir: PathBuf,
    },
    /// Validate a schema.yaml file
    Check { schema_dir: PathBuf },
}
```

### Output Format

Every subcommand supports `--format text` (default, human-readable) and `--format json` (structured, for skills). The JSON output is what makes the CLI truly useful for agent consumption:

```json
{
  "passed": false,
  "blueprint_results": {
    "proposal": [
      {
        "status": "pass",
        "rule": "Has a Why section with at least one sentence"
      },
      {
        "status": "fail",
        "rule": "Has a Crates section listing at least one new or modified crate",
        "detail": "Section heading found but no crate entries below it"
      }
    ],
    "specs/oauth-handler/spec.md": [
      {
        "status": "pass",
        "rule": "Every requirement has at least one scenario"
      },
      {
        "status": "deferred",
        "rule": "Uses SHALL/MUST language for normative requirements",
        "reason": "Semantic check — requires LLM judgment"
      }
    ]
  },
  "cross_checks": [
    { "status": "pass", "rule": "proposal-crates-have-specs" },
    {
      "status": "fail",
      "rule": "design-references-valid",
      "detail": "REQ-005 referenced in design.md not found in specs"
    }
  ]
}
```

The skill prose shrinks from 40 lines of validation instructions to:

```markdown
6. **Validate artifacts**
   ```bash
   specify validate "$CHANGE_DIR" --format json
   ```
   If `passed` is false: report failures to the user and suggest fixes.
   If any results have `status: deferred`: apply your judgment for those rules.
   Do not proceed to implementation until all non-deferred checks pass.
```

### Dependencies (conservative)

```toml
# crates/specify-core/Cargo.toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1"
thiserror = "2"
regex = "1"
glob = "0.3"
chrono = { version = "0.4", features = ["serde"] }

# crates/specify-cli/Cargo.toml
[dependencies]
specify-core = { path = "../specify-core" }
clap = { version = "4", features = ["derive"] }
serde_json = "1"

# crates/specify-check/Cargo.toml
[dependencies]
specify-core = { path = "../specify-core" }
serde_json = "1"
jsonschema = "0.29"
```

No async runtime, no HTTP client, no database. The binary should compile in seconds and produce a ~5MB static binary.

### Makefile Integration

```makefile
.PHONY: build checks dev-plugins prod-plugins

build:
	cargo build --release
	cp target/release/specify .

checks: build
	./specify check --repo .
	@$(DENO) run --allow-read scripts/checks.ts  # keep during migration

dev-plugins:
	@./scripts/dev-plugins.sh

prod-plugins:
	@./scripts/prod-plugins.sh
```

During migration, both `specify check` and `checks.ts` run. As checks migrate from TypeScript to Rust, they are removed from `checks.ts` until it's empty and can be deleted.

## Alternatives Considered

**Single monolithic crate.** Simpler, but prevents reusing `specify-core` in non-CLI contexts (LSP, WASM, integration tests). The three-crate split costs almost nothing in maintenance.

**Agent-only approach (no CLI).** Continue encoding all validation and structural operations in skill prose. Rejected because LLMs are unreliable at structured decision trees — counting sections, verifying ID patterns, checking dependency graphs.

## References

- [RFC-1-A: Deferred Validation](rfc-1a-validation.md) — the three-way Pass/Fail/Deferred classification
- [RFC-2: Iterative Legacy Migration](rfc-2-migration.md) — extends the CLI with `specify migrate` subcommands
- [RFC-3: Multi-Repo Coordination](rfc-3-multi-repo.md) — extends the CLI with `specify federation` subcommands
- [RFC-4: Type-Safe Skill Expression](rfc-4-dsl.md) — extends `specify check` with skill validation

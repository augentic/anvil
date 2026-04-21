# RFC-1: `specify` CLI

> Status: Draft · Depends: — · Enables: [RFC-2](../archive/rfc-2-execution.md), [RFC-3](../rfc-3a-plan.md), [RFC-4](../rfc-4-dsl.md), [RFC-5](../rfc-5-framework-lint.md)

## Abstract

Replace prose-interpreted deterministic operations (validation, task parsing, artifact structure checking) with a Rust CLI binary (`specify`) that returns structured JSON and exit codes. The agent retains judgment; the CLI enforces correctness.

## Motivation

Every precision-critical operation — validation, task parsing, artifact structure checking — is currently performed by the LLM interpreting prose rules. This produces unreliable results for operations that are fundamentally structured decision trees.

The CLI is the foundation everything else builds on. Feature manifest commands ([RFC-2](../archive/rfc-2-execution.md)), multi-repo coordination ([RFC-3](../rfc-3a-plan.md)), and skill validation ([RFC-4](../rfc-4-dsl.md)) all require a binary that understands `.specify/` structure, spec format, and schema rules. Building the CLI first means every subsequent RFC extends an existing tool rather than creating a new one.

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

1. **Cargo workspace scaffold** — root `specify` package (`src/main.rs` + `src/lib.rs`), domain crates under `crates/`, CI integration
2. **`specify validate`** — the Pass/Fail/Deferred validation engine; replaces ~40 lines of prose validation in the build skill
3. **`specify merge`** — deterministic delta-merge replacing `merge-specs.py`
4. **`specify init`** — project initialization replacing scattered mkdir/write logic
5. **Migrate `init`, `merge`, and `build` skills** to use CLI commands
6. **`specify task`** subcommands — deterministic task parsing and progress tracking

The first four items establish a working binary with immediate value. Items 5–6 close the loop on the core workflow. The framework-linter port (`checks.ts` → `specify-check` crate, exposed as `specify check`) is tracked separately in [RFC-5](../rfc-5-framework-lint.md); it runs independently of Phase 1 and is not a prerequisite for RFC-2/RFC-3/RFC-4.

**CI for Phase 1.** The workspace-scaffold item (1) lands with a GitHub Actions workflow — `.github/workflows/ci.yml` — that runs on every pull request and on pushes to `main`. The minimum bar is three jobs against a stable Rust toolchain: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`. Unit tests cover each domain crate (`specify-schema`, `specify-spec`, `specify-merge`, `specify-task`, `specify-validate`, `specify-change`) directly; the root `specify` package adds at least one end-to-end integration test under `tests/` that invokes the built `specify validate` binary against a fixture change directory (e.g. `tests/fixtures/`) and asserts the JSON output matches a golden file (pinning the `schema_version: 1` contract documented in the "Output Format" section). `scripts/checks.ts` continues to run via `make checks` alongside this workflow until `specify check` reaches parity — see [RFC-5](../rfc-5-framework-lint.md).

#### Phase 2: Feature manifest extensions ([RFC-2](../archive/rfc-2-execution.md))

7. **`specify manifest init`** — scaffold `manifest.yaml` from a feature list or legacy codebase scan
8. **`specify manifest next`** — select the next pending feature from the manifest (respecting `depends-on`)
9. **`specify manifest status`** — track initiative progress across iterations
10. **Feature recommender** — analyse legacy dependency graph and suggest feature ordering (migration mode)
11. **Behavioural diff** — compare legacy fixture output against new implementation output (migration mode)

These build on the existing `/spec:extract`, `wiretapper`, `replay-writer`, and core `/spec:*` skills. See [RFC-2](../archive/rfc-2-execution.md) for the full design.

#### Phase 3: Federation extensions ([RFC-3](../rfc-3a-plan.md))

12. **Federation config** and `specify federation sync` for multi-repo
13. **Cross-repo spec references** and `specify federation validate`

See [RFC-3](../rfc-3a-plan.md) for the full design.

### Impact on Existing Skills

| Skill    | Current agent-interpreted logic                           | Moves to CLI                                 |
| -------- | --------------------------------------------------------- | -------------------------------------------- |
| `init`   | mkdir, file creation, schema resolution, cache population | `specify init`                               |
| `define` | Schema resolution, metadata writes, overlap detection     | `specify schema resolve`, `specify status`   |
| `build`  | Artifact validation, task progress tracking               | `specify validate`, `specify task progress/mark` |
| `merge`  | merge-specs.py invocation, coherence check, archive move  | `specify merge`                              |
| `verify` | Spec parsing, requirement extraction                      | `specify diff`                               |
| `status` | Metadata + task parsing                                   | `specify status`                             |

### Workspace Layout

The CLI lives at the repo root as a Cargo workspace with a root package named `specify`. This keeps it alongside the plugins and schemas it operates on — important because integration tests can reference the real `schemas/` directory and because the framework linter added by [RFC-5](../rfc-5-framework-lint.md) needs to validate the repo's own schema files and skills from the same workspace.

The root `specify` package produces the user-facing binary and hosts top-level orchestration; domain logic is factored into focused crates under `crates/`.

```
specify/                              # repo root (already exists)
├── Cargo.toml                        # workspace manifest + root `specify` package
├── Cargo.lock
├── src/                              # root `specify` package
│   ├── main.rs                       # clap dispatch; calls into lib.rs
│   └── lib.rs                        # top-level specify logic: init
│                                     # orchestration, project.yaml
│                                     # handling, curated public API,
│                                     # re-exports from domain crates
├── crates/                           # app logic organised by domain
│   ├── specify-error/                # unified error types (thiserror)
│   ├── specify-schema/               # schema.yaml parsing + composition,
│   │                                 # brief frontmatter, PipelineView,
│   │                                 # cache-meta
│   ├── specify-spec/                 # spec format parsing (requirement
│   │                                 # blocks, scenarios, delta sections)
│   ├── specify-merge/                # deterministic delta-merge
│   │                                 # (replaces merge-specs.py)
│   ├── specify-task/                 # task parsing + mark_complete
│   ├── specify-validate/             # hardcoded rule registry + runner
│   ├── specify-change/               # .metadata.yaml lifecycle state
│   │                                 # machine
│   ├── specify-drift/                # spec-vs-code drift scaffolding
│   │                                 # (RFC-2, stubbed)
│   └── specify-federation/           # multi-repo coordination
│                                     # (RFC-3, stubbed)
├── tests/                            # end-to-end tests for the `specify`
│   └── fixtures/                     # binary + golden JSON outputs
├── plugins/                          # existing — unchanged
├── schemas/                          # existing — unchanged
├── scripts/                          # existing — checks.ts continues to run in CI
└── Makefile                          # updated with new targets
```

Dependencies flow from leaves to root: `specify-error` has no internal deps; `specify-schema` / `specify-spec` / `specify-task` depend on `specify-error`; `specify-merge` depends on `specify-spec`; `specify-validate` depends on `specify-schema`, `specify-spec`, and `specify-task`; `specify-change` depends on `specify-error`. The root `specify` package depends on every domain crate and wires them together. This shape avoids cycles while still letting each crate be tested in isolation.

[RFC-5](../rfc-5-framework-lint.md) adds a `specify-check` crate under `crates/` when the framework-linter port begins; it is deliberately out of scope here.

### Why a Root Package Plus Domain Crates

**Domain crates under `crates/`** hold pure logic. They have no CLI concerns — no argument parsing, no terminal formatting, no exit codes. Each returns `Result<T, specify_error::Error>` from every public function. This matters because:

1. Skills that invoke the CLI get structured output (JSON). But the logic may also be called from other contexts — a future LSP for schema validation in editors, a WASM build for browser-based tooling, or integration tests that call a single crate directly.
2. The merge logic, spec parser, validator, and lifecycle state machine are independently testable without spawning processes — and without pulling in every other domain.
3. Splitting by domain (rather than dumping everything into one `specify-core` blob) keeps compile times, dependency graphs, and ownership boundaries cleaner as the surface grows (drift, federation, manifest — see RFC-2 and RFC-3).

**The root `specify` package** (`src/main.rs` + `src/lib.rs`) owns the user-facing binary and the glue between domain crates. `main.rs` is a thin clap dispatcher — each subcommand is ~20 lines that parse args, call into `lib.rs` (or a domain crate directly), format the result, and set the exit code. `lib.rs` is the home for top-level specify logic: `init` orchestration (which touches schema resolution, project.yaml writing, and lifecycle creation at once), project config parsing, and the curated public API that embedders (editors, future LSP, CI integrations) consume. Putting `main.rs` and `lib.rs` side-by-side at the root — rather than in a separate `specify-cli` crate — means a single `cargo install specify` produces the binary and a single `use specify::…` imports the embeddable surface.

[RFC-5](../rfc-5-framework-lint.md) adds a `specify-check` crate when the framework-linter port begins. It reuses `specify-schema`'s schema and brief parsers but keeps the repo-specific check logic (symlink resolution, SKILL.md frontmatter, docs inventory) out of the runtime crates.

### Module Design: Domain Crates

#### `error.rs` — `crates/specify-error`

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not initialized: .specify/project.yaml not found")]
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

    #[error("specify version {found} is older than the project floor {required}; upgrade the CLI")]
    SpecifyVersionTooOld { required: String, found: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
}
```

A single error type with structured variants means the CLI can pattern-match on the variant to decide exit codes and output format, and the library never touches `std::process::exit`.

#### `config.rs` — `src/lib.rs`

Models the real `.specify/project.yaml` shape written by the `init` skill: a thin overlay keyed by `name`, `domain`, `schema`, `specify_version` (the CLI version floor — see the "CLI Distribution and Fallback" section below), and `rules` (one entry per `pipeline.define` brief, each pointing to an optional markdown file with project-specific rules).

```rust
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectConfig {
    /// Project name (defaults to the project directory name at init time).
    pub name: String,

    /// Free-text description of the project's tech stack, architecture,
    /// and testing approach. Falls back to `schema.domain` when empty.
    #[serde(default)]
    pub domain: Option<String>,

    /// Schema identifier — either a bare name (`omnia`) or a URL.
    pub schema: String,

    /// Minimum `specify` CLI version required to operate on this project.
    /// Written by `specify init` as the running binary's version. Every
    /// subcommand that reads `project.yaml` compares its own version
    /// against this field and refuses to run if it is older (exit code 3,
    /// JSON error `{"error": "specify_version_too_old", ...}`). Skills
    /// surface the upgrade instruction verbatim.
    #[serde(default)]
    pub specify_version: Option<String>,

    /// Map of brief id (e.g. `proposal`, `specs`, `design`, `tasks`) to a
    /// path (relative to `.specify/`) of a markdown file containing extra
    /// rules for that brief. Scaffolded with one empty entry per
    /// `pipeline.define` brief by `specify init`; empty values mean "no
    /// override — use the schema brief as-is."
    #[serde(default)]
    pub rules: BTreeMap<String, String>,
}

impl ProjectConfig {
    /// Load `.specify/project.yaml`, returning `Error::NotInitialized`
    /// if the file is missing.
    pub fn load(project_dir: &Path) -> Result<Self, Error>;

    /// Absolute path to `.specify/project.yaml`.
    pub fn config_path(project_dir: &Path) -> PathBuf;

    pub fn specify_dir(project_dir: &Path) -> PathBuf;
    pub fn changes_dir(project_dir: &Path) -> PathBuf;
    pub fn specs_dir(project_dir: &Path) -> PathBuf;
    pub fn cache_dir(project_dir: &Path) -> PathBuf;

    /// Resolve a `rules` value to an absolute path under `.specify/`.
    /// Returns `None` if the brief has no override.
    pub fn rule_path(&self, project_dir: &Path, brief_id: &str) -> Option<PathBuf>;
}
```

The path helpers centralise the `.specify/changes/`, `.specify/specs/`, `.specify/.cache/` conventions currently scattered across every skill. `rule_path` is the single source of truth for resolving project rule overrides — skills read it rather than reconstructing the path themselves.

#### `schema.rs` — `crates/specify-schema`

The most important module — it encodes the resolution algorithm from `schema-resolution.md` and models the real `schema.yaml` shape defined by `schemas/schema.schema.json`.

```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct Schema {
    pub name: String,
    pub version: u32,
    pub description: String,
    #[serde(default)]
    pub extends: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    pub pipeline: Pipeline,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Pipeline {
    pub define: Vec<PipelineEntry>,
    pub build: Vec<PipelineEntry>,
    pub merge: Vec<PipelineEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PipelineEntry {
    /// Brief identifier (e.g. `proposal`, `specs`, `design`, `tasks`).
    pub id: String,
    /// Relative path (from the schema root) to the brief markdown file.
    pub brief: String,
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

    /// Validate schema.yaml structure against the embedded JSON Schema
    /// (`schemas/schema.schema.json`).
    pub fn validate_structure(&self) -> Vec<ValidationResult>;

    /// Iterator over every pipeline entry in execution order
    /// (define → build → merge), paired with its phase.
    pub fn entries(&self) -> impl Iterator<Item = (Phase, &PipelineEntry)>;

    /// Look up a pipeline entry by `id` across all phases.
    pub fn entry(&self, id: &str) -> Option<(Phase, &PipelineEntry)>;

    /// Merge a child schema on top of a parent (composition via `extends`).
    /// Child pipeline entries with the same `id` replace parent entries;
    /// new ids are appended in order.
    pub fn merge(parent: Schema, child: Schema) -> Schema;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Phase {
    Define,
    Build,
    Merge,
}
```

Per-brief metadata (`description`, `generates`, `needs`, `tracks`) is *not* part of the schema — it lives as YAML frontmatter on the brief markdown files referenced by `PipelineEntry.brief`. Parsing that frontmatter is the job of the `brief.rs` module described below; `schema.rs` deliberately stops at the pipeline-shape boundary so the two concerns can be tested independently.

Note the absence of any HTTP fetching — the `resolve` function handles local and cache paths. Remote fetching (the WebFetch step in the current skill) remains the agent's responsibility. The CLI's `specify schema resolve` subcommand outputs the resolved path so the skill knows where to find files, but the agent does the HTTP fetch if the cache is stale. This keeps the CLI dependency-free for networking and avoids duplicating the agent's authenticated GitHub access.

**Cache-write ownership (M10).** The agent owns every write to `.specify/.cache/` — fetched schema files, `briefs/*`, and `.cache-meta.yaml`. The CLI only reads the cache (via `Schema::resolve` and `CacheMeta::load` below). This asymmetry is deliberate: the agent already has the HTTP client, credential handling, and retry logic for authenticated GitHub access, and the CLI stays dependency-free on the networking side. The trade-off is that the cache format becomes a cross-boundary contract — the agent writes it, the CLI parses it — so the format is pinned in `specify-schema` (see `CacheMeta` below) and published as JSON Schema at `schemas/cache-meta.schema.json` so skill prose can cite one source of truth rather than restating the format inline. This decision is revisitable: if agent-side cache writes turn out to be error-prone in practice, we can move to a `specify schema fetch <url>` subcommand that accepts pre-fetched bytes on stdin and writes the cache itself. For Phase 1, agent-owned writes keep the CLI surface smaller.

```rust
/// On-disk metadata describing the contents of `.specify/.cache/`.
///
/// Written by skills (the agent) whenever they populate or refresh the
/// cache; read by the CLI during schema resolution to decide whether the
/// cache matches the `schema` value in `project.yaml`.
///
/// The on-disk form is `.specify/.cache/.cache-meta.yaml`. The same shape
/// is published as JSON Schema at `schemas/cache-meta.schema.json` and is
/// embedded into `specify-schema` at build time for `specify schema check`.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct CacheMeta {
    /// The `schema` value the cache was populated from.
    /// - Bare-name schemas (no `/`): `local:<name>`, e.g. `local:omnia`.
    /// - URL-based schemas: the full URL including `@ref` when present,
    ///   e.g. `https://github.com/augentic/specify/schemas/omnia@v1`.
    pub schema_url: String,

    /// ISO-8601 timestamp (UTC) of when the agent wrote the cache.
    pub fetched_at: String,
}

impl CacheMeta {
    /// Absolute path to `.specify/.cache/.cache-meta.yaml`.
    pub fn path(project_dir: &Path) -> PathBuf;

    /// Load `.cache-meta.yaml`, returning `Ok(None)` if the file is
    /// absent (cache empty) and `Err` if it exists but is malformed.
    pub fn load(project_dir: &Path) -> Result<Option<Self>, Error>;

    /// Validate an in-memory `CacheMeta` against the embedded JSON
    /// Schema (`schemas/cache-meta.schema.json`). Surfaced through
    /// `specify schema check` so CI can spot agent-side drift.
    pub fn validate_structure(&self) -> Vec<ValidationResult>;

    /// True when the cache on disk was populated from `schema_value`.
    /// Consumed by `Schema::resolve` to decide between `SchemaSource::Cached`
    /// and stale-cache signalling.
    pub fn matches(&self, schema_value: &str) -> bool;
}
```

There is no `CacheMeta::write` on the CLI side — the type is read-only from `specify-schema`'s perspective. The serialized form is intentionally minimal (two scalar fields); future additions (e.g. an `etag` for conditional refetch) are additive and bump neither the outer JSON `schema_version` nor the cache-meta JSON Schema's major version, provided existing fields retain their semantics.

#### `brief.rs` — `crates/specify-schema`

Every define/build/merge/status flow drives off brief frontmatter: `specify validate` needs `generates` globs to know which artifacts to expect, `specify status` needs `needs` to decide which briefs are ready, `specify task` uses `tracks` to locate the task list, and `specify merge` reads `generates` to locate delta-spec files. `brief.rs` is the single parser for that frontmatter, sitting alongside `schema.rs`.

```rust
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Parsed frontmatter of a brief markdown file.
///
/// Only the frontmatter fields are modelled here; the prose body is kept
/// separately as a `String` so downstream consumers (e.g. the agent) can
/// render it without reparsing.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct BriefFrontmatter {
    /// Brief identifier — must match the `PipelineEntry.id` that references
    /// this brief (enforced by `PipelineView::load`).
    pub id: String,

    /// One-line description of the brief's purpose.
    pub description: String,

    /// Filename (or glob, relative to the change directory) this brief
    /// produces when it runs. `None` for briefs that don't generate an
    /// artifact (e.g. `build` in the Omnia schema — it tracks tasks
    /// rather than generating a file).
    #[serde(default)]
    pub generates: Option<String>,

    /// Brief ids this brief depends on. Referenced ids must exist in the
    /// same schema pipeline; `PipelineView::load` validates this.
    #[serde(default)]
    pub needs: Vec<String>,

    /// Id of a brief whose `generates` artifact this brief tracks
    /// progress against (currently only used by `build` → `tasks`).
    #[serde(default)]
    pub tracks: Option<String>,
}

/// A brief = its frontmatter + the markdown body + the file it was parsed from.
#[derive(Debug)]
pub struct Brief {
    pub path: PathBuf,
    pub frontmatter: BriefFrontmatter,
    pub body: String,
}

impl Brief {
    /// Parse a brief markdown file. Splits `---`-delimited YAML
    /// frontmatter from the body, deserializes the frontmatter, and
    /// returns both halves.
    pub fn load(path: &Path) -> Result<Self, Error>;

    /// Parse from an in-memory string (useful for tests and for briefs
    /// that originate from `.specify/.cache/`).
    pub fn parse(path: &Path, contents: &str) -> Result<Self, Error>;
}
```

Resolution is done through a `PipelineView` helper that most subcommands consume. It pairs a resolved schema with every brief its pipeline references, validating cross-references along the way:

```rust
/// The fully-resolved view of a schema: pipeline shape + every brief
/// referenced by it.
pub struct PipelineView {
    pub schema: ResolvedSchema,
    /// One entry per `PipelineEntry`, in pipeline order
    /// (define → build → merge).
    pub briefs: Vec<(Phase, Brief)>,
}

impl PipelineView {
    /// Resolve the schema (via `Schema::resolve`), then load every brief
    /// referenced by `pipeline.{define,build,merge}` from the schema's
    /// root directory.
    ///
    /// Validation performed here:
    /// 1. Every `PipelineEntry.brief` path exists and parses.
    /// 2. `Brief.frontmatter.id` equals the referencing `PipelineEntry.id`.
    /// 3. Every `needs` id refers to a brief that appears earlier in
    ///    pipeline order.
    /// 4. Every `tracks` id refers to a brief in the same schema.
    pub fn load(
        schema_value: &str,
        project_dir: &Path,
    ) -> Result<Self, Error>;

    /// Lookup by brief id.
    pub fn brief(&self, id: &str) -> Option<&Brief>;

    /// Iterator over briefs in a single phase.
    pub fn phase(&self, phase: Phase) -> impl Iterator<Item = &Brief>;
}
```

Almost every other subcommand is written against `PipelineView` rather than `Schema` directly:

- `specify validate` iterates `view.phase(Phase::Build)` to know which artifacts to look for (using `brief.frontmatter.generates`) and which upstream briefs must have run (`brief.frontmatter.needs`).
- `specify status` reports per-brief completion by checking whether each `generates` artifact exists in the change directory.
- `specify task progress` and `specify task mark` use `view.brief("build")?.frontmatter.tracks` to resolve the task list brief id, then read its `generates` to find `tasks.md`.
- `specify merge` uses `view.phase(Phase::Merge)` and inspects each brief's `generates` to know which delta-spec files to merge.

Keeping this logic in one place means the brief-format contract (what fields exist, what their semantics are) lives next to the parser instead of being reimplemented ad-hoc in each subcommand.

#### `spec.rs` — `crates/specify-spec`

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

#### `merge.rs` — `crates/specify-merge`

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

/// Atomic multi-spec merge plus archive. Takes a change directory,
/// merges every spec into its baseline, flips `.metadata.yaml.status`
/// to `merged`, and moves the change into `archive/YYYY-MM-DD-<name>/`.
///
/// Transactional: every merged baseline is computed in memory first; nothing
/// is written to disk until all specs merge cleanly and coherence
/// checks pass. Any failure before the commit point returns an error with
/// the filesystem untouched.
pub fn merge_change(
    change_dir: &Path,
    specs_dir: &Path,
    archive_dir: &Path,
) -> Result<Vec<(String, MergeResult)>, Error>;
```

The merge algorithm is a direct port of `merge-specs.py` with three improvements:

1. **Structured output.** Instead of writing to stdout, it returns `MergeResult` with the merged text and a log of operations. The CLI formats this as JSON for skills or as human-readable text for direct invocation.
2. **Atomic multi-spec merge.** The current skill runs `merge-specs.py` once per spec (one per `specs/<capability>/` directory). The library function `merge_change` takes a change directory and merges all specs in memory; only once every spec merges cleanly and passes coherence validation does it write baselines to disk.
3. **Bundled archive step.** After the in-memory merge commits, `merge_change` flips `.metadata.yaml.status` to `merged` and moves the change directory to `archive/YYYY-MM-DD-<name>/` as part of the same command. A separate `specify archive` is not required — the skill prose's four-step sequence (merge, coherence check, status flip, archive move) is a single CLI invocation.

#### `task.rs` — `crates/specify-task`

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

/// Idempotent: marking an already-complete task returns the input unchanged.
pub fn mark_complete(
    content: &str,
    task_number: &str,
) -> Result<String, Error>;
```

Phase 1 ships only `parse_tasks` (powering `specify task progress`, which returns the `total`/`complete` counts) and `mark_complete` (powering `specify task mark`). The `build/SKILL.md` loop drives task selection from prose; a `next_pending` helper and a `specify task list` / `specify task next` surface are deferred until a consumer actually needs them.

#### `validate.rs` — `crates/specify-validate`

Validation rules are **hardcoded per brief type** (`proposal`, `specs`, `design`, `tasks`) in `specify-validate`. Neither `schema.yaml` nor brief frontmatter carries a `validation:` section today — the rule set is small enough that a Rust registry is the simpler path, and it lets the type system enforce rule ids against the checker implementations. The registry is keyed by brief `id`, so any schema whose pipeline reuses the built-in brief ids (`omnia`, `vectis`, and extensions built on them) picks up validation automatically without modifying `schema.yaml`.

The CLI handles the *structural* rules deterministically and flags the *semantic* ones for the agent. See [RFC-1-A: Deferred Validation](rfc-1a-validation.md) for the classification model.

```rust
pub enum ValidationResult {
    Pass { rule_id: &'static str, rule: &'static str },
    Fail { rule_id: &'static str, rule: &'static str, detail: String },
    Deferred { rule_id: &'static str, rule: &'static str, reason: &'static str },
}

pub struct ValidationReport {
    /// Keyed by brief `id` (e.g. `proposal`, `tasks`) or by the generated
    /// artifact path for briefs that produce multiple files
    /// (e.g. `specs/oauth-handler/spec.md`).
    pub brief_results: BTreeMap<String, Vec<ValidationResult>>,
    pub cross_checks: Vec<ValidationResult>,
    pub passed: bool,
}

/// Run all deterministic validations for a change. Uses the `PipelineView`
/// to know which briefs are in scope, what their `generates` artifacts are,
/// and which `needs` must have run upstream; rules themselves are looked up
/// by brief id from the hardcoded registry.
pub fn validate_change(
    change_dir: &Path,
    pipeline: &PipelineView,
) -> ValidationReport;
```

The key design decision: rules that the CLI can check deterministically (heading structure, ID format, checkbox format, section existence) produce `Pass` or `Fail`. Rules that require semantic judgment (like "Uses SHALL/MUST language for normative requirements") produce `Deferred` with an explanation. The skill prose only needs to handle deferred rules.

The rule registry is a compile-time table keyed by brief id. Each entry pairs a stable `rule_id` (for CLI consumers pinning behaviour), a human-readable description (preserved in the JSON output so skill prose still reads naturally), an explicit classification, and the checker function:

```rust
pub enum Classification {
    /// The CLI can decide Pass/Fail deterministically.
    Structural,
    /// The CLI always emits `Deferred`; the agent applies judgment.
    Semantic,
}

pub enum RuleOutcome {
    Pass,
    Fail { detail: String },
}

pub struct Rule {
    pub id: &'static str,
    pub description: &'static str,
    pub classification: Classification,
    /// Only invoked for `Classification::Structural`.
    pub check: fn(&BriefContext<'_>) -> RuleOutcome,
}

/// Inputs a structural checker needs: the brief's source text, the parsed
/// spec (when the brief generates a spec file), task progress (when the
/// brief generates `tasks.md`), and paths for cross-brief lookups.
pub struct BriefContext<'a> {
    pub brief_id: &'a str,
    pub content: &'a str,
    pub parsed_spec: Option<&'a ParsedSpec>,
    pub tasks: Option<&'a TaskProgress>,
    pub change_dir: &'a Path,
    pub specs_dir: &'a Path,
    pub terminology: &'a str, // e.g. "crate" (omnia) or "feature" (vectis)
}

/// Built-in rules keyed by brief id. Empty slice if the brief id is unknown;
/// `validate_change` then falls back to schema-level generic rules (every
/// artifact must exist and parse).
pub fn rules_for(brief_id: &str) -> &'static [Rule];

/// Cross-brief checks that don't belong to any single brief (e.g. proposal
/// deliverables have matching spec files).
pub fn cross_rules() -> &'static [CrossRule];
```

The primitives the checker functions compose from are unchanged from the previous draft; they are now `pub(crate)` helpers invoked by named rules in `rules_for`:

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

The initial registry covers the four brief types the repo ships today (`proposal`, `specs`, `design`, `tasks`); each entry is a hand-written `Rule` that pairs one of the helpers above (or a small composition) with a stable `rule_id`. Adding a new brief type means adding a `rules_for` arm and the accompanying checkers — no schema change required.

**Extension point toward per-brief frontmatter rules.** The alternative (option (b) in the remediation notes) is to let briefs declare their own structural rules in YAML frontmatter, with the Rust registry acting as the default when none are declared. The shape above supports that without rework: a future `BriefFrontmatter::validations: Vec<ValidationRef>` field would be merged with `rules_for(brief.id)` at `validate_change` time, with frontmatter entries referencing rule ids from the registry (so the actual check code still lives in Rust and stays type-checked). Adopting (b) therefore becomes additive whenever a schema needs per-brief overrides; it is explicitly out of scope for Phase 1 because no current schema needs it.

#### `metadata.rs` — `crates/specify-change`

```rust
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
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

**Full transition graph.** Creation is modeled as an edge from a `START` pseudo-node (there is no "from" state when a change directory first gets a `.metadata.yaml`), so the state machine is:

```text
START ──► Defining ──► Defined ──► Building ──► Complete ──► Merged  (terminal)
            │   ▲          │
            │   │ force-   │
            │   │ reset    │
            │   └──────────┘
            │
            │  extract --baseline
            └──────────────────────────────────► Complete
                                                   (init-scaffolded
                                                    baseline only)

(any non-terminal) ───────── drop ─────────────► Dropped (terminal)
```

Edges, annotated with the command that drives them:

| From       | To         | Driver                                                                 |
|------------|------------|------------------------------------------------------------------------|
| `START`    | `Defining` | `specify init` (fresh + `initial-baseline`), `specify define`          |
| `Defining` | `Defined`  | `specify define` (completes artifacts)                                 |
| `Defined`  | `Defining` | `specify define --force-reset` (re-enter the define phase)             |
| `Defined`  | `Building` | `specify build` (first task started)                                   |
| `Building` | `Complete` | `specify build` (last task completed)                                  |
| `Complete` | `Merged`   | `specify merge`                                                        |
| `Defining` | `Complete` | `specify extract --baseline` (init-scaffolded baseline change only)    |
| any non-terminal | `Dropped` | `specify drop`                                                    |

`Merged` and `Dropped` are terminal — they have no outgoing edges, and every skill that reads `.metadata.yaml` must refuse to mutate a change in either state. Reopening a merged or dropped change is explicitly out of scope for Phase 1; the sanctioned workflow is `specify define` to create a fresh change. Likewise, `Building → Defined` and `Complete → Building` are intentionally absent — if implementation surfaces a design gap, the user either runs `define --force-reset` (before `Building`) or drops and recreates.

The `initial()` constructor is the single legal way to enter `Defining` from outside the machine; in-place moves go through `can_transition_to` / `transition`:

```rust
impl LifecycleStatus {
    /// Creation edge (`START → Defining`). The only legal initial status
    /// for a newly-scaffolded change. Called by `init` and `define`.
    pub fn initial() -> Self {
        LifecycleStatus::Defining
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, LifecycleStatus::Merged | LifecycleStatus::Dropped)
    }

    pub fn can_transition_to(&self, target: &Self) -> bool {
        use LifecycleStatus::*;
        matches!(
            (self, target),
            (Defining, Defined)
                | (Defined, Defining)    // define --force-reset
                | (Defined, Building)
                | (Building, Complete)
                | (Complete, Merged)
                | (Defining, Complete)   // extract --baseline (init flow)
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

The `Defining → Complete` edge is narrow by design: it exists solely so the init-triggered baseline extraction path (`init` scaffolds an `initial-baseline` change at `Defining`, then the agent runs `specify extract --baseline` to populate specs from the existing codebase, then `specify merge` archives it) has a sanctioned route through the state machine without inventing a "this change skipped define/build" flag. Non-baseline uses of `extract` don't touch status — they're read-only analyzers.

#### `init.rs` — `src/lib.rs`

```rust
pub struct InitOptions<'a> {
    pub project_dir: &'a Path,
    pub schema_value: &'a str,
    /// Directory the CLI reads to discover `pipeline.define` briefs for
    /// scaffolding `rules:`. By convention the agent populates this
    /// under `.specify/.cache/` (see "Cache-write ownership" above)
    /// before invoking `specify init`, but any readable schema root
    /// works — the CLI never writes into it.
    pub schema_source_dir: &'a Path,
    /// Project name; defaults to the project directory name when `None`.
    pub name: Option<&'a str>,
    /// Optional project domain description (tech stack, architecture,
    /// testing approach). Written into `.specify/project.yaml` as-is.
    pub domain: Option<&'a str>,
    /// Mode for the `specify_version` floor in `project.yaml`.
    pub version_mode: VersionMode,
}

pub enum VersionMode {
    /// Write the running binary's version as the new floor.
    /// Used by fresh `init` and by `init --upgrade`.
    WriteCurrent,
    /// Preserve the existing `specify_version` in `project.yaml`
    /// (reinitialize flow).
    Preserve,
}

pub struct InitResult {
    pub config_path: PathBuf,
    pub schema_name: String,
    /// True when `.specify/.cache/.cache-meta.yaml` was observed at
    /// `init` time — the agent populates the cache before invoking
    /// `specify init`, and the CLI reports whether it was present so
    /// skills can warn when the agent skipped cache population.
    pub cache_present: bool,
    pub directories_created: Vec<PathBuf>,
    /// Brief ids scaffolded into `rules:` (one per `pipeline.define` entry).
    pub scaffolded_rule_keys: Vec<String>,
    /// Value written (or preserved) in `project.yaml.specify_version`.
    pub specify_version: String,
}

pub fn init(opts: InitOptions<'_>) -> Result<InitResult, Error>;
```

The `init` function handles the mechanical parts (directory creation, `project.yaml` template, `.gitignore` upkeep) and returns what it did so the skill can report to the user. It reads the resolved schema's `pipeline.define` entries and scaffolds a `rules:` key per brief with an empty value (the "no override" signal consumed by `ProjectConfig::rule_path`). It also stamps `project.yaml.specify_version` per `version_mode` — `WriteCurrent` writes `env!("CARGO_PKG_VERSION")`, `Preserve` keeps whatever the existing file holds (used by the reinitialize flow when the user wants to refresh only the schema cache without bumping the CLI floor). The agent still handles the interactive parts (asking which schema, confirming reinitialize, prompting for the domain description) and — per the cache-write ownership decision above — writes the schema files and `.cache-meta.yaml` into `.specify/.cache/` before invoking `specify init`; the CLI never copies schema bytes into the cache itself.

#### `drift.rs` — `crates/specify-drift` (RFC-2/0003, initially stubbed)

```rust
#[serde(rename_all = "kebab-case")]
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

#### `federation.rs` — `crates/specify-federation` (RFC-3, stubbed)

```rust
#[serde(rename_all = "kebab-case")]
pub struct PeerRepo {
    pub name: String,
    pub repo: String,
    pub specs_path: String,
}

pub fn parse_federation_config(
    config: &ProjectConfig,
) -> Vec<PeerRepo>;
```

### CLI Subcommands (`src/main.rs`)

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
        /// Project name (defaults to the project directory name)
        #[arg(long)]
        name: Option<String>,
        /// Project domain description (tech stack, architecture, testing)
        #[arg(long)]
        domain: Option<String>,
        /// Rewrite `specify_version` in `project.yaml` to the running
        /// binary's version. Used to bump the CLI floor after a user-
        /// driven upgrade.
        #[arg(long)]
        upgrade: bool,
    },

    /// Validate change artifacts against schema rules
    Validate {
        /// Change directory (.specify/changes/<name>)
        change_dir: PathBuf,
    },

    /// Merge all delta specs for a change into baseline and archive the change
    Merge {
        /// Change directory
        change_dir: PathBuf,
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
}

#[derive(Subcommand)]
enum TaskAction {
    /// Report task completion counts (total, complete, pending)
    Progress { change_dir: PathBuf },
    /// Mark a task complete (idempotent — no-op if already complete)
    Mark { change_dir: PathBuf, task_number: String },
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
  "schema_version": 1,
  "passed": false,
  "brief_results": {
    "proposal": [
      {
        "status": "pass",
        "rule_id": "proposal.why-has-content",
        "rule": "Has a Why section with at least one sentence"
      },
      {
        "status": "fail",
        "rule_id": "proposal.crates-listed",
        "rule": "Has a Crates section listing at least one new or modified crate",
        "detail": "Section heading found but no crate entries below it"
      }
    ],
    "specs/oauth-handler/spec.md": [
      {
        "status": "pass",
        "rule_id": "specs.requirements-have-scenarios",
        "rule": "Every requirement has at least one scenario"
      },
      {
        "status": "deferred",
        "rule_id": "specs.uses-normative-language",
        "rule": "Uses SHALL/MUST language for normative requirements",
        "reason": "Semantic check — requires LLM judgment"
      }
    ]
  },
  "cross_checks": [
    {
      "status": "pass",
      "rule_id": "cross.proposal-crates-have-specs",
      "rule": "Every crate listed in the proposal has a matching spec file"
    },
    {
      "status": "fail",
      "rule_id": "cross.design-references-valid",
      "rule": "Every requirement id referenced in design.md exists in specs",
      "detail": "REQ-005 referenced in design.md not found in specs"
    }
  ]
}
```

`rule_id` is a stable identifier and `rule` is the human-readable description suitable for rendering to the user. Skills that need to pin behaviour against specific rules (for instance, waiving a particular failure) should key off `rule_id`, not the prose description.

#### JSON Contract Versioning

Every top-level JSON response includes `"schema_version": <integer>`. Phase 1 ships `schema_version: 1`. The CLI commits to semver for the JSON contract:

- **Patch / additive changes** (no version bump): adding a new optional field; adding a new `rule_id` or `status` variant that skills already handle via the existing enum; adding a new subcommand.
- **Major bump** (`schema_version: 2`, …): removing or renaming a field; changing the semantics of an existing field; tightening a type (e.g. making a previously-optional field required); repurposing a `rule_id`.

The `schema_version` field lives on the outermost object returned by every subcommand — not nested per-section — so skills can validate it with a single check. Skills that parse JSON output should:

1. Read `schema_version` first.
2. If it is greater than the version the skill was written against, fail loudly with a message asking the user to upgrade the skill bundle rather than silently proceeding against an unknown shape.
3. If it is less than the expected version, the skill may either fall back to best-effort parsing or hard-fail with an instruction to upgrade the `specify` binary — choice is per-skill.

Unknown `rule_id` values within a known `schema_version` are not a contract break; skills must ignore rule ids they do not recognise and surface them to the user as informational entries.

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

Shared dependencies (pulled in by every domain crate that needs them):

```toml
# common across crates/specify-* — wired per crate as needed
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1"
thiserror = "2"
regex = "1"
glob = "0.3"
chrono = { version = "0.4", features = ["serde"] }
```

Root package (the `specify` binary):

```toml
# Cargo.toml (workspace root + root package)
[package]
name = "specify"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "specify"
path = "src/main.rs"

[lib]
name = "specify"
path = "src/lib.rs"

[dependencies]
specify-error       = { path = "crates/error" }
specify-schema      = { path = "crates/schema" }
specify-spec        = { path = "crates/spec" }
specify-merge       = { path = "crates/merge" }
specify-task        = { path = "crates/task" }
specify-validate    = { path = "crates/validate" }
specify-change      = { path = "crates/change" }
specify-drift       = { path = "crates/drift" }
specify-federation  = { path = "crates/federation" }
clap                = { version = "4", features = ["derive"] }
serde_json          = "1"

[workspace]
members = ["crates/*"]
```

No async runtime, no HTTP client, no database. The binary should compile in seconds and produce a ~5MB static binary.

### Makefile Integration

```makefile
.PHONY: build dev-plugins prod-plugins

build:
	cargo build --release
	cp target/release/specify .

dev-plugins:
	@./scripts/dev-plugins.sh

prod-plugins:
	@./scripts/prod-plugins.sh
```

The `make checks` target remains driven by `scripts/checks.ts` for now; its migration to `specify check` is covered by [RFC-5](../rfc-5-framework-lint.md) and does not interact with the Phase 1 build target above.

### CLI Distribution and Fallback

Downstream projects need a deterministic story for getting the `specify` binary, and skills need a defined failure mode when it is absent.

**Install paths (ranked from most to least preferred for end users):**

1. **`brew install specify`.** Homebrew is the primary install path for the macOS and Linux developer audience Specify targets. A tap (e.g. `augentic/tap`) is maintained from this repo and updated on every tagged release; once the formula reaches `homebrew-core`, the tap step goes away. `brew install` gives users standard `brew upgrade` / `brew uninstall` semantics, auto-handles PATH, and keeps the binary current without manual re-runs of a curl script.
2. **`cargo install specify`.** For users with a Rust toolchain, the root package (`specify`, producing the binary of the same name) is published to crates.io alongside every tagged release. This is the supported path for contributors and for CI images that already carry `cargo`.
3. **Pre-built release binaries from GitHub Releases.** Each tagged release publishes static binaries for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-msvc`. An install script (`curl -sSf https://specify.sh/install.sh | sh` or equivalent, TBD as part of Phase 1) drops the binary onto `$PATH`. This is the fallback for environments where neither Homebrew nor a Rust toolchain is available (restricted images, Windows, air-gapped installs).
4. **`make build` (from a local checkout).** Builds and copies the binary to the repo root. This is the developer path — it also backs `make dev-plugins` for contributors iterating on the CLI itself. Not intended for end users.

`apt`, `dnf`, and similar OS-native package managers are deferred — the ordering above covers the target audience until Linux-distro usage justifies the packaging overhead.

**Skill fallback when the binary is missing.** Skills that invoke `specify` hard-fail with an install instruction. The surface looks like:

```markdown
1. **Validate artifacts**
   ```bash
   specify validate "$CHANGE_DIR" --format json
   ```
   If the command is not found, stop and instruct the user to install the
   CLI via `brew install specify` (preferred), `cargo install specify`, or
   the release script at https://specify.sh/install, then re-run. Do not
   attempt a prose fallback — validation rules have diverged past the
   point where the agent can reliably reproduce them.
```

This is a deliberate break from the current `merge` skill's "if `python3` is unavailable, follow the algorithm in `delta-merge.md`" pattern. The prose fallback worked when the CLI was a 120-line Python script; it does not scale to the validator + parser + merger + task engine. Maintaining a second implementation in skill prose would reintroduce exactly the unreliable-interpretation problem this RFC exists to solve. `specify init` (the one skill that runs before the CLI has been exercised against the project) must carry the same hard-fail guard so projects never enter a state where `project.yaml` exists but no downstream skill can read it.

The install failure message is the only prose every CLI-invoking skill carries verbatim. It is short enough to duplicate, and centralising it in a reference doc would still require every skill to quote it.

**Version pinning.** `.specify/project.yaml` records a `specify_version` field (see `ProjectConfig` above). `specify init` writes the running binary's version into that field; every subsequent subcommand that loads `project.yaml` refuses to run when its own `CARGO_PKG_VERSION` is older (semver comparison on the `specify_version` field treated as a floor). The failure is surfaced as a dedicated error variant (`Error::SpecifyVersionTooOld { required, found }`) so the CLI can exit with a distinct code (3) and so skills can render a clear upgrade prompt. Newer binaries always accept older pinned versions — the field is a floor, not a match — because the CLI's forward compatibility is guaranteed by the JSON contract versioning rules above.

Upgrading `specify_version` is user-driven: running `specify init --upgrade` rewrites the field to the current binary's version after confirming the change via `AskQuestion`. There is no automatic bump — any subcommand silently upgrading the floor would defeat the purpose of recording it.

## Alternatives Considered

**Single monolithic crate.** Simpler on day one, but prevents embedders (LSP, WASM, integration tests) from pulling in just the slice they need, and collapses ownership boundaries between unrelated domains (schema resolution vs task parsing vs merge vs lifecycle) as the surface grows. The domain-crate split under `crates/` costs almost nothing in maintenance and scales cleanly into the RFC-2 (manifest) and RFC-3 (federation) surfaces.

**Separate `specify-cli` / `specify-core` split (earlier draft of this RFC).** Colocated the binary in a `crates/specify-cli/` subdirectory with a single `specify-core` library holding every domain module. Rejected because (a) `specify-core` becomes a catch-all whose coupling grows with every new RFC, (b) publishing a library whose name differs from the binary users install (`specify` vs `specify-cli`) is a discoverability tax, and (c) a root-package shape (`src/main.rs` + `src/lib.rs`) maps more naturally to `cargo install specify` and to embedders importing `use specify::…`.

**Agent-only approach (no CLI).** Continue encoding all validation and structural operations in skill prose. Rejected because LLMs are unreliable at structured decision trees — counting sections, verifying ID patterns, checking dependency graphs.

## References

- [RFC-1-A: Deferred Validation](rfc-1a-validation.md) — the three-way Pass/Fail/Deferred classification
- [RFC-2: Feature Manifests](../archive/rfc-2-execution.md) — extends the CLI with `specify manifest` subcommands
- [RFC-3: Multi-Repo Coordination](../rfc-3a-plan.md) — extends the CLI with `specify federation` subcommands
- [RFC-4: Type-Safe Skill Expression](../rfc-4-dsl.md) — extends the framework linter with skill validation
- [RFC-5: Framework Linter](../rfc-5-framework-lint.md) — ports `checks.ts` into a `specify-check` crate and adds `specify check`

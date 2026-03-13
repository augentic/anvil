# Augentic Lifecycle (specify)

Spec-driven development CLI. Manages the full change lifecycle -- propose, apply, archive -- with schema-governed artifacts, an artifact dependency graph, and structural delta merging for baseline spec evolution.

## Installation

```bash
# from this repo
cargo install --path . --root ~/.local

# from GitHub
cargo install --git https://github.com/augentic/specify --root ~/.local
```

```bash
# Homebrew (macOS / Linux)
# brew install augentic/specify

# from source
cargo install --path . --root ~/.local
```

## Quick Start

```bash
# Initialise OpenSpec in your project
specify init

# Create a new change
specify new add-dark-mode

# Check artifact status
specify status add-dark-mode

# Get instructions for the next artifact
specify instructions proposal add-dark-mode

# ... write artifacts (proposal.md, specs, design.md, tasks.md) ...

# Archive when done (merges delta specs into baseline)
specify archive add-dark-mode
```

## Commands

### `specify init`

Resolve the schema and write project configuration. Creates `.specify/config.yaml`, `.specify/schemas/`, `.specify/changes/`, and `.specify/specs/`.

```bash
specify init                  # interactive
specify init --schema omnia   # non-interactive (CI-friendly)
```

### `specify new <name>`

Create a new change directory with metadata and an empty `specs/` subdirectory.

```bash
specify new add-dark-mode
specify new add-dark-mode --json   # machine-readable output
```

### `specify status [change]`

Report artifact completion state for a change. Auto-selects when only one active change exists.

```bash
specify status                     # auto-select single active change
specify status add-dark-mode       # explicit change name
specify status --json              # machine-readable output
```

### `specify instructions <artifact> [change]`

Output enriched instructions for creating an artifact. Includes schema instruction, project context, rules, template content, and dependency guidance.

```bash
specify instructions proposal add-dark-mode
specify instructions apply add-dark-mode
specify instructions specs --json   # machine-readable output
```

### `specify list`

List active and archived changes with completion summaries.

```bash
specify list
specify list --json
```

### `specify archive <change>`

Merge delta specs into baseline specs and move the change to a dated archive.

```bash
specify archive add-dark-mode
specify archive add-dark-mode --json
```

The archive process:

1. Validates all artifacts are complete
2. Merges each `specs/<capability>/spec.md` into `.specify/specs/<capability>/spec.md`
3. Moves the change to `.specify/changes/archive/<date>-<name>/`

### `specify update`

Fetch the latest schemas from GitHub and write them to the local store (`~/.local/share/specify/schemas/`).

```bash
specify update                        # fetch from augentic/lifecycle main branch
specify update --project              # also update this project's .specify/schemas/
specify update --repo org/repo        # fetch from a different repository
specify update --git-ref v2.0         # fetch from a specific tag or branch
```

### `specify validate`

Validate the project's OpenSpec configuration and directory structure. Checks config, schema, artifact graph consistency, change metadata, and delta spec structure.

```bash
specify validate
```

### `specify schemas`

List all available schemas from embedded, local store, and project sources.

```bash
specify schemas
```

### `specify completions <shell>`

Generate shell completions for bash, zsh, fish, or powershell.

```bash
specify completions zsh > ~/.zfunc/_specify
specify completions bash --output /etc/bash_completion.d/specify
```

## Delta Spec System

Changes use delta operations to evolve baseline specs without replacing them wholesale:

- **ADDED Requirements** -- new requirements appended to the baseline
- **MODIFIED Requirements** -- existing requirements replaced by name match
- **REMOVED Requirements** -- existing requirements deleted by name match
- **RENAMED Requirements** -- requirement header renamed (FROM:/TO: format)

Delta merging is structural (header-based section splitting) and does not interpret markdown content. At archive time, deltas are applied to `.specify/specs/` and the change is moved to the archive.

## Skill Integration

The CLI is designed for invocation from AI coding skills (Cursor, Claude Code, etc.). All commands that produce output support `--json` for machine-readable consumption.

Typical skill workflow:

1. `specify new <name>` -- scaffold the change
2. `specify status <name>` -- check what's ready
3. `specify instructions <artifact> <name>` -- get enriched instructions
4. LLM writes the artifact
5. Repeat 2-4 until all artifacts are complete
6. `specify instructions apply <name>` -- get apply instructions
7. `specify archive <name>` -- merge and archive

## OpenSpec Artifacts

```text
.specify/
  config.yaml                # Project configuration (schema, context, rules)
  specs/                     # Baseline specs (source of truth)
  changes/                   # Active changes (one folder per change)
    <change-name>/
      .metadata.yaml         # Change metadata (schema, created_at)
      proposal.md            # Why this change
      specs/                 # Delta specs (ADDED/MODIFIED/REMOVED/RENAMED)
        <capability>/
          spec.md
      design.md              # How to implement
      tasks.md               # Implementation checklist
    archive/                 # Completed changes (dated)
  schemas/
    <schema-name>/
      schema.yaml            # Artifact definitions and dependency graph
      templates/             # Artifact templates
```

## Configuration

`.specify/config.yaml` controls which schema is active and provides project-specific context and rules for artifact generation.

```yaml
schema: omnia

context: |
  Tech stack: Rust, WASM (wasm32-wasip2), Omnia SDK
  Architecture: Handler<P> pattern with provider trait bounds
  Testing: Rust integration tests, cargo test

rules:
  proposal:
    - Identify the source workflow
  specs:
    - Use WHEN/THEN format for scenarios
  design:
    - Document domain model with entity relationships
  tasks:
    - Structure tasks around the skill chain
```

## Schema Resolution

Schemas are resolved in priority order:

1. **Local store** (`~/.local/share/specify/schemas/`) -- populated by `specify update`
2. **Embedded** -- schemas bundled at compile time from this repository's `schemas/`

## Global Options


| Flag              | Description                                      |
| ----------------- | ------------------------------------------------ |
| `-v`, `--verbose` | Increase log verbosity (`-v` debug, `-vv` trace) |
| `-q`, `--quiet`   | Suppress non-error output                        |


## Development

```bash
cargo build           # build debug binary
cargo clippy          # lint
cargo fmt             # format
cargo run -- --help   # run directly
```

### Project Structure

```text
src/
├── main.rs               -- entry point, command dispatch
├── lib.rs                -- module re-exports
├── cli.rs                -- clap CLI definitions
├── commands/
│   ├── archive.rs        -- specify archive
│   ├── completions.rs    -- specify completions
│   ├── init.rs           -- specify init
│   ├── instructions.rs   -- specify instructions
│   ├── list.rs           -- specify list
│   ├── new.rs            -- specify new
│   ├── schemas.rs        -- specify schemas
│   ├── status.rs         -- specify status
│   ├── update.rs         -- specify update
│   └── validate.rs       -- specify validate
└── core/
    ├── change.rs         -- change lifecycle (create, discover, archive)
    ├── config.rs         -- project config model (serde_yaml)
    ├── delta.rs          -- structural delta merge (parse, apply)
    ├── embedded.rs       -- compile-time embedded schemas
    ├── graph.rs          -- artifact DAG (topological sort, completion)
    ├── paths.rs          -- XDG path resolution, project root detection
    ├── registry.rs       -- schema registry (embedded + local + GitHub)
    └── schema.rs         -- schema model
```


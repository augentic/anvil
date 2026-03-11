# Augentic Lifecycle (alc)

Admin CLI for Augentic's spec-driven development workflow. Manages OpenSpec schemas, templates, and project configuration.

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# Initialise OpenSpec in your project
alc init

# Scaffold a new change
alc new add-caching

# Edit the generated artefacts
# openspec/changes/add-caching/proposal.md  (start here)
# openspec/changes/add-caching/design.md
# openspec/changes/add-caching/specs/
# openspec/changes/add-caching/tasks.md
```

## Commands

### `alc init`

Initialise OpenSpec in the current project. Creates the `openspec/` directory structure with schemas, templates, and a `config.yaml`.

```bash
alc init                                      # interactive
alc init --schema omnia --context "Rust WASM"  # non-interactive (CI-friendly)
alc init --force                               # reinitialise existing project
```

### `alc update`

Fetch the latest schemas from GitHub and write them to the local store (`~/.local/share/openspec/schemas/`).

```bash
alc update                        # fetch from augentic/lifecycle main branch
alc update --project              # also update this project's openspec/schemas/
alc update --repo org/repo        # fetch from a different repository
alc update --git-ref v2.0         # fetch from a specific tag or branch
```

### `alc new <change-name>`

Scaffold a new change directory from the schema's templates.

```bash
alc new add-user-auth
```

Creates `openspec/changes/add-user-auth/` with template files for each artifact defined in the schema (proposal.md, design.md, specs/, tasks.md for the omnia schema).

### `alc validate`

Validate the project's OpenSpec configuration and directory structure.

```bash
alc validate
```

Checks that `config.yaml` is valid, the referenced schema exists with all required templates, and that existing changes have the expected artifact files.

### `alc schemas`

List all available schemas from embedded, local store, and project sources.

```bash
alc schemas
```

### `alc completions <shell>`

Generate shell completions for bash, zsh, fish, or powershell.

```bash
alc completions zsh > ~/.zfunc/_alc
alc completions bash --output /etc/bash_completion.d/alc
```

## Schema Resolution

Schemas are resolved in priority order:

1. **Local store** (`~/.local/share/openspec/schemas/`) -- populated by `alc update`
2. **Embedded** -- schemas bundled at compile time from this repository's `openspec/schemas/`

The embedded schemas provide offline functionality. `alc update` fetches the latest versions from GitHub without requiring a binary update.

## Project Layout

After running `alc init`, your project will have:

```
openspec/
  config.yaml                # Project configuration (schema, context, rules)
  changes/                   # Change directories (created by `alc new`)
    <change-name>/
      proposal.md
      design.md
      specs/
      tasks.md
  schemas/
    <schema-name>/           # Schema definition and templates
      schema.yaml
      templates/
        proposal.md
        design.md
        spec.md
        tasks.md
```

## Configuration

`openspec/config.yaml` controls which schema is active and provides project-specific context and rules for artifact generation.

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

## Global Options

| Flag | Description |
| --- | --- |
| `-v`, `--verbose` | Increase log verbosity (`-v` debug, `-vv` trace) |
| `-q`, `--quiet` | Suppress non-error output |

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
├── main.rs             -- entry point, command dispatch
├── lib.rs              -- module re-exports
├── cli.rs              -- clap CLI definitions
├── commands/
│   ├── init.rs         -- alc init
│   ├── update.rs       -- alc update
│   ├── new.rs          -- alc new
│   ├── validate.rs     -- alc validate
│   ├── schemas.rs      -- alc schemas
│   └── completions.rs  -- alc completions
└── core/
    ├── config.rs       -- config model (serde_yaml)
    ├── embedded.rs     -- compile-time embedded schemas
    ├── paths.rs        -- XDG path resolution, project root detection
    ├── registry.rs     -- schema registry (embedded + local + GitHub)
    └── schema.rs       -- schema model
```

## License

MIT OR Apache-2.0

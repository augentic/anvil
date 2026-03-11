# Augentic Lifecycle (specify)

Admin CLI for Augentic's spec-driven development workflow. Manages OpenSpec schemas, templates, and project configuration.

## Installation

```bash
cargo install --path .
```

## Prerequisites

- [Homebrew](https://brew.sh) -- `specify init` installs the `openspec` CLI via `brew install openspec` if it is not already on PATH.

## Quick Start

```bash
# Initialise OpenSpec in your project (installs openspec CLI if needed)
specify init
```

## Commands

### `specify init`

Install the `openspec` CLI (via Homebrew) if needed, run `openspec init --tools cursor --force` to scaffold the project, then layer on specify-specific schema and configuration.

```bash
specify init                                      # interactive
specify init --schema omnia --context "Rust WASM"  # non-interactive (CI-friendly)
```

### `specify update`

Fetch the latest schemas from GitHub and write them to the local store (`~/.local/share/openspec/schemas/`).

```bash
specify update                        # fetch from augentic/lifecycle main branch
specify update --project              # also update this project's openspec/schemas/
specify update --repo org/repo        # fetch from a different repository
specify update --git-ref v2.0         # fetch from a specific tag or branch
```

### `specify validate`

Validate the project's OpenSpec configuration and directory structure.

```bash
specify validate
```

Checks that `config.yaml` is valid, the referenced schema exists with all required templates, and that existing changes have the expected artifact files.

### `specify schemas`

List all available schemas from embedded, local store, and project sources.

```bash
specify schemas
```

### `specify completions <shell>`

Generate shell completions for bash, zsh, fish, or powershell.

```bash
specify completions zsh > ~/.zfunc/_alc
specify completions bash --output /etc/bash_completion.d/specify
```

## Schema Resolution

Schemas are resolved in priority order:

1. **Local store** (`~/.local/share/openspec/schemas/`) -- populated by `specify update`
2. **Embedded** -- schemas bundled at compile time from this repository's `openspec/schemas/`

The embedded schemas provide offline functionality. `specify update` fetches the latest versions from GitHub without requiring a binary update.

## Project Layout

After running `specify init`, your project will have the standard OpenSpec structure plus specify-specific schema files:

```text
openspec/
  config.yaml                # Project configuration (schema, context, rules)
  specs/                     # Source of truth (your system's behaviour)
  changes/                   # Proposed changes (one folder per change)
  schemas/
    <schema-name>/           # Specify schema definition and templates
      schema.yaml
      templates/

.cursor/
  skills/                    # Cursor skills (created by openspec init)
  commands/                  # Cursor OPSX commands
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
├── main.rs             -- entry point, command dispatch
├── lib.rs              -- module re-exports
├── cli.rs              -- clap CLI definitions
├── commands/
│   ├── init.rs         -- specify init
│   ├── update.rs       -- specify update
│   ├── validate.rs     -- specify validate
│   ├── schemas.rs      -- specify schemas
│   └── completions.rs  -- specify completions
└── core/
    ├── config.rs       -- config model (serde_yaml)
    ├── embedded.rs     -- compile-time embedded schemas
    ├── paths.rs        -- XDG path resolution, project root detection
    ├── registry.rs     -- schema registry (embedded + local + GitHub)
    └── schema.rs       -- schema model
```

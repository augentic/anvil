# Augentic Lifecycle (anvil)

Admin CLI for Augentic's spec-driven development workflow. Manages OpenSpec schemas, templates, and project configuration.

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# Initialise OpenSpec in your project
anvil init
```

## Commands

### `anvil init`

Initialise OpenSpec in the current project. Creates the `openspec/` directory structure with schemas, templates, and a `config.yaml`.

```bash
anvil init                                      # interactive
anvil init --schema omnia --context "Rust WASM"  # non-interactive (CI-friendly)
anvil init --force                               # reinitialise existing project
```

### `anvil update`

Fetch the latest schemas from GitHub and write them to the local store (`~/.local/share/openspec/schemas/`).

```bash
anvil update                        # fetch from augentic/lifecycle main branch
anvil update --project              # also update this project's openspec/schemas/
anvil update --repo org/repo        # fetch from a different repository
anvil update --git-ref v2.0         # fetch from a specific tag or branch
```

### `anvil validate`

Validate the project's OpenSpec configuration and directory structure.

```bash
anvil validate
```

Checks that `config.yaml` is valid, the referenced schema exists with all required templates, and that existing changes have the expected artifact files.

### `anvil schemas`

List all available schemas from embedded, local store, and project sources.

```bash
anvil schemas
```

### `anvil completions <shell>`

Generate shell completions for bash, zsh, fish, or powershell.

```bash
anvil completions zsh > ~/.zfunc/_alc
anvil completions bash --output /etc/bash_completion.d/anvil
```

## Schema Resolution

Schemas are resolved in priority order:

1. **Local store** (`~/.local/share/openspec/schemas/`) -- populated by `anvil update`
2. **Embedded** -- schemas bundled at compile time from this repository's `openspec/schemas/`

The embedded schemas provide offline functionality. `anvil update` fetches the latest versions from GitHub without requiring a binary update.

## Project Layout

After running `anvil init`, your project will have:

```
openspec/
  config.yaml                # Project configuration (schema, context, rules)
  changes/                   # Change directories (created by `anvil new`)
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
│   ├── init.rs         -- anvil init
│   ├── update.rs       -- anvil update
│   ├── new.rs          -- anvil new
│   ├── validate.rs     -- anvil validate
│   ├── schemas.rs      -- anvil schemas
│   └── completions.rs  -- anvil completions
└── core/
    ├── config.rs       -- config model (serde_yaml)
    ├── embedded.rs     -- compile-time embedded schemas
    ├── paths.rs        -- XDG path resolution, project root detection
    ├── registry.rs     -- schema registry (embedded + local + GitHub)
    └── schema.rs       -- schema model
```

## License

MIT OR Apache-2.0
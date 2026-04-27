# Adding a CLI Command

This page walks through adding a new subcommand to the `specify` binary. All work happens in the [`augentic/specify-cli`](https://github.com/augentic/specify-cli) repository.

## Overview

Adding a command involves four layers:

1. **Domain logic** -- a library crate under `crates/` (or extending an existing one)
2. **CLI definition** -- clap arg structs and a `Commands` variant in `src/cli.rs`
3. **Dispatch** -- a handler function in `src/commands/`
4. **Output** -- text and JSON renderers following the v2 contract

## Step by step

### 1. Implement the domain logic

If the new command introduces a new domain concept, create a new crate:

```bash
cargo init crates/<name> --lib --name specify-<name>
```

Add it to the workspace in the root `Cargo.toml`:

```toml
[workspace]
members = [
    # ... existing members
    "crates/<name>",
]
```

Add `specify-error` as a dependency if the crate needs to return structured errors:

```toml
[dependencies]
specify-error = { path = "../error" }
```

Expose a public function that returns `Result<T, specify_error::Error>` (or your own error type for isolated subtrees like `specify-vectis`).

If the command extends an existing domain, add the logic to the appropriate existing crate.

### 2. Define the CLI arguments

In `src/cli.rs`, define the arg struct and add a variant to the appropriate command enum.

**For a new top-level command:**

```rust
#[derive(Subcommand)]
pub enum Commands {
    // ... existing variants

    /// One-line description for --help.
    NewCommand {
        /// Positional argument
        input: PathBuf,
        /// Optional flag
        #[arg(long)]
        dry_run: bool,
    },
}
```

**For a subcommand under an existing command group:**

```rust
#[derive(Subcommand)]
pub enum ExistingAction {
    // ... existing variants

    /// One-line description.
    NewSub {
        name: String,
    },
}
```

If the domain logic lives in a separate library crate with its own clap `Args` structs, flatten them into the CLI enum:

```rust
NewCommand(my_library::NewCommandArgs),
```

### 3. Wire the dispatch

Add a match arm in `src/commands/mod.rs` (or the relevant submodule). Most commands follow this pattern:

```rust
Commands::NewCommand { input, dry_run } => {
    let ctx = CommandContext::require(&cli)?;
    let result = specify_newcrate::do_thing(&input, dry_run)?;
    match cli.format {
        OutputFormat::Text => render_text(&result),
        OutputFormat::Json => emit_json(serde_json::to_value(&result)?),
    }
    CliResult::Success
}
```

Key conventions:

- **`CommandContext::require`** loads `.specify/project.yaml` and validates the CLI version floor. Use it for commands that operate on an initialized project. Use bare dispatch (no context) for commands that work without a project.
- **Return `CliResult`** -- the enum maps to exit codes (`Success` = 0, `GenericFailure` = 1, `ValidationFailed` = 2, `VersionTooOld` = 3).

### 4. Implement text and JSON renderers

**JSON output** must follow the v2 contract:

- Use `emit_json(value)` for success responses -- it auto-injects `"schema-version": 2`
- Use kebab-case keys in all JSON (e.g. `project-dir`, not `project_dir`)
- Error responses use `emit_error` or a parallel error emitter with `"error": "<kebab-variant>"`, `"message": "..."`, and `"exit-code": N`

**Text output** provides a humanised summary. There is no strict format requirement, but follow the patterns of existing commands for consistency.

### 5. Add tests

Add integration tests in `tests/<name>.rs` using `assert_cmd` and `tempfile`:

```rust
use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn new_command_success_json() {
    let dir = TempDir::new().unwrap();
    // ... set up test fixtures

    let output = Command::cargo_bin("specify")
        .unwrap()
        .args(["--format", "json", "new-command", dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema-version"], 2);
}
```

Note that `--format` is a global flag and goes before the subcommand name.

### 6. Register in the skill

If a skill invokes the new command, update the skill's `SKILL.md` in the `specify` repo to document the CLI invocation and expected output shapes.

### 7. Verify

```bash
cargo make ci          # full CI suite: lint, test, fmt, vet, deny
cargo clippy -p specify -p specify-<name> --all-targets -- -D warnings
./target/debug/specify new-command --help
```

## Worked example: `specify vectis`

The `specify vectis` subcommand tree was added by folding the standalone `vectis` CLI into `specify-cli`. The full migration plan at `docs/plans/fold-vectis-into-specify.md` in the specify-cli repo documents the seven-chunk process:

1. **Move** the source tree verbatim into `crates/vectis/`
2. **Convert** from binary to library (drop `[[bin]]`, expose arg structs and handlers)
3. **Wire dispatch** -- add `VectisAction` enum, `run_vectis` handler, `emit_vectis_error`
4. **Rewrite payloads** for the v2 contract (snake_case to kebab-case)
5. **Add text renderers** and integration tests
6. **Delete** the old artifacts from the specify repo
7. **Update** plugin and doc references

The migration demonstrates:

- How an isolated library crate (`specify-vectis`) integrates with the binary through flattened clap args
- How to create a parallel error emitter (`emit_vectis_error`) for a crate with its own error type
- How JSON contract compliance is tested with exact key-set assertions
- How the `--format text|json` global flag threads through to subcommand handlers

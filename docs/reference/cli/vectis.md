# specify vectis

Cross-platform Crux project scaffold and verification.

## Subcommands

### specify vectis init

Scaffold a minimum-viable Crux project.

```bash
specify vectis init <app-name> [--ios] [--android]
```

Creates a Crux project with:

- **Core** (always) -- Rust shared crate with `app.rs`, `Cargo.toml`, and basic capability wiring.
- **iOS shell** (with `--ios`) -- SwiftUI shell with UniFFI bindings.
- **Android shell** (with `--android`) -- Kotlin/Jetpack Compose shell with UniFFI bindings.

Templates include correct version pins for Crux, UniFFI, and platform tooling.

### specify vectis add-shell

Add a platform shell to an existing core-only project.

```bash
specify vectis add-shell <platform>
```

| Platform | Description |
|----------|-------------|
| `ios` | SwiftUI iOS shell |
| `android` | Kotlin/Jetpack Compose Android shell |

Parses `app.rs` for the app name and capabilities, then generates the shell with matching bindings.

### specify vectis verify

Check that all assemblies compile.

```bash
specify vectis verify
```

Builds the core, runs `cargo test`, and (if shells exist) builds each shell. Reports pass/fail per assembly.

### specify vectis update-versions

Manage coherent dependency pins.

```bash
specify vectis update-versions [--verify]
```

Updates Crux, UniFFI, Gradle, and Swift package version pins across the project. With `--verify`, checks that all pins are coherent without modifying files.

### specify vectis versions

Show the resolved version pins.

```bash
specify vectis versions [--dir <path>] [--version-file <path>]
```

Resolves the version-pin hierarchy (embedded → user → project → `--version-file` override) and emits the resolved set. Read-only — skills and briefs shell out to this instead of hardcoding dependency versions. Pair with `--format json` for a machine-readable payload.

## See also

- [Vectis Plugin](../plugins/vectis.md) -- Crux development plugin overview
- [Vectis Schema](../schemas/vectis.md) -- schema for cross-platform projects

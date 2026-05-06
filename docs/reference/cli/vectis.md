# specify-vectis

Standalone binary for cross-platform Crux project scaffolding, verification, and version management. Ships alongside the `specify` CLI in the [`augentic/specify-cli`](https://github.com/augentic/specify-cli) workspace (RFC-13 §4.3a re-extracted Vectis as a separate binary; the pre-RFC-13 `specify vectis ...` subcommand tree on `specify` itself was retired in chunk 2.6).

The same five verbs are reachable from the [`specify-vectis`](https://crates.io/crates/specify-vectis) library API for in-process callers — Vectis capability skills (`/vectis:core-writer`, `/vectis:ios-writer`, `/vectis:android-writer`, `/vectis:template-updater`) drive the binary form via shell-outs in their generated bash blocks; orchestrators that want to skip the process boundary may import the library directly.

## Subcommands

### specify-vectis init

Scaffold a minimum-viable Crux project.

```bash
specify-vectis init <app-name> [--ios] [--android]
```

Creates a Crux project with:

- **Core** (always) -- Rust shared crate with `app.rs`, `Cargo.toml`, and basic capability wiring.
- **iOS shell** (with `--ios`) -- SwiftUI shell with UniFFI bindings.
- **Android shell** (with `--android`) -- Kotlin/Jetpack Compose shell with UniFFI bindings.

Templates include correct version pins for Crux, UniFFI, and platform tooling.

### specify-vectis add-shell

Add a platform shell to an existing core-only project.

```bash
specify-vectis add-shell <platform>
```

| Platform | Description |
|----------|-------------|
| `ios` | SwiftUI iOS shell |
| `android` | Kotlin/Jetpack Compose Android shell |

Parses `app.rs` for the app name and capabilities, then generates the shell with matching bindings.

### specify-vectis verify

Check that all assemblies compile.

```bash
specify-vectis verify
```

Builds the core, runs `cargo test`, and (if shells exist) builds each shell. Reports pass/fail per assembly. The post-merge gate run by [`capabilities/vectis/briefs/merge.md`](../../../capabilities/vectis/briefs/merge.md) (RFC-13 §"Merge and adoption contract") shells out to this verb against the merged baseline; non-zero exit is recorded as a `failure` outcome on the slice.

### specify-vectis update-versions

Manage coherent dependency pins.

```bash
specify-vectis update-versions [--verify]
```

Updates Crux, UniFFI, Gradle, and Swift package version pins across the project. With `--verify`, runs the full cap matrix to confirm the proposed pins scaffold and compile end-to-end without modifying files. When the verify pass fails, the `/vectis:template-updater` skill is the matching repair surface.

### specify-vectis versions

Print the resolved version-pin map for the current project (or the embedded defaults when invoked outside a project).

```bash
specify-vectis versions
```

## Output contract

All subcommands accept `--format text` (default) or `--format json`. The JSON envelope shares the `schema-version: 2` shape used by the `specify` CLI (see [CLI architecture](../../contributing/cli-architecture.md#json-v2-contract)); `specify-vectis verify` extends it with an `assemblies.{core,ios,android,design_system}.steps[]` array that the merge brief threads through `--context` on a journal entry when reporting failures.

## See also

- [Vectis Plugin](../plugins/vectis.md) -- Crux development plugin overview
- [Vectis Capability](../capabilities/vectis.md) -- capability reference for cross-platform projects
- [`capabilities/vectis/briefs/merge.md`](../../../capabilities/vectis/briefs/merge.md) -- post-merge cap-matrix gate

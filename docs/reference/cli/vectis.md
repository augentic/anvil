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

### specify vectis validate

Run a deterministic validation mode against a Vectis input artifact (RFC-11 §H, §I).

```bash
specify vectis validate <mode> [path]
```

| Mode | Validates |
|------|-----------|
| `layout` | `layout.yaml` against the unwired subset of [`composition.schema.json`](../../../schemas/vectis/composition.schema.json) — YAML syntax, schema shape, `screens` only (no `delta`), no define-owned wiring keys (`maps_to`, `bind`, `event`, `error`, overlay `trigger`, `*-when`), and the §G structural-identity rule for any `component:` directives present. |
| `composition` | `composition.yaml` (wired or unwired). Schema shape, `screens` or `delta` as appropriate, structural identity, and cross-artifact reference resolution against sibling `tokens.yaml` / `assets.yaml`. Auto-invokes `tokens` and `assets` modes when those siblings exist (whether change-local or via `artifacts.tokens.paths` / `artifacts.assets.paths`). |
| `tokens` | `tokens.yaml` against [`tokens.schema.json`](../../../schemas/vectis/tokens.schema.json). |
| `assets` | `assets.yaml` against [`assets.schema.json`](../../../schemas/vectis/assets.schema.json), plus referenced-file existence under `design-system/assets/**` and per-platform source coverage. |
| `all` | Runs all four modes against the active change and baseline. Convenience verb. |

The optional `[path]` argument names the file to validate. When omitted, each mode resolves its default from the [`artifacts:` block](../../../schemas/vectis/schema.yaml) — `validate layout` reads `artifacts.layout.paths.change_local` then `artifacts.layout.paths.project`, and so on. An explicit `[path]` always wins.

Exit semantics (every mode):

- **Errors** — exit non-zero with a structured report.
- **Warnings only** — exit zero and print the warning report.
- **Clean** — exit zero silently.

Skills consume the report rather than reimplementing the checks; layout inferers in particular run `validate layout` (and `validate composition` when sibling token / asset manifests exist) on a staging path before atomically renaming onto the final output (see [`plugins/vectis/references/layout-inferer-contract.md`](../../../plugins/vectis/references/layout-inferer-contract.md#verification)).

## See also

- [Vectis Plugin](../plugins/vectis.md) -- Crux development plugin overview
- [Vectis Schema](../schemas/vectis.md) -- schema for cross-platform projects

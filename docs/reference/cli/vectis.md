# specify-vectis

Standalone binary for cross-platform Crux project scaffolding, verification, version management, and UI input validation. Ships alongside the `specify` CLI in the [`augentic/specify-cli`](https://github.com/augentic/specify-cli) workspace (RFC-13 §4.3a re-extracted Vectis as a separate binary; the pre-RFC-13 `specify vectis ...` subcommand tree on `specify` itself was retired in chunk 2.6).

The same six verbs are reachable from the [`specify-vectis`](https://crates.io/crates/specify-vectis) library API for in-process callers — Vectis capability skills (`/vectis:core-writer`, `/vectis:ios-writer`, `/vectis:android-writer`, `/vectis:template-updater`, `/vectis:image-layout-inferer`) drive the binary form via shell-outs in their generated bash blocks; orchestrators that want to skip the process boundary may import the library directly.

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

### specify-vectis validate

Run a deterministic validation mode against a Vectis input artifact (RFC-11 §H, §I).

```bash
specify-vectis validate <mode> [path]
```

| Mode | Validates |
|------|-----------|
| `layout` | `layout.yaml` against the unwired subset of [`composition.schema.json`](../../../capabilities/vectis/composition.schema.json) — YAML syntax, schema shape, `screens` only (no `delta`), no define-owned wiring keys (`maps_to`, `bind`, `event`, `error`, overlay `trigger`, `*-when`), and the §G structural-identity rule for any `component:` directives present. |
| `composition` | `composition.yaml` (wired or unwired). Schema shape, `screens` or `delta` as appropriate, structural identity, and cross-artifact reference resolution against sibling `tokens.yaml` / `assets.yaml`. Auto-invokes `tokens` and `assets` modes when those siblings exist. |
| `tokens` | `tokens.yaml` against [`tokens.schema.json`](../../../capabilities/vectis/tokens.schema.json). |
| `assets` | `assets.yaml` against [`assets.schema.json`](../../../capabilities/vectis/assets.schema.json), plus referenced-file existence under `design-system/assets/**` and per-platform source coverage. |
| `all` | Runs all four modes against the active slice and baseline. Convenience verb. |

The optional `[path]` argument names the file to validate. When omitted, each mode resolves its default from the canonical Vectis cascade: slice-local files first, then project-level design-system files or the merged composition baseline. An explicit `[path]` always wins.

Exit semantics (every mode):

- **Errors** — exit non-zero with a structured report.
- **Warnings only** — exit zero and print the warning report.
- **Clean** — exit zero silently.

Skills consume the report rather than reimplementing the checks; layout inferers in particular run `validate layout` (and `validate composition` when sibling token / asset manifests exist) on a staging path before atomically renaming onto the final output (see [`plugins/vectis/references/layout-inferer-contract.md`](../../../plugins/vectis/references/layout-inferer-contract.md#verification)).

## Output contract

All subcommands accept `--format json` (default) or `--format text`. The JSON envelope shares the `schema-version: 2` shape used by the pre-RFC-13 `specify vectis * --format json` dispatcher; `specify-vectis verify` extends it with an `assemblies.{core,ios,android}.steps[]` array that the merge brief threads through `--context` on a journal entry when reporting failures.

### specify-vectis versions

Show the resolved version pins.

```bash
specify-vectis versions [--dir <path>] [--version-file <path>]
```

Resolves the version-pin hierarchy (embedded → user → project → `--version-file` override) and emits the resolved set. Read-only — skills and briefs shell out to this instead of hardcoding dependency versions.

## See also

- [Vectis Plugin](../plugins/vectis.md) -- Crux development plugin overview
- [Vectis Capability](../capabilities/vectis.md) -- capability reference for cross-platform projects
- [`capabilities/vectis/briefs/merge.md`](../../../capabilities/vectis/briefs/merge.md) -- post-merge cap-matrix gate

# Vectis in-guest tools

Vectis deterministic helpers are **in-guest library code** compiled into the vectis adapter's published component (Omnia-migration cutover). There is no host dispatch verb: the vectis build and merge orchestrations invoke these behaviours directly, and no separate Vectis host binary is part of the command surface.

## Behaviours

### vectis validate

Deterministic validation for Vectis UI input artifacts, run by the build and merge orchestrations:

| Mode | Validates |
|------|-----------|
| `layout` | `layout.yaml` against the unwired subset of [`composition.schema.json`](https://schemas.specify.dev/vectis/composition.schema.json): YAML syntax, schema shape, `screens` only (no `delta`), no define-owned wiring keys (`maps_to`, `bind`, `event`, `error`, overlay `trigger`, `*-when`), and the structural-identity rule for any `component:` directives present. |
| `composition` | `composition.yaml` (wired or unwired), including schema shape, structural identity, and cross-artifact reference resolution against sibling `tokens.yaml` / `assets.yaml`. Auto-invokes `tokens` and `assets` modes when those siblings exist. |
| `tokens` | `tokens.yaml` against [`tokens.schema.json`](https://schemas.specify.dev/vectis/tokens.schema.json). |
| `assets` | `assets.yaml` against [`assets.schema.json`](https://schemas.specify.dev/vectis/assets.schema.json), plus referenced-file existence under `design-system/assets/**` and per-platform source coverage. |
| `all` | Runs all four modes against the active slice and baseline. |

Each mode resolves its default input from the Vectis artifact cascade: slice-local files first, then project-level design-system files or the merged composition baseline. An explicit path always wins.

Report semantics: errors block, warnings report without blocking, clean is silent. Skills consume the report rather than reimplementing the checks; layout inferers validate staged output before atomically renaming it into place.

### vectis verify

Declared-vs-present platform shell verification and host toolchain gates:

| Mode | Purpose |
|------|---------|
| `verify` | Build/lint gate: shell trees, Android toolchain artifacts, iOS scaffold drift, compile verify stamps (`iOS/.vectis/verify.ok`, `Android/.vectis/verify.ok`). |
| `bootstrap-app-icon` | Build-prelude launcher `app-icon` gate for declared UI platforms. |
| `host-prereq` | Build-prelude host toolchain probe (`ANDROID_HOME`, Rust Android targets, `xcodebuild` on macOS). |

The build orchestration's in-guest prelude runs the host-prereq and app-icon gates before code generation; the finalize tail runs the verify gate before stamping `built`.

Successful `make sim-build` / `make verify` write `.vectis/verify.ok` stamps consumed by `verify --mode verify`.

### vectis scaffold

Render Vectis project scaffolds (core, iOS, Android) from embedded templates and explicit inputs.

Scaffolding is render-only. It writes template output under the project tree; it does not run Cargo, Xcode, Gradle, SDK installers, registry updates, or cap-matrix verification. Those host workflow steps belong to the Vectis target's [`build`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/briefs/build.md) and [`merge`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/briefs/merge.md) briefs.

On an existing tree, iOS scaffolding refuses to overwrite agent-immutable files. Use the sync behaviour (below) to repair drift instead.

Version pins come from embedded defaults unless a complete TOML override file is supplied. The tool does not read user config, implicitly discover project-local version files, accept JSON on stdin, or expose per-pin flags in v1.

### vectis sync

Repair agent-immutable iOS scaffold files from the embedded templates without build-prelude side effects (no materialize, bootstrap app-icon gate, or Android setup). It re-renders `iOS/Makefile`, `iOS/project.yml`, and `iOS/.vectis/sim-build.sh` when on-disk bytes diverge from the template (for example, a named simulator destination like `name=iPhone 16` patched into the CLI-owned sim-build script).

The build orchestration also syncs these files at build start. The Vectis iOS build brief has the **orchestrator** run the ios-scaffold sync at the start of each verify iteration; `make sim-build` delegates to the immutable script, which always uses `generic/platform=iOS Simulator`.

## See also

- [Vectis Target](../targets/vectis.md) -- target adapter reference for cross-platform Crux projects
- [`adapters/targets/vectis/`](https://github.com/augentic/specify-adapters/tree/main/targets/vectis) -- the vectis adapter, including its in-guest core

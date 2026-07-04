# Vectis WASI tools

Vectis deterministic helpers are declared adapter tools and are run through [`specify extension`](extension.md). Operators install and invoke `specify`; no separate Vectis host binary is part of the current command surface.

## Tools

### vectis validate

Run deterministic validation for Vectis UI input artifacts:

```bash
specify extension run vectis -- validate <mode> [path]
```

| Mode | Validates |
|------|-----------|
| `layout` | `layout.yaml` against the unwired subset of [`composition.schema.json`](https://schemas.specify.dev/vectis/composition.schema.json): YAML syntax, schema shape, `screens` only (no `delta`), no define-owned wiring keys (`maps_to`, `bind`, `event`, `error`, overlay `trigger`, `*-when`), and the structural-identity rule for any `component:` directives present. |
| `composition` | `composition.yaml` (wired or unwired), including schema shape, structural identity, and cross-artifact reference resolution against sibling `tokens.yaml` / `assets.yaml`. Auto-invokes `tokens` and `assets` modes when those siblings exist. |
| `tokens` | `tokens.yaml` against [`tokens.schema.json`](https://schemas.specify.dev/vectis/tokens.schema.json). |
| `assets` | `assets.yaml` against [`assets.schema.json`](https://schemas.specify.dev/vectis/assets.schema.json), plus referenced-file existence under `design-system/assets/**` and per-platform source coverage. |
| `all` | Runs all four modes against the active slice and baseline. |

The optional `[path]` argument names the file to validate. When omitted, each mode resolves its default from the Vectis artifact cascade: slice-local files first, then project-level design-system files or the merged composition baseline. An explicit `[path]` always wins.

Exit semantics:

- **Errors** -- exit non-zero with a structured report.
- **Warnings only** -- exit zero and print the warning report.
- **Clean** -- exit zero silently.

Skills consume the report rather than reimplementing the checks. Layout inferers run `specify extension run vectis -- validate layout <output-path>.tmp` and, when sibling token or asset manifests exist, `specify extension run vectis -- validate composition <output-path>.tmp` before atomically renaming staged output into place.

### vectis verify

Declared-vs-present platform shell verification and host toolchain gates:

```bash
specify extension run vectis -- verify --mode <mode> [path]
```

| Mode | Purpose |
|------|---------|
| `verify` | Build/lint gate: shell trees, Android toolchain artifacts, iOS scaffold drift, compile verify stamps (`iOS/.vectis/verify.ok`, `Android/.vectis/verify.ok`). |
| `bootstrap-app-icon` | Prepare-time launcher `app-icon` gate for declared UI platforms. |
| `host-prereq` | Prepare-time host toolchain probe (`ANDROID_HOME`, Rust Android targets, `xcodebuild` on macOS). |

`specify slice build --phase prepare` runs the manifest-declared `host_prereq` script when the bound target declares one (Vectis: `scripts/build-host-prereq.sh`) and aborts with `target-build-host-prereq-missing` when it exits non-zero. Operators may also probe prerequisites with `specify extension run vectis -- verify --mode host-prereq`. `specify slice build --phase finalize` runs the manifest-declared `finalize_verify` script when present (Vectis: `scripts/build-finalize-verify.sh`) before stamping `built` (`target-build-verify-gate-failed` on failure).

Successful `make sim-build` / `make verify` write `.vectis/verify.ok` stamps consumed by `verify --mode verify`.

### vectis scaffold

Render Vectis project scaffolds from embedded templates and explicit inputs:

```bash
specify extension run vectis -- scaffold core <app-name> [--caps <csv>] [--android-package <package>] [--version-file <path>]
specify extension run vectis -- scaffold ios <app-name> [--caps <csv>] [--version-file <path>]
specify extension run vectis -- scaffold android <app-name> [--caps <csv>] [--android-package <package>] [--version-file <path>]
```

`vectis` (`scaffold`) is render-only. It writes template output under `PROJECT_DIR` using the permissions declared by [`adapters/targets/vectis/adapter.yaml`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/adapter.yaml) (its `extension` block); it does not run Cargo, Xcode, Gradle, SDK installers, registry updates, or cap-matrix verification. Those host workflow steps belong to the Vectis target's [`build`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/briefs/build.md) and [`merge`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/briefs/merge.md) briefs.

On an existing tree, `scaffold ios` refuses to overwrite agent-immutable files. Use `sync ios-scaffold` (below) to repair drift instead.

Version pins come from embedded defaults unless `--version-file <path>` names a complete TOML override. The tool does not read user config, implicitly discover project-local version files, accept JSON on stdin, or expose per-pin flags in v1.

### vectis sync

Repair agent-immutable iOS scaffold files from the embedded templates without prepare side effects (no materialize, bootstrap app-icon gate, or Android setup):

```bash
specify extension run vectis -- sync ios-scaffold [path]
```

When `[path]` is omitted, the command resolves the project root from `PROJECT_DIR` or a CWD walk-up to `.specify/`. It re-renders `iOS/Makefile`, `iOS/project.yml`, and `iOS/.vectis/sim-build.sh` when on-disk bytes diverge from the template (for example, a named simulator destination like `name=iPhone 16` patched into the CLI-owned sim-build script).

`specify slice build --phase prepare` also syncs these files at build start via `vectis prepare build`. The Vectis iOS build brief has the **orchestrator** run `sync ios-scaffold` at the start of each verify iteration; `make sim-build` delegates to the immutable script, which always uses `generic/platform=iOS Simulator`.

Exit semantics:

- **Success** -- exit zero; JSON reports `scaffold_sync.ios.synced` and `scaffold_sync.ios.unchanged` paths.
- **Project / write failure** -- exit non-zero with `invalid-project`.

## See also

- [specify extension](extension.md) -- declared WASI tool runner surface
- [Vectis Target](../targets/vectis.md) -- target adapter reference for cross-platform Crux projects
- [`adapters/targets/vectis/adapter.yaml`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/adapter.yaml) (its `extension` block) -- Vectis target adapter extension declaration

# Vectis WASI tools

Vectis deterministic helpers are declared adapter tools and are run through [`specify tool`](tool.md). Operators install and invoke `specify`; no separate Vectis host binary is part of the current command surface.

## Tools

### vectis validate

Run deterministic validation for Vectis UI input artifacts:

```bash
specify tool run vectis -- validate <mode> [path]
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

Skills consume the report rather than reimplementing the checks. Layout inferers run `specify tool run vectis -- validate layout <output-path>.tmp` and, when sibling token or asset manifests exist, `specify tool run vectis -- validate composition <output-path>.tmp` before atomically renaming staged output into place.

### vectis scaffold

Render Vectis project scaffolds from embedded templates and explicit inputs:

```bash
specify tool run vectis -- scaffold core <app-name> [--caps <csv>] [--android-package <package>] [--version-file <path>]
specify tool run vectis -- scaffold ios <app-name> [--caps <csv>] [--version-file <path>]
specify tool run vectis -- scaffold android <app-name> [--caps <csv>] [--android-package <package>] [--version-file <path>]
```

`vectis` (`scaffold`) is render-only. It writes template output under `PROJECT_DIR` using the permissions declared by [`adapters/targets/vectis/adapter.yaml`](../../../adapters/targets/vectis/adapter.yaml) (`tools[]`); it does not run Cargo, Xcode, Gradle, SDK installers, registry updates, or cap-matrix verification. Those host workflow steps belong to the Vectis target's [`build`](../../../adapters/targets/vectis/briefs/build.md) and [`merge`](../../../adapters/targets/vectis/briefs/merge.md) briefs.

Version pins come from embedded defaults unless `--version-file <path>` names a complete TOML override. The tool does not read user config, implicitly discover project-local version files, accept JSON on stdin, or expose per-pin flags in v1.

## See also

- [specify tool](tool.md) -- declared WASI tool runner surface
- [Vectis Target](../targets/vectis.md) -- target adapter reference for cross-platform Crux projects
- [`adapters/targets/vectis/adapter.yaml`](../../../adapters/targets/vectis/adapter.yaml) (`tools[]`) -- Vectis target adapter tool declarations

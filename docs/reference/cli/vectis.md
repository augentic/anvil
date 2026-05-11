# Vectis WASI tools

Vectis deterministic helpers are declared capability tools and are run through [`specify tool`](tool.md). Operators install and invoke `specify`; no separate Vectis host binary is part of the current command surface.

The unpublished/private `specify-vectis` binary and the older `specify vectis ...` subcommand tree are superseded. Historical RFCs may still mention them, but active docs and skills should use the declared tool commands below.

## Tools

### vectis validate

Run deterministic validation for Vectis UI input artifacts:

```bash
specify tool run vectis -- validate <mode> [path]
```

| Mode | Validates |
|------|-----------|
| `layout` | `layout.yaml` against the unwired subset of [`composition.schema.json`](../../../capabilities/vectis/composition.schema.json): YAML syntax, schema shape, `screens` only (no `delta`), no define-owned wiring keys (`maps_to`, `bind`, `event`, `error`, overlay `trigger`, `*-when`), and the structural-identity rule for any `component:` directives present. |
| `composition` | `composition.yaml` (wired or unwired), including schema shape, structural identity, and cross-artifact reference resolution against sibling `tokens.yaml` / `assets.yaml`. Auto-invokes `tokens` and `assets` modes when those siblings exist. |
| `tokens` | `tokens.yaml` against [`tokens.schema.json`](../../../capabilities/vectis/tokens.schema.json). |
| `assets` | `assets.yaml` against [`assets.schema.json`](../../../capabilities/vectis/assets.schema.json), plus referenced-file existence under `design-system/assets/**` and per-platform source coverage. |
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

`vectis` (`scaffold`) is render-only. It writes template output under `PROJECT_DIR` using the permissions declared by `capabilities/vectis/tools.yaml`; it does not run Cargo, Xcode, Gradle, SDK installers, registry updates, or cap-matrix verification. Those host workflow steps belong to the Vectis writer, reviewer, and template-updater skills.

Version pins come from embedded defaults unless `--version-file <path>` names a complete TOML override. The tool does not read user config, implicitly discover project-local version files, accept JSON on stdin, or expose per-pin flags in v1.

## Migration map

| Retired surface | Current surface |
|---|---|
| `specify-vectis validate <mode> [path]` | `specify tool run vectis -- validate <mode> [path]` |
| `specify-vectis init <app-name>` | `specify tool run vectis -- scaffold core <app-name>` plus optional `ios` / `android` render steps and skill-owned host workflow |
| `specify-vectis add-shell ios` | `specify tool run vectis -- scaffold ios <app-name>` plus iOS writer post-processing |
| `specify-vectis add-shell android` | `specify tool run vectis -- scaffold android <app-name> [--android-package <package>]` plus Android writer post-processing |
| `specify-vectis verify`, `update-versions`, `versions` | No direct WASI wrapper in v1; skill-owned host workflow and template-updater guidance own these concerns. |

## See also

- [specify tool](tool.md) -- declared WASI tool runner surface
- [Vectis Plugin](../plugins/vectis.md) -- Crux development plugin overview
- [Vectis Capability](../capabilities/vectis.md) -- capability reference for cross-platform projects
- [`capabilities/vectis/tools.yaml`](../../../capabilities/vectis/tools.yaml) -- Vectis capability tool declarations

# RFC-45: Asset Materialization and Mandatory App Icon

> Status: Draft · Serves: Vectis target adapter, `design-system/assets.yaml`, `vectis` WASI tool · Motivated by: iOS `actool` failures on unmaterialized assets and silent substitution of designer SVGs with platform symbols

## Abstract

Vectis today treats `design-system/assets.yaml` as an inventory while shell writers often render platform-native symbols (`SF Symbols`, Material Icons) instead of the designer's assets. Canonical inputs are frequently SVG (scalable, web-friendly), but iOS and Android require platform-specific materialized artifacts. This RFC introduces:

1. A deterministic **`vectis materialize assets`** step that converts canonical sources into per-platform exports.
2. A strict **render-by-`kind`** writer contract: `vector` / `raster` assets always render from materialized shell resources; `symbol` is the only explicit glyph path.
3. A top-level **`app-icon`** field in `assets.yaml` pointing at a `role: app-icon` entry.
4. A **plan-time validation gate** that fails when a plan implies bootstrapping a new UI shell platform and `app-icon` is absent or not materializable.

SVG remains the canonical designer format. Mobile shells consume derived exports; the future web shell reads canonical SVG directly.

## Motivation

Vectis design-system flows exposed three coupled failures:

| Symptom | Root cause |
|---------|------------|
| iOS `CompileAssetCatalogVariant` failed | Missing `AppIcon.appiconset`; complex `splash-image.svg` copied raw into `Assets.xcassets` |
| ~80 icon SVGs unused in generated shells | Writers substituted `Image(systemName:)` / `Icons.Default.*` when `sources.ios` was absent |
| `vectis validate` passed structurally but builds failed | Validation checks file existence and YAML shape, not actool-safe materialization or writer fidelity |

The design-system slice correctly chose SVG as canonical input (small, scalable, ideal for a future web shell). The pipeline never closed the loop from **canonical source → platform binary → shell resource → rendered view**.

Inference and build also conflated two different policies:

- **Build** must render designer assets as specified.
- **Inference** may propose a platform symbol only when a screenshot shape has **no** matching asset and a close platform glyph exists — recorded as `kind: symbol` in `assets.yaml`, not as a writer shortcut.

## Principles

- **Canonical vs materialized.** `source:` (typically SVG under `design-system/assets/`) is designer-owned and web-canonical. `sources.<platform>` and `assets/exports/<platform>/` are tool-generated or operator-pinned derivatives. Shell trees never symlink back into `design-system/assets/` at runtime.
- **Commit materialized exports.** `design-system/assets/exports/` is version-controlled in consumer repos alongside canonical `source:` files. CI and shell builds consume committed exports; they do not require `vectis materialize` (or image-processing deps) on every job. Operators re-run materialize after editing canonical assets and commit the regenerated tree.
- **Fail closed on missing materialization.** A composition-referenced `vector` / `raster` id without exports for a declared project platform is an error — never a silent symbol fallback at build time.
- **Symbols are explicit inventory.** Platform glyph use requires `kind: symbol` on an `assets.yaml` entry (optionally `inferred: true` when promoted from screenshots). Composition still references the asset id.
- **CLI owns determinism.** Materialization, catalog emission, and bootstrap validation live in `vectis` / `specify plan validate`. Shell writers copy pre-validated outputs and emit view code; they do not convert formats or invent icons.
- **Minimal schema growth.** One top-level pointer (`app-icon`), one new `role`, one optional `sources.web`, one optional `inferred` flag on symbol entries. No per-composition-item render mode.

## Design

### 1. Asset rendering contract (shell writers)

For each `icon` / `image` / `icon-button` / `fab` reference in `composition.yaml`, resolve the id in `assets.yaml`:

| `assets.<id>.kind` | iOS | Android | Web (when supported) |
|--------------------|-----|---------|----------------------|
| `vector` | `Image("<id>")` from `Assets.xcassets` | `painterResource(R.drawable.<id_snake>)` | asset URL / inline SVG from `sources.web` |
| `raster` | imageset densities | `drawable-*` | raster URL |
| `symbol` | `Image(systemName: symbols.ios)` | `Icon(… symbols.android)` | mapped web glyph |

**Forbidden:** emitting `systemName` / `Icons.Default.*` for an id whose entry is `vector` or `raster`. Target review briefs and `specify lint project` SHOULD flag this drift.

Remove build-time "fall back to SF Symbols / Material icons when exports missing" language from Vectis target references (e.g. screens-and-navigation `design.md`). Fallback is inference-only (§5).

### 2. `vectis materialize assets`

New WASI subcommand:

```bash
specify tool run vectis -- materialize assets [path/to/assets.yaml] [--platform <csv>] [--dry-run]
```

**Inputs:** `assets.yaml`, canonical files, `project.yaml` `platforms` (default: all declared platforms with on-disk interpretation).

**Outputs:** files under `design-system/assets/exports/<platform>/`, written to paths recorded in `sources.<platform>` (or defaulted by the materializer). These files are **committed** to the repo — not gitignored. Validation checks that each referenced `sources.<platform>` path exists (from the committed tree or after a local materialize run).

**Invocation points:**

| Phase | When |
|-------|------|
| `specify slice build --phase prepare` | Auto-run when `assets.yaml` is a bound target input and any composition-referenced asset lacks fresh exports on disk |
| Operator | Regen after editing canonical SVGs; commit the updated `exports/` tree in the same change |
| Design-system slice | Task: materialize and commit exports before first shell slice builds |
| CI (default) | Relies on committed `exports/`; does not run materialize unless a job explicitly checks freshness |

**Materialization strategy** (by `role` + `kind`):

| `role` | Canonical | iOS output | Android output | Web output |
|--------|-----------|------------|----------------|------------|
| `icon` | SVG | PDF in `<id>.imageset/` | Vector Drawable XML in `drawable/` | copy / link SVG |
| `illustration` | SVG | PNG `@2x` / `@3x` in imageset | PNG per density bucket | SVG |
| `app-icon` | SVG or raster | `AppIcon.appiconset/` (see §4) | adaptive + legacy mipmaps (see §4) | favicon + manifest icons (see §4) |
| `photo` | raster | copy density slots | copy density slots | copy |
| `decorative` | any | same as `icon` / `illustration` by `kind` | same | same |

Implementation lives in `wasi-tools/vectis` (pure Rust: `usvg` / `resvg` for SVG→PDF/PNG; Android Vector Drawable conversion as a dedicated pass). Complex SVG features that fail a lightweight profile check MUST surface as materialization errors with the offending asset id.

**Idempotence:** deterministic output for fixed inputs; content hashes MAY be recorded in a sidecar `exports.lock` (deferred — v1 relies on file presence + validate).

### 3. `assets.yaml` schema extensions

#### 3.1 Top-level `app-icon` (required conditionally)

```yaml
version: 1

# References the kebab-case key under `assets:` for the launcher icon.
# Required when plan validation detects UI platform bootstrap (§6).
app-icon: app-icon

assets:
  app-icon:
    kind: vector          # or raster
    role: app-icon
    alt: "Application"
    source: assets/app-icon.svg
```

Schema (`assets.schema.json`):

- Add optional property `app-icon: { "$ref": "#/$defs/assetId" }`.
- Cross-check: referenced id MUST exist under `assets:` and MUST have `role: app-icon`.

`role` enum gains `app-icon`.

#### 3.2 `sources.web`

Optional on `vectorEntry` / `rasterEntry`:

```yaml
sources:
  web: assets/app-icon.svg   # defaults to `source` when omitted
```

Web shell reads `sources.web` or `source` directly; no PDF/PNG conversion required for v1 web.

#### 3.3 `inferred` on `symbolEntry` (optional)

```yaml
  chevron-right:
    kind: symbol
    role: icon
    inferred: true
    symbols:
      ios: chevron.right
      android: chevron_right
```

Documents screenshot-inferred glyphs. Default `false`. Reviewers MAY flag `inferred: true` on branded shapes.

No other schema changes. Composition continues to reference asset ids only; `kind` on the asset entry selects the render path.

### 4. App icon requirements per platform

The `app-icon` asset is special: stores and launchers require fixed shapes outside normal UI imagesets.

#### 4.1 Canonical input (designer)

| Constraint | Rule |
|------------|------|
| Format | SVG (preferred) or square raster (≥1024×1024) |
| Canvas | Square 1:1; no iOS/Android corner masking in source |
| Transparency | Avoid for iOS marketing icon (Apple rejects alpha on App Store icon); Android adaptive foreground may use transparency |
| Safe zone | For Android adaptive: keep logo inside central ~66% ("mask" safe area) |
| Color | Full-color brand mark; no platform chrome |

#### 4.2 iOS (`AppIcon.appiconset`)

Materializer emits `iOS/<App>/Resources/Assets.xcassets/AppIcon.appiconset/` (scaffold creates empty slot; first materialize / bootstrap build fills it).

| Requirement | Detail |
|-------------|--------|
| Xcode setting | `ASSETCATALOG_COMPILER_APPICON_NAME = AppIcon` (XcodeGen default — scaffold MUST ship the appiconset, not rely on implicit name) |
| Minimum for simulator / debug | Single **1024×1024** PNG, `idiom: universal`, `platform: ios` (iOS 11+ single-size model) |
| Store / release | Same 1024 PNG or full slot grid if materializer expands later |
| Source conversion | SVG → 1024 PNG (opaque background if source has transparency) |
| Writer | Copies generated appiconset; never deletes operator overrides outside generated filenames |

#### 4.3 Android (`mipmap` + adaptive)

Materializer emits under `Android/app/src/main/res/`:

| Artifact | Purpose |
|----------|---------|
| `mipmap-anydpi-v26/ic_launcher.xml` | Adaptive icon definition |
| `mipmap-anydpi-v26/ic_launcher_round.xml` | Round launcher variant |
| `drawable/ic_launcher_foreground.xml` or density PNGs | Foreground layer from SVG |
| `values/ic_launcher_background.xml` or `color` resource | Background (from `tint` token ref on asset entry, or neutral default) |
| `mipmap-{mdpi,hdpi,xhdpi,xxhdpi,xxxhdpi}/ic_launcher.png` | Legacy pre-API-26 fallback (rasterized from canonical) |

`AndroidManifest.xml` continues `android:icon="@mipmap/ic_launcher"`.

#### 4.4 Web (future shell)

When `web ∈ project.yaml.platforms` and web shell is implemented:

| Artifact | Size / notes |
|----------|----------------|
| `favicon.svg` | Canonical passthrough when SVG |
| `favicon.ico` | 16, 32, 48 fallback |
| `apple-touch-icon.png` | 180×180 |
| `manifest` icons | 192, 512 PNG |

Materializer writes to `web/public/icons/` or equivalent; exact tree owned by web scaffold RFC.

#### 4.5 Bootstrap placeholder policy

Plans that trigger UI bootstrap (§6) **require a real `app-icon` entry** — validation does not accept absent field.

Operators MAY use a deliberately ugly placeholder SVG committed to the design-system repo; the field must still be present and materializable. Auto-generated brand-colored placeholders without designer input are **deferred** (optional future `app-icon: { generated: true }` — out of scope for v1).

### 5. Inference-time symbol exception

Screenshot / layout inferers (`adapters/sources/screenshots`, `layout-inferer-contract.md`):

1. Shape matches `assets.<id>` → reference that id in layout/composition.
2. Shape is a generic platform glyph, **no** matching asset → MAY add `kind: symbol` entry (with `inferred: true`) or emit `notes.todo` for operator approval before merge.
3. Branded / custom shape → `notes.todo: add <id> to assets.yaml`; never auto-symbol.

Inferers MUST NOT crop production assets from screenshots (unchanged). Symbol promotion is inventory authoring, not a build shortcut.

### 6. Plan validation: mandatory `app-icon` on UI bootstrap

#### 6.1 Bootstrap trigger

A plan **implies bootstrapping a new UI platform** when **any** of the following hold at `specify plan validate` time for a bound project:

| Trigger | Detection |
|---------|-----------|
| **A. Reconciled bootstrap slices** | `plan.yaml` contains an entry named `app-foundation`, `bootstrap-ios`, or `bootstrap-android` (or `{project}-` prefixed equivalents from multi-project reconcile) |
| **B. Absent UI shells** | `project.yaml.platforms` includes `ios` and/or `android`, and `detect_missing_platforms` (same heuristic as `specify plan propose --reconcile-platforms` / `vectis verify --mode detect`) reports that platform absent |
| **C. Explicit web bootstrap** | (Future) plan entry or proposal flag declares `bootstrap-web` when web shell lands |

`core`-only bootstrap (`app-foundation` with only `core` missing) does **not** require `app-icon`. Trigger fires when **`ios` or `android` (or future `web`) is among missing platforms** for the project.

#### 6.2 Validation rule

When §6.1 triggers for project `P`:

1. Resolve `design-system/assets.yaml` for `P` (project-local path).
2. **Error** `plan-bootstrap-app-icon-missing` if:
   - file absent, or
   - top-level `app-icon` absent, or
   - `app-icon` id not found under `assets:`, or
   - referenced entry lacks `role: app-icon`, or
   - canonical `source` file missing, or
   - materialized exports absent for each **missing UI platform** in the trigger set (after running materialize in check-only mode, or verifying `exports/` paths).

Gate placement:

| Gate | Enforced |
|------|----------|
| `specify plan validate` | Yes — blocks Gate 1 stamp |
| `specify plan transition <name> approved` | Indirect (validate should run first) |
| `specify slice build --phase prepare` | Yes — hard fail for any shell build without materialized `app-icon` when platform is in scope |

`vectis validate assets` gains the same `app-icon` structural checks but does not know plan context; plan validate owns the bootstrap conditional.

#### 6.3 Interaction with `app-foundation` slice

Greenfield reconcile inserts `app-foundation` when all supported platforms are missing. That slice SHOULD depend on design-system existing with `tokens.yaml`, `assets.yaml` (including `app-icon`), and materialized exports before iOS/Android bootstrap build slices run. Plan DAG:

```text
design-system  →  app-foundation (scaffold shells)  →  feature slices
     ↑
  must include app-icon before Gate 1 when bootstrap trigger fires
```

### 7. Validation extensions (ongoing)

Extend existing `vectis validate assets` (composition-referenced assets):

| Check | Severity |
|-------|----------|
| Composition-referenced `vector`/`raster` lacks `sources.<platform>` **and** no export file | error |
| `sources.ios` ends in `.svg` for `role: illustration` | warning (error after materialize mandate) |
| `sources.ios` ends in `.svg` for `role: app-icon` | error |
| Platform set from `project.yaml.platforms` instead of hardcoded `["ios","android"]` | error when missing |
| Shell tree missing catalog entry for referenced non-symbol asset | `vectis verify --mode verify` |

Diagnostic ids (illustrative): `assets-materialization-missing`, `assets-app-icon-invalid`, `assets-svg-illustration-on-ios`, `plan-bootstrap-app-icon-missing`.

### 8. Scaffold changes

| Component | Change |
|-----------|--------|
| `templates/vectis/ios/` | Ship empty `AppIcon.appiconset/Contents.json` skeleton; materialize fills PNG |
| `templates/vectis/ios/project.yml` | Ensure `resources:` includes `<App>/Resources` |
| `templates/vectis/android/` | Default adaptive icon resource stubs pointing at materialized layers |
| Vectis init / design-system template | Document `app-icon` field; no default placeholder file in v1 |

### 9. Documentation updates

| Document | Change |
|----------|--------|
| `adapters/targets/vectis/references/ios/design-system-integration.md` | Materialize-before-copy; remove build-time symbol fallback |
| `adapters/targets/vectis/references/android/design-system-integration.md` | Same |
| `adapters/targets/vectis/briefs/build/ios/write.md` | Writer step: run materialize; render by `kind` |
| `adapters/targets/vectis/briefs/build/android/write.md` | Same |
| `adapters/sources/screenshots/briefs/extract.md` | Symbol inference policy |
| `wasi-tools/vectis/DECISIONS.md` | §K materialization; §L app-icon |

## Implementation phases

### Phase 1 — Policy and gates (no converter yet)

- Schema: `app-icon`, `role: app-icon`, `sources.web`, `inferred`.
- `specify plan validate`: bootstrap trigger + `plan-bootstrap-app-icon-missing`.
- Writer/review doc updates; review rule flagging symbol substitution.
- iOS scaffold: `AppIcon.appiconset` skeleton.

### Phase 2 — Materialize v1

- `vectis materialize assets`: icons (SVG→PDF / VD XML), illustrations (SVG→PNG), `app-icon` (iOS 1024 + Android adaptive).
- Hook into `slice build --phase prepare`.
- Extend `vectis validate assets` for export presence.
- Design-system docs and acceptance fixtures: commit `exports/` outputs; do not gitignore the tree.

### Phase 3 — Fidelity and web

- `vectis verify` catalog completeness check (optional `actool` dry-run).
- Web shell consumes `sources.web`.
- `exports.lock` / digest pinning if needed.

## Non-goals

- Hand-authoring per-density exports as the long-term workflow (materialize owns generation; operators commit the `exports/` tree for reproducibility).
- Automatic symbol substitution at build time.
- Figma / screenshot asset extraction (screenshots remain non-destructive).
- Defining the web shell scaffold (only `sources.web` and materialize outputs reserved).
- Generic image CDN or remote asset hosting.

## Resolved decisions

1. **`exports/` committed vs gitignored?** **Commit.** Consumer repos version-control `design-system/assets/exports/` so CI and shell builds are reproducible without running `vectis materialize` (and without image-processing deps) on every job. Framework acceptance fixtures pin small committed PNG/PDF outputs under the same policy.

## Open questions

1. **Single global `app-icon` vs per-platform ids?** v1: one `app-icon` field; materializer derives platform shapes. Per-platform overrides deferred unless designers require different marks.
2. **Plan trigger B without bootstrap slices?** If operator declines `--reconcile-platforms` but platforms are still absent, validate still fails at Gate 1 — intentional force function.
3. **Raster-only design systems?** `kind: raster` + `role: app-icon` with `sources.ios.1x/2x/3x` supported; materialize copies/resizes.

## References

- [`adapters/targets/vectis/references/ios/design-system-integration.md`](../adapters/targets/vectis/references/ios/design-system-integration.md) — current copy-on-generate contract
- [`wasi-tools/vectis/embedded/assets.schema.json`](https://github.com/augentic/specify-cli/blob/main/wasi-tools/vectis/embedded/assets.schema.json) — assets artifact schema (`specify-cli`)
- [`wasi-tools/vectis/src/validate/engine/assets.rs`](https://github.com/augentic/specify-cli/blob/main/wasi-tools/vectis/src/validate/engine/assets.rs) — cross-artifact validation (`specify-cli`)
- [`wasi-tools/vectis/src/verify.rs`](https://github.com/augentic/specify-cli/blob/main/wasi-tools/vectis/src/verify.rs) — platform shell detect/verify (`specify-cli`)
- [`crates/workflow/src/change/plan/core/propose/platforms.rs`](https://github.com/augentic/specify-cli/blob/main/crates/workflow/src/change/plan/core/propose/platforms.rs) — `reconcile_platforms` / bootstrap slice insertion (`specify-cli`)
- [`adapters/targets/vectis/briefs/build/ios/write.md`](../adapters/targets/vectis/briefs/build/ios/write.md) — verify loop (`make sim-build`)
- Apple Human Interface Guidelines — App Icon (1024×1024, no alpha)
- Android Adaptive Icons — foreground/background safe zone

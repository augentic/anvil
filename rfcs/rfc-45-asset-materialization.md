# RFC-45: Asset Materialization and Mandatory App Icon

> Status: Draft · Serves: Vectis target adapter, `design-system/assets.yaml`, `vectis` WASI tool · Motivated by: iOS `actool` failures on unmaterialized assets and silent substitution of designer SVGs with platform symbols

## Abstract

Vectis today treats `design-system/assets.yaml` as an inventory while shell writers often render platform-native symbols (`SF Symbols`, Material Icons) instead of the designer's assets. Canonical inputs are frequently SVG (scalable, web-friendly), but iOS and Android require platform-specific materialized artifacts. This RFC introduces:

1. A deterministic **`vectis materialize assets`** step that converts canonical sources into per-platform exports.
2. A strict **render-by-`kind`** writer contract: `vector` / `raster` assets always render from materialized shell resources; `symbol` is the only explicit glyph path.
3. A top-level **`app-icon`** field in `assets.yaml` pointing at a `role: app-icon` entry.
4. A **bootstrap-only validation gate** that hard-fails when a plan implies bootstrapping a new UI shell platform and no satisfiable `app-icon` exists — neither a straightforward canonical image the materializer can convert, nor operator-pinned hand-built exports in the expected `exports/<platform>/` layout for each missing platform. Plans that reuse shells with an existing launcher icon proceed without re-checking design-system inventory.

SVG remains the canonical designer format. Mobile shells (iOS, Android) consume derived exports. Web asset materialization is out of scope here and specified separately in [RFC-45a](future/rfc-45a-web-asset-materialization.md), deferred until a web shell scaffold exists.

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
- **Auto-convert or operator-pin.** Default path: `vectis materialize assets` performs straightforward format conversion from `source:` into `exports/<platform>/`. When a platform needs designer-specific treatment (e.g. iOS glass, Android adaptive layers hand-tuned in graphic tools), the operator commits hand-built artifacts under the conventional `exports/<platform>/` tree and pins paths in `sources.<platform>`. The materializer MUST NOT overwrite operator-pinned paths; it fills only missing slots from `source:` when no pin exists.
- **Commit materialized exports.** `design-system/assets/exports/` is version-controlled in consumer repos alongside canonical `source:` files. CI and shell builds consume committed exports; they do not require `vectis materialize` (or image-processing deps) on every job. Operators re-run materialize after editing canonical assets and commit the regenerated tree.
- **Bootstrap-only `app-icon` gate.** Mandatory `app-icon` validation runs only when §6.1 detects UI shell bootstrap for a platform. Incremental plans against shells that already carry a launcher icon (from a prior bootstrap or operator-authored shell resources) are not blocked by design-system `app-icon` inventory.
- **Fail closed on missing materialization.** A composition-referenced `vector` / `raster` id without exports for a declared project platform is an error — never a silent symbol fallback at build time.
- **Symbols are explicit inventory.** Platform glyph use requires `kind: symbol` on an `assets.yaml` entry (optionally `inferred: true` when promoted from screenshots). Composition still references the asset id.
- **CLI owns determinism.** Materialization, catalog emission, and bootstrap validation live in `vectis` / `specify plan validate`. Shell writers copy pre-validated outputs and emit view code; they do not convert formats or invent icons.
- **Minimal schema growth.** One top-level pointer (`app-icon`), one new `role`, one optional `inferred` flag on symbol entries. No per-composition-item render mode. (Web adds one optional `sources.web` later — see [RFC-45a](future/rfc-45a-web-asset-materialization.md).)

## Design

### 1. Asset rendering contract (shell writers)

For each `icon` / `image` / `icon-button` / `fab` reference in `composition.yaml`, resolve the id in `assets.yaml`:

| `assets.<id>.kind` | iOS | Android |
|--------------------|-----|---------|
| `vector` | `Image("<id>")` from `Assets.xcassets` | `painterResource(R.drawable.<id_snake>)` |
| `raster` | imageset densities | `drawable-*` |
| `symbol` | `Image(systemName: symbols.ios)` | `Icon(… symbols.android)` |

**Forbidden:** emitting `systemName` / `Icons.Default.*` for an id whose entry is `vector` or `raster`. Target review briefs and `specify lint project` SHOULD flag this drift.

Remove build-time "fall back to SF Symbols / Material icons when exports missing" language from Vectis target references (e.g. screens-and-navigation `design.md`). Fallback is inference-only (§5).

### 2. `vectis materialize assets`

New WASI subcommand:

```bash
specify tool run vectis -- materialize assets [path/to/assets.yaml] [--platform <csv>] [--dry-run]
```

**Inputs:** `assets.yaml`, canonical files, `project.yaml` `platforms` (default: all declared platforms with on-disk interpretation).

**Outputs:** files under `design-system/assets/exports/<platform>/`, written to paths recorded in `sources.<platform>` (or defaulted by the materializer). These files are **committed** to the repo — not gitignored. Validation checks that each referenced `sources.<platform>` path exists (from the committed tree or after a local materialize run).

**Operator pins:** when `sources.<platform>` is already set and the referenced path exists on disk, `materialize assets` skips that platform slot for the asset. When `sources.<platform>` is absent, the materializer writes to the conventional default under `exports/<platform>/` and MAY update `assets.yaml` with the resulting path (implementation choice — v1 MAY require the operator to record pins manually after first materialize). Operator-pinned exports take precedence over `source:` for that platform; editing `source:` alone does not regenerate a pinned platform until the operator clears the pin or deletes the pinned tree.

**Invocation points:**

| Phase | When |
|-------|------|
| `specify slice build --phase prepare` | Auto-run when `assets.yaml` is a bound target input and any composition-referenced asset lacks fresh exports on disk |
| Operator | Regen after editing canonical SVGs; commit the updated `exports/` tree in the same change |
| Design-system slice | Task: materialize and commit exports before first shell slice builds |
| CI (default) | Relies on committed `exports/`; does not run materialize unless a job explicitly checks freshness |

**Materialization strategy** (by `role` + `kind`):

| `role` | Canonical | iOS output | Android output |
|--------|-----------|------------|----------------|
| `icon` | SVG | PDF in `<id>.imageset/` | Vector Drawable XML in `drawable/` |
| `illustration` | SVG | PNG `@2x` / `@3x` in imageset | PNG per density bucket |
| `app-icon` | SVG or square raster (see §4) | `exports/ios/app-icon/AppIcon.appiconset/` (see §4) — auto-converted or operator-pinned | `exports/android/app-icon/` adaptive + legacy mipmaps (see §4) — auto-converted or operator-pinned |
| `photo` | raster | copy density slots | copy density slots |
| `decorative` | any | same as `icon` / `illustration` by `kind` | same |

Implementation lives in `wasi-tools/vectis` (pure Rust: `usvg` / `resvg` for SVG→PDF/PNG; Android Vector Drawable conversion as a dedicated pass). Complex SVG features that fail a lightweight profile check MUST surface as materialization errors with the offending asset id.

**Idempotence:** deterministic output for fixed inputs; content hashes MAY be recorded in a sidecar `exports.lock` (deferred — v1 relies on file presence + validate).

### 3. `assets.yaml` schema extensions

#### 3.1 Top-level `app-icon` (required conditionally)

```yaml
version: 1

# References the kebab-case key under `assets:` for the launcher icon.
# Required when plan validation detects UI platform bootstrap (§6) and no
# shell-resident launcher icon already satisfies the missing platform.
app-icon: app-icon

assets:
  app-icon:
    kind: vector          # or raster
    role: app-icon
    alt: "Application"
    source: assets/app-icon.svg                    # auto-convert input; omit when every platform is operator-pinned
    sources:
      ios: assets/exports/ios/app-icon/AppIcon.appiconset      # operator-pinned hand-built tree
      android: assets/exports/android/app-icon                 # operator-pinned hand-built tree
```

For `role: app-icon` only, `sources.<platform>` MAY reference a **directory** (the export root) rather than a single file. Per-platform pins are independent: iOS may be hand-built while Android is auto-converted from `source:`, or vice versa.

Schema (`assets.schema.json`):

- Add optional property `app-icon: { "$ref": "#/$defs/assetId" }`.
- Cross-check: referenced id MUST exist under `assets:` and MUST have `role: app-icon`.
- For `role: app-icon`, relax `sources.ios` / `sources.android` to accept a directory path (export root) in addition to single-file paths used by other roles.

`role` enum gains `app-icon`.

#### 3.2 `inferred` on `symbolEntry` (optional)

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

The `app-icon` asset is special: stores and launchers require fixed shapes outside normal UI imagesets. One logical id (`app-icon:` pointer) covers all platforms; per-platform delivery is via auto-conversion from `source:` **or** operator-pinned exports under `design-system/assets/exports/<platform>/app-icon/`.

#### 4.1 Delivery paths (per platform)

Each **missing UI platform** in a bootstrap trigger (§6.1) MUST be satisfiable by **at least one** of:

| Path | When | Requirement |
|------|------|-------------|
| **A. Auto-convert** | Operator provides a straightforward canonical image | `source:` present (SVG or square raster ≥1024×1024); materializer derives platform exports into the conventional `exports/<platform>/app-icon/` tree |
| **B. Operator-pin** | Operator needs platform-specific treatment (glass, adaptive tuning, etc.) | Hand-built artifacts committed under `exports/<platform>/app-icon/` in the platform-acceptable layout (§4.2 / §4.3); `sources.<platform>` points at the export root |

If neither path is satisfiable for a missing platform, validation **hard-fails** (`plan-bootstrap-app-icon-missing` or `assets-app-icon-invalid`). There is no silent fallback, placeholder generation, or writer-side conversion at build time.

Canonical `source:` constraints (path A):

| Constraint | Rule |
|------------|------|
| Format | SVG (preferred) or square raster (≥1024×1024) |
| Canvas | Square 1:1; no iOS/Android corner masking in source |
| Transparency | Avoid for iOS marketing icon (Apple rejects alpha on App Store icon); Android adaptive foreground may use transparency |
| Safe zone | For Android adaptive auto-convert: keep logo inside central ~66% ("mask" safe area) |
| Color | Full-color brand mark; no platform chrome |

#### 4.2 iOS (`exports/ios/app-icon/AppIcon.appiconset`)

**Export root (committed):** `design-system/assets/exports/ios/app-icon/AppIcon.appiconset/`

**Auto-convert (path A):** materializer writes a single **1024×1024** opaque PNG plus `Contents.json` (`idiom: universal`, `platform: ios`) from `source:`.

**Operator-pin (path B):** designer commits a complete `AppIcon.appiconset/` under the export root (e.g. glass or depth effects baked into the PNG). Validation checks actool-safe layout: valid `Contents.json`, at least one 1024×1024 PNG entry, no raw SVG in the appiconset.

**Shell copy:** writer copies from the export root into `iOS/<App>/Resources/Assets.xcassets/AppIcon.appiconset/` (scaffold creates empty slot; bootstrap build fills it). Writer never deletes operator overrides outside generated filenames in the shell tree.

| Requirement | Detail |
|-------------|--------|
| Xcode setting | `ASSETCATALOG_COMPILER_APPICON_NAME = AppIcon` (XcodeGen default — scaffold MUST ship the appiconset, not rely on implicit name) |
| Minimum for simulator / debug | Single **1024×1024** PNG, `idiom: universal`, `platform: ios` (iOS 11+ single-size model) |
| Store / release | Same 1024 PNG or full slot grid when operator-pinned |

#### 4.3 Android (`exports/android/app-icon/`)

**Export root (committed):** `design-system/assets/exports/android/app-icon/`

**Auto-convert (path A):** materializer writes the adaptive + legacy mipmap tree under the export root from `source:`.

**Operator-pin (path B):** designer commits a complete launcher tree under the export root:

| Artifact | Purpose |
|----------|---------|
| `mipmap-anydpi-v26/ic_launcher.xml` | Adaptive icon definition |
| `mipmap-anydpi-v26/ic_launcher_round.xml` | Round launcher variant |
| `drawable/ic_launcher_foreground.xml` or density PNGs | Foreground layer |
| `values/ic_launcher_background.xml` or `color` resource | Background (from `tint` token ref on asset entry when auto-converting; operator-chosen when pinned) |
| `mipmap-{mdpi,hdpi,xhdpi,xxhdpi,xxxhdpi}/ic_launcher.png` | Legacy pre-API-26 fallback |

Validation checks required artifacts exist and referenced XML/PNG formats are well-formed. Writer copies the export tree into `Android/app/src/main/res/`; `AndroidManifest.xml` continues `android:icon="@mipmap/ic_launcher"`.

#### 4.4 Bootstrap placeholder policy

When §6.1 fires, validation requires a satisfiable `app-icon` per missing platform (§4.1) — not merely a YAML field. Operators MAY commit a deliberately ugly placeholder SVG for path A or ugly hand-built PNGs for path B; auto-generated brand-colored placeholders without designer input remain **deferred** (optional future `app-icon: { generated: true }` — out of scope for v1).

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

`core`-only bootstrap (`app-foundation` with only `core` missing) does **not** require `app-icon`. Trigger fires when **`ios` or `android` is among missing platforms** for the project. Web bootstrap (`bootstrap-web`) is deferred to [RFC-45a](future/rfc-45a-web-asset-materialization.md).

#### 6.2 Validation rule

When §6.1 triggers for project `P`, evaluate **only the missing UI platforms** in the trigger set. For each such platform `π`:

1. **Shell-resident escape hatch.** If the on-disk shell for `π` already carries a satisfiable launcher icon (see §6.3), validation for `π` **passes** — no design-system `app-icon` inventory is required. This covers incremental plans, re-bootstrap after a prior plan, and operator-authored shell icons that predate `assets.yaml`.
2. **Design-system satisfaction.** Otherwise resolve `design-system/assets.yaml` for `P` and require the `app-icon` entry to satisfy §4.1 for `π` via path A (canonical `source:` materializable) **or** path B (operator-pinned exports at the conventional `exports/<π>/app-icon/` layout in the platform-acceptable format).

**Error** `plan-bootstrap-app-icon-missing` (plan validate) or `assets-app-icon-invalid` (asset validate) when step 1 does not pass for `π` **and** step 2 fails because:

- `assets.yaml` absent, or
- top-level `app-icon` absent, or
- `app-icon` id not found under `assets:`, or
- referenced entry lacks `role: app-icon`, or
- for `π`: neither path A nor path B is satisfiable — e.g. no `source:` and no valid operator-pinned export tree, or `source:` present but materialize check-only mode cannot derive exports and no pin exists, or pinned tree exists but fails format/layout checks (§4.2 / §4.3).

When §6.1 does **not** trigger, `app-icon` inventory is not gated at plan validate or slice build prepare — existing shell launcher icons and prior bootstrap output are sufficient.

Gate placement:

| Gate | Enforced |
|------|----------|
| `specify plan validate` | Yes — blocks Gate 1 stamp when §6.1 triggers and any missing platform fails §6.2 |
| `specify plan transition <name> approved` | Indirect (validate should run first) |
| `specify slice build --phase prepare` | Yes — same bootstrap-only rule when the build targets a platform that §6.1 would treat as missing and §6.2 has not yet been satisfied for that platform |
| Incremental feature slices on existing shells | No `app-icon` gate |

`vectis validate assets` gains structural `app-icon` checks (format, export layout) but does not know plan bootstrap context; plan validate owns the conditional gate and shell-resident escape hatch.

#### 6.3 Shell-resident launcher icon detection

When §6.1 reports platform `π` absent from the reconcile heuristic (`detect_missing_platforms`), the shell tree may still exist with a launcher icon from a prior bootstrap or operator work. Before requiring design-system inventory, validation probes the shell:

| Platform | Satisfied when |
|----------|----------------|
| **iOS** | `iOS/*/Resources/Assets.xcassets/AppIcon.appiconset/Contents.json` exists **and** at least one referenced PNG is present on disk |
| **Android** | `Android/app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml` exists **or** legacy `mipmap-*/ic_launcher.png` exists |

Exact path heuristics align with `vectis verify --mode detect` shell layout assumptions. A skeleton appiconset with no PNG does **not** satisfy the escape hatch.

#### 6.4 Interaction with `app-foundation` slice

Greenfield reconcile inserts `app-foundation` when all supported platforms are missing. That slice SHOULD depend on design-system existing with `tokens.yaml`, `assets.yaml` (including a satisfiable `app-icon` per §4.1 for each missing platform), and committed `exports/` (auto-converted or operator-pinned) before iOS/Android bootstrap build slices run. Plan DAG:

```text
design-system  →  app-foundation (scaffold shells)  →  feature slices
     ↑
  must satisfy app-icon (path A or B) before Gate 1 when bootstrap trigger fires
     and no shell-resident launcher icon yet exists for the missing platform
```

### 7. Validation extensions (ongoing)

Extend existing `vectis validate assets` (composition-referenced assets):

| Check | Severity |
|-------|----------|
| Composition-referenced `vector`/`raster` lacks `sources.<platform>` **and** no export file | error |
| `role: app-icon` export tree fails layout/format checks (§4.2 / §4.3) | error |
| `sources.ios` ends in `.svg` for `role: illustration` | warning (error after materialize mandate) |
| `sources.ios` ends in `.svg` for `role: app-icon` export | error |
| Platform set from `project.yaml.platforms` instead of hardcoded `["ios","android"]` | error when missing |
| Shell tree missing catalog entry for referenced non-symbol asset | `vectis verify --mode verify` |
| `app-icon` missing when bootstrap trigger fires and shell-resident escape hatch does not apply | error (`plan-bootstrap-app-icon-missing`) |

Diagnostic ids (illustrative): `assets-materialization-missing`, `assets-app-icon-invalid`, `assets-app-icon-export-invalid`, `assets-svg-illustration-on-ios`, `plan-bootstrap-app-icon-missing`.

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

- Schema: `app-icon`, `role: app-icon`, `inferred`; `sources.<platform>` directory paths permitted for `role: app-icon`.
- `specify plan validate`: bootstrap trigger + `plan-bootstrap-app-icon-missing` with shell-resident escape hatch (§6.3).
- `vectis validate assets`: `app-icon` export layout checks (§4.2 / §4.3); bootstrap context remains plan-only.
- Writer/review doc updates; review rule flagging symbol substitution.
- iOS scaffold: `AppIcon.appiconset` skeleton.

### Phase 2 — Materialize v1

- `vectis materialize assets`: icons (SVG→PDF / VD XML), illustrations (SVG→PNG), `app-icon` auto-convert (iOS 1024 + Android adaptive) into `exports/<platform>/app-icon/`; skip operator-pinned platforms.
- Hook into `slice build --phase prepare` (bootstrap-only `app-icon` gate).
- Extend `vectis validate assets` for export presence and path A / path B satisfaction.
- Design-system docs and acceptance fixtures: commit `exports/` outputs; do not gitignore the tree.

### Phase 3 — Fidelity

- `vectis verify` catalog completeness check (optional `actool` dry-run).
- `exports.lock` / digest pinning if needed.

## Non-goals

- Requiring hand-built exports for every asset (auto-convert from `source:` remains the default; operator-pin is opt-in per platform when design demands it).
- Automatic symbol substitution at build time.
- Figma / screenshot asset extraction (screenshots remain non-destructive).
- Web asset materialization (`sources.web`, favicon / manifest icons, web render paths) and the web shell scaffold — deferred to [RFC-45a](future/rfc-45a-web-asset-materialization.md).
- Generic image CDN or remote asset hosting.

## Resolved decisions

1. **`exports/` committed vs gitignored?** **Commit.** Consumer repos version-control `design-system/assets/exports/` so CI and shell builds are reproducible without running `vectis materialize` (and without image-processing deps) on every job. Framework acceptance fixtures pin small committed PNG/PDF outputs under the same policy.
2. **Single global `app-icon` vs per-platform ids?** **One logical id**, per-platform delivery. The top-level `app-icon:` pointer references a single `role: app-icon` entry. Per-platform marks differ via independent `sources.ios` / `sources.android` pins under `exports/<platform>/app-icon/` (operator hand-built) or auto-conversion from shared `source:` — not separate asset ids or composition references. Bootstrap validation hard-fails when a missing platform has neither a materializable canonical image nor valid hand-built exports in the conventional export layout; it does not fire on incremental plans when the shell already carries a launcher icon (§6.2 / §6.3).

## Open questions

1. **Plan trigger B without bootstrap slices?** If operator declines `--reconcile-platforms` but platforms are still absent, validate still fails at Gate 1 — intentional force function.
2. **Raster-only design systems?** `kind: raster` + `role: app-icon` with `source:` as a ≥1024×1024 master; materialize copies/resizes into export trees. Operator-pin path unchanged.
3. **Pin vs `source:` drift.** When an operator updates `source:` but leaves platform pins in place, should `materialize` emit a warning, or should validate flag `assets-app-icon-source-stale`? v1: silent skip (pins win); revisit if operators report confusion.
4. **`assets.yaml` auto-write of `sources.<platform>` after first materialize.** v1 leaves recording pins to the operator; auto-write would reduce toil but couples materialize to manifest mutation.

## References

- [`adapters/targets/vectis/references/ios/design-system-integration.md`](../adapters/targets/vectis/references/ios/design-system-integration.md) — current copy-on-generate contract
- [`wasi-tools/vectis/embedded/assets.schema.json`](https://github.com/augentic/specify-cli/blob/main/wasi-tools/vectis/embedded/assets.schema.json) — assets artifact schema (`specify-cli`)
- [`wasi-tools/vectis/src/validate/engine/assets.rs`](https://github.com/augentic/specify-cli/blob/main/wasi-tools/vectis/src/validate/engine/assets.rs) — cross-artifact validation (`specify-cli`)
- [`wasi-tools/vectis/src/verify.rs`](https://github.com/augentic/specify-cli/blob/main/wasi-tools/vectis/src/verify.rs) — platform shell detect/verify (`specify-cli`)
- [`crates/workflow/src/change/plan/core/propose/platforms.rs`](https://github.com/augentic/specify-cli/blob/main/crates/workflow/src/change/plan/core/propose/platforms.rs) — `reconcile_platforms` / bootstrap slice insertion (`specify-cli`)
- [`adapters/targets/vectis/briefs/build/ios/write.md`](../adapters/targets/vectis/briefs/build/ios/write.md) — verify loop (`make sim-build`)
- Apple Human Interface Guidelines — App Icon (1024×1024, no alpha)
- Android Adaptive Icons — foreground/background safe zone

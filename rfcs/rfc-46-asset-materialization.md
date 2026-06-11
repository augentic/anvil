# RFC-46: Asset Materialization and Mandatory App Icon

> Status: Draft · Serves: Vectis target adapter, `design-system/assets.yaml`, `vectis` WASI tool · Motivated by: iOS `actool` failures on unmaterialized assets and silent substitution of designer SVGs with platform symbols · **Scope includes Phase 0** (specify-cli): remove the optional `--reconcile-platforms` propose flag and make Vectis self-contained for shell-bootstrap detection

## Abstract

Vectis today treats `design-system/assets.yaml` as an inventory while shell writers often render platform-native symbols (`SF Symbols`, Material Icons) instead of the designer's assets. Canonical inputs are frequently SVG (scalable, web-friendly), but iOS and Android require platform-specific materialized artifacts. This RFC introduces:

1. A deterministic **`vectis materialize assets`** step that converts canonical sources into per-platform exports.
2. A strict **render-by-`kind`** writer contract: `vector` / `raster` assets always render from materialized shell resources; `symbol` is the only explicit glyph path.
3. A top-level **`app-icon`** field in `assets.yaml` pointing at a `role: app-icon` entry.
4. A **bootstrap-only validation gate** that hard-fails when a Vectis-bound project is about to bootstrap a missing UI shell platform and no satisfiable `app-icon` exists — neither a straightforward canonical master (SVG or square raster ≥1024×1024) the materializer can convert, nor operator-pinned hand-built exports in the expected `exports/<platform>/` layout for each missing platform. Plans that reuse shells with an existing launcher icon proceed without re-checking design-system inventory.
5. **Phase 0 (prerequisite):** close a pre-existing workflow footgun by making platform-bootstrap slice insertion automatic on `specify plan propose --from` and moving shell-presence detection into the **vectis** tool (`vectis verify --mode detect`). Non-Vectis targets (e.g. Omnia microservices) are unaffected.

SVG remains the canonical designer format. Mobile shells (iOS, Android) consume derived exports. Web asset materialization is out of scope here and specified separately in [RFC-46a](future/rfc-46a-web-asset-materialization.md), deferred until a web shell scaffold exists.

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
- **Auto-convert or operator-pin.** Default path: `vectis materialize assets` performs straightforward format conversion from `source:` into `exports/<platform>/` and auto-writes absent `sources.<platform>` pins (§2, Resolved decisions §7). When a platform needs designer-specific treatment (e.g. iOS glass, Android adaptive layers hand-tuned in graphic tools), the operator commits hand-built artifacts under the conventional `exports/<platform>/` tree and pins paths in `sources.<platform>`. The materializer MUST NOT overwrite operator-pinned paths; it fills only missing slots from `source:` when no pin exists.
- **Commit materialized exports.** `design-system/assets/exports/` is version-controlled in consumer repos alongside canonical `source:` files. CI and shell builds consume committed exports; they do not require `vectis materialize` (or image-processing deps) on every job. Operators re-run materialize after editing canonical assets and commit the regenerated tree.
- **Bootstrap-only `app-icon` gate.** Mandatory `app-icon` validation runs only when §6.1 detects UI shell bootstrap for a platform. Incremental plans against shells that already carry a launcher icon (from a prior bootstrap or operator-authored shell resources) are not blocked by design-system `app-icon` inventory.
- **Fail closed on missing materialization.** A composition-referenced `vector` / `raster` id without exports for a declared project platform is an error — never a silent symbol fallback at build time.
- **Symbols are explicit inventory.** Platform glyph use requires `kind: symbol` on an `assets.yaml` entry (optionally `inferred: true` when promoted from screenshots). Composition still references the asset id.
- **CLI owns determinism.** Materialization, catalog emission, shell-presence detection, and bootstrap validation live in `vectis` / `specify plan validate`. Shell writers copy pre-validated outputs and emit view code; they do not convert formats or invent icons. Crux shell layout heuristics (`shared/`, `iOS/`, `Android/`) live in the vectis tool — not in `specify-workflow`.
- **Automatic bootstrap inference (Phase 0).** When `propose --from` runs for a project with declared `platforms`, the CLI always consults vectis detect and inserts bootstrap slices (`app-foundation`, `bootstrap-<platform>`) for absent shells. Operators do not pass a separate reconcile flag.
- **Minimal schema growth.** One top-level pointer (`app-icon`), one new `role`, one optional `inferred` flag on symbol entries, and one carve-out (`source:` on `rasterEntry` only for `role: app-icon`). No per-composition-item render mode. (Web adds one optional `sources.web` later — see [RFC-46a](future/rfc-46a-web-asset-materialization.md).)

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

**Operator pins:** when `sources.<platform>` is already set and the referenced path exists on disk, `materialize assets` skips that platform slot for the asset — silently; pins win over `source:` (see Resolved decisions §6). When `sources.<platform>` is absent, the materializer writes to the conventional default under `exports/<platform>/` and **MUST** record the resulting path in `assets.yaml` under `sources.<platform>` for each slot it just filled (Resolved decisions §7). Operator-pinned exports take precedence over `source:` for that platform; editing `source:` alone does not regenerate a pinned platform until the operator clears the pin or deletes the pinned tree. Validate does not flag `source:` / pin drift.

**Invocation points:**

| Phase | When |
|-------|------|
| `specify slice build --phase prepare` | Sole automatic hook inside the plan loop (§2.1). Resolve the effective `assets.yaml` (slice-local when present, otherwise `design-system/assets.yaml` — same precedence as the Vectis build brief). Auto-run `materialize assets` when any in-scope asset lacks exports on disk: composition-referenced `vector` / `raster` ids for the active slice, plus `role: app-icon` when §6.1 bootstrap context applies. Idempotent across slices and build retries; incremental when later slices introduce new unpinned inventory. |
| Operator | Regen after editing canonical masters (`source:`); commit the updated `exports/` tree and auto-written pins in the same change |
| CI (default) | Relies on committed `exports/`; does not run materialize unless a job explicitly checks freshness |

#### 2.1 Prepare hook (inventory resolution and scope)

`specify slice build --phase prepare` is the only in-loop caller — skills do not duplicate materialize authority. Prepare runs **before** the build brief (including composition regeneration), so exports and pins exist before `vectis validate` runs inside the brief.

**Inventory path.** Resolve the effective `assets.yaml` with the same precedence the Vectis build brief uses: `${SLICE_DIR}/assets.yaml` when present, otherwise `${PROJECT_DIR}/design-system/assets.yaml`. A feature slice without slice-local `assets.yaml` still materializes incrementally against the project-level inventory. Auto-write targets the same file materialize read.

**In-scope assets.** Materialize only platform slots that are missing on disk (no satisfiable pin per §6). Prepare runs before the brief regenerates `composition.yaml`, so derive the reference set from: (a) the slice's prior `composition.yaml` when present from an earlier `/spec:build` iteration, else (b) asset ids referenced in `spec.md` / `design.md` observable behaviour, plus (c) the full inventory with `source:` and no satisfiable pin when the effective `assets.yaml` is slice-local (typical `design-system` slice bulk pass). Within that set:

- Every in-scope `vector` / `raster` id.
- `role: app-icon` for each declared platform in bootstrap context (§6.1) when §6.2 is not yet satisfied — including `app-foundation` / `bootstrap-*` builds that carry no slice-local inventory.

**Multi-slice plans.** `/spec:execute` runs `refine → build → merge` sequentially. Expect one bulk pass when the first slice with unpinned inventory builds (typically `design-system` or operator pre-work before Gate 1), then zero or more incremental passes when a later slice's inventory delta introduces new `source:` entries without pins. Re-running `/spec:build` on the same slice is a no-op once exports and pins are committed. Detection may surface at `plan validate` (check-only, §6) and again at each prepare — side effects remain idempotent.

**Materialization strategy** (by `role` + `kind`):

| `role` | Canonical | iOS output | Android output |
|--------|-----------|------------|----------------|
| `icon` | SVG | PDF in `<id>.imageset/` | Vector Drawable XML in `drawable/` |
| `illustration` | SVG | PNG `@2x` / `@3x` in imageset | PNG per density bucket |
| `app-icon` | SVG or square raster (see §4) | `exports/ios/app-icon/AppIcon.appiconset/` (see §4) — auto-converted or operator-pinned | `exports/android/app-icon/` adaptive + legacy mipmaps (see §4) — auto-converted or operator-pinned |
| `photo` | raster | copy density slots | copy density slots |
| `decorative` | any | same as `icon` / `illustration` by `kind` | same |

Implementation lives in `wasi-tools/vectis` (pure Rust: `usvg` / `resvg` for SVG→PDF/PNG; Android Vector Drawable conversion as a dedicated pass; raster decode via `image` for PNG/JPEG/WebP masters). Complex SVG features that fail a lightweight profile check MUST surface as materialization errors with the offending asset id.

**`role: app-icon` path A** is format-agnostic: decode `source:` (SVG or allowed square raster — §4.1) to a **1024×1024** canvas, then emit §4.2 / §4.3 export trees through a shared launcher pipeline. `kind` describes the master file type; it does not gate whether materialize runs. Operator pins (path B) still win per platform.

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
  # Path A — SVG master (auto-convert)
  app-icon:
    kind: vector
    role: app-icon
    alt: "Application"
    source: assets/app-icon.svg

  # Path A — raster master (auto-convert); mutually exclusive id in real repos
  app-icon-png:
    kind: raster
    role: app-icon
    alt: "Application"
    source: assets/app-icon.png   # square ≥1024×1024; see §4.1

  # Path B — operator-pinned complete export trees (copy only)
  app-icon-pinned:
    kind: raster
    role: app-icon
    alt: "Application"
    sources:
      ios: assets/exports/ios/app-icon/AppIcon.appiconset
      android: assets/exports/android/app-icon
```

For `role: app-icon` only, `sources.<platform>` MAY reference a **directory** (the export root) rather than a single file. Per-platform pins are independent: iOS may be hand-built while Android is auto-converted from `source:`, or vice versa. When `source:` is present, `kind` MUST match the master file: `vector` for `.svg`, `raster` for `.png` / `.jpg` / `.jpeg` / `.webp`.

Schema (`assets.schema.json`):

- Add optional property `app-icon: { "$ref": "#/$defs/assetId" }`.
- Cross-check: referenced id MUST exist under `assets:` and MUST have `role: app-icon`.
- For `role: app-icon`, relax `sources.ios` / `sources.android` to accept a directory path (export root) in addition to single-file paths used by other roles.
- Add optional `source:` to `rasterEntry` **only when** `role: app-icon` (same `anyOf` as `vectorEntry`: at least one of `source` or `sources` required). Reject `source:` on `rasterEntry` for any other `role` — regular raster inventory stays per-density `sources` only (path B) or fails.

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

No other schema changes beyond `app-icon` / `inferred` / the `role: app-icon` raster `source:` carve-out above. Composition continues to reference asset ids only; `kind` on the asset entry selects the render path.

### 4. App icon requirements per platform

The `app-icon` asset is special: stores and launchers require fixed shapes outside normal UI imagesets. One logical id (`app-icon:` pointer) covers all platforms; per-platform delivery is via auto-conversion from `source:` **or** operator-pinned exports under `design-system/assets/exports/<platform>/app-icon/`.

#### 4.1 Delivery paths (per platform)

Each **missing UI platform** in a bootstrap trigger (§6.1) MUST be satisfiable by **at least one** of:

| Path | When | Requirement |
|------|------|-------------|
| **A. Auto-convert** | Operator provides a single master image and no platform pin | `source:` present (SVG or square raster ≥1024×1024 in PNG, JPEG, or WebP); materializer decodes to a 1024×1024 canvas and derives platform exports into the conventional `exports/<platform>/app-icon/` tree |
| **B. Operator-pin** | Operator needs platform-specific treatment (glass, adaptive tuning, etc.) | Hand-built artifacts committed under `exports/<platform>/app-icon/` in the platform-acceptable layout (§4.2 / §4.3); `sources.<platform>` points at the export root |

If neither path is satisfiable for a missing platform, validation **hard-fails** (`plan-bootstrap-app-icon-missing` or `assets-app-icon-invalid`). There is no silent fallback, placeholder generation, or writer-side conversion at build time.

Canonical `source:` constraints (path A):

| Constraint | Rule |
|------------|------|
| Format | SVG (`.svg`), or square raster in PNG, JPEG, or WebP (closed allow-list for v1) |
| `kind` | `vector` when `source:` is SVG; `raster` when `source:` is a raster extension — mismatch is a validation error |
| Dimensions | Width and height ≥1024 after decode; square 1:1 aspect ratio; no upscaling from smaller masters |
| Canvas | No iOS/Android corner masking baked into source |
| Transparency | Raster masters with alpha fail validation for iOS auto-convert (operator must supply opaque master or path B); Android adaptive foreground may use transparency |
| Safe zone | For Android adaptive auto-convert: keep logo inside central ~66% ("mask" safe area) |
| Color | Full-color brand mark; no platform chrome |

#### 4.2 iOS (`exports/ios/app-icon/AppIcon.appiconset`)

**Export root (committed):** `design-system/assets/exports/ios/app-icon/AppIcon.appiconset/`

**Auto-convert (path A):** materializer decodes `source:` (SVG via `resvg`, raster via `image`), normalizes to a **1024×1024** opaque PNG, and writes `Contents.json` (`idiom: universal`, `platform: ios`).

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

When §6.1 fires, validation requires a satisfiable `app-icon` per missing platform (§4.1) — not merely a YAML field. Operators MAY commit a deliberately ugly placeholder master (SVG or 1024×1024 PNG) for path A or ugly hand-built PNGs for path B; auto-generated brand-colored placeholders without designer input remain **deferred** (optional future `app-icon: { generated: true }` — out of scope for v1).

### 5. Inference-time symbol exception

Screenshot / layout inferers (`adapters/sources/screenshots`, `layout-inferer-contract.md`):

1. Shape matches `assets.<id>` → reference that id in layout/composition.
2. Shape is a generic platform glyph, **no** matching asset → MAY add `kind: symbol` entry (with `inferred: true`) or emit `notes.todo` for operator approval before merge.
3. Branded / custom shape → `notes.todo: add <id> to assets.yaml`; never auto-symbol.

Inferers MUST NOT crop production assets from screenshots (unchanged). Symbol promotion is inventory authoring, not a build shortcut.

### 6. Plan validation: mandatory `app-icon` on UI bootstrap

#### 6.1 Bootstrap trigger

For a **Vectis-bound** project `P`, UI shell bootstrap is implied at `specify plan validate` (and matching slice-build gates) when:

- `project.yaml.platforms` includes `ios` and/or `android`, **and**
- `vectis verify --mode detect` run against `P`'s project directory lists that platform in `missing[]`.

Detection is **filesystem-authoritative** — the same vectis detect pass Phase 0 runs on `propose --from`. `plan.yaml` slice names (`app-foundation`, `bootstrap-*`) are **not** a separate gate input; after Phase 0, propose always inserts those rows when vectis detect reports absences. Operators who hand-curate `plan.yaml` via `specify plan add` / `amend` and omit bootstrap rows are covered by the same detect pass: §6.2 and slice-build prepare gates key off on-disk shell absence, not on whether bootstrap slice names appear in the plan DAG. No additional structural finding (e.g. `plan-bootstrap-slices-missing`) is required — filesystem detect plus automatic propose on the default path is sufficient.

`core`-only bootstrap (`app-foundation` with only `core` among `missing[]`) does **not** require `app-icon`. The trigger fires only when **`ios` or `android` is among `missing[]`**. Web bootstrap (`bootstrap-web`) is deferred to [RFC-46a](future/rfc-46a-web-asset-materialization.md).

**Non-Vectis targets** (Omnia, contracts, …) do not declare shell `platforms` and never satisfy this trigger regardless of on-disk layout. Generalising bootstrap detection to future shell targets is out of scope (Phase 0).

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
| `specify slice build --phase prepare` | Yes — auto-materialize missing exports and auto-write absent `sources.<platform>` pins (§2.1); same bootstrap-only `app-icon` rule when §6.1 applies and §6.2 is not yet satisfied for the build's platform |
| Incremental feature slices on existing shells | No `app-icon` gate |

`vectis validate assets` gains structural `app-icon` checks (format, export layout) but does not know plan bootstrap context; plan validate owns the conditional gate and shell-resident escape hatch.

#### 6.3 Shell-resident launcher icon detection

When vectis detect reports platform `π` in `missing[]`, the shell tree may still carry a satisfiable launcher icon from a prior bootstrap or operator work that detect does not treat as a complete shell. Before requiring design-system inventory, validation probes the shell:

| Platform | Satisfied when |
|----------|----------------|
| **iOS** | `iOS/*/Resources/Assets.xcassets/AppIcon.appiconset/Contents.json` exists **and** at least one referenced PNG is present on disk |
| **Android** | `Android/app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml` exists **or** legacy `mipmap-*/ic_launcher.png` exists |

Exact path heuristics align with `vectis verify --mode detect` shell layout assumptions. A skeleton appiconset with no PNG does **not** satisfy the escape hatch.

#### 6.4 Interaction with `app-foundation` slice

Phase 0 propose inserts `app-foundation` when vectis detect reports all supported Crux platforms (`core`, `ios`, `android`) missing; incremental detect yields per-platform `bootstrap-<platform>` slices instead. That work SHOULD run only after design-system exists with `tokens.yaml`, `assets.yaml` (including a satisfiable `app-icon` per §4.1 for each missing UI platform), and committed `exports/` with recorded `sources.<platform>` pins (auto-converted or operator-pinned). The `design-system` slice's `build --phase prepare` is the usual bulk materialize pass; operators commit exports and auto-written pins at merge. Plan DAG:

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
| `role: app-icon` `kind` disagrees with `source:` extension | error (`assets-app-icon-kind-source-mismatch`) |
| `role: app-icon` raster `source:` below 1024×1024, non-square, or with alpha (iOS path A) | error (`assets-app-icon-source-invalid`) |
| `source:` on `kind: raster` entry where `role` is not `app-icon` | error |
| `sources.ios` ends in `.svg` for `role: illustration` | warning (error after materialize mandate) |
| `sources.ios` ends in `.svg` for `role: app-icon` export | error |
| Platform set from `project.yaml.platforms` instead of hardcoded `["ios","android"]` | error when missing |
| Shell tree missing catalog entry for referenced non-symbol asset | `vectis verify --mode verify` |
| `app-icon` missing when bootstrap trigger fires and shell-resident escape hatch does not apply | error (`plan-bootstrap-app-icon-missing`) |

Diagnostic ids (illustrative): `assets-materialization-missing`, `assets-app-icon-invalid`, `assets-app-icon-export-invalid`, `assets-app-icon-kind-source-mismatch`, `assets-app-icon-source-invalid`, `assets-svg-illustration-on-ios`, `plan-bootstrap-app-icon-missing`.

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
| `plugins/spec/skills/plan/SKILL.md` | Drop `--reconcile-platforms`; bootstrap inference is default-on |
| `specify-cli` `DECISIONS.md` / `AGENTS.md` | Platform reconciliation always-on; vectis-owned detect |

## Implementation phases

### Phase 0 — Platform bootstrap inference (prerequisite, specify-cli)

Close a pre-existing workflow footgun before Phase 1 `app-icon` gates land.

**Problem.** Today `specify plan propose --from` accepts an optional `--reconcile-platforms` flag. When omitted, the plan may list only feature slices while `project.yaml` still declares `ios`/`android` and no shell trees exist — a plan the repo cannot execute. RFC-46 §6 originally papered over this with dual bootstrap triggers (plan slice names *or* filesystem detect). Phase 0 removes the footgun so §6.1 needs only vectis detect.

**Policy.**

| Change | Detail |
|--------|--------|
| **Remove `--reconcile-platforms`** | `propose --from` always runs the bootstrap post-pass for projects with non-empty `project.yaml.platforms`. |
| **Vectis owns shell detection** | Delete Crux-specific `detect_missing_platforms` from `specify-workflow`. Propose and plan validate consult `specify tool run vectis -- verify --mode detect <project-dir>` for Vectis-bound projects. Vectis compares declared `platforms` to on-disk Crux trees (`shared/src/app.rs`, `iOS/**/*.swift`, `Android/**/*.kt`). |
| **Workflow keeps DAG insertion only** | `Plan::reconcile_platforms` remains in `specify-workflow`: prepend `app-foundation` / `bootstrap-<platform>`, wire `depends-on`. No target-specific presence rules in the kernel. |
| **Non-Vectis unaffected** | Omnia, contracts, and similar targets without `platforms.required` carry no platform list; bootstrap post-pass is a no-op; §6 does not apply. |
| **Future targets out of scope** | A later shell target with `platforms.required` must ship its own detect surface; RFC-46 does not generalise workflow heuristics. |

**Deliverables:** propose handler + clap flag removal; workflow `platforms.rs` heuristic deletion; plan skill prose; `DECISIONS.md` / `AGENTS.md` updates; acceptance tests updated to stop passing `--reconcile-platforms`.

**Dependency:** Phase 1 MUST NOT ship until Phase 0 merges — otherwise `app-icon` gating reintroduces the declined-reconcile ambiguity this RFC closes.

### Phase 1 — Policy and gates (no converter yet)

- Schema: `app-icon`, `role: app-icon`, `inferred`; `sources.<platform>` directory paths permitted for `role: app-icon`; optional `source:` on `rasterEntry` restricted to `role: app-icon`.
- `specify plan validate`: §6.1 vectis-detect bootstrap trigger + `plan-bootstrap-app-icon-missing` with shell-resident escape hatch (§6.3). Requires Phase 0.
- `vectis validate assets`: `app-icon` export layout checks (§4.2 / §4.3); bootstrap context is vectis detect, not plan slice names.
- Writer/review doc updates; review rule flagging symbol substitution.
- iOS scaffold: `AppIcon.appiconset` skeleton.

### Phase 2 — Materialize v1

- `vectis materialize assets`: icons (SVG→PDF / VD XML), illustrations (SVG→PNG), `app-icon` auto-convert from `source:` (SVG or square raster ≥1024 — decode → shared 1024 canvas → §4.2 / §4.3 export trees) into `exports/<platform>/app-icon/`; skip operator-pinned platforms; auto-write absent `sources.<platform>` pins (§7).
- Hook into `slice build --phase prepare` (§2.1): effective-inventory resolution, auto-materialize for in-scope missing exports, auto-write, and bootstrap-only `app-icon` gate (§6.2).
- Extend `vectis validate assets` for export presence and path A / path B satisfaction.
- Design-system docs and acceptance fixtures: commit `exports/` outputs and auto-written pins; do not gitignore the tree.

### Phase 3 — Fidelity

- `vectis verify` catalog completeness check (optional `actool` dry-run).
- `exports.lock` / digest pinning if needed.

## Non-goals

- Generalising platform-bootstrap detection or slice insertion to non-Vectis targets (Omnia microservices, contracts, …). Phase 0 explicitly limits shell detect to the vectis tool; future shell targets are a separate adapter concern.
- Requiring hand-built exports for every asset (auto-convert from `source:` remains the default for vector icons and `role: app-icon`; operator-pin is opt-in per platform when design demands it).
- Auto-generating per-density raster ladders for non–`app-icon` assets from a single PNG master (regular `kind: raster` inventory requires complete per-density pins or fails).
- Automatic symbol substitution at build time.
- Figma / screenshot asset extraction (screenshots remain non-destructive).
- Web asset materialization (`sources.web`, favicon / manifest icons, web render paths) and the web shell scaffold — deferred to [RFC-46a](future/rfc-46a-web-asset-materialization.md).
- Generic image CDN or remote asset hosting.

## Resolved decisions

1. **`exports/` committed vs gitignored?** **Commit.** Consumer repos version-control `design-system/assets/exports/` so CI and shell builds are reproducible without running `vectis materialize` (and without image-processing deps) on every job. Framework acceptance fixtures pin small committed PNG/PDF outputs under the same policy.
2. **Single global `app-icon` vs per-platform ids?** **One logical id**, per-platform delivery. The top-level `app-icon:` pointer references a single `role: app-icon` entry. Per-platform marks differ via independent `sources.ios` / `sources.android` pins under `exports/<platform>/app-icon/` (operator hand-built) or auto-conversion from shared `source:` — not separate asset ids or composition references. Bootstrap validation hard-fails when a missing platform has neither a materializable canonical image nor valid hand-built exports in the conventional export layout; it does not fire on incremental plans when the shell already carries a launcher icon (§6.2 / §6.3).
3. **Optional `--reconcile-platforms` vs bootstrap trigger?** **Resolved by Phase 0.** Remove the flag; bootstrap slice insertion is default-on for platform-declaring projects. The `app-icon` gate keys off vectis filesystem detect (§6.1) and the shell-resident escape hatch (§6.3) — not on whether bootstrap slice names appear in `plan.yaml` and not on a propose-time opt-out that today can write unexecutable plans.
4. **Manual plan DAG without bootstrap rows?** **Filesystem detect is sufficient.** Phase 0 covers the default `propose --from` path; operators may still curate `plan.yaml` via `specify plan add` / `amend` and omit bootstrap slices while vectis detect reports missing shells. `plan validate` does **not** emit a separate structural finding (e.g. `plan-bootstrap-slices-missing`). Bootstrap context for the §6 `app-icon` gate and slice-build prepare comes from vectis detect and the shell-resident escape hatch (§6.3), not from whether `app-foundation` / `bootstrap-*` rows appear in the plan DAG.
5. **Raster master on `role: app-icon` (path A)?** **Yes — format-agnostic in v1, `app-icon` only.** When the operator sets `source:` to a square master ≥1024×1024 (SVG, PNG, JPEG, or WebP) without a platform pin, Phase 2 materialize decodes it to a 1024×1024 canvas and emits §4.2 / §4.3 launcher export trees — same contract as an SVG master, not SVG-only. Schema: optional `source:` on `rasterEntry` restricted to `role: app-icon`; `kind: raster` + `.png`/`.jpg`/`.webp`, `kind: vector` + `.svg`; mismatch fails validation. `kind` describes the master file; it does not gate materialize. Operator-pinned complete export trees (path B) remain copy-only. Regular `kind: raster` UI icons (`role: icon`, `illustration`, …) do **not** gain `source:` — they require complete per-density `sources` (path B) or validation fails; Vectis does not invent density ladders from a lone PNG.
6. **Pin vs `source:` drift.** **Pins win.** When `sources.<platform>` is set and the referenced export path exists on disk, `materialize assets` skips that platform slot — no warning, no overwrite — even if `source:` was edited afterward. Validate does not emit `assets-app-icon-source-stale` or any other drift finding for stale canonical masters behind active pins. To pick up a new `source:` for a pinned platform, the operator clears the pin (or deletes the pinned export tree) and re-runs materialize.
7. **`assets.yaml` auto-write of `sources.<platform>` after materialize?** **Yes — auto-write absent pins only.** When `materialize assets` fills a platform slot from `source:` (no existing pin whose referenced path exists on disk), it MUST record the export path under `sources.<platform>` in the same `assets.yaml` it read, in the same invocation as the export write. It MUST NOT overwrite an existing pin (§6). Multiple `prepare` invocations across one plan are safe: the first pass bulk-fills inventory (typically the `design-system` slice or operator pre-work before Gate 1); later passes no-op on committed pins and auto-write only slots for newly introduced unpinned assets. `specify slice build --phase prepare` is the sole automatic in-loop caller; skills do not duplicate materialize authority. Prepare resolves the effective inventory path with slice-local → project-level precedence (§2.1) so feature slices without slice-local `assets.yaml` still trigger incremental materialize against `design-system/assets.yaml`. Operator path B (hand-built exports) remains operator-authored: the operator sets `sources.<platform>` explicitly; materialize skips those slots and never rewrites them.

## References

- [`adapters/targets/vectis/references/ios/design-system-integration.md`](../adapters/targets/vectis/references/ios/design-system-integration.md) — current copy-on-generate contract
- [`wasi-tools/vectis/embedded/assets.schema.json`](https://github.com/augentic/specify-cli/blob/main/wasi-tools/vectis/embedded/assets.schema.json) — assets artifact schema (`specify-cli`)
- [`wasi-tools/vectis/src/validate/engine/assets.rs`](https://github.com/augentic/specify-cli/blob/main/wasi-tools/vectis/src/validate/engine/assets.rs) — cross-artifact validation (`specify-cli`)
- [`wasi-tools/vectis/src/verify.rs`](https://github.com/augentic/specify-cli/blob/main/wasi-tools/vectis/src/verify.rs) — **authoritative** Crux shell detect/verify for Phase 0 and §6.1 (`specify-cli`)
- [`crates/workflow/src/change/plan/core/propose/platforms.rs`](https://github.com/augentic/specify-cli/blob/main/crates/workflow/src/change/plan/core/propose/platforms.rs) — `Plan::reconcile_platforms` bootstrap DAG insertion only; presence heuristics removed in Phase 0 (`specify-cli`)
- [`adapters/targets/vectis/briefs/build/ios/write.md`](../adapters/targets/vectis/briefs/build/ios/write.md) — verify loop (`make sim-build`)
- Apple Human Interface Guidelines — App Icon (1024×1024, no alpha)
- Android Adaptive Icons — foreground/background safe zone

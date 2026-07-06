# RFC-46a: Web Asset Materialization

> Status: Deferred · Extends: the shipped iOS/Android asset-materialization capability (baseline summarized below) · Depends on: a web shell scaffold (not yet specified) · Trigger: `web ∈ project.yaml.platforms` and a web shell target exists

## Abstract

The shipped iOS/Android asset-materialization capability closes the canonical-source → platform-binary → shell-resource → rendered-view loop for native shells and deliberately scopes web out so it ships as a single implementable initiative. This RFC captures the web pre-work so it is not lost: the `sources.web` schema field, the web render-by-`kind` path, web app-icon artifacts (favicon / manifest), and the build-time `app-icon` gate extension for `web`.

It is **deferred** until a web shell scaffold exists. Everything here is additive to the shipped iOS/Android design — no breaking change to `assets.schema.json`, the `role` / `kind` enums, the `exports/<platform>/` tree, or the validation gate's platform handling. The host [`Platform`](../../crates/workflow/src/platform.rs) enum already reserves `Web` as a placeholder; the vectis materialize [`Platform`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/extension/src/materialize/paths.rs) enum is `Ios | Android` today and must be extended for web.

## RFC-46 baseline (iOS/Android, shipped)

The iOS/Android contract this RFC extends is implemented across two repositories:

- **Canonical masters** live under `design-system/assets/`; **committed exports** under `design-system/assets/exports/<platform>/`.
- **Materialize:** operators run `specify extension run vectis -- materialize assets`; the in-loop caller is `specify slice build --phase prepare`, which auto-dispatches materialize for in-scope missing exports (see [DECISIONS §K](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/extension/DECISIONS.md#k--materialization-and-render-by-kind)).
- **Render-by-`kind`:** shell writers copy materialized exports and emit view code by entry `kind` — `vector` / `raster` from shell catalogs, `symbol` only via explicit `symbols.<platform>`. Build-time substitution of platform glyphs for `vector` / `raster` ids is forbidden ([VECTIS-006](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/rules/VECTIS-006-asset-render-by-kind.md)).
- **Bootstrap `app-icon` gate:** `project.yaml.platforms` is the sole authority for platform intent. The gate runs at **build prepare** (`vectis verify --mode bootstrap-app-icon`, re-raised by the host as `plan-bootstrap-app-icon-missing`) — not at plan validate. Platform shell bootstrap is not a plan concern ([plan-propose.md](https://github.com/augentic/specify-adapters/blob/main/shared/prose/references/runtime/cli/plan-propose.md)).

## Motivation

SVG was chosen as the canonical designer format precisely because it is small, scalable, and web-friendly. Unlike iOS/Android, the web shell can consume canonical SVG **directly** for most assets — the materialization burden is limited to the app icon (favicon / touch icon / manifest icon raster sizes). Keeping this design separate from the shipped iOS/Android capability lets each ship as its own single initiative:

- iOS/Android materialization — shipped.
- RFC-46a (this) — web materialization, shippable once a web shell target exists.

## Relationship to the shipped contract

This RFC reuses the iOS/Android contracts unchanged and only adds the web seam to each:

| Shipped contract | Extension surface | RFC-46a addition |
|------------------|-------------------|------------------|
| Render-by-`kind` writer contract (§1) | Future `references/web/design-system-integration.md`, [VECTIS-006](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/rules/VECTIS-006-asset-render-by-kind.md) web column | Web column |
| `specify extension run vectis -- materialize assets` (§2) | [`extension/src/materialize/`](https://github.com/augentic/specify-adapters/tree/main/targets/vectis/extension/src/materialize) | Web outputs per `role` |
| `assets.yaml` schema (§3) | [`extension/schemas/assets.schema.json`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/extension/schemas/assets.schema.json) | Optional `sources.web` field |
| App icon per platform (§4) | `materialize/` app-icon module, `exports/web/` | Web favicon / manifest artifacts |
| Build-time bootstrap `app-icon` gate (§5) | [`extension/src/verify/app_icon.rs`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/extension/src/verify/app_icon.rs) | Extend gate to `web` |

## Ownership (two-repo split)

| Concern | Repo | Primary files |
|---------|------|---------------|
| `sources.web` schema | specify-adapters | `extension/schemas/assets.schema.json`, `validate/engine/assets.rs` |
| Web materialize outputs | specify-adapters | `extension/src/materialize/` (new web paths in `paths.rs`, app-icon favicon/manifest module) |
| `--platform web` filter | specify-adapters | `materialize.rs` (`resolve_platform_filter` currently rejects `web`) |
| Prepare scope + auto-materialize | specify-adapters | [`extension/src/prepare.rs`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/extension/src/prepare.rs); host dispatches `vectis prepare build` from [`src/runtime/commands/slice/build.rs`](../../src/runtime/commands/slice/build.rs) |
| Bootstrap `app-icon` gate for `web` | specify-adapters | `extension/src/verify/app_icon.rs` (`UI_PLATFORMS` = `ios`, `android` today) |
| Render-by-`kind` web writer contract | specify-adapters | future `references/web/design-system-integration.md`, VECTIS-006 web column, `briefs/build/web/` sub-brief (blocked on web scaffold) |

## Design

### 1. Asset rendering contract (web shell writer)

Extends the shipped render-by-`kind` table with a web column:

| `assets.<id>.kind` | Web |
|--------------------|-----|
| `vector` | asset URL / inline SVG from `sources.web` (or `source`) |
| `raster` | raster URL |
| `symbol` | mapped web glyph |

The forbidden-substitution rule applies unchanged: a `vector` / `raster` id must never render as a glyph.

### 2. `sources.web` schema field

Optional on `vectorEntry` / `rasterEntry`:

```yaml
sources:
  web: assets/app-icon.svg   # defaults to `source` when omitted
```

The web shell reads `sources.web` or falls back to `source` directly; no PDF/PNG conversion is required for v1 web vector assets. Adding this optional field to [`assets.schema.json`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/extension/schemas/assets.schema.json) is backward-compatible with existing documents.

### 3. Web materialization outputs

Extends the materialization-strategy table with a web column:

| `role` | Web output |
|--------|------------|
| `icon` | copy / link SVG |
| `illustration` | SVG |
| `app-icon` | favicon + manifest icons (see §4) |
| `photo` | copy |
| `decorative` | same as `icon` / `illustration` by `kind` |

`specify extension run vectis -- materialize assets --platform web` produces these outputs (also auto-dispatched at `specify slice build --phase prepare` when in-scope exports are missing). For non-app-icon assets web is largely a passthrough; the app icon is the only role requiring raster derivation.

### 4. Web app icon artifacts

When `web ∈ project.yaml.platforms` and the web shell is implemented, the materializer derives from the canonical `app-icon` entry:

| Artifact | Size / notes |
|----------|--------------|
| `favicon.svg` | Canonical passthrough when source is SVG |
| `favicon.ico` | 16, 32, 48 fallback |
| `apple-touch-icon.png` | 180×180 |
| `manifest` icons | 192, 512 PNG |

The materializer writes to `web/public/icons/` or equivalent; the exact tree is owned by the web shell scaffold (a separate RFC). Outputs land under `design-system/assets/exports/web/` following the committed-exports policy.

### 5. Build-time bootstrap `app-icon` gate (web extension)

Extends [DECISIONS §L](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/extension/DECISIONS.md#l--bootstrap-app-icon-gate) for the deferred web platform:

| Trigger | Detection |
|---------|-----------|
| **Web UI platform declared** | `web ∈ project.yaml.platforms` |

When `web` is a declared UI platform and a web shell exists:

- Extend the existing `bootstrap-app-icon` gate (`specify extension run vectis -- verify --mode bootstrap-app-icon`) to treat `web` as a UI platform alongside `ios` / `android`.
- Require materialized web app-icon exports under `design-system/assets/exports/web/` (or operator-pinned `sources.web` on the `app-icon` entry) — same path A (canonical `source:` materializable) / path B (operator-pinned export tree) model as iOS/Android.
- Enforcement point stays **`specify slice build --phase prepare`** (host re-raises error-severity findings as `plan-bootstrap-app-icon-missing`).

There is no plan-time bootstrap trigger: no `plan.yaml` entry flag, no reconcile slice name, and no shell-detect bootstrap insertion at `specify plan propose`.

`vectis verify` (default `verify` mode) still emits `platform-not-yet-supported` (info) for `web` until a web shell has an on-disk interpretation ([DECISIONS §J](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/extension/DECISIONS.md#j--platform-shell-verification)). **bootstrap-app-icon** and **verify** are separate modes; web bootstrap can ship before verify gains a web shell tree check.

### 6. Validation extensions

Extends the vectis extension validation gate ([`validate/engine/assets.rs`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/extension/src/validate/engine/assets.rs)):

| Check | Finding id | Severity |
|-------|------------|----------|
| Composition-referenced `vector`/`raster` lacks `sources.web` **and** no canonical `source` | `assets-materialization-missing` | error |
| `web ∈ project.yaml.platforms` but `app-icon` has no materializable web exports | `plan-bootstrap-app-icon-missing` | error (via prepare dispatch) |

## Implementation phases

This RFC is a single initiative once a web shell target exists:

1. **Schema + policy (specify-adapters)** — add optional `sources.web` to `vectorEntry` / `rasterEntry` in `assets.schema.json`; extend `check_platform_coverage`; add DECISIONS §K web column notes.
2. **Materialize web (specify-adapters)** — extend `materialize/paths::Platform` with `Web`; implement passthrough + favicon/manifest raster derivation; enable `--platform web`; commit `exports/web/` in eval fixtures when web scaffold lands.
3. **Adapters prepare hook** — extend `prepare build` scope resolution and export probes (`exports.rs` / `export_layout`) for `web`; ensure scoped materialize passes `web` in `--platform` when `web ∈ project.yaml.platforms`.
4. **Bootstrap gate (specify-adapters + host)** — extend `verify/app_icon.rs`; prepare dispatch is manifest-driven (`prepare.argv` on the target adapter) and delegates web work to the adapter's `prepare build` subcommand.
5. **Web shell consumption (blocked)** — web writer reads `sources.web` / `source` and copies materialized icon tree; requires web scaffold RFC + `briefs/build/web/` sub-brief.

## Non-goals

- Defining the web shell scaffold itself (component tree, build system, routing) — a separate RFC.
- Re-specifying iOS/Android contracts; this RFC only adds the web seam.
- Generic image CDN or remote asset hosting (inherited non-goal from the shipped capability).
- Plan-time shell bootstrap or `bootstrap-web` slice insertion.

## References

**Canonical implementation spec (shipped iOS/Android):**

- [specify-adapters `targets/vectis/extension/DECISIONS.md` §K / §L](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/extension/DECISIONS.md#rfc-46--asset-materialization)

**RFC-46a touch surfaces:**

- [`specify-adapters/.../schemas/assets.schema.json`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/extension/schemas/assets.schema.json)
- [`specify-adapters/.../src/materialize/`](https://github.com/augentic/specify-adapters/tree/main/targets/vectis/extension/src/materialize)
- [`specify-adapters/.../src/validate/engine/assets.rs`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/extension/src/validate/engine/assets.rs)
- [`specify-adapters/.../src/verify/app_icon.rs`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/extension/src/verify/app_icon.rs)
- [`specify-adapters/.../prepare.rs`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/extension/src/prepare.rs)
- [`specify/engine/.../slice/build.rs`](../../src/runtime/commands/slice/build.rs) (prepare hook)
- [`specify/engine/.../platform.rs`](../../crates/workflow/src/platform.rs)
- [VECTIS-006](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/rules/VECTIS-006-asset-render-by-kind.md) (future web column)
- [plan-propose.md](https://github.com/augentic/specify-adapters/blob/main/shared/prose/references/runtime/cli/plan-propose.md) (explicit non-goal: no plan-time shell bootstrap)
- W3C Web App Manifest — icon sizes (192, 512)
- Apple — `apple-touch-icon` (180×180)

# RFC-46a: Web Asset Materialization

> Status: Deferred · Extends: [RFC-46](../rfc-46-asset-materialization.md) (asset materialization and mandatory app icon) · Depends on: a web shell scaffold (not yet specified) · Trigger: `web ∈ project.yaml.platforms` and a web shell target exists

## Abstract

[RFC-46](../rfc-46-asset-materialization.md) closes the canonical-source → platform-binary → shell-resource → rendered-view loop for iOS and Android, and deliberately scopes web out so it ships as a single implementable initiative. This RFC captures the web pre-work extracted from RFC-46 so it is not lost: the `sources.web` schema field, the web render-by-`kind` path, web app-icon artifacts (favicon / manifest), and the `bootstrap-web` plan trigger.

It is **deferred** until a web shell scaffold exists. Everything here is additive to the RFC-46 design — no breaking change to `assets.schema.json`, the `role` / `kind` enums, the `exports/<platform>/` tree, or the validation gate's platform handling. The `Platform` enum already reserves `Web` as a placeholder.

## Motivation

SVG was chosen as the canonical designer format precisely because it is small, scalable, and web-friendly (RFC-46 §Motivation, Principles). Unlike iOS/Android, the web shell can consume canonical SVG **directly** for most assets — the materialization burden is limited to the app icon (favicon / touch icon / manifest icon raster sizes). Keeping this design separate from RFC-46 lets each ship as its own single initiative:

- RFC-46 — iOS/Android materialization, shippable now.
- RFC-46a (this) — web materialization, shippable once a web shell target exists.

## Relationship to RFC-46

This RFC reuses RFC-46's contracts unchanged and only adds the web seam to each:

| RFC-46 contract | RFC-46a addition |
|-----------------|------------------|
| Render-by-`kind` writer contract (§1) | Web column |
| `vectis materialize assets` (§2) | Web outputs per `role` |
| `assets.yaml` schema (§3) | Optional `sources.web` field |
| App icon per platform (§4) | Web favicon / manifest artifacts |
| Plan bootstrap validation (§6) | `bootstrap-web` trigger |

## Design

### 1. Asset rendering contract (web shell writer)

Extends the RFC-46 §1 render-by-`kind` table with a web column:

| `assets.<id>.kind` | Web |
|--------------------|-----|
| `vector` | asset URL / inline SVG from `sources.web` (or `source`) |
| `raster` | raster URL |
| `symbol` | mapped web glyph |

The RFC-46 §1 forbidden-substitution rule applies unchanged: a `vector` / `raster` id must never render as a glyph.

### 2. `sources.web` schema field

Optional on `vectorEntry` / `rasterEntry`:

```yaml
sources:
  web: assets/app-icon.svg   # defaults to `source` when omitted
```

The web shell reads `sources.web` or falls back to `source` directly; no PDF/PNG conversion is required for v1 web vector assets. Adding this optional field to `assets.schema.json` is backward-compatible with RFC-46-era documents.

### 3. Web materialization outputs

Extends the RFC-46 §2 materialization-strategy table with a web column:

| `role` | Web output |
|--------|------------|
| `icon` | copy / link SVG |
| `illustration` | SVG |
| `app-icon` | favicon + manifest icons (see §4) |
| `photo` | copy |
| `decorative` | same as `icon` / `illustration` by `kind` |

`vectis materialize assets --platform web` produces these outputs. For non-app-icon assets web is largely a passthrough; the app icon is the only role requiring raster derivation.

### 4. Web app icon artifacts

When `web ∈ project.yaml.platforms` and the web shell is implemented, the materializer derives from the canonical `app-icon` entry (RFC-46 §3.1, §4.1):

| Artifact | Size / notes |
|----------|--------------|
| `favicon.svg` | Canonical passthrough when source is SVG |
| `favicon.ico` | 16, 32, 48 fallback |
| `apple-touch-icon.png` | 180×180 |
| `manifest` icons | 192, 512 PNG |

The materializer writes to `web/public/icons/` or equivalent; the exact tree is owned by the web shell scaffold (a separate RFC). Outputs land under `design-system/assets/exports/web/` following RFC-46's committed-exports policy (§Principles).

### 5. Plan validation: `bootstrap-web` trigger

Extends RFC-46 §6.1 with the deferred web trigger:

| Trigger | Detection |
|---------|-----------|
| **C. Explicit web bootstrap** | A `plan.yaml` entry or proposal flag declares `bootstrap-web` (or a `{project}-bootstrap-web` reconcile equivalent), or `project.yaml.platforms` includes `web` and `detect_missing_platforms` reports it absent |

When this trigger fires, the RFC-46 §6.2 `plan-bootstrap-app-icon-missing` rule applies with `web` added to the missing-UI-platform set: materialized web app-icon exports must be present (or producible in check-only mode).

### 6. Validation extensions

Extends RFC-46 §7:

| Check | Severity |
|-------|----------|
| Composition-referenced `vector`/`raster` lacks `sources.web` **and** no canonical `source` | error |
| `web ∈ project.yaml.platforms` but `app-icon` has no materializable web exports | error (via `plan-bootstrap-app-icon-missing`) |

## Implementation phases

This RFC is a single initiative once a web shell target exists. Its phases mirror RFC-46:

1. **Schema + policy** — add `sources.web`; add `bootstrap-web` trigger to plan validate; web render-by-`kind` writer rules.
2. **Materialize web** — `vectis materialize assets --platform web`: vector passthrough + app-icon favicon/manifest raster derivation; commit `exports/web/`.
3. **Web shell consumption** — web shell reads `sources.web` / `source` and the materialized icon tree.

## Non-goals

- Defining the web shell scaffold itself (component tree, build system, routing) — a separate RFC.
- Re-specifying RFC-46's iOS/Android contracts; this RFC only adds the web seam.
- Generic image CDN or remote asset hosting (inherited non-goal from RFC-46).

## References

- [RFC-46: Asset materialization and mandatory app icon](../rfc-46-asset-materialization.md) — canonical/materialize contract this RFC extends
- [`wasi-tools/vectis/embedded/assets.schema.json`](https://github.com/augentic/specify-cli/blob/main/wasi-tools/vectis/embedded/assets.schema.json) — assets artifact schema (`specify-cli`)
- [`crates/workflow/src/change/plan/core/propose/platforms.rs`](https://github.com/augentic/specify-cli/blob/main/crates/workflow/src/change/plan/core/propose/platforms.rs) — `reconcile_platforms` / bootstrap slice insertion (`specify-cli`)
- W3C Web App Manifest — icon sizes (192, 512)
- Apple — `apple-touch-icon` (180×180)

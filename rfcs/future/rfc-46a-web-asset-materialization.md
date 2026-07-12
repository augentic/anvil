# RFC-46a: Web Asset Materialization

> Status: Deferred · Extends: the shipped iOS/Android asset-materialization capability (in-guest vectis build prelude) · Depends on: a web shell scaffold (not yet specified) · Trigger: `web ∈ project.yaml.platforms` and a web shell target exists

## Problem

The shipped asset-materialization capability closes the canonical-source → platform-binary → shell-resource → rendered-view loop for iOS and Android: canonical masters live under `design-system/assets/`, committed exports under `design-system/assets/exports/<platform>/`, the vectis guest's build prelude auto-materializes in-scope missing exports inside the guest-routed `specify slice build`, and shell writers render by `assets.yaml` entry `kind` (never substituting platform glyphs for `vector` / `raster` ids). Web is deliberately out of scope.

When a web shell target exists, the same contract needs a web seam — all additive, no breaking change to `assets.schema.json`, the `role` / `kind` enums, the `exports/<platform>/` tree, or the bootstrap `app-icon` gate's platform handling:

- an optional `sources.web` field on `vectorEntry` / `rasterEntry` (the web shell can consume canonical SVG directly, so `source` is the default and no PDF/PNG conversion is needed for v1 vector assets);
- a web column in the render-by-`kind` writer contract;
- web `app-icon` artifacts — `favicon.svg`, `favicon.ico`, `apple-touch-icon.png` (180×180), and manifest icons (192 / 512 PNG) — the only role requiring raster derivation, landing under `design-system/assets/exports/web/`;
- extending the bootstrap `app-icon` gate to treat `web` as a UI platform. There is no plan-time bootstrap trigger: platform shell bootstrap stays a build-time, adapter-owned concern.

The host [`Platform`](../../crates/workflow/src/platform.rs) enum already reserves `Web` as a placeholder; the vectis materialization code in `augentic/specify-adapters` covers `ios | android` today and must be extended. Everything lands in the vectis adapter's in-guest core (schema, materialize outputs, prepare scope, gate) plus a future `references/web/design-system-integration.md` and web build prompt — blocked on the web shell scaffold, which is its own RFC.

## Non-goals

- Defining the web shell scaffold itself (component tree, build system, routing) — a separate RFC.
- Re-specifying iOS/Android contracts; this RFC only adds the web seam.
- Generic image CDN or remote asset hosting.
- Plan-time shell bootstrap or `bootstrap-web` slice insertion.

A fuller prior draft, written against a retired host-dispatch surface, is recoverable from git history.

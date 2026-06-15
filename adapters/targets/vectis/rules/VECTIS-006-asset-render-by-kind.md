---
id: VECTIS-006
title: Asset Render-By-Kind
severity: important
trigger: A Vectis shell renders a composition-referenced asset id whose `assets.yaml` entry is `vector` or `raster` using a platform symbol instead of the materialized export.
applicability:
  adapters: [vectis]
references:
  - label: iOS render-by-kind contract
    path: adapters/targets/vectis/references/ios/design-system-integration.md
  - label: Android render-by-kind contract
    path: adapters/targets/vectis/references/android/design-system-integration.md
---

## Rule

Shell writers resolve each composition `icon` / `image` / `icon-button` / `fab` reference through `assets.yaml` and emit view code strictly by entry `kind`:

| `assets.<id>.kind` | iOS | Android |
|---|---|---|
| `vector` | `Image("<id>")` from a shell-local imageset copied from the materialized export | `painterResource(R.drawable.<id_snake>)` from a shell-local drawable copied from the materialized export |
| `raster` | `Image("<id>")` from a shell-local imageset with per-density PNGs | `painterResource` from per-density drawables |
| `symbol` | `Image(systemName: symbols.ios)` — no catalog copy | `Icon(imageVector = Icons.Default.<glyph>, …)` — no resource copy |

**Forbidden:** emitting `Image(systemName:)` (iOS) or `Icons.Default.*` / Material Icons substitutes (Android) for an id whose entry is `vector` or `raster`. Missing platform exports are validation errors (`assets-materialization-missing`) — never a writer shortcut. Platform glyph use requires an explicit `kind: symbol` entry (optionally `inferred: true` when promoted from screenshot inference).

v1 enforcement is **review-scoped**: build reviewers cross-reference `composition.yaml`, the effective `assets.yaml`, committed `exports/<platform>/` trees, and shell UI sources. Mechanical `specify lint project` hints that join composition ids to shell symbol fallbacks remain deferred to a follow-on rule.

## Look For

- A composition-referenced asset id whose `assets.<id>.kind` is `vector` or `raster`, but the matching screen view uses `Image(systemName:)` (iOS) or `Icons.Default.*` / `material.icons.Icons.*` (Android) for that visual role instead of the catalog / drawable emission.
- Shell code that substitutes SF Symbols or Material Icons when `sources.<platform>` pins or default export paths exist for the id — the export tree is the authority, not platform glyph names.
- `Image(systemName:)` or `Icons.Default.*` used for an id that appears in `composition.yaml` even when the writer intended a one-off decorative glyph — only `kind: symbol` entries may use platform symbols at build time.
- Legitimate `kind: symbol` entries rendered correctly — do **not** flag `Image(systemName:)` when the referenced composition id resolves to `kind: symbol` in the effective `assets.yaml`.

## Spec Guidance

When screenshot inference or a writer shortcut produced a platform symbol for branded artwork, promote the asset to `vector` or `raster` in `assets.yaml`, run `vectis materialize assets`, and regenerate the shell so the view copies the materialized export. When the glyph is genuinely platform-native chrome, add or retain a `kind: symbol` entry (with `symbols.ios` / `symbols.android`) and keep composition referencing that symbol id — do not leave a `vector` / `raster` entry in inventory while the shell still emits platform symbols.

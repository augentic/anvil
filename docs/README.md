# Emery Developer Guide -- Local Development

This directory contains the [mdBook](https://rust-lang.github.io/mdBook/) 0.5 source for the Emery Developer Guide. See [`standards/doc-authoring.md`](standards/doc-authoring.md) for HTML component blocks and admonition conventions.

## Prerequisites

Install the mdBook 0.5 toolchain locally:

```bash
cargo install --locked mdbook
cargo install --locked mdbook-linkcheck2
```

## Serve (live reload)

```bash
mdbook serve docs    # from the repo root
```

Opens at [http://localhost:3000](http://localhost:3000) by default and live-reloads on chapter or theme changes.

## Build

```bash
mdbook build docs   # from the repo root
```

Output lands in `docs/book/html/` (the CI deploy step points Cloudflare Pages at that path). The same build runs [`mdbook-linkcheck2`](https://github.com/marxin/mdbook-linkcheck2) via `[output.linkcheck2]` in [`book.toml`](book.toml) — relative links must resolve; web links are not a CI predicate. Local shorthand: `make links`.

## Custom theme and diagrams

- Forked mdBook theme: [`theme/`](theme/) — re-vendor from stock on mdBook upgrades (see below).
- Project-owned chrome overrides: [`theme/css/chrome.css`](theme/css/chrome.css) (banner block at file bottom), [`theme/head.hbs`](theme/head.hbs).
- Cross-cutting component CSS: [`assets/theme/emery-docs.css`](assets/theme/emery-docs.css).
- Interactive authority widget: [`assets/theme/authority-widget.js`](assets/theme/authority-widget.js).

### Re-vendoring the theme after an mdBook upgrade

1. In a temp directory: `mdbook init --theme tmp-book` using the target mdBook version.
2. Copy `tmp-book/theme/*` into [`docs/theme/`](theme/), replacing stock files.
3. Re-apply project-owned customisations:
  - Augentic brand + breadcrumb in [`theme/index.hbs`](theme/index.hbs) (`menu-title`, `spec-footer`).
  - Banner block at the bottom of [`theme/css/chrome.css`](theme/css/chrome.css).
  - [`theme/head.hbs`](theme/head.hbs) and system-font override in [`theme/fonts/fonts.css`](theme/fonts/fonts.css).
4. Run `mdbook build docs` and spot-check light + navy themes on [`index.md`](index.md).

A first-run failure usually points at a missing tool — install the prerequisites above.

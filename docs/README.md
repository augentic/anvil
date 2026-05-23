# Specify Developer Guide -- Local Development

This directory contains the [mdbook](https://rust-lang.github.io/mdBook/) source for the Specify Developer Guide. The visual system mirrors [`example.html`](example.html); see [`standards/doc-authoring.md`](standards/doc-authoring.md) for component-class and partial reference.

## Prerequisites

The build pipeline pins to the mdbook 0.4 ecosystem (the preprocessor crates have not yet caught up with the 0.5 line). One-shot install:

```bash
./scripts/docs-prereqs.sh
```

The script honours the version pins below; export the corresponding env var to override any one of them:

| Tool | Version | Override env var |
|------|---------|------------------|
| `mdbook` | 0.4.52 | `MDBOOK_VERSION` |
| `mdbook-d2` | 0.3.4 | `MDBOOK_D2_VERSION` |
| `mdbook-linkcheck` | 0.7.7 | `MDBOOK_LINKCHECK_VERSION` |
| `mdbook-pagetoc` | 0.2.0 | `MDBOOK_PAGETOC_VERSION` |
| `mdbook-template` | 1.1.1 | `MDBOOK_TEMPLATE_VERSION` |
| `D2` | latest | (installed via `https://d2lang.com/install.sh` if missing) |

If you prefer manual installation, mirror the same versions:

```bash
cargo install --locked --version 0.4.52 mdbook
cargo install --locked --version 0.3.4 mdbook-d2
cargo install --locked --version 0.7.7 mdbook-linkcheck
cargo install --locked --version 0.2.0 mdbook-pagetoc
cargo install --locked --version 1.1.1 mdbook-template
curl -fsSL https://d2lang.com/install.sh | sh -s --
```

## Serve locally (with live-reload)

```bash
make docs-serve    # from the repo root
```

Opens at <http://localhost:3000> by default and live-reloads on every change to a chapter, a template partial, or the forked theme.

## One-off build

```bash
make docs   # from the repo root, runs mdbook build + linkcheck
```

Output lands in `docs/book/html/` (with `[output.linkcheck]` enabled mdbook nests each backend in its own subdirectory; the CI deploy step points Cloudflare Pages at that path). The `linkcheck` backend validates every internal link and fails the build on the first broken reference — see [`book.toml`](book.toml) `[output.linkcheck]`.

## Custom theme and diagrams

- Forked mdbook theme: [`theme/`](theme/) (commit the snapshot before customising; mdbook upgrades require a re-vendor).
- Project-owned chrome overrides: [`theme/css/chrome.css`](theme/css/chrome.css), bottom banner.
- Cross-cutting component CSS: [`assets/theme/specify-docs.css`](assets/theme/specify-docs.css).
- Interactive authority widget: [`assets/theme/authority-widget.js`](assets/theme/authority-widget.js).
- Reusable partials: [`templates/`](templates/) (see [authoring standards](standards/doc-authoring.md#authoring-partials)).
- SVG diagrams: [`assets/diagrams/`](assets/diagrams/) — see `_STYLE.md` in that folder.
- D2 fences: rendered inline by [`mdbook-d2`](https://github.com/danieleades/mdbook-d2); the `d2` binary must be on `PATH`.

From the repo root you can also run `make docs` / `make docs-serve` directly without invoking the install script — but a first-run failure usually points at a missing tool.

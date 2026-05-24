# Specify docs partials

This directory holds the mdbook-template partials used by chapters to compose
the visual system. Each `.md` file is
a snippet of CommonMark + raw HTML that gets substituted in-place when a
chapter writes:

```text
{{#template ../templates/<partial>.md key=value …}}
```

Paths are resolved relative to the chapter file's location, so chapters in
`docs/` use `templates/<partial>.md` and chapters under `docs/reference/`
use `../templates/<partial>.md`. See
[`../standards/doc-authoring.md`](../standards/doc-authoring.md) for argument
contracts, copy-paste examples, and page-type scaffolds.

Partials with `*-open.md` / `*-close.md` pairs are wrapper blocks; everything
between the two invocations is rendered as normal markdown. Wherever a
single self-closing partial exists, the body (if any) is passed via a named
argument such as `caption=` or `text=`.

## Inventory

| Partial | Args | Use |
| ------- | ---- | --- |
| `hero-open` / `hero-close` | `eyebrow`, `title` | Top-of-chapter hero |
| `meta-row-open` / `meta-row-close` | — | Meta chip row wrapper |
| `meta-chip` | `label`, `value` | Hero meta chip |
| `section-open` / `section-close` | `id`, `num`, `title` | Numbered sections |
| `tutorial-step-open` / `tutorial-step-close` | `num`, `title` | Tutorial steps |
| `prereq-open` / `prereq-close` | — | Prerequisites block |
| `when-open` / `when-close` | — | How-to when-to-use |
| `rhythm-open` / `rhythm-close` | — | Rhythm card grid wrapper |
| `rhythm-step-open` / `rhythm-step-close` | `num`, `label`, `title` | Single rhythm card |
| `card-grid-open` / `card-grid-close` | — | Landing page card grid |
| `card-open` / `card-close` | `title`, `time`, `href` | Single landing card |
| `synopsis-open` / `synopsis-close` | — | Reference synopsis |
| `see-also-open` / `see-also-close` | — | Footer see-also block |
| `platform-open` / `platform-close` | — | Product stack wrapper |
| `platform-product-open` / `platform-product-close` | `name`, `role`, `active` | Single product card |
| `audience-grid-open` / `audience-grid-close` | — | Audience routing grid |
| `audience-open` / `audience-close` | `who` | Single audience card |
| `pipeline-open` / `pipeline-close` | `caption` (close) | SVG diagram frame |
| `callout-open` / `callout-close` | `variant` (optional) | Callout variants |
| `decisions-open` / `decisions-close` | — | Decision card grid |
| `decision-open` / `decision-close` | `id`, `tag`, `title` | Single decision card |
| `decision-consequence` | `text` | Optional consequence line |
| `questions-open` / `questions-close` | — | FAQ grid |
| `question-open` / `question-close` | `qnum`, `q` | Single FAQ card |
| `alt-open` / `alt-close` | `status`, `title`, `tag` | Alternatives accordion |
| `status-pill` | `label` | Inline status badge |
| `matrix-row` | — | Acceptance matrix row |

This directory is **not** linked from `SUMMARY.md` and is excluded from the
search index by virtue of mdbook ignoring non-summary `.md` files.

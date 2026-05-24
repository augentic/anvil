# Diagram authoring contract

Standalone SVG files under `docs/assets/diagrams/` follow these rules so hero pages stay visually consistent with the Specify 2.0 reference style.

## File format

- Valid SVG 1.1 with `xmlns="http://www.w3.org/2000/svg"`.
- Root element: `role="img"` and `aria-label="…"` (one sentence describing the diagram).
- Include a `<title>…</title>` child for accessibility (matches `aria-label`).
- Use a fixed `viewBox`; prefer width 900–940 and height 180–460 depending on layout.
- Embed `<defs>` for arrow markers; reuse the chevron marker pattern from existing diagrams in this tree.

## Colors (light theme, baked in)

Standalone SVG files cannot rely on mdBook CSS variables. Use explicit fills:

| Role | Fill | Stroke |
|------|------|--------|
| Default node | `#f2f1ec` | `#c9c6bc` |
| Source adapter | `#ede9d0` | `#c9b766` |
| Synthesis / output | `#d9e7f1` | `#6ea8d8` |
| Target / build | `#e6d9f0` | `#a489d8` |
| New / highlight | `#d4ecd9` | `#6db978` (stroke-width 1.5) |
| Ink (text) | `#1c1c1c` | — |
| Dim text | `#555555` | — |
| Faint / pillar labels | `#888888` | — |
| Flow arrows | `#555555` | stroke-width 1.2 |

## Typography

- Titles on nodes: `font-family="ui-sans-serif, system-ui, sans-serif"`, `font-size="13"`, `font-weight="600"`.
- Subtitles / paths: `font-family="ui-monospace, monospace"`, `font-size="11"`.
- Pillar column labels: uppercase, `letter-spacing="0.06em"`, `font-size="11"`, `font-weight="600"`.
- NEW badges: `font-size="9"`, `font-weight="600"`, fill `#2f7d3e`.

## Layout

- Multi-column pipeline diagrams: label pillars at the top (e.g. Source adapters, Discovery + plan, Synthesis).
- Rounded rects: `rx="8"`.
- Minimum node width 160px; leave 20px gutter between columns.
- Arrows use `marker-end="url(#arr)"` on paths.

## Markdown embedding

From `docs/explanation/foo.md`:

```markdown
<div class="pipeline">

![Caption text](../assets/diagrams/foo/bar.svg)

<p class="pipeline-caption">Mono caption — what the diagram shows.</p>
</div>
```

Adjust `../` depth for file location (`orientation/` uses `../assets/…`, `reference/` uses `../assets/…`).

## When to use SVG vs other formats

- **SVG**: workflow pipelines, layered architecture, state machines, adapter axis diagrams.
- **Table**: field inventories, CLI flag lists, closed enums.
- **D2** (legacy): reference pages only until individually upgraded.
- **` ```text `**: banned for pipeline diagrams in `docs/explanation/` and `docs/orientation/` (enforced by `make checks`).

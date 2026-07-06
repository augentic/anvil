---
id: CORE-015
title: Docs Missing Diagram Asset
severity: important
trigger: A documentation page embeds an `.svg` image whose asset file does not exist on disk.
rule_hints:
  - kind: path-pattern
    value: "docs/**/*.md"
    description: Long-form contributor and explanation documentation.
  - kind: path-pattern
    value: "!docs/book/**"
    description: Rendered mdBook output is generated, not authored.
  - kind: path-pattern
    value: "!docs/assets/diagrams/_STYLE.md"
    description: The diagram style guide intentionally cites illustrative paths.
  - kind: path-pattern
    value: "!docs/standards/doc-authoring.md"
    description: The authoring standard documents example image syntax.
  - kind: reference-resolves
    value: markdown-link
    description: Flag `![alt](….svg)` image embeds whose relative target does not resolve on disk.
    config:
      image: true
      target-suffix: ".svg"
---

## Rule

Every `![alt](path.svg)` image embed under `docs/` must resolve to a committed asset after the target is joined against the page's parent directory. A missing diagram asset renders as a broken image in the documentation; SVG is the house diagram format, so this rule covers the `.svg` embeds specifically.

The deterministic-hint interpreter consumes the `markdown_link` image facts the indexer produces (`![alt](src)` embeds carry the `image` flag), restricted to the `.svg` suffix carried in `config`. URL-style targets (`http://…`, `https://…`) are skipped because the resolver never attempts to resolve them. The rendered mdBook tree, the diagram style guide, and the authoring standard are excluded because they intentionally cite illustrative paths.

## Look For

- An `![diagram](../assets/diagrams/foo.svg)` embed whose joined path does not exist on disk.
- A diagram renamed or moved under `docs/assets/diagrams/` without updating the page that embeds it.

## Fix

Commit the missing SVG asset, or update the embed to the asset's current path. Author new diagrams under `docs/assets/diagrams/` per the style guide.
